//! The asking: one question to the station's relay, one thumbnail from
//! Planespotters' picture host, and the way back to the window.
//!
//! The decisions are [`super::aircraft_online`]'s; this only sends what it
//! says to send and hands back what arrives, over a channel the window drains
//! every frame. Native sends from a thread, the browser from a `fetch`.
//!
//! What goes out, and only this:
//!
//! * `GET {station}/aircraft-info/{icao}` — the station's own relay. The
//!   browser sends its cookie by itself; the native remote client signs in to
//!   the station with the password it already holds, once, and keeps the
//!   cookie it gets for the length of the run.
//! * `GET` of the thumbnail address the relay returned, unchanged, with no
//!   cookie and no referrer. It is the one request that goes anywhere but the
//!   station, and the one Planespotters' terms ask the client to make.
//!
//! The picture is decoded and drawn from memory and dropped with the window;
//! nothing is written to disk, to the browser's storage, or to the settings.

use std::sync::mpsc::{Receiver, Sender, channel};

use eframe::egui;

use super::aircraft_export::ExportStore;
use super::aircraft_history::{self, HistoryStore};
use super::aircraft_online::{
    Answer, OnlineStore, ThumbStatus, Want, classify, info_url, thumb_fetchable,
};
use super::aircraft_stats::{self, StatsStore};
use super::aircraft_watch::{self, WatchStore};

/// The most bytes read of a reply from the relay.
const MAX_INFO_BYTES: u64 = 64 * 1024;
/// The most bytes read of the station's record of one aircraft: up to 600
/// positions, a few dozen bytes each.
const MAX_HISTORY_BYTES: u64 = 256 * 1024;
/// The most bytes read of the station's table of the fleet: up to 4000 aircraft,
/// a few hundred bytes each.
const MAX_EXPORT_BYTES: u64 = 8 * 1024 * 1024;
/// The most bytes read of the operator's watch list: a hundred short entries.
const MAX_WATCH_BYTES: u64 = 16 * 1024;
/// The most bytes read of the station's list of everything it kept: up to 4000
/// aircraft, a few hundred bytes each.
const MAX_STATS_BYTES: u64 = 4 * 1024 * 1024;
/// The most bytes read of a thumbnail. They are a few tens of kilobytes.
const MAX_THUMB_BYTES: u64 = 2 * 1024 * 1024;
/// Widest or tallest a thumbnail may decode to.
const MAX_THUMB_SIDE: u32 = 2048;
/// Thumbnails kept at once: the open card's, and a few it was just on.
const KEEP_THUMBS: usize = 8;

enum Msg {
    Info { icao: u32, status: Option<u16>, body: String },
    History { icao: u32, status: Option<u16>, body: String },
    Export { status: Option<u16>, body: String },
    Watch { status: Option<u16>, body: String },
    Stats { status: Option<u16>, body: String },
    Thumb { url: String, bytes: Option<Vec<u8>> },
}

struct Thumb {
    url: String,
    status: ThumbStatus,
    texture: Option<egui::TextureHandle>,
}

/// The online half of the AIRCRAFT window, held for the session in memory.
pub(in crate::app) struct OnlineClient {
    pub store: OnlineStore,
    /// The station's record of the one aircraft whose history is on.
    pub history: HistoryStore,
    /// Saving the fleet as CSV or JSON: the station's table, or one made here.
    pub export: ExportStore,
    /// The operator's list of aircraft to be told about, as the station keeps it.
    pub watch: WatchStore,
    /// The station's list of every aircraft it kept, for the statistics view.
    pub stats: StatsStore,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    thumbs: Vec<Thumb>,
}

impl Default for OnlineClient {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self {
            store: OnlineStore::default(),
            history: HistoryStore::default(),
            export: ExportStore::default(),
            watch: WatchStore::default(),
            stats: StatsStore::default(),
            tx,
            rx,
            thumbs: Vec::new(),
        }
    }
}

