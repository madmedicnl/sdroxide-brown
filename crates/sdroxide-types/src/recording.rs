//! Scheduled recordings: "record 49 m at 18:00 UTC for half an hour".
//!
//! A job is a start time, a frequency, how long to record and what to capture.
//! The scheduler that acts on these lives in the interface (see the SWL fork's
//! `recording_jobs`), so the pure part — *when* a job starts and stops — is
//! here and can be tested without a radio or a clock.

use serde::{Deserialize, Serialize};

/// A planned reception that is running right now, as the screens are told.
///
/// It rides [`crate::RadioState`] rather than the plan, because "which row is
/// running" is a fact about this moment and not about the list: a row can keep
/// no file at all, and then the REC chip — which says a file is being written —
/// is correctly dark. This is what says the station is doing something.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PlanRun {
    /// The row's id, so the window can mark it.
    pub id: u64,
    /// What the row is called, for the chip's hover and the popup's line.
    pub name: String,
    /// `HH:MM–HH:MM` of the window it is in.
    pub window: String,
}

/// What a job captures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordingKind {
    /// Demodulated audio (a WAV beside the other recordings).
    #[default]
    Audio,
    /// The raw I/Q the radio delivers, replayable later.
    Iq,
    /// Both at once, which the engine already supports.
    Both,
    /// Nothing: the reception is tuned and, in a decoding mode, decoded, but no
    /// file is written of it.
    ///
    /// Appended last, and the reason a plan row can be a listening or decoding
    /// appointment rather than a recording: a WEFAX chart or a STANAG message
    /// is kept by its own decoder, and an audio file of it as well is a file the
    /// operator did not ask for. `Audio` stays the default, so every row written
    /// before this existed behaves exactly as it did.
    Nothing,
}

impl RecordingKind {
    pub fn label(self) -> &'static str {
        match self {
            RecordingKind::Audio => "Audio",
            RecordingKind::Iq => "I/Q",
            RecordingKind::Both => "Both",
            RecordingKind::Nothing => "None",
        }
    }
}

/// The format a recording of the receiver audio is written in.
///
/// Kept beside the scheduled-recording module rather than in `playback`: it is
/// the operator's choice of *file*, set from the REC control and remembered
/// between sessions, and it decides the extension the recorder stamps on the
/// file as much as it decides the encoder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordingFormat {
    /// Lossy MP3 — small, and the default this station has always written.
    #[default]
    Mp3,
    /// 16-bit PCM WAV at 48 kHz — no codec between the receiver and the file,
    /// for an external decoder or a look at the spectrum.
    Wav,
}

impl RecordingFormat {
    /// The label the REC chip carries.
    pub fn label(self) -> &'static str {
        match self {
            RecordingFormat::Mp3 => "MP3",
            RecordingFormat::Wav => "WAV",
        }
    }

    /// The file extension, without the dot.
    pub fn extension(self) -> &'static str {
        match self {
            RecordingFormat::Mp3 => "mp3",
            RecordingFormat::Wav => "wav",
        }
    }
}

/// Minutes in a day — the ceiling a start or stop may not reach.
pub const MINUTES_PER_DAY: u16 = 24 * 60;

/// Seconds in a day.
pub const SECS_PER_DAY: u32 = 24 * 60 * 60;

/// `"09:45"` from minutes after midnight.
pub fn clock_text(minute: u16) -> String {
    format!("{:02}:{:02}", minute / 60, minute % 60)
}

/// Parse `"HH:MM"` into minutes after midnight. `None` for anything that is
/// not a time of day, which is what a half-typed field looks like.
pub fn parse_clock(text: &str) -> Option<u16> {
    let (h, m) = text.trim().split_once(':')?;
    let h: u16 = h.trim().parse().ok()?;
    let m: u16 = m.trim().parse().ok()?;
    if h >= 24 || m >= 60 {
        return None;
    }
    Some(h * 60 + m)
}

