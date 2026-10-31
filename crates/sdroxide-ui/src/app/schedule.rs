//! The broadcast schedule window: what is on, when, where — and a way to tune
//! to it and log it.
//!
//! The EiBi table is already in the program (it labels the waterfall); this
//! turns it into the list a listener works from. Filter by time, band, language
//! and a free-text query, tune a row, or open a reception with the station
//! filled in.

use crate::i18n::tr;
use eframe::egui::{self, RichText};
use sdroxide_types::{
    BroadcastStation, Command, ProgramsFilters, ProgramsSort, RxId, ScheduleSource, Vfo, broadcast,
};

use crate::app::SdroxideApp;
use crate::app::swl_log::SwlEditForm;
use crate::time::now_unix;

/// The SCHEDULE window's default width.
pub(in crate::app) const SCHEDULE_WINDOW_W: f32 = 1000.0;

/// The narrowest the window goes. **The top bar is two rows and this is the
/// budget they are drawn to**: each row's widgets add up to less than this in
/// every language (see the `bar_budget` test), so the chips never stack on one
/// another or wrap onto a third row. Below it the window simply stops shrinking.
pub(in crate::app) const SCHEDULE_WINDOW_MIN_W: f32 = 860.0;

/// Widths of the three text fields on the first bar row.
const FIND_W: f32 = 130.0;
const LANG_W: f32 = 80.0;
const TARGET_W: f32 = 80.0;
/// Widths of the two comboboxes on the second bar row.
const BAND_W: f32 = 72.0;
const SOURCE_W: f32 = 96.0;
/// The gap between two columns of the list.
const GAP: f32 = 8.0;

/// The sources offered in the source filter, in the order the store ranks them.
const SOURCE_FILTERS: &[(ScheduleSource, &str)] = &[
    (ScheduleSource::Eibi, "EiBi"),
    (ScheduleSource::Aoki, "Aoki"),
    (ScheduleSource::Hfcc, "HFCC"),
    (ScheduleSource::User, "mine"),
];

/// The filter's own label, which must say **all of them** rather than any source's
/// name — the default is the store, and a combobox reading "EiBi" when three
/// schedules are loaded would be the control quietly lying about its state.
fn source_label(source: Option<ScheduleSource>) -> String {
    match source {
        None => tr!("schedule-all-of-them"),
        Some(s) => SOURCE_FILTERS
            .iter()
            .find(|(x, _)| *x == s)
            .map(|(x, n)| source_name(*x, n))
            .unwrap_or_else(|| format!("{s:?}")),
    }
}

/// A source's name as drawn: the schedules are proper names, but "mine" is a word.
fn source_name(source: ScheduleSource, name: &str) -> String {
    if source == ScheduleSource::User { tr!("schedule-source-mine") } else { name.to_string() }
}

/// The metre bands offered in the band filter. Empty = any.
const BAND_FILTERS: [&str; 20] = [
    "", "LW", "MW", "120m", "90m", "75m", "60m", "49m", "41m", "31m", "25m", "22m", "19m", "16m",
    "15m", "13m", "11m", "FM", "AIR", "MIL",
];

/// The schedule window's state: whether it is open, and its filters.
pub(in crate::app) struct ScheduleUi {
    pub show: bool,
    pub query: String,
    pub lang: String,
    pub target: String,
    /// Empty = any; otherwise a metre band name from [`BAND_FILTERS`].
    pub band: String,
    /// The filter time as UTC `HHMM`, ignored while `use_now`.
    pub hhmm: u16,
    pub use_now: bool,
    /// Show only the listener's favourite stations.
    pub favourites_only: bool,
    /// Which schedule to show. `None` is **all of them**, which is the default:
    /// the master store holds every downloaded source and the repetition is
    /// accepted, because this window is where the detail belongs.
    pub source: Option<ScheduleSource>,
    /// Show each row's local time at the transmitter (mean solar time, from the
    /// site's longitude). Off by default: most listeners work in UTC, and the
    /// column is only as good as the site coordinate.
    pub local_time: bool,
    /// The column the list is ordered by, and whether it is read backwards.
    pub sort: ProgramsSort,
    pub sort_desc: bool,
    /// A filter changed and has not been written to the config yet. It waits for
    /// the keyboard to be free so typing does not rewrite the file per letter.
    unsaved: bool,
    /// The filtered list, and the key it was built for. Rebuilt only when the
    /// filters, the favourites, the loaded schedule or the UTC minute change —
    /// not sixty times a second over four and a half thousand rows.
    cache_key: Option<CacheKey>,
    cache: Vec<BroadcastStation>,
}

type CacheKey =
    (String, String, String, String, bool, usize, i64, Option<ScheduleSource>, ProgramsSort, bool);

