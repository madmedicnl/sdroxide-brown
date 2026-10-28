//! **Radio Classic**: a classic-radio face for broadcast listening, with the
//! FM radio (WFM + RDS) as the first thing it does well.
//!
//! One framed panel in three columns of equal height. Docked, it takes the slot
//! under the waterfall that a digital mode's panel takes (and CW's), at the same
//! 46 % of the column; detached, it is an ordinary tool window. The colours all
//! come from the theme in force.
//!
//! | column | content |
//! | --- | --- |
//! | left | bands, modes, MEM and SCHEDULE |
//! | middle | the mode, a mini spectrum, the S-meter, BW/STEP/AGC/SQL, the VU |
//! | right | clocks, the VFO, the station name, RadioText, RDS chips, audio bars |
//!
//! The plan and the decisions are in `docs/internal/RADIO-CLASSIC-PLAN.md`.

use std::collections::VecDeque;

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, RichText, Stroke, StrokeKind, vec2};
use sdroxide_types::{
    AUDIO_BANDS, Band, Command, Mode, RdsData, RxId, broadcast, byte_to_unit, pty_name,
};

use crate::app::SdroxideApp;
use crate::theme;
use crate::widgets::vu_meter::{self, VuBallistics};

/// Where the panel is drawn this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::app) enum RadioClassicPlace {
    /// Not in use.
    Hidden,
    /// Under the waterfall, in the slot a mode panel would take.
    Docked,
    /// In a window of its own.
    Window,
}

/// Where Radio Classic goes: a pure rule, so it can be pinned without a frame.
///
/// * `on` — switched on from EXTRAS; `detached` — the Detachable page's switch.
/// * A detached panel is a window, whatever else is true.
/// * Switched on, it docks in the modes that have nothing under the waterfall.
///   Where the slot is taken — a digital mode's panel, DAB, CW — or on a phone,
///   where there is no room for it, it is a window instead, so that switching it
///   on never does nothing.
pub(in crate::app) fn place(
    on: bool,
    detached: bool,
    mode: Mode,
    phone: bool,
) -> RadioClassicPlace {
    if detached {
        RadioClassicPlace::Window
    } else if !on {
        RadioClassicPlace::Hidden
    } else if phone || mode.has_bottom_panel() || mode == Mode::Cw {
        RadioClassicPlace::Window
    } else {
        RadioClassicPlace::Docked
    }
}

// ───────────────────────────────────────────────────────────────── the bands

/// The groups of the left column's first row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::app) enum Group {
    #[default]
    Broadcast,
    Ham,
    Air,
    Marine,
    Cb,
}

impl Group {
    const ALL: [(Group, &'static str); 5] = [
        (Group::Broadcast, "BROADCAST"),
        (Group::Ham, "HAM"),
        (Group::Air, "AIR"),
        (Group::Marine, "MARINE"),
        (Group::Cb, "CB"),
    ];
}

/// One button of the band rows: where it tunes and when it reads as selected.
#[derive(Clone, Debug, PartialEq)]
struct Preset {
    label: String,
    /// The range the button stands for: it is lit while the dial is inside.
    lo_hz: f64,
    hi_hz: f64,
    /// Where a click goes, and in which mode.
    tune_hz: f64,
    mode: Mode,
    /// A band the engine already knows (its own band stack and rules), when
    /// there is one; the click asks for that rather than for a bare frequency.
    band: Option<Band>,
}

impl Preset {
    fn range(label: &str, lo_mhz: f64, hi_mhz: f64, tune_mhz: f64, mode: Mode) -> Self {
        Self {
            label: label.into(),
            lo_hz: lo_mhz * 1e6,
            hi_hz: hi_mhz * 1e6,
            tune_hz: tune_mhz * 1e6,
            mode,
            band: None,
        }
    }

    fn channel(label: &str, mhz: f64, mode: Mode) -> Self {
        Self::range(label, mhz - 0.0025, mhz + 0.0025, mhz, mode)
    }

    fn band(label: &str, band: Band, lo_mhz: f64, hi_mhz: f64, mode: Mode) -> Self {
        Self {
            band: Some(band),
            ..Self::range(label, lo_mhz, hi_mhz, (lo_mhz + hi_mhz) / 2.0, mode)
        }
    }

