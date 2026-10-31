# Agent notes — SDR Oxide Brown

Technical and process rules for this fork. Kept in the repository so every
agent session starts from the same baseline. Personal working notes live in
`docs/internal/`.

## Project identity

SDR Oxide Brown is a fork of `sdroxide` tuned for 11 m CB, shortwave listening
and decoding. All three live in one program. Build name: **SDR Oxide Brown**.
Config directory: `~/.config/sdroxide-brown` (`org.sdroxide.sdroxide-brown`).
`CARGO_PKG_VERSION` stays plain (`2.2.0`). Executable name, macOS bundle id
and release asset names stay shared with upstream. Origin: `madmedicnl/sdroxide-brown`.

## Fork rules

- Contributions to code and ideas are always greatly appreciated, but
  **madmedicnl has the final say and can overrule** any decision.

- Work happens here. When a choice is between a ham workflow and a CB/listening
  one, this fork takes the CB/listening one.
- Never merge `upstream/main`. Upstream is behind us. Reading upstream is safe
  and encouraged (`git log`, `git show`, `git diff`). Taking a specific commit
  is done by hand, one commit at a time.
- Changes derived from upstream issues may be offered upstream as their own
  branch off `upstream/main`, one idea per PR. Purely fork-shaped work stays here.
- Two divergences are deliberate: Olivia's polarity (positive) and spoken alerts
  (speak regardless of window focus). Do not "fix" either by taking upstream's side.

  
- CTR on = the frequency is always in the middle With the CTR chip lit the red line is at the centre of the waterfall on every frame — wheel, pinch, keyboard zoom, pan, fit, mode or decoder change, every radio, the whole bandwidth, zoom in or out, fast or slow. No drift, no jitter, no off/on to re-centre. Modes keep their preset bandwidth; CTR only moves the line. Touching waterfall or zoom code: keep the ctr_zoom_sim tests in widgets/spectrum_view.rs green and add the gesture.
- Never perform a zoom correction on all radios at the same time. Each radio is different. A correction should not affect the other radios (example: PLUTO = 1 radio = 1 Interface radio).

## F4NRM's areas (PR required, never straight to main)

F4NRM owns: decoders SSTV, DAB, NAVTEX, APRS, WFM, DSC, Olivia, RTTY, WEFAX,
FT8, RDS and backends HackRF, PlutoSDR. RSP1 is NOT on his list (bench receiver).
Confirmed on air by his radio group: WEFAX, DAB, SSTV, DSC, radio classic, aircraft.

Anything touching these — a fault fix, a wire change, a panel change — goes up as
a PR, never straight to `main`. No refactor or "improvement" without a report
against a specific fault.

Detachable windows are his too. When a change touches a panel, make it detachable
or stackable and add it to the Detachable page in Settings.

## House rules

- **Say it short.** Release notes: one line per change, max 60 lines. Replies
  are prose, not documents. No headings, no bulleted essays.
- **Ask a reporter one thing, not three.**
- **F4NRM's replies are AI-drafted** — read as suggestions, not commands.
  Answer short (3-4 sentences, one question max).
- **A measurement that disagrees with a report is the first suspect.** Check:
  visible? measures the right thing? existing number disagrees? something
  fighting the fix?
- **A passing test is evidence about the test.** Tolerances accommodating known
  faults are the fault.
- **A fix carries a test verified to fail on unfixed code.** Remove, watch fail,
  restore.
- **For anything drawn, render and look.** Numbers prove where ink is; only
  looking proves it's there.
- **Never claim fixed/shipped when it isn't.** "Needs feedback" is a request.
- **A control must never silently do nothing.** Name what's off, say why.
- **No paid service, no API key, no account. Ever.** Program works offline.
- **`git checkout --` destroys work.** Undo scratch patches with the patch.
- **`rg -n`, never `rg -rn`.**
- **Format what you touch.** No repo-wide `cargo fmt`. Fork-only files swept on
  arrival. Run rustfmt with `--config-path` or from repo root.
