//! The REC window's plan of planned receptions (issue #754).
//!
//! This window **edits** the plan; it no longer runs it. The clock lives in the
//! engine ([`sdroxide_radio`]'s `poll_job_clock`), because a station that records
//! at four in the morning has nobody looking at a screen, and a browser tab
//! cannot be the clock for a station it is not running on.
//!
//! The plan itself is a station file (`recording_jobs.json`) that every engine
//! reads and one engine writes: edits here are sent as
//! [`Command::SetRecordingJobs`], the engine saves the file and answers with
//! [`sdroxide_types::RadioEvent::RecordingJobs`], and that answer is what fills
//! this list — including the echo of the edit that produced it, so two clients
//! on the same station converge on one plan.

use eframe::egui::{self, RichText};
use sdroxide_types::recording::{
    clock_text, is_date, parse_clock, JobRadio, JobRepeat, RecordingJob, RecordingKind,
    MINUTES_PER_DAY,
};
use sdroxide_types::{Command, Mode, WefaxIoc};

use crate::app::SdroxideApp;
use crate::i18n::tr;

// Every mode the receiver has, including the decoding ones: a planned WEFAX,
// SSTV or STANAG reception is what the request asked for, and for those the
// decoder is what keeps the result — see the note under the form.

/// What the mode picker offers.
///
/// WEFAX is two entries rather than one plus a picker of its own: the index of
/// cooperation is part of what a receiver of charts chooses, and "WEFAX 576/120"
/// is the pair the schedule prints. That keeps the row one field shorter.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PlanMode {
    Mode(Mode),
    /// A chart service: 120 lines a minute with this index of cooperation.
    Wefax(WefaxIoc),
}

impl PlanMode {
    fn of(job: &RecordingJob) -> PlanMode {
        match job.mode {
            Mode::Wefax => PlanMode::Wefax(job.wefax_ioc.unwrap_or_default()),
            m => PlanMode::Mode(m),
        }
    }

    fn text(self) -> String {
        match self {
            PlanMode::Mode(m) => m.label().to_string(),
            PlanMode::Wefax(ioc) => format!("WEFAX {}/120", ioc.value()),
        }
    }

    fn apply(self, job: &mut RecordingJob) {
        match self {
            PlanMode::Mode(m) => {
                job.mode = m;
                job.wefax_ioc = None;
            }
            PlanMode::Wefax(ioc) => {
                job.mode = Mode::Wefax;
                job.wefax_ioc = Some(ioc);
            }
        }
    }
}

/// Every mode, with WEFAX split into its two services where it sits in the list.
fn plan_modes() -> Vec<PlanMode> {
    let mut out = Vec::with_capacity(Mode::ALL.len() + 1);
    for m in Mode::ALL {
        if m == Mode::Wefax {
            out.push(PlanMode::Wefax(WefaxIoc::I576));
            out.push(PlanMode::Wefax(WefaxIoc::I288));
        } else {
            out.push(PlanMode::Mode(m));
        }
    }
    out
}

pub(in crate::app) struct JobsUi {
    pub show: bool,
    /// The row being edited. Its times are kept as text beside it, because a
    /// half-typed `09:4` is not a minute and must not be parsed into one.
    pub edit: Option<RecordingJob>,
    date: String,
    start: String,
    stop: String,
}

impl Default for JobsUi {
    fn default() -> Self {
        JobsUi { show: false, edit: None, date: String::new(), start: String::new(), stop: String::new() }
    }
}

impl SdroxideApp {
    /// Take the station's plan as the engine just published it.
    ///
    /// One-off rows whose day has gone are dropped here rather than in the
    /// engine: the engine only reads the file, and a list that still shows last
    /// Tuesday's reception is exactly the thing the requester asked not to keep.
    /// The drop reaches the file on the next edit, which is also when a one-off
    /// is normally removed.
    pub(in crate::app) fn adopt_job_plan(&mut self, jobs: Vec<RecordingJob>) {
        let today = today_text();
        self.recording_jobs = jobs.into_iter().filter(|j| !is_past_one_off(j, &today)).collect();
    }

