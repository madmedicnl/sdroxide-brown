//! What the station's relay adds to an aircraft card, and how it is worded.
//!
//! The radio's data is the card; this is the optional extra. Everything here is
//! a decision on plain values — which question to ask, whether to ask it again,
//! how an answer reads, which badge a line wears — so it is tested without a
//! socket, a clock or a window. The asking itself is [`super::aircraft_online_io`].
//!
//! The rules it keeps:
//!
//! * the radio's lines come first and are never replaced by an online one; the
//!   online data only fills what the radio has not heard;
//! * a client asks its own station's `GET /aircraft-info/{icao}` and nothing
//!   else — no key, no account, no call to a registry or a photo service;
//! * a station that has the relay off, or none, is asked once and left alone;
//!   a failure is asked again no sooner than [`RETRY_S`] — and none of it is an
//!   error on the card;
//! * all of it lives in memory for the length of the session. Nothing is
//!   written to the browser or to the settings: the durable cache is the
//!   station's.

// ==== IMPL ====
use std::collections::HashMap;

use sdroxide_aircraft_online::{
    FORMAT_VERSION, Info, Photo, Status, clean_callsign, clean_registration,
};

use super::aircraft_card::{self, CardKey, icao_hex};
use super::aircraft_registry::AircraftCard;

/// Seconds before a failed question may be asked again.
pub(in crate::app) const RETRY_S: i64 = 300;
/// Seconds after which a question that never came back is given up on.
pub(in crate::app) const FLIGHT_TIMEOUT_S: i64 = 30;
/// Cards whose online state is kept; the least recently looked at go first.
pub(in crate::app) const MAX_KEPT: usize = 64;

/// Why there is nothing online on a card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum Reason {
    /// The station's operator has not switched the relay on.
    Disabled,
    /// No answer: the station could not reach the services, or this client
    /// could not reach the station, or the reply was not understood.
    Offline,
    /// The station's ceiling on questions per minute was reached.
    Busy,
    /// The station wants a sign-in this client does not have.
    Unauthorized,
    /// The registry answered and does not know this aircraft.
    Unknown,
    /// There is no station to ask: an engine running in this very program.
    NoStation,
}

impl Reason {
    /// What the card says about it. Only the reasons the operator can do
    /// something about are said; a failure that will mend itself is not an
    /// error, and is not drawn as one.
    pub(in crate::app) fn note(self) -> Option<&'static str> {
        match self {
            Reason::Disabled => {
                Some("online unavailable — the station has not switched the relay on")
            }
            Reason::NoStation => {
                Some("online unavailable — no station to ask; the radio data is all there is")
            }
            Reason::Unauthorized => Some("online unavailable — not signed in to the station"),
            Reason::Offline | Reason::Busy | Reason::Unknown => None,
        }
    }

    /// Seconds before the same question may be asked again; `None` is never in
    /// this session.
    pub(in crate::app) fn retry_after(self) -> Option<i64> {
        match self {
            Reason::Offline | Reason::Busy | Reason::Unknown => Some(RETRY_S),
            Reason::Disabled | Reason::Unauthorized | Reason::NoStation => None,
        }
    }

    /// Whether this ends the questions for every card, not just this one.
    fn is_station_wide(self) -> bool {
        matches!(self, Reason::Disabled | Reason::Unauthorized)
    }
}

/// What the card knows about the online half.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::app) enum OnlineState {
    /// Not asked yet.
    Idle,
    /// Asked, no answer yet.
    Loading,
    Ready(Box<Info>),
    Unavailable(Reason),
}

/// What came back from a question.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::app) enum Answer {
    Info(Box<Info>),
    Unavailable(Reason),
}

/// Read a reply: the HTTP status (`None` when there was none) and the body.
///
/// The relay writes an [`Info`] with every status, so the body is read first
/// and the code only decides what a body that is not one means — a reverse
/// proxy's own 503 page is a failure, not a relay that is switched off.
pub(in crate::app) fn classify(http_status: Option<u16>, body: &str) -> Answer {
    let Some(code) = http_status else { return Answer::Unavailable(Reason::Offline) };
    if let Ok(info) = serde_json::from_str::<Info>(body) {
        if info.v != FORMAT_VERSION {
            return Answer::Unavailable(Reason::Offline);
        }
        return match info.status {
            Status::Ok => Answer::Info(Box::new(info)),
            Status::Unknown | Status::Invalid => Answer::Unavailable(Reason::Unknown),
            Status::Unavailable => Answer::Unavailable(Reason::Offline),
            Status::Busy => Answer::Unavailable(Reason::Busy),
            Status::Disabled => Answer::Unavailable(Reason::Disabled),
            Status::Unauthorized => Answer::Unavailable(Reason::Unauthorized),
        };
    }
    Answer::Unavailable(match code {
        401 => Reason::Unauthorized,
        429 => Reason::Busy,
        _ => Reason::Offline,
    })
}

/// One question: an address, and what the radio heard of the flight.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::app) struct Want {
    pub icao: u32,
    pub callsign: Option<String>,
    pub registration: Option<String>,
}

/// The question the open card asks. A card with no ICAO address has none, and
/// a callsign or registration that is not clean is left out rather than sent.
pub(in crate::app) fn want_for(key: &CardKey, card: &AircraftCard) -> Option<Want> {
    let CardKey::Icao(icao) = key else { return None };
    Some(Want {
        icao: *icao & 0xff_ffff,
        callsign: card.flight.as_ref().and_then(|f| clean_callsign(&f.value)),
        registration: card.registration.as_ref().and_then(|r| clean_registration(&r.value)),
    })
}

