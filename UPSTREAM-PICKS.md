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
| Mic stutter | **not applicable** (see below) | — |
| ELAD controls | **done** (see below) | `ELAD FDM-DUO: its own power steps, and Drive within them` |
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

## Mic stutter

Upstream: `6fd0fea6`, "TX Audio chain solve stuttering mic when live testing".

**Not applicable: the defect lives in a feature this fork does not carry.** The
commit is not a standalone fix — it is the inside of upstream's *TX Audio chain
window* (`crates/sdroxide-ui/src/app/tx_audio.rs`, 607 lines in that commit
alone), which they added with `2cdd04a7`. The stutter was in that window's
**live monitor**: the microphone is previewed through the chain on the speakers,
and the microphone's chunks and the speaker's blocks run on two clocks, so
handing samples on as they arrive leaves every block a little short and the
shortfall plays as silence — a gap in nearly every block. Their cure is
`LivePacer`, which primes a block-and-a-chunk before it starts playing and
trims a backlog so the delay cannot creep up.

We have none of that: there is no `txvoice` chain module and no `tx_audio.rs`
here, and nothing in our TX path previews the microphone on the speakers, so
there is nothing for `LivePacer` to pace. Taking it would mean taking the whole
TX Audio chain — 1,000+ lines, three new wire items, a window — which is a
feature port rather than a fix.

**What would make it ours again:** a report of a stuttering microphone on
transmit (as opposed to a stuttering *monitor*). That would be a different
defect in `fill_tx_block`'s microphone path, and it should be reproduced before
anything is changed.

## ELAD controls

Upstream: `e5772095` (AJ3G, #717) and its follow-up `35ffeebc`.

**Taken: the FDM-DUO's own power controls.** `elad.rs` and `cat/lib.rs` applied
clean (our copy of that file had drifted only by its mode digits):

- Menu 34's `TP` register: nine watt steps plus MAX, now a *ceiling* the
  operator chooses rather than something a slider writes. `set_power` is a
  no-op for this family and `set_power_step` writes `TPnn;`.
- `power_by_audio` / `audio_power_ceiling`: Drive and Tune set the output by
  scaling the transmit audio within that ceiling. The scaling lives in
  `AudioCatSource`: `tx_audio_gain`, and `write_tx_audio_scaled` (the old
  `tx_write_audio` body, moved out unchanged) so the gain is applied once.
  `DUO_AUDIO_POWER_CEILING = 0.24` is AJ3G's measurement on his own DUO — 8 W
  by 24 % of full audio, flat above it — so the whole Drive slider lands on the
  linear part of the radio.
- **Reflected power** in `TxMeters`/`TxTelemetry` (`rfl_w`), read from `RP`.
- `CatConfig::elad_tx_power` (as `TP`'s parameter, default 9 = MAX, which is
  what was always sent before) rides `SetRadioConfig`; `Command::SetRigPowerStep`
  is the live half.
- The **buttons** are our own row beside Drive in the strip (`1W 3W 5W MAX`),
  lit from the saved config, and they save the setting as well as sending it —
  upstream's are laid out for their strip and ours is a different shape.

**Not taken: the HobbyPCB Hardrock amplifier panel**, which shares that commit.
It is a new crate (1,750 lines), a 2,159-line window, a safety-interlock
document and its own settings page — a project of its own, and not what "take
the ELAD controls" asks for. If it is ever wanted, `e5772095` is the source and
`crates/sdroxide-hardrock` is where it starts.

Wire: `CatConfig::elad_tx_power` and `TxMeters::rfl_w` both ride messages that
cross the link, and `Command::SetRigPowerStep` is appended — **PROTO_VERSION
235 → 236**, `VERSION_BYTE` 0x37 → 0x38**.

**Not tested on a DUO here** — no ELAD on this bench. The `TP` frames, the
audio gain arithmetic and the `RP` parsing are unit-tested (258 in the CAT
crate green); the on-air check is a real DUO: 1 W/3 W/5 W/MAX changing the
radio's own menu 34, Drive moving the output between them, and forward/
reflected power and SWR reading sensible while keyed.

## SSTV FTP upload

Upstream: `95f24a42` and its follow-up `2f11d02e`.

**Not started.** Scope measured: a new `sdroxide-picup` crate (FTP/FTPS client,
rotation, stamping, a worker — ~1,050 lines), `NetworkConfig` fields, a
`Command`, an engine hook after each received picture, and a settings row. It
does **not** touch any of F4NRM's work if it is added *after* the picture is
saved — the upload is a consumer of the finished file, not a change to the
decoder or the panel — which is the condition the operator set. Start here.
