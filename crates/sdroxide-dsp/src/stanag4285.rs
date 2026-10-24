//! STANAG 4285 — the receive side, uncoded rates.
//!
//! Transcribed from the standard itself (`docs/internal/STANAG4285-EDITION1.md`) rather than
//! from somebody's description of it, and **receive-only**: no modulator, and the
//! coded rates (Annex E) are not built.
//!
//! The frame, in full, because every one of these matters:
//!
//! * **256 symbols at 2400 baud** = 106.67 ms: **80 synchronisation** symbols,
//!   then 176 scrambled ones — three groups of `32 data + 16 reference`, then a
//!   final 32 data symbols with no reference after them.
//! * The synchronisation is **2PSK**, PN31 (two periods plus the first 18
//!   symbols), generator `x⁵ + x² + 1`, register set to `11010` at the start of
//!   every frame, **first symbol the LSB**, then 79 clocks.
//! * The 176 reference and data symbols are **scrambled** by `x⁹ + x⁴ + 1`, set
//!   to 1 each frame, **three bits per symbol**: `n = 4x₂ + 2x₁ + x₀` and
//!   `Bk = e^(jnπ/4)`, the register advancing three positions per symbol.
//! * The reference symbols are **all symbol 0 before scrambling**, which is the
//!   handle the whole receiver hangs on.
//!
//! ## Acquisition, and why it is a fold and not a peak
//!
//! The synchronisation sequence repeats every frame, so the correlation of the
//! received differential stream against the differentially encoded PN31 is a
//! **periodic** statistic: the right question is not "is there a peak here" but
//! "at which of the 256 symbol phases does this average over *many* frames stand
//! up". Measured on the night's captures: a single peak reaches 0.86 and so does
//! a strong PSK carrier of another mode (0.86 against 0.14 for the control when
//! the correlation is **folded over twelve frames**), while one frame's maximum
//! over every offset reaches 0.738 on white noise.
//!
//! Two more things have to be measured rather than assumed, and both were the
//! difference between 0.28 and 0.70 on the reference blocks:
//!
//! * **the sampling instant.** A receiver that resamples at a fixed phase lands
//!   anywhere inside the symbol. The fold is therefore searched over ten
//!   sub-symbol phases, and each frame refines its own timing again.
//! * **the carrier offset inside a block.** 16 symbols is 6.7 ms, so 150 Hz turns
//!   a whole circle inside one reference block and a perfectly good frame
//!   measures 0.3 instead of 0.9. It is corrected per block by a search, and the
//!   same correction is applied to the data.
//!
//! ## What this deliberately does not claim
//!
//! The three uncoded rates are 1200 BPSK, 2400 QPSK and 3600 8PSK, and **each of
//! those constellations is also used by coded rates** (1200 BPSK carries the
//! 75–600 bps rates at 1/16…1/2, QPSK carries 1200 at 1/2, 8PSK carries 2400 at
//! 2/3). A constellation test cannot tell them apart, so the rate is reported as
//! "uncoded, or a coded rate" and never as a claim. The coded payload needs
//! Annex E's FEC and interleaver, which are not transcribed, so the text of a
//! coded transmission is not readable here — and the payload bits are exposed so
//! that shows as bits rather than as silence.

use crate::rtty::BaudotRx;
use crate::stanag4285_fec as fec;
use std::f32::consts::TAU;

/// Symbol rate, fixed by the standard.
pub const BAUD: f64 = 2400.0;
/// Symbols per frame.
pub const FRAME: usize = 256;
/// Synchronisation symbols at the head of every frame.
pub const SYNC: usize = 80;
/// Scrambled symbols: 176 = 3 × (32 data + 16 reference) + 32 data.
pub const SCRAMBLED: usize = FRAME - SYNC;
/// Samples per symbol at the modem rate.
pub const SPS: usize = 5;
/// The modem's internal sample rate: five samples per symbol.
pub const MODEM_RATE: f64 = BAUD * SPS as f64;
/// The reference blocks, as offsets into the 176 scrambled symbols.
pub const REF_BLOCKS: [(usize, usize); 3] = [(32, 48), (80, 96), (128, 144)];
/// The data blocks, as offsets into the 176 scrambled symbols.
pub const DATA_BLOCKS: [(usize, usize); 4] = [(0, 32), (48, 80), (96, 128), (144, 176)];

/// Frames folded together to acquire, and to keep saying the lock is still there.
const ACQ_FRAMES: usize = 12;
/// Sub-symbol phases the acquisition searches. A tenth of a symbol is 42 µs,
/// and on a real capture the two best phases differed by exactly that.
const PHASES: usize = 10;

/// A complex value as a pair, so this module needs no extra dependency.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct C {
    pub re: f32,
    pub im: f32,
}

impl C {
    pub fn new(re: f32, im: f32) -> Self {
        C { re, im }
    }
    pub fn mul(self, o: C) -> C {
        C::new(self.re * o.re - self.im * o.im, self.re * o.im + self.im * o.re)
    }
    pub fn conj(self) -> C {
        C::new(self.re, -self.im)
    }
    pub fn norm(self) -> f32 {
        (self.re * self.re + self.im * self.im).sqrt()
    }
    pub fn scale(self, k: f32) -> C {
        C::new(self.re * k, self.im * k)
    }
    pub fn add(self, o: C) -> C {
        C::new(self.re + o.re, self.im + o.im)
    }
    pub fn sub(self, o: C) -> C {
        C::new(self.re - o.re, self.im - o.im)
    }
    pub fn from_phase(a: f32) -> C {
        C::new(a.cos(), a.sin())
    }
    pub fn arg(self) -> f32 {
        self.im.atan2(self.re)
    }
    fn lerp(self, o: C, t: f32) -> C {
        C::new(self.re + (o.re - self.re) * t, self.im + (o.im - self.im) * t)
    }
}

/// The data rate a frame carries, as far as its constellation can say.
///
/// Uncoded 3600 and **every coded rate** use all eight phases, so they cannot be
/// told apart at this layer; nor can 1200 BPSK from the 75–600 bps coded rates,
/// nor QPSK from 1200 at 1/2. The report says both rather than guessing, and that
/// is the honest ceiling of a constellation test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum S4285Rate {
    /// 1200 bps, one bit per symbol, 2PSK — or a coded BPSK rate (75–600).
    Bps1200,
    /// 2400 bps, a dibit per symbol, QPSK — or 1200 at rate 1/2.
    Bps2400,
    /// 3600 bps uncoded (8PSK) — or 2400 at rate 2/3.
    Bps3600OrCoded,
    /// Not yet decided — a frame whose references did not collapse.
    Unknown,
}

impl S4285Rate {
    pub fn label(self) -> &'static str {
        match self {
            S4285Rate::Bps1200 => "1200 BPSK uncoded, or a coded BPSK rate",
            S4285Rate::Bps2400 => "2400 QPSK uncoded, or 1200 QPSK rate 1/2",
            S4285Rate::Bps3600OrCoded => "3600 8PSK uncoded, or a coded rate",
            S4285Rate::Unknown => "unknown",
        }
    }

    /// Bits carried per symbol.
    pub fn bits_per_symbol(self) -> usize {
        match self {
            S4285Rate::Bps1200 => 1,
            S4285Rate::Bps2400 => 2,
            _ => 3,
        }
    }
}

/// One demodulated frame.
#[derive(Debug, Clone, PartialEq)]
pub struct S4285Frame {
    /// The three reference blocks' concentrations after descrambling, in order.
    /// **Reported separately and never averaged**: a channel that rotates within
    /// one frame separates the blocks while leaving each coherent, and an average
    /// hides exactly that.
    pub reference: [f32; 3],
    /// Carrier offset the reference blocks agree on, in hertz.
    pub carrier_offset_hz: f32,
    /// Which scrambler sign the frame needed.
    pub reversed: bool,
    /// What the descrambled constellation says.
    pub rate: S4285Rate,
    /// The distinct phases the descrambled data symbols used, 0..8.
    pub phases_used: usize,
    /// How tightly the data symbols sit on the 2-, 4- and 8-point lattices. Each
    /// is `|mean e^(jnφ)|`, which is independent of a rotation. A clean BPSK frame
    /// gives ~1, ~1, ~1; QPSK ~0, ~1, ~1; 8PSK ~0, ~0, ~1. Noise lowers all three,
    /// so the verdict compares them against each other rather than an absolute.
    pub constellation: [f32; 3],
    /// The same concentration, taken **per data block** rather than across the
    /// frame: the best of each block's 2-, 4- and 8-fold figures.
    ///
    /// This is the diagnostic that separates two faults a whole-frame number
    /// cannot. A phase that walks *between* blocks — one that a line through the
    /// three reference points cannot follow — leaves each block coherent and the
    /// frame spread, and shows up here as four figures well above
    /// [`S4285Frame::constellation`]. A phase that moves *inside* a block of 32
    /// symbols leaves even these flat, and only a tracker can follow that.
    pub block: [f32; 4],
    /// Payload bits of this frame, oldest first.
    pub bits: Vec<u8>,
    /// The frame's 128 soft decisions, positive for a zero — the values the
    /// Annex E chains are fed from. Exposed so a probe can ask whether a capture
    /// carries the code at all, which the hard bits cannot answer.
    pub soft: Vec<f32>,
}

/// A message the Annex E layer assembled.
#[derive(Debug, Clone, PartialEq)]
pub struct S4285Message {
    /// Which coded rate produced it — the start-of-message correlation is what
    /// tells the four BPSK rates apart, since their constellations are identical.
    pub rate: fec::CodedRate,
    /// The message bits between the SOM and the EOM, oldest first.
    pub bits: Vec<u8>,
    /// An EOM was actually seen. Otherwise this is whatever had arrived.
    pub closed: bool,
}

/// Something the receiver produced.
#[derive(Debug, Clone, PartialEq)]
pub enum S4285Event {
    /// A frame was demodulated.
    Frame(S4285Frame),
    /// Text from the character layer, ITA2 5N1. The standard's payload is a
    /// synchronous bit stream, so this is a best reading of it — the raw bits are
    /// on the frame event for anything that is not ITA2.
    Text(String),
    /// A coded message came out of the Annex E chain, with the rate that
    /// produced it.
    Message(S4285Message),
}

/// The receiver's standing state, for a panel or a probe.
#[derive(Debug, Clone, PartialEq)]
pub struct S4285Report {
    /// A frame is being tracked.
    pub locked: bool,
    /// The folded differential sync correlation at the lock, 0..1. Folded over
    /// [`ACQ_FRAMES`] frames, which is what makes it a measurement: one frame's
    /// best peak over every offset reaches 0.74 on white noise.
    pub sync_quality: f32,
    /// The same correlation's median over the 256 phases: the null it is read
    /// against. On the night's captures the signal reads 0.85 and the control
    /// 0.14.
    pub correlation_floor: f32,
    /// The frame-period autocorrelation: how much of a frame repeats one frame
    /// later. A correct 4285 measures about **0.31-0.60** — the 80 sync symbols
    /// repeat — while a strong PSK carrier that is a different mode measures about
    /// **0.03**. Measured off the air, and the reason a sync correlation alone
    /// cannot tell the two apart.
    pub frame_acf: f32,
    /// Carrier offset measured from the reference blocks, in hertz.
    pub carrier_offset_hz: f32,
    /// Frames demodulated since the last reset.
    pub frames: u64,
    /// The last frame's rate verdict.
    pub rate: S4285Rate,
    /// The last frame's three reference concentrations.
    pub reference: [f32; 3],
    /// The last frame's 2-, 4- and 8-point constellation concentrations.
    pub constellation: [f32; 3],
    /// The sampling phase being tracked, in symbols (0..1).
    pub timing: f32,
    /// The coded rate a start-of-message pattern has been seen for. `None` until
    /// one has, because the four BPSK rates are identical at the constellation.
    pub coded: Option<fec::CodedRate>,
    /// Coded messages assembled by the Annex E layer this session.
    pub messages: u64,
    /// The closest the decoder's output has come to the start-of-message pattern,
    /// in matching bits out of 32. Noise gives about 16; a working decode on real
    /// traffic gets into the twenties even when a fade stops it matching.
    pub best_som: u8,
}