impl Default for ScheduleUi {
    fn default() -> Self {
        ScheduleUi {
            show: false,
            query: String::new(),
            lang: String::new(),
            target: String::new(),
            band: String::new(),
            hhmm: 0,
            use_now: true,
            favourites_only: false,
            source: None,
            local_time: false,
            sort: ProgramsSort::Freq,
            sort_desc: false,
            unsaved: false,
            cache_key: None,
            cache: Vec::new(),
        }
    }
}

impl ScheduleUi {
    /// The window as it was left, from the saved filters.
    pub(in crate::app) fn from_settings(f: &ProgramsFilters) -> Self {
        ScheduleUi {
            query: f.query.clone(),
            lang: f.lang.clone(),
            target: f.target.clone(),
            band: f.band.clone(),
            hhmm: f.hhmm,
            use_now: f.use_now,
            favourites_only: f.favourites_only,
            source: f.source,
            local_time: f.local_time,
            sort: f.sort,
            sort_desc: f.sort_desc,
            ..Default::default()
        }
    }

    /// The filters to save. `all` is the ALL chip, which lives on the app.
    fn to_settings(&self, all: bool) -> ProgramsFilters {
        ProgramsFilters {
            query: self.query.clone(),
            lang: self.lang.clone(),
            target: self.target.clone(),
            band: self.band.clone(),
            hhmm: self.hhmm,
            use_now: self.use_now,
            favourites_only: self.favourites_only,
            source: self.source,
            local_time: self.local_time,
            all,
            sort: self.sort,
            sort_desc: self.sort_desc,
        }
    }
}

/// Order the list by `col`, the other way round when `desc`. Ties fall back to
/// frequency (always ascending), so equal names or bands stay in dial order.
fn sort_rows(rows: &mut [BroadcastStation], col: ProgramsSort, desc: bool) {
    use std::cmp::Ordering;
    fn by_freq(a: &BroadcastStation, b: &BroadcastStation) -> Ordering {
        a.freq_khz.partial_cmp(&b.freq_khz).unwrap_or(Ordering::Equal)
    }
    fn band_rank(khz: f64) -> usize {
        broadcast::metre_band(khz)
            .and_then(|b| BAND_FILTERS.iter().position(|x| *x == b))
            .unwrap_or(usize::MAX)
    }
    rows.sort_by(|a, b| {
        let primary = match col {
            ProgramsSort::Freq => by_freq(a, b),
            ProgramsSort::Band => band_rank(a.freq_khz).cmp(&band_rank(b.freq_khz)),
            ProgramsSort::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            ProgramsSort::Lang => a.lang.to_lowercase().cmp(&b.lang.to_lowercase()),
            ProgramsSort::Target => a.target.to_lowercase().cmp(&b.target.to_lowercase()),
            ProgramsSort::Site => a.site.to_lowercase().cmp(&b.site.to_lowercase()),
        };
        (if desc { primary.reverse() } else { primary }).then_with(|| by_freq(a, b))
    });
}

/// The UTC instant the schedule is filtered at: now, or today at `hhmm`.
fn schedule_time(now: i64, ui: &ScheduleUi) -> i64 {
    if ui.use_now {
        return now;
    }
    let day = now.div_euclid(86_400) * 86_400;
    day + (ui.hhmm as i64 / 100) * 3600 + (ui.hhmm as i64 % 100) * 60
}

fn hhmm_text(hhmm: u16) -> String {
    format!("{:02}:{:02}", hhmm / 100, hhmm % 100)
}

impl SdroxideApp {
    /// The SCHEDULE window. Tuning and logging are done by the commands and the
    /// entry form it fills in, after the window has closed the borrow of the log
    /// — so the body carries the three asks back and the tail acts on them.
    pub(in crate::app) fn schedule_window(&mut self, ctx: &egui::Context, cmds: &mut Vec<Command>) {
        let mut tune: Option<BroadcastStation> = None;
        let mut log: Option<BroadcastStation> = None;
        let mut fav_toggle: Option<(String, bool)> = None;
        let mut all_toggle: Option<bool> = None;
        if !self.schedule.show {
            self.schedule_was_open = false;
            // Closed with a filter still unwritten (the keyboard was busy).
            if self.schedule.unsaved {
                self.save_programs_filters(ctx);
            }
        } else {
            // Each opening gets its own window id, so the remembered geometry
            // is what applies — see `remembered_tool_window`.
            if !self.schedule_was_open {
                self.schedule_epoch += 1;
                self.schedule_was_open = true;
            }
            let epoch = self.schedule_epoch;
            self.schedule.show = self.remembered_tool_window(
                ctx,
                "schedule",
                &tr!("schedule-title"),
                [SCHEDULE_WINDOW_W, 560.0],
                [SCHEDULE_WINDOW_MIN_W, 240.0],
                epoch,
                |me, ui| {
                    me.schedule_body(ui, &mut tune, &mut log, &mut fav_toggle, &mut all_toggle)
                },
            );
            self.save_programs_filters(ctx);
        }

        if let Some((name, on)) = fav_toggle {
            self.broadcast_favs.retain(|n| n != &name);
            if on {
                self.broadcast_favs.push(name);
                self.broadcast_favs.sort();
            }
            crate::app::persist::persist_broadcast_favourites(&self.broadcast_favs);
        }
        if let Some(on) = all_toggle {
            self.set_broadcast_all(on);
        }
        if let Some(s) = tune {
            self.tune_to_station(&s, cmds);
        }
        if let Some(s) = log {
            let smeter = self.meters.map(|m| m.s_dbm);
            let grid = self.my_grid();
            let antenna = self.swl_antenna.clone();
            self.swl_edit = Some(SwlEditForm::from_station(
                s.freq_hz(),
                s.mode(),
                &s.name,
                &s.lang,
                &s.site,
                &s.email,
                &s.address,
                s.lat,
                s.lon,
                smeter,
                grid,
                antenna,
            ));
            self.show_swl = true;
        }
    }

