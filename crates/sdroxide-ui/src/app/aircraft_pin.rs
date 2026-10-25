//! The card pinned to an aircraft on the map, and the one button in it that
//! asks.
//!
//! A click on an aircraft pins its card ([`crate::adsb_map`] holds which); the
//! card is the hover card's ([`super::aircraft_hover`]) with a link that can be
//! followed, a close button and, where it makes sense, **Look up online**. The
//! click and the card ask for nothing. Only that button does, once per press,
//! for the pinned aircraft alone, through the same line the AIRCRAFT window's
//! card uses — one question at a time, the station's own ceiling, the same
//! back-off — and the thumbnail is fetched only after that answer.

// ==== IMPL ====
use eframe::egui;
use sdroxide_types::AdsbAircraft;

use super::SdroxideApp;
use super::aircraft_card::CardKey;
use super::aircraft_hover::{self, LookupOffer, PinAction};
use super::aircraft_online::{self, OnlineState, ThumbStatus, Want};
use super::aircraft_registry::AircraftCard;
use crate::adsb_map;

/// The question the lookup button sends: the pinned aircraft's, and only when
/// the button was on offer.
pub(in crate::app) fn lookup_want(
    pinned: Option<u32>,
    want: Option<Want>,
    offer: LookupOffer,
) -> Option<Want> {
    let want = want?;
    (offer == LookupOffer::Button && pinned.is_some_and(|p| p & 0xff_ffff == want.icao))
        .then_some(want)
}

/// The thumbnail to fetch now: only after a lookup was pressed for the pinned
/// aircraft, and only once its answer carries a picture Planespotters hands out.
pub(in crate::app) fn thumb_wish(
    lookup: Option<u32>,
    pinned: Option<u32>,
    state: &OnlineState,
) -> Option<String> {
    if lookup.is_none() || lookup != pinned {
        return None;
    }
    aircraft_online::photo_view(state, ThumbStatus::NotTried)
        .filter(|p| p.show_image)
        .map(|p| p.thumb_url)
}

/// Whether a pressed lookup is still being followed: until its aircraft is no
/// longer the pinned one, or its question ended without a picture to fetch.
pub(in crate::app) fn lookup_after(
    lookup: Option<u32>,
    pinned: Option<u32>,
    state: &OnlineState,
) -> Option<u32> {
    let followed = lookup.filter(|_| lookup == pinned);
    followed.filter(|_| matches!(state, OnlineState::Loading | OnlineState::Ready(_)))
}

impl SdroxideApp {
    /// Draw the pinned card over the map, and do what was pressed in it.
    pub(in crate::app) fn adsb_pinned_card(
        &mut self,
        ui: &mut egui::Ui,
        aircraft: &[AdsbAircraft],
        home: Option<(f64, f64)>,
        now: i64,
    ) {
        let Some(pinned) = self.adsb_map.pinned else {
            self.adsb_map.lookup = None;
            return;
        };
        let Some(a) = aircraft.iter().find(|a| a.icao == pinned) else { return };
        let ctx = ui.ctx().clone();

        // Esc closes it, unless a text field has the keyboard.
        let typing = ctx.egui_wants_keyboard_input();
        let esc =
            !typing && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        if adsb_map::pin_after_key(Some(pinned), esc, typing).is_none() {
            self.adsb_map.pinned = adsb_map::pin_after_close(self.adsb_map.pinned);
            self.adsb_map.lookup = None;
            return;
        }
        let (Some(anchor), Some(bounds)) = (self.adsb_map.pin_anchor, self.adsb_map.pin_bounds)
        else {
            return;
        };

        // What the line has delivered since the last frame; this is also what
        // brings a lookup's answer in while the Cards view is not on screen.
        self.aircraft_online.poll(&ctx, now);
        let peer = self.ctrl.peer_url();
        let has_station = peer.is_some();
        let icao = a.icao & 0xff_ffff;
        let state = self.aircraft_online.store.state_for(icao, has_station).clone();
        self.adsb_map.lookup = lookup_after(self.adsb_map.lookup, Some(pinned), &state);
        if let Some(url) = thumb_wish(self.adsb_map.lookup, Some(pinned), &state) {
            self.aircraft_online.want_thumb(&ctx, &url);
        }
        let offer = aircraft_hover::lookup_offer(
            &state,
            has_station,
            self.aircraft_online.store.line_busy(now),
        );

        let size = self
            .adsb_map
            .pin_size
            .unwrap_or_else(|| egui::vec2(aircraft_hover::HOVER_W + 16.0, 220.0));
        let at = adsb_map::pin_placement(anchor, size, bounds);
        let (card, tex) = aircraft_hover::card_view(
            a,
            &self.aircraft_registry,
            &self.aircraft_online,
            has_station,
            home,
            now,
        );
        let shown = egui::Area::new(egui::Id::new("adsb-pinned-card"))
            .order(egui::Order::Middle)
            .fixed_pos(at)
            .constrain_to(bounds)
            .show(&ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .show(ui, |ui| aircraft_hover::draw_pinned(ui, &card, tex, offer))
                    .inner
            });
        self.adsb_map.pin_size = Some(shown.response.rect.size());