    fn holds(&self, hz: f64) -> bool {
        (self.lo_hz..=self.hi_hz).contains(&hz)
    }
}

/// The longwave, mediumwave and FM row of the broadcast group.
fn broadcast_row() -> Vec<Preset> {
    vec![
        Preset::band("LW", Band::Lw, 0.1485, 0.2835, Mode::Am),
        Preset::range("MW EU", 0.531, 1.602, 0.999, Mode::Am),
        Preset::range("MW US", 0.530, 1.700, 1.010, Mode::Am),
        Preset::range("FM 87.5–108", 87.5, 108.0, 100.0, Mode::Wfm),
        Preset::range("FM ext 64–87.5", 64.0, 87.5, 76.0, Mode::Wfm),
    ]
}

/// The shortwave broadcast bands, `120 … 11`.
fn shortwave_row() -> Vec<Preset> {
    broadcast::METRE_BANDS
        .iter()
        .map(|&(name, lo_khz, hi_khz)| {
            let mid_khz = ((lo_khz + hi_khz) / 2.0 / 5.0).round() * 5.0;
            Preset::range(
                name.trim_end_matches('m'),
                lo_khz / 1e3,
                hi_khz / 1e3,
                mid_khz / 1e3,
                Mode::Am,
            )
        })
        .collect()
}

/// The rows a group offers, as `(caption, buttons)`.
fn group_rows(group: Group) -> Vec<(&'static str, Vec<Preset>)> {
    match group {
        Group::Broadcast => {
            vec![("OL / OM · FM", broadcast_row()), ("OC · SHORTWAVE", shortwave_row())]
        }
        Group::Ham => vec![(
            "AMATEUR BANDS",
            [
                (Band::M160, 1.8, 2.0, Mode::Lsb),
                (Band::M80, 3.5, 3.8, Mode::Lsb),
                (Band::M60, 5.30, 5.41, Mode::Usb),
                (Band::M40, 7.0, 7.2, Mode::Lsb),
                (Band::M30, 10.1, 10.15, Mode::Cw),
                (Band::M20, 14.0, 14.35, Mode::Usb),
                (Band::M17, 18.068, 18.168, Mode::Usb),
                (Band::M15, 21.0, 21.45, Mode::Usb),
                (Band::M12, 24.89, 24.99, Mode::Usb),
                (Band::M10, 28.0, 29.7, Mode::Usb),
                (Band::M6, 50.0, 54.0, Mode::Usb),
                (Band::M2, 144.0, 146.0, Mode::Nfm),
                (Band::M70, 430.0, 440.0, Mode::Nfm),
            ]
            .into_iter()
            .map(|(b, lo, hi, m)| Preset::band(b.label(), b, lo, hi, m))
            .collect(),
        )],
        Group::Air => vec![(
            "AIRBAND",
            vec![
                Preset::band("AIR 118–137", Band::Air, 118.0, 137.0, Mode::Am),
                Preset::band("MIL 225–400", Band::Mil, 225.0, 400.0, Mode::Am),
            ],
        )],
        Group::Marine => vec![(
            "MARINE VHF · MF",
            vec![
                Preset::channel("16", 156.800, Mode::Nfm),
                Preset::channel("9", 156.450, Mode::Nfm),
                Preset::channel("6", 156.300, Mode::Nfm),
                Preset::channel("12", 156.600, Mode::Nfm),
                Preset::channel("13", 156.650, Mode::Nfm),
                Preset::channel("14", 156.700, Mode::Nfm),
                Preset::channel("70 DSC", 156.525, Mode::Nfm),
                Preset::channel("2182 kHz", 2.182, Mode::Usb),
            ],
        )],
        Group::Cb => vec![(
            "CITIZENS BAND",
            vec![
                Preset::band("11 M", Band::M11, 26.965, 27.405, Mode::Am),
                Preset::channel("CH 9", 27.065, Mode::Am),
                Preset::channel("CH 19", 27.185, Mode::Am),
            ],
        )],
    }
}

/// The modes the left column offers.
const MODES: [(&str, Mode); 6] = [
    ("WFM", Mode::Wfm),
    ("FM", Mode::Nfm),
    ("AM", Mode::Am),
    ("USB", Mode::Usb),
    ("LSB", Mode::Lsb),
    ("CW", Mode::Cw),
];

// ──────────────────────────────────────────────────────────── what is on show

/// The name the big line shows: the station's own on FM, the memory's or the
/// schedule's everywhere else (and on FM too while the station sends no name).
fn station_name(
    mode: Mode,
    ps: Option<&str>,
    memory: Option<&str>,
    schedule: Option<&str>,
) -> Option<String> {
    let ps = ps.map(str::trim).filter(|s| !s.is_empty());
    let own = if mode == Mode::Wfm { ps } else { None };
    own.or(memory).or(schedule).map(str::to_string)
}

/// The width of the three columns for a panel `w` wide, with the gap between
/// them: the left one a quarter, the middle (mini spectrum and VU) a quarter,
/// the right one the rest — and the middle never narrower than the VU plus a
/// usable spectrum.
fn column_widths(w: f32) -> (f32, f32, f32) {
    const GAP: f32 = 8.0;
    let usable = (w - 2.0 * GAP).max(300.0);
    let left = (usable * 0.265).max(150.0);
    let mid = (usable * 0.25).max(200.0);
    let right = (usable - left - mid).max(120.0);
    (left, mid, right)
}

/// The S-meter: how full it reads, `0.0..=1.0`, for a level in dBm. S0 is
/// -127 dBm and +60 dB over S9 (-13 dBm) is the top, as on the scale below it.
fn smeter_fill(s_dbm: f32) -> f32 {
    ((s_dbm + 127.0) / 114.0).clamp(0.0, 1.0)
}

/// `S6`, `S9+20` — the reading a listener says aloud.
fn s_reading(s_dbm: f32) -> String {
    let over = s_dbm + 73.0;
    if over >= 0.0 {
        if over < 3.0 { "S9".into() } else { format!("S9+{}", (over / 5.0).round() as i32 * 5) }
    } else {
        format!("S{}", (9.0 + over / 6.0).max(0.0) as u8)
    }
}

/// How long ago a reading may be and still count as the sound that is playing.
const LEVEL_FRESH_S: f32 = 0.4;
/// Rows kept by the mini waterfall, and the bins in each.
const WF_ROWS: usize = 40;
const WF_BINS: usize = 96;

/// The panel's own state: what has to live between frames.
pub(in crate::app) struct State {
    pub group: Group,
    pub vu: VuBallistics,
    vu_at: Option<f64>,
    bars: Vec<f32>,
    peaks: Vec<f32>,
    wf: VecDeque<Vec<u8>>,
    wf_seq: Option<u32>,
    pub schedule_open: bool,
    /// When `WantAudioLevel` was last sent (wall clock), so it is renewed about
    /// once a second while the panel is on screen.
    lease_at: f64,
    /// The spectrum frame, handed over by the docked layout: it holds the frame
    /// out of `SdroxideApp::frame` while it draws, so the panel under the
    /// waterfall would otherwise find nothing there.
    pub frame: Option<std::sync::Arc<sdroxide_types::SpectrumFrame>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            group: Group::default(),
            vu: VuBallistics::default(),
            vu_at: None,
            bars: vec![0.0; AUDIO_BANDS],
            peaks: vec![0.0; AUDIO_BANDS],
            wf: VecDeque::new(),
            wf_seq: None,
            schedule_open: false,
            lease_at: 0.0,
            frame: None,
        }
    }
}

/// The ink the panel uses, read from the theme once a frame.
struct Ink {
    ground: Color32,
    line: Color32,
    text: Color32,
    strong: Color32,
    dim: Color32,
    accent: Color32,
    amber: Color32,
    frame: Color32,
    green: Color32,
    red: Color32,
}

impl Ink {
    fn now() -> Self {
        Self {
            ground: theme::BG_DEEP(),
            line: theme::LINE_LIT(),
            text: theme::TEXT(),
            strong: theme::TEXT_STRONG(),
            dim: theme::CYAN_DIM(),
            accent: theme::CYAN(),
            amber: theme::YELLOW(),
            frame: theme::PINK(),
            green: theme::GREEN(),
            red: theme::ALERT(),
        }
    }
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8;
    Color32::from_rgb(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()))
}

