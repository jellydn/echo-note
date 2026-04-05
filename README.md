# 🎙️ EchoNote

A privacy-first native desktop app that listens to your meetings, transcribes locally, and generates structured summaries — all without data leaving your device.

> Replace Otter.ai and Fireflies.ai with a fully offline, zero-cost alternative. No meeting bots. No cloud dependency.

## Features

- **Audio Recording** — Capture microphone + system audio (Zoom, Meet, etc.) via bundled BlackHole
- **Local Transcription** — Whisper-powered speech-to-text, running entirely on your machine
- **AI Summaries** — Structured output (key points, decisions, action items) via local Ollama
- **Meeting History** — Browse and review past meetings with transcripts and summaries
- **Privacy First** — No data leaves your device by default

## Tech Stack

| Layer | Technology |
|-------|-----------|
| UI | Tauri v2 + React (TypeScript) |
| Backend | Rust |
| Speech-to-Text | whisper-rs (whisper.cpp bindings) |
| LLM | Ollama (LLaMA / Mistral) |
| Storage | SQLite |
| System Audio | BlackHole (bundled) |

## Prerequisites

- **macOS** (Apple Silicon recommended)
- **[Ollama](https://ollama.com)** installed with a model pulled (e.g., `ollama pull mistral`)
- **Rust** toolchain (`rustup`)
- **Node.js** 18+

## Getting Started

```bash
# Clone the repo
git clone <repo-url> echo-note
cd echo-note

# Install frontend dependencies
npm install

# Run in development mode
cargo tauri dev
```

On first launch, the app will prompt you to download the Whisper `small` model (~500MB).

## How It Works

```
Meeting Audio → Record (mic + system) → Save WAV
  → Whisper Transcription → Transcript
    → Ollama Summarization → Key Points / Decisions / Action Items
```

1. **Start recording** — Click the record button, select your mic
2. **Stop recording** — Name your meeting, transcription starts automatically
3. **Generate summary** — One click to get structured notes via your local LLM
4. **Review later** — Browse meeting history, copy transcripts or summaries

## Configuration

All settings are accessible from the in-app Settings page:

- **Audio Device** — Select your preferred microphone
- **Whisper Model** — Choose model size (tiny / base / small / medium)
- **LLM Provider** — Local (Ollama) or optional cloud API (Groq, OpenRouter)

## Project Structure

```
src-tauri/       # Rust backend (audio, Whisper, Ollama, SQLite)
src/             # React frontend (TypeScript)
tasks/           # PRD and planning docs
scripts/ralph/   # Autonomous agent scripts
```

## Roadmap

- [ ] Real-time transcription during recording
- [ ] Speaker diarization (WhisperX)
- [ ] Semantic search across meetings (vector DB)
- [ ] Calendar integration
- [ ] Multi-language support
- [ ] Export to Markdown / PDF

## License

MIT