    /// Keep the PROGRAMS filters between sessions. A change is copied into the
    /// settings at once; it is written to the config when no text box is being
    /// typed in (or the window has closed), so a search is one write and not one
    /// per letter.
    fn save_programs_filters(&mut self, ctx: &egui::Context) {
        let now = self.schedule.to_settings(self.ui_settings.programs.all);
        if now != self.ui_settings.programs {
            self.ui_settings.programs = now;
            self.schedule.unsaved = true;
        }
        if self.schedule.unsaved && (ctx.memory(|m| m.focused()).is_none() || !self.schedule.show) {
            self.schedule.unsaved = false;
            crate::app::persist::persist_ui_settings(&self.ui_settings);
        }
    }

    /// Tunes the receiver to a PROGRAMS row.
    pub(in crate::app) fn tune_to_station(
        &mut self,
        s: &BroadcastStation,
        cmds: &mut Vec<Command>,
    ) {
        // The VFO the receiver is on, not always A: the engine ignores a
        // retune of the other one, and the dial would stay where it was.
        let vfo = self.state.active_vfo;
        let (hz, mode) = (s.freq_hz(), s.mode());
        cmds.push(Command::SetVfo { vfo, hz });
        cmds.push(Command::SetMode { rx: RxId::Main, mode });
        // Echoed now, as every other route that tunes from the screen does, so
        // the dial does not show the old frequency while the engine answers.
        self.state.rx[0].mode = mode;
        match vfo {
            Vfo::A => self.state.vfo_a_hz = hz,
            Vfo::B => self.state.vfo_b_hz = hz,
        }
    }

