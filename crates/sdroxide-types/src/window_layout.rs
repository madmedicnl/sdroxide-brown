//! Window arrangements. Stable names, never enum ordinals, identify stored slots.
use crate::DetachedWindow;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowStack {
    Bottom,
    Top,
    #[default]
    #[serde(other)]
    Normal,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowMode {
    Minimized,
    Maximized,
    #[default]
    #[serde(other)]
    Normal,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowPrefs {
    pub level: WindowStack,
    pub mode: WindowMode,
    pub undocked: bool,
    pub closed: bool,
    pub window: Option<DetachedWindow>,
    /// Browser and native window coordinates are different coordinate spaces.
    pub embedded_window: Option<DetachedWindow>,
}

/// Append new names. Stored files and the wire use the names, not these indices.
pub const WINDOW_IDS: &[&str] = &[
    "solar3d",
    "floating-operating-panel",
    "main",
    "sdroxide-panadapter",
    "sdroxide-panel",
    "sdroxide-controls",
    "sdroxide-panadapter-aux",
    "sdroxide-bandmenu",
    "sdroxide-rotor",
    "acars",
    "adsb",
    "adsb-setup",
    "aircraft",
    "ais",
    "ais-setup",
    "ale",
    "antenna",
    "aprs",
    "awards",
    "bands",
    "contest",
    "dab",
    "digi-setup",
    "drm",
    "dsc",
    "enigma",
    "extras",
    "fsq",
    "fsq-contacts",
    "grid-tracker",
    "hd-radio",
    "hell",
    "hfdl",
    "image",
    "ism",
    "js8",
    "jtty",
    "known-calls",
    "logbook",
    "mail",
    "memories",
    "meshtastic",
    "morse",
    "navtex",
    "packet",
    "pi4",
    "playback",
    "public-sdrs",
    "radio_classic",
    "rds",
    "recordings",
    "rfpaint",
    "sat",
    "scanner",
    "schedule",
    "session-qsos",
    "settings",
    "signal-id",
    "spots",
    "sstv",
    "swl-log",
    "text",
    "uvpacket",
    "vdl2",
    "vdl2-setup",
    "voice-keyer",
    "weather",
    "wefax",
    "wspr",
    "wxsat",
    "t",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowEntry {
    pub id: String,
    pub prefs: WindowPrefs,
}

/// Compact fixed slots; named serialization tolerates new/removed windows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowLayouts([WindowPrefs; WINDOW_IDS.len()]);

impl Default for WindowLayouts {
    fn default() -> Self {
        Self([WindowPrefs::default(); WINDOW_IDS.len()])
    }
}
impl WindowLayouts {
    pub fn get(&self, id: &str) -> WindowPrefs {
        WINDOW_IDS.iter().position(|name| *name == id).map(|i| self.0[i]).unwrap_or_default()
    }
    pub fn set(&mut self, id: &str, prefs: WindowPrefs) {
        if let Some(i) = WINDOW_IDS.iter().position(|name| *name == id) {
            self.0[i] = prefs;
        }
    }
    pub fn entries(&self) -> Vec<WindowEntry> {
        WINDOW_IDS
            .iter()
            .zip(self.0)
            .filter(|(_, p)| *p != WindowPrefs::default())
            .map(|(id, prefs)| WindowEntry { id: (*id).into(), prefs })
            .collect()
    }
    pub fn from_entries(entries: &[WindowEntry]) -> Self {
        let mut out = Self::default();
        for entry in entries {
            out.set(&entry.id, entry.prefs);
        }
        out
    }
}
impl Serialize for WindowLayouts {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.entries().serialize(s)
    }
}
impl<'de> Deserialize<'de> for WindowLayouts {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(Self::from_entries(&Vec::<WindowEntry>::deserialize(d)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn named_slots_survive_disk_and_wire_without_sharing_geometry() {
        let mut s = crate::UiSettings::default();
        let prefs = WindowPrefs {
            level: WindowStack::Top,
            mode: WindowMode::Maximized,
            undocked: true,
            embedded_window: Some(DetachedWindow { size: [800., 600.], pos: Some([20., 40.]) }),
            ..Default::default()
        };
        s.window_layouts.set("wefax", prefs);
        s.tool_windows_open = 37;
        s.set_detached(crate::DetachableModule::Panadapter, true);
        let disk: crate::UiSettings =
            serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(disk.window_layouts.get("wefax"), prefs);
        assert_eq!(disk.window_layouts.get("dab"), WindowPrefs::default());
        let sent = crate::ClientScreen::from_settings(&s);
        let received: crate::ClientScreen =
            postcard::from_bytes(&postcard::to_allocvec(&sent).unwrap()).unwrap();
        let mut other = crate::UiSettings::default();
        received.apply_to(&mut other);
        assert_eq!(other.window_layouts, s.window_layouts);
        assert_eq!(other.tool_windows_open, 37);
        assert_eq!(other.detached, s.detached);
    }
    #[test]
    fn old_and_future_settings_keep_safe_defaults() {
        let old: crate::UiSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(old.window_layouts, WindowLayouts::default());
        let future: WindowLayouts = serde_json::from_str(
            r#"[{"id":"unknown","prefs":{}},{"id":"wefax","prefs":{"level":"Future"}}]"#,
        )
        .unwrap();
        assert_eq!(future, WindowLayouts::default());
        let mut names = WINDOW_IDS.to_vec();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), WINDOW_IDS.len());
    }
}
