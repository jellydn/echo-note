//! Windowed WAV normalization, independent of model inference and Tauri events.

use super::{pcm_i16_to_mono_f32, resample_audio, MAX_AUDIO_SAMPLES};
use anyhow::Result;
use hound::WavReader;
use std::io::{BufWriter, Read, Write};

/// A failed optional spool never interrupts the inference callback or causes
/// a full-recording RAM fallback. Returning owns the spool, not decoder buffers.
pub(super) fn decode_audio<R: Read, W: Write>(
    mut reader: WavReader<R>,
    spool: Option<W>,
    mut visit: impl FnMut(&[f32], usize, usize, f64) -> Result<()>,
) -> Result<Option<W>> {
    let spec = reader.spec();
    let channels = spec.channels;
    let window_source =
        ((MAX_AUDIO_SAMPLES as f64 * f64::from(spec.sample_rate) / 16000.0) as usize).max(1);
    let total_windows = ((f64::from(reader.duration()) * 16000.0 / f64::from(spec.sample_rate))
        / MAX_AUDIO_SAMPLES as f64)
        .ceil()
        .max(1.0) as usize;
    let mut raw_window = Vec::with_capacity(window_source * usize::from(channels));
    let mut samples_iter = reader.samples::<i16>();
    let mut spool = spool.map(|writer| BufWriter::with_capacity(64 * 1024, writer));
    let mut offset = 0.0;
    let mut index = 0;

    loop {
        raw_window.clear();
        for _ in 0..window_source * usize::from(channels) {
            match samples_iter.next() {
                Some(Ok(sample)) => raw_window.push(sample),
                Some(Err(e)) => {
                    log::warn!("Skipping unreadable WAV sample: {}", e);
                    break;
                }
                None => break,
            }
        }
        if raw_window.is_empty() {
            break;
        }
        let mono = pcm_i16_to_mono_f32(&raw_window, channels);
        let mono_16k = if spec.sample_rate != 16000 {
            resample_audio(&mono, spec.sample_rate, 16000)
        } else {
            mono
        };
        if let Some(writer) = spool.as_mut() {
            if mono_16k
                .iter()
                .try_for_each(|s| writer.write_all(&s.to_le_bytes()))
                .is_err()
            {
                log::warn!("Diarization temp-write failed; keeping default speaker labels");
                spool = None;
            }
        }
        visit(&mono_16k, index, total_windows, offset)?;
        offset += mono_16k.len() as f64 / 16000.0;
        index += 1;
    }
    Ok(spool.and_then(|mut writer| {
        if writer.flush().is_err() {
            log::warn!("Diarization temp-flush failed; keeping default speaker labels");
            return None;
        }
        match writer.into_inner() {
            Ok(writer) => Some(writer),
            Err(_) => {
                log::warn!("Diarization temp-flush failed; keeping default speaker labels");
                None
            }
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diarization::{diarize, Embedder, SegmentSpan};
    use std::io::{BufWriter, Seek, SeekFrom};

    struct MarkerEmbedder;

    impl Embedder for MarkerEmbedder {
        fn embed(&self, audio: &[f32]) -> Result<Vec<f32>> {
            assert_eq!(audio.len(), 16_000);
            assert!(audio.iter().all(|&v| v == 0.25));
            Ok(vec![1.0, 0.0])
        }
    }

    fn stereo_fixture(frames: usize) -> std::fs::File {
        let mut file = tempfile::tempfile().unwrap();
        {
            let mut wav = hound::WavWriter::new(
                BufWriter::new(&mut file),
                hound::WavSpec {
                    channels: 2,
                    sample_rate: 48_000,
                    bits_per_sample: 16,
                    sample_format: hound::SampleFormat::Int,
                },
            )
            .unwrap();
            for frame in 0..frames {
                let (left, right) = if frame < 1_440_000 {
                    (8192i16, -4096i16)
                } else {
                    (-16384, 0)
                };
                wav.write_sample(left).unwrap();
                wav.write_sample(right).unwrap();
            }
            wav.finalize().unwrap();
        }
        file.seek(SeekFrom::Start(0)).unwrap();
        file
    }

    #[test]
    fn normalized_spool_preserves_empty_short_and_partial_stereo_windows() {
        for (frames, samples, count) in [(0, 0, 0), (1, 1, 1), (1_440_003, 480_001, 2)] {
            let mut input = stereo_fixture(frames);
            let mut visits = 0;
            let mut output = decode_audio(
                WavReader::new(&mut input).unwrap(),
                Some(tempfile::tempfile().unwrap()),
                |pcm, index, total, offset| {
                    assert_eq!(total, count.max(1));
                    assert_eq!(index, visits);
                    assert_eq!(offset, if index == 0 { 0.0 } else { 30.0 });
                    assert_eq!(
                        pcm.len(),
                        if frames == 1 || index == 1 {
                            1
                        } else {
                            480_000
                        }
                    );
                    let expected = if index == 0 { 0.0625 } else { -0.25 };
                    assert!(pcm.iter().all(|&v| v == expected));
                    visits += 1;
                    Ok(())
                },
            )
            .unwrap()
            .unwrap();
            assert_eq!(visits, count);
            assert_eq!(output.metadata().unwrap().len(), samples * 4);
            output.seek(SeekFrom::Start(0)).unwrap();
            for i in 0..samples {
                let mut bytes = [0; 4];
                output.read_exact(&mut bytes).unwrap();
                assert_eq!(
                    f32::from_le_bytes(bytes),
                    if i < 480_000 { 0.0625 } else { -0.25 }
                );
            }
            input.seek(SeekFrom::Start(0)).unwrap();
            assert_eq!(WavReader::new(input).unwrap().duration(), frames as u32);
        }
    }

    #[test]
    fn spool_failure_or_absence_keeps_all_inference_windows() {
        struct FailingWriter {
            flush_fails: bool,
        }
        impl Write for FailingWriter {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                if !self.flush_fails {
                    return Err(std::io::Error::other("disk full"));
                }
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Err(std::io::Error::other("flush failed"))
            }
        }
        for spool in [
            None,
            Some(FailingWriter { flush_fails: false }),
            Some(FailingWriter { flush_fails: true }),
        ] {
            let mut transcript = String::new();
            let output = decode_audio(
                WavReader::new(stereo_fixture(1_440_003)).unwrap(),
                spool,
                |_, index, _, _| {
                    transcript.push_str(if index == 0 {
                        "first window "
                    } else {
                        "last window"
                    });
                    Ok(())
                },
            )
            .unwrap();
            assert!(output.is_none());
            assert_eq!(transcript, "first window last window");
        }
    }

    #[test]
    fn inference_error_drops_the_spool() {
        use std::sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        };
        struct DropWriter(Arc<AtomicBool>);
        impl Write for DropWriter {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        impl Drop for DropWriter {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }
        let dropped = Arc::new(AtomicBool::new(false));
        let result = decode_audio(
            WavReader::new(stereo_fixture(1)).unwrap(),
            Some(DropWriter(dropped.clone())),
            |_, _, _, _| anyhow::bail!("inference failed"),
        );
        assert!(result.is_err());
        assert!(dropped.load(Ordering::SeqCst));
    }

    #[cfg(unix)]
    #[test]
    fn spools_are_anonymous_and_independent() {
        use std::os::unix::fs::MetadataExt;
        let mut first = decode_audio(
            WavReader::new(stereo_fixture(1)).unwrap(),
            Some(tempfile::tempfile().unwrap()),
            |_, _, _, _| Ok(()),
        )
        .unwrap()
        .unwrap();
        let second = decode_audio(
            WavReader::new(stereo_fixture(0)).unwrap(),
            Some(tempfile::tempfile().unwrap()),
            |_, _, _, _| Ok(()),
        )
        .unwrap()
        .unwrap();
        assert_eq!(first.metadata().unwrap().nlink(), 0);
        assert_eq!(second.metadata().unwrap().nlink(), 0);
        first.seek(SeekFrom::Start(0)).unwrap();
        let mut bytes = [0; 4];
        first.read_exact(&mut bytes).unwrap();
        assert_eq!(f32::from_le_bytes(bytes), 0.0625);
        assert_eq!(second.metadata().unwrap().len(), 0);
    }

    /// Run in separate processes under /usr/bin/time -l on macOS. The fixture,
    /// production decoder and fixed three segments exclude models and metadata
    /// growth from this audio-buffer-only memory experiment.
    #[test]
    #[ignore = "controlled audio-path RSS probe; set ECHO_AUDIO_SECONDS=300 or 7200"]
    fn audio_memory_probe() {
        let seconds: usize = std::env::var("ECHO_AUDIO_SECONDS")
            .unwrap()
            .parse()
            .unwrap();
        assert!([300, 7200].contains(&seconds));
        let mut input = tempfile::tempfile().unwrap();
        {
            let mut wav = hound::WavWriter::new(
                BufWriter::new(&mut input),
                hound::WavSpec {
                    channels: 1,
                    sample_rate: 16_000,
                    bits_per_sample: 16,
                    sample_format: hound::SampleFormat::Int,
                },
            )
            .unwrap();
            for _ in 0..seconds * 16_000 {
                wav.write_sample(8192i16).unwrap();
            }
            wav.finalize().unwrap();
        }
        input.seek(SeekFrom::Start(0)).unwrap();
        let mut windows = 0;
        let mut audio = decode_audio(
            WavReader::new(input).unwrap(),
            Some(tempfile::tempfile().unwrap()),
            |pcm, _, _, _| {
                assert!(pcm.len() <= 480_000);
                assert!(pcm.iter().all(|&v| v == 0.25));
                windows += 1;
                Ok(())
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(audio.metadata().unwrap().len(), seconds as u64 * 64_000);
        let segments = [
            SegmentSpan {
                start_seconds: 0.0,
                end_seconds: 1.0,
            },
            SegmentSpan {
                start_seconds: 29.5,
                end_seconds: 30.5,
            },
            SegmentSpan {
                start_seconds: seconds as f64 - 1.0,
                end_seconds: seconds as f64,
            },
        ];
        assert_eq!(
            diarize(&mut audio, &segments, &MarkerEmbedder, 0.75).unwrap(),
            ["Speaker 1", "Speaker 1", "Speaker 1"]
        );
        assert_eq!(windows, seconds / 30);
        std::hint::black_box(audio);
        println!("audio_memory_probe seconds={seconds} windows={windows} segments=3 max_pcm_samples=480000");
    }
}
