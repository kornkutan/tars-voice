use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn main() {
    let host = cpal::default_host();
    let device = host.default_input_device().expect("no input device");
    println!("device: {}", device.name().unwrap());
    let supported = device.default_input_config().unwrap();
    println!("config: {:?}", supported);
    let config: cpal::StreamConfig = supported.into();
    let count = Arc::new(Mutex::new(0usize));
    let peak = Arc::new(Mutex::new(0.0f32));
    let (c2, p2) = (count.clone(), peak.clone());
    let stream = device
        .build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                *c2.lock().unwrap() += data.len();
                let m = data.iter().fold(0.0f32, |m, s| m.max(s.abs()));
                if m > *p2.lock().unwrap() { *p2.lock().unwrap() = m; }
            },
            |err| eprintln!("stream error: {err}"),
            None,
        )
        .expect("build stream failed");
    stream.play().expect("play failed");
    std::thread::sleep(Duration::from_secs(3));
    drop(stream);
    println!("frames: {} peak: {:.4}", *count.lock().unwrap(), *peak.lock().unwrap());
    if *count.lock().unwrap() == 0 {
        println!("RESULT: callback never fired -> TCC blocking (responsible process lacks mic grant)");
    } else if *peak.lock().unwrap() < 0.001 {
        println!("RESULT: silence -> mic denied (macOS delivers zero buffers) or muted input");
    } else {
        println!("RESULT: audio captured OK");
    }
}