/// A thumbnail as pixels, or `None` for anything that is not a small picture.
pub(in crate::app) fn decode_thumb(bytes: &[u8]) -> Option<egui::ColorImage> {
    if bytes.is_empty() || bytes.len() as u64 > MAX_THUMB_BYTES {
        return None;
    }
    let img = image::load_from_memory(bytes).ok()?;
    if img.width() == 0
        || img.height() == 0
        || img.width() > MAX_THUMB_SIDE
        || img.height() > MAX_THUMB_SIDE
    {
        return None;
    }
    let rgba = img.to_rgba8();
    Some(egui::ColorImage::from_rgba_unmultiplied(
        [rgba.width() as usize, rgba.height() as usize],
        rgba.as_raw(),
    ))
}

impl OnlineClient {
    /// Take what has arrived since the last frame.
    pub(in crate::app) fn poll(&mut self, ctx: &egui::Context, now: i64) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Msg::Info { icao, status, body } => {
                    self.store.deliver(icao, classify(status, &body), now);
                }
                Msg::History { icao, status, body } => {
                    self.history.deliver(icao, aircraft_history::classify(status, &body), now);
                }
                Msg::Export { status, body } => {
                    self.export.deliver(status, &body, now);
                }
                Msg::Watch { status, body } => {
                    self.watch.deliver(aircraft_watch::classify(status, &body), now);
                }
                Msg::Stats { status, body } => {
                    self.stats.deliver(aircraft_stats::classify(status, &body), now);
                }
                Msg::Thumb { url, bytes } => {
                    let image = bytes.as_deref().and_then(decode_thumb);
                    if let Some(t) = self.thumbs.iter_mut().find(|t| t.url == url) {
                        match image {
                            Some(ci) => {
                                t.texture = Some(ctx.load_texture(
                                    format!("aircraft_thumb_{}", t.url),
                                    ci,
                                    egui::TextureOptions::LINEAR,
                                ));
                                t.status = ThumbStatus::Ready;
                            }
                            None => t.status = ThumbStatus::Failed,
                        }
                    }
                }
            }
        }
    }

    /// Send the question the store says is due, if there is one. `peer` is the
    /// address the radio is connected to; without one there is nobody to ask.
    pub(in crate::app) fn ask(
        &mut self,
        ctx: &egui::Context,
        peer: Option<&str>,
        want: Option<&Want>,
        now: i64,
    ) {
        let Some(w) = self.store.next_request(want, peer.is_some(), now) else { return };
        let Some(url) = peer.and_then(|p| info_url(p, &w)) else {
            // A connection that is not an address cannot be asked.
            self.store.deliver(
                w.icao,
                Answer::Unavailable(super::aircraft_online::Reason::NoStation),
                now,
            );
            return;
        };
        let station = peer.map(crate::login::station_key).unwrap_or_default();
        send_info(url, station, Kind::Info(w.icao), None, self.tx.clone(), ctx.clone());
    }

    /// Fetch the station's record of the aircraft whose history is on, if the
    /// store says one is due. Without a station, or with nothing on, nothing
    /// is sent.
    pub(in crate::app) fn ask_history(
        &mut self,
        ctx: &egui::Context,
        peer: Option<&str>,
        now: i64,
    ) {
        let Some(icao) = self.history.next_request(peer.is_some(), now) else { return };
        let Some(url) = peer.and_then(|p| aircraft_history::history_url(p, icao)) else {
            // A connection that is not an address cannot be asked.
            self.history.deliver(
                icao,
                aircraft_history::Answer::Unavailable(aircraft_history::Reason::NoStation),
                now,
            );
            return;
        };
        let station = peer.map(crate::login::station_key).unwrap_or_default();
        send_info(url, station, Kind::History(icao), None, self.tx.clone(), ctx.clone());
    }

    /// Send what the watch list's store has due: a change the person asked for,
    /// or a read of the list. Without a station nothing is sent.
    pub(in crate::app) fn ask_watch(&mut self, ctx: &egui::Context, peer: Option<&str>, now: i64) {
        let Some(req) = self.watch.next_request(peer.is_some(), now) else { return };
        let Some(url) = peer.and_then(aircraft_watch::watch_url) else {
            // A connection that is not an address cannot be asked.
            self.watch.deliver(
                aircraft_watch::Answer::Unavailable(aircraft_history::Reason::NoStation),
                now,
            );
            return;
        };
        let station = peer.map(crate::login::station_key).unwrap_or_default();
        let post = aircraft_watch::body_of(&req);
        send_info(url, station, Kind::Watch, post, self.tx.clone(), ctx.clone());
    }

    /// Fetch the station's list for the statistics view, if the store says one
    /// is due. Closed, or without a station, nothing is sent.
    pub(in crate::app) fn ask_stats(&mut self, ctx: &egui::Context, peer: Option<&str>, now: i64) {
        if !self.stats.next_request(peer.is_some(), now) {
            return;
        }
        let Some(url) = peer.and_then(aircraft_stats::list_url) else {
            // A connection that is not an address cannot be asked.
            self.stats.deliver(
                aircraft_stats::StatsState::Unavailable(aircraft_history::Reason::NoStation),
                now,
            );
            return;
        };
        let station = peer.map(crate::login::station_key).unwrap_or_default();
        send_info(url, station, Kind::Stats, None, self.tx.clone(), ctx.clone());
    }

    /// Ask the station for its table if an export was pressed and one is due. With
    /// no station nothing is sent: the file is made here.
    pub(in crate::app) fn ask_export(&mut self, ctx: &egui::Context, peer: Option<&str>, now: i64) {
        let Some(format) = self.export.next_fetch(peer.is_some(), now) else { return };
        let Some(url) = peer.and_then(|p| super::aircraft_export::export_url(p, format)) else {
            // A connection that is not an address cannot be asked.
            self.export.give_up(aircraft_history::Reason::NoStation);
            return;
        };
        let station = peer.map(crate::login::station_key).unwrap_or_default();
        send_info(url, station, Kind::Export, None, self.tx.clone(), ctx.clone());
    }

    pub(in crate::app) fn thumb_status(&self, url: &str) -> ThumbStatus {
        self.thumbs.iter().find(|t| t.url == url).map_or(ThumbStatus::NotTried, |t| t.status)
    }

    pub(in crate::app) fn thumb_texture(&self, url: &str) -> Option<&egui::TextureHandle> {
        self.thumbs.iter().find(|t| t.url == url).and_then(|t| t.texture.as_ref())
    }

    /// Start loading this thumbnail if it has not been tried and is an address
    /// Planespotters hands out.
    pub(in crate::app) fn want_thumb(&mut self, ctx: &egui::Context, url: &str) {
        if !thumb_fetchable(url) || self.thumbs.iter().any(|t| t.url == url) {
            return;
        }
        if self.thumbs.len() >= KEEP_THUMBS {
            self.thumbs.remove(0);
        }
        self.thumbs.push(Thumb {
            url: url.to_string(),
            status: ThumbStatus::Loading,
            texture: None,
        });
        send_thumb(url.to_string(), self.tx.clone(), ctx.clone());
    }
}