impl Default for S4285Report {
    fn default() -> Self {
        S4285Report {
            locked: false,
            sync_quality: 0.0,
            correlation_floor: 0.0,
            frame_acf: 0.0,
            carrier_offset_hz: 0.0,
            frames: 0,
            rate: S4285Rate::Unknown,
            reference: [0.0; 3],
            constellation: [0.0; 3],
            timing: 0.0,
            coded: None,
            messages: 0,
            best_som: 0,
        }
    }
}

/// A Fibonacci LFSR, least-significant bit first, as both of the standard's
/// generators are specified: the output is `x₀`, the feedback is the XOR of the
/// tapped stages, and the register shifts one position per output bit.
///
/// The stage count is the polynomial's degree, **not** one more than its highest
/// tap. `x⁵ + x² + 1` is five stages with taps {0, 2} and `x⁹ + x⁴ + 1` is nine
/// with taps {0, 4}; giving either a stage too many makes the register empty out
/// and stay at zero, which produced a sync sequence of period 16 and a scrambler
/// that was a constant — and the references then could not collapse, which read
/// as "no frame". `the_generators_are_maximal_and_match_the_standard` pins it.
fn lfsr(stages: usize, taps: &[usize], n: usize, init: u32) -> Vec<u8> {
    let mut state: Vec<u8> = (0..stages).map(|i| ((init >> i) & 1) as u8).collect();
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        out.push(state[0]);
        let mut feedback = 0;
        for &t in taps {
            feedback ^= state[t];
        }
        state.remove(0);
        state.push(feedback);
    }
    out
}

/// The 80-symbol synchronisation sequence as ±1, PN31 clocked from `11010`.
pub fn sync_symbols() -> [f32; SYNC] {
    let bits = lfsr(5, &[0, 2], SYNC, 0b11010);
    let mut out = [0.0f32; SYNC];
    for (o, b) in out.iter_mut().zip(&bits) {
        *o = if *b == 1 { 1.0 } else { -1.0 };
    }
    out
}

/// The products of adjacent sync symbols: what a differential detector produces,
/// so this is what the correlator has to be matched to.
pub fn sync_differential() -> [f32; SYNC - 1] {
    let s = sync_symbols();
    let mut out = [0.0f32; SYNC - 1];
    for i in 0..SYNC - 1 {
        out[i] = s[i + 1] * s[i];
    }
    out
}

/// The 176-symbol scrambling sequence, `Bk = e^(jnπ/4)` with
/// `n = 4x₂ + 2x₁ + x₀`, the register advancing three positions per symbol.
pub fn scrambling() -> [C; SCRAMBLED] {
    let bits = lfsr(9, &[0, 4], SCRAMBLED * 3, 1);
    let mut out = [C::default(); SCRAMBLED];
    for (k, o) in out.iter_mut().enumerate() {
        let n = 4 * bits[3 * k + 2] as i32 + 2 * bits[3 * k + 1] as i32 + bits[3 * k] as i32;
        *o = C::from_phase(n as f32 * std::f32::consts::FRAC_PI_4);
    }
    out
}

/// A windowed-sinc low-pass at the input rate, applied **before** any resampling.
/// Filtering first is not tidiness: interpolating a full-band signal folds the
/// rest of the band into the channel.
struct Lowpass {
    taps: Vec<f32>,
    buf: Vec<C>,
    pos: usize,
}

impl Lowpass {
    fn new(rate: f64, cutoff_hz: f64, len: usize) -> Self {
        let mid = (len / 2) as isize;
        let fc = (cutoff_hz / rate) as f32;
        let mut taps = Vec::with_capacity(len);
        let mut sum = 0.0f32;
        for i in 0..len {
            let x = i as isize - mid;
            let sinc = if x == 0 {
                2.0 * fc
            } else {
                (TAU as f32 * fc * x as f32).sin() / (std::f32::consts::PI * x as f32)
            };
            let w = 0.54 - 0.46 * ((TAU as f32) * i as f32 / (len - 1) as f32).cos();
            taps.push(sinc * w);
            sum += sinc * w;
        }
        for t in taps.iter_mut() {
            *t /= sum;
        }
        Lowpass { taps, buf: vec![C::default(); len], pos: 0 }
    }

    fn push(&mut self, z: C) -> C {
        let n = self.taps.len();
        self.buf[self.pos] = z;
        let mut acc = C::default();
        let mut k = self.pos;
        for &t in &self.taps {
            acc = acc.add(self.buf[k].scale(t));
            k = if k + 1 == n { 0 } else { k + 1 };
        }
        self.pos = if self.pos + 1 == n { 0 } else { self.pos + 1 };
        acc
    }
}

/// One coded-rate branch: deinterleaved soft bits in, decoded bits into a
/// message framer.
///
/// The four BPSK rates share one deinterleaved stream — the interleaver's `j` is
/// 12 for all of them — and differ only in how many repetitions of each coded
/// pair are averaged back. Two signs are run because the frame's absolute phase
/// is only known up to ±π until a start-of-message pattern says otherwise; a
/// flipped stream decodes to noise, so the branch that never sees an SOM costs
/// nothing but cycles.
const CHUNK_PAIRS: usize = 256;
/// Bits decoded at the front of each chunk and discarded, so the decoder's
/// forced start state has left the trace before the decisions are used.
const VITERBI_OVERLAP: usize = 64;

struct CodedChain {
    rate: fec::CodedRate,
    times: usize,
    sign: f32,
    deint: fec::ConvInterleaver,
    /// Deinterleaved soft values not yet decoded.
    soft: Vec<f32>,
    consumed: usize,
    framer: fec::MessageFramer,
    /// Messages this branch has framed; the first one is what names the rate.
    messages: u64,
}

impl CodedChain {
    fn new(rate: fec::CodedRate, sign: f32) -> Self {
        CodedChain {
            rate,
            times: rate.repeat(),
            sign,
            deint: fec::ConvInterleaver::deinterleaver(rate.j()),
            soft: Vec::new(),
            consumed: 0,
            framer: fec::MessageFramer::new(rate),
            messages: 0,
        }
    }

    fn reset(&mut self) {
        self.deint.reset();
        self.soft.clear();
        self.consumed = 0;
        self.framer.reset();
    }

    /// One soft decision from the demodulator (positive means a zero).
    fn push(&mut self, v: f32, out: &mut Vec<S4285Message>) {
        self.soft.push(self.deint.push(v * self.sign));
        // A chunk of `CHUNK_PAIRS` coded pairs, plus an overlap in front of it
        // that is decoded and **thrown away**.
        //
        // The overlap is in front, not behind, and that is the whole point: the
        // decoder is started with all its metric on state zero, and the traceback
        // runs backwards from the end, so the *opening* of a chunk is the part the
        // forced start distorts. Using the first `CHUNK_PAIRS` decisions threw
        // away the good ones and kept the bad: on a clean generated signal it put
        // a burst of errors in the middle of every message, 256 bits apart.
        let need = (CHUNK_PAIRS + VITERBI_OVERLAP) * 2 * self.times;
        while self.soft.len() - self.consumed >= need {
            let slice = &self.soft[self.consumed..self.consumed + need];
            let pairs = fec::average_pairs(slice, self.times);
            let bits = fec::viterbi_decode(&pairs);
            for b in bits.iter().skip(VITERBI_OVERLAP).take(CHUNK_PAIRS) {
                if let Some(m) = self.framer.push(*b) {
                    self.messages += 1;
                    out.push(S4285Message { rate: m.rate, bits: m.bits, closed: m.closed });
                }
            }
            self.consumed += CHUNK_PAIRS * 2 * self.times;
        }
        // Keep the buffer from growing without bound: everything before
        // `consumed` has been decoded and is dead weight.
        if self.consumed > 64 * 1024 {
            self.soft.drain(..self.consumed);
            self.consumed = 0;
        }
    }
}

/// Where the frame head sits, once acquisition has found it.
#[derive(Debug, Clone, Copy)]
struct Lock {
    /// Absolute index, in baseband samples, of the next frame's first symbol.
    ///
    /// Fractional, because the frame **step** is tracked and not fixed: two sound
    /// cards' clocks differ by far more than the standard's 10 ppm, and a fixed
    /// 256-symbol stride walks out of the timing window after a handful of frames
    /// — measured at ~500 ppm on this bench, which is one symbol every eight
    /// frames.
    next: f64,
}

/// The receiver. Feed it audio at `rate_hz`; it acquires on a fold of the
/// synchronisation, emits a frame per frame once locked, and text as the
/// character layer decodes.
pub struct S4285Receiver {
    rate: f64,
    carrier: f64,
    nco: f32,
    nco_inc: f32,
    lpf: Lowpass,
    step: f64,
    frac: f64,
    prev_bb: C,
    have_prev: bool,
    /// Baseband at [`SPS`] samples per symbol, newest last.
    bb: Vec<C>,
    /// Absolute index of `bb[0]`, in baseband samples.
    base: u64,
    /// Frames searched at each acquisition attempt, so a hopeless channel is not
    /// re-folded on every audio block.
    acq_wait: usize,
    lock: Option<Lock>,
    /// The tracked sampling phase, in symbols.
    timing: f32,
    scrambling: [C; SCRAMBLED],
    sync_diff: [f32; SYNC - 1],
    baudot: BaudotRx,
    /// The eight Annex E branches: four BPSK rates, each with both signs.
    coded: Vec<CodedChain>,
    /// The rate a start-of-message pattern has been seen for, once one has.
    coded_rate: Option<fec::CodedRate>,
    /// Messages the coded layer has assembled.
    coded_messages: u64,
    /// Messages ready to be handed out by the next `process` call.
    pending: Vec<S4285Message>,
    /// The last frame reached the coded-chain floor, whether or not it was good
    /// enough to report. A frame that is merely poor must not cost the lock: the
    /// lock is about the frame *position*, which such a frame still proves.
    last_frame_usable: bool,
    bit_run: Vec<u8>,
    /// Characters decoded but not yet handed out.
    events_text: String,
    report: S4285Report,
}

/// How many baseband samples the acquisition window spans.
const ACQ_SPAN: u64 = (ACQ_FRAMES as u64 + 1) * FRAME as u64 * SPS as u64;

impl S4285Receiver {
    /// `rate_hz` is the audio tap rate; `carrier_hz` is the audio frequency of the
    /// suppressed carrier — 1800 Hz when the dial is on it, but a fixed-frequency
    /// log entry can be a kilohertz out, so it is a parameter.
    pub fn new(rate_hz: f64, carrier_hz: f64) -> Self {
        S4285Receiver {
            rate: rate_hz,
            carrier: carrier_hz,
            nco: 0.0,
            nco_inc: (TAU as f32) * carrier_hz as f32 / rate_hz as f32,
            lpf: Lowpass::new(rate_hz, 1500.0, 129),
            step: rate_hz / MODEM_RATE,
            frac: 0.0,
            prev_bb: C::default(),
            have_prev: false,
            bb: Vec::new(),
            base: 0,
            acq_wait: 0,
            lock: None,
            timing: 0.0,
            scrambling: scrambling(),
            sync_diff: sync_differential(),
            baudot: BaudotRx::new(),
            coded: Vec::new(),
            coded_rate: None,
            coded_messages: 0,
            pending: Vec::new(),
            last_frame_usable: false,
            bit_run: Vec::new(),
            events_text: String::new(),
            report: S4285Report::default(),
        }
    }

    pub fn report(&self) -> &S4285Report {
        &self.report
    }

    /// The raw payload bits recovered so far, oldest first — exposed because a
    /// transmission that is not ITA2 must be visible as bits, not as silence.
    pub fn bits(&self) -> &[u8] {
        &self.bit_run
    }

    pub fn carrier_hz(&self) -> f64 {
        self.carrier
    }

    /// Retune the suppressed carrier and start again: acquisition cannot survive a
    /// jump, and a stale lock on a new frequency would be a lie.
    pub fn set_carrier(&mut self, carrier_hz: f64) {
        self.carrier = carrier_hz;
        self.nco_inc = (TAU as f32) * carrier_hz as f32 / self.rate as f32;
        self.reset();
    }