    /// The schedule's body: the filters, the on-air list and its row actions.
    /// Split out of [`Self::schedule_window`] so the shell can draw it in a
    /// window of its own; the three row asks go back through the parameters.
    fn schedule_body(
        &mut self,
        ui: &mut egui::Ui,
        tune: &mut Option<BroadcastStation>,
        log: &mut Option<BroadcastStation>,
        fav_toggle: &mut Option<(String, bool)>,
        all_toggle: &mut Option<bool>,
    ) {
        crate::chrome::window_body_bg(ui);
        let now = now_unix() as i64;
        let at = schedule_time(now, &self.schedule);

        // The matching rows, cloned out before the closure borrows `self` to
        // edit the filters and to open the log.
        // On-air status changes at the minute, so the cache keys on the minute,
        // not on the second.
        let key: CacheKey = (
            self.schedule.query.clone(),
            self.schedule.lang.clone(),
            self.schedule.target.clone(),
            self.schedule.band.clone(),
            self.schedule.favourites_only,
            self.broadcast.len(),
            at.div_euclid(60),
            self.schedule.source,
            self.schedule.sort,
            self.schedule.sort_desc,
        );
        let rows: Vec<BroadcastStation> = if self.schedule.cache_key.as_ref() == Some(&key) {
            // Lifted out for the frame and put back afterwards, so the closure
            // below is free to borrow `self` mutably.
            std::mem::take(&mut self.schedule.cache)
        } else {
            let f = &self.schedule;
            let mut v: Vec<BroadcastStation> = self
                .broadcast
                .iter()
                .filter(|s| {
                    s.on_air_at(at)
                        && f.source.is_none_or(|w| s.source == w)
                        && s.matches_query(&f.query)
                        && broadcast::contains_ascii_ci(&s.lang, &f.lang)
                        && broadcast::contains_ascii_ci(&s.target, &f.target)
                        && (f.band.is_empty()
                            || broadcast::metre_band(s.freq_khz) == Some(f.band.as_str()))
                        && (!f.favourites_only || self.broadcast_favs.iter().any(|n| n == &s.name))
                })
                .cloned()
                .collect();
            sort_rows(&mut v, f.sort, f.sort_desc);
            v
        };
        let count = rows.len();

        // **Two rows, no more.** The first is the time and the three text
        // searches; the second the two dropdowns, the chips and the clock. Each is
        // budgeted against `SCHEDULE_WINDOW_MIN_W` (see `bar_budget`), so the chips
        // do not climb over one another when the window is narrow. `wrapped` is
        // only the safety net for a screen narrower than the window's floor.
        ui.horizontal_wrapped(|ui| {
            ui.label(tr!("schedule-at"));
            if crate::chrome::chip(ui, self.schedule.use_now, tr!("schedule-now-chip")).clicked() {
                self.schedule.use_now = true;
            }
            let mut hh = self.schedule.hhmm as i32;
            let r = ui.add_enabled(
                !self.schedule.use_now,
                egui::DragValue::new(&mut hh)
                    .speed(1.0)
                    .range(0..=2359)
                    .custom_formatter(|v, _| hhmm_text(v as u16)),
            );
            if r.changed() {
                self.schedule.hhmm = hh.clamp(0, 2359) as u16;
            }
            if ui.button(tr!("schedule-set-time")).clicked() {
                self.schedule.use_now = false;
            }
            ui.separator();
            ui.label(tr!("signalid-find"));
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut self.schedule.query)
                    .desired_width(FIND_W)
                    .hint_text(tr!("schedule-find-hint")),
            );
            ui.label(tr!("settings-ui-language"));
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut self.schedule.lang).desired_width(LANG_W),
            );
            ui.label(tr!("schedule-target"));
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut self.schedule.target).desired_width(TARGET_W),
            );
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(tr!("settings-relay-band"));
            egui::ComboBox::from_id_salt("sched-band")
                .width(BAND_W)
                .selected_text(if self.schedule.band.is_empty() {
                    tr!("schedule-any")
                } else {
                    self.schedule.band.clone()
                })
                .show_ui(ui, |ui| {
                    for b in BAND_FILTERS {
                        let label = if b.is_empty() { tr!("schedule-any") } else { b.to_string() };
                        ui.selectable_value(&mut self.schedule.band, b.to_string(), label);
                    }
                });
            ui.label(tr!("schedule-source"));
            egui::ComboBox::from_id_salt("sched-source")
                .width(SOURCE_W)
                .selected_text(source_label(self.schedule.source))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.schedule.source,
                        None,
                        tr!("schedule-all-of-them"),
                    )
                    .on_hover_text(tr!("schedule-all-hint"));
                    for (src, what) in SOURCE_FILTERS {
                        ui.selectable_value(
                            &mut self.schedule.source,
                            Some(*src),
                            source_name(*src, what),
                        );
                    }
                });
            if crate::chrome::chip(ui, self.schedule.favourites_only, tr!("schedule-favs"))
                .on_hover_text(tr!("schedule-favs-hint"))
                .clicked()
            {
                self.schedule.favourites_only = !self.schedule.favourites_only;
            }
            if crate::chrome::chip(ui, self.broadcast_all, tr!("schedule-all-favs"))
                .on_hover_text(tr!("schedule-all-favs-hint"))
                .clicked()
            {
                *all_toggle = Some(!self.broadcast_all);
            }
            if crate::chrome::chip(ui, self.schedule.local_time, tr!("schedule-solar-time"))
                .on_hover_text(tr!("schedule-solar-time-hint"))
                .clicked()
            {
                self.schedule.local_time = !self.schedule.local_time;
            }
            ui.separator();
            // **The way into LISTEN**, and deliberately here rather than as a
            // chip of its own on the top bar: the SWL row is already at the
            // two-row limit at 1400 pt, and a third chip pushed AM — the main
            // SWL mode — onto a third row. SCHEDULE is already in that row and is
            // the same data, so this costs no width and loses nothing.
            if crate::chrome::chip(ui, self.show_listen, tr!("schedule-map"))
                .on_hover_text(tr!("schedule-map-hint"))
                .clicked()
            {
                self.show_listen = !self.show_listen;
                if self.show_listen
                    && let Some(s) = sdroxide_types::broadcast::at_dial(
                        &self.broadcast,
                        self.state.rx_freq_hz(),
                        now_unix(),
                    )
                {
                    // Open it with **what is on the dial** already selected, which
                    // is the station the operator is listening to and the one they
                    // most often want to write down.
                    let name = s.name.clone();
                    self.listen_select(&name);
                }
            }
        });
        ui.add_space(2.0);
        // The count on the left; the clock and the build on the right. They were
        // on the chip row until that row ran out of width.
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(tr!(
                    "schedule-on-air",
                    count = count,
                    time = if self.schedule.use_now {
                        tr!("schedule-now")
                    } else {
                        hhmm_text(self.schedule.hhmm)
                    }
                ))
                .size(11.0)
                .color(crate::theme::gray(150)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // The build, beside the clock — see the SWL LOG window.
                ui.label(
                    RichText::new(format!("v{}", sdroxide_version::VERSION))
                        .size(10.5)
                        .color(crate::theme::gray(120)),
                )
                .on_hover_text(sdroxide_version::LONG_VERSION);
                ui.label(
                    RichText::new(crate::time::utc_and_local_now(now))
                        .monospace()
                        .color(crate::theme::CYAN()),
                );
            });
        });
        ui.add_space(2.0);

        // The list's columns. The header and every row are cut to the same
        // widths, so a long name or a missing language no longer moves the rest
        // of its row, and the controls sit over the column they act on.
        let cols = Cols::new(ui, ui.available_width(), self.schedule.local_time);
        let row_h = crate::chrome::chip_height(ui, None);
        self.schedule_header(ui, &cols, row_h);
        ui.separator();

        let local_time = self.schedule.local_time;
        egui::ScrollArea::vertical().auto_shrink([false, false]).id_salt("sched-list").show_rows(
            ui,
            row_h,
            rows.len(),
            |ui, range| {
                for s in &rows[range] {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = GAP;
                        cell(ui, cols.freq, row_h, egui::Align::Max, |ui| {
                            ui.label(
                                RichText::new(format!("{}", s.freq_khz.round() as i64)).monospace(),
                            );
                        });
                        cell(ui, cols.band, row_h, egui::Align::Min, |ui| {
                            ui.label(
                                RichText::new(broadcast::metre_band(s.freq_khz).unwrap_or(""))
                                    .size(11.0)
                                    .color(crate::theme::CYAN()),
                            );
                        });
                        cell(ui, cols.name, row_h, egui::Align::Min, |ui| {
                            ui.add(egui::Label::new(RichText::new(&s.name).strong()).truncate());
                        });
                        for (w, text, grey) in [
                            (cols.lang, &s.lang, 160),
                            (cols.target, &s.target, 160),
                            (cols.site, &s.site, 140),
                        ] {
                            cell(ui, w, row_h, egui::Align::Min, |ui| {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(text)
                                            .size(11.0)
                                            .color(crate::theme::gray(grey)),
                                    )
                                    .truncate(),
                                );
                            });
                        }
                        if local_time {
                            cell(ui, cols.local, row_h, egui::Align::Min, |ui| {
                                if let Some(lon) = s.lon {
                                    let (_, _, _, h, mi, _) = sdroxide_types::utc_ymd_hms(now);
                                    let local = broadcast::local_solar_hhmm(
                                        h as u16 * 100 + mi as u16,
                                        lon,
                                    );
                                    ui.label(
                                        RichText::new(local.clone())
                                            .size(11.0)
                                            .monospace()
                                            .color(crate::theme::gray(140)),
                                    )
                                    .on_hover_text(tr!(
                                        "schedule-solar-hover",
                                        local = local,
                                        site = s.site,
                                        lon = format!("{lon:.1}")
                                    ));
                                }
                            });
                        }
                        let stock = crate::theme::spot_color(sdroxide_types::SpotKind::Broadcast);
                        let mut col = self
                            .ui_settings
                            .programs_colours
                            .get(&s.name)
                            .copied()
                            .or(self.ui_settings.programs_colour_all)
                            .unwrap_or([stock.0, stock.1, stock.2]);
                        cell(ui, cols.colour, row_h, egui::Align::Center, |ui| {
                            if super::colour_edit::colour_swatch_framed(ui, &mut col)
                                .on_hover_text(tr!("schedule-colour-station-hint"))
                                .changed()
                            {
                                self.ui_settings.programs_colours.insert(s.name.clone(), col);
                                crate::app::persist::persist_ui_settings(&self.ui_settings);
                            }
                        });
                        let fav = self.broadcast_favs.iter().any(|n| n == &s.name);
                        cell(ui, cols.star, row_h, egui::Align::Center, |ui| {
                            if ui
                                .small_button(if fav { "★" } else { "☆" })
                                .on_hover_text(tr!("schedule-favourite-hint"))
                                .clicked()
                            {
                                *fav_toggle = Some((s.name.clone(), !fav));
                            }
                        });
                        cell(ui, cols.listen, row_h, egui::Align::Min, |ui| {
                            if crate::chrome::chip(ui, false, tr!("schedule-listen"))
                                .on_hover_text(tr!("schedule-listen-hint"))
                                .clicked()
                            {
                                *tune = Some(s.clone());
                            }
                        });
                        cell(ui, cols.log, row_h, egui::Align::Min, |ui| {
                            if crate::chrome::chip(ui, false, tr!("topbar-chip-log")).clicked() {
                                *log = Some(s.clone());
                            }
                        });
                    });
                }
            },
        );

        self.schedule.cache = rows;
        self.schedule.cache_key = Some(key);
    }

    /// The list's header, in three rows over the same columns as the data: a
    /// SORT menu over each data column, the column names, then the colour
    /// controls — the palette over the colour column, PST over the stars and
    /// DEFAULT over LISTEN.
    fn schedule_header(&mut self, ui: &mut egui::Ui, cols: &Cols, row_h: f32) {
        let local = self.schedule.local_time;
        let data: [(ProgramsSort, f32, String); 6] = [
            (ProgramsSort::Freq, cols.freq, tr!("schedule-col-freq")),
            (ProgramsSort::Band, cols.band, tr!("schedule-col-band")),
            (ProgramsSort::Name, cols.name, tr!("schedule-col-name")),
            (ProgramsSort::Lang, cols.lang, tr!("schedule-col-lang")),
            (ProgramsSort::Target, cols.target, tr!("schedule-col-target")),
            (ProgramsSort::Site, cols.site, tr!("schedule-col-site")),
        ];

        // Row 1 — the SORT menus.
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = GAP;
            for (col, w, _) in &data {
                let active = self.schedule.sort == *col;
                let arrow = match (active, self.schedule.sort_desc) {
                    (false, _) => "",
                    (true, false) => " ▲",
                    (true, true) => " ▼",
                };
                cell(ui, *w, row_h, egui::Align::Min, |ui| {
                    egui::ComboBox::from_id_salt(("sched-sort", *col as u8))
                        .width(*w - 20.0)
                        .selected_text(
                            RichText::new(format!("{}{arrow}", tr!("schedule-sort"))).size(10.5),
                        )
                        .show_ui(ui, |ui| {
                            let asc = active && !self.schedule.sort_desc;
                            let desc = active && self.schedule.sort_desc;
                            if ui.selectable_label(asc, tr!("schedule-sort-asc")).clicked() {
                                self.schedule.sort = *col;
                                self.schedule.sort_desc = false;
                            }
                            if ui.selectable_label(desc, tr!("schedule-sort-desc")).clicked() {
                                self.schedule.sort = *col;
                                self.schedule.sort_desc = true;
                            }
                        });
                });
            }
        });

        // Row 2 — the column names.
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = GAP;
            for (_, w, name) in &data {
                cell(ui, *w, row_h, egui::Align::Min, |ui| {
                    ui.label(RichText::new(name).size(10.5).color(crate::theme::gray(150)));
                });
            }
            if local {
                cell(ui, cols.local, row_h, egui::Align::Min, |ui| {
                    ui.label(
                        RichText::new(tr!("schedule-col-local"))
                            .size(10.5)
                            .color(crate::theme::gray(150)),
                    );
                });
            }
            cell(ui, cols.colour, row_h, egui::Align::Center, |ui| {
                ui.label(
                    RichText::new(tr!("schedule-colour-all"))
                        .size(10.5)
                        .color(crate::theme::gray(150)),
                )
                .on_hover_text(tr!("schedule-colour-all-hint"));
            });
        });

        // Row 3 — the colour for every station: palette, PST, DEFAULT.
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = GAP;
            let before = cols.freq
                + cols.band
                + cols.name
                + cols.lang
                + cols.target
                + cols.site
                + if local { cols.local } else { 0.0 }
                + (if local { 7.0 } else { 6.0 }) * GAP;
            ui.add_space(before);
            let stock = crate::theme::spot_color(sdroxide_types::SpotKind::Broadcast);
            let mut all =
                self.ui_settings.programs_colour_all.unwrap_or([stock.0, stock.1, stock.2]);
            let swatch = cell(ui, cols.colour, row_h, egui::Align::Center, |ui| {
                super::colour_edit::colour_swatch_framed(ui, &mut all)
            });
            let paste = cell(ui, cols.star, row_h, egui::Align::Center, |ui| {
                crate::chrome::chip(ui, false, "PST")
            });
            if super::colour_edit::colour_paste(ui, &mut all, &swatch, paste) | swatch.changed() {
                // "All" means all: the colours picked one by one give way to it.
                self.ui_settings.programs_colour_all = Some(all);
                self.ui_settings.programs_colours.clear();
                crate::app::persist::persist_ui_settings(&self.ui_settings);
            }
            cell(ui, cols.listen, row_h, egui::Align::Min, |ui| {
                if crate::chrome::chip(ui, false, tr!("schedule-colour-reset"))
                    .on_hover_text(tr!("schedule-colour-reset-hint"))
                    .clicked()
                {
                    self.ui_settings.programs_colour_all = None;
                    self.ui_settings.programs_colours.clear();
                    crate::app::persist::persist_ui_settings(&self.ui_settings);
                }
            });
        });
    }
}