    /// The RECORDINGS window: the plan and its form.
    pub(in crate::app) fn recordings_window(&mut self, ctx: &egui::Context, cmds: &mut Vec<Command>) {
        let mut new_job = false;
        let mut save = false;
        let mut cancel = false;
        let mut edit: Option<u64> = None;
        let mut delete: Option<u64> = None;
        let open = self.tool_window(
            ctx,
            "recordings",
            &tr!("recjobs-title"),
            [760.0, 500.0],
            self.jobs.show,
            |me, ui| {
                me.recordings_body(ui, &mut new_job, &mut save, &mut cancel, &mut edit, &mut delete)
            },
        );
        self.jobs.show = open;

        if new_job {
            // A row that can be saved without touching anything: today, the
            // next half hour, the dial's own frequency and mode, on this radio.
            let (start, stop) = next_half_hour();
            let mut j = RecordingJob::default();
            j.freq_hz = self.on_air_freq_hz();
            j.mode = self.state.rx[0].mode;
            j.start_min = start;
            j.stop_min = stop;
            j.repeat = JobRepeat::Daily;
            j.radio = JobRadio::Id(self.station_radio_id());
            self.jobs.date = today_text();
            self.jobs.start = clock_text(start);
            self.jobs.stop = clock_text(stop);
            self.jobs.edit = Some(j);
        }
        if let Some(id) = edit
            && let Some(j) = self.recording_jobs.iter().find(|x| x.id == id)
        {
            self.jobs.date = j.date.clone();
            self.jobs.start = clock_text(j.start_min);
            self.jobs.stop = clock_text(j.stop_min);
            self.jobs.edit = Some(j.clone());
        }
        if save && let Some(mut j) = self.jobs.edit.take() {
            // The text fields are the truth while the form is open; a field
            // that will not parse keeps its previous value rather than
            // silently becoming midnight.
            if let Some(m) = parse_clock(&self.jobs.start) {
                j.start_min = m.min(MINUTES_PER_DAY - 1);
            }
            if let Some(m) = parse_clock(&self.jobs.stop) {
                j.stop_min = m.min(MINUTES_PER_DAY - 1);
            }
            if is_date(&self.jobs.date) {
                j.date = self.jobs.date.clone();
            }
            if j.id == 0 {
                j.id = self.recording_jobs.iter().map(|x| x.id).max().unwrap_or(0) + 1;
                self.recording_jobs.push(j);
            } else if let Some(slot) = self.recording_jobs.iter_mut().find(|x| x.id == j.id) {
                *slot = j;
            }
            cmds.push(Command::SetRecordingJobs(self.recording_jobs.clone()));
        }
        if cancel {
            self.jobs.edit = None;
        }
        if let Some(id) = delete {
            self.recording_jobs.retain(|x| x.id != id);
            cmds.push(Command::SetRecordingJobs(self.recording_jobs.clone()));
        }
    }

