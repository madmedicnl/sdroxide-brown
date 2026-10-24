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

use eframe::egui::{self, RichText};
use sdroxide_types::{Command, Mode, RxId};

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

const COLUMN_W: f32 = 104.0;
const BUTTON_H: f32 = 30.0;

impl super::SdroxideApp {
    pub(in crate::app) fn aircraft_window(&mut self, ctx: &egui::Context, cmds: &mut Vec<Command>) {
        if !self.show_aircraft {
            self.aircraft_was_open = false;
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
        let h = ui.available_height().max(120.0);
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(COLUMN_W, h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    crate::chrome::menu_caption(ui, "Aviation");
                    for m in MODES {
                        let b = crate::chrome::chip_sized(
                            ui,
                            m.mode == shown,
                            m.label,
                            egui::vec2(COLUMN_W - 8.0, BUTTON_H),
                        )
                        .on_hover_text(m.hover);
                        if b.clicked() {
                            self.aircraft_view = m.mode;
                            cmds.push(Command::SetMode { rx: RxId::Main, mode: m.mode });
                        }
                    }
                },
            );
            ui.vertical(|ui| {
                ui.set_min_width(ui.available_width());
                self.aircraft_map(ui, cmds, shown, active, h);
            });
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
            Mode::Hfdl => self.hfdl_map_pane(ui, now, map_h),
            _ => {
                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!(
                        "{label} has no map of its own: it carries a position only now and then. \
                         The messages are in the panel under the waterfall, and an aircraft heard \
                         here that also sends ADS-B shows on the ADS-B map."
                    ))
                    .size(11.5)
                    .color(crate::theme::gray(160)),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn the_column_labels_fit_their_buttons() {
        for m in MODES {
            assert!(m.label.len() <= 6, "{} is too long for a {COLUMN_W} pt column", m.label);
        }
    }
}
