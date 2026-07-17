use crate::audio;
use anyhow::{anyhow, Context, Result};
use std::path::Path;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

const TARGET_RATE: u32 = 16_000;

/// Turn a file name into a human title: `math-lecture_01.m4a` -> "math lecture 01".
pub fn title_from_path(path: &Path) -> String {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let cleaned = stem.replace(['_', '-', '.'], " ");
    let words: Vec<&str> = cleaned.split_whitespace().collect();
    if words.is_empty() {
        "Recording".to_string()
    } else {
        words.join(" ")
    }
}

/// Decode any supported audio file to mono f32 samples plus its sample rate.
/// A format/codec boundary (Symphonia) — exercised by the app, excluded from
/// coverage like the other external-media boundaries.
#[cfg_attr(coverage_nightly, coverage(off))]
fn decode_to_mono(path: &Path) -> Result<(Vec<f32>, u32)> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("cannot open {}", path.display()))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .context("unsupported or unreadable audio file")?;
    let mut format = probed.format;
    let track = format.default_track().ok_or_else(|| anyhow!("no audio track"))?;
    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .context("no decoder available for this audio codec")?;

    let mut samples: Vec<f32> = Vec::new();
    let mut rate = 0u32;

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(_) => break, // end of stream / reset
        };
        if packet.track_id() != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(decoded) => {
                let spec = *decoded.spec();
                rate = spec.rate;
                let channels = spec.channels.count().max(1);
                let mut buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
                buf.copy_interleaved_ref(decoded);
                for frame in buf.samples().chunks(channels) {
                    let sum: f32 = frame.iter().copied().sum();
                    samples.push(sum / channels as f32);
                }
            }
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(_) => break,
        }
    }

    if samples.is_empty() || rate == 0 {
        return Err(anyhow!("no audio samples could be decoded"));
    }
    Ok((samples, rate))
}

/// Decode `src`, resample to 16 kHz mono, and write a WAV to `dst`. Returns the
/// recording duration in seconds. Orchestration over the decode boundary.
#[cfg_attr(coverage_nightly, coverage(off))]
pub fn import_to_wav(src: &Path, dst: &Path) -> Result<f64> {
    let (samples, rate) = decode_to_mono(src)?;
    let resampled = audio::resample_to_target(&samples, rate);
    audio::write_wav(dst, &resampled)?;
    Ok(resampled.len() as f64 / TARGET_RATE as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn title_cleans_separators() {
        assert_eq!(title_from_path(&PathBuf::from("/a/math-lecture_01.m4a")), "math lecture 01");
        assert_eq!(title_from_path(&PathBuf::from("Chemistry Class.wav")), "Chemistry Class");
    }

    #[test]
    fn title_falls_back_when_empty() {
        assert_eq!(title_from_path(&PathBuf::from("/a/___.wav")), "Recording");
        assert_eq!(title_from_path(&PathBuf::from("/")), "Recording");
    }
}
