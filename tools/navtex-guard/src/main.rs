//! NAVTEX guard: watch 518 kHz for the SITOR-B FSK carrier and capture raw
//! CF32 while it is present, streaming to disk so a long slot is not held in
//! memory.
//!
//! The RSP1 is tuned to 516.29 kHz so the 518 kHz coast-station carrier sits at
//! +1710 Hz in the I/Q baseband (the same USB offset the program's preset uses).
//! A single-bin DFT at 1710 Hz is the activity detector: the carrier is a steady
//! tone, so it rises far above the noise the moment a transmission starts, and
//! the phasing signal that opens every NAVTEX message keeps it there.
//!
//! Usage: navtex-guard <out.bin>
//!
//! Writes `<out.bin>` with interleaved f32 I, f32 Q at the rate the radio
//! actually runs. Pre-rolls ~5 s so the start of the transmission is not cut.

use std::io::{BufWriter, Write};
use std::time::Instant;

use num_complex::Complex32;
use soapysdr::{Device, Direction};

const CENTER_HZ: f64 = 516_290.0; // 518 kHz dials 1.71 kHz below, USB
const ASKED_RATE: f64 = 2_048_000.0;
const TARGET_HZ: f64 = 1710.0; // NAVTEX carrier in the baseband
const BLOCK: usize = 65_536; // 32 ms at 2.048 Msps
const TRIGGER_BLOCKS: u32 = 3; // persist ~100 ms before it counts
const DROP_BLOCKS: u32 = 30; // ~1 s below the gate ends the run
const RECORD_S: f64 = 900.0; // 15 min of transmission
const MAX_RUN_S: f64 = 3.0 * 3600.0; // give up after 3 h
const PRE_ROLL_S: f64 = 5.0; // keep this much before the trigger
const WARMUP_S: f64 = 30.0; // measure the floor before any trigger
const TRIGGER_X: f32 = 8.0; // carrier must be this many × the floor

/// Power at `target` Hz in one block, normalised per sample, via a single-bin
/// DFT. The I/Q sign convention does not matter: the magnitude is the same.
fn tone_power(x: &[Complex32], rate: f64, target: f64) -> f32 {
    let w = -2.0 * std::f64::consts::PI * target / rate;
    let (mut re, mut im) = (0.0f64, 0.0f64);
    for (i, s) in x.iter().enumerate() {
        let ph = w * i as f64;
        re += f64::from(s.re) * ph.cos() - f64::from(s.im) * ph.sin();
        im += f64::from(s.re) * ph.sin() + f64::from(s.im) * ph.cos();
    }
    let n = x.len() as f64;
    ((re * re + im * im) / (n * n)) as f32
}

