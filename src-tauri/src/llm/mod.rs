use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Instant;

/// Default Ollama API endpoint
pub const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";

/// Default model to use for summarization
pub const DEFAULT_SUMMARY_MODEL: &str = "llama3.2";

/// Summary generation result
#[derive(Clone, serde::Serialize)]
pub struct SummaryResult {
    pub key_points: String,
    pub decisions: String,
    pub action_items: String,
    pub duration_seconds: f64,
}

/// Ollama generate request payload
#[derive(Serialize)]
struct OllamaGenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
    options: Option<OllamaOptions>,
}

/// Ollama options for generation
#[derive(Serialize)]
struct OllamaOptions {
    temperature: f32,
}

/// Ollama generate response (non-streaming)
#[derive(Deserialize)]
struct OllamaGenerateResponse {
    response: String,
    #[allow(dead_code)]
    done: bool,
}

/// LLM Provider types
#[allow(dead_code)]
pub const PROVIDER_OLLAMA: &str = "ollama";
pub const PROVIDER_API: &str = "api";

/// OpenAI-compatible chat completions request
#[derive(Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
}

/// JSON response format hint for OpenAI-compatible providers
#[derive(Serialize)]
struct ResponseFormat {
    r#type: String,
}

/// Chat message for API requests
#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

/// OpenAI-compatible chat completions response
#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

/// Choice in chat completion response
#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

/// Response message from API
#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

/// Generate a summary using OpenAI-compatible API
pub async fn generate_summary_api(
    api_endpoint: &str,
    api_key: &str,
    model: &str,
    transcript: &str,
) -> Result<SummaryResult> {
    let start_time = Instant::now();

    log::info!(
        "Generating summary using API at {} with model {}",
        api_endpoint,
        model
    );

    let prompt = build_summary_json_prompt(transcript);

    let request = ChatCompletionRequest {
        model: model.to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: prompt,
        }],
        temperature: 0.7,
        response_format: Some(ResponseFormat {
            r#type: "json_object".to_string(),
        }),
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .context("Failed to build HTTP client")?;

    let response = client
        .post(api_endpoint)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .send()
        .await
        .context("Failed to connect to API endpoint")?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(anyhow::anyhow!("API error ({}): {}", status, error_text));
    }

    let api_response: ChatCompletionResponse = response
        .json()
        .await
        .context("Failed to parse API response")?;

    let content = api_response
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_default();

    log::info!("Received response from API, parsing summary...");

    let summary = parse_summary_response(&content);
    let duration = start_time.elapsed().as_secs_f64();

    log::info!("Summary generated via API in {:.2} seconds", duration);

    Ok(SummaryResult {
        key_points: summary.key_points,
        decisions: summary.decisions,
        action_items: summary.action_items,
        duration_seconds: duration,
    })
}
pub async fn generate_summary(
    ollama_url: &str,
    model: &str,
    transcript: &str,
) -> Result<SummaryResult> {
    let start_time = Instant::now();

    log::info!(
        "Generating summary using Ollama at {} with model {}",
        ollama_url,
        model
    );
    log::debug!("Transcript length: {} chars", transcript.len());
    log::debug!("Transcript preview: {:.200}", transcript);

    // Check for empty or minimal transcript
    let cleaned = transcript.trim();
    if cleaned.is_empty() {
        return Err(anyhow::anyhow!(
            "Transcript is empty - no speech detected in the recording"
        ));
    }

    // Build the prompt for structured summarization
    let prompt = build_summary_prompt(transcript);

    // Create the request
    let request = OllamaGenerateRequest {
        model: model.to_string(),
        prompt,
        stream: false,
        options: Some(OllamaOptions { temperature: 0.7 }),
    };

    // Send request to Ollama
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .context("Failed to build HTTP client")?;
    let url = format!("{}/api/generate", ollama_url);

    log::debug!("Sending request to Ollama at {}", url);

    let response = client
        .post(&url)
        .json(&request)
        .send()
        .await
        .context("Failed to connect to Ollama. Make sure Ollama is running on port 11434")?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(anyhow::anyhow!(
            "Ollama API error ({}): {}",
            status,
            error_text
        ));
    }

    let ollama_response: OllamaGenerateResponse = response
        .json()
        .await
        .context("Failed to parse Ollama response")?;

    log::info!("Received response from Ollama, parsing summary...");
    log::debug!(
        "Ollama response length: {} chars",
        ollama_response.response.len()
    );
    log::debug!(
        "Ollama response preview: {:.200}",
        &ollama_response.response
    );

    // Check for empty response
    if ollama_response.response.trim().is_empty() {
        return Err(anyhow::anyhow!("Ollama returned empty response - the model may not be loaded or the transcript was too short"));
    }

    // Parse the structured response
    let summary = parse_summary_response(&ollama_response.response);

    // Check if all sections are empty
    if summary.key_points.is_empty()
        && summary.decisions.is_empty()
        && summary.action_items.is_empty()
    {
        return Err(anyhow::anyhow!("Failed to parse summary sections from model response. The model may not be following the expected format. Response preview: {:.200}", ollama_response.response));
    }

    let duration = start_time.elapsed().as_secs_f64();
    log::info!("Summary generated in {:.2} seconds", duration);

    Ok(SummaryResult {
        key_points: summary.key_points,
        decisions: summary.decisions,
        action_items: summary.action_items,
        duration_seconds: duration,
    })
}

