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
    let base = ctx.input(|i| i.time);
    let mut last = Vec::new();
    for i in 1..6 {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 900.0),
            )),
            time: Some(base + f64::from(i) * 0.1),
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

/// Where a label was drawn, as the middle of its text — the lowest one on
/// screen, which is the panel's and not the top bar's.
fn centre_of(shapes: &[egui::epaint::ClippedShape], what: &str) -> Option<egui::Pos2> {
    text_rects(shapes)
        .into_iter()
        .filter(|(t, _)| t == what)
        .map(|(_, r)| r.center())
        .max_by(|a, b| a.y.total_cmp(&b.y))
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
    let base = ctx.input(|i| i.time);
    let shapes = one_frame(ctx, app, base + 1.0, vec![]);
    let Some(at) = centre_of(&shapes, what) else { return false };
    let button = egui::PointerButton::Primary;
    let mods = egui::Modifiers::NONE;
    one_frame(ctx, app, base + 1.1, vec![egui::Event::PointerMoved(at)]);
    one_frame(
        ctx,
        app,
        base + 1.2,
        vec![egui::Event::PointerButton { pos: at, button, pressed: true, modifiers: mods }],
    );
    one_frame(
        ctx,
        app,
        base + 1.3,
        vec![egui::Event::PointerButton { pos: at, button, pressed: false, modifiers: mods }],
    );
    one_frame(ctx, app, base + 1.4, vec![]);
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

/// Every text shape on screen, as `(text, rect)`.
fn text_rects(shapes: &[egui::epaint::ClippedShape]) -> Vec<(String, egui::Rect)> {
    fn walk(s: &egui::Shape, out: &mut Vec<(String, egui::Rect)>) {
        match s {
            egui::Shape::Text(t) => out.push((
                t.galley.text().to_string(),
                egui::Rect::from_min_size(t.pos, t.galley.size()),
            )),
            egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    shapes.iter().for_each(|c| walk(&c.shape, &mut out));
    out
}

/// The big frequency sits in the middle of its column: the hint line under it
/// is centred on the column, and the digits must share its centre.
#[test]
fn the_big_frequency_is_centred_in_its_column() {
    let _guard = crate::multi::frame_test_lock();
    let (ctx, mut app) = app(Mode::Wfm);
    app.ui_settings.radio_classic_on = true;
    run(&ctx, &mut app);
    let shapes = one_frame(&ctx, &mut app, 2.0, vec![]);
    let texts = text_rects(&shapes);
    let hint = texts
        .iter()
        .find(|(t, _)| t.starts_with("wheel on a digit"))
        .map(|(_, r)| *r)
        .expect("no hint line under the frequency");
    let column = hint.center().x;
    let digits: Vec<egui::Rect> = texts
        .iter()
        .filter(|(t, r)| {
            (t.chars().count() == 1 && t.chars().all(|c| c.is_ascii_digit() || c == '.')
                || t.trim() == "Hz")
                && r.bottom() <= hint.top() + 2.0
                && r.top() > hint.top() - 90.0
                && (r.center().x - column).abs() < 450.0
                && r.height() > 14.0
        })
        .map(|(_, r)| *r)
        .collect();
    assert!(digits.len() >= 9, "found {} digits", digits.len());
    let span = digits.iter().fold(digits[0], |a, r| a.union(*r));
    let off = span.center().x - hint.center().x;
    assert!(off.abs() <= 1.5, "the frequency is {off} px off the column's centre {column}");
}

/// The Wideband row of the band menu, drawn on its own.
fn wideband_row(
    app: &mut SdroxideApp,
    ctx: &egui::Context,
    radio: &mut super::top_bar::RadioChip,
) -> Vec<String> {
    let mut show_ism = false;
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0))),
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| {
        super::top_bar::wideband_chip_grid(
            ui,
            Mode::Wfm,
            sdroxide_types::Band::Gen,
            &app.state,
            false,
            &mut show_ism,
            radio,
            &mut Vec::new(),
        );
    });
    out.textures_delta.clear();
    texts(&out.shapes)
}

/// WIDEBAND has a RADIO chip to the right of ISM, and it is lit while RADIO
/// CLASSIC is on — the same state EXTRAS' chip reads.
#[test]
fn the_wideband_row_has_a_radio_chip_after_ism() {
    let _guard = crate::multi::frame_test_lock();
    let (ctx, mut app) = app(Mode::Wfm);
    let mut radio = super::top_bar::RadioChip::new(app.radio_classic_active());
    let row = wideband_row(&mut app, &ctx, &mut radio);
    let ism = row.iter().position(|t| t == "ISM").expect("no ISM chip");
    assert_eq!(row.get(ism + 1).map(String::as_str), Some("RADIO"), "{row:?}");
}

/// EXTRAS' chip and the Wideband row's flip the same switch.
#[test]
fn both_radio_chips_flip_the_same_switch() {
    let _guard = crate::multi::frame_test_lock();
    let (_ctx, mut app) = app(Mode::Wfm);
    assert!(!app.radio_classic_active());
    app.toggle_radio_classic();
    assert!(app.radio_classic_active() && app.ui_settings.radio_classic_on);
    app.toggle_radio_classic();
    assert!(!app.radio_classic_active());
}