fn main() {
    let out = std::env::args().nth(1).expect("usage: navtex-guard <out.bin>");

    let dev = Device::new("driver=sdrplay").expect("open the radio");
    dev.set_sample_rate(Direction::Rx, 0, ASKED_RATE).expect("sample rate");
    dev.set_frequency(Direction::Rx, 0, CENTER_HZ, ()).expect("frequency");
    if dev.has_gain_mode(Direction::Rx, 0).unwrap_or(false) {
        let _ = dev.set_gain_mode(Direction::Rx, 0, false);
    }
    dev.set_gain(Direction::Rx, 0, 40.0).expect("gain");

    let rate = dev.sample_rate(Direction::Rx, 0).expect("read the rate back");
    let freq = dev.frequency(Direction::Rx, 0).expect("read the frequency back");
    eprintln!("navtex-guard: {rate:.0} S/s centred {:.4} MHz, watching {TARGET_HZ} Hz", freq / 1e6);

    let mut stream = dev.rx_stream::<Complex32>(&[0]).expect("stream");
    stream.activate(None).expect("activate");

    // Pre-roll ring: keep the last PRE_ROLL_S of samples, written out on trigger.
    let pre = (rate * PRE_ROLL_S) as usize;
    let mut ring: Vec<Complex32> = Vec::with_capacity(pre);
    let mut ring_at = 0usize;
    let mut ring_full = false;

    let mut file: Option<BufWriter<std::fs::File>> = None;
    let mut recorded = 0u64;
    let mut on_blocks = 0u32;
    let mut off_blocks = 0u32;
    let mut floor = 0.0f32; // the quiet-channel power, measured then tracked
    let mut floor_n = 0u32;
    let mut warmed = false;

    let start = Instant::now();
    let mut chunk = vec![Complex32::new(0.0, 0.0); BLOCK];

    loop {
        if start.elapsed().as_secs_f64() > MAX_RUN_S && file.is_none() {
            eprintln!("navtex-guard: gave up after {} h with no transmission", MAX_RUN_S / 3600.0);
            break;
        }
        let n = {
            let mut bufs: Vec<&mut [Complex32]> = vec![&mut chunk];
            stream.read(&mut bufs, 5_000_000).expect("read")
        };
        if n == 0 {
            continue;
        }
        let x = &chunk[..n];
        let p = tone_power(x, rate, TARGET_HZ);

        // Warm-up: average the floor for WARMUP_S, then track it slowly so a
        // drifting noise level does not keep it from following. A carrier is
        // only a trigger once the floor is known, or the very first quiet block
        // would read as a signal.
        if !warmed {
            floor += p;
            floor_n += 1;
            if start.elapsed().as_secs_f64() >= WARMUP_S {
                floor /= floor_n.max(1) as f32;
                warmed = true;
                eprintln!("navtex-guard: warmed up, floor {floor:.6}");
            }
        } else if file.is_none() {
            floor = floor * 0.999 + p * 0.001;
        }

        let on = warmed && p > floor * TRIGGER_X;

        // Fill the pre-roll ring always.
        for &s in x {
            if ring.len() < pre {
                ring.push(s);
            } else {
                ring[ring_at] = s;
                ring_at = (ring_at + 1) % pre;
                ring_full = true;
            }
        }

        if file.is_some() {
            // Write this block out.
            let mut raw = Vec::with_capacity(n * 8);
            for s in x {
                raw.extend_from_slice(&s.re.to_le_bytes());
                raw.extend_from_slice(&s.im.to_le_bytes());
            }
            file.as_mut().unwrap().write_all(&raw).expect("write");
            recorded += n as u64;
        }

        if file.is_none() {
            if on {
                on_blocks += 1;
                if on_blocks >= TRIGGER_BLOCKS {
                    eprintln!(
                        "navtex-guard: triggered at power {p:.6} (floor {floor:.6}) after {} s",
                        start.elapsed().as_secs_f64()
                    );
                    let f = std::fs::File::create(&out).expect("create");
                    let mut w = BufWriter::with_capacity(1 << 20, f);
                    // Write the pre-roll first.
                    if ring_full {
                        for k in 0..ring.len() {
                            let s = ring[(ring_at + k) % ring.len()];
                            w.write_all(&s.re.to_le_bytes()).unwrap();
                            w.write_all(&s.im.to_le_bytes()).unwrap();
                        }
                    } else {
                        for s in &ring {
                            w.write_all(&s.re.to_le_bytes()).unwrap();
                            w.write_all(&s.im.to_le_bytes()).unwrap();
                        }
                    }
                    file = Some(w);
                    recorded = 0;
                    off_blocks = 0;
                }
            } else {
                on_blocks = 0;
            }
        } else {
            if on {
                off_blocks = 0;
            } else {
                off_blocks += 1;
                if off_blocks >= DROP_BLOCKS {
                    eprintln!(
                        "navtex-guard: carrier gone after {} s, recorded {:.1} s",
                        start.elapsed().as_secs_f64(),
                        recorded as f64 / rate,
                    );
                    break;
                }
            }
            if recorded as f64 / rate >= RECORD_S {
                eprintln!("navtex-guard: hit the {RECORD_S} s record limit");
                break;
            }
        }
    }

    if let Some(w) = file.as_mut() {
        w.flush().ok();
    }
    eprintln!(
        "navtex-guard: done after {:.1} s, {} bytes -> {out}",
        start.elapsed().as_secs_f64(),
        std::fs::metadata(&out).map_or(0, |m| m.len()),
    );
}
