//! Where in the interleaver does a recording start?
//!
//!     cargo run --release -p sdroxide-dsp --example s4285_align_check -- \
//!         ~/s4285-captures/keepers/….wav [carrier_hz]
//!
//! A convolutional deinterleaver has a **commutator position**, and a recording
//! that begins mid-transmission begins with that position unknown — one of 32
//! rows. Every check this fork has run so far started it at row zero, which is
//! the transmitter's own start and the case a loopback can test; on an off-air
//! capture it is right one time in thirty-two, and when it is wrong the
//! deinterleaved stream is a cyclic permutation of the right one and the trellis
//! is invisible however good the signal is.
//!
//! So this one sweeps it: for each branch, the commutator is advanced by `p·32`
//! values before the recording is fed, and the first `31·j·32` values of output
//! are discarded so the deinterleaver's zero fill has flushed. The judge is the
//! decoder's own metric against the same values shuffled, as everywhere else.

use sdroxide_dsp::stanag4285::{EqMode, S4285Event, S4285Receiver};
use sdroxide_dsp::stanag4285_fec as fec;

fn read_wav(path: &str) -> (Vec<f32>, f64) {
    let mut reader = hound::WavReader::open(path).expect("open the wav");
    let spec = reader.spec();
    let raw: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.expect("s")).collect(),
        hound::SampleFormat::Int => match spec.bits_per_sample {
            16 => reader.samples::<i16>().map(|s| s.expect("s") as f32 / 32_768.0).collect(),
            _ => reader.samples::<i32>().map(|s| s.expect("s") as f32 / 2_147_483_648.0).collect(),
        },
    };
    let ch = spec.channels.max(1) as usize;
    let audio = if ch == 1 {
        raw
    } else {
        raw.chunks(ch).map(|f| f.iter().sum::<f32>() / ch as f32).collect()
    };
    (audio, spec.sample_rate as f64)
}

fn shuffled(input: &[f32], seed: u32) -> Vec<f32> {
    let mut out = input.to_vec();
    let mut s = seed;
    for i in (1..out.len()).rev() {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        let j = (s as usize) % (i + 1);
        out.swap(i, j);
    }
    out
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a.is_empty() {
        eprintln!("usage: s4285_align_check <wav> [carrier_hz]");
        std::process::exit(2);
    }
    let (audio, rate) = read_wav(&a[0]);
    let carrier: f64 = a.get(1).and_then(|v| v.parse().ok()).unwrap_or(1800.0);

    let mut rx = S4285Receiver::new(rate, carrier);
    rx.set_eq_mode(EqMode::Auto);
    rx.trace_coded_stream(true);
    let mut frames = 0;
    for chunk in audio.chunks(4096) {
        for ev in rx.process(chunk) {
            if let S4285Event::Frame(_) = ev {
                frames += 1;
            }
        }
    }
    let traced: Vec<f32> = rx.coded_stream().to_vec();
    println!("{}: {frames} frames, {} traced values at {carrier}", a[0], traced.len());
    if traced.len() < 20_000 {
        println!("  too little to judge");
        return;
    }

    let mut best: Vec<(f32, String)> = Vec::new();
    for rate in fec::CodedRate::BPSK {
        // One bit per symbol for the BPSK rates: the trace carries two.
        let soft: Vec<f32> = traced.iter().step_by(2).copied().collect();
        let j = rate.j();
        let flush = 31 * j * 32;
        for sign in [1.0f32, -1.0] {
            for style in fec::Interleave::ALL {
                for phase in 0..32usize {
                    let mut di = fec::ConvInterleaver::deinterleaver_with(j, style);
                    for _ in 0..phase * 32 {
                        di.push(0.0);
                    }
                    let mut out: Vec<f32> = Vec::with_capacity(soft.len());
                    for v in &soft {
                        out.push(di.push(v * sign));
                    }
                    if out.len() <= flush + 5_000 {
                        continue;
                    }
                    let body = &out[flush..];
                    let pairs = fec::average_pairs(body, rate.repeat());
                    let (_, metric) = fec::viterbi_decode_with_metric(&pairs);
                    let null = [shuffled(&pairs, 0x1234_5678), shuffled(&pairs, 0x9E37_79B9)]
                        .iter()
                        .map(|s| fec::viterbi_decode_with_metric(s).1)
                        .fold(f32::NEG_INFINITY, f32::max);
                    let ratio = if null > 0.0 { metric / null } else { 0.0 };
                    best.push((
                        ratio,
                        format!(
                            "{} j{j} sign {sign:+.0} {} commutator phase {phase}",
                            rate.label(),
                            style.label()
                        ),
                    ));
                }
            }
        }
    }
    best.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap_or(std::cmp::Ordering::Equal));
    println!("  best twelve of {} branches (a ratio near 1 is no coding):", best.len());
    for (ratio, label) in best.iter().take(12) {
        println!("  {ratio:5.2}x  {label}");
    }
}
