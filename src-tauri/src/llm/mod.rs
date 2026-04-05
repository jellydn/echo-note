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

/// Generate a structured summary from a transcript using Ollama
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
    let client = reqwest::Client::new();
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

    // Parse the structured response
    let summary = parse_summary_response(&ollama_response.response);

    let duration = start_time.elapsed().as_secs_f64();
    log::info!("Summary generated in {:.2} seconds", duration);

    Ok(SummaryResult {
        key_points: summary.key_points,
        decisions: summary.decisions,
        action_items: summary.action_items,
        duration_seconds: duration,
    })
}

/// Build the prompt for structured meeting summarization
fn build_summary_prompt(transcript: &str) -> String {
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
        transcript
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
    let client = reqwest::Client::new();
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
}
