//! The antenna calculator — a frequency in, dimensions and SWR out — for the
//! shapes the closed-form calculator knows (dipole, inverted-V, vertical,
//! ground plane, loop, Yagi). It lives in the EXTRAS launcher; the
//! method-of-moments solver stays in the standalone `antenna` CLI for now.

use eframe::egui;

impl super::SdroxideApp {
    pub(in crate::app) fn antenna_window(&mut self, ctx: &egui::Context) {
        if !self.show_antenna {
            return;
        }
        let show = self.tool_window(ctx, "antenna", "ANTENNA", [420.0, 600.0], true, |me, ui| {
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
            }
            ui.label("MHz");
        });
        ui.separator();

        let shapes = antenna_model::all(self.antenna_freq_mhz * 1e6);
        egui::ScrollArea::vertical().show(ui, |ui| {
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
        });
    }
}
