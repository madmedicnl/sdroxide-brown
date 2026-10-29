//! ADS-B overlay from adsb.lol: live aircraft positions in the area, fetched
//! periodically so the operator can compare what they receive locally with
//! what the network sees.
//!
//! Offline-first: the overlay is off by default and degrades quietly when there
//! is no network. All fetching happens on a background thread (native) or
//! via fetch (wasm); the panel only reads the cached result.

use std::sync::mpsc::{Receiver, Sender, channel};

/// One aircraft from the adsb.lol overlay.
#[derive(Clone, Debug, Default)]
pub struct OverlayAircraft {
    pub hex: String,
    pub callsign: Option<String>,
    pub lat: f64,
    pub lon: f64,
    pub alt_ft: Option<i32>,
    pub gs_kt: Option<f64>,
    pub track_deg: Option<f64>,
    pub squawk: Option<String>,
}

/// State for the overlay: cached aircraft and fetch status.
#[derive(Default)]
pub struct AdsbOverlay {
    pub aircraft: Vec<OverlayAircraft>,
    pub enabled: bool,
    pub last_fetch: f64,
    pub fetching: bool,
    pub fetch_failed: bool,
    #[cfg(not(target_arch = "wasm32"))]
    rx: Option<Receiver<(Vec<OverlayAircraft>, bool)>>,
}

impl AdsbOverlay {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether it's time to fetch (every 30 seconds).
    pub fn should_fetch(&self, now: f64) -> bool {
        self.enabled && !self.fetching && (self.last_fetch == 0.0 || now - self.last_fetch >= 30.0)
    }

    /// Start a fetch. Returns immediately; result arrives via poll().
    /// `station` is the station URL (peer URL); the fetch goes through the
    /// station's relay, never directly to adsb.lol.
    pub fn start_fetch(&mut self, station: &str, lat: f64, lon: f64, radius_nm: f64) {
        if self.fetching {
            return;
        }
        self.fetching = true;
        let (tx, rx) = channel::<(Vec<OverlayAircraft>, bool)>();
        let station = station.to_string();
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.rx = Some(rx);
            std::thread::spawn(move || {
                let (result, failed) = fetch_sync(&station, lat, lon, radius_nm);
                let _ = tx.send((result, failed));
            });
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (tx, rx);
        }
    }

    /// Poll for a completed fetch. Returns true if new data arrived.
    pub fn poll(&mut self, now: f64) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(rx) = &self.rx {
            if let Ok((result, failed)) = rx.try_recv() {
                self.aircraft = result;
                self.fetch_failed = failed;
                self.last_fetch = now;
                self.fetching = false;
                self.rx = None;
                return true;
            }
        }
        false
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn fetch_sync(station: &str, lat: f64, lon: f64, radius_nm: f64) -> (Vec<OverlayAircraft>, bool) {
    let base = match crate::login::http_base(station) {
        Some(b) => b,
        None => return (Vec::new(), true),
    };
    let url = format!("{}/adsb-overlay?lat={}&lon={}&radius={}", base, lat, lon, radius_nm as i32);
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(12)))
        .build()
        .into();
    match agent.get(&url).call() {
        Ok(mut resp) => {
            if let Ok(body) = resp.body_mut().with_config().limit(1024 * 1024).read_to_string() {
                (parse_server_response(&body), false)
            } else {
                (Vec::new(), true)
            }
        }
        Err(_) => (Vec::new(), true),
    }
}

fn parse_server_response(body: &str) -> Vec<OverlayAircraft> {
    let mut result = Vec::new();
    let mut depth = 0;
    let mut obj_start = None;
    for (i, c) in body.char_indices() {
        match c {
            '{' => {
                if depth == 0 {
                    obj_start = Some(i);
                }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    if let Some(start) = obj_start {
                        let obj = &body[start..=i];
                        if let Some(ac) = parse_overlay_aircraft(obj) {
                            result.push(ac);
                        }
                    }
                    obj_start = None;
                }
            }
            _ => {}
        }
    }
    result
}

fn parse_overlay_aircraft(obj: &str) -> Option<OverlayAircraft> {
    let hex = extract_str(obj, "hex")?;
    let lat = extract_f64(obj, "lat")?;
    let lon = extract_f64(obj, "lon")?;
    Some(OverlayAircraft {
        hex,
        callsign: extract_str(obj, "flight"),
        lat,
        lon,
        alt_ft: extract_i32(obj, "alt_baro"),
        gs_kt: extract_f64(obj, "gs"),
        track_deg: extract_f64(obj, "track"),
        squawk: extract_str(obj, "squawk"),
    })
}

fn parse_response(body: &str) -> Vec<OverlayAircraft> {
    // Simple JSON parsing without serde (to avoid adding deps).
    // The response has an "ac" array with aircraft objects.
    let mut result = Vec::new();
    // Find the "ac" array
    if let Some(ac_start) = body.find("\"ac\"") {
        let rest = &body[ac_start..];
        // Find array start
        if let Some(arr_start) = rest.find('[') {
            let arr = &rest[arr_start..];
            // Parse each object
            let mut depth = 0;
            let mut obj_start = None;
            for (i, c) in arr.char_indices() {
                match c {
                    '{' => {
                        if depth == 0 {
                            obj_start = Some(i);
                        }
                        depth += 1;
                    }
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            if let Some(start) = obj_start {
                                let obj = &arr[start..=i];
                                if let Some(ac) = parse_aircraft(obj) {
                                    result.push(ac);
                                }
                            }
                            obj_start = None;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    result
}

fn parse_aircraft(obj: &str) -> Option<OverlayAircraft> {
    let hex = extract_str(obj, "hex")?;
    let lat = extract_f64(obj, "lat")?;
    let lon = extract_f64(obj, "lon")?;
    Some(OverlayAircraft {
        hex,
        callsign: extract_str(obj, "flight").map(|s| s.trim().to_string()),
        lat,
        lon,
        alt_ft: extract_i32(obj, "alt_baro"),
        gs_kt: extract_f64(obj, "gs"),
        track_deg: extract_f64(obj, "track"),
        squawk: extract_str(obj, "squawk"),
    })
}

fn extract_str(obj: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\"", key);
    let idx = obj.find(&pattern)?;
    let rest = &obj[idx + pattern.len()..];
    let rest = rest.trim_start().trim_start_matches(':').trim_start();
    if rest.starts_with('"') {
        let end = rest[1..].find('"')?;
        Some(rest[1..1 + end].to_string())
    } else {
        None
    }
}

fn extract_f64(obj: &str, key: &str) -> Option<f64> {
    let pattern = format!("\"{}\"", key);
    let idx = obj.find(&pattern)?;
    let rest = &obj[idx + pattern.len()..];
    let rest = rest.trim_start().trim_start_matches(':').trim_start();
    // read until comma or }
    let end = rest.find(|c: char| c == ',' || c == '}' || c == ']').unwrap_or(rest.len());
    rest[..end].trim().parse().ok()
}

fn extract_i32(obj: &str, key: &str) -> Option<i32> {
    extract_f64(obj, key).map(|v| v as i32)
}
