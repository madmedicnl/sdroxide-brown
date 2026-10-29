//! `GET /adsb-overlay`: live aircraft positions from adsb.lol, fetched by the
//! station on behalf of its clients.
//!
//! The client never calls adsb.lol directly (relay rule: outbound calls go
//! through the station). The station fetches, caches for 30 s, and returns the
//! parsed aircraft list.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use crate::Station;

/// How long a cached answer is good for.
const CACHE_TTL: Duration = Duration::from_secs(30);

/// One aircraft from the overlay.
#[derive(Clone, Debug, Serialize)]
pub struct OverlayAircraft {
    pub hex: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flight: Option<String>,
    pub lat: f64,
    pub lon: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_baro: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gs: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track: Option<f64>,
}

/// Cached answer.
struct Cache {
    lat: f64,
    lon: f64,
    radius: i32,
    aircraft: Vec<OverlayAircraft>,
    fetched_at: Instant,
}

/// The relay state, held in the station.
pub struct AdsbOverlayRelay {
    cache: Mutex<Option<Cache>>,
}

impl AdsbOverlayRelay {
    pub fn new() -> Self {
        Self { cache: Mutex::new(None) }
    }

    /// Whether a cached answer is still good.
    fn cached(&self, lat: f64, lon: f64, radius: i32) -> Option<Vec<OverlayAircraft>> {
        let guard = self.cache.lock().ok()?;
        let c = guard.as_ref()?;
        if c.lat == lat && c.lon == lon && c.radius == radius && c.fetched_at.elapsed() < CACHE_TTL
        {
            Some(c.aircraft.clone())
        } else {
            None
        }
    }

    /// Fetch from adsb.lol and cache.
    fn fetch(&self, lat: f64, lon: f64, radius: i32) -> Vec<OverlayAircraft> {
        // Check cache first.
        if let Some(cached) = self.cached(lat, lon, radius) {
            return cached;
        }

        let url = format!("https://api.adsb.lol/v2/lat/{}/lon/{}/dist/{}", lat, lon, radius);
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(12)))
            .build()
            .into();
        let result = match agent.get(&url).call() {
            Ok(mut resp) => {
                if let Ok(body) =
                    resp.body_mut().with_config().limit(4 * 1024 * 1024).read_to_string()
                {
                    parse_response(&body)
                } else {
                    Vec::new()
                }
            }
            Err(_) => Vec::new(),
        };

        // Cache the result.
        if let Ok(mut guard) = self.cache.lock() {
            *guard = Some(Cache {
                lat,
                lon,
                radius,
                aircraft: result.clone(),
                fetched_at: Instant::now(),
            });
        }
        result
    }
}

/// Parse the adsb.lol JSON response.
fn parse_response(body: &str) -> Vec<OverlayAircraft> {
    let mut result = Vec::new();
    if let Some(ac_start) = body.find("\"ac\"") {
        let rest = &body[ac_start..];
        if let Some(arr_start) = rest.find('[') {
            let arr = &rest[arr_start..];
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
        flight: extract_str(obj, "flight").map(|s| s.trim().to_string()),
        lat,
        lon,
        alt_baro: extract_i32(obj, "alt_baro"),
        gs: extract_f64(obj, "gs"),
        track: extract_f64(obj, "track"),
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
    let end = rest.find(|c: char| c == ',' || c == '}' || c == ']').unwrap_or(rest.len());
    rest[..end].trim().parse().ok()
}

fn extract_i32(obj: &str, key: &str) -> Option<i32> {
    extract_f64(obj, key).map(|v| v as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_response() {
        let result = parse_response("");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_valid_response() {
        let body = r#"{"ac":[{"hex":"abc123","flight":"TEST1  ","lat":52.0,"lon":4.0,"alt_baro":10000,"gs":250.5,"track":90.0}]}"#;
        let result = parse_response(body);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].hex, "abc123");
        assert_eq!(result[0].flight.as_deref(), Some("TEST1"));
        assert_eq!(result[0].lat, 52.0);
        assert_eq!(result[0].lon, 4.0);
        assert_eq!(result[0].alt_baro, Some(10000));
        assert_eq!(result[0].gs, Some(250.5));
        assert_eq!(result[0].track, Some(90.0));
    }

    #[test]
    fn parse_multiple_aircraft() {
        let body = r#"{"ac":[{"hex":"aaa111","lat":52.0,"lon":4.0},{"hex":"bbb222","lat":53.0,"lon":5.0}]}"#;
        let result = parse_response(body);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn parse_missing_fields() {
        let body = r#"{"ac":[{"hex":"abc123","lat":52.0,"lon":4.0}]}"#;
        let result = parse_response(body);
        assert_eq!(result.len(), 1);
        assert!(result[0].flight.is_none());
        assert!(result[0].alt_baro.is_none());
    }
}

/// `GET /adsb-overlay?lat=...&lon=...&radius=...`
pub async fn overlay_get(
    State(station): State<std::sync::Arc<Station>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Response {
    let lat: f64 = match params.get("lat").and_then(|v| v.parse().ok()) {
        Some(v) => v,
        None => return StatusCode::BAD_REQUEST.into_response(),
    };
    let lon: f64 = match params.get("lon").and_then(|v| v.parse().ok()) {
        Some(v) => v,
        None => return StatusCode::BAD_REQUEST.into_response(),
    };
    let radius: i32 = params.get("radius").and_then(|v| v.parse().ok()).unwrap_or(250);

    let result = station.adsb_overlay.fetch(lat, lon, radius);
    match serde_json::to_string(&result) {
        Ok(json) => (StatusCode::OK, json).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
