//! Finding receivers, and opening one.
//!
//! `perseus_init` is the library's own enumeration: it walks libusb and returns
//! how many receivers it found. It is called on every [`count`] rather than
//! once per process, because a receiver plugged in after the program started has
//! to appear on a Rescan — which is the only way this backend can be told about
//! one — and the call touches no hardware beyond the USB device list.
//!
//! Nothing here opens a receiver to list it. That has a price: the library only
//! reports a serial number for an *open* descriptor, and opening one downloads
//! firmware (several seconds, moving parts included), so a Rescan that opened
//! every unit to name it would be both slow and noisy. The list is therefore by
//! ordinal — `Perseus SDR #1` — and the configuration stores the ordinal. A
//! station with two receivers sees a stable order as long as neither is
//! unplugged; [`open`] reads the real serial when it opens one and the stream
//! label carries it.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use tracing::{debug, info, warn};

use crate::error::Error;
use crate::ffi::{Api, Descr, ProdId};

/// One receiver the library can see. The ordinal is what [`open`] takes, so it
/// is the only identity a device list can offer before anything is opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevInfo {
    pub index: u32,
}

impl DevInfo {
    pub fn label(&self) -> String {
        format!("Perseus SDR #{}", self.index + 1)
    }
}

struct State {
    api: Option<Arc<Api>>,
    complained: bool,
}

fn state() -> &'static Mutex<State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(State { api: None, complained: false }))
}

/// The loaded library, loading it on first use.
///
/// A machine without `libperseus-sdr` gets one log line and then quiet: the
/// device list is empty, which is the same answer as "none plugged in", and the
/// operator who tries to open the backend is told which library is missing.
pub fn api() -> Result<Arc<Api>, Error> {
    let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(api) = &st.api {
        return Ok(Arc::clone(api));
    }
    match Api::load() {
        Ok(api) => {
            info!("libperseus-sdr loaded");
            let api = Arc::new(api);
            st.api = Some(Arc::clone(&api));
            Ok(api)
        }
        Err(e) => {
            if !st.complained {
                st.complained = true;
                warn!("{e}");
            }
            Err(Error::LibMissing(e))
        }
    }
}

/// How many receivers are open in this process.
///
/// **`perseus_init()` is not safe while one is.** The library's init walks the
/// USB bus and rebuilds its own global device table, under the transfer thread
/// of every descriptor it already handed out: on real hardware that is
/// `perseus: input transfer out of sequence` and then a SIGFPE inside the
/// library, reached simply by opening Settings while the radio streams. So the
/// list is answered from the last walk while anything is open, and only walked
/// again when nothing is.
static OPEN: AtomicUsize = AtomicUsize::new(0);
/// What the last walk saw, for the answers given while one is open.
static LAST_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Whether the bus may be walked now. The whole of the crash above is this
/// question, so it is its own function and it has a test.
fn may_walk(open: usize) -> bool {
    open == 0
}

/// How many receivers the library can see right now.
pub fn count() -> Result<usize, Error> {
    let api = api()?;
    if !may_walk(OPEN.load(Ordering::Relaxed)) {
        debug!("device list answered from the last walk: a receiver is streaming");
        return Ok(LAST_COUNT.load(Ordering::Relaxed));
    }
    // SAFETY: a pure enumeration call, and nothing is open; the library owns
    // everything it touches.
    let n = unsafe { (api.init)() };
    if n < 0 {
        return Err(Error::Driver(crate::ffi::error_string(&api)));
    }
    LAST_COUNT.store(n as usize, Ordering::Relaxed);
    Ok(n as usize)
}

/// The receivers on this machine, in the library's own order.
pub fn try_list() -> Result<Vec<DevInfo>, Error> {
    Ok((0..count()? as u32).map(|index| DevInfo { index }).collect())
}

/// [`try_list`] with the error swallowed, for a Rescan button: no library and
/// no receiver are the same answer there.
pub fn list() -> Vec<DevInfo> {
    try_list().unwrap_or_default()
}

