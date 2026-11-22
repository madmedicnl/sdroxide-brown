//! Microtelecom Perseus SDR, through the vendor's `libperseus-sdr`.
//!
//! Receive only: the hardware has no transmitter, and the `IqSource` trait's
//! transmit methods already answer with an error, which is the right answer
//! here.
//!
//! Three layers, each with one job:
//!
//! - [`ffi`] — the dlopen bindings. Nothing is linked at build time, so this
//!   crate builds everywhere and finds nothing on a machine without the
//!   library.
//! - [`device`] — find receivers, open one, download its firmware, read its
//!   serial number when the library can.
//! - [`handle`] — tune it, attenuate it, and drain the ring the library's own
//!   callback fills. The callback's sample layout is 6 bytes per I/Q pair and
//!   is pinned by a test.
//!
//! Must never be a dependency of any wasm-targeted crate.

pub mod device;
pub mod error;
pub mod ffi;
pub mod handle;

pub use error::Error;
pub use handle::{OpenParams, PerseusHandle, att_index};
