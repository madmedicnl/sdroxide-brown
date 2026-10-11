//! `antennas.json` — the saved designs, in the operator's own config directory.
//!
//! One file, like `memories.json`, holding a list of [`Antenna`]. Native-only
//! and fork-only; the browser build has no use for it.

use std::path::Path;

use crate::model::Antenna;

/// The file's name inside the config directory.
pub const FILE_NAME: &str = "antennas.json";

/// Pretty JSON for a list of designs (the on-disk form).
pub fn to_json(antennas: &[Antenna]) -> Result<String, String> {
    serde_json::to_string_pretty(antennas).map_err(|e| e.to_string())
}

/// Parse a list of designs; a document that will not parse is an error, not a
/// silently empty list (the house rule against a control that does nothing).
pub fn from_json(text: &str) -> Result<Vec<Antenna>, String> {
    let list: Vec<Antenna> = serde_json::from_str(text).map_err(|e| e.to_string())?;
    Ok(list)
}

pub fn save(path: &Path, antennas: &[Antenna]) -> Result<(), String> {
    let text = to_json(antennas)?;
    std::fs::write(path, text).map_err(|e| e.to_string())
}

pub fn load(path: &Path) -> Result<Vec<Antenna>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    from_json(&text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use crate::presets;

    #[test]
    fn a_design_survives_json() {
        let designs = presets::catalog(27_265_000.0);
        let text = to_json(&designs).unwrap();
        let back = from_json(&text).unwrap();
        assert_eq!(designs, back, "the list round-trips");
    }

    #[test]
    fn a_broken_document_is_refused() {
        assert!(from_json("{ not json").is_err());
    }

    #[test]
    fn save_and_load() {
        let dir = std::env::temp_dir().join(format!("antenna-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(FILE_NAME);
        let designs =
            vec![Antenna { name: "test".into(), freq_hz: 27_265_000.0, ..Antenna::default() }];
        save(&path, &designs).unwrap();
        let back = load(&path).unwrap();
        assert_eq!(designs, back);
        std::fs::remove_dir_all(&dir).ok();
    }
}
