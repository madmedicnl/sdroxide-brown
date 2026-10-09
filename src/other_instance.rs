//! Another sdroxide already running on this machine, found at start-up.
//!
//! Two sdroxide processes that open the same radio both read it: each gets a
//! stream with holes in it, the audio breaks up, and a PlutoSDR can stop
//! answering altogether. Nothing on either screen said why — the classic case
//! is a `sdroxide --server` left running as a service while the desktop window
//! is opened on the same box. So the window looks, once, before it opens its
//! radios, and says so if it finds one.
//!
//! Linux only: the scan reads `/proc`, which also sees a server started by
//! another user (a system service runs as root), because every process's
//! `cmdline` is world-readable there. Elsewhere it finds nothing, which is the
//! old behaviour.

use std::path::Path;

/// What kind of sdroxide another process is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// `sdroxide --server`: owns its radios and serves them to browsers.
    Server,
    /// Another desktop window.
    Window,
}

/// One other sdroxide process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instance {
    pub pid: u32,
    pub kind: Kind,
    /// The command line as it would be typed, for the operator to recognise.
    pub command: String,
}

/// Read one `/proc/<pid>/cmdline` (NUL-separated arguments). `None` when the
/// process is not sdroxide, or is an sdroxide that opens no radio.
pub fn classify(pid: u32, cmdline: &[u8]) -> Option<Instance> {
    let args: Vec<String> = cmdline
        .split(|&b| b == 0)
        .filter(|a| !a.is_empty())
        .map(|a| String::from_utf8_lossy(a).into_owned())
        .collect();
    let program = args.first()?;
    // The program itself, wherever it was started from — not a script whose
    // path merely mentions it (`/bin/sh /usr/local/bin/sdroxide-local`, whose
    // first argument is the shell).
    let name = Path::new(program).file_name()?.to_str()?;
    if name != "sdroxide" && name != "sdroxide.exe" {
        return None;
    }
    let rest = &args[1..];
    let has = |flag: &str| rest.iter().any(|a| a == flag || a.starts_with(&format!("{flag}=")));
    // Remote clients drive somebody else's radio, and the one-shot tools open
    // none: neither can collide with us.
    if has("--connect") || has("--freedv-reporter-probe") {
        return None;
    }
    if rest.iter().any(|a| matches!(a.as_str(), "--version" | "-V" | "--help" | "-h")) {
        return None;
    }
    let kind = if has("--server") { Kind::Server } else { Kind::Window };
    Some(Instance { pid, kind, command: args.join(" ") })
}

/// Every other sdroxide under `proc_root` (normally `/proc`), skipping our own
/// process. Unreadable entries — a process that ended mid-scan, a `/proc`
/// mounted with `hidepid` — are skipped rather than reported.
pub fn scan(proc_root: &Path, own_pid: u32) -> Vec<Instance> {
    let Ok(entries) = std::fs::read_dir(proc_root) else { return Vec::new() };
    let mut found: Vec<Instance> = entries
        .flatten()
        .filter_map(|e| e.file_name().to_str()?.parse::<u32>().ok())
        .filter(|&pid| pid != own_pid)
        .filter_map(|pid| {
            let cmdline = std::fs::read(proc_root.join(pid.to_string()).join("cmdline")).ok()?;
            classify(pid, &cmdline)
        })
        .collect();
    found.sort_by_key(|i| i.pid);
    found
}

/// The warning to show, or `None` when this sdroxide is alone.
pub fn warning(others: &[Instance]) -> Option<String> {
    let first = others.first()?;
    let what = match first.kind {
        Kind::Server => "an sdroxide server",
        Kind::Window => "another sdroxide window",
    };
    let more = match others.len() {
        1 => String::new(),
        n => format!(" (and {} more)", n - 1),
    };
    let fix = match first.kind {
        Kind::Server => {
            "stop it first (for a service: sudo systemctl stop sdroxide), or connect to it \
             instead of opening the radio here"
        }
        Kind::Window => "close one of them",
    };
    Some(format!(
        "{what} is already running on this machine{more} — PID {}: {}. If it uses the same \
         radio as this window, both read it and the sound breaks up: {fix}.",
        first.pid, first.command
    ))
}

/// Look for other sdroxide processes on this machine and word the warning.
pub fn check() -> Option<String> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    warning(&scan(Path::new("/proc"), std::process::id()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(args: &[&str]) -> Vec<u8> {
        let mut out = Vec::new();
        for a in args {
            out.extend_from_slice(a.as_bytes());
            out.push(0);
        }
        out
    }

    #[test]
    fn a_service_server_is_recognised() {
        let i = classify(2488, &cmd(&["/usr/bin/sdroxide", "--server", "--port", "4950"]));
        assert_eq!(
            i,
            Some(Instance {
                pid: 2488,
                kind: Kind::Server,
                command: "/usr/bin/sdroxide --server --port 4950".into(),
            })
        );
    }

    #[test]
    fn a_second_window_is_recognised() {
        let i = classify(2321, &cmd(&["sdroxide"])).unwrap();
        assert_eq!(i.kind, Kind::Window);
    }

    #[test]
    fn launchers_clients_and_one_shot_tools_are_not_collisions() {
        // The shell running a launcher script is not sdroxide.
        assert_eq!(classify(1, &cmd(&["/bin/sh", "/usr/local/bin/sdroxide-local"])), None);
        // A remote client opens no radio of its own.
        assert_eq!(classify(2, &cmd(&["sdroxide", "--connect", "host:4950"])), None);
        assert_eq!(classify(3, &cmd(&["sdroxide", "--version"])), None);
        // Nor does a program that merely has sdroxide in its name.
        assert_eq!(classify(4, &cmd(&["/usr/bin/sdroxide-helper", "--server"])), None);
        assert_eq!(classify(5, &[]), None);
    }

    #[test]
    fn the_scan_skips_ourselves_and_unreadable_entries() {
        let root = std::env::temp_dir().join(format!("sdroxide-proc-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let write = |pid: &str, args: &[&str]| {
            let dir = root.join(pid);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("cmdline"), cmd(args)).unwrap();
        };
        write("2488", &["/usr/bin/sdroxide", "--server", "--port", "4950"]);
        write("100", &["sdroxide"]); // ourselves
        write("7", &["/bin/bash"]);
        std::fs::create_dir_all(root.join("self")).unwrap(); // not a pid
        std::fs::create_dir_all(root.join("55")).unwrap(); // no cmdline: ended mid-scan
        let found = scan(&root, 100);
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].pid, 2488);
        assert_eq!(found[0].kind, Kind::Server);
    }

    #[test]
    fn the_warning_names_the_process_and_the_way_out() {
        assert_eq!(warning(&[]), None);
        let server = Instance {
            pid: 2488,
            kind: Kind::Server,
            command: "/usr/bin/sdroxide --server".into(),
        };
        let w = warning(std::slice::from_ref(&server)).unwrap();
        assert!(w.contains("PID 2488"), "{w}");
        assert!(w.contains("sudo systemctl stop sdroxide"), "{w}");
        let window = Instance { pid: 2321, kind: Kind::Window, command: "sdroxide".into() };
        let w = warning(&[window, server]).unwrap();
        assert!(w.contains("another sdroxide window"), "{w}");
        assert!(w.contains("(and 1 more)"), "{w}");
        assert!(w.contains("close one of them"), "{w}");
    }
}