    pub fn reset(&mut self) {
        self.nco = 0.0;
        self.frac = 0.0;
        self.have_prev = false;
        self.bb.clear();
        self.base = 0;
        self.acq_wait = 0;
        self.lock = None;
        self.timing = 0.0;
        for c in self.coded.iter_mut() {
            c.reset();
        }
        self.coded_rate = None;
        self.bit_run.clear();
        self.report = S4285Report::default();
    }

    /// Feed audio; returns whatever it produced.
    pub fn process(&mut self, audio: &[f32]) -> Vec<S4285Event> {
        let mut events = Vec::new();
        for &s in audio {
            let lo = C::from_phase(-self.nco);
            self.nco += self.nco_inc;
            if self.nco > TAU as f32 {
                self.nco -= TAU as f32;
            }
            let z = self.lpf.push(C::new(s, 0.0).mul(lo));
            if !self.have_prev {
                self.prev_bb = z;
                self.have_prev = true;
                continue;
            }
            while self.frac < 1.0 {
                self.bb.push(self.prev_bb.lerp(z, self.frac as f32));
                self.frac += self.step;
            }
            self.frac -= 1.0;
            self.prev_bb = z;
        }

        match self.lock {
            None => {
                if self.bb.len() as u64 >= ACQ_SPAN + SPS as u64 {
                    self.acquire();
                }
            }
            Some(_) => self.drain_frames(&mut events),
        }
        self.trim();
        events
    }

    /// A symbol at a fractional sample position, linearly interpolated. The
    /// baseband is band-limited to 1.5 kHz and sampled at 12 kHz, so a straight
    /// line between two samples is four times oversampled.
    fn at(&self, pos: f64) -> C {
        let i = pos.floor();
        if i < 0.0 {
            return C::default();
        }
        let i = i as usize;
        if i + 1 >= self.bb.len() {
            return self.bb.last().copied().unwrap_or_default();
        }
        self.bb[i].lerp(self.bb[i + 1], (pos - i as f64) as f32)
    }

    /// `FRAME` symbols from a frame head at `start` (a baseband sample index) and
    /// a sampling phase in symbols.
    fn frame_symbols(&self, start: u64, phase: f32) -> [C; FRAME] {
        let mut out = [C::default(); FRAME];
        for (i, o) in out.iter_mut().enumerate() {
            let pos = start as f64 - self.base as f64 + (i as f32 + phase) as f64 * SPS as f64;
            *o = self.at(pos);
        }
        out
    }

    /// The frame-period autocorrelation of the baseband: how much of a frame
    /// repeats one frame later. Independent of the sampling phase, which is why it
    /// can be reported before acquisition has found one.
    ///
    /// It is taken over **everything buffered**, not over one frame. One frame's
    /// worth of a modulated signal is 1280 samples of a statistic that has to see
    /// many frames to mean anything: measured on a live 12.857 MHz capture, one
    /// frame's estimate read 0.06 where the whole 6-second buffer read 0.355
    /// against its own null of 0.019 — the instrument, not the signal.
    fn frame_acf(&self) -> f32 {
        let lag = FRAME * SPS;
        let n = self.bb.len();
        if n < lag + FRAME * SPS {
            return 0.0;
        }
        let take = n - lag;
        let mut acc = C::default();
        let (mut ea, mut eb) = (0.0f32, 0.0f32);
        for k in 0..take {
            let a = self.bb[n - take - lag + k];
            let b = self.bb[n - take + k];
            acc = acc.add(b.mul(a.conj()));
            ea += a.norm() * a.norm();
            eb += b.norm() * b.norm();
        }
        let denom = (ea * eb).sqrt();
        if denom <= 1e-20 {
            return 0.0;
        }
        (acc.norm() / denom).min(1.0)
    }

    /// Fold the differential sync correlation over every frame phase, at each
    /// sub-symbol sampling phase, and take the standing-up one.
    fn acquire(&mut self) {
        if self.acq_wait > 0 {
            self.acq_wait -= 1;
            return;
        }
        let n = self.bb.len();
        if n < ACQ_SPAN as usize {
            return;
        }
        let start = self.base + (n as u64 - ACQ_SPAN);
        let acf = self.frame_acf();
        self.report.frame_acf = acf;
        if acf < MIN_FRAME_ACF {
            self.report.locked = false;
            self.report.correlation_floor = 0.0;
            self.report.sync_quality = 0.0;
            self.acq_wait = ACQ_FRAMES;
            return;
        }

        let count = ACQ_SPAN as usize / SPS;
        let mut best = (0.0f32, 0.0f32, 0usize, 0.0f32);
        for p in 0..PHASES {
            let phase = p as f32 / PHASES as f32;
            // Symbols over the whole window, at this sampling phase.
            let mut sym = Vec::with_capacity(count);
            for k in 0..count {
                let pos = start as f64 - self.base as f64 + (k as f32 + phase) as f64 * SPS as f64;
                sym.push(self.at(pos));
            }
            // Differential products, then the correlation over the sync window.
            let mut corr = vec![0.0f32; count];
            let n = self.sync_diff.len();
            for k in n..count {
                let mut acc = C::default();
                let mut mag = 0.0f32;
                for (j, &w) in self.sync_diff.iter().enumerate() {
                    let i = k - n + j;
                    let d = sym[i + 1].mul(sym[i].conj());
                    acc = acc.add(d.scale(w));
                    mag += d.norm();
                }
                if mag > 1e-12 {
                    corr[k] = acc.norm() / mag;
                }
            }
            // Fold: mean per phase index, skipping the head where no window fits.
            // `corr[k]` covers symbols `k - n ..= k`, so the window's **first**
            // symbol — the frame head — is `k - n`, and that is what is folded.
            let mut sum = [0.0f32; FRAME];
            let mut num = [0u32; FRAME];
            for (k, &c) in corr.iter().enumerate().skip(n) {
                sum[(k - n) % FRAME] += c;
                num[(k - n) % FRAME] += 1;
            }
            let folded: Vec<f32> = sum
                .iter()
                .zip(&num)
                .map(|(s, m)| if *m > 0 { *s / *m as f32 } else { 0.0 })
                .collect();
            let mut sorted = folded.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let median = sorted[FRAME / 2];
            let (o, &score) = folded
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or((0, &0.0));
            if score > best.0 {
                best = (score, median, o, phase);
            }
        }

        let (score, median, offset, phase) = best;
        self.report.correlation_floor = median;
        self.report.sync_quality = score;
        // A folded correlation has to stand well clear of its own null. Measured:
        // 0.85 on the signal, 0.14 on the control, so 2.2x is the middle with room.
        if score < MIN_SYNC || score < median * 2.2 {
            self.report.locked = false;
            self.acq_wait = ACQ_FRAMES;
            return;
        }
        // The fold says where the frame head is; from here every frame is taken at
        // a fixed stride and refines its own timing.
        self.lock = Some(Lock { next: (start + (offset * SPS) as u64) as f64 });
        self.timing = phase;
        self.report.locked = true;
    }

    fn drain_frames(&mut self, events: &mut Vec<S4285Event>) {
        loop {
            let Some(lock) = self.lock else { return };
            if lock.next + (FRAME * SPS) as f64 > (self.base + self.bb.len() as u64) as f64 {
                return;
            }
            let frame = self.take_frame(lock.next.round() as u64);
            match frame {
                Some(f) => {
                    events.push(S4285Event::Frame(f.clone()));
                    for m in self.pending.drain(..) {
                        // The character layer, if the message reads as text. A
                        // reading that does not pass the test stays silent: letters
                        // that mean nothing are the one failure a listener cannot
                        // check.
                        if let Some(t) = fec::message_text(&m.bits) {
                            self.events_text.push_str(&t);
                            events.push(S4285Event::Text(std::mem::take(&mut self.events_text)));
                        }
                        events.push(S4285Event::Message(m));
                    }
                    // Text only from a frame whose constellation is resolved: an
                    // unresolved one produces letters that look plausible and mean
                    // nothing, which is the one failure a listener cannot see. The
                    // bits are still on the event and in `bits()`.
                    if f.rate != S4285Rate::Unknown {
                        for value in f.bits.chunks(5) {
                            let mut v = 0u8;
                            for (i, b) in value.iter().enumerate() {
                                v |= b << i;
                            }
                            if let Some(ch) = self.baudot.decode(v) {
                                self.events_text.push(ch);
                            }
                        }
                    }
                    if !self.events_text.is_empty() {
                        let t: String = self.events_text.drain(..).collect();
                        events.push(S4285Event::Text(t));
                    }
                    if let Some(l) = self.lock.as_mut() {
                        l.next += (FRAME * SPS) as f64;
                    }
                }
                None => {
                    // Poor enough to report nothing — but if it still reached the
                    // coded floor its *position* is sound and the lock stands, with
                    // its 128 soft decisions already in the chains as erasures.
                    // Letting go here would restart the interleaver on the first
                    // fade of every message.
                    if self.last_frame_usable {
                        if let Some(l) = self.lock.as_mut() {
                            l.next += (FRAME * SPS) as f64;
                        }
                        continue;
                    }
                    // The frame did not stand up at all: let go and acquire again
                    // from what is still buffered rather than demodulating a lie.
                    self.report.locked = false;
                    self.lock = None;
                    self.acq_wait = 0;
                    return;
                }
            }
        }
    }

    /// Demodulate the frame whose head is at absolute sample `start`.
    /// The frame's 176 scrambled symbols, equalised across the path.
    ///
    /// The one real coded tape we have cannot put its data symbols on a lattice
    /// while its sync and reference blocks are healthy, and its data *blocks* are
    /// no better than the frame — so the damage is inside 32 symbols, which is
    /// delay spread and not a phase drift. **The channel is estimated from the 80
    /// known sync symbols by least squares and inverted in closed form**; an
    /// adaptive loop trained on the same symbols does not converge (measured, see
    /// the handover). It is used only on a frame where it makes the known
    /// reference symbols collapse harder than they already do, so a channel that
    /// needs no equalising is read exactly as before.
    fn coded_symbols(&self, start: u64, phase: f32, reversed: bool, offset: f32) -> [C; SCRAMBLED] {
        let frame = self.frame_symbols(start, phase);
        // The carrier comes off first: the estimate is taken against real ±1 sync
        // symbols, and a ramp left in looks like a channel that turns a whole
        // circle every 27 symbols.
        let deramp = |f: &[C; FRAME]| -> [C; FRAME] {
            let mut o = [C::default(); FRAME];
            for (k, v) in f.iter().enumerate() {
                let t = k as f32 / BAUD as f32;
                o[k] = v.mul(C::from_phase(-offset * TAU * t));
            }
            o
        };
        let d = deramp(&frame);
        let mut plain = [C::default(); SCRAMBLED];
        for k in 0..SCRAMBLED {
            let sc = if reversed { self.scrambling[k].conj() } else { self.scrambling[k] };
            plain[k] = d[SYNC + k].mul(sc.conj());
        }

        // **The frame's own phase reference.** The reference blocks are known to be
        // symbol 0, so their mean phase is the global rotation the path has put on
        // the frame, and the sync symbols the estimate is taken against are real.
        let mut ref_phase = C::default();
        for &(lo, hi) in &REF_BLOCKS {
            for k in lo..hi {
                ref_phase = ref_phase.add(plain[k]);
            }
        }
        let rot = C::from_phase(-ref_phase.arg());
        let mut r = d;
        for v in r.iter_mut() {
            *v = v.mul(rot);
        }

        let sync = sync_symbols();
        let mut corr = C::default();
        for k in 0..SYNC {
            corr = corr.add(r[k].scale(sync[k]));
        }
        let sg = if corr.re < 0.0 { -1.0 } else { 1.0 };
        let known: Vec<C> = sync.iter().map(|v| C::new(sg * v, 0.0)).collect();
        let channel = ls_channel(&r[..SYNC], &known, LS_TAPS);
        let energy: f32 =
            channel.as_ref().map(|h| h.iter().map(|v| v.norm() * v.norm()).sum()).unwrap_or(0.0);
        let equaliser = channel.as_ref().and_then(|h| zf_equaliser(h, energy * 5e-3));

        // **The filter's group delay is searched, not assumed.** The inverse is the
        // minimum-energy one, so where its unit pulse lands is a property of the
        // solution and not a constant of the code: a symbol out and the reference
        // blocks sample the data either side of them, which is exactly the "worse
        // than plain" the first version measured. Three delays, the middle one
        // first, and the references pick.
        let score = |z: &[C; SCRAMBLED]| {
            let c = ramp_concentrations(z, 0.0);
            c[0] + c[1] + c[2]
        };
        let mut equalised = [C::default(); SCRAMBLED];
        if let Some(w) = &equaliser {
            let mid = (LS_EQ - 1) / 2;
            let mut best = (score(&plain), equalised);
            for shift in [mid, mid - 1, mid + 1] {
                let mut cand = [C::default(); SCRAMBLED];
                for k in 0..SCRAMBLED {
                    let m = SYNC + k;
                    let mut y = C::default();
                    for (i, wi) in w.iter().enumerate() {
                        if let Some(v) = (m + shift).checked_sub(i).and_then(|j| r.get(j)) {
                            y = y.add(v.mul(*wi));
                        }
                    }
                    let sc = if reversed { self.scrambling[k].conj() } else { self.scrambling[k] };
                    cand[k] = y.mul(sc.conj());
                }
                let s = score(&cand);
                if s > best.0 {
                    best = (s, cand);
                }
            }
            equalised = best.1;
        }

        // **It has to earn its place, frame by frame.** On a channel that needed
        // nothing it can only add a little of its own noise — measured, it cost a
        // ramped clean signal its exact bits. The sixteen known reference symbols
        // in each block decide it: whichever path makes them collapse harder is
        // the path the frame is read through.
        let score = |z: &[C; SCRAMBLED]| {
            let c = ramp_concentrations(z, 0.0);
            c[0] + c[1] + c[2]
        };
        if equaliser.is_some() && score(&equalised) > score(&plain) { equalised } else { plain }
    }

