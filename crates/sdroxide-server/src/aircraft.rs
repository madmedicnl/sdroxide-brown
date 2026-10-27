//! `GET /aircraft-info/{icao}`: what the internet knows about one aircraft,
//! asked once for every client of the station.
//!
//! A client with the AIRCRAFT window open sends the address of the card it has
//! open, and optionally the callsign and registration the radio heard
//! (`?callsign=KLM1234&reg=PH-BXA`). It gets one JSON [`Info`] back: the
//! airframe, the route, and the address, page and credit of a photo — never the
//! picture, which is the client's to load from Planespotters itself, because
//! their terms do not allow it to be fetched, kept or re-served from here.
//!
//! The decision of what to answer is [`judge`], a function of plain values, so
//! the status codes are tested without a socket. The questions themselves are
//! the [`Relay`]'s: one out per key, a ceiling per minute, 24 h in the cache.
//!
//! Off unless the station's operator switched it on ([`enabled_by`]): it makes
//! this machine reach out to two services on behalf of whoever is connected,
//! and that is the operator's call, not a client's.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};

use sdroxide_aircraft_online::fetch::HttpUpstream;
use sdroxide_aircraft_online::{
    Info, OnlineCache, Relay, Status, Upstream, clean_callsign, clean_registration,
};

use crate::Station;

/// Questions sent to the two services per minute, whoever is asking. Generous
/// for a person clicking through cards; a station being used to hammer them
/// runs into it.
pub(crate) const MAX_QUESTIONS_PER_MINUTE: usize = 30;

/// The variable that switches the relay on.
pub const ENV_SWITCH: &str = "SDROXIDE_AIRCRAFT_ONLINE";

/// Whether a value of [`ENV_SWITCH`] switches the relay on. Anything but a
/// plain yes is off: a typo must not open a way out of the machine.
pub fn enabled_by(value: Option<&str>) -> bool {
    value.is_some_and(|v| {
        matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
    })
}

/// What the route answers to, decided before anything is sent.
#[derive(Debug, PartialEq)]
pub(crate) enum Verdict {
    Reject(Status),
    Run { icao: u32, callsign: Option<String>, registration: Option<String> },
}

/// A query value that is absent or empty is not there; one that is there must
/// be clean, or the whole question is refused rather than quietly changed.
fn clean_param(
    v: Option<&String>,
    clean: fn(&str) -> Option<String>,
) -> Result<Option<String>, ()> {
    match v.map(|s| s.trim()).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(s) => clean(s).map(Some).ok_or(()),
    }
}

/// Sign-in first, then the switch, then the question itself.
pub(crate) fn judge(
    enabled: bool,
    sign_in_needed: bool,
    signed_in: bool,
    icao: &str,
    query: &HashMap<String, String>,
) -> Verdict {
    if sign_in_needed && !signed_in {
        return Verdict::Reject(Status::Unauthorized);
    }
    if !enabled {
        return Verdict::Reject(Status::Disabled);
    }
    let icao = (icao.len() == 6 && icao.bytes().all(|b| b.is_ascii_hexdigit()))
        .then(|| u32::from_str_radix(icao, 16).ok())
        .flatten();
    let callsign = clean_param(query.get("callsign"), clean_callsign);
    let registration = clean_param(query.get("reg"), clean_registration);
    match (icao, callsign, registration) {
        (Some(icao), Ok(callsign), Ok(registration)) => {
            Verdict::Run { icao, callsign, registration }
        }
        _ => Verdict::Reject(Status::Invalid),
    }
}

pub(crate) fn http_status(s: Status) -> StatusCode {
    match s {
        Status::Ok | Status::Unknown | Status::Unavailable => StatusCode::OK,
        Status::Busy => StatusCode::TOO_MANY_REQUESTS,
        Status::Disabled => StatusCode::SERVICE_UNAVAILABLE,
        Status::Invalid => StatusCode::BAD_REQUEST,
        Status::Unauthorized => StatusCode::UNAUTHORIZED,
    }
}

/// The file whose presence in the config directory means the operator switched
/// the relay on from the AIRCRAFT window. Presence only; there is nothing in it.
pub(crate) const SWITCH_FILE: &str = "aircraft-online.on";

/// The header a switch request must carry. A page on another site cannot add a
/// custom header to a request without the station's say-so (a preflight), so
/// this keeps a stray link or form from flipping the relay.
pub(crate) const SWITCH_HEADER: &str = "x-sdroxide";