impl SdroxideApp {
    /// The docked panel: the frame under the waterfall, then the panel in it.
    pub(in crate::app) fn radio_classic_docked(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        panel_h: f32,
    ) {
        crate::layout::tighten(ui);
        egui::Frame::new()
            .fill(theme::BG_DEEP())
            .inner_margin(egui::Margin { left: 0, right: 0, top: 6, bottom: 0 })
            .show(ui, |ui| {
                self.radio_classic_panel(ui, cmds, (panel_h - 6.0).max(0.0));
            });
    }

    /// The panel itself, `h` points tall and as wide as `ui` is.
    pub(in crate::app) fn radio_classic_panel(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        h: f32,
    ) {
        let ink = Ink::now();
        let w = ui.available_width();
        let (outer, _) = ui.allocate_exact_size(vec2(w, h.max(120.0)), egui::Sense::hover());
        // One frame around all three columns.
        ui.painter().rect_filled(outer, 10.0, ink.ground);
        ui.painter().rect_stroke(
            outer.shrink(1.0),
            10.0,
            Stroke::new(1.5, ink.frame),
            StrokeKind::Inside,
        );
        let inner = outer.shrink2(vec2(12.0, 10.0));
        let (lw, mw, rw) = column_widths(inner.width());
        let left = Rect::from_min_size(inner.min, vec2(lw, inner.height()));
        let mid = Rect::from_min_size(
            Pos2::new(left.right() + 8.0, inner.top()),
            vec2(mw, inner.height()),
        );
        let right = Rect::from_min_size(
            Pos2::new(mid.right() + 8.0, inner.top()),
            vec2(rw, inner.height()),
        );

        let now = crate::time::now_unix();
        let wall = crate::time::now_unix_f64();
        if wall - self.radio_classic.lease_at >= 1.0 || wall < self.radio_classic.lease_at {
            self.radio_classic.lease_at = wall;
            cmds.push(Command::WantAudioLevel);
        }
        let hz = self.state.active_freq_hz();
        let mode = self.state.rx[0].mode;
        let name = self.rc_name(hz, mode, now);

        self.rc_left(ui, left, cmds, &ink, hz, mode);
        self.rc_middle(ui, mid, cmds, &ink, hz, mode, name.as_deref());
        self.rc_right(ui, right, cmds, &ink, hz, mode, name.as_deref(), now);

        // Bars, needles and the marquee move between messages: ask for frames
        // only while there is something to move (and once a second for the
        // clocks, which the frame loop already does).
        let moving = self.radio_classic.vu.is_live()
            || self.radio_classic.bars.iter().any(|b| *b > 0.01)
            || self.audio_level.is_some();
        crate::repaint::after_ms(ui.ctx(), if moving { 33 } else { 250 });
    }

    /// The name on the big line, from what is known about the dial.
    fn rc_name(&self, hz: f64, mode: Mode, now: i64) -> Option<String> {
        let rounded = hz.round() as i64;
        let memory = self
            .memories
            .iter()
            .find(|m| m.mode == mode && m.freq_hz.round() as i64 == rounded)
            .map(|m| m.name.as_str());
        let scheduled = broadcast::at_dial(&self.broadcast, hz, now).map(|s| s.name.as_str());
        let ps = self.rds.as_ref().and_then(|r| r.ps.as_deref());
        station_name(mode, ps, memory, scheduled)
    }

    // ───────────────────────────────────────────────────────────── left column

