//! Radio Classic through a whole app: docked under the waterfall when it is
//! switched on in a voice mode, in a window when the Detachable page says so or
//! when the slot is taken, and not there at all when it is off.

use eframe::App;
use eframe::egui;
use sdroxide_types::{Mode, RadioController, RadioEvent};

use super::SdroxideApp;
use super::frame::TW_RADIO_CLASSIC;

struct NoStation;

impl RadioController for NoStation {
    fn send(&mut self, _cmd: sdroxide_types::Command) {}
    fn poll_event(&mut self) -> Option<RadioEvent> {
        None
    }
}

fn texts(shapes: &[egui::epaint::ClippedShape]) -> Vec<String> {
    fn walk(s: &egui::Shape, out: &mut Vec<String>) {
        match s {
            egui::Shape::Text(t) => out.push(t.galley.text().to_string()),
            egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    shapes.iter().for_each(|c| walk(&c.shape, &mut out));
    out
}

fn app(mode: Mode) -> (egui::Context, SdroxideApp) {
    let dir = std::env::temp_dir().join(format!("sdroxide-rc-frame-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) };
    let ctx = egui::Context::default();
    let mut app = SdroxideApp::new_tab(&ctx, None, None, Box::new(NoStation), 0, true);
    app.state.rx[0].mode = mode;
    (ctx, app)
}

/// Run a few frames at 1600x900 and return the text drawn in the last one.
fn run(ctx: &egui::Context, app: &mut SdroxideApp) -> Vec<String> {
    let mut last = Vec::new();
    for i in 1..6 {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 900.0),
            )),
            time: Some(f64::from(i) * 0.1),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| app.ui(ui, &mut eframe::Frame::_new_kittest()));
        last = texts(&out.shapes);
        out.textures_delta.clear();
    }
    last
}

fn count(text: &[String], what: &str) -> usize {
    text.iter().filter(|l| l.as_str() == what).count()
}

#[test]
fn off_it_is_not_drawn() {
    let _guard = crate::multi::frame_test_lock();
    let (ctx, mut app) = app(Mode::Wfm);
    let text = run(&ctx, &mut app);
    assert_eq!(count(&text, "RADIO CLASSIC"), 0, "{text:?}");
}

#[test]
fn switched_on_in_a_voice_mode_it_docks_under_the_waterfall() {
    let _guard = crate::multi::frame_test_lock();
    let (ctx, mut app) = app(Mode::Wfm);
    app.ui_settings.radio_classic_on = true;
    let text = run(&ctx, &mut app);
    // The panel's own heading, once: docked, so no window title beside it.
    assert_eq!(count(&text, "RADIO CLASSIC"), 1, "{text:?}");
    assert!(count(&text, "BROADCAST") == 1 && count(&text, "WFM") >= 1, "{text:?}");
    assert!(!app.show_radio_classic, "docking must not open the window");
}

#[test]
fn the_detachable_pages_switch_opens_it_as_a_window_and_keeps_its_bit() {
    let _guard = crate::multi::frame_test_lock();
    let (ctx, mut app) = app(Mode::Wfm);
    app.ui_settings.radio_classic_on = true;
    app.ui_settings.tool_windows_open |= TW_RADIO_CLASSIC;
    let text = run(&ctx, &mut app);
    assert!(app.show_radio_classic, "the switch did not reach the window flag");
    assert_ne!(app.ui_settings.tool_windows_open & TW_RADIO_CLASSIC, 0, "the bit was lost");
    // Window title and panel heading, and not a second copy docked under the
    // waterfall.
    assert_eq!(count(&text, "RADIO CLASSIC"), 2, "{text:?}");
}

#[test]
fn where_a_panel_owns_the_slot_it_opens_as_a_window_instead_of_doing_nothing() {
    let _guard = crate::multi::frame_test_lock();
    for mode in [Mode::Ft8, Mode::Cw] {
        let (ctx, mut app) = app(mode);
        app.ui_settings.radio_classic_on = true;
        let text = run(&ctx, &mut app);
        assert_eq!(count(&text, "RADIO CLASSIC"), 2, "{mode:?}: {text:?}");
        assert!(!app.show_radio_classic, "{mode:?}: the window is forced, not detached");
    }
}

