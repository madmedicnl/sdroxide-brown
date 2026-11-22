//! Hand-written bindings for `libperseus-sdr`, the Microtelecom vendor library
//! (github.com/Microtelecom/libperseus-sdr, LGPL-2.1-or-later).
//!
//! The library is loaded with dlopen at runtime, exactly as `sdroxide-fobos`,
//! `sdroxide-sdrplay` and `sdroxide-lime` load theirs: nothing is linked at build
//! time, so this crate builds everywhere and merely fails to find the library
//! where it is not installed. See the crate's `Cargo.toml` for the licence
//! reasoning.
//!
//! # Nothing here crosses the boundary by value
//!
//! Every call takes an opaque `perseus_descr *` the library owns — never
//! dereferenced on this side — plus primitives, and one output struct
//! ([`ProdId`]) whose layout is pinned by `repr(C)` against the header's own
//! `__attribute__((packed))`. There is no ABI drift to misalign beyond that one
//! struct, and it is only used to name a receiver.
//!
//! # A missing symbol is not always a missing library
//!
//! The two serial-number calls are bound as `Option`: a library built without
//! them streams just as well, and refusing to load it would report "no Perseus
//! found" for a receiver that is plugged in and working. Everything this crate
//! actually streams with stays required, so a genuinely wrong library still
//! fails at load rather than mid-session.

#![allow(dead_code)]

use std::ffi::{CStr, c_char, c_int, c_uint, c_void};

/// `perseus_descr *` — an opaque descriptor owned by the library.
pub type Descr = *mut c_void;

/// `perseus_input_callback`: the library's own thread calls this with a buffer
/// of **6 bytes per I/Q pair** (see [`crate::handle`] for the conversion) plus
/// whatever `extra` was handed to `perseus_start_async_input`. Returning a
/// negative value stops the stream.
pub type InputCb =
    unsafe extern "C" fn(buf: *mut c_void, buf_size: c_int, extra: *mut c_void) -> c_int;

/// `eeprom_prodid` — the packed product id the library reads out of the
/// receiver's EEPROM. `repr(C, packed)` is the header's own layout: `u16 sn`,
/// `u16 prodcode`, `u8 hwrel`, `u8 hwver`, six signature bytes.
#[repr(C, packed)]
#[derive(Clone, Copy, Default)]
pub struct ProdId {
    pub sn: u16,
    pub prodcode: u16,
    pub hwrel: u8,
    pub hwver: u8,
    pub signature: [u8; 6],
}

/// `PERSEUS_ATT_*`: the attenuator is a four-position switch, and the ordinal
/// is what the library takes. Ten dB per step.
pub const ATT_STEPS_DB: [i32; 4] = [0, 10, 20, 30];

/// `PERSEUS_ASYNCSTARTED` — "a stream is already running". The one code worth
/// naming: it is what a second start on the same descriptor answers, and it
/// says the descriptor is still ours rather than broken.
pub const ERR_ASYNCSTARTED: c_int = -19;

/// The library's own `perseus_errorstr`, once a call has failed.
pub fn error_string(api: &Api) -> String {
    // SAFETY: the library returns a pointer to a static, NUL-terminated buffer.
    unsafe {
        let p = (api.errorstr)();
        if p.is_null() {
            return "unknown perseus error".into();
        }
        CStr::from_ptr(p).to_string_lossy().trim().to_string()
    }
}

/// Every symbol this crate uses, resolved once per process.
pub struct Api {
    _lib: libloading::Library,

    pub init: unsafe extern "C" fn() -> c_int,
    pub exit: unsafe extern "C" fn() -> c_int,
    pub open: unsafe extern "C" fn(n_dev: c_int) -> Descr,
    pub close: unsafe extern "C" fn(descr: Descr) -> c_int,
    pub firmware_download: unsafe extern "C" fn(descr: Descr) -> c_int,
    pub set_sampling_rate: unsafe extern "C" fn(descr: Descr, rate: c_int) -> c_int,
    pub get_sampling_rates:
        unsafe extern "C" fn(descr: Descr, buf: *mut c_int, size: c_uint) -> c_int,
    pub set_ddc_center_freq:
        unsafe extern "C" fn(descr: Descr, hz: f64, enable_presel: c_int) -> c_int,
    pub set_attenuator: unsafe extern "C" fn(descr: Descr, att: c_int) -> c_int,
    pub start_async_input:
        unsafe extern "C" fn(descr: Descr, buf_size: u32, cb: InputCb, extra: *mut c_void) -> c_int,
    pub stop_async_input: unsafe extern "C" fn(descr: Descr) -> c_int,
    pub errorstr: unsafe extern "C" fn() -> *mut c_char,