- **Commit messages:** short imperative subject, then the why.
- **CTR on = the frequency is always in the middle.** With the CTR chip lit the
  red line is at the centre of the waterfall on every frame: wheel, pinch,
  keyboard zoom, pan, fit, mode or decoder change; every radio, the whole
  bandwidth, zoom in or out, fast or slow. No drift, no jitter, no off/on to
  re-centre. Modes keep their preset bandwidth; CTR only moves the line. Touching
  waterfall or zoom code: keep the `ctr_zoom_sim` tests in
  `widgets/spectrum_view.rs` green and add the gesture you changed.

## Working autonomously

When the end goal is clear and boundaries are set, keep going, not ask.

A task is clear when all four can be written down:
1. End goal with definition of done (passing test, real capture, gate run)
2. Boundaries (crates, files, applicable rules)
3. How you will know it failed
4. What needs the bench (radio, off-air recording, another person)

Then work the queue end to end. Stop only for:
- F4NRM's areas, his panels, or the wire → branch, tests, PR, carry on
- A release or tag (never; operator and F4NRM cut those)
- Deleting branches, history, or work you didn't create
- A measurement needing a radio, recording, or another person
- A genuinely operator decision (product, licence, public reply, money)
- A check you cannot run (wasm target, platform build)

Keep the journal as you go: `docs/internal/AUTONOMY-JOURNAL.md`, newest first,
one entry per work item.

## Task tracking (Kanban + TODO)

- Every task started goes into `TODO.md` under **Now** (one line: what, why,
  where it came from) **and** the GitHub Projects board's **In Progress**
  before starting. Both must be updated — not just one.
- Every task finished moves in `TODO.md` to **Recently done** (today's date,
  one line) **and** to the board's **Done**; remove from In Progress. Both
  must be updated.
- Every task blocked goes under **Waiting on other people** in `TODO.md` with
  the single question, and to the board's **Blocked**.
- **Issues take priority over development.** Check open issues before starting
  new work. A report against an area is a new report about a specific fault.
- **Each task must name who added it and who owns it.** Format:
  `Item (added by X, owned by Y) — description.`
- The board is scripted (`gh project item-create` + `item-edit`). If the
  `project` scope is not granted, update `PROJECT-BOARD.md` itself and say so.
  Either way, `PROJECT-BOARD.md` must reflect the current state — it is the
  local mirror of the GitHub Projects board and must be updated for every
  task, not only when the script fails.
- Re-measurable numbers (crate version, PROTO_VERSION, open issues/PRs) must be
  kept current in TODO.md's state line and AGENTS.md.

## The wire

`PROTO_VERSION` and `VERSION_BYTE` live in `crates/sdroxide-proto/src/lib.rs`,
the only register kept current. Current on main: **256 / `0x4C`** (AGENTS.md
may lag; verify in code). Append last, always. A variant inserted mid-enum
shifts every discriminant below it. Two people work this repository — the
maintainer's agent and F4NRM's sessions — and `PROTO_VERSION` is one counter
between them.

- A wire change goes to a PR, never straight to `main`. This includes adding a
  `Command`, `ServerMsg`, `RadioEvent`, `Mode` or `Action` variant, a
  `DigiConfig`/`RadioConfig`/`NetworkConfig` field, or a `Mode` discriminant.
- `VERSION_BYTE` rides `PROTO_VERSION`. Bump it, or the client passes every
  test and is refused at the socket.
- A new wire variant needs a postcard round-trip test.
- `UiSettings` is not postcard-safe. The presentation-only wire type is
  `ClientScreen`. `InputSettings` is safe.
- `DigiConfig` and `RadioConfig` ride whole: never write one from a panel's
  stale copy.

## Build, test, and the gate

- `cargo check --workspace --all-targets` — the gate, must be silent.
- Test the crates you touched: `cargo test --release -p <crate>`. Reserve
  `--workspace` for release day.
