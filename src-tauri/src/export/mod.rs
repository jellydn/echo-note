//! Meeting export (issue #13).
//!
//! Renders a meeting — metadata, transcript, and summary — into Markdown so
//! users can archive or share meeting notes outside the app. The rendering is
//! a pure function over plain data so it is easy to unit test and safe to
//! call from a Tauri command.

use crate::db::{Meeting, Summary, Transcript};

/// Render a complete meeting export as Markdown.
pub fn render_meeting_markdown(
    meeting: &Meeting,
    transcript: Option<&Transcript>,
    summary: Option<&Summary>,
) -> String {
    let mut md = String::new();

    md.push_str(&format!("# {}\n\n", meeting.title));
    md.push_str(&format!(
        "- **Date:** {}\n",
        meeting.date.format("%Y-%m-%d %H:%M UTC")
    ));
    md.push_str(&format!(
        "- **Duration:** {}\n\n",
        format_duration(meeting.duration_seconds)
    ));

    if let Some(summary) = summary {
        md.push_str("## Summary\n\n");
        push_section(&mut md, "Key Points", &summary.key_points);
        push_section(&mut md, "Decisions", &summary.decisions);
        push_section(&mut md, "Action Items", &summary.action_items);
        md.push('\n');
    }

    if let Some(transcript) = transcript {
        md.push_str("## Transcript\n\n");
        md.push_str(&transcript.content);
        md.push('\n');
    }

    md
}

/// Render a plain-text (non-Markdown) export for users who want raw notes.
pub fn render_meeting_text(
    meeting: &Meeting,
    transcript: Option<&Transcript>,
    summary: Option<&Summary>,
) -> String {
    let mut text = String::new();

    text.push_str(&format!("{}\n", meeting.title));
    text.push_str(&format!(
        "Date: {}\n",
        meeting.date.format("%Y-%m-%d %H:%M UTC")
    ));
    text.push_str(&format!(
        "Duration: {}\n",
        format_duration(meeting.duration_seconds)
    ));

    if let Some(summary) = summary {
        text.push_str("\n--- Summary ---\n");
        text.push_str(&format!("Key Points:\n{}\n", summary.key_points));
        text.push_str(&format!("Decisions:\n{}\n", summary.decisions));
        text.push_str(&format!("Action Items:\n{}\n", summary.action_items));
    }

    if let Some(transcript) = transcript {
        text.push_str("\n--- Transcript ---\n");
        text.push_str(&transcript.content);
        text.push('\n');
    }

    text
}

/// Slugify a meeting title for use in an export filename: lowercase, keep
/// Unicode alphanumerics, replace everything else with '-', trim edge dashes,
/// and fall back to "meeting" when empty.
pub fn export_slug(title: &str) -> String {
    let slug = title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>();
    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        "meeting".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Build the export filename `{slug}-{YYYYMMDD}.{ext}` with the date stamped
/// in UTC (`%Y%m%d`), mirroring the real export command's filename generation.
pub fn export_filename(meeting: &Meeting, ext: &str) -> String {
    format!(
        "{}-{}.{}",
        export_slug(&meeting.title),
        meeting.date.format("%Y%m%d"),
        ext
    )
}

/// Push a bulleted Markdown section, skipping empty content.
fn push_section(md: &mut String, title: &str, content: &str) {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return;
    }
    md.push_str(&format!("### {title}\n\n"));
    for line in trimmed.lines().filter(|l| !l.trim().is_empty()) {
        let line = line.trim();
        if line.starts_with('-') || line.starts_with('*') {
            md.push_str(line);
        } else {
            md.push_str(&format!("- {line}"));
        }
        md.push('\n');
    }
    md.push('\n');
}