    /// Newer than the first header this crate targeted, and not needed to
    /// stream: see the module note.
    pub get_product_id: Option<unsafe extern "C" fn(descr: Descr, out: *mut ProdId) -> c_int>,
    pub is_preserie: Option<unsafe extern "C" fn(descr: Descr, flag: *mut c_int) -> c_int>,
}

impl Api {
    /// Find and bind the library, or say why not.
    ///
    /// The message is what the operator sees in place of a device list, so it
    /// names the library rather than the crate.
    pub fn load() -> Result<Api, String> {
        let mut tried = Vec::new();
        for name in lib_candidates() {
            // SAFETY: dlopen of a name we chose; the handle is kept alive by
            // `Api` for as long as any symbol from it is callable.
            match unsafe { libloading::Library::new(&name) } {
                Ok(lib) => return unsafe { Api::from_lib(lib) },
                Err(e) => tried.push(format!("{name}: {e}")),
            }
        }
        Err(format!(
            "libperseus-sdr not found (tried {}). Install the Perseus library \
             (github.com/Microtelecom/libperseus-sdr) to use this receiver.",
            tried.join(", ")
        ))
    }

    unsafe fn from_lib(lib: libloading::Library) -> Result<Api, String> {
        /// A call this crate cannot work without.
        macro_rules! sym {
            ($name:literal) => {
                *unsafe { lib.get(concat!($name, "\0").as_bytes()) }
                    .map_err(|e| format!("{} missing from libperseus-sdr: {e}", $name))?
            };
        }
        /// A call the library may predate; not used to stream.
        macro_rules! opt {
            ($name:literal) => {
                unsafe { lib.get(concat!($name, "\0").as_bytes()) }.ok().map(|s| *s)
            };
        }
        Ok(Api {
            init: sym!("perseus_init"),
            exit: sym!("perseus_exit"),
            open: sym!("perseus_open"),
            close: sym!("perseus_close"),
            firmware_download: sym!("perseus_firmware_download"),
            set_sampling_rate: sym!("perseus_set_sampling_rate"),
            get_sampling_rates: sym!("perseus_get_sampling_rates"),
            set_ddc_center_freq: sym!("perseus_set_ddc_center_freq"),
            set_attenuator: sym!("perseus_set_attenuator"),
            start_async_input: sym!("perseus_start_async_input"),
            stop_async_input: sym!("perseus_stop_async_input"),
            errorstr: sym!("perseus_errorstr"),
            get_product_id: opt!("perseus_get_product_id"),
            is_preserie: opt!("perseus_is_preserie"),
            _lib: lib,
        })
    }
}

/// Where the library is called on each platform, most specific first.
fn lib_candidates() -> Vec<String> {
    #[cfg(target_os = "linux")]
    {
        vec!["libperseus-sdr.so.0".into(), "libperseus-sdr.so".into()]
    }
    #[cfg(target_os = "macos")]
    {
        vec![
            "/usr/local/lib/libperseus-sdr.dylib".into(),
            "/opt/homebrew/lib/libperseus-sdr.dylib".into(),
            "libperseus-sdr.dylib".into(),
        ]
    }
    #[cfg(target_os = "windows")]
    {
        let mut v = vec!["libperseus-sdr.dll".into(), "perseus-sdr.dll".into()];
        if let Ok(pf) = std::env::var("ProgramFiles") {
            v.push(format!("{pf}\\Microtelecom\\Perseus\\libperseus-sdr.dll"));
        }
        v
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        vec!["libperseus-sdr.so".into()]
    }
}
