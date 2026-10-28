//! The antenna calculator — a frequency in, dimensions and SWR out — for the
//! shapes the closed-form calculator knows (dipole, inverted-V, vertical,
//! ground plane, loop, Yagi), plus an on-demand method-of-moments solve of the
//! presets for the real impedance and gain. It lives in the EXTRAS launcher.

use eframe::egui;

/// One preset's solved impedance and gain.
pub(in crate::app) struct SolvedRow {
    pub(in crate::app) name: String,
    pub(in crate::app) r: f64,
    pub(in crate::app) x: f64,
    pub(in crate::app) swr: f64,
    pub(in crate::app) gain: f64,
}

impl super::SdroxideApp {
    pub(in crate::app) fn antenna_window(&mut self, ctx: &egui::Context) {
        if !self.show_antenna {
            return;
        }
        let show = self.tool_window(ctx, "antenna", "ANTENNA", [420.0, 680.0], true, |me, ui| {
            me.antenna_body(ui);
        });
        self.show_antenna = show;
    }

    fn antenna_body(&mut self, ui: &mut egui::Ui) {
        crate::chrome::window_body_bg(ui);
        crate::chrome::menu_caption(ui, "Cutting lengths");
        ui.horizontal(|ui| {
            ui.label("Frequency");
            let mut mhz = self.antenna_freq_mhz;
            if ui
                .add(egui::DragValue::new(&mut mhz).speed(0.05).range(0.05..=3000.0))
                .on_hover_text("The frequency to cut for")
                .changed()
            {
                self.antenna_freq_mhz = mhz;
                // A new frequency invalidates the earlier solve.
                self.antenna_solved = None;
            }
            ui.label("MHz");
        });

        egui::ScrollArea::vertical().show(ui, |ui| {
            if ui
                .button("Solve impedance and gain")
                .on_hover_text(
                    "Run the method-of-moments solver on the presets — the real feed \
                     impedance, SWR and gain. Takes a few seconds.",
                )
                .clicked()
            {
                self.antenna_solve();
            }
            ui.separator();

            let shapes = antenna_model::all(self.antenna_freq_mhz * 1e6);
            for d in shapes {
                ui.group(|ui| {
                    ui.label(egui::RichText::new(d.name).strong());
                    ui.label(egui::RichText::new(&d.notes).small().weak());
                    for dim in &d.dims {
                        ui.horizontal(|ui| {
                            ui.label(&dim.what);
                            ui.add_space(8.0);
                            ui.monospace(format!("{:.2} m", dim.metres));
                        });
                    }
                    if let Some((r, x)) = d.feed_z {
                        ui.horizontal(|ui| {
                            ui.label(format!("feed ≈ {r:.0} Ω"));
                            ui.add_space(8.0);
                            ui.monospace(format!("SWR {:.2}", antenna_model::swr_50(r, x)));
                        });
                    }
                });
                ui.add_space(4.0);
            }

            if let Some(rows) = &self.antenna_solved {
                ui.separator();
                crate::chrome::menu_caption(ui, "Solved");
                for row in rows {
                    ui.group(|ui| {
                        ui.label(egui::RichText::new(&row.name).strong());
                        ui.horizontal(|ui| {
                            ui.label(format!("Zin {:.1}{:+.1}j Ω", row.r, row.x));
                            ui.add_space(8.0);
                            ui.monospace(format!("SWR {:.2}", row.swr));
                            ui.add_space(8.0);
                            ui.monospace(format!("{:.1} dBi", row.gain));
                        });
                    });
                    ui.add_space(4.0);
                }
            }
        });
    }

    /// Solve the presets at the current frequency (synchronously — the solver is
    /// a few seconds, which is the honest cost of the method of moments).
    fn antenna_solve(&mut self) {
        use antenna_model::solver::{SolverConfig, analyze, max_gain, swr};
        let freq_hz = self.antenna_freq_mhz * 1e6;
        let mut rows = Vec::new();
        for a in antenna_model::presets::catalog(freq_hz) {
            if let Ok(sol) = analyze(&a, &SolverConfig::default()) {
                let (g, _, _) = max_gain(&sol);
                rows.push(SolvedRow {
                    name: a.name,
                    r: sol.zin.re,
                    x: sol.zin.im,
                    swr: swr(sol.zin, 50.0),
                    gain: g,
                });
            }
        }
        self.antenna_solved = Some(rows);
    }
}
