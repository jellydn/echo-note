use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
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

    let prompt = build_summary_prompt(transcript);

    let request = ChatCompletionRequest {
        model: model.to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: prompt,
        }],
        temperature: 0.7,
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

/// Parsed summary structure
struct ParsedSummary {
    key_points: String,
    decisions: String,
    action_items: String,
}

/// Parse the LLM response into structured sections
fn parse_summary_response(response: &str) -> ParsedSummary {
    let response = response.trim();

    // Find sections
    let key_points = extract_section(response, "KEY POINTS:", Some("DECISIONS:"))
        .or_else(|| extract_section(response, "Key Points:", Some("Decisions:")))
        .or_else(|| extract_section(response, "Key points:", Some("Decisions:")))
        .unwrap_or_default();

    let decisions = extract_section(response, "DECISIONS:", Some("ACTION ITEMS:"))
        .or_else(|| extract_section(response, "Decisions:", Some("Action Items:")))
        .or_else(|| extract_section(response, "Decisions:", Some("Action items:")))
        .unwrap_or_default();

    let action_items = extract_section(response, "ACTION ITEMS:", None)
        .or_else(|| extract_section(response, "Action Items:", None))
        .or_else(|| extract_section(response, "Action items:", None))
        .unwrap_or_default();

    // Clean up and format
    ParsedSummary {
        key_points: clean_section(&key_points),
        decisions: clean_section(&decisions),
        action_items: clean_section(&action_items),
    }
}

/// Extract a section from text between start marker and optional end marker
fn extract_section(text: &str, start_marker: &str, end_marker: Option<&str>) -> Option<String> {
    let start_idx = text.find(start_marker)?;
    let content_start = start_idx + start_marker.len();

    let content_end = match end_marker {
        Some(marker) => text[content_start..]
            .find(marker)
            .map(|idx| content_start + idx),
        None => None,
    };

    let content = match content_end {
        Some(end) => &text[content_start..end],
        None => &text[content_start..],
    };

    Some(content.trim().to_string())
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
    fn test_extract_section() {
        let text = "KEY POINTS:\n- point 1\n\nDECISIONS:\n- decision 1";

        let section = extract_section(text, "KEY POINTS:", Some("DECISIONS:"));
        assert_eq!(section.unwrap().trim(), "- point 1");
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
