//! **AIRCRAFT**: one window for everything the radio can put on a map of the
//! sky. A column of mode buttons on the left, the map of the mode in force on
//! the right. The System box's EXTRAS window opens it, and like EXTRAS it is a
//! tool window — it can be pulled out into a window of its own with ⇱, and it
//! remembers where it was left.
//!
//! The maps are the ones the modes already have ([`crate::adsb_map`],
//! [`crate::hfdl_map`]), drawn here at the window's size rather than under the
//! waterfall. A new mode is one entry in [`MODES`] and one arm in
//! `aircraft_map`.
//!
//! One radio hears one wideband mode at a time, so the column is a mode switch:
//! a button sends `SetMode` exactly as the band menu does, and the map shows
//! what that mode has heard.
//!
//! **Cards** is not a mode: it sets nothing on the radio. It swaps the map for
//! the list of aircraft every mode has heard, merged per aircraft by
//! [`super::aircraft_registry`] and worded by [`super::aircraft_card`], so the
//! list survives a change of mode.

use eframe::egui::{self, RichText};
use sdroxide_types::{Command, Mode, RxId};

use super::aircraft_card::{self, CardKey};
use super::aircraft_export;
use super::aircraft_history::{self, HistoryOffer, HistoryState};
use super::aircraft_online::{self, Badge, Provenance, ThumbStatus};
use super::aircraft_search;
use super::aircraft_share;
use super::aircraft_stats::{self, SortBy, Stats};
use super::aircraft_watch::{self, WatchState};
use sdroxide_aircraft_history::export::Format;

/// The opening size of the window and its floor.
const AIRCRAFT_SIZE: [f32; 2] = [920.0, 560.0];
const AIRCRAFT_MIN: [f32; 2] = [560.0, 320.0];

/// One button of the mode column.
struct AirMode {
    mode: Mode,
    label: &'static str,
    hover: &'static str,
}

/// The modes the window offers, in column order.
const MODES: &[AirMode] = &[
    AirMode {
        mode: Mode::Adsb,
        label: "ADS-B",
        hover: "ADS-B, 1090 MHz — aircraft with their position, altitude and speed",
    },
    AirMode {
        mode: Mode::Hfdl,
        label: "HFDL",
        hover: "HFDL, HF data link — aircraft over oceans and poles, from their position reports",
    },
    AirMode {
        mode: Mode::Vdl2,
        label: "VDL2",
        hover: "VDL2, VHF data link — messages from aircraft; few carry a position",
    },
    AirMode {
        mode: Mode::Acars,
        label: "ACARS",
        hover: "ACARS — short messages between aircraft and the ground",
    },
];

/// Where ACARS is put when the dial is nowhere near the airband: a widely used
/// channel. (VDL2 and ADS-B are retuned by the engine itself.)
const ACARS_DEFAULT_HZ: f64 = 131_725_000.0;
const AIRBAND_HZ: std::ops::RangeInclusive<f64> = 118_000_000.0..=137_000_000.0;

/// What a click on a mode button sends.
///
/// **`SetModeListen`, not `SetMode`.** The engine refuses `SetMode` for a mode
/// the current band does not carry, and the band table lists HFDL only on
/// shortwave, VDL2 and ACARS only on the airband, and ADS-B on none of the
/// service bands: on the airband the ADS-B and HFDL buttons did nothing but put
/// a notice in the top bar. These buttons are the aviation tools and have to
/// work wherever the radio is, which is what the listener path is for.
///
/// HFDL's decoder only runs when it is enabled, so the button enables it and
/// brings the dial to its channel. ACARS takes the dial to a channel only when
/// it is not already in the airband, so a frequency the operator chose is never
/// moved; VDL2 and ADS-B are retuned by the engine on the mode change.
fn mode_commands(
    mode: Mode,
    hfdl: &sdroxide_types::HfdlSettings,
    vfo: sdroxide_types::Vfo,
    dial_hz: f64,
) -> Vec<Command> {
    let mut out = vec![Command::SetModeListen { rx: RxId::Main, mode }];
    match mode {
        Mode::Hfdl => {
            if !hfdl.enabled {
                out.push(Command::SetHfdlConfig(sdroxide_types::HfdlSettings {
                    enabled: true,
                    ..*hfdl
                }));
            }
            out.push(Command::SetVfo { vfo, hz: hfdl.frequency_hz });
        }
        Mode::Acars if !AIRBAND_HZ.contains(&dial_hz) => {
            out.push(Command::SetVfo { vfo, hz: ACARS_DEFAULT_HZ });
        }
        _ => {}
    }
    out
}

const COLUMN_W: f32 = 104.0;
const BUTTON_H: f32 = 30.0;
const LIST_W: f32 = 230.0;

const RADIO_GREEN: egui::Color32 = egui::Color32::from_rgb(110, 210, 140);
const ONLINE_BLUE: egui::Color32 = egui::Color32::from_rgb(110, 165, 255);
const HISTORY_VIOLET: egui::Color32 = egui::Color32::from_rgb(190, 150, 255);
/// A watched aircraft is on the air, or the list could not be read.
const WATCH_AMBER: egui::Color32 = egui::Color32::from_rgb(235, 170, 60);

fn chip(ui: &mut egui::Ui, text: &str, color: egui::Color32) {
    ui.label(
        RichText::new(text)
            .size(10.0)
            .strong()
            .color(color)
            .background_color(color.gamma_multiply(0.18)),
    );
}