    fn take_frame(&mut self, start: u64) -> Option<S4285Frame> {
        // The sampling phase is searched over **most of a symbol**, not over a
        // fifth of one. Acquisition answers it to a tenth of a symbol, and a
        // tracked value that starts half a symbol out never recovers if the
        // per-frame search can only reach ±0.2: on a clean generated signal that
        // cost a handful of bit errors in the middle of a message, which is
        // exactly how a decode looks when it is nearly right.
        let mut best: Option<(f32, f32, bool, f32)> = None;
        for step in -9i32..=9 {
            // The window is anchored at **half a symbol inside the frame**, not on
            // the last frame's answer: it is the clock tracker below that keeps the
            // true sampling instant there, and a window that follows its own
            // previous answer simply walks away with it.
            let phase = 0.5 + step as f32 * 0.05;
            let frame = self.frame_symbols(start, phase);
            for &sign in &[1.0f32, -1.0f32] {
                let mut z = [C::default(); SCRAMBLED];
                for k in 0..SCRAMBLED {
                    let s = if sign > 0.0 { self.scrambling[k] } else { self.scrambling[k].conj() };
                    z[k] = frame[SYNC + k].mul(s.conj());
                }
                // Coarse ramp, then a fine one round the best. A 16-symbol block
                // is 6.67 ms, so 150 Hz turns a whole circle inside it.
                let mut coarse = (0.0f32, 0.0f32);
                let mut hz = -150.0f32;
                while hz <= 150.0 {
                    let c = ramp_concentrations(&z, hz);
                    let r = (c[0] + c[1] + c[2]) / 3.0;
                    if r > coarse.0 {
                        coarse = (r, hz);
                    }
                    hz += 5.0;
                }
                let mut fine = coarse;
                let mut hz = coarse.1 - 5.0;
                while hz <= coarse.1 + 5.0 {
                    let c = ramp_concentrations(&z, hz);
                    let r = (c[0] + c[1] + c[2]) / 3.0;
                    if r > fine.0 {
                        fine = (r, hz);
                    }
                    hz += 1.0;
                }
                if best.as_ref().is_none_or(|b| fine.0 > b.0) {
                    best = Some((fine.0, phase, sign < 0.0, fine.1));
                }
            }
        }
        let (score, mut phase, reversed, offset) = best?;
        self.last_frame_usable = score >= MIN_CODED_REFERENCE;
        // **The sampling instant has to come from the data, not from the
        // references.** Sixteen identical symbols score equally well anywhere
        // across them, so a phase search that maximises the reference
        // concentration is flat in time — it cannot tell a correct sampling
        // instant from half a symbol out, and on a real path a frame decoded half
        // a symbol late has every data symbol smeared while its reference blocks
        // still read 0.9. That is exactly the shape of the failure this cost on
        // the bench.
        //
        // So once the sign and the carrier offset are known, the phase is chosen
        // again — this time by how tightly the *data* symbols land on the
        // constellation.
        {
            let mut tight = (0.0f32, phase);
            for step in -10i32..=10 {
                let p = phase + step as f32 * 0.05;
                let frame = self.frame_symbols(start, p);
                let mut z = [C::default(); SCRAMBLED];
                for k in 0..SCRAMBLED {
                    let sc = if reversed { self.scrambling[k].conj() } else { self.scrambling[k] };
                    let t = k as f32 / BAUD as f32;
                    z[k] = frame[SYNC + k].mul(sc.conj()).mul(C::from_phase(-offset * TAU * t));
                }
                // The channel first, then the data: on a real path the three
                // reference blocks differ in phase *and level*, and a
                // concentration measured on the raw data is dominated by that
                // rotation rather than by the sampling instant — which makes the
                // sweep answer noise.
                let fit = ChannelFit::from_references(&z);
                let (mut c2, mut c4, mut c8) = (C::default(), C::default(), C::default());
                let mut n = 0.0f32;
                for &(lo, hi) in &DATA_BLOCKS {
                    for k in lo..hi {
                        let c = fit.apply(k, z[k]);
                        let a = c.arg();
                        c2 = c2.add(C::from_phase(2.0 * a));
                        c4 = c4.add(C::from_phase(4.0 * a));
                        c8 = c8.add(C::from_phase(8.0 * a));
                        n += 1.0;
                    }
                }
                let data_conc = (c2.norm() / n).max(c4.norm() / n).max(c8.norm() / n);
                // The references are added in, not used alone. The data is what
                // decides where the eye is open — sixteen identical symbols
                // cannot — but the reference concentration is the number the
                // panel shows and the gate reads, and it does vary a little with
                // the instant; scoring both keeps the reported figure honest at
                // the instant the data wants.
                let refs = ramp_concentrations(&z, 0.0);
                // **And the sync's own fit, which is the only honest timing
                // signal here.** `data_conc` is circular on a smeared channel: it
                // asks the data where the eye is, and the smear is what is closing
                // it. The 80 known sync symbols have a transition on almost every
                // symbol, so their least-squares residual says whether the sampling
                // instant is right without asking the data anything. Weighted to
                // dominate, because when the two disagree the sequence is right.
                let mut deramped = [C::default(); FRAME];
                for (k, v) in deramped.iter_mut().enumerate() {
                    let t = k as f32 / BAUD as f32;
                    *v = frame[k].mul(C::from_phase(-offset * TAU * t));
                }
                let (residual, _) = sync_fit(&deramped, &sync_symbols());
                let timing = 1.0 - residual.min(1.0);
                let v = data_conc + (refs[0] + refs[1] + refs[2]) / 3.0 + 2.0 * timing;
                if v > tight.0 {
                    tight = (v, p);
                }
            }
            phase = tight.1;
        }
        // Annex E's chains are fed from the frame *before* the reference gate: the
        // stream into the interleaver has to be continuous, and a frame too poor to
        // report is still a frame's worth of soft decisions that the code can use.
        // A frame that is missing altogether breaks the stream, so the chains are
        // reset wherever one is skipped.
        let mut coded_msgs = Vec::new();
        let mut soft = Vec::new();
        if score >= MIN_CODED_REFERENCE {
            soft = self.feed_coded(start, phase, reversed, offset, &mut coded_msgs);
        } else {
            // A frame too poor to decode is **not** a reason to start the chains
            // again: its position in the stream is known, so its 128 missing soft
            // decisions go in as erasures. A zero contributes nothing to either
            // hypothesis, which is what an erasure is — and a restart would throw
            // away the ten seconds of interleaver that precede this frame, which
            // on a fading path means never completing a message at all.
            self.feed_erasures(&mut coded_msgs);
        }
        for m in coded_msgs {
            self.coded_messages += 1;
            self.report.messages = self.coded_messages;
            if self.coded_rate.is_none() {
                self.coded_rate = Some(m.rate);
                self.report.coded = Some(m.rate);
            }
            self.pending.push(m);
        }
        if score < MIN_REFERENCE {
            return None;
        }
        // The carrier offset has to come off the symbols themselves, not just off
        // the score: the channel phase is read from the reference blocks, and a
        // ramp left in place turns a whole circle between two of them, so the
        // interpolation across the frame would be nonsense.
        let z = self.coded_symbols(start, phase, reversed, offset);
        let reference = ramp_concentrations(&z, 0.0);
        // **Track the symbol clock, not only the phase.** The frame stride is a
        // number of samples, and two clocks that differ by even a few hundred ppm
        // walk that stride out of the ±0.45-symbol window in a handful of frames.
        // The best sampling phase inside a frame is the error signal: it sits at
        // 0.5 when the stride is right, and its departure from 0.5 is how far the
        // next frame head has to move.
        // The error is taken **modulo one symbol**: the phase sweep can answer
        // outside 0..1 (it reaches ±0.5 past the window's centre), and an error of
        // −0.6 read literally walks the frame head the wrong way every frame until
        // the timing runs away.
        let mut err = phase - 0.5;
        if err > 0.5 {
            err -= 1.0;
        } else if err < -0.5 {
            err += 1.0;
        }
        if let Some(l) = self.lock.as_mut() {
            if err.abs() < 0.45 {
                l.next += (err * SPS as f32 * 0.5).clamp(-2.0, 2.0) as f64;
            }
        }
        self.timing = phase.rem_euclid(1.0);

        // The channel inside the frame, from its three reference blocks — phase
        // and level. See `ChannelFit`.
        let fit = ChannelFit::from_references(&z);

        let mut used = [false; 8];
        let mut data: Vec<C> = Vec::with_capacity(128);
        for &(lo, hi) in &DATA_BLOCKS {
            for k in lo..hi {
                let c = fit.apply(k, z[k]);
                let idx = (c.arg() / std::f32::consts::FRAC_PI_4).round() as i32;
                used[idx.rem_euclid(8) as usize] = true;
                data.push(c);
            }
        }
        let phases_used = used.iter().filter(|u| **u).count();

        // The constellation, as concentrations that a rotation cannot change.
        let (mut c2, mut c4, mut c8) = (C::default(), C::default(), C::default());
        for c in &data {
            c2 = c2.add(C::from_phase(2.0 * c.arg()));
            c4 = c4.add(C::from_phase(4.0 * c.arg()));
            c8 = c8.add(C::from_phase(8.0 * c.arg()));
        }
        let n = data.len().max(1) as f32;
        let constellation = [c2.norm() / n, c4.norm() / n, c8.norm() / n];

        // Per block, with the same channel: see `S4285Frame::block`.
        let mut block = [0.0f32; 4];
        for (b, &(lo, hi)) in DATA_BLOCKS.iter().enumerate() {
            let (mut b2, mut b4, mut b8) = (C::default(), C::default(), C::default());
            for k in lo..hi {
                let a = fit.apply(k, z[k]).arg();
                b2 = b2.add(C::from_phase(2.0 * a));
                b4 = b4.add(C::from_phase(4.0 * a));
                b8 = b8.add(C::from_phase(8.0 * a));
            }
            let nb = (hi - lo).max(1) as f32;
            block[b] = (b2.norm() / nb).max(b4.norm() / nb).max(b8.norm() / nb);
        }

        // A 2PSK frame leaves the even symbol numbers only; a dibit frame the
        // multiples of two; a tribit frame all eight. Noise smears each of those,
        // so the classes are told apart by which concentration stands up, not by
        // counting phases that noise has already spread over all eight. A leading
        // concentration under 0.2 is not a resolved constellation and is reported
        // as such rather than guessed — a rate with no evidence behind it is how a
        // wrong demodulation looks right.
        let (c2, c4, c8) = (constellation[0], constellation[1], constellation[2]);
        let leader = c2.max(c4).max(c8);
        let runner = if leader == c2 {
            c4.max(c8)
        } else if leader == c4 {
            c2.max(c8)
        } else {
            c2.max(c4)
        };
        let rate = if leader < 0.20 || leader < runner * 1.5 {
            S4285Rate::Unknown
        } else if leader == c2 {
            S4285Rate::Bps1200
        } else if leader == c4 {
            S4285Rate::Bps2400
        } else {
            S4285Rate::Bps3600OrCoded
        };
        let bits = transcode(&data, rate);

        self.report.frames += 1;
        self.report.rate = rate;
        self.report.reference = reference;
        self.report.constellation = constellation;
        self.report.carrier_offset_hz = offset;
        self.report.timing = self.timing;
        self.bit_run.extend_from_slice(&bits);
        if self.bit_run.len() > 8192 {
            let cut = self.bit_run.len() - 8192;
            self.bit_run.drain(..cut);
        }
        Some(S4285Frame {
            reference,
            carrier_offset_hz: offset,
            reversed,
            rate,
            phases_used,
            constellation,
            block,
            bits,
            soft,
        })
    }