/// Filter out Whisper special tokens from transcript
fn clean_transcript(transcript: &str) -> String {
    // Remove common Whisper special tokens and non-speech markers
    let special_tokens = [
        "[_SOT_]",
        "[_EOT_]",
        "[_BEG_]",
        "[_TT_",
        "[_PREV_]",
        "[_LANG_",
        "[_TRANSCRIBE_]",
        "[_TRANSLATE_]",
        "[ Pause ]",
        "[Typing]",
        "[typing]",
        "[SILENCE]",
        "[NOISE]",
        "[MUSIC]",
        "[APPLAUSE]",
        "[LAUGHTER]",
        "[BLANK_AUDIO]",
    ];

    let mut cleaned = transcript.to_string();

    // Remove tokens that appear in brackets
    for token in special_tokens {
        cleaned = cleaned.replace(token, "");
    }

    // Clean up multiple spaces and trim
    cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");

    cleaned
}

/// Build the prompt for structured meeting summarization
fn build_summary_prompt(transcript: &str) -> String {
    // Clean special tokens before building prompt
    let cleaned_transcript = clean_transcript(transcript);

    format!(
        r#"You are a meeting assistant that creates structured summaries from meeting transcripts.

Please analyze the following meeting transcript and provide a structured summary with these three sections:

1. KEY POINTS (3-5 bullet points of the most important discussion points)
2. DECISIONS (any decisions made during the meeting)
3. ACTION ITEMS (tasks assigned with who should do what, if mentioned)

Format your response exactly like this:

KEY POINTS:
- point 1
- point 2
- point 3

DECISIONS:
- decision 1
- decision 2

ACTION ITEMS:
- person: task description
- person: task description

If a section has no content, write "None" under that section.

Here is the transcript:

{}"#,
        cleaned_transcript
    )
}

/// Build the prompt for JSON meeting summarization with API providers.
fn build_summary_json_prompt(transcript: &str) -> String {
    format!(
        r#"You are a meeting assistant that creates structured summaries from meeting transcripts.

Analyze the following meeting transcript and return only a valid JSON object with this shape:

{{
  "key_points": ["3-5 bullet points of the most important discussion points"],
  "decisions": ["Any decisions made during the meeting"],
  "action_items": ["Tasks assigned with who should do what, if mentioned"]
}}

Use empty arrays for sections with no content.

Here is the transcript:

{}"#,
        transcript
    )
}

