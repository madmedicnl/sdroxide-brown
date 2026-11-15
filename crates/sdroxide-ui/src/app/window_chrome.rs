//! Shared controls for native child windows and embedded browser panels.
use crate::i18n::{Tr, tr};
use eframe::egui;
use sdroxide_types::{WindowMode, WindowPrefs, WindowStack};

#[cfg(any(not(target_arch = "wasm32"), test))]
pub(crate) fn stack(level: WindowStack) -> egui::WindowLevel {
    match level {
        WindowStack::Bottom => egui::WindowLevel::AlwaysOnBottom,
        WindowStack::Normal => egui::WindowLevel::Normal,
        WindowStack::Top => egui::WindowLevel::AlwaysOnTop,
    }
}

pub(super) fn order(level: WindowStack) -> egui::Order {
    match level {
        WindowStack::Bottom => egui::Order::Background,
        WindowStack::Normal => egui::Order::Middle,
        WindowStack::Top => egui::Order::Foreground,
    }
}

pub(super) fn level_picker(ui: &mut egui::Ui, prefs: &mut WindowPrefs, native: bool) {
    let supported = !native
        || ui
            .ctx()
            .data(|d| d.get_temp::<bool>(egui::Id::new("window-stack-supported")))
            .unwrap_or(true);
    ui.add_enabled_ui(supported, |ui| {
        let levels = if ui.layout().main_dir() == egui::Direction::RightToLeft {
            [WindowStack::Top, WindowStack::Normal, WindowStack::Bottom]
        } else {
            [WindowStack::Bottom, WindowStack::Normal, WindowStack::Top]
        };
        for level in levels {
            ui.selectable_value(&mut prefs.level, level, level.tr());
        }
    })
    .response
    .on_disabled_hover_text(tr!("window-wayland-hint"))
    .on_hover_text(if supported {
        tr!("window-level-hint")
    } else {
        tr!("window-wayland-hint")
    });
}

fn icon_button(
    ui: &mut egui::Ui,
    icon: crate::chrome::WindowIcon,
    action: &str,
    label: String,
) -> egui::Response {
    let response = ui
        .push_id(action, |ui| {
            crate::chrome::chip_window_icon_sized(ui, icon, egui::Vec2::splat(22.))
        })
        .inner;
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &label)
    });
    #[cfg(test)]
    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(("window-control", action)), response.rect));
    response.on_hover_text(label)
}

/// Returns close. Minimized web panels retain this bar, including Restore.
pub(crate) fn controls(ui: &mut egui::Ui, prefs: &mut WindowPrefs, native: bool) -> bool {
    let mut close = false;
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        close = icon_button(ui, crate::chrome::WindowIcon::Close, "close", tr!("window-close"))
            .clicked();
        let restored = prefs.mode == WindowMode::Normal;
        if icon_button(
            ui,
            if restored {
                crate::chrome::WindowIcon::Maximize
            } else {
                crate::chrome::WindowIcon::Restore
            },
            if restored { "maximize" } else { "restore" },
            if restored { tr!("window-maximize") } else { tr!("window-restore") },
        )
        .clicked()
        {
            prefs.mode = if restored { WindowMode::Maximized } else { WindowMode::Normal };
            if native {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(
                    prefs.mode == WindowMode::Maximized,
                ));
            }
        }
        if icon_button(ui, crate::chrome::WindowIcon::Minimize, "minimize", tr!("window-minimize"))
            .clicked()
        {
            prefs.mode = WindowMode::Minimized;
            if native {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }
        }
        ui.push_id("level", |ui| level_picker(ui, prefs, native));
    });
    close
}

impl super::SdroxideApp {
    pub(super) fn remember_window_prefs(&mut self, id: &str, prefs: WindowPrefs) {
        let previous = self.ui_settings.window_layouts.get(id);
        if previous != prefs {
            self.ui_settings.window_layouts.set(id, prefs);
            super::persist::persist_ui_settings(&self.ui_settings);
            if (previous.level, previous.mode, previous.undocked, previous.closed)
                != (prefs.level, prefs.mode, prefs.undocked, prefs.closed)
            {
                self.flush_window_controls();
            }
        }
    }