    fn rc_left(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        cmds: &mut Vec<Command>,
        ink: &Ink,
        hz: f64,
        mode: Mode,
    ) {
        let band = self.state.band;
        ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
            ui.set_clip_rect(rect.intersect(ui.clip_rect()));
            // The sections share whatever height is left over, so the column
            // ends where the other two do instead of stopping short.
            let gap = ((rect.height() - 300.0) / 6.0).clamp(0.0, 16.0);
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            ui.label(RichText::new("RADIO CLASSIC").strong().size(14.0).color(ink.strong));
            ui.add_space(2.0 + gap);

            crate::chrome::menu_caption(ui, "Bands");
            ui.horizontal_wrapped(|ui| {
                for (g, label) in Group::ALL {
                    if crate::chrome::chip(ui, self.radio_classic.group == g, label).clicked() {
                        self.radio_classic.group = g;
                    }
                }
            });

            for (caption, row) in group_rows(self.radio_classic.group) {
                ui.add_space(gap);
                crate::chrome::menu_caption(ui, caption);
                ui.horizontal_wrapped(|ui| {
                    for p in row {
                        let lit = match p.band {
                            Some(b) if b != Band::Gen => band == b && p.holds(hz),
                            _ => p.holds(hz),
                        };
                        if crate::chrome::chip(ui, lit, p.label.as_str()).clicked() && !lit {
                            match p.band {
                                Some(b) => cmds.push(Command::SetBand(b)),
                                None => {
                                    cmds.push(Command::SetVfo {
                                        vfo: self.state.active_vfo,
                                        hz: p.tune_hz,
                                    });
                                    cmds.push(Command::SetMode { rx: RxId::Main, mode: p.mode });
                                }
                            }
                        }
                    }
                });
            }

            ui.add_space(gap);
            crate::chrome::menu_caption(ui, "Mode");
            ui.horizontal_wrapped(|ui| {
                for (label, m) in MODES {
                    if crate::chrome::chip(ui, mode == m, label).clicked() && mode != m {
                        cmds.push(Command::SetMode { rx: RxId::Main, mode: m });
                    }
                }
            });

            ui.add_space(gap);
            crate::chrome::menu_caption(ui, "Mem · Schedule");
            self.rc_memories(ui, cmds, hz, mode);
        });
    }

    /// The memory buttons and the SCHEDULE popup.
    fn rc_memories(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, hz: f64, mode: Mode) {
        let rounded = hz.round() as i64;
        let mut button = None;
        ui.horizontal_wrapped(|ui| {
            for m in self.memories.iter().take(4) {
                let lit = m.mode == mode && m.freq_hz.round() as i64 == rounded;
                let label = format!("{} · {:.3}", m.name, m.freq_hz / 1e6);
                if crate::chrome::chip(ui, lit, label)
                    .on_hover_text("Recall: band, mode, frequency and filter at once")
                    .clicked()
                {
                    cmds.push(Command::RecallMemory(m.id));
                }
            }
            if self.memories.is_empty() {
                ui.label(
                    RichText::new("no memories yet — store one with MEM in the top bar")
                        .size(11.0)
                        .color(theme::gray(130)),
                );
            }
            let r = crate::chrome::chip(ui, self.radio_classic.schedule_open, "SCHEDULE")
                .on_hover_text("Broadcasts on the air now, on this band");
            if r.clicked() {
                self.radio_classic.schedule_open = !self.radio_classic.schedule_open;
            }
            button = Some(r.rect);
        });
        if self.radio_classic.schedule_open
            && let Some(at) = button
        {
            self.rc_schedule_popup(ui.ctx(), at, cmds, hz);
        }
    }

    /// The SCHEDULE list: the broadcasts on the air now on the band the dial
    /// is in (every band, when the dial is outside the broadcast bands).
    fn rc_schedule_popup(
        &mut self,
        ctx: &egui::Context,
        anchor: Rect,
        cmds: &mut Vec<Command>,
        hz: f64,
    ) {
        let now = crate::time::now_unix();
        let band = broadcast::metre_band(hz / 1e3);
        let mut rows: Vec<&sdroxide_types::BroadcastStation> = self
            .broadcast
            .iter()
            .filter(|s| {
                s.on_air_at(now)
                    && band.is_none_or(|b| broadcast::metre_band(s.freq_khz) == Some(b))
            })
            .collect();
        rows.sort_by(|a, b| b.power_kw.unwrap_or(0.0).total_cmp(&a.power_kw.unwrap_or(0.0)));
        rows.truncate(14);
        rows.sort_by(|a, b| a.freq_khz.total_cmp(&b.freq_khz));
        let mut pick = None;
        let mut close = false;
        egui::Area::new(egui::Id::new("radio-classic-schedule"))
            .order(egui::Order::Foreground)
            .fixed_pos(Pos2::new(anchor.left(), (anchor.top() - 4.0).max(0.0)))
            .pivot(Align2::LEFT_BOTTOM)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_width(320.0);
                    ui.horizontal(|ui| {
                        let head = match band {
                            Some(b) => format!("ON THE AIR NOW · {b}"),
                            None => "ON THE AIR NOW".to_string(),
                        };
                        crate::chrome::menu_caption(ui, &head);
                        if ui.small_button("✕").clicked() {
                            close = true;
                        }
                    });
                    if rows.is_empty() {
                        ui.label(
                            RichText::new("nothing listed on air here at this hour")
                                .size(11.0)
                                .color(theme::gray(130)),
                        );
                    }
                    for s in &rows {
                        let label = format!("{:>8.0} kHz  {}  {}", s.freq_khz, s.name, s.lang);
                        if ui.selectable_label(false, label).clicked() {
                            pick = Some((*s).clone());
                        }
                    }
                });
            });
        if let Some(s) = pick {
            cmds.push(Command::SetVfo { vfo: self.state.active_vfo, hz: s.freq_hz() });
            cmds.push(Command::SetMode { rx: RxId::Main, mode: s.mode() });
            self.radio_classic.schedule_open = false;
        }
        if close {
            self.radio_classic.schedule_open = false;
        }
    }

    // ─────────────────────────────────────────────────────────── middle column

    #[allow(clippy::too_many_arguments)]
    fn rc_middle(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        cmds: &mut Vec<Command>,
        ink: &Ink,
        hz: f64,
        mode: Mode,
        name: Option<&str>,
    ) {
        let vu_w = vu_meter::WIDTH_COMPACT.max(64.0);
        let main =
            Rect::from_min_max(rect.min, Pos2::new(rect.right() - vu_w - 8.0, rect.bottom()));
        let vu_rect = Rect::from_min_max(Pos2::new(rect.right() - vu_w, rect.top()), rect.max);

        // The header: the name small at the top left, the band's range at the
        // right, the mode centred and big.
        let head_h = (rect.height() * 0.17).clamp(54.0, 90.0);
        let head = Rect::from_min_size(main.min, vec2(main.width(), head_h));
        let p = ui.painter();
        p.rect_filled(head, 6.0, ink.ground);
        p.rect_stroke(head, 6.0, Stroke::new(1.0, ink.line), StrokeKind::Inside);
        if let Some(n) = name {
            p.text(
                head.left_top() + vec2(10.0, 8.0),
                Align2::LEFT_TOP,
                n.to_uppercase(),
                FontId::proportional(14.0),
                ink.amber,
            );
        }
        if let Some((lo, hi)) = self.rc_range(hz) {
            p.text(
                head.right_top() + vec2(-10.0, 8.0),
                Align2::RIGHT_TOP,
                format!("{lo}–{hi} MHz"),
                FontId::proportional(10.0),
                ink.dim,
            );
        }
        let mode_label = MODES
            .iter()
            .find(|(_, m)| *m == mode)
            .map_or_else(|| format!("{mode:?}"), |(l, _)| (*l).to_string());
        p.text(
            head.center() + vec2(0.0, 8.0),
            Align2::CENTER_CENTER,
            mode_label,
            FontId::proportional((head_h * 0.52).clamp(24.0, 46.0)),
            ink.accent,
        );
        p.text(
            head.right_bottom() + vec2(-10.0, -6.0),
            Align2::RIGHT_BOTTOM,
            "MODE",
            FontId::proportional(10.0),
            ink.dim,
        );

        // The boxes at the foot: BW, STEP, AGC, SQL.
        let boxes_h = 34.0;
        let meter_h = (rect.height() * 0.2).clamp(52.0, 84.0);
        let boxes = Rect::from_min_size(
            Pos2::new(main.left(), main.bottom() - boxes_h),
            vec2(main.width(), boxes_h),
        );
        let meter = Rect::from_min_size(
            Pos2::new(main.left(), boxes.top() - 6.0 - meter_h),
            vec2(main.width(), meter_h),
        );
        let scope = Rect::from_min_max(
            Pos2::new(main.left(), head.bottom() + 6.0),
            Pos2::new(main.right(), meter.top() - 6.0),
        );
        if scope.height() > 30.0 {
            self.rc_scope(ui, scope, ink, hz, mode);
        }
        self.rc_smeter(ui, meter, ink);

        let rx = &self.state.rx[0];
        let bw = (rx.filter_hi - rx.filter_lo).abs();
        let items = [
            (
                "BW",
                if bw >= 1000.0 {
                    format!("{:.1} kHz", bw / 1000.0)
                } else {
                    format!("{bw:.0} Hz")
                },
            ),
            ("STEP", format!("{:.0}", self.ui_settings.tune_step_hz)),
            ("AGC", rx.agc.label().to_uppercase()),
            (
                "SQL",
                if rx.squelch_db <= sdroxide_types::SQUELCH_OPEN_DB + 0.5 {
                    "OPEN".to_string()
                } else {
                    format!("{:.0} dB", rx.squelch_db)
                },
            ),
        ];
        let cell_w = (boxes.width() - 3.0 * 6.0) / 4.0;
        for (i, (cap, val)) in items.iter().enumerate() {
            let r = Rect::from_min_size(
                boxes.min + vec2(i as f32 * (cell_w + 6.0), 0.0),
                vec2(cell_w, boxes_h),
            );
            let p = ui.painter();
            p.rect_stroke(r, 3.0, Stroke::new(1.0, ink.line), StrokeKind::Inside);
            p.text(
                r.left_top() + vec2(5.0, 3.0),
                Align2::LEFT_TOP,
                *cap,
                FontId::proportional(8.5),
                ink.dim,
            );
            p.text(
                r.left_bottom() + vec2(5.0, -4.0),
                Align2::LEFT_BOTTOM,
                val,
                FontId::proportional(12.0),
                ink.strong,
            );
        }

        // The VU: the volume control too, as in the DAB panel.
        self.rc_vu_step(ui);
        ui.scope_builder(egui::UiBuilder::new().max_rect(vu_rect), |ui| {
            let out = vu_meter::stereo(
                ui,
                &self.radio_classic.vu,
                vu_rect.size(),
                true,
                Some(self.state.rx[0].volume),
            );
            self.dab_vu_volume(ui, cmds, out);
        });
    }

    /// The range of the band the dial sits in, in MHz, for the header.
    fn rc_range(&self, hz: f64) -> Option<(String, String)> {
        let preset = group_rows(Group::Broadcast)
            .into_iter()
            .flat_map(|(_, r)| r)
            .filter(|p| p.holds(hz))
            .min_by(|a, b| (a.hi_hz - a.lo_hz).total_cmp(&(b.hi_hz - b.lo_hz)))?;
        let fmt = |v: f64| {
            let s = format!("{:.3}", v / 1e6);
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        };
        Some((fmt(preset.lo_hz), fmt(preset.hi_hz)))
    }

    /// The mini spectrum and its waterfall, around the dial.
    fn rc_scope(&mut self, ui: &mut egui::Ui, rect: Rect, ink: &Ink, hz: f64, mode: Mode) {
        let rx = &self.state.rx[0];
        let half = if mode == Mode::Wfm {
            150_000.0
        } else {
            ((rx.filter_hi - rx.filter_lo).abs() as f64 * 1.5).clamp(6_000.0, 60_000.0)
        };
        let held = self.frame.clone().or_else(|| self.radio_classic.frame.clone());
        if let Some(frame) = held.as_ref()
            && self.radio_classic.wf_seq != Some(frame.seq)
            && !frame.bins.is_empty()
            && frame.span_hz > 0.0
        {
            self.radio_classic.wf_seq = Some(frame.seq);
            let n = frame.bins.len();
            let lo = frame.center_hz - frame.span_hz / 2.0;
            let row: Vec<u8> = (0..WF_BINS)
                .map(|i| {
                    let f = hz - half + 2.0 * half * (i as f64 + 0.5) / WF_BINS as f64;
                    let at = ((f - lo) / frame.span_hz * n as f64).floor();
                    if at < 0.0 || at >= n as f64 { 0 } else { frame.bins[at as usize] }
                })
                .collect();
            let wf = &mut self.radio_classic.wf;
            wf.push_front(row);
            wf.truncate(WF_ROWS);
        }
        let p = ui.painter_at(rect);
        p.rect_filled(rect, 4.0, ink.ground);
        let line_h = (rect.height() * 0.28).max(18.0);
        let line_rect = Rect::from_min_size(rect.min, vec2(rect.width(), line_h));
        let wf_rect = Rect::from_min_max(Pos2::new(rect.left(), line_rect.bottom()), rect.max);
        if let Some(newest) = self.radio_classic.wf.front() {
            let pts: Vec<Pos2> = newest
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    Pos2::new(
                        line_rect.left() + line_rect.width() * (i as f32 + 0.5) / WF_BINS as f32,
                        line_rect.bottom()
                            - 2.0
                            - (line_rect.height() - 6.0) * f32::from(*v) / 255.0,
                    )
                })
                .collect();
            p.add(egui::Shape::line(pts, Stroke::new(1.2, ink.accent)));
        }
        let rows = self.radio_classic.wf.len().max(1);
        let cell_h = wf_rect.height() / WF_ROWS as f32;
        let cell_w = wf_rect.width() / WF_BINS as f32;
        for (r, row) in self.radio_classic.wf.iter().enumerate() {
            for (i, v) in row.iter().enumerate() {
                let t = f32::from(*v) / 255.0;
                let c = if t < 0.8 {
                    mix(ink.ground, ink.accent, t / 0.8)
                } else {
                    mix(ink.accent, ink.strong, (t - 0.8) * 5.0)
                };
                let cell = Rect::from_min_size(
                    Pos2::new(
                        wf_rect.left() + i as f32 * cell_w,
                        wf_rect.top() + r as f32 * cell_h,
                    ),
                    vec2(cell_w + 0.5, cell_h + 0.5),
                );
                p.rect_filled(cell, 0.0, c);
            }
        }
        let _ = rows;
        // The dial's own mark, at the centre.
        let x = rect.center().x;
        p.line_segment(
            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            Stroke::new(1.0, ink.amber.gamma_multiply(0.6)),
        );
        p.rect_stroke(rect, 4.0, Stroke::new(1.0, ink.line), StrokeKind::Inside);
    }

    /// The segmented S-meter, red above S9.
    fn rc_smeter(&self, ui: &mut egui::Ui, rect: Rect, ink: &Ink) {
        let s_dbm = self.meters.map(|m| m.s_dbm);
        let p = ui.painter_at(rect);
        p.rect_stroke(rect, 4.0, Stroke::new(1.0, ink.line), StrokeKind::Inside);
        let reading = s_dbm.map_or_else(|| "—".to_string(), s_reading);
        p.text(
            rect.left_top() + vec2(8.0, 5.0),
            Align2::LEFT_TOP,
            reading,
            FontId::proportional(13.0),
            ink.strong,
        );
        p.text(
            rect.right_top() + vec2(-8.0, 6.0),
            Align2::RIGHT_TOP,
            "S-METER",
            FontId::proportional(10.0),
            ink.dim,
        );
        let bar = Rect::from_min_max(
            Pos2::new(rect.left() + 8.0, rect.top() + 24.0),
            Pos2::new(
                rect.right() - 8.0,
                rect.top() + 24.0 + (rect.height() * 0.28).clamp(10.0, 18.0),
            ),
        );
        const SEGS: usize = 38;
        let fill = s_dbm.map_or(0.0, smeter_fill);
        let s9 = smeter_fill(-73.0);
        let seg_w = bar.width() / SEGS as f32;
        for i in 0..SEGS {
            let t = (i as f32 + 0.5) / SEGS as f32;
            let lit = t <= fill;
            let hot = t > s9;
            let c = match (lit, hot) {
                (true, false) => ink.green,
                (true, true) => ink.red,
                (false, false) => mix(ink.ground, ink.green, 0.12),
                (false, true) => mix(ink.ground, ink.red, 0.12),
            };
            let r = Rect::from_min_size(
                Pos2::new(bar.left() + i as f32 * seg_w, bar.top()),
                vec2((seg_w - 1.5).max(1.0), bar.height()),
            );
            p.rect_filled(r, 1.0, c);
        }
        for (label, dbm, hot) in [
            ("1", -121.0, false),
            ("3", -109.0, false),
            ("5", -97.0, false),
            ("7", -85.0, false),
            ("9", -73.0, false),
            ("+20", -53.0, true),
            ("+40", -33.0, true),
            ("+60", -13.0, true),
        ] {
            let x = bar.left() + bar.width() * smeter_fill(dbm);
            p.text(
                Pos2::new(x.min(bar.right() - 6.0), bar.bottom() + 2.0),
                Align2::CENTER_TOP,
                label,
                FontId::proportional(9.0),
                if hot { ink.red } else { ink.dim },
            );
        }
        if let Some(d) = s_dbm {
            p.text(
                rect.left_bottom() + vec2(8.0, -4.0),
                Align2::LEFT_BOTTOM,
                format!("RSSI {:.0} dBµV", d + 107.0),
                FontId::proportional(10.0),
                ink.text,
            );
            p.text(
                rect.right_bottom() + vec2(-8.0, -4.0),
                Align2::RIGHT_BOTTOM,
                format!("{d:.0} dBm"),
                FontId::proportional(10.0),
                ink.text,
            );
        }
    }

    /// Move the VU and the bars on by this frame's time, from the last level
    /// the station sent (and to silence when it has stopped sending).
    fn rc_vu_step(&mut self, ui: &egui::Ui) {
        let now = ui.input(|i| i.time);
        let dt = self.radio_classic.vu_at.map_or(0.0, |t| (now - t) as f32);
        self.radio_classic.vu_at = Some(now);
        let wall = crate::time::now_unix_f64();
        let fresh = self
            .audio_level
            .as_ref()
            .filter(|(at, _)| wall - *at < f64::from(LEVEL_FRESH_S))
            .map(|(_, l)| l);
        self.radio_classic.vu.step(fresh.map(|l| l.rms_db), dt);
        let st = &mut self.radio_classic;
        for i in 0..AUDIO_BANDS {
            let target = fresh.and_then(|l| l.bands.get(i)).map_or(0.0, |b| byte_to_unit(*b));
            // Quick up, slower down: a bar, not a trace.
            let k = if target > st.bars[i] { 0.6 } else { (dt * 6.0).clamp(0.05, 0.5) };
            st.bars[i] += (target - st.bars[i]) * k;
            if st.bars[i] >= st.peaks[i] {
                st.peaks[i] = st.bars[i];
            } else {
                st.peaks[i] = (st.peaks[i] - dt * 0.35).max(st.bars[i]);
            }
        }
    }

    // ──────────────────────────────────────────────────────────── right column

    #[allow(clippy::too_many_arguments)]
    fn rc_right(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        cmds: &mut Vec<Command>,
        ink: &Ink,
        hz: f64,
        mode: Mode,
        name: Option<&str>,
        now: i64,
    ) {
        {
            let p = ui.painter();
            p.rect_filled(rect, 8.0, ink.ground);
            p.rect_stroke(rect, 8.0, Stroke::new(1.0, ink.line), StrokeKind::Inside);
        }
        let inner = rect.shrink2(vec2(14.0, 10.0));
        let h = inner.height();
        let rds = self.rds.as_ref().filter(|r| mode == Mode::Wfm && !r.is_empty());

        // Row 1: the mode tag, STEREO and MEM chips on the left; the clocks on
        // the right.
        let head_h = 34.0;
        let p = ui.painter();
        let tag = if mode == Mode::Wfm { "FM".to_string() } else { mode_tag(mode) };
        let tag_rect =
            p.text(inner.left_top(), Align2::LEFT_TOP, tag, FontId::proportional(24.0), ink.amber);
        let mut x = tag_rect.right() + 12.0;
        let stereo = mode == Mode::Wfm && self.meters.is_some_and(|m| m.stereo);
        if stereo {
            x = pill(p, Pos2::new(x, inner.top() + 3.0), "STEREO", ink.strong, ink.line) + 8.0;
        }
        let memory = self
            .memories
            .iter()
            .find(|m| m.mode == mode && m.freq_hz.round() as i64 == hz.round() as i64);
        if let Some(m) = memory {
            let label = format!("MEM {}", m.id);
            x = pill(p, Pos2::new(x, inner.top() + 3.0), &label, ink.strong, ink.red) + 8.0;
        }
        p.text(
            Pos2::new(x, inner.top() + 6.0),
            Align2::LEFT_TOP,
            format!(
                "{} {:.1}",
                if (64e6..=108e6).contains(&hz) { "FM" } else { self.state.band.label() },
                hz / 1e6
            ),
            FontId::proportional(15.0),
            ink.strong,
        );
        let off = crate::time::local_offset_at(now);
        for (i, (cap, clock)) in
            [("LOC", crate::time::utc_clock(now + off)), ("UTC", crate::time::utc_clock(now))]
                .into_iter()
                .enumerate()
        {
            let y = inner.top() + i as f32 * 17.0;
            p.text(
                Pos2::new(inner.right() - 92.0, y + 3.0),
                Align2::RIGHT_TOP,
                cap,
                FontId::proportional(9.0),
                ink.dim,
            );
            p.text(
                Pos2::new(inner.right(), y),
                Align2::RIGHT_TOP,
                clock,
                FontId::monospace(15.0),
                if i == 1 { ink.amber } else { ink.strong },
            );
        }

        // Row 2: the VFO, in the same widget as the top bar's.
        let digit = (h * 0.15).clamp(26.0, 62.0);
        let vfo_rect = Rect::from_min_size(
            Pos2::new(inner.left(), inner.top() + head_h + 6.0),
            vec2(inner.width(), digit * 1.25),
        );
        let wheel = self.input.cfg.wheel;
        let active = self.state.active_vfo;
        let mut new_hz = None;
        ui.scope_builder(egui::UiBuilder::new().max_rect(vfo_rect), |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                new_hz = crate::widgets::freq_display::show(
                    ui,
                    crate::layout::salted_id(ui.ctx(), "radio-classic-freq"),
                    hz,
                    wheel,
                    digit,
                    Some(ink.amber),
                    9,
                );
            });
        });
        if let Some(hz) = new_hz {
            cmds.push(Command::SetVfo { vfo: active, hz });
        }
        let hint_y = vfo_rect.bottom() + 1.0;
        ui.painter().text(
            Pos2::new(inner.center().x, hint_y),
            Align2::CENTER_TOP,
            "wheel on a digit · click upper/lower half ± · right-click zero · double-click to type",
            FontId::proportional(9.5),
            ink.dim,
        );

        // Row 3: the station name.
        let name_y = hint_y + 18.0;
        let name_size = (h * 0.1).clamp(20.0, 40.0);
        let shown = name.map(|n| n.to_uppercase()).unwrap_or_else(|| "—".into());
        let spaced: String = shown.chars().flat_map(|c| [c, ' ']).collect();
        ui.painter().text(
            Pos2::new(inner.center().x, name_y),
            Align2::CENTER_TOP,
            spaced.trim_end(),
            FontId::proportional(name_size),
            ink.amber,
        );

        // Row 4: RadioText.
        let rt_rect = Rect::from_min_size(
            Pos2::new(inner.left(), name_y + name_size + 8.0),
            vec2(inner.width(), 30.0),
        );
        let rt_text =
            rds.and_then(|r| r.radiotext.as_deref()).map(str::trim).filter(|s| !s.is_empty());
        if let Some(rt) = rt_text {
            marquee(ui, rt_rect, rt, ink);
        }

        // Row 5: the RDS chips — only what the station sends.
        let chips_y = rt_rect.bottom() + 8.0;
        if let Some(r) = rds {
            self.rds_chips(ui, Pos2::new(inner.left(), chips_y), inner.right(), r, ink);
        }

        // Row 6: the audio bars, down to the foot.
        let bars_top = chips_y + 30.0;
        let bars = Rect::from_min_max(Pos2::new(inner.left(), bars_top), inner.right_bottom());
        if bars.height() > 20.0 {
            self.rc_bars(ui, bars, ink);
        }
    }

    fn rds_chips(&self, ui: &mut egui::Ui, at: Pos2, right: f32, r: &RdsData, ink: &Ink) {
        let standard = self.view.rds_standard.resolve(r.pi, r.ecc);
        let mut chips: Vec<(&str, String, Color32)> = Vec::new();
        if let Some(pty) = r.pty {
            chips.push(("PTY", pty_name(pty, standard).to_string(), ink.line));
        }
        if let Some(pi) = r.pi {
            chips.push(("PI", format!("{pi:04X}"), ink.line));
        }
        if r.tp {
            chips.push(("", "TP".into(), ink.green));
        }
        if r.ta {
            chips.push(("", "TA".into(), ink.red));
        }
        match r.music {
            Some(true) => chips.push(("", "MUSIC ♪".into(), ink.line)),
            Some(false) => chips.push(("", "SPEECH".into(), ink.line)),
            None => {}
        }
        if !r.af.is_empty() {
            let list: Vec<String> =
                r.af.iter().take(3).map(|hz| format!("{:.1}", f64::from(*hz) / 1e6)).collect();
            let more = if r.af.len() > 3 { format!(" +{}", r.af.len() - 3) } else { String::new() };
            chips.push(("AF", format!("{}{more} MHz", list.join(" · ")), ink.line));
        }
        if let Some(c) = &r.clock {
            chips.push(("CT", c.label(), ink.line));
        }
        let mut x = at.x;
        let p = ui.painter();
        for (cap, val, edge) in chips {
            let text = if cap.is_empty() { val } else { format!("{cap}  {val}") };
            let end = pill(p, Pos2::new(x, at.y), &text, ink.strong, edge);
            if end > right {
                break;
            }
            x = end + 6.0;
        }
    }

    /// The audio bars with their peak caps.
    fn rc_bars(&self, ui: &mut egui::Ui, rect: Rect, ink: &Ink) {
        let st = &self.radio_classic;
        let n = st.bars.len();
        let slot = rect.width() / n as f32;
        let p = ui.painter_at(rect);
        for i in 0..n {
            let x = rect.left() + i as f32 * slot;
            let hgt = rect.height() * st.bars[i].clamp(0.0, 1.0);
            let bar = Rect::from_min_max(
                Pos2::new(x + 1.0, rect.bottom() - hgt),
                Pos2::new(x + slot - 1.0, rect.bottom()),
            );
            p.rect_filled(bar, 1.5, mix(ink.accent, ink.ground, 0.15));
            let cap_y = rect.bottom() - rect.height() * st.peaks[i].clamp(0.0, 1.0);
            p.rect_filled(
                Rect::from_min_size(
                    Pos2::new(x + 1.0, cap_y - 3.0),
                    vec2((slot - 2.0).max(1.0), 2.0),
                ),
                0.0,
                ink.strong,
            );
        }
    }
}

