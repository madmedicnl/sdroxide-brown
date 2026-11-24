//! An open receiver: tuning, attenuation, and the sample ring the library fills.
//!
//! The library owns its own USB thread and calls [`on_input`] from it with raw
//! buffers of **6 bytes per I/Q pair** — three bytes of I, then three of Q,
//! little-endian, 24-bit signed. That layout is not something to infer from the
//! header: it is what the vendor's own `examples/perseustest.c` consumes (its
//! `iq_sample` union leaves the low byte at zero and reads the three buffer
//! bytes into the top of the word the other way round), and it is what
//! [`i24`] implements, with the byte order pinned by a test.
//!
//! Everything else here is a thin, honest layer: the setters call the library
//! directly (it serialises internally — the vendor example calls them from the
//! main thread while its callback runs), and the ring is the only place this
//! crate stores samples.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tracing::{debug, info};

use crate::device::{self, Opened};
use crate::error::Error;
use crate::ffi::{ATT_STEPS_DB, Api, InputCb};

/// The buffer the library is asked to collect per callback. Its own
/// documentation suggests `6 * 1024` bytes — 1024 I/Q pairs, which at 2 MS/s is
/// half a millisecond, small enough that the ring is never bursty and large
/// enough that the per-call overhead does not matter.
const TRANSFER_BYTES: u32 = 6 * 1024;

/// How the receiver is opened.
#[derive(Debug, Clone, PartialEq)]
pub struct OpenParams {
    /// Ordinal from [`device::list`].
    pub index: u32,
    pub center_hz: f64,
    pub rate_hz: f64,
    /// 0, 10, 20 or 30 dB, snapped to the switch's own positions.
    pub att_db: u8,
    /// The preselector is only usable above roughly 1.5 MHz; the panel decides
    /// and this is what it decided.
    pub preselector: bool,
    /// Hardware dithering in the ADC (`perseus_set_adc`).
    pub dither: bool,
    /// The ADC's preamplifier, which is the receiver's second gain stage.
    pub preamp: bool,
}

/// The context the library's callback is handed. It outlives every callback:
/// `stop_async_input` returns only once the library has cancelled its transfers,
/// and the `Arc` in [`PerseusHandle`] is dropped afterwards.
struct Ctx {
    producer: Mutex<rtrb::Producer<f32>>,
    lost: AtomicU64,
    /// Milliseconds since `born`, written by the callback so a silent receiver
    /// can be told from a slow one.
    last_ms: AtomicU64,
    alive: AtomicBool,
    born: Instant,
}

/// `PERSEUS_ATT_*` ordinal for a requested attenuation, rounding to the nearest
/// switch position.
pub fn att_index(db: u8) -> usize {
    let mut best = 0;
    let mut dist = i32::MAX;
    for (i, step) in ATT_STEPS_DB.iter().enumerate() {
        let d = (i32::from(db) - step).abs();
        if d < dist {
            dist = d;
            best = i;
        }
    }
    best
}

/// One 24-bit little-endian sample as a float in −1.0 … +1.0.
///
/// The vendor example scales the same value by `INT_MAX - 256` after shifting
/// it into the top of an `i32`; dividing the sign-extended 24-bit value by 2^23
/// is the same number without the shift, and it is exact in `f32` up to the
/// last bit.
fn i24(b0: u8, b1: u8, b2: u8) -> f32 {
    let v = i32::from(b0) | (i32::from(b1) << 8) | (i32::from(b2) << 16);
    let v = (v << 8) >> 8; // sign-extend 24 → 32 bits
    v as f32 / 8_388_608.0
}

