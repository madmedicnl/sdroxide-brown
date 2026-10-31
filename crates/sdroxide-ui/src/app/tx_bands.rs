//! The operator's transmit band list, on screen.
//!
//! The list itself is `RadioConfig::tx_band_enable`, and the engine refuses to
//! key on a band that is not in it (see `Engine::tx_band_refusal`). This is the
//! half the operator sees: the notice that asks them to check and confirm the
//! list once per radio, the PTT and TUNE that stay dark where TX is off, and the
//! notice shown when the dial is on a band that is off. The dial is never held.
//!
//! None of it applies on a listening screen (SWL): there is nothing to key.

use sdroxide_types::Band;

use super::SdroxideApp;
use crate::i18n::tr;

/// Where this radio stands with its transmit band list.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::app) enum TxBands {
    /// A listening screen, a receive-only radio, or no configuration yet.
    NotApplicable,
    /// Not confirmed. `carried_over` is an upgrade whose bands were filled in
    /// from the previous version; otherwise a new radio with every band off.
    Unconfirmed {
        carried_over: bool,
    },
    Confirmed,
}

/// How many times per session and radio tab the refused-band notice shows.
pub(in crate::app) const TX_BAND_NOTICES_MAX: u8 = 3;

/// The name of the band `hz` is in, as the operator sees it.
fn band_name(hz: f64) -> String {
    match Band::containing(hz) {
        Band::Gen => tr!("settings-other"),
        b => b.label().to_string(),
    }
}

impl SdroxideApp {
    pub(in crate::app) fn tx_bands_status(&self) -> TxBands {
        if !self.tx_capable() {
            return TxBands::NotApplicable;
        }
        let Some(cfg) = self.radio_cfg.as_ref() else { return TxBands::NotApplicable };
        if cfg.tx_bands_confirmed {
            TxBands::Confirmed
        } else {
            TxBands::Unconfirmed {
                carried_over: cfg.tx_band_enable.as_ref().is_some_and(|b| !b.is_empty()),
            }
        }
    }

    /// Whether PTT and TUNE may be offered at the dial this radio would
    /// transmit on. Always `true` where nothing is offered anyway.
    pub(in crate::app) fn tx_keyable_here(&self) -> bool {
        if !self.tx_capable() {
            return true;
        }
        self.radio_cfg.as_ref().is_some_and(|c| c.tx_dial_allowed(self.state.tx_freq_hz()))
    }

    /// The notice for an unconfirmed list, or `None` when there is nothing to say.
    pub(in crate::app) fn tx_bands_notice(&self) -> Option<String> {
        match self.tx_bands_status() {
            TxBands::Unconfirmed { carried_over: false } => Some(tr!("tx-bands-notice-new")),
            TxBands::Unconfirmed { carried_over: true } => Some(tr!("tx-bands-notice-carried")),
            _ => None,
        }
    }

    /// Warn, without ever touching the dial, when the frequency is outside the
    /// bands the operator enabled and confirmed. Transmit is already off there
    /// (the engine refuses to key); this is only the notice.
    ///
    /// The dial is free: nothing is put back and the screen is not changed.
    /// Each move onto a refused band counts as one showing, at most
    /// [`TX_BAND_NOTICES_MAX`] per session and per radio tab — the count is
    /// not saved, so it starts again at the next launch.
    pub(in crate::app) fn tx_band_guard(&mut self) {
        if self.tx_bands_status() != TxBands::Confirmed || self.state.playback.is_some() {
            self.tx_band_msg = None;
            self.tx_band_refused_in = None;
            return;
        }
        let Some(cfg) = self.radio_cfg.as_ref() else { return };
        let hz = self.state.tx_freq_hz();
        if cfg.tx_dial_allowed(hz) {
            self.tx_band_msg = None;
            self.tx_band_refused_in = None;
            return;
        }
        let band = Band::containing(hz);
        if self.tx_band_refused_in == Some(band) {
            return;
        }
        self.tx_band_refused_in = Some(band);
        if self.tx_band_shown < TX_BAND_NOTICES_MAX {
            self.tx_band_shown += 1;
            self.tx_band_msg =
                Some(tr!("tx-bands-off", band = band_name(hz), max = TX_BAND_NOTICES_MAX));
        } else {
            self.tx_band_msg = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use sdroxide_types::{Band, RadioConfig};

    fn confirmed(bands: &[Band]) -> RadioConfig {
        RadioConfig {
            tx_band_enable: Some(bands.to_vec()),
            tx_bands_confirmed: true,
            ..RadioConfig::default()
        }
    }

    #[test]
    fn a_band_is_allowed_only_when_listed_and_confirmed() {
        let cfg = confirmed(&[Band::M40]);
        assert!(cfg.tx_dial_allowed(7_074_000.0));
        assert!(!cfg.tx_dial_allowed(3_573_000.0), "80 m is not listed");
        let unconfirmed = RadioConfig { tx_bands_confirmed: false, ..cfg };
        assert!(
            !unconfirmed.tx_dial_allowed(7_074_000.0),
            "a list nobody confirmed allows nothing"
        );
        assert!(!RadioConfig::default().tx_dial_allowed(7_074_000.0), "no list, no band");
    }

    /// ALL is a selector, not a permission: the check is on the real frequency.
    #[test]
    fn a_frequency_outside_every_band_needs_the_other_row() {
        assert_eq!(Band::containing(35_000_000.0), Band::Gen);
        let cfg = confirmed(&[Band::M40, Band::M20]);
        assert!(!cfg.tx_dial_allowed(35_000_000.0));
        let cfg = confirmed(&[Band::Gen]);
        assert!(cfg.tx_dial_allowed(35_000_000.0));
        assert!(!cfg.tx_dial_allowed(7_074_000.0));
    }

    #[test]
    fn an_upgrade_is_filled_in_unconfirmed_and_only_once() {
        let mut cfg = RadioConfig::default();
        assert!(cfg.tx_band_enable.is_none());
        assert!(cfg.migrate_tx_bands(|hz| hz < 30e6));
        let bands = cfg.tx_band_enable.clone().unwrap();
        assert!(bands.contains(&Band::M40) && bands.contains(&Band::M10));
        assert!(!bands.contains(&Band::M2), "out of the radio's range");
        assert!(!bands.contains(&Band::M11), "CB is not an amateur band");
        assert!(!bands.contains(&Band::Gen));
        assert!(!cfg.tx_bands_confirmed, "carried over, never confirmed");
        assert!(!cfg.migrate_tx_bands(|_| true), "an existing list is left alone");
    }
}