/// Where the question goes: the station this client is connected to, by its
/// socket address (`wss://host/prefix/ws/2` → `https://host/prefix/…`).
pub(in crate::app) fn info_url(peer_url: &str, want: &Want) -> Option<String> {
    let base = crate::login::http_base(&crate::login::station_key(peer_url))?;
    let mut url = format!("{base}/aircraft-info/{}", icao_hex(want.icao));
    let mut sep = '?';
    for (name, value) in [("callsign", &want.callsign), ("reg", &want.registration)] {
        if let Some(v) = value {
            url.push(sep);
            url.push_str(&format!("{name}={v}"));
            sep = '&';
        }
    }
    Some(url)
}

/// Where the switch request goes: `POST {station}/aircraft-online?on=1|0`.
pub(in crate::app) fn switch_url(peer_url: &str, on: bool) -> Option<String> {
    let base = crate::login::http_base(&crate::login::station_key(peer_url))?;
    Some(format!("{base}/aircraft-online?on={}", u8::from(on)))
}

/// What the station answered to a request to switch the relay.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::app) enum SwitchAnswer {
    /// The relay is on now.
    On,
    /// The relay is off now.
    Off,
    /// It did not change, and the sentence says why.
    Refused(&'static str),
}

/// Read the reply to a switch request.
pub(in crate::app) fn classify_switch(http_status: Option<u16>, body: &str) -> SwitchAnswer {
    match http_status {
        None => SwitchAnswer::Refused("the station did not answer"),
        Some(401) => SwitchAnswer::Refused("not signed in to the station"),
        Some(403) => SwitchAnswer::Refused("the station refused the request"),
        Some(200) => {
            #[derive(serde::Deserialize)]
            struct Reply {
                enabled: bool,
            }
            match serde_json::from_str::<Reply>(body) {
                Ok(r) if r.enabled => SwitchAnswer::On,
                Ok(_) => SwitchAnswer::Off,
                Err(_) => {
                    SwitchAnswer::Refused("the station does not offer this switch (too old?)")
                }
            }
        }
        Some(404) => SwitchAnswer::Refused("the station does not offer this switch (too old?)"),
        Some(_) => SwitchAnswer::Refused("the station could not change it"),
    }
}

struct Entry {
    state: OnlineState,
    /// When a failure may be asked again; `None` is not this session.
    retry_at: Option<i64>,
    /// A counter, not a time: the order cards were last looked at in.
    touched: u64,
}

static IDLE: OnlineState = OnlineState::Idle;
static NO_STATION: OnlineState = OnlineState::Unavailable(Reason::NoStation);

/// The online state of every card looked at this session. Memory only.
#[derive(Default)]
pub(in crate::app) struct OnlineStore {
    entries: HashMap<u32, Entry>,
    /// Set once the station says it will not answer anybody (relay off, sign-in
    /// missing): after that no card is asked about.
    blocked: Option<OnlineState>,
    /// The question out, and when it went.
    in_flight: Option<(u32, i64)>,
    clock: u64,
}

impl OnlineStore {
    /// Forget that the station said no: the relay has just been switched on,
    /// so every card may ask again. Cards that did get an answer keep it.
    pub(in crate::app) fn unblock(&mut self) {
        self.blocked = None;
        self.in_flight = None;
        self.entries.retain(|_, e| matches!(e.state, OnlineState::Ready(_)));
    }

    #[cfg(test)]
    pub(in crate::app) fn len(&self) -> usize {
        self.entries.len()
    }

    /// Every card the relay has answered for this session, with its address.
    pub(in crate::app) fn infos(&self) -> impl Iterator<Item = (u32, &Info)> {
        self.entries.iter().filter_map(|(icao, e)| match &e.state {
            OnlineState::Ready(info) => Some((*icao, &**info)),
            _ => None,
        })
    }

    /// What to draw for this card.
    pub(in crate::app) fn state_for(&self, icao: u32, has_station: bool) -> &OnlineState {
        if let Some(Entry { state: s @ OnlineState::Ready(_), .. }) = self.entries.get(&icao) {
            return s;
        }
        if !has_station {
            return &NO_STATION;
        }
        if let Some(b) = &self.blocked {
            return b;
        }
        self.entries.get(&icao).map_or(&IDLE, |e| &e.state)
    }

    /// Whether a question is out and still being waited for. One at a time:
    /// while this is true no other aircraft can be asked about.
    pub(in crate::app) fn line_busy(&self, now: i64) -> bool {
        self.in_flight.is_some_and(|(_, at)| now - at < FLIGHT_TIMEOUT_S)
    }

    /// The question to send now, if any. Called every frame the card is open:
    /// it answers `Some` once, and then only when a back-off has run out.
    pub(in crate::app) fn next_request(
        &mut self,
        want: Option<&Want>,
        has_station: bool,
        now: i64,
    ) -> Option<Want> {
        let want = want?;
        if !has_station || self.blocked.is_some() {
            return None;
        }
        if let Some((_, at)) = self.in_flight {
            if now - at < FLIGHT_TIMEOUT_S {
                return None;
            }
            // Never came back. Let the line move; the card waits out a back-off.
            if let Some((lost, _)) = self.in_flight.take() {
                self.settle(lost, OnlineState::Unavailable(Reason::Offline), Some(now + RETRY_S));
            }
        }
        self.clock += 1;
        if let Some(e) = self.entries.get_mut(&want.icao) {
            e.touched = self.clock;
            let due = match (&e.state, e.retry_at) {
                (OnlineState::Unavailable(_), Some(t)) => now >= t,
                _ => false,
            };
            if !due {
                return None;
            }
        }
        self.settle(want.icao, OnlineState::Loading, None);
        self.in_flight = Some((want.icao, now));
        Some(want.clone())
    }