    /// Descramble one frame's data again for the Annex E chains and push its 128
    /// soft decisions into each branch.
    ///
    /// The chain runs on the *raw* constellation, not on the rate verdict: the
    /// four BPSK rates are identical at the constellation, and the SOM is what
    /// tells them apart.
    fn feed_coded(
        &mut self,
        start: u64,
        phase: f32,
        reversed: bool,
        offset: f32,
        out: &mut Vec<S4285Message>,
    ) -> Vec<f32> {
        let z = self.coded_symbols(start, phase, reversed, offset);
        // **The same channel line the frame itself is read through.** Without it
        // the soft decisions keep the frame's residual phase, which is a different
        // number every frame on anything that is not exactly on frequency — and a
        // BPSK decision's whole content is its sign, so a rotating phase turns the
        // code stream into noise while the constellation still reads 0.99 and the
        // references still collapse. Measured: a 1 Hz/s ramp, well inside the
        // standard's 3.5 Hz/s, took the message from readable to nothing. The
        // level is corrected with the phase (see `ChannelFit`), which is what a
        // recording whose audio filter tilts across the frame needs.
        let fit = ChannelFit::from_references(&z);
        let mut data: Vec<C> = Vec::with_capacity(128);
        let mut sum = 0.0f32;
        for &(lo, hi) in &DATA_BLOCKS {
            for k in lo..hi {
                let c = fit.apply(k, z[k]);
                sum += c.norm();
                data.push(c);
            }
        }
        if sum <= 1e-9 {
            return Vec::new();
        }
        // A BPSK symbol's soft decision is its real part, scaled so a clean
        // symbol is ±1 — Annex E's "linear distance from the decision boundary".
        let scale = data.len() as f32 / sum;
        if self.coded.is_empty() {
            for r in fec::CodedRate::BPSK {
                for sign in [1.0f32, -1.0] {
                    self.coded.push(CodedChain::new(r, sign));
                }
            }
        }
        let soft: Vec<f32> = data.iter().map(|c| c.re * scale).collect();
        for chain in self.coded.iter_mut() {
            for v in &soft {
                chain.push(*v, out);
            }
            if chain.framer.best_som > self.report.best_som {
                self.report.best_som = chain.framer.best_som;
            }
        }
        if let Some(rate) = self.coded_rate {
            // Only the branch that has produced a message is worth decoding; the
            // others are noise by definition once the rate is known.
            self.coded.retain(|c| c.rate == rate && c.messages > 0);
        }
        soft
    }

    /// One frame's worth of soft decisions as erasures, to keep the Annex E
    /// chains aligned with the frame index when a frame is too poor to read.
    fn feed_erasures(&mut self, out: &mut Vec<S4285Message>) {
        if self.coded.is_empty() {
            return;
        }
        for chain in self.coded.iter_mut() {
            for _ in 0..128 {
                chain.push(0.0, out);
            }
        }
    }

    /// Keep only what the next frame and the acquisition window need.
    fn trim(&mut self) {
        let need = match self.lock {
            Some(l) => (l.next as u64).saturating_sub(self.base),
            None => 0,
        };
        let keep_from = need.min(self.bb.len() as u64) as usize;
        let drop = match self.lock {
            Some(_) => keep_from,
            None => self.bb.len().saturating_sub(ACQ_SPAN as usize + SPS),
        };
        if drop > 4096 {
            self.bb.drain(..drop);
            self.base += drop as u64;
        }
    }

    /// Characters decoded but not yet handed out.
    pub fn take_text(&mut self) -> String {
        std::mem::take(&mut self.events_text)
    }
}

/// The mean reference concentration a block needs before it is called a frame.
///
/// Sixteen symbols of noise give `1/sqrt(16)` = 0.25; a frame whose blocks have
/// collapsed measured 0.65–0.81 across the night's captures. 0.45 sits between
/// them and is deliberately nearer the noise floor than the signal: a frame that
/// is wrongly accepted is visible in the three numbers, while one wrongly rejected
/// is silence.
const MIN_REFERENCE: f32 = 0.45;

/// The reference concentration below which a frame's soft decisions are too poor
/// to be worth handing to the code, but above which they still are: the reported
/// three numbers and the coded stream want different things from a fading frame.
const MIN_CODED_REFERENCE: f32 = 0.30;

/// The folded sync correlation a lock needs, and the margin over its own null.
///
/// A correct 4285 measures 0.85 folded over twelve frames; the strong PSK carrier
/// that is a different mode measures 0.14 on the same statistic, and a single
/// frame's maximum over every offset reaches 0.738 on white noise — which is why
/// the peak alone was never a detector.
const MIN_SYNC: f32 = 0.40;

/// The frame-period autocorrelation a lock needs.
///
/// A correct 4285 measures 0.31-0.60 (its 80 sync symbols repeat out of 256) and a
/// strong PSK carrier of another mode measures ~0.03, both measured. 0.12 sits
/// between them with room for a fading capture.
const MIN_FRAME_ACF: f32 = 0.12;

/// The channel across one frame, from its three reference blocks: the phase at
/// each block, and the least-squares line through them.
///
/// This is what the reference symbols are *for*. A single ramp corrects a carrier
/// offset; it does not correct a channel whose phase at the third block is not
/// where the first two predict — and on a real path it usually is not. The data
/// has to be derotated by the line, not by the ramp alone, or every data block
/// after the first is rotated by whatever the channel did between them. On a
/// clean file that difference is nil, which is exactly why it survived so long.
/// The complex channel across one frame, read from its three reference blocks.
///
/// A straight **phase** line was enough on a clean file and on the bench path, and
/// it is not enough on a real station: the level and the phase both move inside
/// the 107 ms frame, so the data reads as a smear while the references still
/// collapse. The first real coded capture we were sent measures exactly that —
/// references 0.69 / 0.76 / 0.61 and a data constellation of 0.34 — and its
/// spectrum is 2.2 kHz wide, so the tilt is the recording's filter as much as the
/// path. This fits **I and Q** linearly across the frame, interpolating between
/// the reference blocks and extrapolating to the data blocks at either end, so a
/// symbol is corrected in amplitude as well as in phase.
///
/// The slope is unambiguous because the carrier offset is taken off the symbols
/// before this is called: left in, 55 Hz turns a whole circle over 48 symbols and
/// the fit wraps. See `take_frame`.
#[derive(Clone, Copy)]
struct ChannelFit {
    slope: C,
    intercept: C,
}

/// Taps in the least-squares channel estimate. Three covers an echo up to two
/// symbols late once the fractional delay is split across two taps, and it is
/// short enough to estimate from one frame's 80 known sync symbols.
const LS_TAPS: usize = 5;
/// Taps in the equaliser computed from that estimate. Longer than the channel,
/// because the inverse of a short FIR is not short.
const LS_EQ: usize = 9;

/// Divide by a complex number, which `C` has no operator for.
fn cdiv(a: C, b: C) -> C {
    let d = b.norm() * b.norm();
    if d <= 1e-12 {
        return C::default();
    }
    a.mul(b.conj()).scale(1.0 / d)
}

/// Solve `a·x = b` for a small dense complex system, by Gauss-Jordan with
/// partial pivoting. `n` is at most [`LS_EQ`], so the cost is nothing.
fn solve(mut a: Vec<Vec<C>>, mut b: Vec<C>) -> Option<Vec<C>> {
    let n = b.len();
    for col in 0..n {
        let (piv, _) = (col..n)
            .map(|r| (r, a[r][col].norm()))
            .max_by(|x, y| x.1.partial_cmp(&y.1).unwrap_or(std::cmp::Ordering::Equal))?;
        if a[piv][col].norm() < 1e-9 {
            return None;
        }
        a.swap(col, piv);
        b.swap(col, piv);
        let d = a[col][col];
        for j in col..n {
            a[col][j] = cdiv(a[col][j], d);
        }
        b[col] = cdiv(b[col], d);
        for r in 0..n {
            if r == col {
                continue;
            }
            let f = a[r][col];
            if f.norm() <= 1e-12 {
                continue;
            }
            for j in col..n {
                a[r][j] = a[r][j].sub(f.mul(a[col][j]));
            }
            b[r] = b[r].sub(f.mul(b[col]));
        }
    }
    Some(b)
}

/// Estimate the channel from **known** symbols, by least squares.
///
/// `rx[k]` is the received symbol at instant `k`, `known[k]` what was sent, and
/// the model is `rx[k] = Σ h[i]·known[k-i]` — an FIR, which is what a path with
/// a few milliseconds of delay spread looks like at 2400 baud. The normal
/// equations are small and real-symmetric in structure, so they are solved
/// directly rather than iterated towards: an adaptive loop on 80 symbols does
/// not converge here, measured, and this does not have to.
fn ls_channel(rx: &[C], known: &[C], taps: usize) -> Option<Vec<C>> {
    if rx.len() <= taps || known.len() != rx.len() {
        return None;
    }
    // Rows are the equations, columns the tap index: A[i][j] = conj(k[n-i])·k[n-j].
    let mut ata = vec![vec![C::default(); taps]; taps];
    let mut atb = vec![C::default(); taps];
    for n in taps - 1..rx.len() {
        for i in 0..taps {
            let xi = known[n - i].conj();
            for j in 0..taps {
                ata[i][j] = ata[i][j].add(xi.mul(known[n - j]));
            }
            atb[i] = atb[i].add(xi.mul(rx[n]));
        }
    }
    solve(ata, atb)
}

/// The least-squares residual of the **known** sync against a frame, and the
/// frame's own phase rotation.
///
/// This is the timing error signal the data cannot give: the sync has a transition
/// on almost every symbol, so a misfit means the sampling instant is wrong, while
/// the data-tightness statistic is circular on a smeared channel — the channel
/// estimate and the constellation both want the eye that the smear is closing.
/// The residual is normalised by the sync's own received amplitude, so it is a
/// fraction rather than a level and one threshold works at any AGC setting.
fn sync_fit(r: &[C; FRAME], sync: &[f32; SYNC]) -> (f32, C) {
    let mut ref_phase = C::default();
    for &(lo, hi) in &REF_BLOCKS {
        for k in lo..hi {
            ref_phase = ref_phase.add(r[SYNC + k]);
        }
    }
    let rot = C::from_phase(-ref_phase.arg());
    let mut r2 = *r;
    for v in r2.iter_mut() {
        *v = v.mul(rot);
    }
    let mut corr = C::default();
    let mut amp = 0.0f32;
    for k in 0..SYNC {
        corr = corr.add(r2[k].scale(sync[k]));
        amp += r2[k].norm();
    }
    amp /= SYNC as f32;
    if amp <= 1e-9 {
        return (1.0, rot);
    }
    let sg = if corr.re < 0.0 { -1.0 } else { 1.0 };
    let known: Vec<C> = sync.iter().map(|v| C::new(sg * v, 0.0)).collect();
    let Some(h) = ls_channel(&r2[..SYNC], &known, LS_TAPS) else { return (1.0, rot) };
    let mut err = 0.0f32;
    let mut n = 0.0f32;
    for k in LS_TAPS - 1..SYNC {
        let mut y = C::default();
        for (i, hi) in h.iter().enumerate() {
            if let Some(v) = k.checked_sub(i).map(|j| known[j]) {
                y = y.add(v.mul(*hi));
            }
        }
        let e = r2[k].sub(y).norm();
        err += e * e;
        n += 1.0;
    }
    ((err / n.max(1.0)).sqrt() / amp, rot)
}

