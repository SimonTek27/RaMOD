//! S6.4 — Runtime hosts Live Update server; optional auto-client smoke.
//!
//! ```bash
//! cargo run -p ramod_runtime --example live_update_demo --features live-update
//! ```

#[cfg(feature = "live-update")]
fn main() {
    use ramod_runtime::{
        live_update::{LiveUpdateClient, LiveUpdateServer},
        EventDescription, PlayParams, Runtime, RuntimeConfig,
    };
    use std::thread;
    use std::time::Duration;

    let mut rt = Runtime::new(RuntimeConfig {
        open_device: false,
        ..Default::default()
    })
    .expect("runtime");

    // Demo tone event
    let rate = 48_000u32;
    let pcm: Vec<f32> = (0..rate)
        .map(|i| {
            let t = i as f32 / rate as f32;
            (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.2
        })
        .collect();
    let buf = rt.load_pcm(pcm, 1, rate).unwrap();
    rt.register_event(
        EventDescription::new("event:/sfx/beep", buf).with_parameter("Intensity", 1.0),
    );
    let _ = rt.play_event("event:/sfx/beep");

    let mut server = LiveUpdateServer::bind_default().expect("bind");
    println!("Live Update listening on {}", server.local_addr());

    // Background client smoke after 200ms
    thread::spawn(|| {
        thread::sleep(Duration::from_millis(200));
        match LiveUpdateClient::connect("127.0.0.1:17420") {
            Ok(mut c) => {
                let _ = c.set_param("event:/sfx/beep", "Intensity", 0.5);
                let _ = c.set_bus_volume("Master", -6.0);
                match c.ping() {
                    Ok(p) => println!("client ping → {}", p.trim()),
                    Err(e) => eprintln!("ping failed: {e}"),
                }
            }
            Err(e) => eprintln!("client connect failed: {e}"),
        }
    });

    for _ in 0..100 {
        let _ = server.poll(&mut rt);
        rt.tick(0.01);
        let mut out = vec![0.0f32; 480];
        rt.render(&mut out);
        thread::sleep(Duration::from_millis(10));
    }
    println!(
        "demo done; clients seen ≈ {}",
        server.client_count().max(0)
    );
}

#[cfg(not(feature = "live-update"))]
fn main() {
    eprintln!("enable feature live-update");
}
