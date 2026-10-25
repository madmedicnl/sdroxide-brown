//! Auto mode — the app-side half of the unattended FT8/FT4/FT2 sequencer.
//!
//! The policy (which station to answer, whether the mode and band allow it) is
//! pure and lives in [`sdroxide_types::auto`]. This is the loop that runs it
//! once a frame: it keeps the operator's input clock, honours the transmit
//! watchdog as a pause rather than a stop, and puts the chosen action on the
//! command channel to the engine.
//!
//! It runs app-wide rather than inside the FT8 panel for a reason: an
//! unattended run must not depend on which pane or tab is on screen. The engine
//! does the sequencing; this only ever starts the next contact.

use sdroxide_types::auto;
use sdroxide_types::{Band, Command, QsoStep};

use super::SdroxideApp;

impl SdroxideApp {
    /// Whether this station may key the band the dial is on, by the same
    /// band-level rule the engine's key-down gate applies: an amateur
    /// allocation always, 11 m once the operator has opened it, and the
    /// broadcast and general-coverage bands never.
    ///
    /// The engine still has the final say at key-down — the device's own range,
    /// the offset against the band plan — but this is the question auto mode
    /// has to answer before it offers to arm at all.
    pub(in crate::app) fn auto_tx_permitted(&self) -> bool {
        let band = Band::containing(self.state.rx_freq_hz());
        auto::band_may_transmit(band, !self.state.oob_tx, sdroxide_types::cb_tx_allowed())
    }

    /// The inactivity stop for this radio, in seconds, from its own setting,
    /// or `None` when it is switched off (a PlutoSDR set to DISABLE).
    fn auto_idle_stop_s(&self) -> Option<f64> {
        let (min, long) = self
            .radio_cfg
            .as_ref()
            .map_or((0, false), |c| (c.auto_idle_stop_min, auto::has_long_idle_stop(c.backend)));
        auto::auto_idle_stop_s(min, long)
    }

    /// Run auto mode for this frame. Cheap and a no-op unless it is armed.
    pub(in crate::app) fn tick_auto_mode(
        &mut self,
        ctx: &eframe::egui::Context,
        now: f64,
        cmds: &mut Vec<Command>,
    ) {
        // Any input event at all counts as the operator being present. A
        // minimized or unfocused window produces none, which is exactly what
        // the inactivity stop is for.
        if !ctx.input(|i| i.events.is_empty()) {
            self.auto_last_activity = now;
        }
        let Some(role) = self.auto_role else {
            return;
        };
        // The engine's own view of the mode, band and watchdog, not the UI's
        // editable copies: this is what will actually key.
        let Some((mode, step, watchdog, tx_watchdog, status_tx_next, my_call, my_grid)) =
            self.digi_status.as_ref().map(|s| {
                (
                    s.mode,
                    s.step,
                    s.config.tx_watchdog_min,
                    s.tx_watchdog,
                    s.tx_next,
                    s.config.my_call.clone(),
                    s.config.my_grid.clone(),
                )
            })
        else {
            return;
        };
        // **Count the CQ calls of an Auto CQ run from what actually went out.**
        // The sequencer repeats the CQ by itself, so one command is many
        // transmissions; the exchange log is the record of them. Observed every
        // frame, because the run has to be counted whether or not the loop is
        // free to act on it this second.
        let cq_sent = if let Some(s) = self.digi_status.as_ref() {
            self.auto_cq.observe(s.step, &s.transcript)
        } else {
            0
        };
        let tx_permitted = self.auto_tx_permitted();
        if let Some(reason) = auto::auto_block_reason(mode, watchdog, tx_permitted) {
            self.stop_auto(format!("auto stopped: {reason}"), cmds);
            return;
        }
        if let Some(stop_s) = self.auto_idle_stop_s()
            && now - self.auto_last_activity >= stop_s
        {
            let mins = (stop_s / 60.0).round() as u32;
            self.stop_auto(format!("auto stopped: {mins} minutes with no operator input"), cmds);
            return;
        }
        // Only start a contact from a state that is genuinely free. `Idle` is
        // the obvious one; `Confirming` is the engine holding a *logged*
        // contact for a few minutes in case the far end repeats its final
        // message, and waiting out that hold would idle an unattended run for
        // five minutes a contact. It is free once no re-send is owed — while
        // one is, the engine is about to transmit and must be left to it.
        let free = step == QsoStep::Idle || (step == QsoStep::Confirming && !status_tx_next);
        if !free {
            self.auto_resume_at = None;
            return;
        }
        if now < self.auto_cooldown_until {
            return;
        }
        // The transmit watchdog is the pacing, not the end: after it trips,
        // wait one watchdog span and try again.
        if tx_watchdog {
            let span = f64::from(watchdog) * 60.0;
            let resume = *self.auto_resume_at.get_or_insert(now + span);
            if now < resume {
                return;
            }
        }
        self.auto_resume_at = None;

        // **The two roles, and the only thing that separates them.** Auto Rx
        // listens and answers and never keys a CQ of its own; Auto CQ calls and
        // works whoever answers, and does not go chasing somebody else's CQ —
        // that is the other role's job, and they are switched rather than
        // blended (issue #236).
        let answer = |me: &mut Self, cmds: &mut Vec<Command>| -> bool {
            match me.auto_target(&my_call, &my_grid) {
                Some(t) => {
                    me.auto_tried.insert(t.call.to_ascii_uppercase());
                    // The role *now*: an Auto CQ run that has reached its limit
                    // switches to Rx and answers from there, and the note has to
                    // say what the station is actually doing.
                    let label = me.auto_role.map_or("auto", |r| r.label());
                    me.auto_note = format!("{label}: answering {}", t.call);
                    cmds.push(Command::DigiStartQso {
                        from: t.call,
                        grid: t.grid,
                        snr: t.snr_db,
                        audio_hz: t.audio_hz,
                        wait_for_cq: false,
                    });
                    true
                }
                None => false,
            }
        };
        match role {
            auto::AutoRole::Rx => {
                if !answer(self, cmds) {
                    // Nothing worth answering: stay quiet and keep listening.
                    // This is the difference from Auto CQ, and it must not look
                    // like a stalled loop.
                    self.auto_note = "auto rx: listening".into();
                }
            }
            auto::AutoRole::Cq => match auto::cq_limit_action(
                cq_sent,
                self.digi_cfg_edit.auto_cq_repeats,
                self.digi_cfg_edit.auto_cq_then_listen,
            ) {
                auto::CqLimit::Calling => {
                    self.auto_note = format!("auto cq: calling CQ (#{})", cq_sent + 1);
                    cmds.push(Command::DigiCallCq);
                }
                auto::CqLimit::Listen => {
                    // The run has made the calls the operator allowed. Hand over
                    // to Auto Rx rather than going quiet: the station is still
                    // watching, and a CQ that finally gets answered is picked up
                    // by the listening half — but nothing is keyed unasked.
                    let limit = self.digi_cfg_edit.auto_cq_repeats;
                    self.auto_role = Some(auto::AutoRole::Rx);
                    self.auto_cq = auto::CqCounter::default();
                    self.auto_note =
                        format!("auto cq: no answer after {limit} calls — listening (auto rx)");
                    if !answer(self, cmds) {
                        return;
                    }
                }
                auto::CqLimit::Stop => {
                    let limit = self.digi_cfg_edit.auto_cq_repeats;
                    self.stop_auto(format!("auto cq stopped: no answer after {limit} calls"), cmds);
                    return;
                }
            },
        }
        self.auto_cooldown_until = now + auto::AUTO_COOLDOWN_S;
    }

