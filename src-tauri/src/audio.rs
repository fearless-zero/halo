use crate::types::{AudioDevice, AudioLevel, Settings};
use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, Sample, SampleFormat};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

const TARGET_RATE: u32 = 16_000;

struct CaptureData {
    samples: Vec<f32>,
    sample_rate: u32,
}

pub struct Recorder {
    stop: Arc<AtomicBool>,
    handles: Vec<JoinHandle<()>>,
    captures: Vec<Arc<Mutex<CaptureData>>>,
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// Enumerate available capture devices for the settings UI.
pub fn list_inputs() -> Vec<AudioDevice> {
    let host = cpal::default_host();
    let default_name = host.default_input_device().and_then(|d| d.name().ok());
    let mut out = Vec::new();
    if let Ok(devs) = host.input_devices() {
        for d in devs {
            if let Ok(name) = d.name() {
                out.push(AudioDevice {
                    is_default: Some(&name) == default_name.as_ref(),
                    id: name.clone(),
                    name,
                });
            }
        }
    }
    out
}

fn pick_input(host: &Host, id: Option<&str>) -> Option<Device> {
    if let Some(id) = id {
        if let Ok(devs) = host.input_devices() {
            for d in devs {
                if d.name().map(|n| n == id).unwrap_or(false) {
                    return Some(d);
                }
            }
        }
    }
    host.default_input_device()
}

/// Find a device that carries the system's own output (the other side of a
/// call). On Linux these are PulseAudio/PipeWire ".monitor" sources; on Windows
/// "Stereo Mix"/"What U Hear"; on macOS a virtual loopback device such as
/// BlackHole or an Aggregate device.
fn pick_system(host: &Host) -> Option<Device> {
    const PATTERNS: [&str; 8] = [
        "monitor", "stereo mix", "blackhole", "aggregate", "loopback", "soundflower",
        "what u hear", "wave out",
    ];
    if let Ok(devs) = host.input_devices() {
        for d in devs {
            if let Ok(name) = d.name() {
                let n = name.to_lowercase();
                if PATTERNS.iter().any(|p| n.contains(p)) {
                    return Some(d);
                }
            }
        }
    }
    None
}

fn build_and_run<T>(
    device: &Device,
    config: &cpal::StreamConfig,
    channels: usize,
    is_primary: bool,
    app: AppHandle,
    stop: Arc<AtomicBool>,
    data: Arc<Mutex<CaptureData>>,
) -> Result<()>
where
    T: cpal::SizedSample + Send + 'static,
    f32: cpal::FromSample<T>,
{
    let last_emit = Arc::new(AtomicU64::new(0));
    let ch = channels.max(1);
    let err_fn = |e| eprintln!("audio stream error: {e}");

    let stream = device.build_input_stream(
        config,
        move |input: &[T], _: &cpal::InputCallbackInfo| {
            let mut mono = Vec::with_capacity(input.len() / ch);
            let mut peak = 0f32;
            let mut sumsq = 0f32;
            for frame in input.chunks(ch) {
                let mut acc = 0f32;
                for &x in frame {
                    acc += f32::from_sample(x);
                }
                let v = acc / ch as f32;
                mono.push(v);
                let a = v.abs();
                if a > peak {
                    peak = a;
                }
                sumsq += v * v;
            }
            if is_primary && !mono.is_empty() {
                let rms = (sumsq / mono.len() as f32).sqrt();
                let now = now_ms();
                if now.saturating_sub(last_emit.load(Ordering::Relaxed)) > 66 {
                    last_emit.store(now, Ordering::Relaxed);
                    let _ = app.emit(
                        "recording-level",
                        AudioLevel { rms: (rms * 4.0).min(1.0), peak: peak.min(1.0) },
                    );
                }
            }
            if let Ok(mut d) = data.lock() {
                d.samples.extend_from_slice(&mono);
            }
        },
        err_fn,
        None,
    )?;

    stream.play()?;
    while !stop.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(50));
    }
    // `stream` is dropped here, stopping capture.
    Ok(())
}

fn run_capture(
    device: Device,
    is_primary: bool,
    app: AppHandle,
    stop: Arc<AtomicBool>,
    data: Arc<Mutex<CaptureData>>,
) -> Result<()> {
    let supported = device.default_input_config()?;
    let sample_format = supported.sample_format();
    let channels = supported.channels() as usize;
    if let Ok(mut d) = data.lock() {
        d.sample_rate = supported.sample_rate().0;
    }
    let config: cpal::StreamConfig = supported.into();
    match sample_format {
        SampleFormat::F32 => build_and_run::<f32>(&device, &config, channels, is_primary, app, stop, data),
        SampleFormat::I16 => build_and_run::<i16>(&device, &config, channels, is_primary, app, stop, data),
        SampleFormat::U16 => build_and_run::<u16>(&device, &config, channels, is_primary, app, stop, data),
        other => Err(anyhow!("unsupported sample format: {other:?}")),
    }
}