/// `YYYY-MM-DD`, and nothing else: four digits, a dash, two, a dash, two.
pub fn is_date(text: &str) -> bool {
    let b = text.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

/// One scheduled recording.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RecordingJob {
    /// Stable id (0 = unassigned; the interface assigns on first store).
    pub id: u64,
    /// When to start, Unix seconds UTC.
    pub at_unix: u64,
    /// The frequency to tune to at the start.
    pub freq_hz: f64,
    pub mode: crate::Mode,
    /// How long to record, in seconds.
    pub duration_s: u32,
    pub kind: RecordingKind,
    /// The station, for the row and the report. Free text.
    pub name: String,
    /// Set once the job has run, so a one-shot does not fire twice.
    pub done: bool,

    /// How the job comes round again, appended so a job written before this
    /// existed stays exactly what it was: a one-shot at [`Self::at_unix`].
    #[serde(default)]
    pub repeat: JobRepeat,

    /// The local time of day the reception starts, in minutes after midnight.
    /// Zero (with [`Self::stop_min`]) means "this job is scheduled by
    /// `at_unix`", which is what every job written before the planner had a
    /// wall clock is.
    ///
    /// Local minutes rather than a UTC instant on purpose: "09:45 every day"
    /// must not walk an hour when the clocks change, and a listener reading
    /// the schedule is reading their own wall clock.
    #[serde(default)]
    pub start_min: u16,

    /// The local minute the reception ends. Equal to [`Self::start_min`] is
    /// refused by [`Self::problem`]; a stop *before* the start crosses
    /// midnight.
    #[serde(default)]
    pub stop_min: u16,

    /// `YYYY-MM-DD` for a one-off planned by the wall clock; empty for a daily
    /// one and for a job scheduled by `at_unix`.
    #[serde(default)]
    pub date: String,

    /// Which radio of the station the job drives, by its roster id.
    /// [`JobRadio::Any`] for everything written before the field existed: the
    /// radio that happens to be open, which is what the old clock assumed.
    #[serde(default)]
    pub radio: JobRadio,

    /// The WEFAX index of cooperation, for a row whose mode is
    /// [`crate::Mode::Wefax`].
    ///
    /// Two WEFAX services and no column: the panel offers "WEFAX 576/120" and
    /// "WEFAX 288/120" as two entries in the mode list rather than a mode plus a
    /// second picker, because the pair is what a receiver of charts actually
    /// chooses, and 120 lines a minute is what every station in the published
    /// schedule sends. `None` on every other mode and on every row written
    /// before this existed.
    #[serde(default)]
    pub wefax_ioc: Option<crate::WefaxIoc>,
}

impl Default for RecordingJob {
    fn default() -> Self {
        RecordingJob {
            id: 0,
            at_unix: 0,
            freq_hz: 0.0,
            mode: crate::Mode::Am,
            duration_s: 30 * 60,
            kind: RecordingKind::Audio,
            name: String::new(),
            done: false,
            repeat: JobRepeat::Once,
            start_min: 0,
            stop_min: 0,
            date: String::new(),
            radio: JobRadio::default(),
            wefax_ioc: None,
        }
    }
}

/// How a planned reception comes round again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum JobRepeat {
    /// On one day, which [`RecordingJob::date`] names.
    #[default]
    Once,
    /// Every day.
    Daily,
}

/// Which radio a planned reception drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobRadio {
    /// Whichever radio is open, which is what a job written before the
    /// planner could name one means.
    Any,
    /// One radio of the station, by its roster id.
    Id(u32),
}

impl Default for JobRadio {
    fn default() -> Self {
        JobRadio::Any
    }
}

impl JobRadio {
    /// The roster id this names, if it names one.
    pub fn id(self) -> Option<u32> {
        match self {
            JobRadio::Any => None,
            JobRadio::Id(id) => Some(id),
        }
    }

    /// Whether a job for this selection runs on radio `id`.
    pub fn matches(self, id: u32) -> bool {
        match self {
            JobRadio::Any => true,
            JobRadio::Id(want) => want == id,
        }
    }
}

/// What the scheduler should do with a job at a given instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobAction {
    /// Start recording for this job.
    Start,
    /// Stop the recording this job started.
    Stop,
    /// Nothing to do.
    Idle,
}

impl RecordingJob {
    /// When the recording should end.
    pub fn end_unix(&self) -> u64 {
        self.at_unix.saturating_add(self.duration_s as u64)
    }

    /// What to do at `now`, given whether this job is the one currently
    /// recording.
    ///
    /// A finished job does nothing even if it is still "running", so a job that
    /// never got stopped (the program was closed mid-over) does not restart.
    pub fn action(&self, now: u64, running: bool) -> JobAction {
        if self.done && !running {
            return JobAction::Idle;
        }
        if now >= self.end_unix() {
            return if running { JobAction::Stop } else { JobAction::Idle };
        }
        if now >= self.at_unix && !running {
            return JobAction::Start;
        }
        JobAction::Idle
    }