fn format_duration(seconds: i64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if hours > 0 {
        format!("{hours}h {minutes}m {secs:02}s")
    } else if minutes > 0 {
        format!("{minutes}m {secs:02}s")
    } else {
        format!("{secs}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn meeting() -> Meeting {
        Meeting {
            id: 1,
            title: "Weekly Sync".to_string(),
            date: Utc.with_ymd_and_hms(2026, 5, 6, 15, 56, 0).unwrap(),
            duration_seconds: 3605,
            audio_path: "/tmp/rec.wav".to_string(),
            created_at: Utc::now(),
        }
    }

    fn transcript() -> Transcript {
        Transcript {
            id: 1,
            meeting_id: 1,
            content: "[00:00] Speaker 1: Hello world".to_string(),
            created_at: Utc::now(),
        }
    }

    fn summary() -> Summary {
        Summary {
            id: 1,
            meeting_id: 1,
            key_points: "- Discussed Q2 plan".to_string(),
            decisions: "None".to_string(),
            action_items: "- Alice: send notes".to_string(),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn markdown_export_includes_metadata() {
        let md = render_meeting_markdown(&meeting(), None, None);
        assert!(md.contains("# Weekly Sync"));
        assert!(md.contains("**Date:**"));
        assert!(md.contains("**Duration:** 1h 0m 05s"));
    }

    #[test]
    fn markdown_export_includes_transcript_and_summary() {
        let md = render_meeting_markdown(&meeting(), Some(&transcript()), Some(&summary()));
        assert!(md.contains("## Summary"));
        assert!(md.contains("### Key Points"));
        assert!(md.contains("- Discussed Q2 plan"));
        assert!(md.contains("## Transcript"));
        assert!(md.contains("[00:00] Speaker 1: Hello world"));
        // "None" sections are skipped, not emitted as content
        assert!(!md.contains("Decisions\n\nNone"));
    }

    #[test]
    fn markdown_export_handles_empty_sections() {
        let s = Summary {
            id: 1,
            meeting_id: 1,
            key_points: String::new(),
            decisions: String::new(),
            action_items: String::new(),
            created_at: Utc::now(),
        };
        let md = render_meeting_markdown(&meeting(), None, Some(&s));
        assert!(!md.contains("### Key Points"));
        assert!(!md.contains("### Decisions"));
        assert!(!md.contains("### Action Items"));
    }

    #[test]
    fn text_export_has_plain_sections() {
        let text = render_meeting_text(&meeting(), Some(&transcript()), Some(&summary()));
        assert!(text.contains("--- Summary ---"));
        assert!(text.contains("--- Transcript ---"));
        assert!(text.contains("Alice: send notes"));
    }

    #[test]
    fn duration_formatting() {
        assert_eq!(format_duration(65), "1m 05s");
        assert_eq!(format_duration(3605), "1h 0m 05s");
        assert_eq!(format_duration(30), "30s");
    }

    #[test]
    fn slug_handles_lowercase_and_separators() {
        assert_eq!(export_slug("Weekly Sync"), "weekly-sync");
        // Adjacent separators collapse to multiple dashes; only edges are trimmed.
        assert_eq!(export_slug("One-on-one: Carol"), "one-on-one--carol");
        assert_eq!(export_slug("  Padding Test  "), "padding-test");
    }

    #[test]
    fn slug_keeps_unicode_alphanumerics_and_replaces_symbols() {
        // 'é' and CJK are alphanumeric; emoji and em-dash are not (so ☕ and —
        // each become '-', and the surrounding spaces add more).
        assert_eq!(export_slug("Café ☕ Q3 Roadmap — 日本語"), "café---q3-roadmap---日本語");
    }

    #[test]
    fn slug_falls_back_to_meeting_when_empty() {
        assert_eq!(export_slug(""), "meeting");
        assert_eq!(export_slug("!!!---???"), "meeting");
    }

    #[test]
    fn filename_stamps_utc_date_with_ext() {
        let m = meeting();
        assert_eq!(export_filename(&m, "md"), "weekly-sync-20260506.md");
        assert_eq!(export_filename(&m, "txt"), "weekly-sync-20260506.txt");
    }
}