struct Recorder(std::sync::Arc<std::sync::Mutex<Vec<sdroxide_types::Command>>>);

impl RadioController for Recorder {
    fn send(&mut self, cmd: sdroxide_types::Command) {
        self.0.lock().unwrap().push(cmd);
    }
    fn poll_event(&mut self) -> Option<RadioEvent> {
        None
    }
}

fn wants(sent: &std::sync::Mutex<Vec<sdroxide_types::Command>>) -> usize {
    let sent = sent.lock().unwrap();
    sent.iter().filter(|c| matches!(c, sdroxide_types::Command::WantAudioLevel)).count()
}

/// The station sends the level only to a screen that asked for it: closed, the
/// panel asks for nothing; open, it asks once and renews, not every frame.
#[test]
fn the_panel_asks_for_the_audio_level_only_while_it_is_open() {
    let _guard = crate::multi::frame_test_lock();
    let sent = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let (ctx, mut app) = app(Mode::Wfm);
    app.ctrl = Box::new(Recorder(sent.clone()));
    run(&ctx, &mut app);
    assert_eq!(wants(&sent), 0, "a closed panel must not ask for the stream");
    app.ui_settings.radio_classic_on = true;
    run(&ctx, &mut app);
    assert_eq!(wants(&sent), 1, "five frames in well under a second: one lease");
}

/// Where a label was drawn, as the middle of its text.
fn centre_of(shapes: &[egui::epaint::ClippedShape], what: &str) -> Option<egui::Pos2> {
    fn walk(s: &egui::Shape, what: &str) -> Option<egui::Pos2> {
        match s {
            egui::Shape::Text(t) if t.galley.text() == what => {
                Some(t.pos + t.galley.size() / 2.0)
            }
            egui::Shape::Vec(v) => v.iter().find_map(|s| walk(s, what)),
            _ => None,
        }
    }
    shapes.iter().find_map(|c| walk(&c.shape, what))
}

fn one_frame(
    ctx: &egui::Context,
    app: &mut SdroxideApp,
    time: f64,
    events: Vec<egui::Event>,
) -> Vec<egui::epaint::ClippedShape> {
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 900.0))),
        time: Some(time),
        events,
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| app.ui(ui, &mut eframe::Frame::_new_kittest()));
    out.textures_delta.clear();
    out.shapes
}

/// Settle the layout, then click the middle of the first label that reads
/// `what`; returns false when no such label is on screen.
fn click(ctx: &egui::Context, app: &mut SdroxideApp, what: &str) -> bool {
    run(ctx, app);
    let shapes = one_frame(ctx, app, 1.0, vec![]);
    let Some(at) = centre_of(&shapes, what) else { return false };
    let button = egui::PointerButton::Primary;
    let mods = egui::Modifiers::NONE;
    one_frame(ctx, app, 1.1, vec![egui::Event::PointerMoved(at)]);
    one_frame(
        ctx,
        app,
        1.2,
        vec![egui::Event::PointerButton { pos: at, button, pressed: true, modifiers: mods }],
    );
    one_frame(
        ctx,
        app,
        1.3,
        vec![egui::Event::PointerButton { pos: at, button, pressed: false, modifiers: mods }],
    );
    one_frame(ctx, app, 1.4, vec![]);
    true
}

/// PROGRAMS opens the PROGRAMS window — the one the top bar opens — not a
/// little list of its own.
#[test]
fn the_programs_chip_opens_the_programs_window() {
    let _guard = crate::multi::frame_test_lock();
    let (ctx, mut app) = app(Mode::Wfm);
    app.ui_settings.radio_classic_on = true;
    assert!(!app.schedule.show);
    assert!(click(&ctx, &mut app, "PROGRAMS"), "no PROGRAMS chip on the panel");
    assert!(app.schedule.show, "the click did not open the PROGRAMS window");
}