// ── native ────────────────────────────────────────────────────────────────

/// Cookies the stations handed out this run, by station. Memory only.
#[cfg(not(target_arch = "wasm32"))]
static COOKIES: std::sync::LazyLock<std::sync::Mutex<std::collections::BTreeMap<String, String>>> =
    std::sync::LazyLock::new(Default::default);

#[cfg(not(target_arch = "wasm32"))]
fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(std::time::Duration::from_secs(6)))
        .timeout_global(Some(std::time::Duration::from_secs(12)))
        .http_status_as_error(false)
        .build()
        .into()
}

/// Which question a reply answers: the relay's card, or the station's record.
#[derive(Clone, Copy)]
enum Kind {
    Info(u32),
    History(u32),
    Export,
    Watch,
    Stats,
}

impl Kind {
    fn limit(self) -> u64 {
        match self {
            Kind::Info(_) => MAX_INFO_BYTES,
            Kind::History(_) => MAX_HISTORY_BYTES,
            Kind::Export => MAX_EXPORT_BYTES,
            Kind::Watch => MAX_WATCH_BYTES,
            Kind::Stats => MAX_STATS_BYTES,
        }
    }

    fn message(self, status: Option<u16>, body: String) -> Msg {
        match self {
            Kind::Info(icao) => Msg::Info { icao, status, body },
            Kind::History(icao) => Msg::History { icao, status, body },
            Kind::Export => Msg::Export { status, body },
            Kind::Watch => Msg::Watch { status, body },
            Kind::Stats => Msg::Stats { status, body },
        }
    }
}