    /// The plan's body, split out so the shell can draw it in a window of its
    /// own.
    fn recordings_body(
        &mut self,
        ui: &mut egui::Ui,
        new_job: &mut bool,
        save: &mut bool,
        cancel: &mut bool,
        edit: &mut Option<u64>,
        delete: &mut Option<u64>,
    ) {
        crate::chrome::window_body_bg(ui);
        let rows = self.recording_jobs.clone();
        let today = today_text();
        let radios = self.station_radios();
        ui.horizontal(|ui| {
            if crate::chrome::chip(ui, false, tr!("recjobs-new-job")).clicked() {
                *new_job = true;
            }
            if self.state.recording || self.state.iq_recording {
                ui.label(
                    RichText::new(tr!("recjobs-recording")).size(11.0).color(crate::theme::ALERT()),
                );
            }
            ui.add(
                egui::Label::new(RichText::new(tr!("recjobs-plan-on-station")).size(11.0).weak())
                    .wrap(),
            );
        });
        if self.jobs.edit.is_some() {
            ui.add_space(4.0);
            self.job_form(ui, save, cancel);
        }
        ui.separator();
        egui::ScrollArea::vertical().auto_shrink([false, false]).id_salt("jobs-list").show(ui, |ui| {
            if rows.is_empty() {
                ui.label(RichText::new(tr!("recjobs-none")).color(crate::theme::gray(150)));
            }
            let running = self.state.plan_running.as_ref().map(|r| r.id);
            for j in &rows {
                ui.horizontal(|ui| {
                    // The row on the air, marked in the list it came from.
                    if running == Some(j.id) {
                        ui.label(RichText::new("●").size(9.0).color(crate::theme::ALERT()));
                    }
                    // When it runs: the day, or "every day".
                    let when = match j.repeat {
                        JobRepeat::Daily => tr!("recjobs-daily").to_string(),
                        JobRepeat::Once => j.date.clone(),
                    };
                    ui.label(RichText::new(when).monospace().size(11.0));
                    ui.label(
                        RichText::new(format!("{}-{}", j.start_text(), j.stop_text()))
                            .monospace()
                            .size(11.0),
                    );
                    ui.label(
                        RichText::new(format!("{:>6} kHz", j.freq_hz.round() as i64)).monospace(),
                    );
                    ui.label(RichText::new(PlanMode::of(j).text()).size(11.0));
                    ui.label(RichText::new(truncate(&j.name, 24)).strong());
                    ui.label(RichText::new(kind_label(j.kind)).size(11.0).color(crate::theme::gray(160)));
                    ui.label(RichText::new(radio_text(&radios, &tr!("recjobs-radio-any"), j.radio)).size(11.0).color(crate::theme::gray(160)));
                    if j.problem(&today).is_some() {
                        ui.label(
                            RichText::new(tr!("recjobs-incomplete"))
                                .size(10.5)
                                .color(crate::theme::ALERT()),
                        );
                    }
                    if ui.small_button(tr!("recjobs-edit")).clicked() {
                        *edit = Some(j.id);
                    }
                    if ui.small_button(tr!("recjobs-del")).clicked() {
                        *delete = Some(j.id);
                    }
                });
            }
        });
    }