        for act in shown.inner.actions {
            match act {
                PinAction::Close => {
                    self.adsb_map.pinned = adsb_map::pin_after_close(self.adsb_map.pinned);
                    self.adsb_map.lookup = None;
                }
                PinAction::OpenLink(url) => {
                    if !crate::download::open_external(&url) {
                        tracing::warn!(%url, "no handler for this link");
                    }
                }
                PinAction::Lookup => {
                    let none = AircraftCard::default();
                    let card =
                        super::aircraft_card::lookup(&self.aircraft_registry, &CardKey::Icao(icao))
                            .unwrap_or(&none);
                    let want = aircraft_online::want_for(&CardKey::Icao(icao), card);
                    if let Some(w) = lookup_want(Some(pinned), want, offer) {
                        self.aircraft_online.ask(&ctx, peer.as_deref(), Some(&w), now);
                        self.adsb_map.lookup = Some(pinned);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::aircraft_online::{Answer, OnlineStore, Reason};
    use super::*;
    use sdroxide_aircraft_online::{Info, Photo, Status};

    const NOW: i64 = 1_000_000;

    fn want(icao: u32) -> Want {
        Want { icao, callsign: Some("KLM1234".into()), registration: None }
    }

    fn ready_with(thumb: &str) -> OnlineState {
        let mut i = Info::bare(Status::Ok);
        i.photo = Some(Photo {
            thumb_url: thumb.into(),
            page_url: "https://www.planespotters.net/photo/1/x".into(),
            photographer: "A. Spotter".into(),
        });
        OnlineState::Ready(Box::new(i))
    }

    const THUMB: &str = "https://t.plnspttrs.net/1/a_t.jpg";

    #[test]
    fn the_lookup_button_asks_only_the_pinned_aircraft() {
        // What the button may send.
        assert_eq!(lookup_want(Some(5), Some(want(5)), LookupOffer::Button), Some(want(5)));
        assert_eq!(lookup_want(Some(5), Some(want(6)), LookupOffer::Button), None);
        assert_eq!(lookup_want(None, Some(want(5)), LookupOffer::Button), None);
        assert_eq!(lookup_want(Some(5), None, LookupOffer::Button), None);
        for offer in [LookupOffer::None, LookupOffer::Asking, LookupOffer::Waiting] {
            assert_eq!(lookup_want(Some(5), Some(want(5)), offer), None, "{offer:?}");
        }

        // What the line does with it: once, one at a time, and no second try on
        // its own after the answer.
        let mut store = OnlineStore::default();
        let w = lookup_want(Some(5), Some(want(5)), LookupOffer::Button).unwrap();
        assert_eq!(store.next_request(Some(&w), true, NOW), Some(w.clone()));
        assert_eq!(store.next_request(Some(&w), true, NOW + 1), None, "pressed twice");
        assert_eq!(store.next_request(Some(&want(6)), true, NOW + 1), None, "another aircraft");
        store.deliver(5, Answer::Unavailable(Reason::Busy), NOW + 2);
        assert_eq!(store.next_request(Some(&w), true, NOW + 3), None, "back-off, not a retry");
    }

    #[test]
    fn the_thumbnail_is_wanted_only_after_a_lookup_for_the_pinned_aircraft() {
        let st = ready_with(THUMB);
        assert_eq!(thumb_wish(Some(5), Some(5), &st).as_deref(), Some(THUMB));
        assert_eq!(thumb_wish(None, Some(5), &st), None, "never looked up: not fetched");
        assert_eq!(thumb_wish(Some(6), Some(5), &st), None, "another aircraft's lookup");
        assert_eq!(thumb_wish(Some(5), None, &st), None);
        for st in
            [OnlineState::Idle, OnlineState::Loading, OnlineState::Unavailable(Reason::Offline)]
        {
            assert_eq!(thumb_wish(Some(5), Some(5), &st), None, "{st:?}");
        }
        let elsewhere = ready_with("https://evil.example/a.jpg");
        assert_eq!(thumb_wish(Some(5), Some(5), &elsewhere), None, "only the picture host");
    }

    #[test]
    fn a_lookup_ends_with_its_aircraft_or_its_failure() {
        let ready = ready_with(THUMB);
        assert_eq!(lookup_after(Some(5), Some(5), &OnlineState::Loading), Some(5));
        assert_eq!(lookup_after(Some(5), Some(5), &ready), Some(5), "the picture is still coming");
        assert_eq!(lookup_after(Some(5), Some(6), &ready), None, "the pin moved on");
        assert_eq!(lookup_after(Some(5), None, &ready), None, "the card was closed");
        for r in [Reason::Disabled, Reason::Offline, Reason::Busy, Reason::Unknown] {
            assert_eq!(lookup_after(Some(5), Some(5), &OnlineState::Unavailable(r)), None, "{r:?}");
        }
        assert_eq!(lookup_after(None, Some(5), &ready), None);
    }

    fn shipped(src: &str) -> &str {
        src.split("\n#[cfg(test)]").next().unwrap_or(src)
    }

    #[test]
    fn the_only_question_the_pinned_card_asks_is_the_lookup_buttons() {
        let code = shipped(include_str!("aircraft_pin.rs"));
        let ask = concat!(".as", "k(");
        let thumb = concat!("want_", "thumb(");
        assert_eq!(code.matches(ask).count(), 1, "one place may ask");
        assert_eq!(code.matches(thumb).count(), 1, "one place may fetch a picture");
        assert!(
            code.matches(concat!("next_", "request(")).count() == 0,
            "the line is the client's"
        );
    }

    #[test]
    fn the_pinned_card_keeps_nothing_in_the_browser_and_names_no_service() {
        let banned = [
            concat!("local", "Storage"),
            concat!("session", "Storage"),
            concat!("Indexed", "DB"),
            concat!("api.", "adsbdb"),
            concat!("api.", "planespotters"),
            concat!("api_", "key"),
            concat!("web_sys", "::"),
        ];
        for src in [include_str!("aircraft_pin.rs"), include_str!("aircraft_hover.rs")] {
            let code = shipped(src);
            for b in banned {
                assert!(!code.contains(b), "contains {b}");
            }
        }
    }
}