/// Parsed summary structure
struct ParsedSummary {
    key_points: String,
    decisions: String,
    action_items: String,
}

#[derive(Clone, Copy)]
enum SummarySection {
    KeyPoints,
    Decisions,
    ActionItems,
}

/// Parse the LLM response into structured sections
fn parse_summary_response(response: &str) -> ParsedSummary {
    let response = response.trim();

    parse_json_summary(response).unwrap_or_else(|| parse_text_summary(response))
}

fn parse_json_summary(response: &str) -> Option<ParsedSummary> {
    let json_text = find_json_object(response)?;
    let value: Value = serde_json::from_str(json_text).ok()?;

    let summary = ParsedSummary {
        key_points: clean_section(&json_section_value(
            &value,
            &[
                "key_points",
                "keyPoints",
                "key points",
                "Key Points",
                "KEY POINTS",
            ],
        )),
        decisions: clean_section(&json_section_value(
            &value,
            &["decisions", "Decisions", "DECISIONS"],
        )),
        action_items: clean_section(&json_section_value(
            &value,
            &[
                "action_items",
                "actionItems",
                "action items",
                "Action Items",
                "ACTION ITEMS",
            ],
        )),
    };

    if summary.key_points.is_empty()
        && summary.decisions.is_empty()
        && summary.action_items.is_empty()
    {
        None
    } else {
        Some(summary)
    }
}

fn find_json_object(response: &str) -> Option<&str> {
    let start = response.find('{')?;
    let mut in_string = false;
    let mut escaped = false;
    let mut depth = 0usize;

    for (offset, ch) in response[start..].char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        match ch {
            '\\' if in_string => escaped = true,
            '"' => in_string = !in_string,
            '{' if !in_string => depth += 1,
            '}' if !in_string => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let end = start + offset + ch.len_utf8();
                    return Some(&response[start..end]);
                }
            }
            _ => {}
        }
    }

    None
}

fn json_section_value(value: &Value, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|key| value.get(*key))
        .map(json_value_to_lines)
        .unwrap_or_default()
}

fn json_value_to_lines(value: &Value) -> String {
    match value {
        Value::Array(items) => items
            .iter()
            .filter_map(json_value_to_line)
            .collect::<Vec<_>>()
            .join("\n"),
        _ => json_value_to_line(value).unwrap_or_default(),
    }
}

fn json_value_to_line(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Null => None,
        Value::Object(object) => object
            .get("task")
            .and_then(Value::as_str)
            .or_else(|| object.get("description").and_then(Value::as_str))
            .map(|task| {
                object
                    .get("person")
                    .or_else(|| object.get("owner"))
                    .or_else(|| object.get("assignee"))
                    .and_then(Value::as_str)
                    .map(|person| format!("{}: {}", person, task))
                    .unwrap_or_else(|| task.to_string())
            })
            .or_else(|| Some(value.to_string())),
        _ => Some(value.to_string()),
    }
}

fn parse_text_summary(response: &str) -> ParsedSummary {
    let mut current_section = None;
    let mut key_points = Vec::new();
    let mut decisions = Vec::new();
    let mut action_items = Vec::new();

    for line in response.lines() {
        if let Some((section, remainder)) = parse_section_header(line) {
            current_section = Some(section);
            if !remainder.is_empty() {
                push_section_line(
                    section,
                    remainder,
                    &mut key_points,
                    &mut decisions,
                    &mut action_items,
                );
            }
            continue;
        }

        if let Some(section) = current_section {
            push_section_line(
                section,
                line,
                &mut key_points,
                &mut decisions,
                &mut action_items,
            );
        }
    }

    // Clean up and format
    ParsedSummary {
        key_points: clean_section(&key_points.join("\n")),
        decisions: clean_section(&decisions.join("\n")),
        action_items: clean_section(&action_items.join("\n")),
    }
}

