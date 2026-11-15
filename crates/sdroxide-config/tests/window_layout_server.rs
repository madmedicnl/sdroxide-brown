//! A new browser recovers its arrangement from disk on the server, not localStorage.
use sdroxide_types::{
    ClientScreen, DetachableModule, DetachedWindow, UiSettings, WindowMode, WindowPrefs,
    WindowStack,
};

#[test]
fn a_fresh_browser_recovers_windows_after_a_server_restart() {
    let dir = std::env::temp_dir().join(format!("sdroxide-window-server-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // This integration binary contains one test, so no other test races this variable.
    unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) };
    let mut browser = UiSettings::default();
    let prefs = WindowPrefs {
        level: WindowStack::Top,
        mode: WindowMode::Minimized,
        embedded_window: Some(DetachedWindow { size: [640., 480.], pos: Some([123., 45.]) }),
        ..Default::default()
    };
    browser.window_layouts.set("wefax", prefs);
    browser.tool_windows_open = 42;
    browser.set_detached(DetachableModule::Panadapter, true);
    let mut server = sdroxide_config::ClientSettingsStore::default();
    server.set(Some("F4NRM"), ClientScreen::from_settings(&browser));
    server.set(None, ClientScreen::from_settings(&browser));
    sdroxide_config::save_client_settings(&server).unwrap();
    drop(server);
    drop(browser);
    let restarted = sdroxide_config::load_client_settings();
    for profile in [Some("F4NRM"), None] {
        let mut fresh: UiSettings = serde_json::from_str("{}").unwrap();
        restarted.for_profile(profile).unwrap().1.apply_to(&mut fresh);
        assert_eq!(fresh.window_layouts.get("wefax"), prefs);
        assert_eq!(fresh.window_layouts.get("dab"), WindowPrefs::default());
        assert_eq!(fresh.tool_windows_open, 42);
        assert!(fresh.is_detached(DetachableModule::Panadapter));
    }
    std::fs::remove_dir_all(dir).unwrap();
}