    /// Take an answer to the question for `icao`.
    pub(in crate::app) fn deliver(&mut self, icao: u32, answer: Answer, now: i64) {
        if self.in_flight.is_some_and(|(i, _)| i == icao) {
            self.in_flight = None;
        }
        match answer {
            Answer::Info(info) => self.settle(icao, OnlineState::Ready(info), None),
            Answer::Unavailable(r) => {
                if r.is_station_wide() {
                    self.blocked = Some(OnlineState::Unavailable(r));
                }
                self.settle(icao, OnlineState::Unavailable(r), r.retry_after().map(|s| now + s));
            }
        }
    }

    fn settle(&mut self, icao: u32, state: OnlineState, retry_at: Option<i64>) {
        self.clock += 1;
        self.entries.insert(icao, Entry { state, retry_at, touched: self.clock });
        while self.entries.len() > MAX_KEPT {
            let oldest = self
                .entries
                .iter()
                .filter(|(i, _)| self.in_flight.map(|(f, _)| f) != Some(**i) && **i != icao)
                .min_by_key(|(_, e)| e.touched)
                .map(|(i, _)| *i);
            match oldest {
                Some(i) => self.entries.remove(&i),
                None => break,
            };
        }
    }
}

/// Which provenance a line wears.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum Badge {
    /// Heard on the air: green, with its source and age beside it.
    Radio,
    /// From the registry or the photo service, through the station: blue.
    Online,
    /// From the station's own record of the last day: violet.
    History,
    /// Nobody has it: grey, with the reason.
    Missing,
}

/// One line of the card.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::app) struct Row {
    pub label: &'static str,
    pub value: String,
    pub badge: Badge,
    pub note: String,
}

const ONLINE_NOTE: &str = "adsbdb, through the station";

fn online_row(label: &'static str, value: String) -> Row {
    Row { label, value, badge: Badge::Online, note: ONLINE_NOTE.to_string() }
}

fn airport(a: &sdroxide_aircraft_online::Airport) -> String {
    match (a.code.is_empty(), a.name.is_empty()) {
        (false, false) => format!("{} — {}", a.code, a.name),
        (false, true) => a.code.clone(),
        _ => a.name.clone(),
    }
}

/// Every line of the open card: what the radio heard, in its own order and
/// untouched, then what the station's relay adds.
pub(in crate::app) fn rows(card: &AircraftCard, state: &OnlineState, now: i64) -> Vec<Row> {
    let mut out: Vec<Row> = aircraft_card::lines(card, now)
        .into_iter()
        .map(|l| Row {
            label: l.label,
            value: l.value,
            badge: if l.present { Badge::Radio } else { Badge::Missing },
            note: l.note,
        })
        .collect();
    match state {
        OnlineState::Ready(info) => {
            if let Some(a) = &info.aircraft {
                if let Some(reg) = &a.registration {
                    if let Some(r) = out.iter_mut().find(|r| r.label == "Registration") {
                        if r.badge == Badge::Missing {
                            *r = online_row("Registration", reg.clone());
                        }
                    }
                }
                let kind: Vec<&str> = [a.manufacturer.as_deref(), a.type_name.as_deref()]
                    .into_iter()
                    .flatten()
                    .collect();
                if !kind.is_empty() {
                    out.push(online_row("Type", kind.join(" ")));
                }
                if let Some(op) = &a.operator {
                    out.push(online_row("Operator", op.clone()));
                }
            }
            if let Some(route) = &info.route {
                if let Some(o) = &route.origin {
                    out.push(online_row("Origin", airport(o)));
                }
                if let Some(d) = &route.destination {
                    out.push(online_row("Destination", airport(d)));
                }
            }
        }
        OnlineState::Loading => out.push(Row {
            label: "Online",
            value: "…".to_string(),
            badge: Badge::Missing,
            note: "asking the station".to_string(),
        }),
        OnlineState::Unavailable(r) => {
            if let Some(note) = r.note() {
                out.push(Row {
                    label: "Online",
                    value: "—".to_string(),
                    badge: Badge::Missing,
                    note: note.to_string(),
                });
            }
        }
        OnlineState::Idle => {}
    }
    out
}

/// Where a card's data comes from, as the badge at its top says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum Provenance {
    Radio,
    RadioOnline,
    Online,
    Nothing,
}

impl Provenance {
    pub(in crate::app) fn label(self) -> &'static str {
        match self {
            Provenance::Radio => "radio",
            Provenance::RadioOnline => "radio + online",
            Provenance::Online => "online",
            Provenance::Nothing => "",
        }
    }
}

fn has_radio_data(card: &AircraftCard) -> bool {
    aircraft_card::lines(card, 0).iter().any(|l| l.present)
}

fn has_online_data(state: &OnlineState) -> bool {
    let OnlineState::Ready(info) = state else { return false };
    let a = info.aircraft.as_ref().is_some_and(|a| {
        a.type_name.is_some()
            || a.manufacturer.is_some()
            || a.operator.is_some()
            || a.registration.is_some()
    });
    let r = info.route.as_ref().is_some_and(|r| r.origin.is_some() || r.destination.is_some());
    a || r || info.photo.is_some()
}