fn parse_section_header(line: &str) -> Option<(SummarySection, &str)> {
    let trimmed = line.trim().trim_start_matches('#').trim();
    let (label, remainder) = trimmed
        .split_once(':')
        .map(|(label, remainder)| (label.trim(), remainder.trim()))
        .unwrap_or((trimmed, ""));

    let label = label
        .trim_start_matches(|ch: char| ch.is_ascii_digit() || ch == '.' || ch == ')')
        .trim();
    let normalized = label.to_lowercase().replace(['_', '-'], " ");

    match normalized.as_str() {
        "key points" => Some((SummarySection::KeyPoints, remainder)),
        "decisions" => Some((SummarySection::Decisions, remainder)),
        "action items" => Some((SummarySection::ActionItems, remainder)),
        _ => None,
    }
}

fn push_section_line(
    section: SummarySection,
    line: &str,
    key_points: &mut Vec<String>,
    decisions: &mut Vec<String>,
    action_items: &mut Vec<String>,
) {
    match section {
        SummarySection::KeyPoints => key_points.push(line.to_string()),
        SummarySection::Decisions => decisions.push(line.to_string()),
        SummarySection::ActionItems => action_items.push(line.to_string()),
    }
}

/// Clean up a section (remove "None" if it's the only content, normalize bullets)
fn clean_section(content: &str) -> String {
    let trimmed = content.trim();

    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("none") {
        return String::new();
    }

    // Normalize line endings and bullet points
    let lines: Vec<&str> = trimmed.lines().collect();
    let cleaned_lines: Vec<String> = lines
        .iter()
        .map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return String::new();
            }
            // Ensure bullet points start with "- "
            if !line.starts_with("-") && !line.starts_with("*") && !line.starts_with("•") {
                format!("- {}", line)
            } else {
                line.to_string()
            }
        })
        .filter(|line| !line.is_empty())
        .collect();

    cleaned_lines.join("\n")
}

