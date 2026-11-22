//! What can go wrong before any samples arrive.

/// A Perseus backend failure. Every variant says what to do about it, because
/// each one arrives in the device list or the log where the operator reads it.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    LibMissing(String),

    /// `perseus_init` counted nothing. The library being present and the
    /// receiver not being plugged in are the same answer to a Rescan.
    #[error("no Perseus receiver found (is it plugged in, and is the USB device accessible?)")]
    NotFound,

    /// The configured ordinal is not there any more — unplugged, or another
    /// Perseus was plugged in ahead of it.
    #[error("Perseus receiver #{0} is not there")]
    Index(u32),

    /// The vendor library's own words, from `perseus_errorstr`.
    #[error("Perseus: {0}")]
    Driver(String),

    #[error("the Perseus receiver is in use by another program")]
    InUse,
}