/// The library's callback. No unwinding may cross this boundary, so there is
/// no indexing that can panic and no `unwrap` that can fail loudly: a full ring
/// counts a dropped pair and carries on.
unsafe extern "C" fn on_input(
    buf: *mut std::ffi::c_void,
    buf_size: std::ffi::c_int,
    extra: *mut std::ffi::c_void,
) -> std::ffi::c_int {
    if buf.is_null() || extra.is_null() || buf_size < 6 {
        return 0;
    }
    // SAFETY: `extra` is the `Arc::as_ptr` of the context this handle keeps
    // alive; `buf` is `buf_size` bytes the library owns for the duration of the
    // call.
    let ctx = unsafe { &*(extra as *const Ctx) };
    let bytes = unsafe { std::slice::from_raw_parts(buf as *const u8, buf_size as usize) };
    let mut lost = 0u64;
    {
        let mut prod = ctx.producer.lock().unwrap_or_else(|e| e.into_inner());
        for s in bytes.as_chunks::<6>().0 {
            if prod.slots() < 2 {
                lost += 1;
                continue;
            }
            let _ = prod.push(i24(s[0], s[1], s[2]));
            let _ = prod.push(i24(s[3], s[4], s[5]));
        }
    }
    if lost > 0 {
        ctx.lost.fetch_add(lost, Ordering::Relaxed);
    }
    ctx.last_ms.store(ctx.born.elapsed().as_millis() as u64, Ordering::Relaxed);
    0
}

/// An open Perseus, streaming into a ring.
pub struct PerseusHandle {
    opened: Opened,
    consumer: rtrb::Consumer<f32>,
    ctx: Arc<Ctx>,
    rate_hz: f64,
    center: f64,
    att_db: u8,
    presel: bool,
    dither: bool,
    preamp: bool,
    streaming: bool,
    label: String,
}

// SAFETY: the descriptor is only ever used from this struct's own thread (the
// source thread), and the library serialises its internal state. The callback
// runs on the library's thread and touches only the `Arc<Ctx>`, which is Send +
// Sync.
unsafe impl Send for PerseusHandle {}

impl PerseusHandle {
    pub fn open(p: &OpenParams) -> Result<PerseusHandle, Error> {
        let opened = device::open(p.index)?;
        let api = Arc::clone(&opened.api);

        // One second of interleaved I/Q at the opened rate, capped so a 2 MS/s
        // receiver does not ask for a 16 MB ring on a small machine.
        let pairs = (p.rate_hz as usize).clamp(48_000, 2_000_000);
        let (producer, consumer) = rtrb::RingBuffer::<f32>::new(pairs * 2);
        let ctx = Arc::new(Ctx {
            producer: Mutex::new(producer),
            lost: AtomicU64::new(0),
            last_ms: AtomicU64::new(0),
            alive: AtomicBool::new(true),
            born: Instant::now(),
        });

        let mut h = PerseusHandle {
            opened,
            consumer,
            ctx,
            rate_hz: p.rate_hz,
            center: p.center_hz,
            att_db: ATT_STEPS_DB[att_index(p.att_db)] as u8,
            presel: p.preselector,
            dither: p.dither,
            preamp: p.preamp,
            streaming: false,
            label: String::new(),
        };
        h.apply_rate(p.rate_hz)?;
        h.apply_center(p.center_hz)?;
        h.apply_att()?;
        h.apply_adc()?;
        h.start(&api)?;

        let serial = h.opened.serial.clone();
        h.label = if serial.is_empty() {
            format!("Perseus SDR #{} @ {:.3} Msps", p.index + 1, h.rate_hz / 1e6)
        } else {
            format!("Perseus SDR {serial} @ {:.3} Msps", h.rate_hz / 1e6)
        };
        info!(device = %h.label, center_hz = h.center, "perseus streaming");
        Ok(h)
    }

    fn start(&mut self, api: &Api) -> Result<(), Error> {
        // SAFETY: the descriptor is open; the context pointer outlives the
        // stream because `self` holds the only strong reference and drops it
        // after `stop_async_input`.
        let rc = unsafe {
            (api.start_async_input)(
                self.opened.descr,
                TRANSFER_BYTES,
                on_input as InputCb,
                Arc::as_ptr(&self.ctx) as *mut std::ffi::c_void,
            )
        };
        if rc < 0 {
            let msg = crate::ffi::error_string(api);
            // Starting twice is not a fault: it means the stream never stopped.
            if rc == crate::ffi::ERR_ASYNCSTARTED {
                self.streaming = true;
                return Ok(());
            }
            return Err(Error::Driver(msg));
        }
        self.streaming = true;
        Ok(())
    }