/// One cell of the list: `w` wide whatever it holds, so the columns line up
/// from row to row. The content is aligned inside it by `align`.
fn cell<R>(
    ui: &mut egui::Ui,
    w: f32,
    h: f32,
    align: egui::Align,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let layout = match align {
        egui::Align::Max => egui::Layout::right_to_left(egui::Align::Center),
        // Centred by the layout, not by a justified one that would stretch the
        // widget: the cell is as tall as the widgets it holds.
        egui::Align::Center => egui::Layout::top_down(egui::Align::Center),
        egui::Align::Min => egui::Layout::left_to_right(egui::Align::Center),
    };
    let mut out = None;
    ui.allocate_ui_with_layout(egui::vec2(w, h), layout, |ui| {
        ui.set_width(w);
        ui.set_clip_rect(ui.clip_rect().intersect(ui.max_rect()));
        out = Some(add(ui));
    });
    out.expect("the cell body ran")
}

/// The widths of the list's columns.
struct Cols {
    freq: f32,
    band: f32,
    name: f32,
    lang: f32,
    target: f32,
    site: f32,
    local: f32,
    colour: f32,
    star: f32,
    listen: f32,
    log: f32,
}

impl Cols {
    /// The columns for a list `avail` wide. Every column is as wide as the
    /// widest thing that ever goes in it — its SORT menu, its chip — measured in
    /// the language on screen, so nothing overlaps its neighbour; the name takes
    /// whatever is left.
    fn new(ui: &egui::Ui, avail: f32, local: bool) -> Self {
        let font = egui::FontId::proportional(10.5);
        let sort_w =
            crate::chrome::text_width(ui, &format!("{} ▼", tr!("schedule-sort")), font) + 28.0;
        let chip = |label: String| crate::chrome::chip_width(ui, &label, None);
        let colour = super::colour_edit::colour_swatch_framed_width();
        let star = chip("PST".into()).max(30.0);
        let listen = chip(tr!("schedule-listen")).max(chip(tr!("schedule-colour-reset")));
        let log = chip(tr!("topbar-chip-log"));
        let (freq, band) = (sort_w.max(54.0), sort_w.max(46.0));
        let (lang, target, site) = (sort_w.max(64.0), sort_w.max(64.0), sort_w.max(100.0));
        let local_w = if local { 52.0 } else { 0.0 };
        let n = if local { 11.0 } else { 10.0 };
        let fixed = freq + band + lang + target + site + local_w + colour + star + listen + log;
        let name = (avail - fixed - (n - 1.0) * GAP - 2.0).max(90.0);
        Cols { freq, band, name, lang, target, site, local: local_w, colour, star, listen, log }
    }
}