/// An opened receiver, with the library and the descriptor owned together so
/// neither can outlive the other.
pub struct Opened {
    pub api: Arc<Api>,
    pub descr: Descr,
    /// What the EEPROM says, when the library can read it; empty otherwise.
    pub serial: String,
}

impl Opened {
    /// Download firmware, which `perseus_open` alone does not do and which every
    /// tuning call needs.
    pub fn load_firmware(&self) -> Result<(), Error> {
        // SAFETY: the descriptor is open and owned by `self`.
        let rc = unsafe { (self.api.firmware_download)(self.descr) };
        if rc < 0 {
            return Err(Error::Driver(crate::ffi::error_string(&self.api)));
        }
        Ok(())
    }

    /// The serial number, if this library has the call and the unit has one.
    fn read_serial(&self) -> String {
        let Some(get) = self.api.get_product_id else {
            return String::new();
        };
        let mut id = ProdId::default();
        // SAFETY: `id` is a live, correctly laid out `eeprom_prodid`; the
        // library writes only into it.
        if unsafe { get(self.descr, &mut id) } < 0 {
            return String::new();
        }
        // Copy out of the packed struct before formatting: a reference to a
        // field of a packed struct is a misaligned reference, which is UB.
        let sn = id.sn;
        let sig = id.signature;
        if sn == 0 {
            return String::new();
        }
        format!(
            "{sn:05}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}",
            sig[5], sig[4], sig[3], sig[2], sig[1], sig[0]
        )
    }
}

impl Drop for Opened {
    fn drop(&mut self) {
        if self.descr.is_null() {
            return;
        }
        // SAFETY: the descriptor came from `perseus_open` and is closed once,
        // here. The library keeps no reference to it afterwards.
        unsafe { (self.api.close)(self.descr) };
        self.descr = std::ptr::null_mut();
        OPEN.fetch_sub(1, Ordering::Relaxed);
    }
}

/// Open receiver `index`, downloading its firmware.
/// The rule the crash taught: walk the bus only when nothing is streaming.
#[cfg(test)]
mod tests {
    #[test]
    fn the_bus_is_only_walked_when_nothing_is_open() {
        assert!(super::may_walk(0));
        assert!(!super::may_walk(1));
        assert!(!super::may_walk(2));
    }
}

pub fn open(index: u32) -> Result<Opened, Error> {
    let api = api()?;
    // Re-enumerate first — **unless a receiver is already open**, in which case
    // the walk would tear its stream down (see `may_walk`). A station with one
    // Perseus streaming and a second one being added then trusts the ordinal
    // against the last walk, which is the list the panel was drawn from anyway.
    let n = if may_walk(OPEN.load(Ordering::Relaxed)) {
        // SAFETY: a pure enumeration call with nothing open.
        let n = unsafe { (api.init)() };
        if n < 0 {
            return Err(Error::Driver(crate::ffi::error_string(&api)));
        }
        LAST_COUNT.store(n as usize, Ordering::Relaxed);
        n
    } else {
        debug!("opening by the last walk: a receiver is already streaming");
        LAST_COUNT.load(Ordering::Relaxed) as i32
    };
    if index >= n as u32 {
        return Err(Error::Index(index));
    }
    // SAFETY: an ordinal the library itself just reported.
    let descr = unsafe { (api.open)(index as i32) };
    if descr.is_null() {
        let msg = crate::ffi::error_string(&api);
        // The library says "LIBUSBERR" for a receiver another process holds;
        // that is the one failure worth its own sentence.
        if msg.to_ascii_lowercase().contains("busy") || msg.to_ascii_lowercase().contains("access")
        {
            return Err(Error::InUse);
        }
        return Err(Error::Driver(msg));
    }
    OPEN.fetch_add(1, Ordering::Relaxed);
    let opened = Opened { api, descr, serial: String::new() };
    opened.load_firmware()?;
    let mut opened = opened;
    opened.serial = opened.read_serial();
    debug!(index, serial = %opened.serial, "perseus receiver opened");
    Ok(opened)
}
