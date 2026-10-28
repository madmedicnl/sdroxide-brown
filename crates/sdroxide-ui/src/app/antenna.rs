//! The antenna calculator — a frequency in, dimensions and SWR out — for the
//! shapes the closed-form calculator knows (dipole, inverted-V, vertical,
//! ground plane, loop, Yagi), an on-demand method-of-moments solve of the
//! presets, and a Yagi optimizer. It lives in the EXTRAS launcher.

use eframe::egui;

use antenna_model::obstacle::{Obstacle, attenuation_db};
use antenna_model::optimize::Objective;

/// One preset's solved impedance and gain.
pub(in crate::app) struct SolvedRow {
    pub(in crate::app) name: String,
    pub(in crate::app) r: f64,
    pub(in crate::app) x: f64,
    pub(in crate::app) swr: f64,
    pub(in crate::app) gain: f64,
}

/// The optimizer's objective rendered for a person.
fn objective_value(obj: Objective, score: f64) -> String {
    match obj {
        Objective::Gain => format!("{:.1} dBi", -score),
        Objective::Swr => format!("{:.2}:1", score),
        Objective::Fb => format!("{:.1} dB", -score),
        Objective::Jx => format!("{:.1} Ω", score),
    }
}

impl super::SdroxideApp {
    pub(in crate::app) fn antenna_window(&mut self, ctx: &egui::Context) {
        if !self.show_antenna {
            return;
        }
        self.antenna_poll();
        let show = self.tool_window(ctx, "antenna", "ANTENNA", [420.0, 720.0], true, |me, ui| {
            me.antenna_body(ui);
        });
        self.show_antenna = show;
    }