    fn apply_rate(&mut self, hz: f64) -> Result<(), Error> {
        // SAFETY: descriptor open, value in range.
        let rc = unsafe { (self.opened.api.set_sampling_rate)(self.opened.descr, hz as i32) };
        if rc < 0 {
            return Err(Error::Driver(crate::ffi::error_string(&self.opened.api)));
        }
        Ok(())
    }

    fn apply_center(&mut self, hz: f64) -> Result<(), Error> {
        // SAFETY: descriptor open; the second argument is the preselector
        // switch, 1 = in circuit.
        let rc = unsafe {
            (self.opened.api.set_ddc_center_freq)(self.opened.descr, hz, i32::from(self.presel))
        };
        if rc < 0 {
            return Err(Error::Driver(crate::ffi::error_string(&self.opened.api)));
        }
        Ok(())
    }

    fn apply_att(&mut self) -> Result<(), Error> {
        // SAFETY: descriptor open; the ordinal is 0..3, which is what the
        // library's own `PERSEUS_ATT_*` constants are.
        let rc = unsafe {
            (self.opened.api.set_attenuator)(self.opened.descr, att_index(self.att_db) as u8)
        };
        if rc < 0 {
            return Err(Error::Driver(crate::ffi::error_string(&self.opened.api)));
        }
        Ok(())
    }

    /// The rates this receiver's firmware offers, from the library, with the
    /// zero terminator stripped.
    pub fn rates(&self) -> Vec<f64> {
        let mut buf = [0i32; 32];
        // SAFETY: `buf` is 32 live integers and the size says so.
        let rc = unsafe {
            (self.opened.api.get_sampling_rates)(
                self.opened.descr,
                buf.as_mut_ptr(),
                buf.len() as u32,
            )
        };
        if rc < 0 {
            return Vec::new();
        }
        buf.iter().take_while(|v| **v > 0).map(|v| f64::from(*v)).collect()
    }

    pub fn rate_hz(&self) -> f64 {
        self.rate_hz
    }

