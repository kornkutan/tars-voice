use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream, StreamConfig};
use std::sync::{Arc, Mutex};

pub struct Recorder {
    /// Keep the stream alive while recording; dropping stops it.
    _stream: Stream,
    samples: Arc<Mutex<Vec<f32>>>,
    native_rate: u32,
    channels: u16,
}

pub fn start() -> Result<Recorder> {
    let host = cpal::default_host();
    let device: Device = host
        .default_input_device()
        .ok_or_else(|| anyhow!("no audio input device found"))?;
    let supported = device.default_input_config()?;
    eprintln!(
        "[tars-voice] input device: {} ({:?} {}ch)",
        device.name().unwrap_or_else(|_| "unknown".into()),
        supported.sample_format(),
        supported.channels()
    );
    let rate = supported.sample_rate().0;
    let channels = supported.channels();
    let fmt = supported.sample_format();
    if !matches!(fmt, cpal::SampleFormat::F32) {
        anyhow::bail!("unsupported input sample format: {fmt:?} (need f32)");
    }

    let config: StreamConfig = supported.into();
    let samples = Arc::new(Mutex::new(Vec::<f32>::new()));
    let buf = samples.clone();

    let stream = device.build_input_stream(
        &config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            buf.lock().unwrap().extend_from_slice(data);
        },
        |err| eprintln!("[tars-voice] audio error: {err}"),
        None,
    )?;
    stream.play()?;

    Ok(Recorder {
        _stream: stream,
        samples,
        native_rate: rate,
        channels,
    })
}

impl Recorder {
    /// Stop capture and return 16kHz mono samples for whisper.
    pub fn finish(self) -> Vec<f32> {
        drop(self._stream);
        let raw = self.samples.lock().unwrap().clone();
        let peak = raw.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        eprintln!(
            "[tars-voice] captured {} frames @ {}Hz {}ch, peak {peak:.4}",
            raw.len(),
            self.native_rate,
            self.channels
        );
        if raw.is_empty() {
            return Vec::new();
        }
        let mono = downmix(&raw, self.channels as usize);
        resample_to_16k(&mono, self.native_rate)
    }
}

fn downmix(raw: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return raw.to_vec();
    }
    raw.chunks(channels)
        .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
        .collect()
}

/// Moving-average anti-alias followed by linear interpolation.
/// Good enough for 16kHz speech recognition; not studio grade.
fn resample_to_16k(input: &[f32], from: u32) -> Vec<f32> {
    const TARGET: u32 = 16_000;
    if from == TARGET || input.len() < 2 {
        return input.to_vec();
    }
    let decim = (from / TARGET).max(1) as usize;
    let filtered: Vec<f32> = if decim > 1 {
        input
            .windows(decim)
            .map(|w| w.iter().sum::<f32>() / decim as f32)
            .collect()
    } else {
        input.to_vec()
    };
    if filtered.len() < 2 {
        return filtered;
    }
    let step = from as f64 / TARGET as f64;
    let n_out = ((filtered.len() - 1) as f64 / step).floor() as usize + 1;
    (0..n_out)
        .map(|i| {
            let pos = i as f64 * step;
            let i0 = pos.floor() as usize;
            let i1 = (i0 + 1).min(filtered.len() - 1);
            let t = pos - i0 as f64;
            filtered[i0] * (1.0 - t) as f32 + filtered[i1] * t as f32
        })
        .collect()
}