/// A short mode tag for the right column's corner.
fn mode_tag(mode: Mode) -> String {
    MODES
        .iter()
        .find(|(_, m)| *m == mode)
        .map_or_else(|| format!("{mode:?}").to_uppercase(), |(l, _)| (*l).to_string())
}

/// A small outlined label; returns the x it ends at.
fn pill(p: &egui::Painter, at: Pos2, text: &str, ink: Color32, edge: Color32) -> f32 {
    let galley = p.layout_no_wrap(text.to_string(), FontId::proportional(11.5), ink);
    let r = Rect::from_min_size(at, galley.size() + vec2(16.0, 8.0));
    p.rect_stroke(r, r.height() / 2.0, Stroke::new(1.0, edge), StrokeKind::Inside);
    p.galley(Pos2::new(r.left() + 8.0, r.top() + 4.0), galley, ink);
    r.right()
}

/// RadioText in its box, scrolling when it is longer than the box.
fn marquee(ui: &egui::Ui, rect: Rect, text: &str, ink: &Ink) {
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 6.0, mix(ink.ground, ink.line, 0.12));
    p.rect_stroke(rect, 6.0, Stroke::new(1.0, ink.line), StrokeKind::Inside);
    let tag_end = pill(&p, rect.left_top() + vec2(8.0, 5.0), "RT", ink.accent, ink.accent);
    let text_rect =
        Rect::from_min_max(Pos2::new(tag_end + 10.0, rect.top()), rect.max - vec2(8.0, 0.0));
    let clip = ui.painter_at(text_rect);
    let font = FontId::proportional(15.0);
    let galley = clip.layout_no_wrap(text.to_string(), font.clone(), ink.strong);
    let overflow = galley.size().x - text_rect.width();
    if overflow <= 0.0 {
        clip.text(
            Pos2::new(text_rect.left(), rect.center().y),
            Align2::LEFT_CENTER,
            text,
            font,
            ink.strong,
        );
    } else {
        let gap = 48.0;
        let t = ui.input(|i| i.time) as f32;
        let off = (t * 36.0) % (galley.size().x + gap);
        for k in 0..2 {
            clip.text(
                Pos2::new(
                    text_rect.left() - off + k as f32 * (galley.size().x + gap),
                    rect.center().y,
                ),
                Align2::LEFT_CENTER,
                text,
                font.clone(),
                ink.strong,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use RadioClassicPlace::*;

    #[test]
    fn off_is_hidden_and_on_docks_in_a_voice_mode() {
        assert_eq!(place(false, false, Mode::Wfm, false), Hidden);
        assert_eq!(place(true, false, Mode::Wfm, false), Docked);
        assert_eq!(place(true, false, Mode::Usb, false), Docked);
    }

    #[test]
    fn a_detached_panel_is_a_window_whatever_else_is_true() {
        for mode in [Mode::Wfm, Mode::Cw, Mode::Ft8, Mode::Dab] {
            for on in [false, true] {
                assert_eq!(place(on, true, mode, false), Window, "{mode:?} on={on}");
            }
        }
    }

    #[test]
    fn where_the_slot_is_taken_or_there_is_no_room_it_is_a_window_not_nothing() {
        for mode in [Mode::Ft8, Mode::Dab, Mode::Cw] {
            assert_eq!(place(true, false, mode, false), Window, "{mode:?}");
        }
        assert_eq!(place(true, false, Mode::Wfm, true), Window, "a phone has no room to dock");
    }

    #[test]
    fn on_fm_the_name_is_the_stations_and_elsewhere_the_memorys_or_the_schedules() {
        let name = |m, ps, mem, sch| station_name(m, ps, mem, sch);
        assert_eq!(
            name(Mode::Wfm, Some("EUROPE 1"), Some("M3"), None).as_deref(),
            Some("EUROPE 1")
        );
        // A station that sends no name falls back to the memory, then the schedule.
        assert_eq!(name(Mode::Wfm, Some("  "), Some("M3"), Some("BBC")).as_deref(), Some("M3"));
        assert_eq!(name(Mode::Wfm, None, None, Some("BBC")).as_deref(), Some("BBC"));
        // Off FM the RDS name is never used, even if one is lying around.
        assert_eq!(name(Mode::Am, Some("EUROPE 1"), None, Some("RFI")).as_deref(), Some("RFI"));
        assert_eq!(
            name(Mode::Am, Some("EUROPE 1"), Some("Radio 3"), Some("RFI")).as_deref(),
            Some("Radio 3")
        );
        assert_eq!(name(Mode::Usb, None, None, None), None);
    }

    #[test]
    fn the_s_meter_reads_like_a_radio() {
        assert_eq!(s_reading(-73.0), "S9");
        assert_eq!(s_reading(-53.0), "S9+20");
        assert_eq!(s_reading(-109.0), "S3");
        assert_eq!(s_reading(-140.0), "S0");
        assert!(smeter_fill(-127.0) < 0.001 && smeter_fill(-13.0) > 0.999);
        assert!(smeter_fill(-200.0) == 0.0 && smeter_fill(40.0) == 1.0);
        // S9 sits where the scale prints it, a little under the middle.
        assert!((smeter_fill(-73.0) - 0.4737).abs() < 0.001);
    }

    #[test]
    fn the_fm_button_is_lit_inside_the_band_and_the_extension_below_it() {
        let row = broadcast_row();
        let lit = |hz: f64| -> Vec<&str> {
            row.iter().filter(|p| p.holds(hz)).map(|p| p.label.as_str()).collect()
        };
        assert_eq!(lit(105.5e6), vec!["FM 87.5–108"]);
        assert_eq!(lit(76.0e6), vec!["FM ext 64–87.5"]);
        assert!(lit(1.0e6).contains(&"MW EU"));
    }

    #[test]
    fn the_shortwave_row_is_the_broadcast_bands_in_order() {
        let row = shortwave_row();
        let labels: Vec<&str> = row.iter().map(|p| p.label.as_str()).collect();
        assert_eq!(labels.first(), Some(&"120"));
        assert_eq!(labels.last(), Some(&"11"));
        assert_eq!(labels.len(), broadcast::METRE_BANDS.len());
        // A click lands inside the band it names, in AM, on a 5 kHz channel.
        for p in &row {
            assert!(p.holds(p.tune_hz), "{} tunes outside itself", p.label);
            assert_eq!(p.mode, Mode::Am);
            assert_eq!((p.tune_hz / 1e3) % 5.0, 0.0, "{}", p.label);
        }
    }

    #[test]
    fn every_group_has_something_to_click() {
        for (g, _) in Group::ALL {
            let rows = group_rows(g);
            assert!(!rows.is_empty() && rows.iter().all(|(_, r)| !r.is_empty()), "{g:?}");
        }
    }

    #[test]
    fn the_columns_fill_the_panel_without_overlapping() {
        for w in [1180.0f32, 1500.0, 1896.0] {
            let (l, m, r) = column_widths(w);
            assert!(l + m + r + 16.0 <= w + 0.5, "{w}: {l}+{m}+{r}");
            assert!(r > l && r > m, "the readout is the widest column at {w}");
        }
    }
}
