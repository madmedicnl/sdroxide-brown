//! The antenna calculator — a frequency in, dimensions and SWR out — for the
//! shapes the closed-form calculator knows (dipole, inverted-V, vertical,
//! ground plane, loop, Yagi), an on-demand method-of-moments solve of the
//! presets, and a Yagi optimizer. It lives in the EXTRAS launcher.

use crate::i18n::tr;
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
    /// The takeoff angle — the elevation of the peak gain, above the horizon.
    pub(in crate::app) takeoff: f64,
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

/// Draw an azimuth cut (azimuth in degrees, gain in dBi) as a polar field
/// pattern: the radius is the field, normalised to the peak.
fn pattern_plot(ui: &mut egui::Ui, name: &str, cut: &[(f64, f64)]) {
    let size = egui::vec2(240.0, 240.0);
    let (response, painter) = ui.allocate_painter(size, egui::Sense::hover());
    let rect = response.rect;
    let center = rect.center();
    let radius = rect.width().min(rect.height()) / 2.0 - 10.0;
    painter.circle_stroke(
        center,
        radius,
        egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
    );
    let max_g = cut.iter().map(|&(_, g)| g).fold(f64::NEG_INFINITY, f64::max);
    let mut points: Vec<egui::Pos2> = cut
        .iter()
        .map(|&(az, g)| {
            let a = az.to_radians();
            let r = 10f64.powf((g - max_g) / 20.0) * radius as f64;
            center + egui::vec2((a.sin() * r) as f32, (-a.cos() * r) as f32)
        })
        .collect();
    if let Some(first) = points.first().copied() {
        points.push(first);
    }
    painter.add(egui::Shape::line(
        points,
        egui::Stroke::new(1.5, egui::Color32::from_rgb(120, 200, 120)),
    ));
    painter.text(
        center + egui::vec2(0.0, radius + 6.0),
        egui::Align2::CENTER_TOP,
        format!("{name} — {max_g:.1} dBi"),
        egui::FontId::proportional(11.0),
        egui::Color32::from_gray(150),
    );
}

/// Draw the 3D pattern surface as a rotatable wireframe (drag to rotate), using
/// a software yaw/pitch projection. `z` is up.
fn surface_plot(
    ui: &mut egui::Ui,
    verts: &[[f64; 3]],
    nt: usize,
    np: usize,
    yaw: &mut f32,
    pitch: &mut f32,
) {
    let size = egui::vec2(240.0, 240.0);
    let (response, painter) = ui.allocate_painter(size, egui::Sense::drag());
    let rect = response.rect;
    let center = rect.center();
    let scale = rect.width().min(rect.height()) / 2.0 - 10.0;
    if response.dragged() {
        let d = response.drag_delta();
        *yaw += d.x * 0.01;
        *pitch = (*pitch + d.y * 0.01).clamp(-1.5, 1.5);
    }
    let (cy, sy) = (yaw.cos(), yaw.sin());
    let (cp, sp) = (pitch.cos(), pitch.sin());
    let proj = |v: [f64; 3]| -> egui::Pos2 {
        let (x, y, z) = (v[0] as f32, v[1] as f32, v[2] as f32);
        let (x1, y1) = (x * cy - y * sy, x * sy + y * cy); // yaw around z
        let y2 = y1 * cp - z * sp; // pitch around x
        center + egui::vec2(x1 * scale, -y2 * scale)
    };
    let stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(120, 200, 120));
    // φ-rings: for each theta, the closed ring over phi.
    for t in 0..nt {
        let ring: Vec<egui::Pos2> = (0..=np).map(|p| proj(verts[t * np + (p % np)])).collect();
        painter.add(egui::Shape::line(ring, stroke));
    }
    // θ-lines: for each phi, the meridian over theta.
    for p in 0..np {
        let meridian: Vec<egui::Pos2> = (0..nt).map(|t| proj(verts[t * np + p])).collect();
        painter.add(egui::Shape::line(meridian, stroke));
    }
}