/// Check if Ollama is available at the given URL
pub async fn check_ollama_status(ollama_url: &str) -> Result<bool> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_default();
    let url = format!("{}/api/tags", ollama_url);

    match client.get(&url).send().await {
        Ok(response) => Ok(response.status().is_success()),
        Err(_) => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_summary_prompt() {
        let prompt = build_summary_prompt("Test transcript");
        assert!(prompt.contains("KEY POINTS"));
        assert!(prompt.contains("DECISIONS"));
        assert!(prompt.contains("ACTION ITEMS"));
        assert!(prompt.contains("Test transcript"));
    }

    #[test]
    fn test_build_summary_json_prompt() {
        let prompt = build_summary_json_prompt("Test transcript");
        assert!(prompt.contains("\"key_points\""));
        assert!(prompt.contains("\"decisions\""));
        assert!(prompt.contains("\"action_items\""));
        assert!(prompt.contains("valid JSON object"));
        assert!(prompt.contains("Test transcript"));
    }

    #[test]
    fn test_parse_summary_response() {
        let response = r#"KEY POINTS:
- Discussed project timeline
- Reviewed budget

DECISIONS:
- Approved Q1 plan

ACTION ITEMS:
- Alice: Prepare report
- Bob: Schedule follow-up"#;

        let summary = parse_summary_response(response);
        assert!(summary.key_points.contains("Discussed project timeline"));
        assert!(summary.decisions.contains("Approved Q1 plan"));
        assert!(summary.action_items.contains("Alice: Prepare report"));
    }

    #[test]
    fn test_parse_summary_response_json() {
        let response = r#"{
  "key_points": ["Discussed project timeline", "Reviewed budget"],
  "decisions": ["Approved Q1 plan"],
  "action_items": ["Alice: Prepare report", "Bob: Schedule follow-up"]
}"#;

        let summary = parse_summary_response(response);
        assert_eq!(
            summary.key_points,
            "- Discussed project timeline\n- Reviewed budget"
        );
        assert_eq!(summary.decisions, "- Approved Q1 plan");
        assert_eq!(
            summary.action_items,
            "- Alice: Prepare report\n- Bob: Schedule follow-up"
        );
    }

    #[test]
    fn test_parse_summary_response_markdown_fenced_json() {
        let response = r#"Here is the summary:

```json
{
  "Key Points": ["Discussed launch readiness"],
  "Decisions": "Ship the beta",
  "Action Items": [{"person": "Alice", "task": "Send release notes"}]
}
```"#;

        let summary = parse_summary_response(response);
        assert_eq!(summary.key_points, "- Discussed launch readiness");
        assert_eq!(summary.decisions, "- Ship the beta");
        assert_eq!(summary.action_items, "- Alice: Send release notes");
    }

    #[test]
    fn test_parse_summary_response_lowercase_headers() {
        let response = r#"key points:
- Discussed project timeline

decisions:
- Approved Q1 plan

action items:
- Alice: Prepare report"#;

        let summary = parse_summary_response(response);
        assert_eq!(summary.key_points, "- Discussed project timeline");
        assert_eq!(summary.decisions, "- Approved Q1 plan");
        assert_eq!(summary.action_items, "- Alice: Prepare report");
    }

    #[test]
    fn test_parse_summary_response_markdown_headers_with_missing_section() {
        let response = r#"## Key Points
- Discussed project timeline

## Action Items
- Alice: Prepare report"#;

        let summary = parse_summary_response(response);
        assert_eq!(summary.key_points, "- Discussed project timeline");
        assert_eq!(summary.decisions, "");
        assert_eq!(summary.action_items, "- Alice: Prepare report");
    }

    #[test]
    fn test_parse_summary_response_json_missing_sections() {
        let response = r#"{"key_points": ["Discussed project timeline"]}"#;

        let summary = parse_summary_response(response);
        assert_eq!(summary.key_points, "- Discussed project timeline");
        assert_eq!(summary.decisions, "");
        assert_eq!(summary.action_items, "");
    }

    #[test]
    fn test_parse_summary_response_malformed_response() {
        let response = "This is not structured summary content.";

        let summary = parse_summary_response(response);
        assert_eq!(summary.key_points, "");
        assert_eq!(summary.decisions, "");
        assert_eq!(summary.action_items, "");
    }

    #[test]
    fn test_clean_section() {
        assert_eq!(clean_section("None"), "");
        assert_eq!(clean_section(""), "");
        assert_eq!(clean_section("Point 1\nPoint 2"), "- Point 1\n- Point 2");
        assert_eq!(
            clean_section("- Point 1\n- Point 2"),
            "- Point 1\n- Point 2"
        );
    }

    #[test]
    fn test_default_constants() {
        assert_eq!(DEFAULT_OLLAMA_URL, "http://localhost:11434");
        assert_eq!(DEFAULT_SUMMARY_MODEL, "llama3.2");
    }

    #[test]
    fn test_clean_transcript() {
        // Test removing special tokens
        let dirty = "[_BEG_] Hello world [_EOT_]";
        assert_eq!(clean_transcript(dirty), "Hello world");

        // Test removing pause tokens
        let with_pause = "[ Pause ] Discussing the project [ Pause ]";
        assert_eq!(clean_transcript(with_pause), "Discussing the project");

        // Test removing typing tokens
        let with_typing = "[typing] Important point [typing]";
        assert_eq!(clean_transcript(with_typing), "Important point");

        // Test multiple spaces cleanup
        let messy = "  Multiple    spaces   here  ";
        assert_eq!(clean_transcript(messy), "Multiple spaces here");

        // Test empty result when only special tokens
        let only_special = "[_BEG_] [ Pause ] [typing] [_EOT_]";
        assert_eq!(clean_transcript(only_special), "");
    }
}