    /// Discrete actions are sent immediately; drag geometry uses the existing debounce.
    pub(super) fn flush_window_controls(&mut self) {
        if !self.focused || self.client_settings_from.is_none() {
            return;
        }
        let changed = self.client_settings_stored.as_ref().is_none_or(|stored| {
            if stored.tool_windows_open != self.ui_settings.tool_windows_open
                || stored.detached != self.ui_settings.detached
            {
                return true;
            }
            let previous = sdroxide_types::WindowLayouts::from_entries(&stored.window_layouts);
            sdroxide_types::WINDOW_IDS.iter().any(|id| {
                let a = previous.get(id);
                let b = self.ui_settings.window_layouts.get(id);
                (a.level, a.mode, a.undocked, a.closed) != (b.level, b.mode, b.undocked, b.closed)
            })
        });
        if changed {
            self.save_screen_to_profile_silent();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn root_window_prefs(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        use raw_window_handle::{HasDisplayHandle, RawDisplayHandle};
        let supported = frame
            .display_handle()
            .is_ok_and(|h| !matches!(h.as_raw(), RawDisplayHandle::Wayland(_)));
        ctx.data_mut(|d| d.insert_temp(egui::Id::new("window-stack-supported"), supported));
        if !self.focused {
            return;
        }
        let mut prefs = self.ui_settings.window_layouts.get("main");
        let key = egui::Id::new("root-window-prefs");
        let previous = ctx.data(|d| d.get_temp::<WindowPrefs>(key));
        if previous.map(|p| p.level) != Some(prefs.level) && supported {
            ctx.send_viewport_cmd_to(
                egui::ViewportId::ROOT,
                egui::ViewportCommand::WindowLevel(stack(prefs.level)),
            );
        }
        if previous.is_none()
            || previous.is_some_and(|p| p.mode != prefs.mode || p.window != prefs.window)
        {
            if let Some(g) = prefs.window {
                ctx.send_viewport_cmd_to(
                    egui::ViewportId::ROOT,
                    egui::ViewportCommand::InnerSize(egui::vec2(g.size[0], g.size[1])),
                );
                if let Some(p) = g.pos {
                    ctx.send_viewport_cmd_to(
                        egui::ViewportId::ROOT,
                        egui::ViewportCommand::OuterPosition(egui::pos2(p[0], p[1])),
                    );
                }
            }
            ctx.send_viewport_cmd_to(
                egui::ViewportId::ROOT,
                egui::ViewportCommand::Maximized(prefs.mode == WindowMode::Maximized),
            );
            ctx.send_viewport_cmd_to(
                egui::ViewportId::ROOT,
                egui::ViewportCommand::Minimized(prefs.mode == WindowMode::Minimized),
            );
        } else {
            let (min, max, rect, outer) = ctx.input(|i| {
                let v = i.viewport();
                (v.minimized, v.maximized, v.inner_rect, v.outer_rect)
            });
            prefs.mode = if min == Some(true) {
                WindowMode::Minimized
            } else if max == Some(true) {
                WindowMode::Maximized
            } else {
                WindowMode::Normal
            };
            if prefs.mode == WindowMode::Normal
                && let Some(rect) = rect
            {
                let geometry = sdroxide_types::DetachedWindow {
                    size: [rect.width().round(), rect.height().round()],
                    pos: outer.map(|r| [r.left().round(), r.top().round()]),
                };
                let settle = key.with("geometry");
                if ctx.data(|d| d.get_temp::<sdroxide_types::DetachedWindow>(settle))
                    == Some(geometry)
                {
                    prefs.window = Some(geometry);
                }
                ctx.data_mut(|d| d.insert_temp(settle, geometry));
            }
        }
        ctx.data_mut(|d| d.insert_temp(key, prefs));
        self.remember_window_prefs("main", prefs);
    }
}

/// Embedded counterpart of a native viewport; the server supplies initial geometry.
pub(super) fn embedded(
    ctx: &egui::Context,
    id: egui::Id,
    title: &str,
    default_size: [f32; 2],
    min_size: [f32; 2],
    mut prefs: WindowPrefs,
    draw: impl FnOnce(&mut egui::Ui),
) -> (bool, WindowPrefs, Option<egui::Rect>) {
    let screen = ctx.content_rect();
    let normal = prefs.mode == WindowMode::Normal;
    let seed = prefs.embedded_window;
    let generation = ctx
        .data(|d| {
            d.get_temp::<u64>(egui::Id::new((
                "server-window-generation",
                crate::layout::radio_salt(ctx),
            )))
        })
        .unwrap_or(0);
    let win_id = id.with(prefs.mode as u8).with(generation);
    let init = win_id.with("server-seeded");
    let first = !ctx.data(|d| d.get_temp::<bool>(init)).unwrap_or(false);
    let mut window = egui::Window::new(title)
        .id(win_id)
        .title_bar(false)
        .order(order(prefs.level))
        .resizable(normal)
        .frame(crate::chrome::window_frame())
        .default_size(egui::vec2(default_size[0], default_size[1]))
        .min_size(egui::vec2(min_size[0], min_size[1]))
        .max_size(screen.size() * 0.98);
    if prefs.mode == WindowMode::Maximized {
        window = window
            .fixed_pos(screen.min + egui::vec2(4., 4.))
            .fixed_size(screen.size() - egui::vec2(8., 8.));
    } else if prefs.mode == WindowMode::Minimized {
        window = window
            .fixed_size(egui::vec2(default_size[0].min(screen.width() * 0.95).max(300.), 28.));
        if let Some(pos) = seed.and_then(|g| g.pos) {
            window = window.default_pos(egui::pos2(pos[0], pos[1]));
        }
    } else if first && let Some(g) = seed {
        let size =
            egui::vec2(g.size[0], g.size[1]).max(egui::vec2(100., 40.)).min(screen.size() * 0.95);
        let p = g.pos.unwrap_or([screen.left(), screen.top()]);
        let pos = egui::pos2(
            p[0].clamp(screen.left(), (screen.right() - size.x).max(screen.left())),
            p[1].clamp(screen.top(), (screen.bottom() - 40.).max(screen.top())),
        );
        window = window.current_pos(pos).fixed_size(size);
    }
    let mut close = false;
    let response = window.show(ctx, |ui| {
        if prefs.mode != WindowMode::Minimized {
            ui.set_min_size(ui.available_size());
        }
        ui.horizontal(|ui| {
            ui.label(title);
            close = controls(ui, &mut prefs, false);
        });
        if (normal || prefs.mode == WindowMode::Maximized) && !close {
            draw(ui);
        }
        ui.min_rect().size()
    });
    ctx.data_mut(|d| d.insert_temp(init, true));
    let rect = response.as_ref().map(|r| r.response.rect);
    if normal && let Some(rect) = rect {
        let size = rect.size();
        let g = sdroxide_types::DetachedWindow {
            size: [size.x.round(), size.y.round()],
            pos: Some([rect.left().round(), rect.top().round()]),
        };
        let key = id.with("settled-geometry");
        if ctx.data(|d| d.get_temp::<sdroxide_types::DetachedWindow>(key)) == Some(g) {
            prefs.embedded_window = Some(g);
        }
        ctx.data_mut(|d| d.insert_temp(key, g));
    }
    (close, prefs, rect)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn pass(
        ctx: &egui::Context,
        prefs: &mut WindowPrefs,
        events: Vec<egui::Event>,
    ) -> (bool, bool, egui::FullOutput) {
        let drawn = Cell::new(false);
        let closed = Cell::new(false);
        let out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1000., 700.),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                let result = embedded(
                    ui.ctx(),
                    egui::Id::new("window-test"),
                    "WEFAX",
                    [560., 320.],
                    [300., 40.],
                    *prefs,
                    |ui| {
                        drawn.set(true);
                        ui.label("Window content");
                        ui.allocate_space(egui::vec2(420., 200.));
                    },
                );
                closed.set(result.0);
                *prefs = result.1;
            },
        );
        (closed.get(), drawn.get(), out)
    }
    fn control(ctx: &egui::Context, action: &str) -> egui::Pos2 {
        ctx.data(|d| d.get_temp::<egui::Rect>(egui::Id::new(("window-control", action))))
            .expect("visible control")
            .center()
    }
    fn click(
        ctx: &egui::Context,
        prefs: &mut WindowPrefs,
        pos: egui::Pos2,
    ) -> (bool, bool, egui::FullOutput) {
        let event = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        let (_, _, first) = pass(ctx, prefs, vec![egui::Event::PointerMoved(pos), event(true)]);
        first.drop_without_applying_deltas();
        pass(ctx, prefs, vec![event(false)])
    }
    #[test]
    fn window_chrome_controls_reduce_restore_maximize_and_close() {
        let _guard = crate::multi::frame_test_lock();
        let ctx = egui::Context::default();
        let mut prefs = WindowPrefs::default();
        for _ in 0..3 {
            pass(&ctx, &mut prefs, vec![]).2.drop_without_applying_deltas();
        }
        let (_, _, out) = pass(&ctx, &mut prefs, vec![]);
        let minimize = control(&ctx, "minimize");
        out.drop_without_applying_deltas();
        click(&ctx, &mut prefs, minimize).2.drop_without_applying_deltas();
        assert_eq!(prefs.mode, WindowMode::Minimized);
        for _ in 0..3 {
            pass(&ctx, &mut prefs, vec![]).2.drop_without_applying_deltas();
        }
        let (_, drawn, out) = pass(&ctx, &mut prefs, vec![]);
        assert!(!drawn, "a minimized panel must leave only its controls");
        let restore = control(&ctx, "restore");
        out.drop_without_applying_deltas();
        click(&ctx, &mut prefs, restore).2.drop_without_applying_deltas();
        assert_eq!(prefs.mode, WindowMode::Normal);
        for _ in 0..3 {
            pass(&ctx, &mut prefs, vec![]).2.drop_without_applying_deltas();
        }
        let (_, drawn, out) = pass(&ctx, &mut prefs, vec![]);
        assert!(drawn);
        let maximize = control(&ctx, "maximize");
        out.drop_without_applying_deltas();
        click(&ctx, &mut prefs, maximize).2.drop_without_applying_deltas();
        assert_eq!(prefs.mode, WindowMode::Maximized);
        for _ in 0..3 {
            pass(&ctx, &mut prefs, vec![]).2.drop_without_applying_deltas();
        }
        let (_, _, out) = pass(&ctx, &mut prefs, vec![]);
        assert!(out.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Rect(r) if r.rect.width() > 900. && r.rect.height() > 650.)), "maximized must fill the page, even with short content");
        let close = control(&ctx, "close");
        out.drop_without_applying_deltas();
        let (closed, _, out) = click(&ctx, &mut prefs, close);
        out.drop_without_applying_deltas();
        assert!(closed, "close must report to the owner without stopping a radio");
    }

    #[test]
    fn window_chrome_minimizing_keeps_normal_geometry_and_levels() {
        let _guard = crate::multi::frame_test_lock();
        let ctx = egui::Context::default();
        let saved = sdroxide_types::DetachedWindow { size: [560., 320.], pos: Some([45., 60.]) };
        let mut prefs = WindowPrefs {
            mode: WindowMode::Minimized,
            level: WindowStack::Top,
            embedded_window: Some(saved),
            ..Default::default()
        };
        for _ in 0..3 {
            pass(&ctx, &mut prefs, vec![]).2.drop_without_applying_deltas();
        }
        assert_eq!(prefs.embedded_window, Some(saved));
        assert_eq!(prefs.level, WindowStack::Top);
        assert_eq!(order(WindowStack::Top), egui::Order::Foreground);
        assert_eq!(stack(WindowStack::Bottom), egui::WindowLevel::AlwaysOnBottom);
    }
    /// Optional visual QA export of the actual egui meshes and font atlas.
    #[test]
    fn window_chrome_visual_preview() {
        let Ok(folder) = std::env::var("SDROXIDE_WINDOW_QA") else {
            return;
        };
        let _guard = crate::multi::frame_test_lock();
        std::fs::create_dir_all(&folder).unwrap();
        for (name, mode) in [
            ("normal", WindowMode::Normal),
            ("minimized", WindowMode::Minimized),
            ("maximized", WindowMode::Maximized),
        ] {
            let ctx = egui::Context::default();
            let mut prefs = WindowPrefs { mode, ..Default::default() };
            let mut atlas: Option<egui::ColorImage> = None;
            for frame in 0..6 {
                let (_, _, out) = pass(&ctx, &mut prefs, vec![]);
                for (id, deltas) in &out.textures_delta.set {
                    if *id != egui::TextureId::Managed(0) {
                        continue;
                    }
                    for delta in deltas {
                        let egui::ImageData::Color(image) = &delta.image;
                        if let Some([x, y]) = delta.pos {
                            let atlas = atlas.as_mut().unwrap();
                            for row in 0..image.height() {
                                let at = (y + row) * atlas.width() + x;
                                atlas.pixels[at..at + image.width()].copy_from_slice(
                                    &image.pixels[row * image.width()..(row + 1) * image.width()],
                                );
                            }
                        } else {
                            atlas = Some((**image).clone());
                        }
                    }
                }
                if frame == 5 {
                    let atlas = atlas.as_ref().unwrap();
                    let rgba: Vec<u8> = atlas.pixels.iter().flat_map(|c| c.to_array()).collect();
                    let meshes: Vec<_> = ctx.tessellate(out.shapes.clone(), out.pixels_per_point).iter().filter_map(|p| {
                        let egui::epaint::Primitive::Mesh(mesh) = &p.primitive else { return None; };
                        Some(serde_json::json!({ "clip":[p.clip_rect.min.x,p.clip_rect.min.y,p.clip_rect.max.x,p.clip_rect.max.y],
                            "indices":mesh.indices, "vertices":mesh.vertices.iter().map(|v| [v.pos.x,v.pos.y,v.uv.x,v.uv.y,v.color.r() as f32,v.color.g() as f32,v.color.b() as f32,v.color.a() as f32]).collect::<Vec<_>>() }))
                    }).collect();
                    std::fs::write(format!("{folder}/{name}.rgba"), rgba).unwrap();
                    std::fs::write(
                        format!("{folder}/{name}.json"),
                        serde_json::to_vec(
                            &serde_json::json!({"atlas":atlas.size,"meshes":meshes}),
                        )
                        .unwrap(),
                    )
                    .unwrap();
                }
                out.drop_without_applying_deltas();
            }
        }
    }
}