/// The badge in front of a line: green for the radio, blue for the relay, and
/// nothing for a gap, whose "—" says it.
pub(in crate::app) fn badge_chip(ui: &mut egui::Ui, badge: Badge) {
    match badge_look(badge) {
        Some((text, color)) => chip(ui, text, color),
        None => {
            ui.label("");
        }
    }
}

/// What a badge shows. The radio's is not drawn: the card's header already says
/// where the data comes from as a whole, and a green "radio" on every line
/// repeated it ten times. Only the lines that are *not* from the radio are
/// marked, which makes the mark mean something.
pub(in crate::app) fn badge_look(badge: Badge) -> Option<(&'static str, egui::Color32)> {
    match badge {
        Badge::Radio | Badge::Missing => None,
        Badge::Online => Some(("online", ONLINE_BLUE)),
        Badge::History => Some(("history", HISTORY_VIOLET)),
    }
}

/// The badge at the top of the card: where its data comes from as a whole.
pub(in crate::app) fn provenance_chip(ui: &mut egui::Ui, p: Provenance) {
    match p {
        Provenance::Radio => chip(ui, p.label(), RADIO_GREEN),
        Provenance::RadioOnline | Provenance::Online => chip(ui, p.label(), ONLINE_BLUE),
        Provenance::Nothing => {}
    }
}

/// Lines of a card in four columns: the label, where it comes from, the value
/// and a note.
fn rows_grid(ui: &mut egui::Ui, id: &str, rows: Vec<aircraft_online::Row>) {
    egui::Grid::new(id).num_columns(4).spacing([12.0, 4.0]).show(ui, |ui| {
        for r in rows {
            ui.label(RichText::new(r.label).color(crate::theme::gray(160)));
            badge_chip(ui, r.badge);
            match r.badge {
                Badge::Missing => ui.label(RichText::new(r.value).weak()),
                _ => ui.label(r.value),
            };
            ui.label(RichText::new(r.note).size(10.5).color(crate::theme::gray(130)));
            ui.end_row();
        }
    });
}

const STATS_TABLE_W: f32 = 250.0;
/// A row's label is cut to this; the whole of it is in the hover.
const STATS_LABEL_W: f32 = 190.0;

/// The statistics view: the way to order the tables, where the numbers come
/// from, and one table each with its badge.
fn stats_view(ui: &mut egui::Ui, stats: &Stats, note: &str, sort: &mut SortBy, h: f32) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("{} aircraft", stats.total)).strong());
        ui.add_space(8.0);
        for (s, label) in [(SortBy::Count, "Most first"), (SortBy::Name, "A\u{2013}Z")] {
            if ui.selectable_label(*sort == s, label).clicked() {
                *sort = s;
            }
        }
    });
    ui.label(RichText::new(note).size(11.0).color(crate::theme::gray(160)));
    if stats.total == 0 {
        ui.add_space(6.0);
        ui.label(
            RichText::new(
                "No aircraft yet. The tables fill from ADS-B, VDL2, HFDL and ACARS; nothing \
                 here needs the internet.",
            )
            .size(11.5)
            .color(crate::theme::gray(160)),
        );
        return;
    }
    ui.label(
        RichText::new(
            "Model, operator and route count only the aircraft the relay has answered about \
             \u{2014} open a card in Cards to look one up.",
        )
        .size(10.5)
        .color(crate::theme::gray(130)),
    );
    egui::ScrollArea::vertical().max_height((h - 56.0).max(80.0)).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            for (i, t) in stats.tables.iter().enumerate() {
                ui.allocate_ui_with_layout(
                    egui::vec2(STATS_TABLE_W, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(t.title).strong());
                            badge_chip(ui, t.badge);
                        });
                        ui.label(
                            RichText::new(format!("{} known \u{b7} {} kinds", t.known, t.distinct))
                                .size(10.5)
                                .color(crate::theme::gray(130)),
                        );
                        egui::Grid::new(("aircraft_stats", i)).num_columns(2).show(ui, |ui| {
                            for r in &t.rows {
                                ui.add_sized(
                                    [STATS_LABEL_W, 16.0],
                                    egui::Label::new(&r.label).truncate(),
                                )
                                .on_hover_text(&r.label);
                                ui.label(RichText::new(r.count.to_string()).monospace());
                                ui.end_row();
                            }
                        });
                        ui.add_space(8.0);
                    },
                );
            }
        });
    });
}

/// A mode button is lit only while the map is the thing on show.
fn mode_chip_lit(chip: Mode, shown: Mode, cards_view: bool) -> bool {
    !cards_view && chip == shown
}