#[cfg(test)]
mod source_filter_tests {
    use super::{ScheduleUi, source_label};
    use sdroxide_types::ScheduleSource;

    /// **The default has to say what it actually does.** With three schedules in
    /// the store, a combobox reading "EiBi" would be this fork's house rule in
    /// miniature — a control that cannot be misread. It reads *all of them*.
    #[test]
    fn the_source_filter_default_says_all_of_them() {
        assert_eq!(source_label(None), "all of them");
        assert_eq!(ScheduleUi::default().source, None, "and starts on the store");
    }

    /// Every source is **distinguishable** in the filter. Two sources sharing a
    /// label would make the combobox a control that cannot say which one is
    /// selected — the same fault as the default being wrong, one row over.
    #[test]
    fn every_source_has_its_own_name() {
        let all = [
            ScheduleSource::Eibi,
            ScheduleSource::Aoki,
            ScheduleSource::Hfcc,
            ScheduleSource::User,
        ];
        let mut seen: Vec<String> = Vec::new();
        for src in all {
            let label = source_label(Some(src));
            assert!(!label.is_empty(), "{src:?} has no name at all");
            assert!(
                !seen.contains(&label),
                "{src:?} reads {label:?}, which another source also uses"
            );
            seen.push(label);
        }
        assert_eq!(source_label(Some(ScheduleSource::Aoki)), "Aoki");
    }
}