/// `GET` — or `POST` of a JSON `post` — with an optional cookie: the status and
/// the body, or `None` for no answer at all.
#[cfg(not(target_arch = "wasm32"))]
fn get_info(
    agent: &ureq::Agent,
    url: &str,
    cookie: Option<&str>,
    limit: u64,
    post: Option<&str>,
) -> Option<(u16, String)> {
    let mut resp = match post {
        None => {
            let mut req = agent.get(url);
            if let Some(c) = cookie {
                req = req.header("Cookie", c);
            }
            req.call().ok()?
        }
        Some(body) => {
            let mut req = agent.post(url).header("content-type", "application/json");
            if let Some(c) = cookie {
                req = req.header("Cookie", c);
            }
            req.send(body.to_string()).ok()?
        }
    };
    let status = resp.status().as_u16();
    let body = resp.body_mut().with_config().limit(limit).read_to_string().unwrap_or_default();
    Some((status, body))
}

/// Trade the password this client already signed in with for the station's
/// cookie, the way a browser does.
#[cfg(not(target_arch = "wasm32"))]
fn sign_in(agent: &ureq::Agent, station: &str) -> Option<String> {
    let access = crate::login::session_access(station)?;
    let url = format!("{}/signin", crate::login::http_base(station)?);
    let body = serde_json::json!({
        "username": access.username,
        "password": access.password,
        "remember": false,
    })
    .to_string();
    let resp = agent.post(&url).header("content-type", "application/json").send(body).ok()?;
    if resp.status().as_u16() != 200 {
        return None;
    }
    let set = resp.headers().get("set-cookie")?.to_str().ok()?;
    Some(set.split(';').next()?.trim().to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn send_info(
    url: String,
    station: String,
    kind: Kind,
    post: Option<String>,
    tx: Sender<Msg>,
    ctx: egui::Context,
) {
    std::thread::spawn(move || {
        let agent = agent();
        let cookie = COOKIES.lock().unwrap_or_else(|e| e.into_inner()).get(&station).cloned();
        let mut got = get_info(&agent, &url, cookie.as_deref(), kind.limit(), post.as_deref());
        if got.as_ref().is_some_and(|(s, _)| *s == 401) {
            if let Some(c) = sign_in(&agent, &station) {
                COOKIES.lock().unwrap_or_else(|e| e.into_inner()).insert(station, c.clone());
                got = get_info(&agent, &url, Some(&c), kind.limit(), post.as_deref());
            }
        }
        let (status, body) = match got {
            Some((s, b)) => (Some(s), b),
            None => (None, String::new()),
        };
        let _ = tx.send(kind.message(status, body));
        ctx.request_repaint();
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn send_thumb(url: String, tx: Sender<Msg>, ctx: egui::Context) {
    std::thread::spawn(move || {
        let bytes =
            agent().get(&url).call().ok().filter(|r| r.status().as_u16() == 200).and_then(
                |mut r| r.body_mut().with_config().limit(MAX_THUMB_BYTES).read_to_vec().ok(),
            );
        let _ = tx.send(Msg::Thumb { url, bytes });
        ctx.request_repaint();
    });
}

// ── browser ───────────────────────────────────────────────────────────────

/// `fetch` and hand back the response. `info` is the station's own relay,
/// same-origin with its cookie; otherwise it is the picture host, which gets
/// neither a cookie nor a referrer, and which the browser may refuse for want
/// of CORS headers — that is a failed picture, not an error.
#[cfg(target_arch = "wasm32")]
async fn fetch(url: &str, info: bool, post: Option<&str>) -> Option<web_sys::Response> {
    use wasm_bindgen::JsCast;
    let window = web_sys::window()?;
    let init = web_sys::RequestInit::new();
    match post {
        None => init.set_method("GET"),
        Some(body) => {
            init.set_method("POST");
            let headers = web_sys::Headers::new().ok()?;
            headers.set("content-type", "application/json").ok()?;
            init.set_headers(&headers);
            init.set_body(&wasm_bindgen::JsValue::from_str(body));
        }
    }
    if info {
        init.set_mode(web_sys::RequestMode::SameOrigin);
    } else {
        init.set_mode(web_sys::RequestMode::Cors);
        init.set_credentials(web_sys::RequestCredentials::Omit);
        init.set_referrer_policy(web_sys::ReferrerPolicy::NoReferrer);
    }
    let request = web_sys::Request::new_with_str_and_init(url, &init).ok()?;
    let response =
        wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request)).await.ok()?;
    response.dyn_into::<web_sys::Response>().ok()
}

#[cfg(target_arch = "wasm32")]
fn send_info(
    url: String,
    _station: String,
    kind: Kind,
    post: Option<String>,
    tx: Sender<Msg>,
    ctx: egui::Context,
) {
    wasm_bindgen_futures::spawn_local(async move {
        let (status, body) = match fetch(&url, true, post.as_deref()).await {
            Some(r) => {
                let status = r.status();
                let text = match r.text() {
                    Ok(p) => wasm_bindgen_futures::JsFuture::from(p).await.ok(),
                    Err(_) => None,
                };
                (Some(status), text.and_then(|t| t.as_string()).unwrap_or_default())
            }
            None => (None, String::new()),
        };
        let body = if body.len() as u64 > kind.limit() { String::new() } else { body };
        let _ = tx.send(kind.message(status, body));
        ctx.request_repaint();
    });
}

#[cfg(target_arch = "wasm32")]
fn send_thumb(url: String, tx: Sender<Msg>, ctx: egui::Context) {
    wasm_bindgen_futures::spawn_local(async move {
        let bytes = async {
            let r = fetch(&url, false, None).await?;
            if r.status() != 200 {
                return None;
            }
            let buf = wasm_bindgen_futures::JsFuture::from(r.array_buffer().ok()?).await.ok()?;
            let bytes = js_sys::Uint8Array::new(&buf).to_vec();
            (bytes.len() as u64 <= MAX_THUMB_BYTES).then_some(bytes)
        }
        .await;
        let _ = tx.send(Msg::Thumb { url, bytes });
        ctx.request_repaint();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(w, h, image::Rgba([10, 200, 30, 255]));
        let mut out = std::io::Cursor::new(Vec::new());
        img.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn a_thumbnail_is_decoded_from_memory_to_pixels() {
        let ci = decode_thumb(&png(4, 3)).expect("a PNG decodes");
        assert_eq!(ci.size, [4, 3]);
        assert_eq!(ci.pixels[0], egui::Color32::from_rgb(10, 200, 30));
    }

    #[test]
    fn anything_that_is_not_a_small_picture_is_no_picture() {
        assert!(decode_thumb(&[]).is_none());
        assert!(decode_thumb(b"<html>not a picture</html>").is_none());
        assert!(decode_thumb(&png(MAX_THUMB_SIDE + 1, 1)).is_none(), "too wide");
        assert!(decode_thumb(&vec![0u8; MAX_THUMB_BYTES as usize + 1]).is_none(), "too big");
    }

    #[test]
    fn a_thumbnail_that_failed_is_not_asked_for_again() {
        let ctx = egui::Context::default();
        let mut c = OnlineClient::default();
        let url = "https://t.plnspttrs.net/a_t.jpg";
        c.want_thumb(&ctx, url);
        assert_eq!(c.thumb_status(url), ThumbStatus::Loading);
        c.tx.send(Msg::Thumb { url: url.into(), bytes: Some(b"nope".to_vec()) }).unwrap();
        c.poll(&ctx, 0);
        assert_eq!(c.thumb_status(url), ThumbStatus::Failed);
        c.want_thumb(&ctx, url);
        assert_eq!(c.thumb_status(url), ThumbStatus::Failed, "not retried");
        assert!(c.thumb_texture(url).is_none());
    }

    #[test]
    fn a_thumbnail_address_that_is_not_planespotters_is_never_requested() {
        let ctx = egui::Context::default();
        let mut c = OnlineClient::default();
        c.want_thumb(&ctx, "https://evil.example/a.jpg");
        assert_eq!(c.thumb_status("https://evil.example/a.jpg"), ThumbStatus::NotTried);
    }

    #[test]
    fn a_reply_from_the_station_reaches_the_store() {
        let ctx = egui::Context::default();
        let mut c = OnlineClient::default();
        let w = Want { icao: 7, callsign: None, registration: None };
        assert!(c.store.next_request(Some(&w), true, 0).is_some());
        let body = serde_json::to_string(&sdroxide_aircraft_online::Info::bare(
            sdroxide_aircraft_online::Status::Disabled,
        ))
        .unwrap();
        c.tx.send(Msg::Info { icao: 7, status: Some(503), body }).unwrap();
        c.poll(&ctx, 1);
        assert_eq!(
            c.store.state_for(7, true),
            &super::super::aircraft_online::OnlineState::Unavailable(
                super::super::aircraft_online::Reason::Disabled
            )
        );
    }

    /// **This test cannot catch a regression of the `peer.is_some()` guard.**
    /// Without the guard the store would send the question, `info_url` would
    /// find no address to build it from, and `ask` would answer `NoStation` at
    /// once — nothing sent and the same state drawn, which is all this checks.
    /// It pins that outcome, not the path to it; the guard is only a shortcut
    /// past a request that could not have been built anyway.
    #[test]
    fn a_station_the_client_is_not_connected_to_is_not_asked_at_all() {
        let ctx = egui::Context::default();
        let mut c = OnlineClient::default();
        let w = Want { icao: 9, callsign: None, registration: None };
        c.ask(&ctx, None, Some(&w), 0);
        assert!(c.rx.try_recv().is_err(), "nothing was sent");
        assert_eq!(
            c.store.state_for(9, false),
            &super::super::aircraft_online::OnlineState::Unavailable(
                super::super::aircraft_online::Reason::NoStation
            )
        );
    }

    #[test]
    fn a_history_reply_from_the_station_reaches_the_history_store() {
        let ctx = egui::Context::default();
        let mut c = OnlineClient::default();
        c.history.set_on(Some(7));
        assert_eq!(c.history.next_request(true, 0), Some(7));
        let body = serde_json::to_string(&sdroxide_aircraft_history::AircraftReply::bare(
            sdroxide_aircraft_history::Status::Disabled,
        ))
        .unwrap();
        c.tx.send(Msg::History { icao: 7, status: Some(503), body }).unwrap();
        c.poll(&ctx, 1);
        assert_eq!(
            c.history.state(7),
            &aircraft_history::HistoryState::Unavailable(aircraft_history::Reason::Disabled)
        );
        assert!(!c.history.line_busy(1), "the answer freed the line");
        // The relay's store is a different thing and was not touched.
        assert_eq!(c.store.len(), 0);
    }

    #[test]
    fn history_is_not_asked_until_it_is_on_and_there_is_a_station() {
        let ctx = egui::Context::default();
        let mut c = OnlineClient::default();
        c.ask_history(&ctx, Some("wss://shack.example/ws"), 0);
        assert!(c.rx.try_recv().is_err(), "nothing on, nothing sent");
        assert!(!c.history.line_busy(0));
        c.history.set_on(Some(9));
        c.ask_history(&ctx, None, 0);
        assert!(!c.history.line_busy(0), "no station, nothing sent");
        // The control: on, with a station, a request is out.
        c.ask_history(&ctx, Some("wss://shack.example/ws"), 0);
        assert!(c.history.line_busy(0), "a request went");
    }

    #[test]
    fn a_connection_that_is_not_an_address_is_told_no_station_without_a_request() {
        let ctx = egui::Context::default();
        let mut c = OnlineClient::default();
        c.history.set_on(Some(9));
        c.ask_history(&ctx, Some("not an address"), 0);
        assert!(c.rx.try_recv().is_err());
        assert_eq!(
            c.history.state(9),
            &aircraft_history::HistoryState::Unavailable(aircraft_history::Reason::NoStation)
        );
    }

    #[test]
    fn the_stations_table_reaches_the_export_store() {
        use super::super::aircraft_export::Ready;
        use sdroxide_aircraft_history::export::Format;
        let ctx = egui::Context::default();
        let mut c = OnlineClient::default();
        c.export.start(Format::Csv);
        assert_eq!(c.export.next_fetch(true, 0), Some(Format::Csv));
        let csv = "icao,callsign,registration\n";
        c.tx.send(Msg::Export { status: Some(200), body: csv.into() }).unwrap();
        c.poll(&ctx, 1);
        assert_eq!(c.export.take_ready(), Some(Ready::Station(Format::Csv, csv.into())));
        assert_eq!(c.store.len(), 0, "the relay's store was not touched");
    }

    #[test]
    fn the_stations_list_reaches_the_watch_store_and_frees_the_line() {
        let ctx = egui::Context::default();
        let mut c = OnlineClient::default();
        assert!(c.watch.next_request(true, 0).is_some());
        let body = serde_json::to_string(&sdroxide_aircraft_history::watch::WatchReply::of(
            &sdroxide_aircraft_history::watch::Watchlist::default(),
            None,
        ))
        .unwrap();
        c.tx.send(Msg::Watch { status: Some(200), body }).unwrap();
        c.poll(&ctx, 1);
        assert_eq!(*c.watch.state(), aircraft_watch::WatchState::Ready);
        assert!(!c.watch.line_busy(1));
        assert_eq!(c.store.len(), 0, "the relay's store was not touched");
    }

    #[test]
    fn an_export_is_not_asked_without_a_station_and_the_file_is_made_here() {
        use super::super::aircraft_export::Ready;
        use sdroxide_aircraft_history::export::Format;
        let ctx = egui::Context::default();
        let mut c = OnlineClient::default();
        c.ask_export(&ctx, Some("wss://shack.example/ws"), 0);
        assert!(c.rx.try_recv().is_err() && !c.export.busy(), "nothing pressed, nothing sent");
        c.export.start(Format::Json);
        c.ask_export(&ctx, None, 0);
        assert_eq!(
            c.export.take_ready(),
            Some(Ready::Local(Format::Json, aircraft_history::Reason::NoStation))
        );
        // The control: with a station a request goes and the store waits for it.
        c.export.start(Format::Json);
        c.ask_export(&ctx, Some("wss://shack.example/ws"), 0);
        assert!(c.export.busy() && c.export.take_ready().is_none());
    }

    #[test]
    fn a_connection_that_is_not_an_address_makes_the_file_here() {
        use super::super::aircraft_export::Ready;
        use sdroxide_aircraft_history::export::Format;
        let ctx = egui::Context::default();
        let mut c = OnlineClient::default();
        c.export.start(Format::Csv);
        c.ask_export(&ctx, Some("not an address"), 0);
        assert!(c.rx.try_recv().is_err());
        assert_eq!(
            c.export.take_ready(),
            Some(Ready::Local(Format::Csv, aircraft_history::Reason::NoStation))
        );
    }

    fn the_watch_list_is_not_asked_without_a_station_and_an_edit_waits_for_one() {
        let ctx = egui::Context::default();
        let mut c = OnlineClient::default();
        c.ask_watch(&ctx, None, 0);
        assert!(!c.watch.line_busy(0), "no station, nothing sent");
        assert!(c.watch.request_edit(sdroxide_aircraft_history::watch::WatchEdit {
            add: Some("KLM1234".into()),
            remove: None
        }));
        c.ask_watch(&ctx, None, 0);
        assert_eq!(c.watch.queued(), 1, "still waiting");
        c.ask_watch(&ctx, Some("wss://shack.example/ws"), 0);
        assert!(c.watch.line_busy(0), "the control: with a station, the edit went");
        assert_eq!(c.watch.queued(), 0);
    }

    #[test]
    fn a_connection_that_is_not_an_address_is_told_no_station_for_the_watch_list() {
        let ctx = egui::Context::default();
        let mut c = OnlineClient::default();
        c.ask_watch(&ctx, Some("not an address"), 0);
        assert!(c.rx.try_recv().is_err());
        assert_eq!(
            *c.watch.state(),
            aircraft_watch::WatchState::Unavailable(aircraft_history::Reason::NoStation)
        );
    }

    #[test]
    fn a_list_from_the_station_reaches_the_stats_store_and_frees_the_line() {
        let ctx = egui::Context::default();
        let mut c = OnlineClient::default();
        c.stats.set_on(true);
        assert!(c.stats.next_request(true, 0));
        let body = serde_json::to_string(&sdroxide_aircraft_history::ListReply::bare(
            sdroxide_aircraft_history::Status::Disabled,
        ))
        .unwrap();
        c.tx.send(Msg::Stats { status: Some(503), body }).unwrap();
        c.poll(&ctx, 1);
        assert_eq!(
            c.stats.state(),
            &aircraft_stats::StatsState::Unavailable(aircraft_history::Reason::Disabled)
        );
        assert!(!c.stats.line_busy(1));
        assert_eq!(c.store.len(), 0, "the relay's store was not touched");
        assert_eq!(c.history.on(), None, "nor the history's");
    }

    #[test]
    fn the_list_is_not_asked_until_the_view_is_open_and_there_is_a_station() {
        let ctx = egui::Context::default();
        let mut c = OnlineClient::default();
        c.ask_stats(&ctx, Some("wss://shack.example/ws"), 0);
        assert!(!c.stats.line_busy(0), "closed, nothing sent");
        c.stats.set_on(true);
        c.ask_stats(&ctx, None, 0);
        assert!(!c.stats.line_busy(0), "no station, nothing sent");
        c.ask_stats(&ctx, Some("wss://shack.example/ws"), 0);
        assert!(c.stats.line_busy(0), "the control: open, with a station, a request went");
    }

    #[test]
    fn a_connection_that_is_not_an_address_is_told_no_station_for_the_list() {
        let ctx = egui::Context::default();
        let mut c = OnlineClient::default();
        c.stats.set_on(true);
        c.ask_stats(&ctx, Some("not an address"), 0);
        assert!(c.rx.try_recv().is_err());
        assert_eq!(
            c.stats.state(),
            &aircraft_stats::StatsState::Unavailable(aircraft_history::Reason::NoStation)
        );
    }
}
