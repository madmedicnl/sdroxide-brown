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

use super::aircraft_online::{
    Answer, OnlineStore, ThumbStatus, Want, classify, info_url, thumb_fetchable,
};

/// The most bytes read of a reply from the relay.
const MAX_INFO_BYTES: u64 = 64 * 1024;
/// The most bytes read of a thumbnail. They are a few tens of kilobytes.
const MAX_THUMB_BYTES: u64 = 2 * 1024 * 1024;
/// Widest or tallest a thumbnail may decode to.
const MAX_THUMB_SIDE: u32 = 2048;
/// Thumbnails kept at once: the open card's, and a few it was just on.
const KEEP_THUMBS: usize = 8;

enum Msg {
    Info { icao: u32, status: Option<u16>, body: String },
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
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    thumbs: Vec<Thumb>,
}

impl Default for OnlineClient {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self { store: OnlineStore::default(), tx, rx, thumbs: Vec::new() }
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
        send_info(url, station, w.icao, self.tx.clone(), ctx.clone());
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

/// `GET` with an optional cookie: the status and the body, or `None` for no
/// answer at all.
#[cfg(not(target_arch = "wasm32"))]
fn get_info(agent: &ureq::Agent, url: &str, cookie: Option<&str>) -> Option<(u16, String)> {
    let mut req = agent.get(url);
    if let Some(c) = cookie {
        req = req.header("Cookie", c);
    }
    let mut resp = req.call().ok()?;
    let status = resp.status().as_u16();
    let body =
        resp.body_mut().with_config().limit(MAX_INFO_BYTES).read_to_string().unwrap_or_default();
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
fn send_info(url: String, station: String, icao: u32, tx: Sender<Msg>, ctx: egui::Context) {
    std::thread::spawn(move || {
        let agent = agent();
        let cookie = COOKIES.lock().unwrap_or_else(|e| e.into_inner()).get(&station).cloned();
        let mut got = get_info(&agent, &url, cookie.as_deref());
        if got.as_ref().is_some_and(|(s, _)| *s == 401) {
            if let Some(c) = sign_in(&agent, &station) {
                COOKIES.lock().unwrap_or_else(|e| e.into_inner()).insert(station, c.clone());
                got = get_info(&agent, &url, Some(&c));
            }
        }
        let (status, body) = match got {
            Some((s, b)) => (Some(s), b),
            None => (None, String::new()),
        };
        let _ = tx.send(Msg::Info { icao, status, body });
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
async fn fetch(url: &str, info: bool) -> Option<web_sys::Response> {
    use wasm_bindgen::JsCast;
    let window = web_sys::window()?;
    let init = web_sys::RequestInit::new();
    init.set_method("GET");
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
fn send_info(url: String, _station: String, icao: u32, tx: Sender<Msg>, ctx: egui::Context) {
    wasm_bindgen_futures::spawn_local(async move {
        let (status, body) = match fetch(&url, true).await {
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
        let body = if body.len() as u64 > MAX_INFO_BYTES { String::new() } else { body };
        let _ = tx.send(Msg::Info { icao, status, body });
        ctx.request_repaint();
    });
}

#[cfg(target_arch = "wasm32")]
fn send_thumb(url: String, tx: Sender<Msg>, ctx: egui::Context) {
    wasm_bindgen_futures::spawn_local(async move {
        let bytes = async {
            let r = fetch(&url, false).await?;
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
}