/// The relay and the file its cache lives in.
pub(crate) struct AircraftHub<U = HttpUpstream> {
    /// Whether the relay answers. Starts from the environment variable or the
    /// switch file, and the AIRCRAFT window can change it while running.
    enabled: AtomicBool,
    /// The environment variable is on, which no window can undo.
    forced: bool,
    relay: Relay<U>,
    file: Option<PathBuf>,
    switch_file: Option<PathBuf>,
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

impl AircraftHub<HttpUpstream> {
    /// The station's relay. When it is off nothing is read, nothing is
    /// written and no connection is ever made.
    pub(crate) fn start(forced: bool) -> AircraftHub {
        let dir = sdroxide_config::config_dir().ok();
        let switch_file = dir.as_ref().map(|d| d.join(SWITCH_FILE));
        let enabled = forced || switch_file.as_ref().is_some_and(|f| f.exists());
        let file = enabled.then(|| dir.as_ref().map(|d| d.join("aircraft-online.json"))).flatten();
        let cache = file
            .as_ref()
            .and_then(|f| std::fs::read_to_string(f).ok())
            .map(|text| OnlineCache::from_json(&text, now_unix()))
            .unwrap_or_default();
        AircraftHub {
            enabled: AtomicBool::new(enabled),
            forced,
            relay: Relay::new(HttpUpstream::new(), MAX_QUESTIONS_PER_MINUTE, cache),
            file,
            switch_file,
        }
    }
}

impl<U: Upstream> AircraftHub<U> {
    #[cfg(test)]
    pub(crate) fn with(enabled: bool, upstream: U, max_per_minute: usize) -> AircraftHub<U> {
        AircraftHub {
            enabled: AtomicBool::new(enabled),
            forced: false,
            relay: Relay::new(upstream, max_per_minute, OnlineCache::default()),
            file: None,
            switch_file: None,
        }
    }

    pub(crate) fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Switch the relay on or off from a client, and remember the choice in the
    /// config directory so a restart keeps it. The environment variable wins
    /// over a request to turn it off: it is the operator's, set on the machine.
    /// Returns what the relay is now.
    pub(crate) fn set_enabled(&self, on: bool) -> bool {
        let on = on || self.forced;
        if let Some(f) = &self.switch_file {
            if on && !self.forced {
                // A switch that cannot be remembered still works until restart.
                let _ = std::fs::write(f, b"");
            } else if !on {
                let _ = std::fs::remove_file(f);
            }
        }
        self.enabled.store(on, Ordering::Relaxed);
        on
    }