#[cfg(test)]
mod bar_budget {
    use super::*;

    /// What each of the two bar rows asks of the window, in points, measured
    /// with the real chip and text metrics. Labels are English stretched by a
    /// third — German and Dutch run about that much longer — because the language
    /// is process-wide and a test cannot switch it without racing the others.
    fn rows_need(ctx: &egui::Context) -> (f32, f32) {
        let mut out = (0.0, 0.0);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1000.0, 600.0),
            )),
            ..Default::default()
        };
        let _ = ctx.run_ui(input, |ui| {
            let gap = ui.spacing().item_spacing.x;
            let label =
                |t: &str| crate::chrome::text_width(ui, t, egui::FontId::proportional(14.0)) * 1.33;
            let chip = |t: &str| crate::chrome::chip_width(ui, t, None) * 1.33;
            let btn = |t: &str| label(t) + 2.0 * ui.spacing().button_padding.x;
            let sep = 1.0;
            let row1 = [
                label("At"),
                chip("NOW"),
                50.0,
                btn("set time"),
                sep,
                label("Find"),
                FIND_W,
                label("Language"),
                LANG_W,
                label("Target"),
                TARGET_W,
            ];
            let row2 = [
                label("Band"),
                BAND_W + 20.0,
                label("Source"),
                SOURCE_W + 20.0,
                chip("★ FAVS"),
                chip("⭐ ALL"),
                chip("LOCAL TIME"),
                sep,
                chip("SHOW MAP"),
            ];
            let total = |r: &[f32]| r.iter().sum::<f32>() + gap * (r.len() as f32 - 1.0);
            out = (total(&row1), total(&row2));
        });
        out
    }

    /// **Two rows, and each fits the window's floor.** The chips of the old
    /// single wrapping row stacked on one another in a narrow window; the bar is
    /// now two rows whose widths are checked here against the narrowest the
    /// window goes, window margins taken off.
    #[test]
    fn each_bar_row_fits_the_narrowest_window() {
        let ctx = egui::Context::default();
        let inner = SCHEDULE_WINDOW_MIN_W - 2.0 * 10.0 - 2.0;
        let (r1, r2) = rows_need(&ctx);
        assert!(r1 < inner, "the time and search row needs {r1:.0} pt of {inner:.0}");
        assert!(r2 < inner, "the dropdown and chip row needs {r2:.0} pt of {inner:.0}");
    }
}