    /// Collect a background-thread optimization that has finished.
    fn antenna_poll(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(rx) = &self.antenna_optimize_rx {
            if let Ok(result) = rx.try_recv() {
                self.antenna_optimized = result;
                self.antenna_optimize_rx = None;
                self.antenna_optimizing = false;
            }
        }
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
                // A new frequency invalidates the earlier solve and optimization.
                self.antenna_solved = None;
                self.antenna_optimized = None;
            }
            ui.label("MHz");
        });

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal(|ui| {
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
                ui.label("Yagi stack");
                for n in [1, 2, 4] {
                    let sel = self.antenna_stack == n;
                    if ui.selectable_label(sel, format!("{n}×")).clicked() {
                        self.antenna_stack = n;
                        self.antenna_solved = None;
                    }
                }
            });

            ui.horizontal(|ui| {
                ui.label("Optimize Yagi for");
                for obj in Objective::ALL {
                    let sel = self.antenna_objective == obj;
                    if ui.selectable_label(sel, obj.label()).clicked() {
                        self.antenna_objective = obj;
                    }
                }
            });
            #[cfg(not(target_arch = "wasm32"))]
            let optimizing = self.antenna_optimizing;
            #[cfg(target_arch = "wasm32")]
            let optimizing = false;
            if optimizing {
                ui.label(egui::RichText::new("Optimizing…").italics());
            } else if ui
                .button("Optimize Yagi")
                .on_hover_text(
                    "Tune the Yagi's element lengths and gaps against the chosen \
                     objective. A local hill-climb on a background thread; the \
                     window stays usable while it runs.",
                )
                .clicked()
            {
                self.antenna_optimize();
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

            if let Some((a, score)) = &self.antenna_optimized {
                ui.separator();
                crate::chrome::menu_caption(ui, "Optimized Yagi");
                ui.label(format!(
                    "{}: {}",
                    self.antenna_objective.label(),
                    objective_value(self.antenna_objective, *score)
                ));
                let mut y = 0.0_f64;
                for (i, w) in a.wires.iter().enumerate() {
                    let len = (w.to[0] - w.from[0]).abs();
                    let gap = if i == 0 { 0.0 } else { w.from[1] - y };
                    let label = match i {
                        0 => "reflector".to_string(),
                        1 => "driven".to_string(),
                        n => format!("director {}", n - 1),
                    };
                    ui.horizontal(|ui| {
                        ui.label(label);
                        ui.add_space(8.0);
                        ui.monospace(format!("{len:.2} m"));
                        if i > 0 {
                            ui.monospace(egui::RichText::new(format!("  gap {gap:.2} m")).weak());
                        }
                    });
                    y = w.from[1];
                }
            }

            // Obstacles: a coarse, labelled attenuation model (vegetation and
            // knife-edge), not a terrain solver.
            ui.separator();
            crate::chrome::menu_caption(ui, "Obstacles (approximate)");
            let mut obstacles: Vec<Obstacle> = Vec::new();
            ui.horizontal(|ui| {
                let mut on = self.antenna_trees.is_some();
                if ui.checkbox(&mut on, "Trees").changed() {
                    self.antenna_trees = if on { Some((0.0, 40.0, true)) } else { None };
                }
                if let Some((az, depth, leaf)) = &mut self.antenna_trees {
                    ui.add(egui::DragValue::new(az).speed(1.0).range(0.0..=359.0).prefix("az "));
                    ui.add(egui::DragValue::new(depth).speed(1.0).range(0.0..=400.0).prefix("depth "));
                    ui.checkbox(leaf, "in leaf");
                }
            });
            if let Some((az, depth, leaf)) = self.antenna_trees {
                obstacles.push(Obstacle::Vegetation {
                    az_deg: az,
                    width_deg: 30.0,
                    distance_m: 30.0,
                    depth_m: depth,
                    in_leaf: leaf,
                });
            }
            ui.horizontal(|ui| {
                let mut on = self.antenna_ridge.is_some();
                if ui.checkbox(&mut on, "Ridge").changed() {
                    self.antenna_ridge = if on { Some((0.0, 50.0)) } else { None };
                }
                if let Some((az, height)) = &mut self.antenna_ridge {
                    ui.add(egui::DragValue::new(az).speed(1.0).range(0.0..=359.0).prefix("az "));
                    ui.add(egui::DragValue::new(height).speed(1.0).range(0.0..=500.0).prefix("height "));
                }
            });
            if let Some((az, height)) = self.antenna_ridge {
                obstacles.push(Obstacle::KnifeEdge {
                    az_deg: az,
                    width_deg: 15.0,
                    distance_m: 100.0,
                    height_m: height,
                });
            }
            if !obstacles.is_empty() {
                let att = attenuation_db(&obstacles, 0.0, self.antenna_freq_mhz * 1e6);
                ui.label(format!("attenuation on the 0° bearing: {att:.1} dB"));
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
        // A stacked Yagi, when asked for — the array's combined gain.
        if self.antenna_stack > 1 {
            let single = antenna_model::presets::yagi(freq_hz, 3, antenna_model::presets::DEFAULT_HEIGHT_M);
            let stacked = antenna_model::transform::stack(
                &single,
                self.antenna_stack as usize,
                0.5 * antenna_model::wavelength_m(freq_hz),
                true,
            );
            if let Ok(sol) = analyze(&stacked, &SolverConfig::default()) {
                let (g, _, _) = max_gain(&sol);
                rows.push(SolvedRow {
                    name: format!("Yagi ×{}", self.antenna_stack),
                    r: sol.zin.re,
                    x: sol.zin.im,
                    swr: swr(sol.zin, 50.0),
                    gain: g,
                });
            }
        }
        self.antenna_solved = Some(rows);
    }

    /// Optimize the 3-element Yagi against the chosen objective, at the dense
    /// solver setting (the coarse one misleads the far field into spurious
    /// optima). Native runs it on a background thread; the browser has no
    /// threads and runs it inline.
    fn antenna_optimize(&mut self) {
        use antenna_model::optimize::optimize_yagi;
        use antenna_model::solver::SolverConfig;
        let freq_hz = self.antenna_freq_mhz * 1e6;
        let obj = self.antenna_objective;
        let cfg = SolverConfig { segs_per_wavelength: 80.0 };
        #[cfg(not(target_arch = "wasm32"))]
        {
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let result =
                    optimize_yagi(freq_hz, antenna_model::presets::DEFAULT_HEIGHT_M, 3, obj, &cfg);
                let _ = tx.send(result);
            });
            self.antenna_optimizing = true;
            self.antenna_optimize_rx = Some(rx);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.antenna_optimized =
                optimize_yagi(freq_hz, antenna_model::presets::DEFAULT_HEIGHT_M, 3, obj, &cfg);
        }
    }
}