/// The equaliser for `h`: the `LS_EQ`-tap filter whose convolution with the
/// channel is as close to a unit pulse as it can be.
///
/// This is the normal-equation form of it, which is the one that behaves: `R` is
/// the channel's **autocorrelation** matrix, `p` the matched filter, and
/// `(R + σ²I)·w = p` is solved for `w`. The loading is not optional — a three-tap
/// channel cannot be inverted exactly by a longer FIR, so `R` is singular by
/// construction and the extra diagonal is what picks the minimum-energy answer out
/// of the null space.
fn zf_equaliser(h: &[C], load: f32) -> Option<Vec<C>> {
    let n = LS_EQ;
    let centre = (n - 1) / 2;
    // r[m] = Σ conj(h[k])·h[k+m], and r[-m] = conj(r[m]).
    let mut r = vec![C::default(); n];
    for (m, slot) in r.iter_mut().enumerate() {
        let mut acc = C::default();
        for k in 0..h.len() {
            if let Some(v) = h.get(k + m) {
                acc = acc.add(h[k].conj().mul(*v));
            }
        }
        *slot = acc;
    }
    let mut ata = vec![vec![C::default(); n]; n];
    for i in 0..n {
        for j in 0..n {
            ata[i][j] = if j >= i { r[j - i] } else { r[i - j].conj() };
        }
        ata[i][i] = ata[i][i].add(C::new(load, 0.0));
    }
    let mut atb = vec![C::default(); n];
    for (i, slot) in atb.iter_mut().enumerate() {
        let tapped = i as isize - centre as isize;
        let want = -tapped;
        if want >= 0
            && let Some(v) = h.get(want as usize)
        {
            *slot = v.conj();
        }
    }
    solve(ata, atb)
}

/// The centres of the three reference blocks, in symbols from the frame head.
const REF_CENTRES: [f32; 3] = [
    (REF_BLOCKS[0].0 + REF_BLOCKS[0].1 - 1) as f32 / 2.0,
    (REF_BLOCKS[1].0 + REF_BLOCKS[1].1 - 1) as f32 / 2.0,
    (REF_BLOCKS[2].0 + REF_BLOCKS[2].1 - 1) as f32 / 2.0,
];

impl ChannelFit {
    fn from_references(z: &[C; SCRAMBLED]) -> Self {
        let mut g = [C::default(); 3];
        for (i, &(lo, hi)) in REF_BLOCKS.iter().enumerate() {
            let mut acc = C::default();
            for k in lo..hi {
                acc = acc.add(z[k]);
            }
            // The mean, not the sum: the level of the reference is the level the
            // data has to be brought to.
            g[i] = acc.scale(1.0 / (hi - lo) as f32);
        }
        let mean_c = (REF_CENTRES[0] + REF_CENTRES[1] + REF_CENTRES[2]) / 3.0;
        let mean_g = g[0].add(g[1]).add(g[2]).scale(1.0 / 3.0);
        let mut num = C::default();
        let mut den = 0.0f32;
        for i in 0..3 {
            let dc = REF_CENTRES[i] - mean_c;
            num = num.add(g[i].sub(mean_g).scale(dc));
            den += dc * dc;
        }
        let slope = if den > 1e-6 { num.scale(1.0 / den) } else { C::default() };
        let intercept = mean_g.sub(slope.scale(mean_c));
        ChannelFit { slope, intercept }
    }

    /// The channel at symbol `k` of the frame (0 is the first data symbol).
    fn at(&self, k: f32) -> C {
        self.slope.scale(k).add(self.intercept)
    }

    /// One symbol, corrected to the reference's phase **and** level.
    fn apply(&self, k: usize, z: C) -> C {
        let g = self.at(k as f32);
        let n2 = g.norm().powi(2);
        if n2 > 1e-9 { z.mul(g.conj()).scale(1.0 / n2) } else { z }
    }
}

/// The three reference blocks' concentrations with a carrier offset removed
/// inside each block.
fn ramp_concentrations(z: &[C], hz: f32) -> [f32; 3] {
    let mut out = [0.0f32; 3];
    for (i, &(lo, hi)) in REF_BLOCKS.iter().enumerate() {
        let mut sum = C::default();
        let mut mag = 0.0f32;
        for k in lo..hi {
            let t = k as f32 / BAUD as f32;
            let c = z[k].mul(C::from_phase(-hz * TAU * t));
            sum = sum.add(c);
            mag += c.norm();
        }
        out[i] = if mag > 1e-12 { (sum.norm() / mag).min(1.0) } else { 0.0 };
    }
    out
}