    /// Whether this job is planned by the wall clock (a daily or dated
    /// reception) rather than at the one instant `at_unix` holds.
    ///
    /// The two are exclusive: a job written before the planner had a wall
    /// clock has no stop minute, and behaves exactly as it always did.
    pub fn is_wall_clock(&self) -> bool {
        self.stop_min > 0
    }

    /// Whether the job is running at `minute` on `date` (`YYYY-MM-DD`), for a
    /// wall-clock job. A window whose stop is before its start crosses
    /// midnight.
    pub fn active_at(&self, date: &str, minute: u16) -> bool {
        if !self.is_wall_clock() {
            return false;
        }
        match self.repeat {
            JobRepeat::Daily => {}
            JobRepeat::Once => {
                if self.date != date {
                    return false;
                }
            }
        }
        if self.start_min <= self.stop_min {
            self.start_min <= minute && minute < self.stop_min
        } else {
            minute >= self.start_min || minute < self.stop_min
        }
    }

    /// Whether the job should already be tuned at `secs` after midnight,
    /// `lead` seconds before its own start.
    ///
    /// WEFAX opens with five seconds of 300 Hz start tone and then the phasing
    /// lines, and SSTV with a leader and a VIS code; a receiver that arrives on
    /// the minute itself has missed both and gets a chart that starts skew or a
    /// picture whose mode had to be guessed. So the clock tunes ahead by the
    /// lead and the decoder — not a recorder — is what starts on the tone.
    ///
    /// The window's *end* is untouched: a job does not keep the radio longer
    /// for having arrived early.
    pub fn active_seconds(&self, date: &str, secs: u32, lead: u32) -> bool {
        if !self.is_wall_clock() {
            return false;
        }
        match self.repeat {
            JobRepeat::Daily => {}
            JobRepeat::Once => {
                if self.date != date {
                    return false;
                }
            }
        }
        let start = u32::from(self.start_min) * 60;
        let stop = u32::from(self.stop_min) * 60;
        // Wrapped, so a job at 00:00 is still due for its lead the evening before.
        let open = (start + SECS_PER_DAY - lead % SECS_PER_DAY) % SECS_PER_DAY;
        if open <= stop {
            open <= secs && secs < stop
        } else {
            // Crossing midnight: open in the evening, closed after midnight.
            secs >= open || secs < stop
        }
    }

    /// What a wall-clock job should do at `minute` on `date`, given whether its
    /// recording is the one running.
    ///
    /// Deliberately the same three answers as [`Self::action`], so the clock
    /// nowhere needs to know which kind of job it is looking at.
    pub fn wall_action(&self, date: &str, minute: u16, running: bool) -> JobAction {
        let active = self.active_at(date, minute);
        match (active, running) {
            (true, false) => JobAction::Start,
            (false, true) => JobAction::Stop,
            _ => JobAction::Idle,
        }
    }

    /// What is wrong with this job, if anything — one sentence for the panel to
    /// show beside the row.
    ///
    /// `date_today` is the panel's own day: a dated reception in the past is
    /// refused there, and this is where that is decided (the type has no clock).
    pub fn problem(&self, date_today: &str) -> Option<String> {
        if !self.is_wall_clock() {
            return None;
        }
        if self.start_min >= 24 * 60 || self.stop_min >= 24 * 60 {
            return Some("a time must be between 00:00 and 23:59".into());
        }
        if self.start_min == self.stop_min {
            return Some("start and end are the same minute".into());
        }
        if !(self.freq_hz.is_finite() && self.freq_hz > 0.0) {
            return Some("no frequency".into());
        }
        if self.repeat == JobRepeat::Once {
            if !is_date(&self.date) {
                return Some("the date must be YYYY-MM-DD".into());
            }
            if self.date.as_str() < date_today {
                return Some("that date has passed".into());
            }
        }
        None
    }

    /// `"09:45"`.
    pub fn start_text(&self) -> String {
        clock_text(self.start_min)
    }

    /// `"10:05"`.
    pub fn stop_text(&self) -> String {
        clock_text(self.stop_min)
    }

    /// `("09:45", "10:05")` — the window as the list shows it.
    pub fn window_text(&self) -> (String, String) {
        (clock_text(self.start_min), clock_text(self.stop_min))
    }

