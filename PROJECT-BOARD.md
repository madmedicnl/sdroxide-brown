# SDR Oxide Brown — project board content

Three columns. Achieved → **Done**, Open → **In Progress**, Roadmap → **Todo**.
Scripted via `gh project item-create` + `item-edit` once the `project` scope is
granted (`gh auth refresh -h github.com -s project`).

## Achieved (Done)

- **FT8/FT4/FT2 session ignore list** — a `−` per decode row mutes a station for
  the session, at decode ingress
- Release **v1.9.6_brown** (2026-09-29) and the pre-release **v1.9.6_brown.experimental**
- Release **v1.9.5_brown**, **v1.9.4_brown** (Brown rename), nightly builds
- **ALE** receiver core + front end + TX primitive (experimental); upstream draft PR #598
- **JTTY** receive and transmit (experimental, bench-confirmed)
- **Wide CB callsign grammar** (experimental, bench-confirmed)
- **FT8 checkpointed signal subtraction** (weak signals under strong neighbours)
- **digi-mode editable macros** (upstream PR #596, ported to the fork)
- **FSK441** decoder (upstream #555) + transmit (PR #561) and the empty-box/reply fixes
- **Panadapter "levels are hiding the picture" hint** + FIT
- **Auto-record silence split** review fixes (hidden-tab gate, own-stop, Stop-after)
- **Band openings** warm-up/dedupe fix
- **SWL report pre-fills** from the schedule on + NEW
- **Spoken alerts**: phrase preview + SAY
- **Morse trainer**, **CW key** (Settings → CW, USB paddle), **station profiles**
- **Tabbed band/mode menu** (LISTEN/OPERATE, ALL, dock), **SIG ID** guide
- **SWL**: reception log, QSL/report tracking, CSV/ADIF export, filters, schedule,
  local solar time, ECSS, receive tone, replay, scheduled recordings, band scanning
- **Grey line** on the flat maps, **meteor calendar**, **IBP beacons**, **Kp history**
- Decoders landed: **ACARS, HFDL, DSC, UVPacket, NAVTEX, HD Radio, AIS zoom, QO-100**
- **Nine upstream PRs merged** into upstream on 2026-09-28 (#583, #588, #590, #591, #593, …)
- Upstream fixes taken: WEFAX auto start/stop, RADE RX reporting, HD-on-AM, Icom WFM,
  PureSignal gate, TX drive ceiling, LimeSDR Mini, HFDL lane rate

## Open (In Progress)

- **Support queue, 2026-10-04.** What the fork's testers reported and where it
  stands. Issues are disabled on the fork; reports arrive as Discussions.
  - **Discussion #5 — Olivia decode (kevin2008-01): RESOLVED.** Find the fault
    and fix in MORNING.md §1. In 1.9.16 the decoder decodes real Olivia; a
    known-text sample decoded by the reporter (Wikipedia's 32/1000) reproduces
    here and is pinned as an `#[ignore]`d test
    (`SDROXIDE_OLIVIA_1000_32`). **Still open, and not a code matter:** nobody
    has copied *our own* Olivia over on fldigi/MultiPSK — the transmit half is
    unverified on air. Decode is confirmed; transmit awaiting a listener.
  - **Discussion #5 — three store/UI bugs (kevin2008-01): OPEN, unverified.**
    All separate from Olivia and not yet fixed:
    - **CLEAR RX appears to do nothing on Olivia.** Wiring is correct
      (`clear_rx_chip` → `DigiClearRx` → controller `clear_rx` → `text_rx`);
      most likely it was a symptom of the 1.9.15 decoder producing no text.
      Re-test on 1.9.16+; still broken with text on screen ⇒ real, check the
      web-session path.
    - **Frequency memories capped at 3 in a new web session** (10 saved → 3
      survive). Store/sync in the server session path.
    - **A decimal frequency is rounded** (7.038.5 MHz → 7.037). `MemoryChannel
      .freq_hz` is `f64`, so the save or display is rounding it.
  - **SSTV receive (operator, this bench): FIXED in 1.9.17.** The low end
    (200–294 Hz) was cancelled to a dead band by the auto-notch, which was
    running on the image/tone modes; `auto_notch_applies` now excludes SSTV,
    SSTV-FM, RIFP, WEFAX, Hell, RF Paint. Separately, "hears a clear signal but
    does not decode" was an **overloaded RSP1** (a keyed CRT inches away) —
    RF-side, not code; the panels now warn when the front end is overloading.
  - **NAVTEX "jibberish" (Cal, via the operator): support advice sent.** NAVTEX
    can only decode when the dial is set to **channel − 1.7 kHz, USB** (518 →
    516.300) and a station is in its slot. The panel hovers and manual now say
    both; no code fault found. Awaiting Cal's result.
  - **Discussion #8 — the phone is the control, and it settles what the fault
    is NOT.** A phone held to the speaker decodes everything, and a phone is
    clock-independent: it just decodes the sound it hears on its own clock. So
    the RF and the audio reaching the operator are good; do not re-litigate RF,
    antennas or audio levels.
  - **Discussion #8 — BENCH RESULT (2026-10-04, decisive): the server engine
    decodes fine, with and without a client.** Instrumented `poll_digi`
    (`DIGIDECODES` log), ran the 1.9.17 build as `--server` FT8 on 14.074:
    - **no client attached:** 25 decode batches (6/13/3/16/3…stations), no
      panic.
    - **native client attached** (`--connect 127.0.0.1`): 26 batches,
      **280 stations**.
    So the **fork's server-side decode path is not the fault** — not the
    engine, not the tap, not the relay to a native client. Kevin's failure is
    **specific to the browser client** (WASM page load / WebAudio / the WASM
    relay) or to his environment — which matches his own clue (minimise →
    sound cuts → reload/lag). **Next and only candidate: does the bug appear
    in the browser on *upstream* sdroxide too** (question posted to #8). If
    upstream's browser is fine and ours is not, it is a fork browser-client
    regression; if upstream is also bad, it is shared upstream + his setup.
  - **Discussion #8 — FT8/FTx decode, REOPENED by kevin2008-01 (2026-10-04
    16:41).** "FTx is not fixed in 1.9.16." New detail: with a brand-new web
    session it still decodes almost nothing; his phone on the speaker decodes
    everything. **New hypothesis (his, and plausible):** the web client losing
    focus / a network blip desynchronises the *server's* clock or the audio
    stream, and FTx is clock-critical. Two concrete claims to chase:
    (a) minimising the browser to the taskbar cuts the sound, and restoring it
    reloads and leaves a waterfall/audio lag; (b) any power interruption
    "throws off the clock". **The server-side decode path was fixed (the
    subtract panic) but Kevin's case is still failing, so the panic was not the
    whole story — or not his story.** Next: ask him for the exact build
    hash in the `1.9.16` screenshot, and whether a **native `--connect` client
    to the same server** decodes (that separates "server engine" from "browser
    relay/timing").
  - **Discussion #7 — SSTV (kevin2008-01), open.** Three asks: (1) menu extra
    to **re-upload**, **send to** (email a QSL picture) and **save to** a chosen
    location on a received picture; (2) **CTR does not centre** in SSTV
    (1.9.10–15) — frequencies stay on the left; (3) received pictures and the
    **SAVE** button only appear after closing and reopening the session —
    images are not refreshed within a session. (3) is the same shape as the
    screen-settings sync area; treat as a real UI/state bug.
- **Upstream PR #626 (CW engine-side keyer) — NEXT SESSION (queued 2026-10-04).**
  Review received 2026-10-03; branch `upstream-pr/cw-key-engine`. Do it in this
  order: **(2) the disconnect-safety fix first**, then (3) disarm, then the
  rest, with the A-vs-B squeeze test. Work
  needed.** Maintainer: shape is right (keyer in the engine, contacts not
  edges, appended wire), but the **lifecycle** is wrong:
  - a paddle press from idle never starts an over (`set_cw_contacts` doesn't
    key TX the way `key_down` does — only works once `fill_tx_block` runs; the
    tests hide it);
  - **a held paddle is not released when the client disconnects** (session
    cleanup sends only `CwKey(false)`, and the keyer skips the hold cap — must
    fix before it keys a transmitter unattended);
  - the keyer is **never disarmed**: `abort_tx`, `set_straight(false)` and
    config changes leave it running; after one paddle use the straight key goes
    dead; on a CAT rig it is armed *before* the refusal;
  - **mode B does not send the trailing element** when a squeeze is released
    mid-element — add a squeeze-release test pinning A vs B;
  - taps are latched only in `poll`, so press+release in one batch is lost;
  - the code/out text buffers grow unbounded **on the audio thread**.
  These are the next work item on this PR (it is a draft, so no rush, but they
  are real).
- **ALE**: decode a real burst off-air; wire TX; fold the mode into PR #598
- Upstream PRs awaiting review: **#537** band openings, **#545** (tr)uSDX nG,
  **#554** UVPacket, **#557** rec silence, **#559** band-menu captions,
  **#561** FSK441 TX, **#568** Morse trainer, **#569** CW keyer,
  **#572** CW no-link fallback, **#573** CW key package, **#586** FT8 SIC,
  **#598** ALE
- **Rebase #568 and #561** onto current `upstream/main`
- ALE experimental release: publish/finish once the live decode works

## Session ignore list — design (coded, 2026-09-29)

**Why session-only:** a memory `HashSet` with no file, no wire, no schema is
cheaper to run and to build than a persisted list, and avoids permanent hiding.
Familiar `−` UX without the permanence. Keep the filter as
`session_ignored ∪ (future persisted)` so persistence can be added later.

**As built (fork-only, UI layer):**
- `SdroxideApp` gains `session_ignored: ignore::Ignored` (a `HashSet<String>`,
  init in `app/mod.rs` alongside `digi_status`). No `PROTO_VERSION` change.
- `app/ignore.rs` holds the whole rule: `key` (trim + upper), `is_ignored`,
  `toggle`, `retain_unignored`, and `toggle_ignore` / `clear_ignored` /
  `ignored_hover`. Six unit tests.
- Decode row: a `−` chip beside the queue `+`/`＋` chip (`panels/decodes.rs`),
  drawn for any decode naming a sender, so it works for listeners too. The
  press is staged and written after the row loop (which holds borrows).
- **Filter at ingress**, in the `RadioEvent::Ft8Decodes` arm in `frame.rs` —
  before the announcer, the audible alerts, the decode list and the propagation
  field, so one rule covers all of them. Empty batch → `continue`.
- **Mute everything** also asked in `auto_target`, because rows held *before*
  the press are still in `digi_decodes`; without it auto mode would answer the
  very station just muted. Spotting/upload untouched.
- **Undo/visibility**: an `N ignored · clear` chip in the decode header, drawn
  only when non-empty. Muted rows are dimmed (`ROW_BG × 0.45`, grey accent,
  badge `MUTED`) and age out, so a mis-click is reversible and nothing refills
  the space.
- Glyphs are `−` / `×` — both already load-bearing in `sdroxide-ui`; a
  pictographic "no entry" sign was the one glyph at risk of the font atlas not
  carrying it.


## Roadmap (Todo)

- **Performance pass** — a full optimize run over the per-frame paths, not
  started. Nothing measured is *known* to be slow; the point is to find out
  where the time actually goes rather than to guess. Where to look, and what is
  already ruled out, so the next session does not re-derive it:
  - **The decode-list row loop** (`app/panels/decodes.rs`, 200 rows/frame) is the
    prime suspect: per row it clones the sender for `who`, builds several
    `RichText`s, and runs two layout passes. This is the heaviest per-frame
    allocation site in the UI.
  - **The session ignore list is NOT a problem** — measured 2026-09-29 in
    release: `is_ignored` over all 200 rows costs **7.6 µs/frame (37 ns/call)**,
    0.05 % of a 60 fps frame, and 3.4 µs of that is the `to_ascii_uppercase`
    `String` the row already pays for `who`. **Do not "optimize" it**; a
    `HashSet<String>` was chosen over `&str`/`Cow` deliberately — the set is
    session-sized and the clarity is worth the allocation.
  - Other suspects, unmeasured: `log_index()` (rebuilds when the log length
    changes — check it is not rebuilding per frame), `auto_target`'s `live` Vec
    clone (only when the ignore set is non-empty), the waterfall/spectrum
    repaint, and the prop/map texture rebuilds.
  - Method: `cargo test --release` with a `#[test]` + `Instant` harness
    (`std::time::Instant`, no criterion in the tree) run under
    `--release` — a debug-build measurement of a path this hot is meaningless.
    Beware the repo's `rustfmt.toml`: a bench harness left in a source file
    must be removed or it will not compile.
- **DAB/DAB+** — blocked on `xoolive/desperado#52` (library split)
- **M17** (RX-first) and **ALE Selcall**
- **DSC** audio front end (protocol+framer done; detector needs a real burst)
- **Inmarsat** (wideband-lane decoder, upstream #187)
- Decoder candidates: **POCSAG/FLEX**, **ARDOP**, **FLARM/OGN**, **UAT 978**
- Listener: **D-RAP absorption map**, **azimuthal map**, solar-wind/Bz sparklines,
  solar-cycle chart, broadcast noise reduction
- **Phase 4 polish**: say why the dial is not the frequency you picked; UTC everywhere
- **CW reception reporting** to PSK Reporter