/// Symbol phases to payload bits, oldest first, by Annex A's tables.
///
/// The symbols are descrambled and quantised to the nearest of eight phases, and
/// the table is chosen by the rate the constellation implied. At
/// [`S4285Rate::Bps3600OrCoded`] the 8PSK table is used, which is right for the
/// uncoded 3600 rate and is a best-effort tribit split for the coded ones — the
/// raw bits are exposed so that a coded frame is visible as wrong bits rather
/// than as silence.
fn transcode(data: &[C], rate: S4285Rate) -> Vec<u8> {
    let mut bits = Vec::with_capacity(data.len() * 3);
    for z in data {
        let idx = ((z.arg() / std::f32::consts::FRAC_PI_4).round() as i32).rem_euclid(8) as u8;
        match rate {
            S4285Rate::Bps1200 => bits.push(u8::from(idx != 0)),
            S4285Rate::Bps2400 => match idx {
                0 => bits.extend_from_slice(&[0, 0]),
                2 => bits.extend_from_slice(&[0, 1]),
                6 => bits.extend_from_slice(&[1, 0]),
                _ => bits.extend_from_slice(&[1, 1]),
            },
            _ => match idx {
                1 => bits.extend_from_slice(&[0, 0, 0]),
                0 => bits.extend_from_slice(&[0, 0, 1]),
                2 => bits.extend_from_slice(&[0, 1, 0]),
                3 => bits.extend_from_slice(&[0, 1, 1]),
                6 => bits.extend_from_slice(&[1, 0, 0]),
                7 => bits.extend_from_slice(&[1, 0, 1]),
                5 => bits.extend_from_slice(&[1, 1, 0]),
                _ => bits.extend_from_slice(&[1, 1, 1]),
            },
        }
    }
    bits
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Both generators are the standard's, and both got wrong once: a register
    /// given one stage too many empties out and stays at zero, which made the
    /// sync sequence periodic with 16 and the scrambler a constant. Nothing
    /// failed loudly — the sync correlated *better* against the degenerate
    /// pattern and the references simply never collapsed, which read as "no frame
    /// in this file".
    #[test]
    fn the_generators_are_maximal_and_match_the_standard() {
        let sync = sync_symbols();
        // PN31: two periods plus the first 18, and the period really is 31.
        let bits: Vec<i8> = sync.iter().map(|v| if *v > 0.0 { 1 } else { 0 }).collect();
        assert_eq!(&bits[..31], &bits[31..62], "the sync is not periodic with 31");
        assert_ne!(&bits[..31], &bits[16..47], "the sync has a shorter period than 31");
        // The first ten symbols, from the register `11010` clocked as specified.
        assert_eq!(&bits[..10], &[0, 1, 0, 1, 1, 0, 0, 1, 1, 1], "sync head");

        let scr = scrambling();
        // A 511-length maximal sequence: over one frame's worth of symbols all
        // eight phases appear, and the sequence is not constant.
        let mut seen = [false; 8];
        for z in scr {
            let n = ((z.arg() / std::f32::consts::FRAC_PI_4).round() as i32).rem_euclid(8) as usize;
            seen[n] = true;
        }
        assert!(seen.iter().filter(|s| **s).count() >= 7, "the scrambler is not maximal");
        // The first symbol of the sequence: the register is all zeros but its
        // lowest stage, so n = 1 and the phase is one step.
        assert_eq!(((scr[0].arg() / std::f32::consts::FRAC_PI_4).round() as i32) & 7, 1);
    }

    /// Is this scrambled offset a reference symbol?
    fn is_reference(k: usize) -> bool {
        REF_BLOCKS.iter().any(|&(lo, hi)| k >= lo && k < hi)
    }

    /// A frame-accurate 4285 BPSK transmitter, for the regression test only.
    ///
    /// Raised-cosine pulses with 30 % excess bandwidth — the standard's 3000 Hz
    /// occupied bandwidth at 2400 baud — so the loopback is a fair signal and not
    /// a rectangular-pulse spectrum the receiver's own filter would have to
    /// truncate. Frame `n` carries `bits[n*128 ..]`, so the payload is a known
    /// sequence rather than noise.
    fn modulate(bits: &[u8], frames: usize, rate: f64, ramp_hz_s: f64) -> Vec<f32> {
        let sync = sync_symbols();
        let scr = scrambling();
        // Symbol phases on a 5-sample grid, then pulse-shaped.
        let mut sym = Vec::with_capacity(frames * FRAME * 5);
        for f in 0..frames {
            let mut frame = [0.0f32; FRAME];
            for (i, s) in sync.iter().enumerate() {
                frame[i] = if *s > 0.0 { 0.0 } else { 4.0 };
            }
            let mut d = 0;
            for k in 0..SCRAMBLED {
                let sym_phase = if is_reference(k) {
                    0.0
                } else {
                    let b = bits[(f * 128 + d) % bits.len().max(1)];
                    d += 1;
                    if b == 1 { 4.0 } else { 0.0 }
                };
                frame[SYNC + k] =
                    (sym_phase + scr[k].arg() / std::f32::consts::FRAC_PI_4).rem_euclid(8.0);
            }
            for v in frame {
                sym.push(C::from_phase(v * std::f32::consts::FRAC_PI_4));
                for _ in 1..5 {
                    sym.push(C::default());
                }
            }
        }
        // Raised cosine, alpha 0.3, five symbols either side.
        let alpha = 0.3f32;
        let taps: Vec<f32> = (-20i32..=20)
            .map(|i| {
                let x = i as f32 / 5.0;
                let sinc = if x.abs() < 1e-6 {
                    1.0
                } else {
                    (std::f32::consts::PI * x).sin() / (std::f32::consts::PI * x)
                };
                let d = 1.0 - (2.0 * alpha * x).powi(2);
                let rc = if d.abs() < 1e-6 {
                    std::f32::consts::FRAC_PI_4 * (std::f32::consts::PI * x / 2.0).sin()
                } else {
                    (std::f32::consts::PI * alpha * x).cos() / d
                };
                sinc * rc
            })
            .collect();
        let mut bb = vec![C::default(); sym.len()];
        for (i, s) in sym.iter().enumerate() {
            if s.norm() < 1e-9 {
                continue;
            }
            for (j, t) in taps.iter().enumerate() {
                let k = i as isize + j as isize - 20;
                if k >= 0 && (k as usize) < bb.len() {
                    bb[k as usize] = bb[k as usize].add(s.scale(*t));
                }
            }
        }
        bb.iter()
            .enumerate()
            .map(|(i, z)| {
                // The phase is accumulated in f64 and reduced: `TAU * f * i` in
                // f32 loses its fractional part within a second and the carrier
                // lands on the wrong frequency entirely.
                let t = i as f64 / rate;
                // `ramp_hz_s` is a carrier *rate* error, in hertz per second: the
                // phase is quadratic in time, which is what a drifting station and
                // a free-running sound card both look like.
                let ph = (TAU as f64 * (1800.0 * t + 0.5 * ramp_hz_s * t * t))
                    .rem_euclid(TAU as f64) as f32;
                0.5 * z.mul(C::from_phase(ph)).re
            })
            .collect()
    }

    #[test]
    fn a_synthetic_frame_locks_and_its_references_collapse() {
        // A pseudo-random payload, so the data cannot pass for references. A
        // plain multiplicative hash has period 256 here and makes frame *n+1*
        // the exact complement of frame *n*, which cancels the sync in the frame
        // ACF — that fixture was the thing that was wrong, not the receiver.
        let mut seed = 0x2545_f491u32;
        let bits: Vec<u8> = (0..4096)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                (seed & 1) as u8
            })
            .collect();
        // Noise, because a noiseless loopback hides the decoder's own faults: a
        // chunk's forced start state is corrected by the metric within a few
        // constraint lengths when the symbols are perfect, and only bites when
        // they are not. Ten decibels is what Annex E's own tables say a 600 bps
        // rate needs (BER 3.4e-4 at 10 dB in 3 kHz).
        let mut audio = modulate(&bits, 40, 12000.0, 0.0);
        let rms = (audio.iter().map(|v| v * v).sum::<f32>() / audio.len() as f32).sqrt();
        let noise = rms / 10f32.powf(10.0 / 20.0) * 1.4;
        let mut seed = 0x5EED_1234u32;
        for v in audio.iter_mut() {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            // A sum of four uniforms is close enough to Gaussian for a floor.
            let mut n = 0.0f32;
            for _ in 0..4 {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                n += (seed >> 8) as f32 / 16_777_216.0 - 0.5;
            }
            *v += noise * n;
        }
        let mut rx = S4285Receiver::new(12000.0, 1800.0);
        for chunk in audio.chunks(1024) {
            rx.process(chunk);
        }
        let r = rx.report();
        assert!(r.locked, "a clean synthetic frame did not lock: {:?}", r);
        assert!(r.frames > 8, "only {} frames demodulated", r.frames);
        for (i, c) in r.reference.iter().enumerate() {
            assert!(*c > 0.85, "reference block {i} read {c:.3}, and it should collapse");
        }
        assert!(r.frame_acf > 0.25, "frame ACF {:.3}", r.frame_acf);
    }

    /// The whole coded chain, end to end: SOM + payload + EOM, convolutional
    /// code, pair repetition, interleaver, BPSK modulation — through the receiver,
    /// the deinterleaver, the Viterbi decoder and the framer.
    ///
    /// This is the regression test for the plumbing. It cannot prove the
    /// interleaver's row structure against the standard (a loopback shares whatever
    /// convention both halves use); what proves that is a start-of-message pattern
    /// on real traffic, which is what the off-air captures are for.
    fn coded_round_trip(ramp_hz_s: f64) -> (Vec<u8>, Vec<S4285Message>) {
        coded_round_trip_band(ramp_hz_s, None)
    }

    /// The same, through an audio chain that only passes `band`.
    fn coded_round_trip_band(
        ramp_hz_s: f64,
        band: Option<(f64, f64)>,
    ) -> (Vec<u8>, Vec<S4285Message>) {
        coded_round_trip_path(ramp_hz_s, band, None)
    }

    /// The same, through a multipath channel as well.
    #[allow(clippy::too_many_lines)]
    fn coded_round_trip_path(
        ramp_hz_s: f64,
        band: Option<(f64, f64)>,
        taps: Option<&[(f64, f32)]>,
    ) -> (Vec<u8>, Vec<S4285Message>) {
        use super::fec::{self, CodedRate};
        let rate = CodedRate::Bps600;
        // A payload whose bits are not all the same, so a decoder that drops
        // them cannot pass by luck.
        // Long enough to cross several of the decoder's chunks: the overlap that
        // protects a chunk's opening is only exercised when a message spans more
        // than one, and a 128-bit payload fits inside a single 256-pair chunk.
        let payload: Vec<u8> = (0..600u32).map(|i| ((i * 5 + (i >> 2)) % 3 == 0) as u8).collect();
        let mut msg = vec![0u8; 6]; // six zero bits, so the SOM is not at the head
        for k in (0..32).rev() {
            msg.push(((fec::SOM >> k) & 1) as u8);
        }
        msg.extend_from_slice(&payload);
        for k in (0..32).rev() {
            msg.push(((fec::EOM >> k) & 1) as u8);
        }
        // Annex E's flush: zeros to the interleaver's delay plus 102.
        msg.extend(std::iter::repeat_n(0u8, 6246));

        let coded = fec::encode(&msg);
        let repeated = fec::repeat_pairs(&coded, rate.repeat());
        let mut il = fec::ConvInterleaver::interleaver(rate.j());
        let sent: Vec<u8> = repeated
            .iter()
            .map(|b| {
                let v = il.push(if *b == 1 { -1.0 } else { 1.0 });
                if v < 0.0 { 1 } else { 0 }
            })
            .collect();
        // One frame's worth of padding so the last bits are carried.
        let frames = sent.len() / 128 + 2;
        let mut audio = modulate(&sent, frames, 12000.0, ramp_hz_s);
        // Noise, because a noiseless loopback hides the decoder's own faults: the
        // forced start state at the front of each chunk is pulled right by the
        // metric within a few constraint lengths when the symbols are perfect, and
        // only bites when they are not. Ten decibels is what Annex E's own tables
        // say 600 bps needs (3.4e-4 at 10 dB in 3 kHz).
        {
            let rms = (audio.iter().map(|v| v * v).sum::<f32>() / audio.len() as f32).sqrt();
            // The sum of four uniforms has σ = 1/√3, so scale to a real 10 dB.
            let amp = rms / 10f32.powf(10.0 / 20.0) / (4.0f32 / 12.0).sqrt();
            let mut seed = 0x5EED_1234u32;
            for v in audio.iter_mut() {
                let mut n = 0.0f32;
                for _ in 0..4 {
                    seed ^= seed << 13;
                    seed ^= seed >> 17;
                    seed ^= seed << 5;
                    n += (seed >> 8) as f32 / 16_777_216.0 - 0.5;
                }
                *v += amp * n;
            }
        }

        // The chain in front of the receiver: a rig's filter, a sound card, a
        // WebSDR's audio. Applied after the noise so a truncated band is the only
        // thing being measured.
        let audio = match band {
            Some((lo, hi)) => band_limit(&audio, 12000.0, lo, hi),
            None => audio,
        };
        // A real path arrives as several delayed copies of itself: the impairment
        // the standard's own tables budget for ("two equal Rayleigh paths 2 ms
        // apart cost roughly 5 dB") and the one a receiver answers with an
        // equaliser.
        let audio = match taps {
            Some(t) => multipath(&audio, t),
            None => audio,
        };
        let mut rx = S4285Receiver::new(12000.0, 1800.0);
        let mut got: Vec<S4285Message> = Vec::new();
        for chunk in audio.chunks(1024) {
            for ev in rx.process(chunk) {
                if let S4285Event::Message(m) = ev {
                    got.push(m);
                }
            }
        }
        if got.is_empty() {
            eprintln!("no coded message at {ramp_hz_s} Hz/s, band {band:?}: {:?}", rx.report());
        }
        (payload, got)
    }

    /// A steep band-pass, to model the audio chain in front of a real capture.
    ///
    /// A rig's SSB filter, a sound card's anti-alias filter and a WebSDR's audio
    /// stream all truncate the 300–3300 Hz an STANAG 4285 occupies, and the
    /// question this measures is how much of that band the coded chain actually
    /// needs. Hann-windowed, 401 taps at 12 kHz: a transition of about 60 Hz, so
    /// a corner is where it says it is rather than a slope.
    fn band_limit(audio: &[f32], rate: f64, lo: f64, hi: f64) -> Vec<f32> {
        const TAPS: usize = 401;
        let mid = (TAPS - 1) as f64 / 2.0;
        let mut h = vec![0.0f32; TAPS];
        for (i, v) in h.iter_mut().enumerate() {
            let n = i as f64 - mid;
            let hann = 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / (TAPS - 1) as f64).cos();
            // An ideal low-pass of cutoff f is 2f/fs·sinc(2fn/fs); a band-pass is
            // the difference of two, and both have unity gain in their flat region.
            let lp = |f: f64| {
                let x = std::f64::consts::PI * 2.0 * f * n / rate;
                let s = if x.abs() < 1e-9 { 1.0 } else { x.sin() / x };
                2.0 * f / rate * s
            };
            *v = ((lp(hi) - lp(lo)) * hann) as f32;
        }
        let mut out = vec![0.0f32; audio.len()];
        for (k, o) in out.iter_mut().enumerate() {
            let start = k.saturating_sub(TAPS - 1);
            let mut acc = 0.0f32;
            for j in start..=k {
                acc += audio[j] * h[k - j];
            }
            *o = acc;
        }
        out
    }

    /// A multipath channel: the sum of `taps` scaled and delayed copies.
    ///
    /// `delay` is in **symbols**, at 2400 baud on 12 kHz audio, and a fractional
    /// delay is interpolated — crude, but the point is to place energy a symbol
    /// or two after the symbol that caused it, which is what a real path does and
    /// what a phase fit cannot undo.
    fn multipath(audio: &[f32], taps: &[(f64, f32)]) -> Vec<f32> {
        let mut out = vec![0.0f32; audio.len()];
        for &(delay, gain) in taps {
            let d = delay * (12000.0 / BAUD);
            let whole = d.floor() as usize;
            let frac = (d - whole as f64) as f32;
            for (k, v) in out.iter_mut().enumerate() {
                let a = k.checked_sub(whole).map(|i| audio[i]).unwrap_or(0.0);
                let b = k.checked_sub(whole + 1).map(|i| audio[i]).unwrap_or(0.0);
                *v += gain * (a * (1.0 - frac) + b * frac);
            }
        }
        out
    }

    /// The whole chain, over a channel that is exactly on frequency.
    #[test]
    fn a_coded_message_survives_the_whole_chain() {
        use super::fec::CodedRate;
        let (payload, got) = coded_round_trip(0.0);
        let m = &got[0];
        assert_eq!(m.rate, CodedRate::Bps600, "the wrong rate named the message");
        assert!(m.closed, "the EOM did not close the message");
        assert_eq!(m.bits, payload, "the message bits are not the ones that were sent");
    }

    /// The same chain on a channel that is **not** exactly on frequency.
    ///
    /// A 1 Hz/s ramp is well inside the 3.5 Hz/s the standard allows, and it is
    /// the case that used to break the receiver outright: the soft decisions
    /// handed to Annex E kept the frame's residual phase, so the bits going into
    /// the code arrived rotated by a different amount every frame while the
    /// constellation still read 0.99 and the references still collapsed at 0.998.
    /// Verified failing against the code before `feed_coded` was given the same
    /// channel line the frame itself is read through.
    #[test]
    fn a_coded_message_survives_a_carrier_ramp() {
        for ramp in [1.0f64, 4.0] {
            let (payload, got) = coded_round_trip(ramp);
            assert_eq!(got[0].bits, payload, "the wrong bits at {ramp} Hz/s");
        }
    }

    /// The closed-form equaliser, at the symbol level and with nothing else going
    /// on: estimate the channel from known symbols, invert it, recover the rest.
    ///
    /// This is the piece the adaptive attempt failed at. Two paths, the second a
    /// symbol and a half late at 0.67 — the same channel the audio test builds.
    #[test]
    fn the_least_squares_equaliser_undoes_a_symbol_level_echo() {
        let sync = sync_symbols();
        // The transmitted frame: the sync, then eight-phase symbols on the π/4
        // lattice that a 4285 actually sends.
        let mut sent = [C::default(); FRAME];
        for k in 0..FRAME {
            let p = if k < SYNC {
                if sync[k] > 0.0 { 0.0 } else { std::f32::consts::PI }
            } else {
                ((k * 5) % 8) as f32 * std::f32::consts::FRAC_PI_4
            };
            sent[k] = C::from_phase(p);
        }
        // Two paths: one symbol and a half of delay, split across two taps the
        // way a fractional delay lands on a symbol grid.
        let rx: Vec<C> = (0..FRAME)
            .map(|k| {
                let a = if k >= 1 { sent[k - 1].scale(0.67 * 0.5) } else { C::default() };
                let b = if k >= 2 { sent[k - 2].scale(0.67 * 0.5) } else { C::default() };
                sent[k].add(a).add(b)
            })
            .collect();

        let h = ls_channel(&rx[..SYNC], &sent[..SYNC], LS_TAPS).expect("a channel estimate");
        assert!(h[0].norm() > 0.7, "the direct path is not in the estimate: {:?}", h[0]);
        assert!(h[1].norm() + h[2].norm() > 0.3, "the echo is not in the estimate: {:?}", h);

        let energy: f32 = h.iter().map(|v| v.norm() * v.norm()).sum();
        let w = zf_equaliser(&h, energy * 1e-3).expect("an equaliser");
        let mut worst = 0.0f32;
        let mut clean = 0;
        for k in SYNC..FRAME {
            let mut y = C::default();
            for (i, wi) in w.iter().enumerate() {
                if let Some(v) = k.checked_sub(i).and_then(|j| rx.get(j)) {
                    y = y.add(v.mul(*wi));
                }
            }
            // The equaliser delays by its centre tap, so compare against the
            // symbol that lands there.
            let centre = (LS_EQ - 1) / 2;
            if let Some(want) = k.checked_sub(centre).map(|j| sent[j]) {
                let e = y.sub(want).norm();
                worst = worst.max(e);
                if e < 0.35 {
                    clean += 1;
                }
            }
        }
        println!("LS equaliser: {clean} of {} symbols, worst error {worst:.2}", FRAME - SYNC);
        assert!(
            clean > (FRAME - SYNC) * 9 / 10,
            "only {clean} of {} symbols recovered",
            FRAME - SYNC
        );
    }

    /// **The impairment the one real coded tape has: two paths a symbol apart.**
    ///
    /// This is the definition of done for the next piece of work, and it is
    /// ignored because that work is not built. The receiver cannot put the tape's
    /// data symbols on a lattice (2/4/8-fold **0.234 / 0.070 / 0.080**) while its
    /// sync and its sixteen-identical-symbol reference blocks are healthy, and its
    /// data *blocks* are no better than the frame — so the damage is inside 32
    /// symbols, which is delay spread and not a phase drift. This reproduces it on
    /// a synthetic whose bits are known: the same coded transmission plus a copy
    /// of itself 1.5 symbols late at two thirds the amplitude.
    ///
    /// **Where it stands.** The channel is estimated from the 80 known sync
    /// symbols by least squares and inverted in closed form
    /// (`the_least_squares_equaliser_undoes_a_symbol_level_echo` recovers 176 of
    /// 176 symbols through this channel at the symbol level), and the receiver uses
    /// it on any frame where it makes the known reference symbols collapse harder
    /// than they already do, so it cannot make a clean frame worse.
    ///
    /// **What is missing, and one thing now ruled out.** On pulse-shaped audio the
    /// equaliser's output never beats the plain path, whatever the filter's group
    /// delay (which is searched over three) — so the gate declines it and the frame
    /// reads as before. The timing criterion was the obvious suspect and is now
    /// **in and ruled out**: the sampling phase is chosen by the least-squares
    /// residual over the known sync (`sync_fit`), which has a transition on almost
    /// every symbol and so cannot be circular the way data tightness is, and it
    /// picks the *same* instant the old criterion did (0.60) on this channel. So
    /// the model is what is short, not the eye: a T-spaced FIR fitted to the sync
    /// does not describe a raised-cosine signal sampled off a two-path channel well
    /// enough to invert, and the next attempt should estimate the channel
    /// **including the pulse** — a longer, fractionally-spaced estimate — rather
    /// than a five-tap T-spaced one.
    ///
    /// **What the first attempt established.** An adaptive
    /// (NLMS) fractionally-spaced FFE+DFE trained on the 80 known sync symbols does
    /// **not** converge here: the taps stall at the delay line's gain and the
    /// training residual sits at 0.88 against a unit target, which measurably
    /// *lowers* the reference concentration instead of raising it. The inputs are
    /// not the reason — measured, the sync is recovered at the current timing
    /// exactly (its correlation equals its mean amplitude, sign for sign), so a
    /// closed-form estimate is the way in: a least-squares channel from the 80
    /// known symbols, then a computed inverse or a DFE seeded with it, rather than
    /// a loop left to find it.
    #[test]
    #[ignore = "the equaliser works at the symbol level; on pulse-shaped audio the model is too short — see the note"]
    fn a_coded_message_survives_a_two_path_channel() {
        // -3.5 dB of echo 1.5 symbols late: a symbol is 0.417 ms, so this is
        // 0.63 ms of delay spread, the low end of what a 13 MHz path does.
        let taps = [(0.0, 1.0f32), (1.5, 0.67)];
        let (payload, got) = coded_round_trip_path(0.0, Some((300.0, 3300.0)), Some(&taps));
        assert!(!got.is_empty(), "a two-path channel lost the message entirely");
        assert_eq!(got[0].bits, payload, "the bits do not survive two paths");
    }

    /// **How much of the 300–3300 Hz band does the coded chain need?**
    ///
    /// The question the one real coded capture we were sent turned on. Mark's FUF
    /// tape (13031.2 kHz, 23 Oct) arrives with its audio running about 800–3000 Hz
    /// where an STANAG 4285 occupies 300–3300, and every Annex E branch sits at
    /// its shuffled null on it — so either the missing third of the waveform is
    /// what kills the code, or the band is a coincidence and the fault is
    /// elsewhere.
    ///
    /// **It is not the band.** Measured on a synthetic whose bits are known, the
    /// chain decodes the message exactly through every one of these:
    ///
    /// | high-pass | low-pass | |
    /// |---|---|---|
    /// | 300, 500, 650, 800, 900, 1000 Hz | 3300 | decodes |
    /// | 300 | 3300, 3100, 3000, 2800, 2600 | decodes |
    /// | 800 | 3000 — his tape's own band | decodes |
    ///
    /// So a receiver whose audio chain is narrower than the standard's own
    /// 3000 Hz is not what stops a coded tape being read, and asking a reporter
    /// for a wider filter on that evidence would have been a wrong errand. The
    /// two corners are pinned here; the rest of the table is what the run that
    /// found this printed.
    #[test]
    fn the_coded_chain_survives_a_truncated_audio_band() {
        let bits_of = |band: (f64, f64)| {
            let (payload, got) = coded_round_trip_band(0.0, Some(band));
            got.first().is_some_and(|m| m.bits == payload)
        };
        assert!(bits_of((300.0, 3300.0)), "the chain lost a message inside its own band");
        assert!(
            bits_of((800.0, 3000.0)),
            "a chain band-limited the way a real capture's audio is no longer decodes — \
             which is the measurement this test exists for"
        );
        // The corners, so a front end that needs more than this fails loudly
        // rather than being discovered on the air.
        assert!(
            bits_of((1000.0, 3300.0)),
            "the low corner moved: 1000 Hz high-pass used to decode"
        );
        assert!(bits_of((300.0, 2600.0)), "the high corner moved: 2600 Hz low-pass used to decode");
    }

    /// The equaliser corrects **level** as well as phase, over the whole frame.
    ///
    /// This is what the first real coded capture we were sent needed: its audio
    /// filter tilts the signal across the 107 ms frame, so a straight phase line
    /// leaves the data smeared while the references still collapse (references
    /// 0.69 / 0.76 / 0.61, constellation 0.34). The fit is read from the three
    /// reference blocks and has to hold out to the **last** data block, which has
    /// no reference block after it — that is the extrapolated end, and it is the
    /// half a naively-interpolated fit gets wrong.
    #[test]
    fn the_channel_fit_recovers_level_and_phase_across_the_frame() {
        // A channel that is linear in I/Q — which is what the fit models — with a
        // level that falls from 1.22 to 0.86 and a phase that swings across the
        // frame. Written this way the fit is exact, so the test measures the
        // correction rather than the interpolation error.
        let g = |k: f32| {
            let t = k / 176.0;
            C::new(1.22 - 0.36 * t, 0.40 - 0.55 * t)
        };
        let mut z = [C::default(); SCRAMBLED];
        for k in 0..SCRAMBLED {
            // References are symbol 0, i.e. +1: the fit reads the channel from
            // them. Data carries a +1 BPSK symbol, so it must come back as +1.
            z[k] = g(k as f32);
        }
        let fit = ChannelFit::from_references(&z);
        for k in [0usize, 31, 48, 79, 96, 127, 144, 175] {
            let c = fit.apply(k, z[k]);
            assert!(
                (c.norm() - 1.0).abs() < 0.02,
                "symbol {k}: level {:.3}, not corrected",
                c.norm()
            );
            assert!(c.arg().abs() < 0.04, "symbol {k}: phase {:.3} rad, not corrected", c.arg());
        }
    }

    /// The negative that makes the lock a detector rather than a lamp: noise must
    /// not lock, and neither must a strong carrier of a different shape.
    #[test]
    fn noise_and_a_plain_carrier_do_not_lock() {
        let mut seed = 0x1234_5678u32;
        let mut noise = Vec::new();
        for _ in 0..12 * 12000 {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
            noise.push(((seed >> 16) as f32 / 32768.0 - 1.0) * 0.2);
        }
        let mut rx = S4285Receiver::new(12000.0, 1800.0);
        for chunk in noise.chunks(1024) {
            rx.process(chunk);
        }
        assert!(!rx.report().locked, "white noise locked: {:?}", rx.report());

        // A single steady tone at the carrier: a strong carrier is not a frame.
        let tone: Vec<f32> = (0..12 * 12000)
            .map(|i| 0.5 * (TAU as f32 * 1800.0 * i as f32 / 12000.0).cos())
            .collect();
        let mut rx = S4285Receiver::new(12000.0, 1800.0);
        for chunk in tone.chunks(1024) {
            rx.process(chunk);
        }
        assert!(!rx.report().locked, "a plain carrier locked: {:?}", rx.report());
    }

    /// The material this was built against: a real station off the air. Not in
    /// the tree — the capture is the operator's — so it runs only when pointed at
    /// one, exactly as the other off-air gates here do.
    ///
    ///     SDROXIDE_S4285_SAMPLE=~/s4285-captures/6348.000kHz-20261022-045545Z.wav \
    ///       cargo test -p sdroxide-dsp --release -- --ignored --nocapture an_off_air_capture
    #[test]
    #[ignore]
    fn an_off_air_capture_locks_and_its_references_collapse() {
        let path = std::env::var("SDROXIDE_S4285_SAMPLE").expect("set SDROXIDE_S4285_SAMPLE");
        let mut reader = hound::WavReader::open(&path).expect("open the wav");
        let spec = reader.spec();
        let raw: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.unwrap()).collect(),
            hound::SampleFormat::Int => {
                reader.samples::<i16>().map(|s| s.unwrap() as f32 / 32_768.0).collect()
            }
        };
        // A capture can be stereo, and its samples are then interleaved: read as
        // mono, the rate is effectively halved and two channels are spliced into
        // one stream, so nothing locks and this gate measures nothing. The real
        // coded capture we were sent was stereo for exactly that reason.
        let ch = spec.channels.max(1) as usize;
        let audio: Vec<f32> = if ch == 1 {
            raw
        } else {
            raw.chunks(ch).map(|f| f.iter().sum::<f32>() / ch as f32).collect()
        };
        let mut rx = S4285Receiver::new(spec.sample_rate as f64, 1800.0);
        let mut refs = [0.0f64; 3];
        let mut n = 0.0f64;
        for chunk in audio.chunks(4096) {
            for ev in rx.process(chunk) {
                if let S4285Event::Frame(f) = ev {
                    for i in 0..3 {
                        refs[i] += f.reference[i] as f64;
                    }
                    n += 1.0;
                }
            }
        }
        let r = rx.report();
        println!("{:?}", r);
        assert!(n > 50.0, "only {n} frames demodulated");
        let mean = [refs[0] / n, refs[1] / n, refs[2] / n];
        println!("mean references {mean:?}");
        for (i, c) in mean.iter().enumerate() {
            assert!(*c > 0.55, "reference block {i} averaged {c:.3} over {n} frames");
        }
    }
}