    pub fn center_hz(&self) -> f64 {
        self.center
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn att_db(&self) -> u8 {
        self.att_db
    }

    /// Drain the ring. Never blocks: a short answer means "nothing yet" and the
    /// reader sleeps, which is what the other USB backends do too.
    pub fn read(&mut self, out: &mut [f32]) -> usize {
        let want = out.len().min(self.consumer.slots());
        let mut n = 0;
        while n < want {
            match self.consumer.pop() {
                Ok(v) => {
                    out[n] = v;
                    n += 1;
                }
                Err(_) => break,
            }
        }
        if n == 0 {
            std::thread::sleep(Duration::from_millis(2));
        }
        n
    }

    pub fn set_center_hz(&mut self, hz: f64) -> Result<(), Error> {
        self.center = hz;
        self.apply_center(hz)
    }

    pub fn set_att_db(&mut self, db: u8) -> Result<(), Error> {
        self.att_db = ATT_STEPS_DB[att_index(db)] as u8;
        self.apply_att()
    }

    /// Hardware dithering and the ADC preamplifier — `perseus_set_adc`, live
    /// while the stream runs.
    pub fn set_dither(&mut self, on: bool) -> Result<(), Error> {
        self.dither = on;
        self.apply_adc()
    }

    pub fn set_preamp(&mut self, on: bool) -> Result<(), Error> {
        self.preamp = on;
        self.apply_adc()
    }

    fn apply_adc(&mut self) -> Result<(), Error> {
        // SAFETY: descriptor open and the stream, if any, is the library's own;
        // this writes the ADC control word it already owns.
        let rc = unsafe {
            (self.opened.api.set_adc)(
                self.opened.descr,
                i32::from(self.dither),
                i32::from(self.preamp),
            )
        };
        if rc < 0 {
            return Err(Error::Driver(crate::ffi::error_string(&self.opened.api)));
        }
        Ok(())
    }

    pub fn dither(&self) -> bool {
        self.dither
    }

    pub fn preamp(&self) -> bool {
        self.preamp
    }

    /// Remember the switch position. Applied with the next tune, because the
    /// library takes the frequency and the switch in one call.
    pub fn set_preselector(&mut self, on: bool) {
        self.presel = on;
    }

    /// Apply the switch on its own, for when nothing else changed.
    pub fn apply_preselector(&mut self) -> Result<(), Error> {
        self.apply_center(self.center)
    }

    /// Pairs the callback could not hand over because the ring was full.
    pub fn lost(&self) -> u64 {
        self.ctx.lost.load(Ordering::Relaxed)
    }

    /// Whether the library is still delivering anything.
    pub fn is_alive(&self) -> bool {
        self.ctx.alive.load(Ordering::Relaxed)
    }

    /// How long since the last callback, which for a streaming receiver is
    /// never long unless the cable or the receiver has gone.
    pub fn silent_for(&self) -> Duration {
        let last = self.ctx.last_ms.load(Ordering::Relaxed);
        if last == 0 {
            // Nothing has arrived since the stream started. Measure from the
            // start rather than reporting "silent since forever", which would
            // reopen instantly.
            return self.ctx.born.elapsed();
        }
        self.ctx.born.elapsed().saturating_sub(Duration::from_millis(last))
    }

    /// Stop the stream and give the receiver back. Idempotent, and also what
    /// `Drop` does.
    pub fn release(&mut self) {
        if !self.streaming {
            return;
        }
        self.streaming = false;
        self.ctx.alive.store(false, Ordering::Relaxed);
        // SAFETY: the descriptor is open and this is the same descriptor the
        // stream was started on. When this returns the library has cancelled
        // its transfers, so no callback can be in flight and the context below
        // may be dropped.
        let stopped = unsafe { (self.opened.api.stop_async_input)(self.opened.descr) };
        if stopped < 0 {
            debug!("perseus stop_async_input: {}", crate::ffi::error_string(&self.opened.api));
        }
        // And the device goes back too: a source that is being *reopened* is
        // released before its replacement is built, and the replacement cannot
        // claim a receiver this one still holds.
        self.opened.close();
    }
}

impl Drop for PerseusHandle {
    fn drop(&mut self) {
        self.release();
        // `opened` closes the descriptor; `ctx` is dropped last, by field order,
        // after the stream has stopped.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sample_is_three_little_endian_bytes_sign_extended() {
        assert_eq!(i24(0x00, 0x00, 0x00), 0.0);
        // +1 - one step: 2^23 - 1.
        assert!((i24(0xFF, 0xFF, 0x7F) - (8_388_607.0 / 8_388_608.0)).abs() < 1e-7);
        assert_eq!(i24(0x00, 0x00, 0x80), -1.0);
        // 0x000100 = 256 → 256 / 2^23
        assert!((i24(0x00, 0x01, 0x00) - 256.0 / 8_388_608.0).abs() < 1e-7);
        // 0xFFFF00 = -256
        assert!((i24(0x00, 0xFF, 0xFF) + 256.0 / 8_388_608.0).abs() < 1e-7);
    }

    #[test]
    fn the_attenuator_snaps_to_the_switch_positions() {
        assert_eq!(att_index(0), 0);
        assert_eq!(att_index(4), 0);
        assert_eq!(att_index(6), 1);
        assert_eq!(att_index(10), 1);
        assert_eq!(att_index(19), 2);
        assert_eq!(att_index(30), 3);
        assert_eq!(att_index(255), 3);
    }
}