    fn job_form(&mut self, ui: &mut egui::Ui, save: &mut bool, cancel: &mut bool) {
        let radios = self.station_radios();
        let any_label = tr!("recjobs-radio-any");
        let Some(j) = self.jobs.edit.as_mut() else { return };
        egui::Frame::new()
            .fill(crate::theme::ROW_BG())
            .stroke(egui::Stroke::new(1.0, crate::theme::RED_DEEP()))
            .inner_margin(egui::Margin::same(9))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                egui::Grid::new("job-form").num_columns(4).spacing([10.0, 6.0]).show(ui, |ui| {
                    ui.label(tr!("recjobs-station"));
                    crate::chrome::field(
                        ui,
                        egui::TextEdit::singleline(&mut j.name)
                            .desired_width(220.0)
                            .hint_text(tr!("recjobs-station-hint")),
                    );
                    ui.label(tr!("recjobs-kind"));
                    egui::ComboBox::from_id_salt("job-kind")
                        .width(90.0)
                        .selected_text(kind_label(j.kind))
                        .show_ui(ui, |ui| {
                            for k in [
                                RecordingKind::Audio,
                                RecordingKind::Iq,
                                RecordingKind::Both,
                                RecordingKind::Nothing,
                            ] {
                                ui.selectable_value(&mut j.kind, k, kind_label(k));
                            }
                        });
                    ui.end_row();

                    ui.label(tr!("recjobs-when"));
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut j.repeat, JobRepeat::Daily, tr!("recjobs-daily"));
                        ui.selectable_value(&mut j.repeat, JobRepeat::Once, tr!("recjobs-on-date"));
                        ui.add_enabled_ui(j.repeat == JobRepeat::Once, |ui| {
                            crate::chrome::field(
                                ui,
                                egui::TextEdit::singleline(&mut self.jobs.date)
                                    .desired_width(96.0)
                                    .hint_text("2026-12-24"),
                            );
                        });
                    });
                    ui.label(tr!("recjobs-radio"));
                    egui::ComboBox::from_id_salt("job-radio")
                        .width(150.0)
                        .selected_text(radio_text(&radios, &any_label, j.radio))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut j.radio, JobRadio::Any, any_label.clone());
                            for (id, name) in &radios {
                                let label = if name.is_empty() {
                                    format!("Radio {}", id + 1)
                                } else {
                                    name.clone()
                                };
                                ui.selectable_value(&mut j.radio, JobRadio::Id(*id), label);
                            }
                        });
                    ui.end_row();

                    ui.label(tr!("recjobs-start-local"));
                    ui.horizontal(|ui| {
                        crate::chrome::field(
                            ui,
                            egui::TextEdit::singleline(&mut self.jobs.start).desired_width(64.0),
                        );
                        ui.label("–");
                        crate::chrome::field(
                            ui,
                            egui::TextEdit::singleline(&mut self.jobs.stop).desired_width(64.0),
                        );
                    });
                    ui.label(tr!("settings-frequency"));
                    ui.horizontal(|ui| {
                        let mut khz = (j.freq_hz / 1e3).round() as i64;
                        if ui
                            .add(egui::DragValue::new(&mut khz).speed(1.0).range(0..=1_000_000))
                            .changed()
                        {
                            j.freq_hz = khz as f64 * 1e3;
                        }
                        ui.label("kHz");
                        let current = PlanMode::of(j);
                        let mut chosen = current;
                        egui::ComboBox::from_id_salt("job-mode")
                            .width(130.0)
                            .selected_text(current.text())
                            .show_ui(ui, |ui| {
                                for pm in plan_modes() {
                                    ui.selectable_value(&mut chosen, pm, pm.text());
                                }
                            });
                        if chosen != current {
                            chosen.apply(j);
                        }
                    });
                    ui.end_row();
                });
                ui.add_space(4.0);
                ui.add(
                    egui::Label::new(
                        RichText::new(tr!("recjobs-times-note")).size(10.5).weak(),
                    )
                    .wrap(),
                );
                ui.add(
                    egui::Label::new(
                        RichText::new(tr!("recjobs-decode-note")).size(10.5).weak(),
                    )
                    .wrap(),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if crate::chrome::chip(ui, true, tr!("recjobs-save")).clicked() {
                        *save = true;
                    }
                    if crate::chrome::chip(ui, false, tr!("logbook-cancel")).clicked() {
                        *cancel = true;
                    }
                });
            });
    }

    /// This station's radios, by the id its own configuration knows them by.
    fn station_radios(&self) -> Vec<(u32, String)> {
        let station = self.station_key();
        self.radio_roster
            .iter()
            .filter(|c| c.station == station)
            .map(|c| (c.station_id, c.name.clone()))
            .collect()
    }
}

/// How a row's radio reads: this station's name for it, or "whichever".
fn radio_text(radios: &[(u32, String)], any: &str, radio: JobRadio) -> String {
    match radio {
        JobRadio::Any => any.to_string(),
        JobRadio::Id(id) => radios
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, n)| {
                if n.is_empty() {
                    format!("Radio {}", id + 1)
                } else {
                    n.clone()
                }
            })
            .unwrap_or_else(|| format!("Radio {}", id + 1)),
    }
}