    /// UTC as a listener reads it, e.g. `2026-09-16 18:00 UTC`.
    pub fn utc_text(&self) -> String {
        let (y, mo, d, h, mi, _s) = crate::utc_ymd_hms(self.at_unix as i64);
        format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02} UTC")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(at: u64, dur: u32) -> RecordingJob {
        RecordingJob { at_unix: at, duration_s: dur, ..Default::default() }
    }

    #[test]
    fn a_job_starts_at_its_time_and_stops_at_its_end() {
        let j = job(1000, 60);
        assert_eq!(j.action(999, false), JobAction::Idle, "too early");
        assert_eq!(j.action(1000, false), JobAction::Start, "at the start");
        assert_eq!(j.action(1030, true), JobAction::Idle, "running");
        assert_eq!(j.action(1060, true), JobAction::Stop, "at the end");
        assert_eq!(j.action(2000, false), JobAction::Idle, "long after");
    }

    #[test]
    fn a_finished_job_never_restarts() {
        let mut j = job(1000, 60);
        j.done = true;
        assert_eq!(j.action(1030, false), JobAction::Idle, "done is done");
    }

    #[test]
    fn end_is_start_plus_duration() {
        assert_eq!(job(1000, 1800).end_unix(), 2800);
    }

    fn wall(start: u16, stop: u16) -> RecordingJob {
        RecordingJob {
            start_min: start,
            stop_min: stop,
            repeat: JobRepeat::Daily,
            freq_hz: 7_878_100.0,
            ..Default::default()
        }
    }

    #[test]
    fn a_daily_window_runs_from_start_to_just_before_stop() {
        let mut j = wall(9 * 60 + 45, 10 * 60 + 5);
        j.repeat = JobRepeat::Daily;
        assert!(!j.active_at("2026-11-23", 9 * 60 + 44));
        assert!(j.active_at("2026-11-23", 9 * 60 + 45));
        assert!(j.active_at("2026-11-23", 10 * 60 + 4));
        assert!(!j.active_at("2026-11-23", 10 * 60 + 5), "the stop minute is outside");
    }

    #[test]
    fn a_window_that_crosses_midnight_is_active_on_both_sides() {
        let mut j = wall(22 * 60 + 30, 15);
        j.repeat = JobRepeat::Daily;
        assert!(j.active_at("2026-11-23", 23 * 60));
        assert!(j.active_at("2026-11-23", 14));
        assert!(!j.active_at("2026-11-23", 12 * 60));
    }

    #[test]
    fn a_dated_reception_runs_on_its_own_day_only() {
        let mut j = wall(11 * 60 + 30, 12 * 60 + 30);
        j.repeat = JobRepeat::Once;
        j.date = "2026-12-24".into();
        assert!(j.active_at("2026-12-24", 12 * 60));
        assert!(!j.active_at("2026-12-25", 12 * 60));
    }

    #[test]
    fn a_job_with_no_stop_minute_is_the_old_one_shot() {
        let j = job(1000, 60);
        assert!(!j.is_wall_clock());
        assert_eq!(j.wall_action("2026-11-23", 0, false), JobAction::Idle, "not a wall-clock job");
        assert_eq!(j.action(1000, false), JobAction::Start, "and it still runs as it always did");
    }

    #[test]
    fn a_wall_clock_job_starts_and_stops_from_the_clock() {
        let j = wall(600, 700);
        assert_eq!(j.wall_action("2026-11-23", 599, false), JobAction::Idle);
        assert_eq!(j.wall_action("2026-11-23", 600, false), JobAction::Start);
        assert_eq!(j.wall_action("2026-11-23", 650, true), JobAction::Idle);
        assert_eq!(j.wall_action("2026-11-23", 700, true), JobAction::Stop);
        assert_eq!(j.wall_action("2026-11-23", 700, false), JobAction::Idle);
    }

    #[test]
    fn a_wall_clock_job_says_what_is_wrong_with_it() {
        assert_eq!(wall(600, 700).problem("2026-11-23"), None);
        assert!(wall(600, 600).problem("2026-11-23").is_some());
        let mut past = wall(600, 700);
        past.repeat = JobRepeat::Once;
        past.date = "2020-01-01".into();
        assert!(past.problem("2026-11-23").is_some());
        let mut bad = wall(600, 700);
        bad.repeat = JobRepeat::Once;
        bad.date = "24.12.2026".into();
        assert!(bad.problem("2026-11-23").is_some());
    }

    #[test]
    fn the_clock_tunes_ahead_so_a_start_tone_is_not_missed() {
        let mut j = wall(10 * 60, 10 * 60 + 10);
        // Twenty seconds early: already this job's radio to hold.
        assert!(j.active_seconds("2026-11-23", 36_000 - 20, 25));
        assert!(!j.active_seconds("2026-11-23", 36_000 - 30, 25));
        // The end does not move for the early arrival.
        assert!(!j.active_seconds("2026-11-23", 36_600, 25));
        // A job that is not a wall-clock one is never "due" by seconds.
        let old = job(1000, 60);
        assert!(!old.active_seconds("2026-11-23", 0, 25));
        // The lead may not wrap a midnight-crossing window the wrong way.
        j.start_min = 0;
        j.stop_min = 10;
        assert!(j.active_seconds("2026-11-23", 86_395, 25), "23:59:55 leads into a 00:00 start");
        assert!(j.active_seconds("2026-11-23", 5, 25));
        assert!(!j.active_seconds("2026-11-23", 600, 25));
    }

    #[test]
    fn a_radio_selection_matches_only_the_radio_it_names() {
        assert!(JobRadio::Any.matches(3));
        assert!(JobRadio::Id(3).matches(3));
        assert!(!JobRadio::Id(3).matches(4));
        assert_eq!(JobRadio::Id(3).id(), Some(3));
        assert_eq!(JobRadio::Any.id(), None);
    }

    #[test]
    fn an_old_job_still_parses_and_gets_the_new_defaults() {
        // Exactly what a job written before the planner had a wall clock or a
        // radio looked like on disk.
        let text = r#"{"id":3,"at_unix":1789587720,"freq_hz":6185000.0,"mode":"Am",
            "duration_s":1800,"kind":"both","name":"BBC World Service","done":false}"#;
        let j: RecordingJob = serde_json::from_str(text).unwrap();
        assert_eq!(j.repeat, JobRepeat::Once);
        assert!(!j.is_wall_clock());
        assert_eq!(j.radio, JobRadio::Any);
        assert_eq!(j.action(1_789_587_720, false), JobAction::Start);
    }

    #[test]
    fn a_plan_row_can_keep_nothing() {
        let mut j = RecordingJob::default();
        assert_eq!(j.kind, RecordingKind::Audio, "audio stays the default");
        j.kind = RecordingKind::Nothing;
        assert_eq!(RecordingKind::Nothing.label(), "None");
        let text = serde_json::to_string(&j).unwrap();
        assert!(text.contains("nothing"), "the wire name is the serde one: {text}");
        assert_eq!(serde_json::from_str::<RecordingJob>(&text).unwrap().kind, RecordingKind::Nothing);
        // A decoding appointment with no file is still a valid row.
        j.repeat = JobRepeat::Daily;
        j.start_min = 600;
        j.stop_min = 700;
        j.freq_hz = 3_853_100.0;
        assert_eq!(j.problem("2026-11-23"), None);
    }

    #[test]
    fn a_job_round_trips_through_json() {
        let j = RecordingJob {
            id: 3,
            at_unix: 1_789_587_720,
            freq_hz: 6_185_000.0,
            mode: crate::Mode::Am,
            duration_s: 1800,
            kind: RecordingKind::Both,
            name: "BBC World Service".into(),
            done: false,
            ..RecordingJob::default()
        };
        let text = serde_json::to_string(&j).unwrap();
        assert_eq!(serde_json::from_str::<RecordingJob>(&text).unwrap(), j);
    }

    #[test]
    fn the_format_defaults_to_mp3_and_names_its_file() {
        assert_eq!(RecordingFormat::default(), RecordingFormat::Mp3, "the station's own default");
        assert_eq!(RecordingFormat::Mp3.extension(), "mp3");
        assert_eq!(RecordingFormat::Wav.extension(), "wav");
        assert_eq!(RecordingFormat::Wav.label(), "WAV");
    }

    #[test]
    fn an_older_job_still_loads() {
        let j: RecordingJob =
            serde_json::from_str(r#"{"at_unix":1000,"freq_hz":6185000.0}"#).unwrap();
        assert_eq!(j.kind, RecordingKind::Audio, "a missing kind defaults to audio");
        assert_eq!(j.duration_s, 1800, "and a missing duration to half an hour");
    }
}