    /// Keep the log index current and run the pure selection against it.
    ///
    /// The index is borrowed directly rather than cloned: it is the whole log
    /// as sets, and cloning it every frame would be the one expensive thing in
    /// this path.
    ///
    /// The ignore list is asked again here even though the ingress filter
    /// already drops new decodes, because a row that was in the list *before*
    /// the operator muted that station is still in it. Muting is an
    /// instruction not to answer somebody, and auto mode answers on its own
    /// initiative — so it honours the list for a decode it would otherwise
    /// pick out of the ones already held.
    ///
    /// The list is borrowed, not copied: the rule is handed to the picker as a
    /// predicate so it can drop those rows in place. An earlier cut built a
    /// filtered `Vec` of the whole decode list instead, which meant cloning up
    /// to 200 rows every frame for the sake of the few that were muted.
    fn auto_target(&mut self, my_call: &str, my_grid: &str) -> Option<auto::AutoTarget> {
        let len = self.qso_log.len();
        if self.log_index_cache.as_ref().map(|(l, _)| *l) != Some(len) {
            self.log_index_cache = Some((len, sdroxide_types::LogIndex::build(&self.qso_log)));
        }
        let ix = &self.log_index_cache.as_ref().expect("just filled").1;
        if self.session_ignored.is_empty() {
            return auto::pick_cq(
                &self.digi_decodes,
                ix,
                my_call,
                my_grid,
                &self.auto_tried,
                |_| false,
            );
        }
        auto::pick_cq(&self.digi_decodes, ix, my_call, my_grid, &self.auto_tried, |d| {
            crate::app::ignore::is_ignored(&self.session_ignored, d.from.as_deref())
        })
    }

    /// Disarm auto mode, leaving `note` as the reason shown on the toggle.
    ///
    /// Disarming is a kill switch, not just a stop-selecting: it also tells
    /// the engine to stop the QSO and abort any burst in flight. An unattended
    /// run that has gone wrong must be stoppable in one click, and leaving the
    /// contact in hand to sequence on its own is exactly the "runs wild" this
    /// exists to end.
    pub(in crate::app) fn stop_auto(&mut self, note: String, cmds: &mut Vec<Command>) {
        if self.auto_role.is_some() {
            cmds.push(Command::DigiStopQso);
            cmds.push(Command::DigiAbortTx);
        }
        self.disarm_auto(note);
    }

    /// Clear auto mode without sending anything, for the case where the stop
    /// command is already on its way (STOP QSO, STOP TX, a bound Abort TX).
    pub(in crate::app) fn disarm_auto(&mut self, note: String) {
        if self.auto_role.is_some() {
            self.auto_role = None;
            self.auto_cq = auto::CqCounter::default();
            self.auto_resume_at = None;
            self.auto_note = note;
        }
    }

    /// Arm one of the two roles now. Returns `false` (doing nothing) when it may
    /// not run.
    pub(in crate::app) fn arm_auto(&mut self, now: f64, role: auto::AutoRole) -> bool {
        let Some(s) = self.digi_status.as_ref() else { return false };
        let tx_permitted = self.auto_tx_permitted();
        if auto::auto_block_reason(s.mode, s.config.tx_watchdog_min, tx_permitted).is_some() {
            return false;
        }
        self.auto_role = Some(role);
        self.auto_cq = auto::CqCounter::default();
        self.auto_last_activity = now;
        self.auto_cooldown_until = 0.0;
        self.auto_resume_at = None;
        self.auto_note = match role {
            auto::AutoRole::Rx => "auto rx: listening".into(),
            auto::AutoRole::Cq => "auto cq: calling CQ".into(),
        };
        true
    }
}