fn spectrum(seq: u32) -> std::sync::Arc<sdroxide_types::SpectrumFrame> {
    std::sync::Arc::new(sdroxide_types::SpectrumFrame {
        seq,
        center_hz: 100e6,
        span_hz: 2e6,
        db_floor: -120.0,
        db_ceil: 0.0,
        bins: (0..512).map(|i| (i % 256) as u8).collect(),
        rows: Vec::new(),
        rows_clocked: false,
    })
}

/// Feed the panel `n` spectrum frames, one per screen frame, and say how many
/// waterfall rows it kept.
fn rows_after(n: u32) -> usize {
    let (ctx, mut app) = app(Mode::Wfm);
    app.ui_settings.radio_classic_on = true;
    run(&ctx, &mut app);
    app.radio_classic.wf.clear();
    for i in 0..n {
        app.frame = Some(spectrum(1000 + i));
        one_frame(&ctx, &mut app, 10.0 + f64::from(i) * 0.05, vec![]);
    }
    app.radio_classic.wf.len()
}

/// The little waterfall scrolls at half the speed it did — one row for every
/// two spectrum frames — and keeps ten times as many rows.
#[test]
fn the_scope_waterfall_runs_at_half_speed_and_keeps_ten_times_the_rows() {
    let _guard = crate::multi::frame_test_lock();
    assert_eq!(rows_after(20), 10, "twenty frames should leave ten rows");
    assert_eq!(rows_after(1200), 400, "the waterfall keeps 400 rows");
}

type Sent = std::sync::Arc<std::sync::Mutex<Vec<sdroxide_types::Command>>>;

/// A panel on, a recorder behind it, and the receiver set to `mode`.
fn recorded(mode: Mode) -> (egui::Context, SdroxideApp, Sent) {
    let sent: Sent = std::sync::Arc::default();
    let (ctx, mut app) = app(mode);
    app.ctrl = Box::new(Recorder(sent.clone()));
    app.ui_settings.radio_classic_on = true;
    (ctx, app, sent)
}

/// The four boxes under the S-meter act: BW takes the next filter of the mode.
#[test]
fn the_bw_box_takes_the_next_filter_of_the_mode() {
    let _guard = crate::multi::frame_test_lock();
    let (ctx, mut app, sent) = recorded(Mode::Usb);
    app.state.rx[0].filter_lo = 200.0;
    app.state.rx[0].filter_hi = 2600.0; // the 2.4k preset
    assert!(click(&ctx, &mut app, "BW"), "no BW box");
    let sent = sent.lock().unwrap();
    let got = sent.iter().find_map(|c| match c {
        sdroxide_types::Command::SetFilter { rx: sdroxide_types::RxId::Main, lo, hi } => {
            Some((*lo, *hi))
        }
        _ => None,
    });
    assert_eq!(got, Some((150.0, 2850.0)), "{sent:?}");
}

/// STEP takes the next tuning step, the one the top bar's step chip takes.
#[test]
fn the_step_box_takes_the_next_tuning_step() {
    let _guard = crate::multi::frame_test_lock();
    let (ctx, mut app, _sent) = recorded(Mode::Wfm);
    let before = app.ui_settings.tune_step_hz;
    let next = app.ui_settings.next_tune_step();
    assert_ne!(before, next);
    assert!(click(&ctx, &mut app, "STEP"), "no STEP box");
    assert_eq!(app.ui_settings.tune_step_hz, next);
}

/// AGC cycles Off → Slow → Med → Fast.
#[test]
fn the_agc_box_cycles_the_agc() {
    let _guard = crate::multi::frame_test_lock();
    let (ctx, mut app, sent) = recorded(Mode::Wfm);
    app.state.rx[0].agc = sdroxide_types::AgcMode::Med;
    assert!(click(&ctx, &mut app, "AGC"), "no AGC box");
    let sent = sent.lock().unwrap();
    assert!(
        sent.iter().any(|c| matches!(
            c,
            sdroxide_types::Command::SetAgc {
                rx: sdroxide_types::RxId::Main,
                agc: sdroxide_types::AgcMode::Fast
            }
        )),
        "{sent:?}"
    );
}

/// SQL: a click closes the gate on the last threshold, the next one opens it.
#[test]
fn the_sql_box_toggles_the_squelch_between_open_and_the_last_threshold() {
    let _guard = crate::multi::frame_test_lock();
    let (ctx, mut app, sent) = recorded(Mode::Wfm);
    app.state.rx[0].squelch_db = sdroxide_types::SQUELCH_OPEN_DB;
    assert!(click(&ctx, &mut app, "SQL"), "no SQL box");
    let db = |sent: &Sent| {
        sent.lock().unwrap().iter().rev().find_map(|c| match c {
            sdroxide_types::Command::SetSquelch { rx: sdroxide_types::RxId::Main, db } => Some(*db),
            _ => None,
        })
    };
    let set = db(&sent).expect("the click set no squelch");
    assert!(set > sdroxide_types::SQUELCH_OPEN_DB + 1.0, "still open: {set}");
    assert!(click(&ctx, &mut app, "SQL"));
    assert_eq!(db(&sent), Some(sdroxide_types::SQUELCH_OPEN_DB), "the second click did not open");
}