impl super::SdroxideApp {
    pub(in crate::app) fn aircraft_window(&mut self, ctx: &egui::Context, cmds: &mut Vec<Command>) {
        if !self.show_aircraft {
            self.aircraft_was_open = false;
            self.aircraft_online.stats.set_on(false);
            return;
        }
        // Each opening gets its own window id, so egui holds no position or
        // size for it and the remembered geometry below is what applies.
        if !self.aircraft_was_open {
            self.aircraft_epoch += 1;
            self.aircraft_was_open = true;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if !self.tool_windows.contains_key("aircraft") {
            self.tool_windows.insert(
                "aircraft",
                super::ToolWindowState {
                    undocked: false,
                    window: self.ui_settings.aircraft_os_window,
                },
            );
        }
        let screen = ctx.content_rect();
        let frame =
            egui::Rect::from_min_max(egui::pos2(screen.left(), self.top_bar_bottom), screen.max);
        let (pos, size) = super::extras::tool_place_at(
            self.ui_settings.aircraft_window,
            frame,
            screen,
            egui::vec2(AIRCRAFT_SIZE[0], AIRCRAFT_SIZE[1]),
            egui::vec2(AIRCRAFT_MIN[0], AIRCRAFT_MIN[1]),
        );
        self.tool_place = Some(super::ToolPlace { pos, size, epoch: self.aircraft_epoch });
        self.tool_rect = None;
        let mut out = Vec::new();
        let show = self.tool_window_sized(
            ctx,
            "aircraft",
            "AIRCRAFT",
            AIRCRAFT_SIZE,
            AIRCRAFT_MIN,
            false,
            true,
            |me, ui| me.aircraft_body(ui, &mut out),
        );
        self.tool_place = None;
        cmds.extend(out);
        self.show_aircraft = show;
        self.remember_tool_geometry(ctx, "aircraft", pos, size);
    }

    fn aircraft_body(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        crate::chrome::window_body_bg(ui);
        // The station's list is fetched only while the statistics are on show.
        self.aircraft_online.stats.set_on(self.aircraft_stats_view);
        // A resizable window shrinks to its content, and the map has no height
        // of its own: hold the size the window was given.
        ui.set_min_size(ui.available_size());
        let active = self.state.rx[0].mode;
        // The mode the map on the right shows: the radio's own when it is one of
        // ours, else the one last picked here, so the window is not blank while
        // the radio is on FT8.
        let shown =
            if MODES.iter().any(|m| m.mode == active) { active } else { self.aircraft_view };
        self.aircraft_view = shown;
        // An export asked for, asked of the station, and saved.
        if let Some(done) = self.aircraft_export_poll(ui.ctx()) {
            crate::download::save(&done.name, &done.bytes);
            self.aircraft_online.export.set_notice(done.note, crate::time::now_unix());
        }
        if let Some(note) = self.aircraft_online.export.notice(crate::time::now_unix()) {
            ui.label(RichText::new(note).size(11.0).color(crate::theme::gray(160)));
        }
        // A watched aircraft came on the air and nobody has looked yet.
        if !self.aircraft_watch_view {
            if let Some(a) = self.aircraft_tracker.alerts().find(|a| !a.seen) {
                let text = aircraft_watch::alert_text(a);
                let more = self.aircraft_tracker.unseen().saturating_sub(1);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("\u{25cf}").color(WATCH_AMBER));
                    ui.label(RichText::new(text).strong());
                    if more > 0 {
                        ui.label(format!("and {more} more"));
                    }
                    if ui.small_button("Show").clicked() {
                        self.aircraft_watch_view = true;
                        self.aircraft_cards = false;
                    }
                });
            }
        }
        let h = ui.available_height().max(120.0);
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(COLUMN_W, h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    // The column is taller than the window's smallest size (four
                    // modes, three views and the export pair at 30 pt each), so it
                    // scrolls rather than cutting the export buttons off.
                    egui::ScrollArea::vertical()
                        .id_salt("aircraft_modes")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            crate::chrome::menu_caption(ui, "Aviation");
                            for m in MODES {
                                let b = crate::chrome::chip_sized(
                                    ui,
                                    mode_chip_lit(
                                        m.mode,
                                        shown,
                                        self.aircraft_cards
                                            || self.aircraft_watch_view
                                            || self.aircraft_stats_view,
                                    ),
                                    m.label,
                                    egui::vec2(COLUMN_W - 8.0, BUTTON_H),
                                )
                                .on_hover_text(m.hover);
                                if b.clicked() {
                                    self.aircraft_view = m.mode;
                                    self.aircraft_cards = false;
                                    self.aircraft_watch_view = false;
                                    self.aircraft_stats_view = false;
                                    cmds.extend(mode_commands(
                                        m.mode,
                                        &self.state.hfdl,
                                        self.state.active_vfo,
                                        self.state.active_freq_hz(),
                                    ));
                                }
                            }
                            ui.add_space(6.0);
                            let n = self.aircraft_registry.len() + self.aircraft_registry.fallback_len();
                            let b = crate::chrome::chip_sized(
                                ui,
                                self.aircraft_cards,
                                format!("Cards {n}"),
                                egui::vec2(COLUMN_W - 8.0, BUTTON_H),
                            )
                            .on_hover_text(
                                "Every aircraft heard on ADS-B, VDL2, HFDL and ACARS, one card each — \
                                 kept for ten minutes, whichever mode the radio is on now",
                            );
                            if b.clicked() {
                                self.aircraft_cards = !self.aircraft_cards;
                                self.aircraft_watch_view = false;
                                self.aircraft_stats_view = false;
                            }
                            let unseen = self.aircraft_tracker.unseen();
                            let n = self.aircraft_online.watch.list().len();
                            let b = crate::chrome::chip_sized(
                                ui,
                                self.aircraft_watch_view,
                                format!("Watch {n}{}", aircraft_watch::chip_suffix(unseen)),
                                egui::vec2(COLUMN_W - 8.0, BUTTON_H),
                            )
                            .on_hover_text(
                                "Aircraft you asked to be told about, by callsign or registration — \
                                 the list is kept by the station, and an alert is shown when one comes \
                                 on the air",
                            );
                            if b.clicked() {
                                self.aircraft_watch_view = !self.aircraft_watch_view;
                                self.aircraft_cards = false;
                            }
                            let b = crate::chrome::chip_sized(
                                ui,
                                self.aircraft_stats_view,
                                "Stats",
                                egui::vec2(COLUMN_W - 8.0, BUTTON_H),
                            )
                            .on_hover_text(
                                "The most common categories, airlines, models, operators and routes \
                                 among the aircraft this station knows — counted from the radio first, \
                                 with the relay's cards as a complement",
                            );
                            if b.clicked() {
                                self.aircraft_stats_view = !self.aircraft_stats_view;
                                self.aircraft_cards = false;
                            }
                            ui.add_space(6.0);
                            crate::chrome::menu_caption(ui, "Export");
                            for (format, hover) in [
                                (
                                    Format::Csv,
                                    "Save the aircraft as a CSV table for a spreadsheet: the station\u{2019}s \
                                     last 24 hours if it keeps a history, else what is heard now. No \
                                     positions.",
                                ),
                                (
                                    Format::Json,
                                    "Save the aircraft as JSON: the station\u{2019}s last 24 hours if it \
                                     keeps a history, else what is heard now. No positions.",
                                ),
                            ] {
                                let label = format.ext().to_ascii_uppercase();
                                let b = crate::chrome::chip_sized(
                                    ui,
                                    self.aircraft_online.export.busy(),
                                    label,
                                    egui::vec2(COLUMN_W - 8.0, BUTTON_H),
                                )
                                .on_hover_text(hover);
                                if b.clicked() {
                                    self.aircraft_online.export.start(format);
                                }
                            }
                        });
                },
            );
            ui.vertical(|ui| {
                ui.set_min_width(ui.available_width());
                if self.aircraft_watch_view {
                    self.aircraft_watch_pane(ui, h);
                } else if self.aircraft_stats_view {
                    self.aircraft_stats_pane(ui, h);
                } else if self.aircraft_cards {
                    self.aircraft_cards_pane(ui, h);
                } else {
                    self.aircraft_map(ui, cmds, shown, active, h);
                }
            });
        });
    }

    /// Take what has arrived, ask the station for its table if an export was
    /// pressed, and give back the file to save once it is ready. The saving is the
    /// caller's, so that this can be run without a file dialog.
    pub(in crate::app) fn aircraft_export_poll(
        &mut self,
        ctx: &egui::Context,
    ) -> Option<aircraft_export::Done> {
        let now = crate::time::now_unix();
        let peer = self.ctrl.peer_url();
        self.aircraft_online.poll(ctx, now);
        self.aircraft_online.ask_export(ctx, peer.as_deref(), now);
        let ready = self.aircraft_online.export.take_ready()?;
        Some(aircraft_export::finish(
            ready,
            || {
                aircraft_export::local_rows(
                    self.aircraft_registry.entries(),
                    self.aircraft_online.store.infos(),
                )
            },
            now,
        ))
    }

    /// Once a frame, whether the window is open or not: read the watch list from
    /// the station, send what the person asked to change, and look at the cards
    /// for a watched aircraft that has just come on the air.
    pub(in crate::app) fn aircraft_watch_tick(&mut self, ctx: &egui::Context) {
        let now = crate::time::now_unix();
        let peer = self.ctrl.peer_url();
        self.aircraft_online.poll(ctx, now);
        self.aircraft_online.ask_watch(ctx, peer.as_deref(), now);
        self.aircraft_online.ask_switch(ctx, peer.as_deref());
        // Until the station has said what the list is there is nothing to watch.
        if !matches!(self.aircraft_online.watch.state(), WatchState::Ready) {
            return;
        }
        let list = self.aircraft_online.watch.list();
        if !self.aircraft_tracker.due(now) {
            return;
        }
        let here = if list.is_empty() {
            Vec::new()
        } else {
            aircraft_watch::presences(self.aircraft_registry.entries())
        };
        self.aircraft_tracker.observe(now, list, &here);
    }

    /// The watch list: a box to add to it, the entries with a way to take each
    /// off and a mark for the ones on the air now, and the alerts so far.
    fn aircraft_watch_pane(&mut self, ui: &mut egui::Ui, h: f32) {
        let now = crate::time::now_unix();
        let has_station = self.ctrl.peer_url().is_some();
        ui.label(
            RichText::new(
                "Callsigns and registrations you want to be told about. The list is kept by \
                 the station, so it is the same from every screen; an alert comes when one \
                 appears, or comes back after ten minutes off the air.",
            )
            .size(11.0)
            .color(crate::theme::gray(160)),
        );
        if !has_station {
            ui.add_space(6.0);
            ui.label(
                RichText::new(
                    "The watch list is kept by the station you are connected to. Connect to \
                     one to use it.",
                )
                .size(11.5)
                .color(crate::theme::gray(160)),
            );
            return;
        }
        match self.aircraft_online.watch.state() {
            WatchState::Idle | WatchState::Loading => {
                ui.label(RichText::new("Reading the list\u{2026}").size(11.0).weak());
            }
            WatchState::Unavailable(r) => {
                ui.label(
                    RichText::new(format!("Cannot read the list: {}.", r.note()))
                        .size(11.0)
                        .color(WATCH_AMBER),
                );
            }
            WatchState::Ready => {}
        }
        ui.add_space(4.0);
        let mut edit = None;
        ui.horizontal(|ui| {
            let r = ui.add(
                egui::TextEdit::singleline(&mut self.aircraft_watch_input)
                    .desired_width(160.0)
                    .hint_text("KLM1234 or PH-BXA"),
            );
            let go = ui.button("Add").clicked()
                || (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
            if go {
                if let Some(e) = aircraft_watch::add_edit(&self.aircraft_watch_input) {
                    edit = Some(e);
                    self.aircraft_watch_input.clear();
                }
            }
        });
        if self.aircraft_online.watch.queued() > 0 {
            ui.label(RichText::new("Sending\u{2026}").size(11.0).weak());
        }
        if let Some(note) =
            self.aircraft_online.watch.last_outcome().and_then(aircraft_watch::outcome_note)
        {
            ui.label(RichText::new(note).size(11.0).color(WATCH_AMBER));
        }
        let entries: Vec<String> = self.aircraft_online.watch.list().entries().to_vec();
        egui::ScrollArea::vertical().max_height((h - 60.0).max(80.0)).show(ui, |ui| {
            if entries.is_empty() {
                ui.label(
                    RichText::new("Nothing on the list yet.")
                        .size(11.5)
                        .color(crate::theme::gray(160)),
                );
            }
            egui::Grid::new("aircraft_watch_entries").num_columns(3).spacing([12.0, 4.0]).show(
                ui,
                |ui| {
                    for e in &entries {
                        ui.label(RichText::new(e).monospace());
                        if self.aircraft_tracker.is_on_air(e) {
                            ui.label(RichText::new("on the air now").size(10.5).color(RADIO_GREEN));
                        } else {
                            ui.label("");
                        }
                        if ui.small_button("Remove").clicked() {
                            edit = Some(aircraft_watch::remove_edit(e));
                        }
                        ui.end_row();
                    }
                },
            );
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Alerts").strong());
                if ui.small_button("Clear").clicked() {
                    self.aircraft_tracker.clear();
                }
            });
            if self.aircraft_tracker.alerts().next().is_none() {
                ui.label(RichText::new("None yet.").size(11.5).color(crate::theme::gray(160)));
            }
            for a in self.aircraft_tracker.alerts() {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(super::util::fmt_age(now - a.at))
                            .size(10.5)
                            .color(crate::theme::gray(130)),
                    );
                    let t = RichText::new(aircraft_watch::alert_text(a));
                    ui.label(if a.seen { t } else { t.strong() });
                });
            }
        });
        if let Some(e) = edit {
            self.aircraft_online.watch.request_edit(e);
        }
        // Looking at them is what makes them seen.
        self.aircraft_tracker.mark_seen();
    }

    /// The statistics: the live registry and the relay's cards, with the
    /// station's list of the last day when it keeps one. It asks for that list
    /// while this view is open, and for nothing else.
    fn aircraft_stats_pane(&mut self, ui: &mut egui::Ui, h: f32) {
        let now = crate::time::now_unix();
        let ctx = ui.ctx().clone();
        let peer = self.ctrl.peer_url();
        self.aircraft_online.poll(&ctx, now);
        self.aircraft_online.ask_stats(&ctx, peer.as_deref(), now);
        let live = aircraft_stats::facts_from_live(
            self.aircraft_registry.entries(),
            self.aircraft_online.store.infos(),
        );
        let kept = match self.aircraft_online.stats.state() {
            aircraft_stats::StatsState::Ready(list) => aircraft_stats::facts_from_summaries(list),
            _ => Vec::new(),
        };
        let stats = aircraft_stats::compute(
            &aircraft_stats::merge(live, kept),
            self.aircraft_stats_sort,
            aircraft_stats::TOP,
        );
        let note = aircraft_stats::source_note(self.aircraft_online.stats.state(), peer.is_some());
        stats_view(ui, &stats, &note, &mut self.aircraft_stats_sort, h);
    }

    /// The list of aircraft on the left, the open card on the right.
    fn aircraft_cards_pane(&mut self, ui: &mut egui::Ui, h: f32) {
        let now = crate::time::now_unix();
        let rows = aircraft_card::rows(&self.aircraft_registry, now);
        if rows.is_empty() {
            ui.add_space(6.0);
            ui.label(
                RichText::new(
                    "No aircraft heard yet. Cards fill from ADS-B, VDL2, HFDL and ACARS — \
                     press one of them and listen. Nothing here needs the internet.",
                )
                .size(11.5)
                .color(crate::theme::gray(160)),
            );
            return;
        }
        // A card that expired while open closes rather than showing stale text.
        if let Some(sel) = &self.aircraft_card_sel {
            if aircraft_card::lookup(&self.aircraft_registry, sel).is_none() {
                self.aircraft_card_sel = None;
            }
        }
        // The search box narrows the list from what is in memory; it asks for
        // nothing. See [`aircraft_search`].
        let query = aircraft_search::Query::parse(&self.aircraft_query);
        let found = aircraft_search::search_rows(
            &query,
            rows,
            &self.aircraft_registry,
            &self.aircraft_online.store,
            self.ctrl.peer_url().is_some(),
        );
        let counts = (!query.is_blank()).then_some((found.rows.len(), found.total));
        aircraft_search::search_box(
            ui,
            &mut self.aircraft_query,
            200.0,
            counts,
            found.hint.as_deref(),
        );
        let h = (h - 26.0 - if found.hint.is_some() { 16.0 } else { 0.0 }).max(80.0);
        let rows = found.rows;
        // The card open is the one chosen while it matches; when the box leaves
        // it out, the first match is shown instead and the choice is kept for
        // when the box lets it back.
        let Some(open) =
            aircraft_search::open_card(self.aircraft_card_sel.as_ref(), &rows, !query.is_blank())
        else {
            ui.add_space(6.0);
            ui.label(
                RichText::new(format!(
                    "No aircraft match \u{201c}{}\u{201d}.",
                    self.aircraft_query.trim()
                ))
                .size(11.5)
                .color(crate::theme::gray(160)),
            );
            return;
        };
        let sel = open.key.clone();
        // The relay's half: poll what arrived, ask what is due, and start the
        // thumbnail. All of it from the card that is open, and nothing else.
        let ctx = ui.ctx().clone();
        let peer = self.ctrl.peer_url();
        let want = aircraft_card::lookup(&self.aircraft_registry, &sel)
            .and_then(|c| aircraft_online::want_for(&sel, c));
        self.aircraft_online.poll(&ctx, now);
        // The station's own record of the aircraft whose history is on, fetched
        // again every 30 s while it is; nothing is asked unless it is on.
        self.aircraft_online.ask_history(&ctx, peer.as_deref(), now);
        // A card shown only because the box left it first was not chosen: it is
        // not asked about, and its photo is not fetched.
        self.aircraft_online.ask(
            &ctx,
            peer.as_deref(),
            want.as_ref().filter(|_| !open.by_filter),
            now,
        );
        let state = match &want {
            Some(w) => self.aircraft_online.store.state_for(w.icao, peer.is_some()).clone(),
            None => aircraft_online::OnlineState::Idle,
        };
        let mut photo = aircraft_online::photo_view(&state, ThumbStatus::NotTried);
        if open.by_filter {
            if let Some(p) = photo.as_mut() {
                p.show_image = false;
            }
        }
        if let Some(p) = photo.as_ref().filter(|p| p.show_image) {
            self.aircraft_online.want_thumb(&ctx, &p.thumb_url);
            // Now that it has been asked for, what became of it decides whether
            // the picture is drawn or only its credit and link.
            let status = self.aircraft_online.thumb_status(&p.thumb_url);
            photo = aircraft_online::photo_view(&state, status);
        }
        let mut picked = None;
        let mut history_pressed = None;
        let has_station = peer.is_some();
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(LIST_W, h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    egui::ScrollArea::vertical().id_salt("aircraft_cards_list").show(ui, |ui| {
                        for r in &rows {
                            let text = format!(
                                "{}   {}\n{}",
                                r.title,
                                super::util::fmt_age(r.age_s),
                                r.detail
                            );
                            let on = r.key == sel;
                            if ui.selectable_label(on, text).clicked() {
                                picked = Some(r.key.clone());
                            }
                        }
                    });
                },
            );
            ui.separator();
            ui.vertical(|ui| {
                if let Some(card) = aircraft_card::lookup(&self.aircraft_registry, &sel) {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(aircraft_card::title(&sel, card)).strong().size(15.0),
                        );
                        provenance_chip(ui, aircraft_online::provenance(card, &state));
                    });
                    ui.label(
                        RichText::new(aircraft_card::detail(&sel, card))
                            .size(11.0)
                            .color(crate::theme::gray(160)),
                    );
                    // The station's record of this aircraft, on a button: it is
                    // asked for only once this is pressed.
                    let icao = match &sel {
                        CardKey::Icao(i) => Some(*i & 0xff_ffff),
                        CardKey::Fallback(_) => None,
                    };
                    let offer = aircraft_history::offer(
                        self.aircraft_online.history.on(),
                        icao,
                        has_station,
                    );
                    if offer != HistoryOffer::Hidden {
                        ui.add_space(4.0);
                        let b = ui.small_button(offer.label()).on_hover_text(
                            "Show where this aircraft has been in the last day, from the \
                             station's own record. It asks your station, never the internet.",
                        );
                        if b.clicked() {
                            history_pressed = icao;
                        }
                    }
                    let card_rows = aircraft_online::rows(card, &state, now);
                    ui.add_space(4.0);
                    if ui
                        .small_button("Copy summary")
                        .on_hover_text(
                            "Copy this card as text: what is known, and where each line came from. \
                             The position and the last message are left out.",
                        )
                        .clicked()
                    {
                        let text = aircraft_share::summary_text(
                            &aircraft_card::title(&sel, card),
                            &aircraft_card::detail(&sel, card),
                            &card_rows,
                            photo.as_ref(),
                        );
                        ui.ctx().copy_text(text);
                    }
                    ui.add_space(6.0);
                    rows_grid(ui, "aircraft_card_fields", card_rows);
                    self.online_switch_row(ui, &state);
                    if let Some(i) = icao.filter(|i| self.aircraft_online.history.on() == Some(*i))
                    {
                        ui.add_space(6.0);
                        match self.aircraft_online.history.state(i) {
                            HistoryState::Ready(j) => {
                                rows_grid(
                                    ui,
                                    "aircraft_card_history",
                                    aircraft_history::rows(j, now),
                                );
                            }
                            HistoryState::Unavailable(r) => {
                                ui.label(RichText::new(r.note()).size(10.5).weak());
                            }
                            HistoryState::Idle | HistoryState::Loading => {
                                ui.label(RichText::new("asking the station…").size(10.5).weak());
                            }
                        }
                    }
                    if let Some(p) = &photo {
                        ui.add_space(8.0);
                        self.photo_block(ui, p);
                    }
                }
            });
        });
        if let Some(i) = history_pressed {
            self.aircraft_online.history.toggle(i);
        }
        if picked.is_some() {
            self.aircraft_card_sel = picked;
        }
    }

    /// The switch for the station's online lookup, drawn under the card's lines.
    ///
    /// Off is the default, because it makes the station reach out to adsbdb and
    /// Planespotters; and an operator cannot be expected to set an environment
    /// variable on the station, so the window offers the switch itself. The
    /// station keeps the choice across restarts.
    fn online_switch_row(&mut self, ui: &mut egui::Ui, state: &aircraft_online::OnlineState) {
        use aircraft_online::{OnlineState, Reason};
        let on = match state {
            OnlineState::Unavailable(Reason::Disabled) => false,
            OnlineState::Ready(_) => true,
            _ => return,
        };
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if self.aircraft_online.switch_asking {
                ui.label(RichText::new("asking the station…").size(10.5).weak());
            } else if !on {
                let b = ui.button("Turn on online lookup").on_hover_text(
                    "Photos, type, operator and route come from adsbdb and Planespotters. \
                     Your station asks them for you, and only when this is on; it keeps \
                     your choice. Nothing else leaves the station.",
                );
                if b.clicked() {
                    self.aircraft_online.want_switch(true);
                }
            } else if ui
                .small_button("Turn off online lookup")
                .on_hover_text("Stop the station asking adsbdb and Planespotters.")
                .clicked()
            {
                self.aircraft_online.want_switch(false);
            }
            if let Some(why) = self.aircraft_online.switch_note {
                ui.label(
                    RichText::new(format!("Not changed: {why}.")).size(10.5).color(WATCH_AMBER),
                );
            }
        });
    }

    /// The photo, its credit and the link to its page. The credit and the link
    /// are there whether or not the picture could be loaded.
    fn photo_block(&self, ui: &mut egui::Ui, p: &aircraft_online::PhotoView) {
        if p.show_image {
            match self.aircraft_online.thumb_texture(&p.thumb_url) {
                Some(tex) => {
                    ui.add(egui::Image::new(tex).max_height(140.0).corner_radius(3.0));
                }
                None => {
                    ui.label(RichText::new("loading photo…").size(10.5).weak());
                }
            }
        }
        ui.horizontal(|ui| {
            ui.label(RichText::new(&p.credit).size(10.5).color(crate::theme::gray(150)));
            if let Some(url) = &p.page_url {
                if ui.link(RichText::new("Planespotters").size(10.5)).on_hover_text(url).clicked()
                    && !crate::download::open_external(url)
                {
                    tracing::warn!(%url, "no handler for this link");
                }
            }
        });
    }

    /// The right-hand side: a line saying what the radio is doing, then the map.
    fn aircraft_map(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        shown: Mode,
        active: Mode,
        h: f32,
    ) {
        let now = crate::time::now_unix();
        let label = MODES.iter().find(|m| m.mode == shown).map_or("", |m| m.label);
        if shown != active {
            ui.label(
                RichText::new(format!(
                    "The radio is on {} — press {label} to listen. This is what {label} heard \
                     before.",
                    active.label()
                ))
                .size(11.0)
                .color(crate::theme::YELLOW()),
            );
        }
        let map_h = (h - 24.0).max(100.0);
        match shown {
            Mode::Adsb => match self.adsb_status.as_deref().cloned() {
                Some(st) => self.adsb_map_pane(ui, cmds, &st, now, map_h),
                None => {
                    ui.label(RichText::new("No ADS-B heard yet.").weak());
                }
            },
            Mode::Hfdl => {
                if !self.state.hfdl.enabled {
                    ui.label(
                        RichText::new("The HFDL decoder is off — press HFDL to switch it on.")
                            .size(11.0)
                            .color(crate::theme::YELLOW()),
                    );
                }
                self.hfdl_map_pane(ui, now, map_h)
            }
            // VDL2 and ACARS carry a position only now and then, so what the
            // window shows for them is what they actually hear: the messages and
            // the stations (VDL2), the message log (ACARS). An aircraft that is
            // also heard on ADS-B shows on the ADS-B map.
            Mode::Vdl2 => match self.vdl2_status.as_deref().cloned() {
                Some(st) => self.vdl2_columns(ui, &st, now, map_h),
                None => {
                    ui.label(RichText::new("Starting the VDL2 decoder…").weak());
                }
            },
            Mode::Acars => self.acars_panel(ui, cmds, map_h),
            _ => {
                ui.label(RichText::new(format!("{label} has no view here.")).weak());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn only_lines_that_are_not_from_the_radio_wear_a_badge() {
        assert!(super::badge_look(Badge::Radio).is_none());
        assert!(super::badge_look(Badge::Missing).is_none());
        assert_eq!(super::badge_look(Badge::Online).map(|b| b.0), Some("online"));
        assert_eq!(super::badge_look(Badge::History).map(|b| b.0), Some("history"));
    }

    use super::*;

    /// At the window's smallest size (560 x 320) the column of buttons is taller
    /// than the window, and the export buttons were cut off with no way to them.
    #[test]
    fn the_button_column_scrolls_so_a_small_window_does_not_cut_it_off() {
        let src = include_str!("aircraft.rs");
        let body = &src[..src.find("#[cfg(test)]").unwrap()];
        assert!(body.contains("id_salt(\"aircraft_modes\")"));
    }

    /// The operator's screenshot: VDL2 selected and the right-hand side empty bar
    /// a sentence. Every mode button now has a view that shows what it hears.
    #[test]
    fn no_aviation_button_is_left_with_an_empty_map() {
        let src = include_str!("aircraft.rs");
        let body = &src[..src.find("#[cfg(test)]").unwrap()];
        assert!(!body.contains("has no map of its own"), "the placeholder is back");
        for arm in ["Mode::Adsb =>", "Mode::Hfdl =>", "Mode::Vdl2 =>", "Mode::Acars =>"] {
            assert!(body.contains(arm), "aircraft_map has no view for {arm}");
        }
    }

    #[test]
    fn an_aviation_button_never_sends_the_band_checked_set_mode() {
        let hfdl = sdroxide_types::HfdlSettings::default();
        for m in MODES {
            let cmds = mode_commands(m.mode, &hfdl, sdroxide_types::Vfo::A, 7_000_000.0);
            assert!(
                matches!(cmds[0], Command::SetModeListen { mode, .. } if mode == m.mode),
                "{:?}: {cmds:?}",
                m.mode
            );
            assert!(!cmds.iter().any(|c| matches!(c, Command::SetMode { .. })), "{cmds:?}");
        }
    }

    #[test]
    fn the_hfdl_button_switches_the_decoder_on_and_goes_to_its_channel() {
        let off = sdroxide_types::HfdlSettings { enabled: false, frequency_hz: 8_912_000.0 };
        let cmds = mode_commands(Mode::Hfdl, &off, sdroxide_types::Vfo::A, 145_000_000.0);
        assert!(
            cmds.iter().any(|c| matches!(c, Command::SetHfdlConfig(h) if h.enabled)),
            "{cmds:?}"
        );
        assert!(cmds.iter().any(|c| matches!(c, Command::SetVfo { hz, .. } if *hz == 8_912_000.0)));
        let on = sdroxide_types::HfdlSettings { enabled: true, ..off };
        let cmds = mode_commands(Mode::Hfdl, &on, sdroxide_types::Vfo::A, 8_912_000.0);
        assert!(!cmds.iter().any(|c| matches!(c, Command::SetHfdlConfig(_))));
    }

    #[test]
    fn acars_leaves_a_dial_that_is_already_in_the_airband() {
        let hfdl = sdroxide_types::HfdlSettings::default();
        let moved = mode_commands(Mode::Acars, &hfdl, sdroxide_types::Vfo::A, 7_000_000.0);
        assert!(
            moved
                .iter()
                .any(|c| matches!(c, Command::SetVfo { hz, .. } if *hz == ACARS_DEFAULT_HZ))
        );
        let kept = mode_commands(Mode::Acars, &hfdl, sdroxide_types::Vfo::A, 131_550_000.0);
        assert_eq!(kept.len(), 1, "{kept:?}");
    }

    #[test]
    fn every_button_is_a_mode_the_band_menu_can_set() {
        for m in MODES {
            assert!(
                crate::app::top_bar::WIDEBAND_MODES.contains(&m.mode)
                    || Mode::DIGITAL.contains(&m.mode)
                    || m.mode == Mode::Acars,
                "{} is not a mode the radio switches to",
                m.label
            );
        }
    }

    #[test]
    fn a_mode_button_is_lit_only_while_the_map_is_shown() {
        assert!(mode_chip_lit(Mode::Adsb, Mode::Adsb, false));
        assert!(!mode_chip_lit(Mode::Hfdl, Mode::Adsb, false));
        assert!(!mode_chip_lit(Mode::Adsb, Mode::Adsb, true), "the Cards view lights Cards alone");
    }

    #[test]
    fn the_column_labels_fit_their_buttons() {
        for m in MODES {
            assert!(m.label.len() <= 6, "{} is too long for a {COLUMN_W} pt column", m.label);
        }
    }

    // ── the statistics view ───────────────────────────────────────────────

    fn stats_of(n: u32) -> Stats {
        let facts: Vec<aircraft_stats::Fact> = (0..n)
            .map(|i| aircraft_stats::Fact {
                icao: i,
                callsign: Some(format!("KLM{i}")),
                category: Some(format!("cat{}", i % 5)),
                type_name: Some(format!("Model {}", i % 12)),
                operator: Some("A rather long operator name indeed".repeat(3)),
                origin: Some("AMS".into()),
                destination: Some(format!("D{}", i % 9)),
            })
            .collect();
        aircraft_stats::compute(&facts, SortBy::Count, aircraft_stats::TOP)
    }

    #[test]
    fn the_statistics_draw_inside_the_pane_with_and_without_aircraft() {
        for n in [0u32, 1, 4000] {
            let stats = stats_of(n);
            let ctx = egui::Context::default();
            let mut sort = SortBy::Count;
            let mut rect = egui::Rect::NOTHING;
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.set_max_width(700.0);
                let before = ui.cursor().min;
                stats_view(ui, &stats, "a note", &mut sort, 400.0);
                rect = egui::Rect::from_min_max(before, ui.min_rect().max);
            });
            out.textures_delta.clear();
            assert!(rect.width() > 40.0 && rect.height() > 10.0, "{n}: nothing drawn {rect:?}");
            assert!(rect.width() <= 701.0, "{n}: too wide {rect:?}");
            assert!(rect.height() <= 401.0, "{n}: past the pane {rect:?}");
        }
    }

    #[test]
    fn the_stats_button_is_one_view_among_three_and_the_store_follows_it() {
        let src = include_str!("aircraft.rs");
        let code = &src[..src.find("#[cfg(test)]").unwrap()];
        assert_eq!(code.matches("aircraft_stats_pane(ui, h)").count(), 1);
        assert_eq!(code.matches("ask_stats(").count(), 1, "asked from the pane only");
        assert_eq!(code.matches("stats.set_on(").count(), 2, "the body and the closed window");
    }
}