/// The station's local day and minute-of-day, `("2026-11-24", 585)`.
///
/// The clock itself is the station's — the engine compares the plan against its
/// own `chrono::Local` — but a browser client must still offer a sensible
/// default date and show "today" for the expiry note, and `chrono` is a
/// native-only dependency here. So this asks the platform for the UTC instant
/// and its current offset, which is the same answer `chrono::Local` gives and
/// works on both targets (`crate::time` is where that split already lives).
fn local_now() -> (String, u16) {
    let unix = crate::time::now_unix();
    let offset = crate::time::local_offset_seconds();
    let (y, mo, d, h, mi, _) = sdroxide_types::utc_ymd_hms(unix + offset);
    (format!("{y:04}-{mo:02}-{d:02}"), u16::try_from(h * 60 + mi).unwrap_or(0))
}

/// This station's local day, `YYYY-MM-DD`.
fn today_text() -> String {
    local_now().0
}

/// The half hour we are in, or the next one: `(start, stop)` minutes.
fn next_half_hour() -> (u16, u16) {
    let (_, minute) = local_now();
    let start = (minute / 30 + 1) * 30;
    (start.min(MINUTES_PER_DAY - 1), (start + 30).min(MINUTES_PER_DAY - 1))
}

/// A dated reception whose day is behind us. Recurring rows stay for ever.
fn is_past_one_off(j: &RecordingJob, today: &str) -> bool {
    j.repeat == JobRepeat::Once && is_date(&j.date) && j.date.as_str() < today
}

fn kind_label(k: RecordingKind) -> String {
    match k {
        RecordingKind::Audio => tr!("recjobs-kind-audio"),
        RecordingKind::Iq => tr!("recjobs-kind-iq"),
        RecordingKind::Both => tr!("recjobs-kind-both"),
        RecordingKind::Nothing => tr!("recjobs-kind-none"),
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(n.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wefax_is_two_choices_in_one_field() {
        let mut j = RecordingJob::default();
        PlanMode::Wefax(WefaxIoc::I288).apply(&mut j);
        assert_eq!(j.mode, Mode::Wefax);
        assert_eq!(j.wefax_ioc, Some(WefaxIoc::I288));
        assert_eq!(PlanMode::of(&j).text(), "WEFAX 288/120");
        // And choosing an ordinary mode takes the index away with it, so a row
        // cannot carry a leftover 288 while it is on USB.
        PlanMode::Mode(Mode::Usb).apply(&mut j);
        assert_eq!(j.wefax_ioc, None);
        assert_eq!(PlanMode::of(&j).text(), "USB");
        // Both services are offered, and WEFAX appears where it sits in ALL.
        let list = plan_modes();
        assert_eq!(list.iter().filter(|p| matches!(p, PlanMode::Wefax(_))).count(), 2);
        let at = list.iter().position(|p| *p == PlanMode::Mode(Mode::Wefax));
        assert!(at.is_none(), "plain WEFAX is replaced by the two services");
    }

    #[test]
    fn a_past_one_off_is_dropped_and_a_daily_row_is_not() {
        let mut past = RecordingJob { repeat: JobRepeat::Once, ..Default::default() };
        past.date = "2020-01-01".into();
        assert!(is_past_one_off(&past, "2026-11-23"));
        let mut today = RecordingJob { repeat: JobRepeat::Once, ..Default::default() };
        today.date = "2026-11-23".into();
        assert!(!is_past_one_off(&today, "2026-11-23"));
        let mut daily = RecordingJob { repeat: JobRepeat::Daily, ..Default::default() };
        daily.date = "2020-01-01".into();
        assert!(!is_past_one_off(&daily, "2026-11-23"), "recurring rows stay for ever");
    }

    #[test]
    fn a_new_row_starts_on_the_next_half_hour_inside_the_day() {
        let (start, stop) = next_half_hour();
        assert!(start < MINUTES_PER_DAY && stop < MINUTES_PER_DAY);
        assert!(start % 30 == 0, "a row starts on the half hour: {start}");
        assert!(stop > start);
    }
}
