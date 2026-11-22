//! An [`IqSource`] for a Microtelecom Perseus driven by the vendor library in
//! `sdroxide-perseus` — no SoapySDR, no libusb binding of ours.
//!
//! Receive only: the hardware has no transmitter, so the trait's transmit
//! methods keep their "not transmit capable" answer.
//!
//! Two things are settled at open and nowhere else: the sample rate (the
//! library reprograms the FPGA for it, so changing it means reopening the
//! receiver) and the device ordinal. Everything else — the dial, the
//! attenuator, the preselector — moves while the stream runs.

use std::time::Duration;

use sdroxide_dsp::Complex32;
use sdroxide_perseus::{OpenParams, PerseusHandle};
use sdroxide_radio::{IqSource, RadioError, Result};
use sdroxide_types::PerseusConfig;

/// How long the receiver may deliver nothing before the stream counts as dead.
/// A local USB receiver that has streamed even once should deliver every
/// millisecond; three seconds is the same window `sdroxide-fobos` uses.
const SILENCE_BEFORE_REOPEN: Duration = Duration::from_secs(3);

/// Whether the preselector can be in circuit at this frequency.
///
/// The operator's switch is a wish, not a command: the filter's passband sits
/// above 1.5 MHz, and in circuit below it the receiver hears almost nothing —
/// which reads as a deaf radio rather than as a filter. So the wish is honoured
/// only where it is usable, and comes back by itself when the dial does.
fn preselector_at(wanted: bool, hz: f64) -> bool {
    wanted && hz >= PerseusConfig::PRESELECTOR_MIN_HZ
}

pub struct PerseusSource {
    handle: PerseusHandle,
    center: f64,
    /// Interleaved I/Q from the ring, before it becomes `Complex32`.
    scratch: Vec<f32>,
    /// What the operator asked for, kept apart from what the filter is doing:
    /// the preselector is forced out of circuit below
    /// [`PerseusConfig::PRESELECTOR_MIN_HZ`] whatever this says, and back in
    /// when the dial returns above it.
    presel_wanted: bool,
    /// What the receiver is actually doing with it right now.
    preselector: bool,
}

impl PerseusSource {
    pub fn open(cfg: &PerseusConfig, center_hz: f64) -> anyhow::Result<Self> {
        let presel_wanted = cfg.preselector;
        let preselector = preselector_at(presel_wanted, center_hz);
        let rate = if cfg.sample_rate_hz > 0.0 {
            cfg.sample_rate_hz
        } else {
            PerseusConfig::default().sample_rate_hz
        };
        let handle = PerseusHandle::open(&OpenParams {
            index: cfg.device,
            center_hz,
            rate_hz: rate,
            att_db: cfg.attenuator_db,
            preselector,
        })
        .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(PerseusSource {
            center: center_hz,
            scratch: Vec::new(),
            presel_wanted,
            preselector,
            handle,
        })
    }

    /// The rates the opened receiver reports, for the settings dropdown.
    pub fn rates(&self) -> Vec<f64> {
        self.handle.rates()
    }

    fn driver_err(e: sdroxide_perseus::Error) -> RadioError {
        RadioError::Msg(e.to_string())
    }

    /// Put the preselector where the dial says it can be, and remember whether
    /// the receiver had to change its mind.
    fn settle_preselector(&mut self, hz: f64) -> bool {
        let want = preselector_at(self.presel_wanted, hz);
        let changed = want != self.preselector;
        self.preselector = want;
        changed
    }
}

impl IqSource for PerseusSource {
    fn sample_rate(&self) -> f64 {
        self.handle.rate_hz()
    }

    fn center_hz(&self) -> f64 {
        self.center
    }

    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        // The library takes the frequency and the preselector switch in one
        // call, so the switch is settled first and applied with the tune.
        self.settle_preselector(hz);
        self.center = hz;
        self.handle.set_preselector(self.preselector);
        self.handle.set_center_hz(hz).map_err(Self::driver_err)
    }

    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        let need = buf.len() * 2;
        if self.scratch.len() < need {
            self.scratch.resize(need, 0.0);
        }
        let n = self.handle.read(&mut self.scratch[..need]);
        let pairs = n / 2;
        for (p, out) in buf.iter_mut().enumerate().take(pairs) {
            *out = Complex32::new(self.scratch[2 * p], self.scratch[2 * p + 1]);
        }
        // A short buffer is a timeout, which `read`'s contract says is `Ok(0)`.
        Ok(pairs)
    }

    fn describe(&self) -> String {
        self.handle.label().to_string()
    }

    fn set_gain_element(&mut self, name: &str, db: f64) -> Result<()> {
        match name {
            PerseusConfig::ATT_ELEMENT => {
                self.handle.set_att_db(db.clamp(0.0, 30.0) as u8).map_err(Self::driver_err)?;
            }
            // A switch rides the gain channel as 0.0/1.0 — see the constant's
            // own doc in `sdroxide-types`.
            PerseusConfig::PRESEL_ELEMENT => {
                self.presel_wanted = db >= 0.5;
                if self.settle_preselector(self.center) {
                    self.handle.set_preselector(self.preselector);
                    self.handle.apply_preselector().map_err(Self::driver_err)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn current_gains(&self) -> Vec<(String, f64)> {
        vec![
            (PerseusConfig::ATT_ELEMENT.to_string(), f64::from(self.handle.att_db())),
            (PerseusConfig::PRESEL_ELEMENT.to_string(), f64::from(u8::from(self.preselector))),
        ]
    }

    fn needs_reopen(&self) -> bool {
        !self.handle.is_alive() || self.handle.silent_for() >= SILENCE_BEFORE_REOPEN
    }

    fn release(&mut self) {
        self.handle.release();
    }
}

#[cfg(test)]
mod tests {
    use super::preselector_at;

    #[test]
    fn the_preselector_is_only_in_circuit_where_it_can_be() {
        assert!(!preselector_at(true, 1_000_000.0), "below the floor it must be out");
        assert!(preselector_at(true, 1_500_000.0), "at the floor it is usable");
        assert!(preselector_at(true, 7_100_000.0));
        assert!(!preselector_at(false, 7_100_000.0), "the operator's choice wins above it");
    }
}
