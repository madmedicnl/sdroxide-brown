//! The optional online half of an aircraft card: what adsbdb knows about the
//! airframe and the flight, and Planespotters' photo of it.
//!
//! Everything but [`fetch`] is a decision, never a side effect — which requests
//! a card needs, whether an answer is still good, what a reply means — so it
//! is tested without a network and compiles for wasm32, where the client reads
//! [`Info`] and nothing else. The rules it keeps:
//!
//! * no key, no account;
//! * only fixed hosts are ever asked, and only for an ICAO address, a callsign
//!   or a registration that has been cleaned first;
//! * answers are kept as JSON for at most 24 h (Planespotters' own limit),
//!   a failure for five minutes, and a photo is never fetched or kept — only
//!   the address Planespotters gave for it, unchanged, and the credit.
//!
//! The [`Relay`] is what a station runs so that its clients do not each reach
//! the internet themselves: one question out per key however many clients ask,
//! a ceiling on questions per minute, and one cache for all of them.

mod cache;
mod model;
mod parse;
mod relay;

#[cfg(feature = "fetch")]
pub mod fetch;

pub use cache::{BACKOFF_S, MAX_ENTRIES, OnlineCache, TTL_S};
pub use model::{
    Adsbdb, AircraftInfo, Airport, Lookup, Photo, Reply, Request, Route, clean_callsign,
    clean_registration, user_agent,
};
pub use parse::{parse_adsbdb, parse_photo, reply_for};
pub use relay::{FORMAT_VERSION, Info, Relay, Status, Upstream};