pub(in crate::app) fn provenance(card: &AircraftCard, state: &OnlineState) -> Provenance {
    match (has_radio_data(card), has_online_data(state)) {
        (true, true) => Provenance::RadioOnline,
        (true, false) => Provenance::Radio,
        (false, true) => Provenance::Online,
        (false, false) => Provenance::Nothing,
    }
}

/// What loading the thumbnail has come to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum ThumbStatus {
    NotTried,
    Loading,
    Ready,
    Failed,
}

/// The photo block: the credit and the link always, the picture when it can be.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::app) struct PhotoView {
    /// Planespotters' address for the thumbnail, exactly as the relay gave it.
    pub thumb_url: String,
    pub show_image: bool,
    pub credit: String,
    /// The photo's page on Planespotters, when it is one.
    pub page_url: Option<String>,
}

fn host_of(url: &str) -> Option<&str> {
    let rest = url.strip_prefix("https://")?;
    let host = rest.split(['/', '?', '#']).next()?;
    (!host.is_empty() && !host.contains('@') && !host.contains(':')).then_some(host)
}

/// Whether this is a picture address Planespotters hands out. The relay checks
/// it too; a client does not trust a station to have done so before it opens a
/// connection to whatever the reply names.
pub(in crate::app) fn thumb_fetchable(url: &str) -> bool {
    host_of(url) == Some("t.plnspttrs.net")
}

fn page_ok(url: &str) -> bool {
    matches!(host_of(url), Some("www.planespotters.net" | "planespotters.net"))
}

