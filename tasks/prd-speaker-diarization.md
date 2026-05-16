# PRD: Speaker Diarization for EchoNote

## Overview

**Issue**: [github.com/jellydn/echo-note/issues/15](https://github.com/jellydn/echo-note/issues/15)
**Goal**: Identify who spoke when in multi-speaker meetings.

## Problem

Currently, EchoNote can transcribe meetings via Whisper and extract timestamped segments, but **all segments are labeled "Speaker 1"**. There's no mechanism to distinguish between different speakers in multi-person conversations. This blocks:
- Action item assignment per person
- Differentiated speaker analysis
- Meaningful multi-speaker transcript display

## Solution Architecture

### Approach: Embedding-based Speaker Clustering

Use a lightweight voice embedding model (Qwen3-Voice-Embedding-12Hz-0.6B-onnx) to extract voice embeddings per Whisper segment, then cluster segments by speaker similarity.

```text
Audio File
    │
    ├──► Whisper Transcription ──► Segments with timestamps + text
    │
    └──► Speaker Diarization Pipeline:
              │
              ├─ 1. For each segment: extract audio slice
              ├─ 2. Compute speaker embedding (Qwen3 voice embedding model via ONNX)
              ├─ 3. Agglomerative clustering on embeddings
              └─ 4. Assign "Speaker A", "Speaker B", ... labels
                      │
                      └──► Merge labels with segments ──► Final Transcript
```

### Key Design Decision: ONNX Runtime (ort crate)

| Option | Pros | Cons | Verdict |
|--------|------|------|---------|
| **ONNX Runtime (ort)** | Pure Rust, local, privacy-first, flexible | 30MB runtime size, model download | ✅ Best fit |
| sherpa-onnx | Full pipeline built-in | Heavy dep, complex build | Too heavy for MVP |
| Python (pyannote) | Best accuracy | Requires Python runtime | Violates privacy-first |
| Cloud API | Best accuracy | Privacy leak, cost, latency | Keep as future option |

## Implementation Plan

### Phase 1: Foundation (Rust Backend)

#### Step 1: Add ONNX Runtime dependency

```toml
# src-tauri/Cargo.toml
ort = "2.0"
```

#### Step 2: Speaker Embedding Module (`src-tauri/src/diarization/`)

New module with:

1. **`mod.rs`** — Public API
   - `DiarizationEngine` struct (holds model, audio data)
   - `diarize(audio_path, segments) -> Result<Vec<SegmentWithSpeaker>>`

2. **`embedding.rs`** — Speaker embedding extraction
   - Load Qwen3-Voice-Embedding-12Hz-0.6B-onnx model
   - For each Whisper segment, extract audio slice from WAV file
   - Preprocess audio (normalize, resample to 16kHz if needed)
   - Run ONNX inference → embedding vector (1024 dimensions)
   - Return `Vec<Vec<f32>>` (one embedding per segment)

3. **`clustering.rs`** — Speaker clustering
   - Agglomerative clustering with cosine distance
   - Merge threshold: configurable (default 0.75 similarity)
   - Handles 1-N speakers
   - Returns `Vec<usize>` (cluster assignment per segment)

#### Step 3: Integrate with Whisper Pipeline

Modify `src-tauri/src/whisper/mod.rs`:

- After `transcribe_audio()` returns segments, pass them through diarization
- If diarization succeeds, replace "Speaker 1" labels with detected speakers
- If diarization fails (single speaker, or model not available), fall back to "Speaker 1"

#### Step 4: New Tauri Commands

- `check_diarization_status_command` — Check if diarization model is downloaded
- `download_diarization_model_command` — Download ONNX model with progress

### Phase 2: UI (Frontend)

#### Step 5: Enhanced Transcript Display

Update `MeetingDetailView.tsx`:

- Parse timestamp/segment format from transcript content
- Display each segment with colored speaker badge
- Color mapping: assign stable colors per speaker label
- Collapsible/expandable segments

#### Step 6: Diarization Settings

Add to `SettingsView.tsx`:
- Diarization model status indicator
- Model download button with progress
- Option to enable/disable diarization
- Cluster sensitivity slider

### Phase 3: Polish

#### Step 7: Rename Speakers

- Allow user to rename "Speaker A" → "Alice" in the UI
- Store speaker names in settings/local state
- Persist across meeting viewings

#### Step 8: Performance Optimization

- Cache embeddings for repeated segments
- Parallel embedding extraction for long meetings
- Progress reporting during diarization

## Detailed Technical Design

### Qwen3 Voice Embedding ONNX Model

- **Source**: `Qwen3-Voice-Embedding-12Hz-0.6B-onnx`
- **Input**: 16kHz mono audio, variable length (pad to ~3s or use full segment)
- **Output**: 1024-dimensional speaker embedding vector
- **Size**: ~15-25MB
- **Hosting**: Download from HuggingFace or bundled URLs

### ONNX Inference Flow

```rust
use ort::Session;

struct DiarizationEngine {
    session: Session,
    sample_rate: u32,  // 16000
}

impl DiarizationEngine {
    fn compute_embedding(&self, audio_slice: &[f32]) -> Result<Vec<f32>> {
        // 1. Pad/truncate to expected input length (e.g., 48000 samples = 3s)
        // 2. Normalize audio
        // 3. Run inference
        let outputs = self.session.run(...)?;
        // 4. Extract embedding from output tensor
        Ok(embedding)
    }
}
```

### Clustering Implementation

```rust
fn cluster_speakers(embeddings: &[Vec<f32>], threshold: f32) -> Vec<usize> {
    // 1. Compute cosine similarity matrix
    // 2. Agglomerative clustering:
    //    - Start: each segment is its own cluster
    //    - Iteratively merge closest pair with similarity > threshold
    //    - Stop when no pair exceeds threshold
    // 3. Assign cluster IDs
}
```

### Pipeline Integration

In `transcribe_audio()` (whisper/mod.rs), after segments are collected:

```rust
// After Whisper transcription
let diarization = DiarizationEngine::new(&app_handle)?;
let labeled_segments = diarization.diarize_segments(&audio_data, &transcript_segments)?;

// Replace speaker labels
for (segment, labeled) in transcript_segments.iter_mut().zip(labeled_segments) {
    segment.speaker = labeled.speaker;
}
```

### Database Changes

The transcript `content` field already stores formatted text like:
```text
[00:12] Speaker 1: Hello everyone
[00:15] Speaker 2: Hi, thanks for joining
```

With diarization this becomes:
```text
[00:12] Speaker A: Hello everyone
[00:15] Speaker B: Hi, thanks for joining
```

**No schema changes needed** — the formatted text already carries speaker labels. Diarization just improves which label is assigned.

### Future: Separate segments table (if needed)

If we want richer speaker metadata (colors, confidence, editable names), add a table:

```sql
CREATE TABLE IF NOT EXISTS transcript_segments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    transcript_id INTEGER NOT NULL,
    start_seconds REAL NOT NULL,
    end_seconds REAL NOT NULL,
    speaker_label TEXT NOT NULL DEFAULT 'Speaker 1',
    speaker_confidence REAL DEFAULT 0.0,
    text TEXT NOT NULL,
    FOREIGN KEY (transcript_id) REFERENCES transcripts(id) ON DELETE CASCADE
);
```

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Diarization accuracy | ≥80% (2 speakers) | Manual test with 5 real meeting recordings |
| Processing overhead | ≤2x transcription time | Wall-clock comparison |
| Model size | ≤25MB | File size |
| Model download | User-visible progress bar | Visual verification |

## Test Plan

1. **Unit tests** (Rust):
   - Embedding extraction from known audio
   - Clustering correctness (2 speakers, 3 speakers, 1 speaker)
   - Edge cases: single speaker throughout, overlapping speech (graceful fallback)

2. **Integration tests**:
   - End-to-end: Record → Transcribe → Diarize → Display in UI
   - Model download and caching
   - Fallback when model not available

3. **Manual tests**:
   - 2-person meeting (clear speaker separation)
   - 3-person meeting
   - Single speaker (no false positives)
   - Background noise tolerance
   - Short segments (<1s) handling

## Open Questions/Decisions

1. **Model hosting**: Where to host the ONNX model? HuggingFace releases page? Bundled? User-downloaded?
2. **Embedding dimension**: 1024 (Qwen3 voice embedding model) vs smaller models — tradeoff between accuracy and speed
3. **Segment padding**: How much audio context before/after each segment for reliable embeddings?
4. **Real-time vs post-hoc**: Currently post-hoc (after recording). Future real-time?
5. **Maximum speakers**: Should we limit clustering to N speakers (e.g., 4)?

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| ort crate build complexity | High | Pre-build container, document build deps |
| Model download size (~25MB) | Medium | Progress bar, resume support, chunked download |
| Poor diarization quality | High | Fallback to "Speaker 1", configurable threshold |
| Long processing time | Medium | Parallel embedding extraction, progress reporting |

## Timeline Estimate

- **Phase 1** (Rust diarization engine): 3-4 sessions
- **Phase 2** (UI integration): 2 sessions
- **Phase 3** (Polish + tests): 2 sessions
- **Total**: ~7-8 sessions

## References

- [Qwen3-Voice-Embedding-12Hz-0.6B-onnx](https://huggingface.co/marksverdhei/Qwen3-Voice-Embedding-12Hz-0.6B-onnx)
- [ort crate](https://crates.io/crates/ort) — ONNX Runtime for Rust
- [Whisper segment extraction (already implemented)](https://github.com/jellydn/echo-note/commit/289d9e5)