- `sdroxide-radio`'s tests: name the binary (`cargo test -p sdroxide-radio
  --test cw_panadapter`).
- Browser build: `cargo check --release --target wasm32-unknown-unknown -p
  sdroxide-ui` when wasm-reachable code changes.
- Known flakes: `skim_window`, `sdroxide-pluto`'s `iiod_loopback`,
  `sdroxide`'s `icomnet_source`.
- A test that touches the machine is not a test: split decision from side effect.

## Releases

- No release happens by itself. No workflow carries a `schedule:` trigger.
  F4NRM cuts pre-releases; full release when he says everything is good.
- Release day: full gate green before the tag.
- Bump `Cargo.toml`, let cargo refresh `Cargo.lock`, check with `--locked`.
- Write changelog entry, tag `vX.Y.Z_brown`, push. Do not also dispatch.
- Never re-cut a tag. Bad tag → fourth-point tag (`v2.0.9.1_brown`).
- Install locally: `cargo build --release`, copy to `~/.cargo/bin/sdroxide`.

## Pull requests

A PR must be clean and mergeable: up to date with main, gate silent, touched
crates' tests passing, rustfmt clean on touched files, one complete thing per PR.

- F4NRM's PRs are never merged by us. He merges his own.
- Our own PRs are never merged by us. They sit until F4NRM reviews or merges.
- A sandboxed build needs `github.com/xiph/opus` and `media.xiph.org`.

## Hardware (bench)

- SDRplay RSP1 via `sdrplay_apiService`. Gain 20-59 dB, 0.01-2000 MHz.
- CRT SS9900v (11 m CB) as radio 1: USB audio, no CAT, keyed by VOX.
- Transmitting authorised on licence-free frequency — keep it short.
- Traps: rig's Mic/PCM levels separate; squelch defaults wide open; wireless
  mouse radiates into SS9900v.
- No Pluto, no HackRF on this network.

## Focus

No UI work unless a new mode needs a panel or something is very off. The
programme is the modes that did not finish in 2.1.0: 8PSK filter, wide PSK
banks, speeds that lose a word, MT63/Contestia/DominoEX variants. STANAG 4285
is finished. F4NRM's list (SSTV, DAB, NAVTEX, APRS, WFM, DSC, Olivia, RTTY,
WEFAX, FT8, RDS, HackRF, PlutoSDR) is his — report faults as PRs, never
unrequested tidy-ups. RSP1 is ours. DMR is out (patent). ATS Mini/DiamondRadio
are closed/unlicensed (research only). SDRuno look is parked.

## Where the work stands

- `main` at crate 2.2.0, PROTO_VERSION 256 (0x4C). Latest release v2.2.0.3_brown.
- Open issues: 13 (re-measured 2026-10-29). Open PRs: 4 (antenna #304, navtex
  #317, adsb #312, ft8 #323) + upstream #651 (SAT Doppler, SP9TKN testing).
- STANAG 4285 finished (Mark proved decode with video; coded payload not
  implemented — legal restriction). Olivia proven by user (RX+TX). Grey-line
  confirmed. S-meter still an issue.
- Live handovers: ALE (no real burst yet), STANAG 4285 (finished — Mark proved
  decode with video; coded payload intentionally not implemented, restricted by
  law in several countries), Olivia (proven on air RX+TX), CW keyer
  (built), SWL schedule sources (scoped), FST4W (do not ship).
- Confirmed on air: WEFAX, DAB, SSTV, DSC, HD Radio, Olivia.

## Current work

- SAT Doppler for CAT rigs: branch `fork/sat-doppler-cat`, step 1+2a+2b+2c
  implemented. MSI build available. Wacht op SP9TKN's testresultaten.
- NAVTEX restore: PR #317 open, mergeable, wacht op F4NRM's test.
- ADS-B overlay: PR #312 open, wacht op F4NRM's review.
- Airspy HF+: PR #309 open, wacht op Graeme's test.
- Antenna: PR #304 open.
- FT8 auto queue UI: PR #323 open (not live-tested).


## History

`docs/internal/SESSIONS.md` is the record (2026-09-25 to 2026-10-24). Read it
when a rule needs its evidence. It is a record, not current state.