pub(in crate::app) fn photo_view(state: &OnlineState, thumb: ThumbStatus) -> Option<PhotoView> {
    let OnlineState::Ready(info) = state else { return None };
    let Photo { thumb_url, page_url, photographer } = info.photo.as_ref()?;
    Some(PhotoView {
        thumb_url: thumb_url.clone(),
        show_image: thumb != ThumbStatus::Failed && thumb_fetchable(thumb_url),
        credit: format!("Photo: {photographer} / Planespotters"),
        page_url: page_ok(page_url).then(|| page_url.clone()),
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_switch_request_names_the_station_and_the_state_wanted() {
        assert_eq!(
            switch_url("wss://shack.example/sdr/ws/2", true).as_deref(),
            Some("https://shack.example/sdr/aircraft-online?on=1")
        );
        assert_eq!(
            switch_url("ws://shack.test:4950/ws", false).as_deref(),
            Some("http://shack.test:4950/aircraft-online?on=0")
        );
        assert_eq!(switch_url("not an address", true), None);
    }

    #[test]
    fn a_reply_to_the_switch_is_read_and_a_refusal_says_why() {
        assert_eq!(
            classify_switch(Some(200), r#"{"enabled":true,"forced":false}"#),
            SwitchAnswer::On
        );
        assert_eq!(
            classify_switch(Some(200), r#"{"enabled":false,"forced":false}"#),
            SwitchAnswer::Off
        );
        for (status, body) in [
            (None, ""),
            (Some(401), ""),
            (Some(403), ""),
            (Some(404), ""),
            (Some(200), "<html>"),
            (Some(500), ""),
        ] {
            assert!(
                matches!(classify_switch(status, body), SwitchAnswer::Refused(_)),
                "{status:?}"
            );
        }
    }

    #[test]
    fn switching_the_relay_on_lets_every_card_ask_again_and_keeps_the_answers_it_has() {
        let mut s = OnlineStore::default();
        let w = |icao| Want { icao, callsign: None, registration: None };
        assert!(s.next_request(Some(&w(1)), true, 0).is_some());
        s.deliver(1, Answer::Unavailable(Reason::Disabled), 1);
        assert_eq!(s.state_for(2, true), &OnlineState::Unavailable(Reason::Disabled));
        assert!(s.next_request(Some(&w(2)), true, 2).is_none(), "blocked while the relay is off");
        s.unblock();
        assert_eq!(s.state_for(2, true), &OnlineState::Idle);
        assert!(s.next_request(Some(&w(2)), true, 3).is_some(), "asks again after the switch");
    }
    use super::super::aircraft_card::lines;
    use super::super::aircraft_registry::{Source, Stamped};
    use super::*;
    use sdroxide_aircraft_online::{AircraftInfo, Airport, FORMAT_VERSION, Photo, Route, Status};

    const NOW: i64 = 1_000_000;

    #[test]
    fn the_line_is_busy_while_a_question_is_out_and_free_after_its_answer_or_timeout() {
        let mut st = OnlineStore::default();
        assert!(!st.line_busy(NOW));
        let w = Want { icao: 5, callsign: None, registration: None };
        assert!(st.next_request(Some(&w), true, NOW).is_some());
        assert!(st.line_busy(NOW + 1));
        assert!(!st.line_busy(NOW + FLIGHT_TIMEOUT_S), "a question that never came back");
        st.deliver(5, Answer::Unavailable(Reason::Offline), NOW + 2);
        assert!(!st.line_busy(NOW + 3));
    }

    fn stamped<T>(value: T, source: Source, age: i64) -> Option<Stamped<T>> {
        Some(Stamped { value, source, at: NOW - age })
    }

    fn heard() -> AircraftCard {
        AircraftCard {
            flight: stamped("KLM1234".to_string(), Source::Adsb, 12),
            registration: stamped("PH-BXA".to_string(), Source::Vdl2, 40),
            altitude_ft: stamped(35_000, Source::Adsb, 12),
            last_seen: NOW - 12,
            ..AircraftCard::default()
        }
    }

    fn no_registration() -> AircraftCard {
        AircraftCard { registration: None, ..heard() }
    }

    fn info() -> Info {
        let mut i = Info::bare(Status::Ok);
        i.aircraft = Some(AircraftInfo {
            type_name: Some("737-800".into()),
            manufacturer: Some("Boeing".into()),
            operator: Some("KLM".into()),
            registration: Some("PH-ONLINE".into()),
        });
        i.route = Some(Route {
            origin: Some(Airport { code: "EHAM".into(), name: "Amsterdam Schiphol".into() }),
            destination: Some(Airport { code: "EGLL".into(), name: "London Heathrow".into() }),
        });
        i.photo = Some(Photo {
            thumb_url: "https://t.plnspttrs.net/12345/abc_t.jpg".into(),
            page_url: "https://www.planespotters.net/photo/12345/x".into(),
            photographer: "A. Spotter".into(),
        });
        i
    }

    fn ready(i: Info) -> OnlineState {
        OnlineState::Ready(Box::new(i))
    }

    fn want(icao: u32) -> Want {
        Want { icao, callsign: None, registration: None }
    }

    fn row<'a>(rs: &'a [Row], label: &str) -> &'a Row {
        rs.iter().find(|r| r.label == label).unwrap_or_else(|| panic!("no {label} row"))
    }

    // ── what the card says ────────────────────────────────────────────────

    #[test]
    fn the_radio_lines_come_first_and_in_their_own_order() {
        let card = heard();
        let radio: Vec<&str> = lines(&card, NOW).iter().map(|l| l.label).collect();
        let all = rows(&card, &ready(info()), NOW);
        let head: Vec<&str> = all.iter().take(radio.len()).map(|r| r.label).collect();
        assert_eq!(head, radio);
        assert!(all.len() > radio.len(), "the online lines follow");
        assert!(all[radio.len()..].iter().all(|r| r.badge == Badge::Online));
    }

    #[test]
    fn what_the_radio_heard_wears_the_radio_badge_with_its_source_and_age() {
        let all = rows(&heard(), &ready(info()), NOW);
        let f = row(&all, "Flight");
        assert_eq!(f.badge, Badge::Radio);
        assert_eq!(f.value, "KLM1234");
        assert_eq!(f.note, "ADS-B, 12s ago");
        assert_eq!(row(&all, "Altitude").note, "ADS-B, 12s ago");
        // Not heard: grey, "—", and the reason stays.
        let p = row(&all, "Position");
        assert_eq!((p.badge, p.value.as_str()), (Badge::Missing, "—"));
        assert!(p.note.contains("not heard"), "{}", p.note);
    }

    #[test]
    fn the_registration_the_radio_heard_wins_over_the_online_one() {
        let all = rows(&heard(), &ready(info()), NOW);
        let r = row(&all, "Registration");
        assert_eq!((r.value.as_str(), r.badge), ("PH-BXA", Badge::Radio));
        assert_eq!(all.iter().filter(|r| r.label == "Registration").count(), 1);
    }

    #[test]
    fn an_online_registration_fills_the_gap_the_radio_left() {
        let all = rows(&no_registration(), &ready(info()), NOW);
        let r = row(&all, "Registration");
        assert_eq!((r.value.as_str(), r.badge), ("PH-ONLINE", Badge::Online));
        assert_eq!(all.iter().filter(|r| r.label == "Registration").count(), 1);
    }

    #[test]
    fn a_card_with_no_online_data_reads_exactly_as_the_radio_card_does() {
        for st in [OnlineState::Idle, OnlineState::Unavailable(Reason::Offline)] {
            let all = rows(&heard(), &st, NOW);
            let radio = lines(&heard(), NOW);
            assert_eq!(all.len(), radio.len(), "{st:?}");
            assert!(all.iter().all(|r| r.badge != Badge::Online), "{st:?}");
            for (r, l) in all.iter().zip(&radio) {
                assert_eq!((r.label, &r.value, &r.note), (l.label, &l.value, &l.note));
            }
        }
    }

    #[test]
    fn online_data_with_nothing_heard_leaves_the_radio_lines_grey() {
        let all = rows(&AircraftCard::default(), &ready(info()), NOW);
        assert_eq!(row(&all, "Flight").badge, Badge::Missing);
        assert_eq!(row(&all, "Type").badge, Badge::Online);
    }

    #[test]
    fn an_online_field_that_is_absent_adds_no_line() {
        let mut i = Info::bare(Status::Ok);
        i.aircraft = Some(AircraftInfo { type_name: Some("A320".into()), ..Default::default() });
        let all = rows(&heard(), &ready(i), NOW);
        assert_eq!(row(&all, "Type").value, "A320");
        for gone in ["Operator", "Origin", "Destination"] {
            assert!(all.iter().all(|r| r.label != gone), "{gone}");
        }
    }

    #[test]
    fn the_online_lines_are_worded_for_a_reader() {
        let all = rows(&heard(), &ready(info()), NOW);
        assert_eq!(row(&all, "Type").value, "Boeing 737-800");
        assert_eq!(row(&all, "Operator").value, "KLM");
        assert_eq!(row(&all, "Origin").value, "EHAM — Amsterdam Schiphol");
        assert_eq!(row(&all, "Destination").value, "EGLL — London Heathrow");
        assert_eq!(row(&all, "Operator").note, "adsbdb, through the station");
    }

    #[test]
    fn the_badge_at_the_top_says_radio_or_radio_plus_online() {
        let c = heard();
        assert_eq!(provenance(&c, &OnlineState::Idle), Provenance::Radio);
        assert_eq!(provenance(&c, &OnlineState::Loading), Provenance::Radio);
        assert_eq!(provenance(&c, &OnlineState::Unavailable(Reason::Disabled)), Provenance::Radio);
        assert_eq!(provenance(&c, &ready(Info::bare(Status::Ok))), Provenance::Radio);
        assert_eq!(provenance(&c, &ready(info())), Provenance::RadioOnline);
        assert_eq!(provenance(&AircraftCard::default(), &ready(info())), Provenance::Online);
        assert_eq!(provenance(&AircraftCard::default(), &OnlineState::Idle), Provenance::Nothing);
        assert_eq!(Provenance::Radio.label(), "radio");
        assert_eq!(Provenance::RadioOnline.label(), "radio + online");
        assert_eq!(Provenance::Online.label(), "online");
        assert_eq!(Provenance::Nothing.label(), "");
    }

    // ── what an answer means ──────────────────────────────────────────────

    fn body(status: Status) -> String {
        serde_json::to_string(&Info::bare(status)).unwrap()
    }

    #[test]
    fn every_way_an_answer_can_come_back_is_a_state_and_none_is_an_error() {
        let ok = serde_json::to_string(&info()).unwrap();
        assert_eq!(classify(Some(200), &ok), Answer::Info(Box::new(info())));
        let table: [(Option<u16>, String, Reason); 12] = [
            (Some(503), body(Status::Disabled), Reason::Disabled),
            (Some(429), body(Status::Busy), Reason::Busy),
            (Some(200), body(Status::Unknown), Reason::Unknown),
            (Some(200), body(Status::Unavailable), Reason::Offline),
            (Some(401), body(Status::Unauthorized), Reason::Unauthorized),
            (Some(400), body(Status::Invalid), Reason::Unknown),
            (None, String::new(), Reason::Offline),
            (Some(200), "not json".into(), Reason::Offline),
            (Some(503), "<html>bad gateway</html>".into(), Reason::Offline),
            (Some(401), String::new(), Reason::Unauthorized),
            (Some(429), String::new(), Reason::Busy),
            (Some(404), String::new(), Reason::Offline),
        ];
        for (code, b, want) in table {
            assert_eq!(classify(code, &b), Answer::Unavailable(want), "{code:?} {b}");
        }
    }

    #[test]
    fn a_reply_in_a_shape_this_client_does_not_know_is_a_quiet_failure() {
        let mut i = info();
        i.v = FORMAT_VERSION + 1;
        let b = serde_json::to_string(&i).unwrap();
        assert_eq!(classify(Some(200), &b), Answer::Unavailable(Reason::Offline));
    }

    #[test]
    fn only_a_switched_off_relay_or_a_missing_station_says_so_on_the_card() {
        assert!(Reason::Disabled.note().unwrap().contains("online unavailable"));
        assert!(Reason::NoStation.note().unwrap().contains("online unavailable"));
        assert!(Reason::Unauthorized.note().is_some());
        for quiet in [Reason::Offline, Reason::Busy, Reason::Unknown] {
            assert_eq!(quiet.note(), None, "{quiet:?}");
        }
        let all = rows(&heard(), &OnlineState::Unavailable(Reason::Disabled), NOW);
        let r = all.last().unwrap();
        assert_eq!((r.label, r.value.as_str(), r.badge), ("Online", "—", Badge::Missing));
        assert!(r.note.contains("online unavailable"), "{}", r.note);
        let quiet = rows(&heard(), &OnlineState::Unavailable(Reason::Busy), NOW);
        assert_eq!(quiet.len(), lines(&heard(), NOW).len());
    }

    #[test]
    fn a_question_in_flight_is_shown_as_such() {
        let all = rows(&heard(), &OnlineState::Loading, NOW);
        let r = all.last().unwrap();
        assert_eq!((r.label, r.badge), ("Online", Badge::Missing));
    }

    // ── what is asked, and when ───────────────────────────────────────────

    #[test]
    fn a_card_is_asked_about_once() {
        let mut s = OnlineStore::default();
        let w = want(0x4CA1FA);
        assert_eq!(s.next_request(Some(&w), true, NOW), Some(w.clone()));
        assert_eq!(s.state_for(w.icao, true), &OnlineState::Loading);
        // Redrawn every frame, asked once.
        for t in 0..100 {
            assert_eq!(s.next_request(Some(&w), true, NOW + t), None);
        }
        s.deliver(w.icao, Answer::Info(Box::new(info())), NOW + 1);
        assert!(matches!(s.state_for(w.icao, true), OnlineState::Ready(_)));
        assert_eq!(s.next_request(Some(&w), true, NOW + 1_000_000), None, "an answer is kept");
    }

    #[test]
    fn nothing_is_asked_without_an_open_card_an_icao_or_a_station() {
        let mut s = OnlineStore::default();
        assert_eq!(s.next_request(None, true, NOW), None);
        assert_eq!(s.next_request(Some(&want(1)), false, NOW), None);
        assert_eq!(s.state_for(1, false), &OnlineState::Unavailable(Reason::NoStation));
        // The same address, later, with a station: not poisoned by the first look.
        assert!(s.next_request(Some(&want(1)), true, NOW).is_some());
        let fallback =
            CardKey::Fallback(super::super::aircraft_registry::FallbackKey::AircraftId(7));
        assert_eq!(want_for(&fallback, &heard()), None);
    }

    #[test]
    fn a_switched_off_relay_is_not_asked_again_for_any_card_all_session() {
        let mut s = OnlineStore::default();
        assert!(s.next_request(Some(&want(1)), true, NOW).is_some());
        s.deliver(1, Answer::Unavailable(Reason::Disabled), NOW);
        for t in [0, 1, RETRY_S, RETRY_S * 100] {
            assert_eq!(s.next_request(Some(&want(1)), true, NOW + t), None);
            assert_eq!(s.next_request(Some(&want(2)), true, NOW + t), None, "another card too");
        }
        assert_eq!(s.state_for(2, true), &OnlineState::Unavailable(Reason::Disabled));
    }

    #[test]
    fn a_missing_sign_in_is_not_asked_again_either() {
        let mut s = OnlineStore::default();
        assert!(s.next_request(Some(&want(1)), true, NOW).is_some());
        s.deliver(1, Answer::Unavailable(Reason::Unauthorized), NOW);
        assert_eq!(s.next_request(Some(&want(2)), true, NOW + RETRY_S * 10), None);
    }

    #[test]
    fn a_failure_busy_or_unknown_is_asked_again_only_after_the_back_off() {
        for r in [Reason::Offline, Reason::Busy, Reason::Unknown] {
            let mut s = OnlineStore::default();
            let w = want(5);
            assert!(s.next_request(Some(&w), true, NOW).is_some());
            s.deliver(5, Answer::Unavailable(r), NOW);
            assert_eq!(s.next_request(Some(&w), true, NOW), None, "{r:?}");
            assert_eq!(s.next_request(Some(&w), true, NOW + RETRY_S - 1), None, "{r:?}");
            assert!(s.next_request(Some(&w), true, NOW + RETRY_S).is_some(), "{r:?}");
            // And again: each failure restarts the wait.
            s.deliver(5, Answer::Unavailable(r), NOW + RETRY_S);
            assert_eq!(s.next_request(Some(&w), true, NOW + RETRY_S + 1), None, "{r:?}");
        }
        assert_eq!(Reason::Offline.retry_after(), Some(RETRY_S));
        assert_eq!(Reason::Disabled.retry_after(), None);
        assert_eq!(Reason::Unauthorized.retry_after(), None);
        assert_eq!(Reason::NoStation.retry_after(), None);
    }

    #[test]
    fn one_question_is_out_at_a_time() {
        let mut s = OnlineStore::default();
        assert!(s.next_request(Some(&want(1)), true, NOW).is_some());
        assert_eq!(s.next_request(Some(&want(2)), true, NOW + 1), None);
        s.deliver(1, Answer::Unavailable(Reason::Offline), NOW + 2);
        assert!(s.next_request(Some(&want(2)), true, NOW + 3).is_some());
    }

    #[test]
    fn a_question_that_never_comes_back_does_not_hold_the_line_for_ever() {
        let mut s = OnlineStore::default();
        assert!(s.next_request(Some(&want(1)), true, NOW).is_some());
        assert_eq!(s.next_request(Some(&want(2)), true, NOW + FLIGHT_TIMEOUT_S - 1), None);
        assert!(s.next_request(Some(&want(2)), true, NOW + FLIGHT_TIMEOUT_S).is_some());
        // The lost one is not asked again at once either.
        assert_eq!(s.next_request(Some(&want(1)), true, NOW + FLIGHT_TIMEOUT_S + 1), None);
    }

    #[test]
    fn what_is_kept_is_bounded() {
        let mut s = OnlineStore::default();
        for i in 0..(MAX_KEPT as u32 + 20) {
            let t = NOW + i as i64;
            assert!(s.next_request(Some(&want(i)), true, t).is_some());
            s.deliver(i, Answer::Info(Box::new(info())), t);
        }
        assert!(s.len() <= MAX_KEPT);
        assert!(matches!(s.state_for(MAX_KEPT as u32 + 19, true), OnlineState::Ready(_)));
    }

    #[test]
    fn the_question_carries_only_a_clean_callsign_and_registration() {
        let k = CardKey::Icao(0x4CA1FA);
        let w = want_for(&k, &heard()).unwrap();
        assert_eq!(
            w,
            Want {
                icao: 0x4CA1FA,
                callsign: Some("KLM1234".into()),
                registration: Some("PH-BXA".into())
            }
        );
        let mut dirty = heard();
        dirty.flight = stamped("KLM 12&x=1".to_string(), Source::Adsb, 1);
        dirty.registration = stamped("../etc".to_string(), Source::Vdl2, 1);
        let w = want_for(&k, &dirty).unwrap();
        assert_eq!((w.callsign, w.registration), (None, None));
    }

    // ── where it is asked ─────────────────────────────────────────────────

    #[test]
    fn the_question_goes_to_the_station_the_client_is_connected_to() {
        let w = Want {
            icao: 0x4CA1FA,
            callsign: Some("KLM1234".into()),
            registration: Some("PH-BXA".into()),
        };
        assert_eq!(
            info_url("ws://shack.test:4950/ws", &w).as_deref(),
            Some("http://shack.test:4950/aircraft-info/4CA1FA?callsign=KLM1234&reg=PH-BXA")
        );
        // Behind a reverse proxy with a prefix, and on a radio other than the first.
        assert_eq!(
            info_url("wss://host.test/shack/ws/2", &want(0xAB)).as_deref(),
            Some("https://host.test/shack/aircraft-info/0000AB")
        );
        assert_eq!(info_url("https://host.test", &want(1)), None, "not a socket address");
        assert_eq!(info_url("", &want(1)), None);
    }

    // ── the photo ─────────────────────────────────────────────────────────

    #[test]
    fn a_photo_has_its_credit_and_its_page_and_its_address_untouched() {
        let v = photo_view(&ready(info()), ThumbStatus::NotTried).unwrap();
        assert_eq!(v.thumb_url, "https://t.plnspttrs.net/12345/abc_t.jpg");
        assert!(v.show_image);
        assert!(
            v.credit.contains("A. Spotter") && v.credit.contains("Planespotters"),
            "{}",
            v.credit
        );
        assert_eq!(v.page_url.as_deref(), Some("https://www.planespotters.net/photo/12345/x"));
        assert!(photo_view(&ready(Info::bare(Status::Ok)), ThumbStatus::NotTried).is_none());
        assert!(photo_view(&OnlineState::Loading, ThumbStatus::NotTried).is_none());
    }

    #[test]
    fn a_picture_that_cannot_be_shown_leaves_the_credit_and_the_link() {
        let v = photo_view(&ready(info()), ThumbStatus::Failed).unwrap();
        assert!(!v.show_image);
        assert!(v.credit.contains("A. Spotter"));
        assert!(v.page_url.is_some());
        for ok in [ThumbStatus::NotTried, ThumbStatus::Loading, ThumbStatus::Ready] {
            assert!(photo_view(&ready(info()), ok).unwrap().show_image, "{ok:?}");
        }
    }

    #[test]
    fn only_planespotters_own_addresses_are_loaded_or_opened() {
        for bad in [
            "",
            "http://t.plnspttrs.net/a.jpg",
            "https://evil.example/a.jpg",
            "https://t.plnspttrs.net.evil.example/a.jpg",
            "https://user@t.plnspttrs.net/a.jpg",
            "https://t.plnspttrs.net:8443/a.jpg",
        ] {
            assert!(!thumb_fetchable(bad), "{bad}");
            let mut i = info();
            i.photo.as_mut().unwrap().thumb_url = bad.to_string();
            let v = photo_view(&ready(i), ThumbStatus::NotTried).unwrap();
            assert!(!v.show_image, "{bad}");
            assert!(v.credit.contains("A. Spotter"), "the credit stays");
        }
        assert!(thumb_fetchable("https://t.plnspttrs.net/12345/abc_t.jpg"));
        let mut i = info();
        i.photo.as_mut().unwrap().page_url = "https://evil.example/photo".into();
        assert_eq!(photo_view(&ready(i), ThumbStatus::Ready).unwrap().page_url, None);
    }

    // ── what this client must not do ──────────────────────────────────────

    /// The files that make up this feature on the client.
    fn sources() -> [(&'static str, &'static str); 3] {
        [
            ("aircraft_online.rs", include_str!("aircraft_online.rs")),
            ("aircraft_online_io.rs", include_str!("aircraft_online_io.rs")),
            ("aircraft.rs", include_str!("aircraft.rs")),
        ]
    }

    /// The code of a file: its comments and its test module are not what ships.
    fn shipped(src: &str) -> String {
        let code = src.split("#[cfg(test)]").next().unwrap_or(src);
        code.lines().filter(|l| !l.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n")
    }

    #[test]
    fn nothing_is_kept_in_the_browser_or_in_the_settings() {
        let banned = [
            concat!("local", "Storage"),
            concat!("session", "Storage"),
            concat!("Indexed", "DB"),
            concat!("indexed", "_db"),
            concat!("local", "_storage"),
            concat!("session", "_storage"),
            concat!("web_sys::", "Storage"),
            concat!("set_", "string("),
            concat!("set_", "value("),
            concat!("Cache", "Storage"),
        ];
        for (name, src) in sources() {
            let code = shipped(src);
            for b in banned {
                assert!(!code.contains(b), "{name} touches {b}");
            }
        }
    }

    /// The settings structure itself carries no field for it. The test that
    /// matters is the one in `app/mod.rs`, which saves a whole app holding an
    /// online answer and reads every stored value back.
    #[test]
    fn the_settings_structure_has_no_field_for_the_online_state() {
        let saved = ron::to_string(&sdroxide_types::UiSettings::default()).unwrap();
        for word in ["online", "Online", "planespotters", "adsbdb", "thumb"] {
            assert!(!saved.contains(word), "UiSettings mentions {word}");
        }
    }

    #[test]
    fn no_client_asks_the_registry_or_the_photo_service_and_none_carries_a_key() {
        let banned = [
            concat!("api.", "adsbdb"),
            concat!("api.", "planespotters"),
            concat!("planespotters.net/", "pub"),
            concat!("api_", "key"),
            concat!("api", "key"),
            concat!("x-api-", "key"),
            concat!("Bear", "er "),
            concat!("Request", "::Adsbdb"),
            concat!("Request", "::Photo"),
            concat!("features = [\"", "fetch"),
        ];
        for (name, src) in sources() {
            let code = shipped(src);
            for b in banned {
                assert!(!code.contains(b), "{name} contains {b}");
            }
        }
        let manifest = include_str!("../../Cargo.toml");
        assert!(
            !manifest.contains(concat!("sdroxide-aircraft-online\", features")),
            "the client must not link the half that talks to the network"
        );
    }
}