#[cfg(test)]
mod sort_and_filters {
    use super::*;

    fn st(name: &str, khz: f64, lang: &str) -> BroadcastStation {
        BroadcastStation {
            name: name.into(),
            freq_khz: khz,
            lang: lang.into(),
            ..Default::default()
        }
    }

    fn names(rows: &[BroadcastStation]) -> Vec<&str> {
        rows.iter().map(|s| s.name.as_str()).collect()
    }

    #[test]
    fn the_list_sorts_by_each_column_both_ways() {
        let mut v = vec![
            st("bravo", 9410.0, "FR"),
            st("Alpha", 6070.0, "en"),
            st("charlie", 15735.0, "FR"),
        ];
        sort_rows(&mut v, ProgramsSort::Freq, false);
        assert_eq!(names(&v), ["Alpha", "bravo", "charlie"]);
        sort_rows(&mut v, ProgramsSort::Freq, true);
        assert_eq!(names(&v), ["charlie", "bravo", "Alpha"]);
        sort_rows(&mut v, ProgramsSort::Name, false);
        assert_eq!(names(&v), ["Alpha", "bravo", "charlie"], "names compare without case");
        sort_rows(&mut v, ProgramsSort::Name, true);
        assert_eq!(names(&v), ["charlie", "bravo", "Alpha"]);
        sort_rows(&mut v, ProgramsSort::Lang, false);
        assert_eq!(names(&v), ["Alpha", "bravo", "charlie"], "equal languages keep dial order");
        sort_rows(&mut v, ProgramsSort::Band, false);
        assert_eq!(
            names(&v),
            ["Alpha", "bravo", "charlie"],
            "49m, 31m, 19m is 6070 before 9410 before 15735"
        );
    }

    /// Every filter, the sort and the TOUT chip go out and come back as they
    /// were, and the window built from them filters the same way.
    #[test]
    fn the_window_reopens_as_it_was_left() {
        let mut ui = ScheduleUi::default();
        ui.query = "bbc".into();
        ui.lang = "en".into();
        ui.target = "af".into();
        ui.band = "49m".into();
        ui.hhmm = 1423;
        ui.use_now = false;
        ui.favourites_only = true;
        ui.source = Some(ScheduleSource::Hfcc);
        ui.local_time = true;
        ui.sort = ProgramsSort::Site;
        ui.sort_desc = true;
        let saved = ui.to_settings(true);
        assert!(saved.all, "TOUT is kept");
        let back = ScheduleUi::from_settings(&saved);
        assert_eq!(back.to_settings(true), saved);
        assert!(!back.use_now && back.local_time && back.sort_desc);
        assert_eq!(back.source, Some(ScheduleSource::Hfcc));
        // A fresh install opens on "now", sorted by frequency.
        let fresh = ScheduleUi::from_settings(&ProgramsFilters::default());
        assert!(fresh.use_now);
        assert_eq!(fresh.sort, ProgramsSort::Freq);
    }
}
