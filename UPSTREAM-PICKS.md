# Upstream picks — the journal

The operator's instruction, 2026-10-23: **pick up four things from upstream
and work autonomously, keeping track here.** The four, in the order he gave
them:

1. **Wavelog** — the self-hosted web logbook, as an upload and download target.
2. **The mic stutter** — upstream's `6fd0fea6`, "TX Audio chain solve stuttering
   mic when live testing".
3. **The ELAD controls** — the FDM-DUO's own power steps, forward/reflected
   power and SWR, and a Drive scale that matches the radio.
4. **SSTV FTP upload** — a received picture posted to the operator's own FTP
   server.

**Only if it does not disturb F4NRM's work.** His files are DAB, the graphic
modes, APRS, the waterfall scrollbar, `swap_iq`, the per-memory colour and
WEFAX; nothing here may refactor or "improve" any of them on the way past.

## The rules this work follows

- ⛔ **Never merge `upstream/main`.** Everything here is read from a commit and
  written into our tree by hand. See the guard in `AGENTS.md`.
- Each item lands as **its own commit**, with the gate green: `cargo check
  --workspace --all-targets` silent, the touched crates' tests passing, and
  `rustfmt --check` clean. A fix carries a test verified to fail without it.
- A wire change is **appended last** and renumbered into our own register, with
  `VERSION_BYTE` stepped with it. Ours is at **v234** (`Action::BroadcastLabels`)
  as this starts.
- An item that cannot be taken is recorded here with the reason, not left
  half-done in the tree.

## State at a glance

| item | state | our commits |
| --- | --- | --- |
| Wavelog | **done** (see below) | `wavelog: the operator's own logbook, picked up from upstream #666` |
| Mic stutter | to assess | — |
| ELAD controls | pending | — |
| SSTV FTP upload | pending | — |

## Wavelog

Upstream: `06f14174` (merge of #666 by joscandreu) and `c40987c9` (stop the
download on an empty page, manual).

What it is: a **self-hosted** logbook, so the address, the API key and the
certificate are the operator's own. A QSO is posted to one of the *station
locations* the server defines, named by number, and contacts can be pulled back
so "worked before" and "new one" are right even for contacts made elsewhere.

### What it involves in our tree

- `sdroxide-types/src/wavelog.rs` (new, ~550 lines): `WavelogConfig`,
  `WavelogStation`, `WavelogProbe`, `WavelogBatch`, and the ADIF/JSON shapes.
- `sdroxide-net/src/wavelog.rs` (new, ~1200 lines) and `tls_pin.rs` (~250): the
  API client and the **certificate pin** for a server with a self-signed
  certificate — a pin, not "accept anything".
- `sdroxide-net/Cargo.toml`: `rustls` (ring only, as ureq uses) + `ring`.
- `NetworkConfig` gains `wavelog: WavelogConfig`; `DigiConfig` gains
  `wavelog_station_id`; `UploadTarget` gains `Wavelog`. All three ride the
  wire, so one **PROTO_VERSION** bump, appended last.
- Config persistence, the logbook's upload path, the engine's hook, and a
  Settings page.

### State — landed 2026-10-23

Taken: `sdroxide-types/src/wavelog.rs` and `sdroxide-net/src/{wavelog,tls_pin}.rs`
verbatim, plus the wiring:

- `NetworkConfig::wavelog`, `DigiConfig::wavelog_station_id`,
  `QsoRecord::{wavelog_sent, wavelog_origin}`, `UploadResult::retryable`,
  `UploadTarget::Wavelog`, `LoginTarget::Wavelog`, `Command::WavelogSync`,
  `RadioEvent::{WavelogProbe, WavelogBatch}` — all appended, **PROTO_VERSION
  234 → 235**, `VERSION_BYTE` 0x36 → 0x37**, with the register entry.
- `sdroxide-net`: the API client, the certificate pin (`rustls` + `ring`, the
  libraries ureq already builds on), the upload **queue** (a backlog is a
  thousand contacts, not a thousand threads), the download with cursors, and
  the live radio status the engine feeds every tick.
- `sdroxide-net/Cargo.toml`: `rustls` (ring only) + `ring`.
- The settings page is a **compact** section of my own rather than upstream's
  461 lines: address, key, station picker filled from the probe, a certificate
  row that pins what the server presented, Test, a download tick with its
  interval and locations, and Download now with a status line.
- `wavelog_sync.json` beside `qso_log.json` holds the cursors — they describe
  what the log **holds**, so they must not outlive it.
- Downloaded pages are merged as they arrive (`merge_from_wavelog`), so an
  interrupted first sync resumes rather than restarts.

**Not taken from upstream, deliberately:**
- The **retry tick** (a QSO Wavelog refused is retried every few minutes). The
  upload itself is queued and never lost, and a refusal is reported in the
  logbook's QSO status; the automatic retry is a nicety that wants the "refused"
  set persisted to be worth having. Journal it as the next step if anyone asks.
- The **full-resync review** (download everything, diff, let the operator
  approve). The incremental download covers the ordinary case; a review flow is
  a screen of its own.
- The **automatic** download timer (`sync_interval_min` is stored and shown but
  nothing fires on it yet — "Download now" does the work).
- `tests/wavelog_live.rs` — it needs a real Wavelog to write test QSOs to, and
  it calls free functions upstream re-exports that are private here.

**One fault found on the way, and it was ours, not upstream's:**
`adif_to_qso_log` read only `MODE` and ignored `SUBMODE`, so every imported
FT4/JS5/Q65 contact came in as "MFSK" — which matches nothing: not the decode
rows, not worked-before, not the mode chip. Fixed and pinned by
`an_adif_submode_is_the_mode_the_contact_was_made_in`. It is the reason
upstream's own download test failed here first.

Gate: `cargo check --workspace --all-targets` silent, 958 ui + 127 net + 789
types tests green, `rustfmt --check` clean on the files that were clean.