impl super::SdroxideApp {
    pub(in crate::app) fn antenna_window(&mut self, ctx: &egui::Context) {
        if !self.show_antenna {
            return;
        }
        self.antenna_poll();
        let show = self.tool_window(ctx, "antenna", &tr!("antenna-antenna"), [420.0, 720.0], true, |me, ui| {
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
        crate::chrome::menu_caption(ui, &tr!("antenna-cutting-lengths"));
        ui.horizontal(|ui| {
            ui.label(tr!("speech-cat-frequency"));
            let mut mhz = self.antenna_freq_mhz;
            if ui
                .add(egui::DragValue::new(&mut mhz).speed(0.05).range(0.05..=3000.0))
                .on_hover_text(tr!("antenna-the-frequency-to-cut-for"))
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
                    .button(tr!("antenna-solve-impedance-and-gain"))
                    .on_hover_text(
                        tr!("antenna-run-the-method-of-moments-solver"),
                    )
                    .clicked()
                {
                    self.antenna_solve();
                }
                ui.label(tr!("antenna-yagi-stack"));
                for n in [1, 2, 4] {
                    let sel = self.antenna_stack == n;
                    if ui.selectable_label(sel, format!("{n}×")).clicked() {
                        self.antenna_stack = n;
                        self.antenna_solved = None;
                    }
                }
            });

            ui.horizontal(|ui| {
                ui.label(tr!("antenna-optimize-yagi-for"));
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
                ui.label(egui::RichText::new(tr!("antenna-optimizing")).italics());
            } else if ui
                .button(tr!("antenna-optimize-yagi"))
                .on_hover_text(
                    tr!("antenna-tune-the-yagi-s-element-lengths"),
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
                            ui.label(tr!("antenna-feed-ohms", r = format!("{r:.0}")));
                            ui.add_space(8.0);
                            ui.monospace(format!("SWR {:.2}", antenna_model::swr_50(r, x)));
                        });
                    }
                });
                ui.add_space(4.0);
            }

            if let Some(rows) = &self.antenna_solved {
                ui.separator();
                crate::chrome::menu_caption(ui, &tr!("antenna-solved"));
                for row in rows {
                    ui.group(|ui| {
                        ui.label(egui::RichText::new(&row.name).strong());
                        ui.horizontal(|ui| {
                            ui.label(format!("Zin {:.1}{:+.1}j Ω", row.r, row.x));
                            ui.add_space(8.0);
                            ui.monospace(format!("SWR {:.2}", row.swr));
                            ui.add_space(8.0);
                            ui.monospace(format!("{:.1} dBi", row.gain));
                            ui.add_space(8.0);
                            ui.monospace(
                                egui::RichText::new(tr!("antenna-takeoff", deg = format!("{:.0}", row.takeoff))).weak(),
                            );
                        });
                    });
                    ui.add_space(4.0);
                }
            }

            if !self.antenna_patterns.is_empty() {
                ui.separator();
                crate::chrome::menu_caption(ui, &tr!("antenna-azimuth-pattern"));
                let mut sel = self.antenna_pattern_sel;
                let names: Vec<&str> =
                    self.antenna_patterns.iter().map(|(n, _)| n.as_str()).collect();
                egui::ComboBox::from_id_salt("antenna-pattern-sel")
                    .selected_text(names[sel])
                    .show_ui(ui, |ui| {
                        for (i, name) in names.iter().enumerate() {
                            ui.selectable_value(&mut sel, i, *name);
                        }
                    });
                self.antenna_pattern_sel = sel;
                if let Some((name, cut)) = self.antenna_patterns.get(self.antenna_pattern_sel) {
                    pattern_plot(ui, name, cut);
                }
                if let Some((verts, nt, np)) = self.antenna_surfaces.get(self.antenna_pattern_sel) {
                    ui.small(tr!("antenna-3d-drag-to-rotate"));
                    surface_plot(ui, verts, *nt, *np, &mut self.antenna_yaw, &mut self.antenna_pitch);
                }
            }

            if let Some((a, score)) = &self.antenna_optimized {
                ui.separator();
                crate::chrome::menu_caption(ui, &tr!("antenna-optimized-yagi"));
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
                        0 => tr!("antenna-reflector"),
                        1 => tr!("antenna-driven"),
                        n => tr!("antenna-director", n = n - 1),
                    };
                    ui.horizontal(|ui| {
                        ui.label(label);
                        ui.add_space(8.0);
                        ui.monospace(format!("{len:.2} m"));
                        if i > 0 {
                            ui.monospace(egui::RichText::new(format!("  {}", tr!("antenna-gap", v = format!("{gap:.2}")))).weak());
                        }
                    });
                    y = w.from[1];
                }
            }

            // Obstacles: a coarse, labelled attenuation model (vegetation and
            // knife-edge), not a terrain solver.
            ui.separator();
            crate::chrome::menu_caption(ui, &tr!("antenna-obstacles-approximate"));
            let mut obstacles: Vec<Obstacle> = Vec::new();
            ui.horizontal(|ui| {
                let mut on = self.antenna_trees.is_some();
                if ui.checkbox(&mut on, tr!("antenna-trees")).changed() {
                    self.antenna_trees = if on { Some((0.0, 40.0, true)) } else { None };
                }
                if let Some((az, depth, leaf)) = &mut self.antenna_trees {
                    ui.add(egui::DragValue::new(az).speed(1.0).range(0.0..=359.0).prefix(format!("{} ", tr!("antenna-az"))));
                    ui.add(egui::DragValue::new(depth).speed(1.0).range(0.0..=400.0).prefix(format!("{} ", tr!("antenna-depth"))));
                    ui.checkbox(leaf, tr!("antenna-in-leaf"));
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
                if ui.checkbox(&mut on, tr!("antenna-ridge")).changed() {
                    self.antenna_ridge = if on { Some((0.0, 50.0)) } else { None };
                }
                if let Some((az, height)) = &mut self.antenna_ridge {
                    ui.add(egui::DragValue::new(az).speed(1.0).range(0.0..=359.0).prefix(format!("{} ", tr!("antenna-az"))));
                    ui.add(egui::DragValue::new(height).speed(1.0).range(0.0..=500.0).prefix(format!("{} ", tr!("antenna-height"))));
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
                ui.label(tr!("antenna-attenuation", att = format!("{att:.1}")));
            }
        });
    }

    /// Solve the presets at the current frequency (synchronously — the solver is
    /// a few seconds, which is the honest cost of the method of moments).
    fn antenna_solve(&mut self) {
        use antenna_model::solver::{SolverConfig, analyze, cut, max_gain, pattern_surface, swr};
        let freq_hz = self.antenna_freq_mhz * 1e6;
        let mut rows = Vec::new();
        let mut patterns = Vec::new();
        let mut surfaces = Vec::new();
        for a in antenna_model::presets::catalog(freq_hz) {
            if let Ok(sol) = analyze(&a, &SolverConfig::default()) {
                let (g, th, _) = max_gain(&sol);
                let name = a.name.clone();
                patterns.push((name.clone(), cut(&sol, true, 90.0, 5.0)));
                surfaces.push(pattern_surface(&sol, 37, 48));
                rows.push(SolvedRow { name, r: sol.zin.re, x: sol.zin.im, swr: swr(sol.zin, 50.0), gain: g, takeoff: 90.0 - th });
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
                let (g, th, _) = max_gain(&sol);
                let name = format!("Yagi ×{}", self.antenna_stack);
                patterns.push((name.clone(), cut(&sol, true, 90.0, 5.0)));
                surfaces.push(pattern_surface(&sol, 37, 48));
                rows.push(SolvedRow { name, r: sol.zin.re, x: sol.zin.im, swr: swr(sol.zin, 50.0), gain: g, takeoff: 90.0 - th });
            }
        }
        self.antenna_solved = Some(rows);
        self.antenna_patterns = patterns;
        self.antenna_surfaces = surfaces;
        self.antenna_pattern_sel = self.antenna_pattern_sel.min(self.antenna_patterns.len().saturating_sub(1));
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