    /// Ask, and write the cache out if a minute has passed with something new
    /// in it. Blocking: the route runs it off the async threads.
    pub(crate) fn run(
        &self,
        icao: u32,
        callsign: Option<&str>,
        registration: Option<&str>,
    ) -> Info {
        let now = now_unix();
        let info = self.relay.lookup(now, icao, callsign, registration);
        if let (Some(file), Some(json)) = (&self.file, self.relay.snapshot_if_due(now)) {
            // A cache that cannot be written costs a question tomorrow.
            let _ = std::fs::write(file, json);
        }
        info
    }
}

fn json(status: StatusCode, info: &Info) -> Response {
    (
        status,
        [(header::CONTENT_TYPE, "application/json"), (header::CACHE_CONTROL, "no-store")],
        serde_json::to_string(info).unwrap_or_default(),
    )
        .into_response()
}

pub(crate) async fn info_route(
    State(station): State<Arc<Station>>,
    Path(icao): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    let hub = station.aircraft.clone();
    // The same door as the sockets: a station that asks for a password asks
    // for it here, and the cookie it handed out at sign-in is what answers.
    let needed = station.auth.required().is_some();
    let signed = crate::session_cookie::from_headers(&headers, &station.auth).is_some();
    match judge(hub.enabled(), needed, signed, &icao, &query) {
        Verdict::Reject(status) => json(http_status(status), &Info::bare(status)),
        Verdict::Run { icao, callsign, registration } => {
            let info = tokio::task::spawn_blocking(move || {
                hub.run(icao, callsign.as_deref(), registration.as_deref())
            })
            .await
            .unwrap_or_else(|_| Info::bare(Status::Unavailable));
            station.history.note_card(now_unix(), icao, &info);
            json(http_status(info.status), &info)
        }
    }
}

/// What `/aircraft-online` says: whether the relay is on, and whether the
/// station's environment variable holds it on.
#[derive(serde::Serialize)]
struct Switch {
    enabled: bool,
    forced: bool,
}

fn switch_json(status: StatusCode, hub: &AircraftHub) -> Response {
    let body = Switch { enabled: hub.enabled(), forced: hub.forced };
    (
        status,
        [(header::CONTENT_TYPE, "application/json"), (header::CACHE_CONTROL, "no-store")],
        serde_json::to_string(&body).unwrap_or_default(),
    )
        .into_response()
}

/// What a request to the switch is answered with before it is allowed to do
/// anything: the sign-in first, then (for a change) the custom header.
pub(crate) fn switch_refusal(
    sign_in_needed: bool,
    signed_in: bool,
    changing: bool,
    has_header: bool,
) -> Option<StatusCode> {
    if sign_in_needed && !signed_in {
        return Some(StatusCode::UNAUTHORIZED);
    }
    (changing && !has_header).then_some(StatusCode::FORBIDDEN)
}

/// `GET /aircraft-online`: is the relay on.
pub(crate) async fn switch_get(
    State(station): State<Arc<Station>>,
    headers: HeaderMap,
) -> Response {
    let needed = station.auth.required().is_some();
    let signed = crate::session_cookie::from_headers(&headers, &station.auth).is_some();
    if let Some(code) = switch_refusal(needed, signed, false, false) {
        return code.into_response();
    }
    switch_json(StatusCode::OK, &station.aircraft)
}

/// `POST /aircraft-online?on=1|0`: switch the relay on or off. Same sign-in as
/// the sockets, plus the custom header (see [`SWITCH_HEADER`]).
pub(crate) async fn switch_post(
    State(station): State<Arc<Station>>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    let needed = station.auth.required().is_some();
    let signed = crate::session_cookie::from_headers(&headers, &station.auth).is_some();
    if let Some(code) = switch_refusal(needed, signed, true, headers.contains_key(SWITCH_HEADER)) {
        return code.into_response();
    }
    let Some(on) = query.get("on").map(|v| enabled_by(Some(v.as_str()))) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    station.aircraft.set_enabled(on);
    switch_json(StatusCode::OK, &station.aircraft)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use sdroxide_aircraft_online::{Lookup, Reply, Request, parse_adsbdb};

    use super::*;

    fn q(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn a_change_needs_the_sign_in_and_the_custom_header() {
        // Sign-in first.
        assert_eq!(switch_refusal(true, false, true, true), Some(StatusCode::UNAUTHORIZED));
        assert_eq!(switch_refusal(true, false, false, false), Some(StatusCode::UNAUTHORIZED));
        // A read needs no header; a change does.
        assert_eq!(switch_refusal(false, false, false, false), None);
        assert_eq!(switch_refusal(false, false, true, false), Some(StatusCode::FORBIDDEN));
        assert_eq!(switch_refusal(true, true, true, true), None);
    }

    #[test]
    fn a_client_can_switch_the_relay_on_and_off_unless_the_environment_holds_it() {
        let hub = AircraftHub::with(false, Counting(Arc::new(AtomicUsize::new(0))), 10);
        assert!(!hub.enabled());
        assert!(hub.set_enabled(true) && hub.enabled());
        assert!(!hub.set_enabled(false) && !hub.enabled());
        let forced = AircraftHub {
            forced: true,
            ..AircraftHub::with(true, Counting(Arc::new(AtomicUsize::new(0))), 10)
        };
        assert!(forced.set_enabled(false), "the environment variable wins");
        assert!(forced.enabled());
    }

    #[test]
    fn the_choice_is_remembered_in_a_file_and_the_route_turns_a_question_away_while_off() {
        let dir =
            std::env::temp_dir().join(format!("sdroxide-aircraft-switch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join(SWITCH_FILE);
        let mut hub = AircraftHub::with(false, Counting(Arc::new(AtomicUsize::new(0))), 10);
        hub.switch_file = Some(file.clone());
        hub.set_enabled(true);
        assert!(file.exists(), "on is written down");
        hub.set_enabled(false);
        assert!(!file.exists(), "off is written down too");
        assert_eq!(
            judge(hub.enabled(), false, false, "484006", &q(&[])),
            Verdict::Reject(Status::Disabled)
        );
    }

    #[test]
    fn only_a_plain_yes_switches_the_relay_on() {
        for on in ["1", "true", "TRUE", "yes", " on "] {
            assert!(enabled_by(Some(on)), "{on:?}");
        }
        for off in ["", "0", "false", "no", "off", "tru", "enabled", "2"] {
            assert!(!enabled_by(Some(off)), "{off:?}");
        }
        assert!(!enabled_by(None), "off by default");
    }

    #[test]
    fn a_good_question_runs_with_its_address_callsign_and_registration_cleaned() {
        let v =
            judge(true, false, false, "484006", &q(&[("callsign", "klm1234"), ("reg", "ph-bxa")]));
        assert_eq!(
            v,
            Verdict::Run {
                icao: 0x484006,
                callsign: Some("KLM1234".into()),
                registration: Some("PH-BXA".into())
            }
        );
        let v = judge(true, false, false, "4ca1fa", &q(&[("callsign", ""), ("reg", "  ")]));
        assert_eq!(v, Verdict::Run { icao: 0x4CA1FA, callsign: None, registration: None });
    }

    #[test]
    fn an_address_that_is_not_six_hex_digits_is_refused() {
        for bad in ["", "48400", "4840066", "48400G", "0x4840", "48 006", "../..", "-48400"] {
            assert_eq!(
                judge(true, false, false, bad, &q(&[])),
                Verdict::Reject(Status::Invalid),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn a_callsign_or_registration_that_is_not_clean_refuses_the_question() {
        for (k, v) in
            [("callsign", "KLM 123"), ("callsign", "KLM1234&x=1"), ("reg", "PH/BXA"), ("reg", "A")]
        {
            assert_eq!(
                judge(true, false, false, "484006", &q(&[(k, v)])),
                Verdict::Reject(Status::Invalid),
                "{k}={v}"
            );
        }
    }

    #[test]
    fn the_door_is_checked_before_the_switch_and_the_switch_before_the_question() {
        // No sign-in where one is needed: 401, even for a question that is also bad.
        assert_eq!(judge(true, true, false, "zz", &q(&[])), Verdict::Reject(Status::Unauthorized));
        assert_eq!(
            judge(false, true, false, "484006", &q(&[])),
            Verdict::Reject(Status::Unauthorized)
        );
        // Signed in but switched off: disabled, not invalid.
        assert_eq!(judge(false, true, true, "zz", &q(&[])), Verdict::Reject(Status::Disabled));
        // Signed in and on: the question is judged.
        assert_eq!(judge(true, true, true, "zz", &q(&[])), Verdict::Reject(Status::Invalid));
        // No password on the station: nobody needs a cookie.
        assert!(matches!(judge(true, false, false, "484006", &q(&[])), Verdict::Run { .. }));
    }

    #[test]
    fn every_status_has_the_http_code_a_client_can_act_on() {
        assert_eq!(http_status(Status::Ok), StatusCode::OK);
        assert_eq!(http_status(Status::Unknown), StatusCode::OK);
        assert_eq!(http_status(Status::Unavailable), StatusCode::OK);
        assert_eq!(http_status(Status::Busy), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(http_status(Status::Disabled), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(http_status(Status::Invalid), StatusCode::BAD_REQUEST);
        assert_eq!(http_status(Status::Unauthorized), StatusCode::UNAUTHORIZED);
    }

    struct Counting(Arc<AtomicUsize>);

    impl Upstream for Counting {
        fn fetch(&self, req: &Request) -> Reply {
            self.0.fetch_add(1, Ordering::SeqCst);
            match req {
                Request::Adsbdb { .. } => Reply::Adsbdb(Lookup::Found(
                    parse_adsbdb(r#"{"response":{"aircraft":{"type":"A320"}}}"#).unwrap(),
                )),
                Request::Photo { .. } => Reply::Photo(Lookup::NotFound),
            }
        }
    }

    #[test]
    fn a_hub_answers_through_the_relay_and_the_second_ask_costs_nothing() {
        let asked = Arc::new(AtomicUsize::new(0));
        let hub = AircraftHub::with(true, Counting(asked.clone()), 100);
        let a = hub.run(0x484006, None, None);
        assert_eq!(a.status, Status::Ok);
        assert_eq!(a.aircraft.unwrap().type_name.as_deref(), Some("A320"));
        let b = hub.run(0x484006, None, None);
        assert!(b.cached);
        assert_eq!(asked.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn past_the_ceiling_the_hub_says_busy_and_the_route_would_say_429() {
        let asked = Arc::new(AtomicUsize::new(0));
        let hub = AircraftHub::with(true, Counting(asked.clone()), 1);
        assert_eq!(hub.run(1, None, None).status, Status::Ok);
        let busy = hub.run(2, None, None);
        assert_eq!(busy.status, Status::Busy);
        assert_eq!(http_status(busy.status), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(asked.load(Ordering::SeqCst), 1);
    }
}