impl Recorder {
    pub fn start(app: &AppHandle, settings: &Settings) -> Result<Recorder> {
        let host = cpal::default_host();
        let stop = Arc::new(AtomicBool::new(false));
        let mut handles = Vec::new();
        let mut captures: Vec<Arc<Mutex<CaptureData>>> = Vec::new();

        let mic_device = if settings.capture_microphone {
            pick_input(&host, settings.input_device_id.as_deref())
        } else {
            None
        };
        let system_device = if settings.capture_system_audio {
            pick_system(&host)
        } else {
            None
        };
        let mic_present = mic_device.is_some();

        let mut spawn_source = |device: Device, is_primary: bool| {
            let data = Arc::new(Mutex::new(CaptureData { samples: Vec::new(), sample_rate: TARGET_RATE }));
            captures.push(data.clone());
            let app = app.clone();
            let stop = stop.clone();
            handles.push(std::thread::spawn(move || {
                if let Err(e) = run_capture(device, is_primary, app, stop, data) {
                    eprintln!("capture failed: {e}");
                }
            }));
        };

        if let Some(dev) = mic_device {
            spawn_source(dev, true);
        }
        if let Some(dev) = system_device {
            spawn_source(dev, !mic_present);
        }
        drop(spawn_source);

        if captures.is_empty() {
            stop.store(true, Ordering::Relaxed);
            return Err(anyhow!(
                "No audio input available. Enable a microphone or a system-audio device."
            ));
        }

        Ok(Recorder { stop, handles, captures })
    }

    /// Stop capture, mix all sources into a 16 kHz mono WAV, and return the
    /// recording duration in seconds.
    pub fn stop(self, wav_path: &Path) -> Result<f64> {
        self.stop.store(true, Ordering::Relaxed);
        for h in self.handles {
            let _ = h.join();
        }
        let mixed = mix(&self.captures);
        write_wav(wav_path, &mixed)?;
        Ok(mixed.len() as f64 / TARGET_RATE as f64)
    }

    pub fn cancel(self) {
        self.stop.store(true, Ordering::Relaxed);
        for h in self.handles {
            let _ = h.join();
        }
    }
}

fn resample_to_target(input: &[f32], from: u32) -> Vec<f32> {
    if from == TARGET_RATE || input.is_empty() {
        return input.to_vec();
    }
    let ratio = TARGET_RATE as f32 / from as f32;
    let out_len = (input.len() as f32 * ratio) as usize;
    let mut out = Vec::with_capacity(out_len);
    let last = input.len() - 1;
    for i in 0..out_len {
        let src = i as f32 / ratio;
        let idx = src.floor() as usize;
        let frac = src - idx as f32;
        let a = input[idx.min(last)];
        let b = input[(idx + 1).min(last)];
        out.push(a + (b - a) * frac);
    }
    out
}

fn mix(captures: &[Arc<Mutex<CaptureData>>]) -> Vec<f32> {
    let resampled: Vec<Vec<f32>> = captures
        .iter()
        .map(|c| {
            let d = c.lock().unwrap();
            resample_to_target(&d.samples, d.sample_rate)
        })
        .collect();
    let len = resampled.iter().map(|r| r.len()).max().unwrap_or(0);
    let sources = resampled.len().max(1) as f32;
    let mut out = vec![0f32; len];
    for r in &resampled {
        for (i, &s) in r.iter().enumerate() {
            out[i] += s;
        }
    }
    for v in out.iter_mut() {
        *v = (*v / sources).clamp(-1.0, 1.0);
    }
    out
}

fn write_wav(path: &Path, samples: &[f32]) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: TARGET_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)?;
    for &s in samples {
        let v = (s * 32767.0).clamp(-32768.0, 32767.0) as i16;
        writer.write_sample(v)?;
    }
    writer.finalize()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capture(samples: Vec<f32>, rate: u32) -> Arc<Mutex<CaptureData>> {
        Arc::new(Mutex::new(CaptureData { samples, sample_rate: rate }))
    }

    #[test]
    fn resample_passthrough_and_empty() {
        assert_eq!(resample_to_target(&[0.1, 0.2], TARGET_RATE), vec![0.1, 0.2]);
        assert!(resample_to_target(&[], 8000).is_empty());
    }

    #[test]
    fn resample_upsamples_length() {
        let out = resample_to_target(&[0.0, 1.0, 0.0, 1.0], 8000);
        assert_eq!(out.len(), 8);
        assert_eq!(out[0], 0.0);
    }

    #[test]
    fn mix_averages_sources_and_clamps() {
        let mixed = mix(&[capture(vec![1.0, -1.0], TARGET_RATE), capture(vec![-1.0, 1.0], TARGET_RATE)]);
        assert_eq!(mixed, vec![0.0, 0.0]);

        // A single loud source is clamped into range.
        let loud = mix(&[capture(vec![2.0, -2.0], TARGET_RATE)]);
        assert_eq!(loud, vec![1.0, -1.0]);
    }

    #[test]
    fn mix_handles_uneven_lengths() {
        let mixed = mix(&[capture(vec![1.0, 1.0], TARGET_RATE), capture(vec![1.0], TARGET_RATE)]);
        assert_eq!(mixed.len(), 2);
        assert_eq!(mixed[0], 1.0); // (1+1)/2
        assert_eq!(mixed[1], 0.5); // (1+0)/2
    }

    #[test]
    fn wav_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.wav");
        write_wav(&path, &[0.0, 0.5, -0.5]).unwrap();
        let reader = hound::WavReader::open(&path).unwrap();
        assert_eq!(reader.spec().sample_rate, TARGET_RATE);
        assert_eq!(reader.spec().channels, 1);
        assert_eq!(reader.into_samples::<i16>().count(), 3);
    }

    #[test]
    fn list_inputs_does_not_panic() {
        // Just ensure enumeration runs on the host without panicking.
        let _ = list_inputs();
    }
}
