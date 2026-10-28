# Autonomy journal

The running record of work done without the operator at the screen. **Newest
first**, one entry per work item, written when the item ends — a session can be
closed without warning, so nothing is held back for a tidy write-up at the end.

The rules for working this way are under "Working on your own" in
[`../../AGENTS.md`](../../AGENTS.md). The queue is [`../../TODO.md`](../../TODO.md)
and the view is [`PROJECT-BOARD.md`](PROJECT-BOARD.md). This file is not a
narrative and not a changelog: it is the state of play, so the next session (or
the operator after a night away) can start from it in one read.

Each entry, in this order:

- **Summary** — three or four plain sentences for the operator: what changed for
  him, what he can look at, what is still not done, what comes next. No crate
  names, no file paths, no jargon.
- **Done** — the item, the commits, and the commands with their numbers.
- **Not done / blocked** — what stopped, and the single question or dependency.
- **Next** — the first step of the next item.

---

## 2026-10-28 (Radio Classic — the panel, and the audio level it needs)

**In plain words.** EXTRAS now has a RADIO chip that opens a classic-radio face
under the waterfall, the way DAB does: bands, memories and a schedule of what is
on the air on the left; the mode, a small spectrum, the S-meter and the stereo VU
in the middle; clocks, a big interactive frequency, the station's RDS name,
RadioText and the RDS chips on the right. It follows the theme, and Settings →
Detachable has a line and a switch to put it in a window. It is not merged and
should not be: the VU and the bars needed one new message from the station, which
is a wire change (256 → 257), so it is a PR. Not yet seen with a real station's
audio and RDS, and the click paths (the EXTRAS chip, a schedule row) are untested
here.

**Done.**

- `2aa127c` — the audio level on the wire: `ServerMsg::AudioLevel` appended last,
  `PROTO_VERSION` 256 → 257, `VERSION_BYTE` 0x4C → 0x4D, register line, postcard
  round trip, and `the_audio_level_message_is_appended_last` — **shown to fail**
  with the variant moved above `DabSlide` (`AudioLevel must directly follow
  DabSlide`) and to pass with it back. The engine measures where the sound is
  handed to the mixer (`AudioMeter` in `sdroxide-dsp`: mean power over 33 ms, 48
  log bands from a 1024-point FFT; silence is sent once, not streamed); the
  meter, the bands and the byte mapping have their own tests in their crates.
- `320e245` — the panel (`panels/radio_classic.rs`), `TW_RADIO_CLASSIC` (bit 50),
  `UiSettings::radio_classic_on`, the EXTRAS chip, the docking in the voice branch
  of `frame.rs` (digital modes, DAB and CW untouched), the Detachable line.
  Where a mode's panel owns the slot, or on a phone, it opens as a window instead.
- Rendered the real panel to a PNG in Classic and Amber Phosphor, in FM and in AM
  (`radio_classic_preview`, ignored) and looked at all of them. That found the
  mini waterfall empty (the docked layout holds the spectrum frame out of the app
  while it draws, so the panel is handed it) and two glyphs drawn as boxes.
- Found by reading rather than by a test: `Instant::now()` panics in the browser
  build, which is the main client, so the level is stamped with the wall clock.

**Numbers.** `cargo check --workspace --all-targets`: builds, **11 warnings, all
in files this branch does not touch** (navtex, text_modem, tci, server aircraft,
ui spectrum and sstv_composer, stanag4285) — the gate is not silent on `main`
either; not fixed here. `cargo test --release -p sdroxide-ui`: 1329 passed, 0
failed. `-p sdroxide-types -p sdroxide-proto`: 820 and 20 passed. `-p sdroxide-dsp
--lib audio_bands`: 6 passed. `cargo check --release --target
wasm32-unknown-unknown -p sdroxide-ui`: clean. `rustfmt --check` clean on every
file touched except two hunks that were already unformatted on `main`
(`engine.rs` ~18624, `frame.rs` ~6629).

**Not done / blocked.**

- Not run against a real station: the level, the bars and the RDS block have only
  been fed synthetic data. The first look at a real FM station is the bench's.
- The sandbox cannot fetch `github.com/xiph/opus` by archive, so `sdroxide-rade`
  was built with `OPUS_URL` pointing at a clone of the pinned commit; nothing in
  the tree changed for it. `sdroxide-radio`'s test binaries were not run (the
  engine change is five lines of glue, compiled).
- Every client is now sent about 30 small messages a second (about 100 bytes
  each) whether or not the panel is open. If that matters on a poor link, the
  next step is a `Command` that asks for the level — which is a second wire
  change, so not done unasked.
- The memory list shows the first four memories only; a longer list wants a
  scrolling row. SCHEDULE shows at most 14 stations, the strongest on the air now
  on the dial's band.

**Review follow-up (same day).** madmedicnl asked that the level stream only while a panel is open. `Command::WantAudioLevel` (appended last, still wire 257 / `0x4D`) is a 3 s lease the open panel renews every second; the engine meters and sends only under a lease. Test `the_panel_asks_for_the_audio_level_only_while_it_is_open` fails with the push removed and passes with it. Also merged `main` (one journal conflict, both entries kept). Engine side not run against a real station.

**Next.** The PR waits for F4NRM's review; the next item in `TODO.md`.

---

## 2026-10-28 (#239 WEFAX — auto-correct a chart received negative)

**In plain words.** Fixed the remaining part of F4NRM's WEFAX report: a chart
that arrives negative (black and white swapped) because the receiver is on the
wrong sideband is now fixed automatically. The decoder spots it from the
phasing signal at lock time and flips the pixels so the chart comes out the
right way round. The manual ROTATE 180° button stays for a genuinely rotated
source. It's opened as a PR for F4NRM to read, since WEFAX is on his list.

**Done.**

- The phasing pulse is found in either polarity, and the page's polarity is
  recorded at lock (start tone + phasing, or phasing alone). The emitted
  pixels are inverted when negative. Lock, auto-slant and mid-picture rephase
  all use the settled polarity so a normal chart can't be misread.
- Branch `wefax-negative-autocorrect`, commit `590b4a2a` (rebased onto current
  `main` after #293 merged), PR
  [#295](https://github.com/madmedicnl/sdroxide-brown/pull/295) opened against
  `main`.
- `cargo test -p sdroxide-dsp --lib` — 511 passed, 0 failed (3 new tests:
  negative phasing detection/flagging, negative signal corrected via start+phasing,
  negative phasing-signal-only start corrected). Test verified to fail on unfixed
  code, then restored. `cargo check --workspace --all-targets` silent.

**Not done / blocked.** Needs a real on-air negative capture to confirm, and
PR needs F4NRM's review/merge. No wire change.

**Next.** Look at the next item in the queue — #239 is the main one addressed;
issue #91 (triggering without signal) remains blocked on a recording.

---

## 2026-10-28 (#257 NAVTEX — no more false sync on noise)

**In plain words.** Stopped the NAVTEX decoder from claiming SYNC on noise. The
sensitivity scaling that landed after F4NRM's working build made positive
sensitivity too permissive, so the bit-phase lock could grab a random noise
signal and print garbage while reporting SYNC. The lock now needs stronger
agreement at non-negative sensitivity, while sensitivity 0 behaves exactly as
before, so the reference recordings still decode. Up as a PR for F4NRM to read,
since NAVTEX is on his list.

**Done.**

- The BitPhase sync / loss / disagree thresholds are tightened for non-negative
  sensitivity; sensitivity 0 is bit-identical to the pre-regression build.
- Branch `navtex-false-sync-fix`, commit `5aac9587` (rebased onto current `main`
  after #293 merged), PR
  [#298](https://github.com/madmedicnl/sdroxide-brown/pull/298) opened against
  `main`.
- F4NRM's own capture from #257 added as a fixture
  (`tests/testdata/navtex/F4NRM-2026-10-22-001113-48k.wav` + `.txt`), and the
  false sync reproduced on noise before the fix.
- `cargo test -p sdroxide-dsp --lib` and `-p sdroxide-digi --lib` pass;
  `navtex_reference` (ignored) passes; `cargo check --workspace --all-targets`
  silent.

**Not done / blocked.** PR needs F4NRM's review/merge. A real on-air check on
the band is the bench's, to confirm no true signal is now missed.

**Next.** The queue in `TODO.md` — #294 rustls, then the 2.2.0 regressions.

---

## 2026-10-28 (HF+ skirt — view held inside the usable span)

**Done.**

- The panadapter widget read the raw sample rate (768 kHz) as the span it fits,
  zooms and pans within, so the skirt returned on tune. It now reads
  `display_span(sample_rate, usable_hz)` — the same number the app's own span
  already used — while the raw rate stays as the sub-receiver's DDC bound, where
  the skirt is still inside the stream.
- Commits `d07ffea6` (the fix) and the navtex-guard `.gitignore` (build artifacts).
- Gate `cargo check --workspace --all-targets` silent; wasm check green;
  `spectrum_view` tests 50 passed; `display_span` test passes.

**Not done / blocked.** The wiring is inside the egui widget and is not
unit-tested — verified by the gate and by Graeme's on-air report on the next
build. Mid-item the working tree was switched to `fork/antenna-tool` and my
uncommitted fix was stashed as "stash before antenna tool"; I re-applied and
committed it on `main`.

**Next.** Await Graeme's re-check. The NAVTEX 518 kHz capture is still pending.

---

## 2026-10-28 (#257 NAVTEX — guard set up, waiting on the capture)

**In plain words.** Started the NAVTEX work. The only recent change to the decoder
is the receive-sensitivity control, which is an exact no-op at Auto, so the
garbage F4NRM saw is on a real on-air signal that synthetic noise/carrier can't
reproduce. A background guard is now watching 518 kHz and will capture the next
transmission for us.

**Done.**

- Investigated: the NAVTEX change since the working 10-23 build is `c9364531`
  (sensitivity control). It scales `sync_score`/`loss_limit`/`disagree_limit` via
  `sens_k = 1 + 0.12·s`, no-op at Auto. The committed recordings (SITOR-B,
  Bermuda 48k) still decode; pure noise, a carrier, and noise+faint-carrier at
  0 and +5 all stay un-synced. So the regression needs a real capture.
- `tools/navtex-guard` (`1d8b3875`) — opens the RSP1 at 516.29 kHz, watches the
  1710 Hz carrier with a single-bin DFT, warms up to the noise floor, then
  streams raw CF32 to disk for 15 min when the carrier rises 8× above it.
  Running in the background (PID 16481) writing `/home/druid/navtex_518khz.bin`.

**Not done / blocked.**

- Waiting on the next 518 kHz transmission (~1 h from 08:00 UTC). Once captured:
  USB-demodulate the CF32 to 48 kHz audio, run it through `NavtexRx`, and see
  whether the garbage is the sensitivity slider or a false-sync regression. Fix
  goes up as a PR (NAVTEX is F4NRM's).

**Next.**

- When the capture lands, demod + reproduce + fix as a PR.

---

## 2026-10-28 (morning triage — issues and replies)

**In plain words.** Checked the queue and the issue replies while the operator
was away. Nothing needs a code change today: every fixable report is waiting on
a recording, a specific radio, or F4NRM. Two replies went out — one asking for
the right NAVTEX recording (the link was the WEFAX one), one thanking the
installer reporter for confirming and noting the icon fix — and the counts were
re-measured.

**Done.**

- Re-measured: **17 open issues** (was 18 — #260 Chrome web-audio closed),
  **2 open PRs, both F4NRM's** (#292 re-applies the MinGW static-link, #293
  Radio Classic panel, wire 257/0x4D). Fixed the stale wire register in
  `AGENTS.md` (243/0x3F → 256/0x4C). Commits `ac4f4f04`.
- Replied to #257 (F4NRM's "sample in the PRs" link is the DWD 3855 WEFAX
  recording, not a NAVTEX capture — asked for the right one) and #280 (the
  reporter confirmed the MSI runs; noted the desktop icon is embedded now).

**Not done / blocked.**

- #239 orientation — still waiting on the upside-down-vs-negative answer.
- #257 — the operator will find or capture a NAVTEX recording himself (the
  reply to F4NRM was edited to say so, rather than ask for another).
- #264 — waiting on F4NRM's PWA hard-reload result.
- #259/#262 — F4NRM's area, no new reports.

**Next.**

- Nothing in the queue that can move without a recording or F4NRM.

---

## 2026-10-28 (#265 — the range notices are logged, not shown)

**In plain words.** The "would not open on X, so it opened on Y" and "the dial
moved to Y" notices fire while the receiver is settling and the dial still holds
the previous frequency, which is noise. They are now logged only — the dial
still moves on screen — while a frequency the operator asks for and the rig
refuses is still said out loud.

**Done.**

- `fix(radio): log the startup/swap range notices instead of showing them`
  (`bd1f41df`) — `note_opened_elsewhere` and the `adopt_source` "dial moved"
  notice now `warn!` only (the `RadioEvent::Notice` sends are gone). The
  operator-tune refusal (`tune_refused`) keeps its notice.

**Not done / blocked.**

- Corrected after the operator's note: the ADS-B→VDL2-on-Pluto landing on
  2.941 MHz is **not** a mode fault — the dial simply sits on the old mode's
  frequency for a moment during a switch, which is when the notice caught it.
  The demotion above is the whole fix.

**Next.**

- The #239 orientation question is still the open one.

---

## 2026-10-28 (the SWL fix, corrected — the seed must not touch a transceiver)

**In plain words.** The earlier switch fix was the wrong layer. The real cause is
the global "start in SWL mode" preference seeding SWL mode on every radio — so a
transceiver came up in SWL mode and the LISTEN/OPERATE tab did nothing until the
seed was cleared by hand. Now a transceiver drops that seed as soon as its front
end reports it can transmit, so it comes up with its transmit controls and the
tab works. `--swl` still forces SWL mode.

**Done.**

- Operator's report: on OPERATE, the SWL switch unchecked, LISTEN↔OPERATE did
  nothing; toggling the switch in settings fixed it. Confirmed the cause:
  `start_swl = true` seeds `swl_start` on every radio, and `swl_mode()` ORs it in
  ahead of the tab.
- `fix(ui): a transceiver does not start in SWL mode …` (`d8b7513c`) — the
  `Capabilities` handler drops `swl_start` once the front end is
  transmit-capable, unless `--swl` forced it. Helper
  `drop_swl_seed_for_transmitter` + test
  `a_transmitter_drops_the_start_in_swl_seed_but_not_a_forced_run`.
- Kept the earlier `cb069dd4` switch fix (the checkbox now shows the whole SWL
  state), which is still right for receive-only radios.

**Not done / blocked.**

- Not tested live — the operator's next launch against their own config is the
  confirmation.

**Next.**

- The #239 orientation question is still the open one.

---

## 2026-10-27 (the CRT starts in SWL mode and needs two switch flips)

**In plain words.** A transceiver that came up in SWL mode needed the SWL switch
toggled twice to get its transmit controls back — once to clear the seed, once
to undo the first toggle. The switch now reads as "on" when the radio is
actually in SWL mode, so one toggle turns it off. Both SWL switches (Radio tab
and UI tab) share the one state.

**Done.**

- Confirmed the cause in the operator's own config: `config.toml` has
  `start_swl = true`, which seeds `swl_start` on *every* radio at startup, while
  the CRT (radio 1) has `hide_tx = false`. The per-radio switch showed only
  `hide_tx`, so it looked off while `swl_start` held SWL mode on.
- `fix(ui): the SWL switch clears the start-in-SWL seed in one toggle`
  (`cb069dd4`) — a `swl_hide` state seeded from `swl_start || hide_tx`, bound to
  both switches, and `clear_swl_seed(hide_changed, swl_start, swl_hide)` clears
  the seed on a single off-toggle even when `hide_tx` never changed.
- Test `turning_the_swl_switch_off_clears_the_seed_in_one_toggle`. Gate + wasm
  check clean.

**Not done / blocked.**

- Not tested live (a restart that comes back up in SWL mode) — the unit test
  pins the decision, but the operator's next launch is the confirmation.

**Next.**

- The #239 orientation question is still the open one; otherwise pick up the
  queue.

---

## 2026-10-27 (#239 orientation — a ROTATE 180° chip as the in-between step)

**In plain words.** A manual half-turn is now in the WEFAX panel: a "ROTATE
180°" chip that flips a chart that arrived upside-down. It only acts once the
chart is complete — while lines are still arriving it is greyed out, because
turning the half already on screen while the rest keeps appending would scramble
the picture. The automatic orientation detection is still open; this is the
hold-over so an upside-down chart is not stuck upside-down.

**Done.**

- `feat(wefax): a ROTATE 180° chip …` (`83e12b96`) — `WefaxUi::rotate_live`
  reverses the row-major buffer (rows bottom-to-top + pixels right-to-left =
  the half-turn), and the panel chip is gated with `add_enabled_ui` to
  `has_live() && !receiving`.
- Tests `the_half_turn_reverses_rows_and_pixels_together` (two turns return to
  the start) and `the_half_turn_of_nothing_is_a_no_op`. `cargo test --release
  -p sdroxide-ui --lib wefax::` 16/16; gate + wasm check clean.

**Not done / blocked.**

- The automatic orientation detection (the real #239 ask) is still open. The
  decoder has no signal cue for up/down — WEFAX is vertically symmetric — so a
  detector would have to read the picture content (text/title position), and I
  cannot view images with this model to verify one. One question out to the
  operator: was the observed flip upside-down or a negative (white-on-dark)?
- #257 (NAVTEX) still waits on F4NRM's sample.

**Next.**

- Answer the operator's upside-down-vs-negative question, then build the
  automatic detector; meanwhile #257 when F4NRM's sample arrives.

---

## 2026-10-27 (#263 zoom — the 48 kHz snap is a stale usable span)

**In plain words.** The zoom-freeze is fixed. When a radio is switched off it
runs on a 48 kHz placeholder, and linking the real SDR was carrying that
placeholder's "usable span" over into the new radio — so the panadapter thought
it could only show 48 kHz and snapped back to that on every zoom-out. A swap now
recomputes the span from the device actually in use. The fix is engine-side and
covered by a test that fails with the old number and passes with the new.

**Done.**

- Reproduced the trigger from the operator's log and drdatapi's tuner clue: a
  radio switched off at exit (`NullSource`, 48 kHz) leaves `usable_hz` at
  48 kHz, and `Engine::adopt_source` set `sample_rate`/`decimation` but left
  `usable_hz` to ride over from `..self.state.clone()`. Linking the 3 Msps RSP1
  kept `usable_hz` at 48 kHz, so `display_span` came up at 48 kHz and the view
  narrowed back to it.
- `fix(radio): recompute the usable span when a front end is swapped`
  (`c2889ac6`) — `adopt_source` now sets `state.usable_hz = self.usable_span_hz()`
  after the swap, the same as `set_decimation`. Covers the operator "link" and
  every reconnect path (`reopen_source` and `poll_reconnect` both call
  `adopt_source`).
- Test `crates/sdroxide-radio/tests/source_swap.rs`:
  `a_swap_recomputes_the_usable_span_instead_of_carrying_the_stand_in_s` — starts
  a 48 kHz stand-in, links a 3 Msps source with `usable_frac = 0.86`, asserts
  the usable span becomes 2 580 000 Hz. Verified to fail with the fix removed
  (`usable span follows the new source (2580000 Hz), not the stale stand-in
  (48000 Hz)`) and pass with it.
- Gate `cargo check --workspace --all-targets` finished (pre-existing warnings
  in ui/digi/dsp only; `sdroxide-radio` clean).
- Live check on the bench RSP1: asked 3 Msps, the device reports
  `3 000 000 Hz` and keeps the default `usable_frac = 1.0`, so the post-swap
  usable span is the whole 3 MHz — not the 48 kHz the stale stand-in left.

**Confirmed.**

- The operator re-ran the exact sequence (unlink → exit → relaunch → link →
  zoom out) on the fixed build. The 48 kHz snap is gone: after linking the
  RSP1 the view fits to 3 MHz, zooms in to ~647 kHz and back out to ~2 MHz with
  `clamped=false` throughout, and no `span_before=48000` entry appears anywhere
  (the old log had one after every link). `dev_span` stays `3000000`.

**Not done / blocked.**

- #239 orientation (the 180°-flipped WEEFAX picture) is still open.
- #257 (NAVTEX) still waits on F4NRM's sample.

**Next.**

- #263 is closed out: the fix is on `main`, confirmed live by the operator, and
  the reply is posted (thanking drdatapi for the tuner clue). Next: #239
  orientation.

---

## 2026-10-27 (public SDR, the wire guard, and the CW report)

**In plain words.** The public SDR list was listing receivers as online that had
gone down, so a connect that timed out looked like a dead program. A receiver
whose connect fails this session is now greyed out with "could not connect"
instead of being offered again. I also added a test that pins the wire layout of
the three config structs, so the mid-struct field mistake that needed
`PROTO_VERSION 255 → 256` cannot happen again without a failing test. And I
cleared a textures-delta panic that made one aircraft map-style test red on
`main`.

**Done.**

- Confirmed the public-SDR connect code is not the fault: the sdroxide connect
  path (both `probe` and `connect_wideband`) reaches the one live SpyServer
  (`101.58.3.220:5555`) and reads its device info; 7 of 8 directory "online"
  servers time out — the directory is stale. Replied to #264 asking for a PWA
  hard-reload, and to #282/#280 (fixed in 2.2.0).
- `feat(public-sdr): grey out a receiver whose connect failed this session`
  (`9fe6bf65`) — `dead_public_sdrs` + `last_public_sdr_try` in the app state,
  the "Interface change failed" notice charges the last-tried address, and the
  row greys out. Test `a_failed_interface_change_marks_the_receiver_just_tried_dead`.
- `test(proto): pin the wire layout …` (`5e2059e7`) — serializes
  `DigiConfig`/`RadioConfig`/`NetworkConfig` defaults and pins length + FNV-1a,
  so any layout change is a failing test. Gate silent, wasm build clean, ui +
  proto suites green.
- `test(ui): clear the textures delta in the map-style frame test` (`e57a4541`)
  — the aircraft map-style buttons draw an icon, so `run_ui` leaves a
  `TexturesDelta`; dropping without `drop_without_applying_deltas` panicked.
  Rebased on F4NRM's #286 (web-audio playout) merge.

**Not done / blocked.**

- #239 (WEEFAX): **auto-tilt done** (`61526907`) — the receiver measures the
  phasing pulse's drift across the phasing signal (spanning the lock) and trims
  `slant_ppm` once, above a 25 ppm floor. The decoder works on F4NRM's recording
  (one clean 2150-line chart, no false trigger on the carrier). **Orientation**
  (a 180°-flipped picture) is the remaining half of his ask, not built yet.
- #257 (NAVTEX): sensitivity scales the sync thresholds but is a no-op at Auto;
  needs the sample F4NRM said is "in the PRs" (asked him where exactly).
- #263 (zoom): the reporter's log shows the view snapping to 48 kHz on his
  SDRplay (2 Msps, `audio_mode=false`) after every zoom-out, so pan/zoom is
  frozen. The widget's own clamp uses `state.sample_rate` (2 MHz), so the snap
  comes from the app side — most likely `recenter_if_tuned_away` narrowing to
  `zoom_out_window()`'s span. The exact number isn't in the log, so a debug line
  (`9c72b485`) now prints `out_span`/`sample_rate`/`usable_hz` when a restored
  view is narrowed; the reporter's next run names it.

**Next.**

- #239: the orientation half (detect + flip a 180°-rotated picture).
- #263: wait for the next run (or reproduce on the RSP1) to see which number is
  48 kHz, then fix.
- #257: act on F4NRM's sample when it arrives.

---

## 2026-10-27 (the 2.2.0 release — Windows MSI and the MinGW C++ runtime)

**In plain words.** The 2.2.0 release is cut as `v2.2.0.2_brown` with the
AppImage, the Windows MSI, and — for the first time — the Apple Silicon Mac
builds. The Windows installer was broken in three layers and each fix exposed
the next; all three are fixed and the MSI now builds and passes WiX validation.
I also opened PR #284 so the Windows build links its C++ runtime statically
instead of shipping a 2.6 MB DLL — the check that was meant to guarantee this
had a broken grep and never fired.

**Done.**

- Bumped the crate 2.1.2 → 2.2.0 (Cargo.toml + Cargo.lock), wrote the changelog
  entry. Gate green: `cargo check --workspace --all-targets` silent,
  `cargo test --release --workspace` 294 suites / 0 failures,
  `cargo check --locked` clean.
- Tag `v2.2.0_brown` → MSI job failed `WIX0094` (the shortcut directories used
  `CommonProgramsFolder`/`CommonDesktopFolder`, which are the resolved *property*
  names, not WiX directory IDs). Fixed to `ProgramMenuFolder`/`DesktopFolder`.
- Re-tagged `v2.2.0.1_brown` → `ICE50` (the non-advertised shortcut + file key
  path became an advertised shortcut, and its `.ico` then mismatched the `.exe`).
  Fixed: advertised + `DISABLEADVTSHORTCUTS=1`, dropped the shortcut `Icon`
  (the exe has no embedded icon; Add/Remove Programs keeps the Brown one).
- Re-tagged `v2.2.0.2_brown` → **success**, 20 assets: AppImage, Windows MSI
  (version 2.2.0), macOS aarch64 + x86_64, Linux, Windows zip/soapy, web client.
  Deleted the two partial release pages (`v2.2.0_brown`, `v2.2.0.1_brown`).
- PR #284 (`fix/windows-mingw-static-libstdcxx`): the DLL-import check's grep
  spelled `+` as `\+` and never matched on the MSYS2 grep, so it passed while
  `libstdc++-6.dll` was still imported (confirmed via `objdump` on the
  v2.2.0.2 build). Fixed to `libstdc[+][+]`, and gave `sdroxide-olivia` and
  `sdroxide-s4285` the same `static:-bundle=stdc++`/`winpthread` treatment
  `sdroxide-drm` already had — those two compile C++ but left `cc`'s default
  `-lstdc++`.
- PR #285 (`fix/windows-exe-icon`): embed the Brown icon into `sdroxide.exe` so
  the advertised Start Menu / desktop shortcuts show it. winres 0.1 failed —
  MSYS2's `windres` mangles backslash paths (`D:asdroxide-brown...`); switched
  to a direct `windres` invocation with forward-slash paths and a one-line
  `resource.rc`. Verified by a `windows-msi.yml` dispatch: MSI built + uploaded.

**Not done / blocked.**

- PR #284 needs the Windows CI to prove the static link actually holds; no
  MinGW toolchain here so it could not be tested locally. `cargo check` passes
  (windows-gnu branch skipped on Linux) and the grep is verified on Linux.

**Next.**

- Review/merge PR #284 (static C++ runtime) and PR #285 (exe icon).

---

## 2026-10-27 (the Aircraft window, from the operator's five screenshots)

**In plain words.** Ten pull requests are open, none merged. They fix what the
screenshots showed: the French word "indisponible" still in six strings (my
earlier audit said there was one; that was wrong), distances and speeds in
metric or miles, the same fact repeated on every line of the card, columns that
printed over each other, three buttons for the map style instead of one that
cycled, a box for an airport, mode buttons that did nothing on the wrong band,
VDL2 and ACARS views that were a sentence, a button column cut off at the
window's smallest size, and a switch for the online photo
lookup that an operator can press. Nothing was seen in a picture and the
switch and the mode buttons were never tried against a real radio or a real
online answer; those are the first things to look at. Next: the narrow-window check, and whatever the review of these says.

**Done** (PR, base, what it carries):
- #266 `claude/aircraft-a-english-translate`, main — "online unavailable", the
  guard list extended, `translate="no"` on the web page.
- #267 `claude/aircraft-i-units`, main — aviation, metric, imperial.
- #268 `claude/aircraft-c-duplicates`, on #267 — radio badge and shared note once.
- #269 `claude/aircraft-d-columns`, on #268 — columns from the right, no overprint.
- #270 `claude/aircraft-e-style-buttons`, main — Radar, Terrain, Chart side by side.
- #271 `claude/aircraft-j-airport-card`, on #267 — airport card and pinned box.
- #272 `claude/aircraft-g-mode-buttons`, main — `SetModeListen`, HFDL enabled.
- #273 `claude/aircraft-h-views`, on #272 — VDL2 and ACARS views.
- #276 `claude/aircraft-f-narrow`, on #273 — the button column scrolls at
  560 x 320; seen in a rendered picture before and after (ignored preview test
  `aircraft_window_preview`).
- #274 `claude/aircraft-b-photo-switch`, main — `GET/POST /aircraft-online`,
  no `PROTO_VERSION` change (still 255 / 0x4B).
- Numbers: `cargo test --release -p sdroxide-ui` 1283 to 1288 passed per branch,
  native `--all-targets` without errors, wasm release check finished, rustfmt
  clean on what was touched. Tests were red before the fix where a fix existed
  (guard words, web page, columns overprint, hover card units); the others are
  new behaviour with their own tests.

**Not done / blocked.**
- Lot F was done with a rendered picture, but only at the smallest size and with
  stand-in data (the map showed no aircraft); not run in a real window.
- The radio and server crates do not build as-is in this sandbox (the RADE
  crate downloads opus, which is blocked). With a temporary stub of that crate
  (never committed) both were built and tested: `-p sdroxide-server` 80 unit and
  the integration tests green (#274), and a new engine test on the airband
  `SetMode` refused / `SetModeListen` taken (#272). Nothing on a real radio, and
  no real adsbdb/Planespotters answer.
- Only lot F was looked at in a rendered picture; the other lots were not, and
  the Chart style's colours are unchanged.
- #271 now also closes its box with Esc.
- `AGENTS.md` still says the relay is switched only by the environment variable.
- main carries "frozen until #263"; I opened PRs on the operator's explicit
  order and merged nothing.

**Next.** Read the review of the ten.

---

---

## 2026-10-26 (the 2.1.2 download is withdrawn, and nothing lands until #263 is understood)

**In plain words.** Nobody can download the 2.1.2 build any more, and the tag
that could have published it has been deleted, so a re-run cannot republish it
by accident. The version on offer is the one that works — v2.1.1.1, the
pre-release from this morning. Nothing else is being merged while we work out
what F4NRM is seeing, and every PR of ours stays where it is.

**Done.**
- Measured on origin: **no `v2.1.2.x` release page exists at all** — `gh release
  list` shows v2.1.1.1 as the newest (pre-release) and v2.1.0.2_brown as
  `Latest`. The two partial pages had already been deleted.
- The **dangling tag `v2.1.2.2_brown`** (`9f2c4dac`, its Release run 40397491309
  cancelled) was still on origin and is now deleted:
  `git push origin :refs/tags/v2.1.2.2_brown` → `- [deleted] v2.1.2.2_brown`.
  That tag was the only way the 2.1.2 build could still be published, by a
  re-run.
- Still open and untouched: **PR #258** (WEFAX centring), and every merged PR of
  today stays merged — the operator does not believe `main` is buggy and did not
  ask for a revert of the code.

**What the investigation has ruled out, with numbers.**
- v2.1.1.1 (the build that works, tagged 00:31 today) **contains #234**:
  `git merge-base --is-ancestor 433a2d74 v2.1.1.1` is true. So the handover's
  "today's Airspy display-span merge broke the panadapter" is wrong, and
  reverting it would fix nothing.
- **The panadapter's own code is byte-identical to v2.1.1.1**:
  `git diff --stat v2.1.1.1..main -- crates/sdroxide-ui/src/widgets/ crates/sdroxide-ui/src/app/spectrum.rs crates/sdroxide-ui/src/view.rs` is empty. The state/caps wire types are untouched too (`types/state.rs`, `types/caps.rs`).
- A headless probe drove the real app — capabilities, a snapshot, a frame, then a
  real Shift+wheel and a 120-point left-drag on the panadapter found by its own
  background colour — over ten configurations: 1.125 Msps SDR zoomed out and in,
  HF+ 0.86 usable span, a 48 kHz state against a 1.1 MHz frame, an audio rig, a
  CAT rig at 4.8 kHz, a wide lane over narrow I/Q, a bogus `usable_hz`, and no
  state at all. **Pan and zoom worked in every one but the no-state case**
  (zero-wide view). The probe was a throwaway and is **not in the tree**; a copy
  is at `/tmp/opencode/probe_263_keep.rs`.
- **Nothing on the radio side changed but two things** since v2.1.1.1:
  **#240** (the composer's six `ImageSetComposer`/`Image…` command arms in
  `engine.rs`, `image_store.rs`, wire 249→254) and **#253** (`DigiConfig` gains
  `auto_call_retries`/`auto_session_min`, wire 255). `sdroxide-cat`,
  `panadapter_source.rs`, `source.rs`, `device.rs`, `sdroxide-server`,
  `sdroxide-web` and `src/` have **no changes at all** in that range.
- The other four PRs after the tag are UI only: #254 (`scanner.rs`), #255
  (`frame.rs`, the fax CTR anchor), #241 (`persist.rs`) and #261 (windows:
  `frame`/`mod`/`top_bar`/`chrome`/`layout`/`multi`).

**Not done / blocked.**
- **#263 itself.** The one question out to F4NRM: *when he rolls the wheel over
  the waterfall, does the frequency change?* It separates "the gesture never
  reaches the widget" (a new overlay, which only #261 could have added) from "the
  widget gets it and refuses". Reproducing it needs his screen or his console:
  the client logs `panadapter view moved` with `dev_center`, `dev_span` and
  `view_lo/hi` on every move, target `sdroxide::panadapter`.
- **His SWR reading (3.5 against a tuner's 1.2) and FT8 not syncing after he
  downgraded.** No code path here changed: the Kenwood SWR is the rig's own `RM`
  meter bars read through a per-model curve chosen from its `ID;` reply, and that
  file is untouched since before the build he calls fine. A downgrade reverting
  the binary and leaving a fault means the fault is outside the binary — the
  browser client Chrome caches (the server refuses a wire mismatch by name), the
  station's own config files, or the audio/rig side.
- **No release is to be cut** and no PR merged while this runs, on the operator's
  instruction of tonight.

**Next.** Get the server log from the moment his browser reconnects: it either
names a protocol mismatch or shows the frames arriving. Then pick the half that
is dead before touching code.

---

## 2026-10-26 (v2.1.2_brown is missing Windows — `M_PI` in the vendored decoder)

**In plain words.** The full release published with **14 assets** — Linux (plain,
SoapySDR and the 22.04 compat build with its AppImage), macOS on both arches, and
the web client — but **no Windows build**. The cause is the vendored STANAG 4285
decoder: its headers use `M_PI`, which the MinGW toolchain on the runner does not
define. The same failure killed **v2.1.1.1** earlier today, so Windows has had no
build since the s4285 decoder was vendored, and neither page says so.

**Done.**
- `crates/sdroxide-s4285/build.rs` passes `-DM_PI=3.14159265358979323846` when
  `CARGO_CFG_TARGET_OS` is `windows`; `vendor/` untouched. Windows only, because
  glibc already defines it and a second definition is a redefinition warning.
- Verified as far as this machine can: `cargo check -p sdroxide-s4285` compiles
  on Linux, and `g++ -std=c++17 -UM_PI -DM_PI=… -fsyntax-only src/shim.cc` — the
  Windows shape, with the library's own `M_PI` removed and ours supplied —
  compiles clean. **The Windows job itself is the proof**, and no Windows target
  is installed here.

**Not done / blocked.**
- Recovery, on the operator's word: the partial **`v2.1.2_brown` release and its
  tag are deleted** (`gh release delete --cleanup-tag`, "it's too confusing"),
  the changelog heading moved to `## [2.1.2.1_brown]` (`9eb58559`) so the
  workflow reads the same notes, and **`v2.1.2.1_brown`** is tagged on it — the
  fourth-point tag the rule asks for, on crate version 2.1.2.
- The WEFAX multi-page puzzle: his two recordings each hold **one** page (the
  Drive file runs 42.7 s → 1121.6 s, 2153 lines, ended by its stop tone), so
  neither can reproduce "only the first one or two of ten". A short audio clip
  spanning a page boundary is what would settle it.

---

## 2026-10-26 (v2.1.2_brown — the full release)

**In plain words.** The windows work is merged and a **full release** is being cut
as **v2.1.2_brown**: one line per change in the notes with the author behind each,
and a short note saying what the new window icons are and what the three keys do.
The release workflow is running.

**Done.**
- **PR #261 merged** (`3ed50c4c`) — on the operator's explicit word ("merge it,
  we're cutting a full release"), which overrides the rule that only F4NRM merges
  our PRs. Recorded here because the rule is his and the exception should be
  visible, not inferred.
- `docs/internal/WINDOWS-HANDOVER.md` **rewritten** (`deef561b`) to describe what
  merged, not the states we passed through: the grammar table, the bar's
  constants, one drawn icon family, the browser having no `out`, the per-window
  layout answers, the Classic answer, and a "still owed" list.
- Release prep (`db3cbc4f`): crate **2.1.0 → 2.1.2**, `Cargo.lock` refreshed and
  `cargo check --locked -p sdroxide-ui` passing; `Unreleased` rewritten as one line
  per change with an author (`madmedicnl` / `F4NRM` / upstream pick), 26 lines
  against the workflow's 60-line ceiling, closing with the icons-and-keys note.
- Checks on the tagged tree: `cargo check --workspace --all-targets` compiles
  (only warnings already on `main`, in crates this does not touch);
  `cargo test --release --workspace` **294 suites, 0 failed**.
- Tag **`v2.1.2_brown`** (annotated, "SDR Oxide Brown 2.1.2") on `db3cbc4f`,
  pushed; release run **40380236447**, watched to its end.

**Not done / blocked.**
- Nothing from this release is installed locally: `~/.cargo/bin/sdroxide` is still
  the 17:28 windows build on purpose (the operator is using it tonight).
- **v2.1.1.1's own release run failed** (19m37s, 15:46 today) although its page has
  11 assets — a job after the uploads, so the page is complete and the failure is
  worth one line of reading before the next release.
- **#258** (WEFAX centring) is still open with F4NRM — the only PR of ours there.

## 2026-10-26 (the panel window's bar takes the panel's background)

**In plain words.** The panel's own OS window opened with a black strip and one
full-size chip above the panel, so it read as a second panel pasted on top. It
now wears the same title bar as the float — 14 pt, the panel's name at 10 pt, the
dock glyph at 12 pt on the right — drawn on the panel's own background, with the
whole window filled in that colour so no black shows around it.

**Done.**
- Commit **`3431aae9`** on `local/windows`, not pushed.
- `frame.rs`, the panel's detached window: an `egui::Frame` filled with
  `theme::PANEL()` (the same colour egui's `window_fill` uses, so the float and the
  window match), 4 pt margin, the bar from `chrome::{BAR_H, BAR_CHIP, BAR_FONT}`,
  a separator, then the panel.
- Gate 0 errors, cargo test --release -p sdroxide-ui --lib 1283 passed / 0 failed,
  wasm32 clean, rustfmt clean.
- **Not seen**: a detached OS window cannot be photographed headlessly (embedded
  viewports), so this one is his eye on the real window; the float's bar — same
  three constants — is the render at `/tmp/opencode/float-bar14.png`.



**In plain words.** The floating panel's title bar is now 14 pt tall with 12 pt
square buttons and a 10 pt name — the operator's numbers, and "width relative to
the height" is exactly how they are drawn: the picture is a fraction of whatever
box it gets. At 12 pt the three buttons are still readable as shapes but they are
a small mouse target; the three constants are in one place if he wants them
larger.

**Done.**
- Commit **`e3742eef`** on `local/windows`, not pushed.
- `chrome::{BAR_H, BAR_CHIP, BAR_FONT}` = 14 / 12 / 10, and
  `chip_window_icon_sized` for the exact box (fixed 2 pt inset instead of the
  theme's chip padding, which is sized for buttons and would have left an 8 pt
  chip with a 6 pt picture).
- Gate 0 errors, cargo test --release -p sdroxide-ui --lib 1283 passed / 0 failed,
  wasm32 clean, rustfmt clean.
- Seen: `/tmp/opencode/float-bar14.png`.



**In plain words.** The three buttons in the new title bar were three different
weights — a painted arrow next to a font ▾ and a font × — and he saw it at once.
They are now one drawn family, same chip, same stroke, same size: out, roll-up,
close, and the dock picture the panel window uses, all from one painter. And the
"lift the pane out" chip now reaches the panels that never had it — WSPR, PI4,
NAVTEX, FSQ, JTTY, RADE and SSTV — so it sits in the same corner on every panel
that has a header row.

**Done.**
- Commit **`0b8bd86c`** on `local/windows`, not pushed.
- `WindowIcon::{Collapse, Unfold, Close}` drawn in `paint_window_icon` at the
  same 1.3 stroke and the same fractions of the box as `Out`/`In`/`Dock`; the
  float's title bar draws *only* painted glyphs now, no font characters.
- `window_chip` (the pane's way out) added to the header row of WSPR, PI4,
  NAVTEX, FSQ, JTTY, RADE and SSTV.
- **Not done, and why**: NAVTEX's and JTTY's header functions have no `&mut self`,
  so the chip cannot call `float_operating_panel()` from there — those two stay
  on `Ctrl+Alt+F` and the list, and the chip edits were backed out rather than
  plumbed through a signature. RF Paint and the text modems were left alone for
  the same reason: their first `row_tail` is not a header row, and guessing would
  have put a window button in a list.
- Numbers: `cargo check --workspace --all-targets` **0 errors**; `cargo test
  --release -p sdroxide-ui --lib` **1283 passed, 0 failed, 10 ignored**; wasm32
  clean; `rustfmt --check` clean on every file touched.
- **Seen**: `floating_panel_preview` at 540 (`/tmp/opencode/float-icons2.png`).

**Next.** Whatever he says after looking at the bar: the drag feel, roll-up
height, and whether the two `&mut self`-less headers are worth a signature change.



**In plain words.** The floating panel now has its **own title bar** with the
three things a window has: out to a window of its own, roll up, and close — and
close still docks it, because the panel is the control surface, not a tool. Roll
up is the answer to floating on a laptop: the panel becomes its title bar and the
waterfall gets the rest of the screen. This is the first change in the whole
windows thread that *removes* more than it adds: the floating panel's chips row
is gone, the panel's OS window carries one dock picture instead of a row of chips,
and the ▣ is out of every panel header — the list is one place again, the ▣ beside
the gear and `Ctrl+Alt+W`.

**Done.**
- Commit **`151828df`** on `local/windows`. Not pushed: he tests first.
- `frame.rs`: the floating panel is `.title_bar(false)` with our own bar — mode
  name (a noun) left, then `WindowIcon::Out`, roll-up and `×` right, in that order.
  egui has no hook for a fourth title-bar button (`title_ui` is private and draws
  only collapse, title and close), which is why the bar is ours; with no title bar
  egui falls back to dragging from anywhere, so the bar's ground and the body's
  empty space move the window and controls keep their own drags.
- `panel_rolled` (session state): the body is not drawn when rolled, `min_height`
  is 0 so the window really does become a bar, and `float_operating_panel()`
  unrolls — a float remembered rolled up would hand back a title bar and nothing.
- **Removed**: the floating panel's chips row, `panels::panel_has_windows_chip`
  and its mode table and test, the ▣ in the six panel headers, packet's
  `windows_list` plumbing, `WindowsChip` / `module_windows_chip` /
  `SdroxideApp::windows_chip` / `panel_windows_chip`, and `chrome::aside_popup`
  with its open/close pair (the `popup_body` `aside` flag went with them).
- The panel's **OS window** row is now one glyph — the dock picture — in its own
  content, because a tiling WM draws no title bar to hold it. Floating from a
  window is two steps now (dock, then float) or the list's middle icon.
- `Ctrl+Alt+F` **stays**: panels with no header chip row (SSTV, WSPR, NAVTEX, PI4,
  FSQ, JTTY, CW, RADE, RF Paint, the text modems) have no other way to float.
- Numbers: `cargo check --workspace --all-targets` **0 errors** (the same two
  pre-existing warnings); `cargo test --release -p sdroxide-ui --lib` **1283
  passed, 0 failed, 10 ignored**; wasm32 clean; `rustfmt --check` clean on every
  file touched.
- **Seen, not assumed**: `floating_panel_preview` renders the new bar — FT8 left,
  out · roll-up · close right, panel beneath (`/tmp/opencode/float-titlebar.png`).
- Test rewritten: `every_state_of_the_operating_pane_has_a_way_out_of_it` now
  pins the window grammar — the float's bar carries all three, the bar is ours,
  close docks, out detaches, and a panel can still float from its own header.

**Not done / blocked.**
- **Dragging by the bar** relies on egui's drag-anywhere fallback; it needs his
  hand on it (a drag on a label may go to the window rather than to nothing).
- The panels **without** a header chip row still have no ⇱ float chip: SSTV floats
  with `Ctrl+Alt+F` or the list, not from its own header. Giving every panel that
  chip means a row on the panels that have none, which is his call.
- Step 5 of the handover (deleting the remaining float/dock chips) is now most of
  the way done by this commit; what is left is the band popup's chips and the
  radio strip's WINDOWS menu.



**In plain words.** Right click on the ▣ did nothing visible because the list it
opened was drawn in the **main window** — and he was looking at a panel in a
window of its own. The list is now an ordinary menu next to the chip he clicked,
in the same window, closing when he clicks elsewhere. And the chips row above a
detached panel window is gone where the panel's own header now carries the ▣:
that row was the way home, the header has it, so the height goes back to the
decoder — which is what he meant by "safe to remove the top row".

**Done.**
- Commit **`f52099af`** on `local/windows`.
- `chrome.rs`: `aside_popup` / `open_aside_popup` / `close_aside_popup` — the
  menu chrome with `Popup::from_response` instead of the toggle form, so one
  button opens it and the other does the docking. `popup_body` takes an `aside`
  flag; the three existing menu popups pass `false` and are unchanged.
- `frame.rs`: the list is drawn **anchored to the chip**, inside whatever window
  the chip is in (`self.windows_chip`); the separate list *window* is now only
  what `Ctrl+Alt+W` opens, because with every module docked there is no chip to
  hang a popup on. `panel_windows_chip(ui, mode)` is the one call a panel header
  makes: it knows whether the panel is in a window, so left docks *that* panel
  and otherwise docks every window home — and where nothing is out, the left
  press opens the list, so it is never a press that can only do nothing.
- `panels::panel_has_windows_chip(mode)` — the panels that draw the ▣ in their own
  header (FT8/FT4/FT2, JS8, AtCHAT, ADS-B, SSTV/RIFP). The detached panel window
  and the floating panel skip their chips row for exactly those; the rest keep it,
  because without it a window has no way home on a desktop with no title bars.
  Packet keeps the row: its header has no `SdroxideApp`, so its ▣ reports the
  click out like its `detach` and cannot open a popup where it stands.
- Numbers: `cargo check --workspace --all-targets` **0 errors**; `cargo test
  --release -p sdroxide-ui --lib` **1285 passed, 0 failed, 10 ignored**; wasm32
  clean (the first run caught `panel_has_windows_chip` in the floating panel,
  which exists in the browser too); `rustfmt --check` clean on every file touched
  (frame.rs and packet.rs were rustfmt-clean before this, so the run touched only
  these hunks).
- Test: `every_panel_with_a_chip_row_actually_draws_it` reads the five panel files
  and pins the mode list in both directions.

**Not done / blocked.**
- The panels with **no chip row at all** (WSPR, PI4, NAVTEX, FSQ, JTTY, CW, RADE,
  RF Paint, the text modems) still have nowhere for the ▣ — that is the operator's
  call, since it means giving each panel a row.
- Step 5 (deleting the old float/dock chips) and the list's filter/arrow keys are
  untouched.



**In plain words.** The ▣ is now the window's own button, the way he asked:
**left click brings the window back** and **right click opens the list**. In a
window it docks that window; from a docked panel's header or the strip, where
there is no window of its own, the left press brings every window home — the
recovery you actually want when they are scattered — and the chip is not shown
when nothing is out, so a press can never do nothing. The hover names both
buttons, because a chip with a second button nobody knows about is not a control.

**Done.**
- Commit **`a9d47d20`** on `local/windows`.
- `panels::module_windows_chip(ui, dock)` returns `WindowsChip { Dock, List,
  None }`; `SdroxideApp::windows_chip(ui, dock)` is the one place both actions are
  applied, so the panel headers (SSTV, FT8 family, JS8, AtCHAT, packet, ADS-B),
  the floating panel, the detached panel window and the System row cannot drift.
  Packet's header has no `self`, so it reports both through out-parameters the way
  its `detach` already did.
- Numbers: `cargo check --workspace --all-targets` **0 errors**; `cargo test
  --release -p sdroxide-ui --lib` **1284 passed, 0 failed, 10 ignored**;
  wasm32 clean; `rustfmt --check` clean on every file touched.
- Test: `the_module_chip_docks_on_the_left_and_lists_on_the_right` draws the chip
  in one frame to learn its rect, then sends a real press-and-release on it —
  primary gives `Dock`, secondary gives `List`. It fails on the old chip, which
  reported a single `bool` for a left click and nothing for a right one.

**Not done / blocked.**
- The same as the entry below: panels with **no chip row** (WSPR, PI4, NAVTEX,
  FSQ, JTTY, CW, RADE, RF Paint, the text modems) still have nowhere for the ▣ —
  it needs a row of chips on those panels, which is the operator's call.
- Step 5 (deleting the old float/dock chips, now that the ▣ does the docking) is
  untouched, and so is the list's filter box / arrow-key navigation.



## 2026-10-26 (the title bar at 14 pt)

**Done.** Commit `e3742eef`: `chrome::{BAR_H, BAR_CHIP, BAR_FONT}` = 14 / 12 / 10
and `chip_window_icon_sized` for the exact box.

## 2026-10-26 (one drawn icon family, and the float chip on more panels)

**Done.** Commit `0b8bd86c`: `WindowIcon::{Collapse, Unfold, Close}` painted at the
same 1.3 stroke as the rest, so every window button is one family; and
`window_chip` added to WSPR, PI4, NAVTEX, FSQ, JTTY, RADE and SSTV's headers
(NAVTEX and JTTY could not take it — their header is a free function with no
`&mut self` — and RF Paint's first chip row is a list, so it was left alone).

## 2026-10-26 (the float's title bar carries the window button)

**Done.** Commit `151828df`: the floating panel is `.title_bar(false)` with our own
bar — mode name left, out · roll-up · close right — `panel_rolled` for the roll-up,
and the chips row, the mode table, the ▣ in panel headers and the two-button chip
machinery all removed.

## 2026-10-26 (the ▣'s list opens where it was clicked)

**Done.** Commit `f52099af`: the list became an aside popup anchored to the chip
(right click) instead of a window drawn by the main frame — which on a tiling
desktop was a window he could not see. The chips row above a panel window was
skipped for panels that carried the ▣ in their own header
(`panel_has_windows_chip`), **both of which this session has since undone** in
favour of the float's title bar (`151828df`): the popup, the two-button chip and
the mode table are gone, and the OS window carries one dock glyph.

## 2026-10-26 (the ▣: left docks, right lists)

**In plain words.** The ▣ is now the window's own button, the way he asked:
**left click brings the window back** and **right click opens the list**. In a
window it docks that window; from a docked panel's header or the strip, where
there is no window of its own, the left press brings every window home.

**Done.**
- Commit **`a9d47d20`** on `local/windows`.
- `panels::module_windows_chip(ui, dock)` returns `WindowsChip { Dock, List,
  None }`; `SdroxideApp::windows_chip(ui, dock)` is the one place both actions are
  applied, so the panel headers (SSTV, FT8 family, JS8, AtCHAT, packet, ADS-B),
  the floating panel, the detached panel window and the System row cannot drift.
  Packet's header has no `self`, so it reports both through out-parameters the way
  its `detach` already did.
- Numbers: `cargo check --workspace --all-targets` **0 errors**; `cargo test
  --release -p sdroxide-ui --lib` **1284 passed, 0 failed, 10 ignored**;
  wasm32 clean; `rustfmt --check` clean on every file touched.
- Test: `the_module_chip_docks_on_the_left_and_lists_on_the_right` draws the chip
  in one frame to learn its rect, then sends a real press-and-release on it —
  primary gives `Dock`, secondary gives `List`.

**Not done / blocked.** Panels with no chip row of their own still have nowhere
for the ▣ (the operator's call), step 5 is untouched, and so is the list's filter
box / arrow-key navigation.

## 2026-10-26 (the Classic strip, and the ▣ where the panels are)

**In plain words.** Two faults from his screenshot. In Classic UI with the
spectrum pulled out into its own window, the main window drew the **whole** strip
of controls under the frequency even though the column beside it already held
every one of them — the strip was measuring "is there a column?" from the width
left *after* the column had taken its share, so it concluded there was no room
for one. Now the answer is decided once, where the column is drawn, and the strip
reads it, so Classic is Classic: readout and meter over the picture, every
control in the column. The second is that the ▣ was only where a gear is, so a
panel like SSTV — which has no gear — had no route to it; it is now in every
panel header that has a chip row, and in the floating and detached panel windows.

**Done.**
- Item: the Classic half of handover step 2, and the operator's report that the
  ▣ "is not in every panel because if a panel has no gear it does not show".
  Commit **`7574093c`** on `local/windows`.
- `layout.rs`: `set_classic_column` / `classic_column` — the answer published
  once, plus the guard (`ViewportScope`) carrying it so a child window publishes
  `false` for its own draw and puts the main window's back on drop.
- `frame.rs`: `classic_here` computed from the window's **whole** width where the
  column is decided, published there, and used for the column and rail draws.
  `classic_column_this_frame(ctx)` now reads the register instead of re-deriving
  it from the width left over; the console window (a `WindowShape::Desk` scope)
  gets `false`, so it keeps every control.
- `panels::module_windows_chip` — the ▣ for a panel's own header, drawn in SSTV,
  the FT8 family, JS8, AtCHAT, packet and ADS-B, and in the chips rows of the
  floating panel and the detached panel window. Packet's header has no `self`, so
  it reports the click through a `windows_list` out-parameter like its `detach`.
- Numbers: `cargo check --workspace --all-targets` **0 errors**; `cargo test
  --release -p sdroxide-ui --lib` **1283 passed, 0 failed, 10 ignored**;
  `cargo check --release --target wasm32-unknown-unknown -p sdroxide-ui` clean
  (**the panel chips are native-only and cfg'd at every call site** — the first
  wasm run caught three call sites where they were not); `rustfmt --check` clean
  on every file touched.
- Test: `a_classic_window_publishes_one_answer_for_the_strip` replaces the one
  that pinned the old two-argument answer — frames at 1600 (column drawn,
  published true), at 560 (no room, false), a `WindowShape::Desk` scope inside a
  true answer (the console keeps its controls), and Classic off.

**Not done / blocked.**
- The panels whose header has **no chip row at all** — WSPR, PI4, NAVTEX, FSQ,
  JTTY, CW, RADE, RF Paint, the text modems — have nowhere for the ▣ to go
  without giving each one a row of chips, which is a height change in F4NRM's
  panels rather than a chip. `Ctrl+Alt+W` reaches the list from those, and the
  strip carries the ▣ whenever a module is out. **This wants the operator's
  word**: a row on those panels, or the keys for them.
- Step 5 (deleting the old chips) is still untouched, and the list's filter box /
  arrow-key navigation with it.

**Next.** Either the sweep above or step 5, whichever the operator picks.



## 2026-10-26 (windows: the two keys, and workspaces on the number row)

**In plain words.** The windows work now has keys, which is what makes it usable
on a keyboard-only desktop. `Ctrl+Alt+W` opens the list of all your windows
anywhere — including when every window is docked, which is exactly when the ▣
chip is not on the strip — and `Ctrl+Alt+F` lifts the operating panel out to
float over the main window or puts it back. `Ctrl+Alt+1` to nine and `Ctrl+Alt+0`
call back the arrangements you saved in Settings, so "my listening layout" is one
press away; an empty slot puts a line on the banner instead of doing nothing
quietly. Nothing is pushed; the build is installed for you to try.

**Done.**
- Item: step 4 of [`WINDOWS-HANDOVER.md`](WINDOWS-HANDOVER.md). Commits
  **`1dd73489`** (the keys, and the list as a window) and **`a8a2a842`**
  (workspaces), on `local/windows`.
- `frame.rs`: both chords are read where F1 is, so a text field never eats them,
  and from the **events** rather than the frame's modifier state — a chord
  pressed and released inside one frame still arrives, and `repeat: false` means
  a held key cannot flicker a window. `workspace_slot` maps 1…9 then 0 as a pure
  function, so the numbering is a test.
- The list moved out of the SYS menu into a window of its own
  (`module_windows_window`): a popup is anchored to the chip that opened it and
  the key has no chip to hang one on. It reuses `module_windows_list` unchanged,
  is salted per radio like every other fixed id, and closes with its own close
  box. The ▣ chip and `Ctrl+Alt+W` open the same window.
- Applying a workspace docks the floating panel: `Workspace` stores module
  windows, not the floating panel, so the panel comes back to the pane with the
  arrangement that asked for it docked rather than floating over it as the one
  module the workspace did not mention.
- An empty workspace slot raises a notice naming it ("No workspace 3 saved — save
  one in Settings › UI › Workspaces…"), because a key that ignores a press is
  the one thing a control must never do.
- Numbers: `cargo check --workspace --all-targets` **0 errors** (the same two
  pre-existing warnings, untouched files); `cargo test --release -p sdroxide-ui
  --lib` **1283 passed, 0 failed, 10 ignored**; wasm32 check clean;
  `rustfmt --check` clean on every file touched. The key test drives real
  press/release events through whole frames — which is how it found that a press
  with no release arrives as a repeat, and that a synthetic keyboard has to send
  both.

- Nothing pushed; built and installed for him to click through
  (`~/.cargo/bin/sdroxide`, 14:30, `--version` says `SDR Oxide Brown 2.1.0_brown`,
  from `a8a2a842`).
- The design's remaining half of step 4: a **filter box and arrow-key
  navigation** inside the list (`D`/`F`/`S` per row). The rows are pointer-driven
  for now; the list is reachable and complete without it, and the operator's own
  complaint was about chips, not about typing into a list.
- Step **5** — deleting the old entry points (the float/dock chips, the panel
  header's window chip, the radio strip's WINDOWS menu) — is untouched: with the
  keys in place it is now safe to do, but it is a separate item and it touches
  the chips F4NRM's own PRs use.
- The detached windows still cannot be photographed headlessly; the keys were
  verified through frames, the windows still want the bench.

**Next.** Step 5, or the step-2 leftover in Classic (a detached panadapter should
draw just the picture, without its DISP chip) — the second is one small change
and wants the operator's eye on whether the window then looks bare.



## 2026-10-26 (windows: one icon, one list)

**In plain words.** There is now a single place that answers "which of my windows
are out, and how do I get one back": a small ▣ beside the gear in the System
menu opens a list of every window the program can pull out — the spectrum, the
second spectrum, the operating panel, the RX control, the band selector, the
rotator — with the three places each can be (here, floating over this window, or
its own window), the one it is in lit, and a press to move it. Because the list
is a list, a window you left on another workspace is still a row you can bring
home, which is the whole point on a tiling window manager that draws no close
box. Floating is offered only for the operating panel, because that is the only
thing with a second place to put it; the other rows leave that slot empty.

**Done.**
- Item: step 3 of [`WINDOWS-HANDOVER.md`](WINDOWS-HANDOVER.md) — "one icon, one
  list". Commit **`aa716e97`** on `local/windows`.
- `crates/sdroxide-ui/src/app/mod.rs`: `ModuleWindow { Docked, Floating,
  OwnWindow }` with `module_window_state` / `set_module_window_state` — the one
  place the state is read and written, so the list and (later) the keys cannot
  drift. Floating is refused for anything but the panel, and setting one state
  clears the others (a panel cannot be in a window and floating at once).
- `crates/sdroxide-ui/src/app/top_bar.rs`: the `▣` chip in the System row,
  immediately before the gear, drawing `module_windows_list` — a row per module
  (name from `DetachableModule::window_name`), three icon chips right-aligned and
  reversed so the row reads *docked → floating → its own window* on every tier,
  the current state lit, an **empty slot** where a module cannot go, and DOCK ALL
  / UNDOCK ALL under them. Drawn at the end of both menus that carry the chip
  (the System box and the phone's ☰).
- `crates/sdroxide-ui/src/chrome.rs`: `chip_window_icon_lit` (the icon chip with
  a lit state), `window_icon_width` (so an empty slot is exactly a chip's width),
  and disabled icon chips no longer look pressable.
- `crates/sdroxide-types/src/ui.rs`: `DetachableModule::window_name`, moved up
  from `multi.rs` so the list and the radio strip's WINDOWS menu name a module
  the same way (SP1 / SP2 / RX control).
- **The chip costs the strip nothing**: it is on the row only while a module is
  out of this window (or the list is open). Measured, not assumed — as a
  permanent ninth chip it took the widest 1600 pt arrangement from two rows to
  three, which `the_cessb_rail_costs_the_desktop_strip_no_row` caught, and a row
  costs the waterfall a module height (issue #294's rule). The first window goes
  out from Settings › UI › Detachable or a panel's own chip.
- Numbers: `cargo check --workspace --all-targets` **0 errors** (the same two
  pre-existing warnings, in files this does not touch); `cargo test --release -p
  sdroxide-ui --lib` **1282 passed, 0 failed, 10 ignored** (two new tests:
  `the_module_list_moves_a_module_and_reads_it_back` and the ▣-beside-the-gear
  half of `the_simple_interface_drops_the_advanced_system_chips`);
  `cargo check --release --target wasm32-unknown-unknown -p sdroxide-ui` clean;
  `rustfmt --check` clean on every file touched.
- **Looked at rather than assumed**: `module_windows_preview` (new ignored tool,
  `MODLIST_PREVIEW` for the path) renders the list with the spectrum and band
  selector out and the panel floating: `/tmp/opencode/modlist4.png`. Two things
  it caught that reading would not: the icons were invisible until the context
  was dressed with the theme, and the rows read in the wrong order until the
  chips were drawn right-to-left in reverse.

**Not done / blocked.**
- The old entry points are still there — the panel window's float/dock chips, the
  band popup's, the radio strip's WINDOWS menu, the Settings page. Deleting them
  is step **5**; the list has to be proven first, and deleting the panel chips
  without a key (step 4) would leave a floating panel with one way home.
- The keys (`Ctrl+Alt+W` / `Ctrl+Alt+F`) and the list's filter box are step **4**.
  Nothing here shows a key it does not have — a control must not point at a
  binding that does not exist yet.
- Still not verifiable headlessly: the detached windows themselves.

**Next.** Step 4: `Ctrl+Alt+W` opens the list anywhere (which is what makes it
reachable with every window docked, and the real recovery on a keyboard-only
WM), `Ctrl+Alt+F` toggles the panel's float, and workspaces get `Ctrl+Alt+1…9`.



## 2026-10-26 (windows: a narrow panel window keeps its own pane tabs)

**In plain words.** A panel pulled out into its own window, and made narrow —
which is what his tiling window manager does — now shows one panel at a time
with its own DECODES / QSO chips at the top, instead of squeezing two columns
in and cutting the right-hand one off at the window's edge. The chips belong to
that window, so it can sit on the decode list while the main window shows the
waterfall, and tapping REPLY in it moves *it* to the QSO pane. Widening it past
600 points brings the two columns back. The floating panel behaves the same way
and stops at its own 540-point width instead of growing to fit two columns.

**Done.**
- Item: step 2 of [`WINDOWS-HANDOVER.md`](WINDOWS-HANDOVER.md) — the half that
  the 560 pt measurement from the previous entry decided. Commit **`3a5b1d9e`**
  on `local/windows`.
- `layout.rs`: `WindowShape::Panes` — the one shape that lifts
  `tier_for_window`'s Tablet floor, because the window draws the tab row that
  makes one pane at a time reachable. Under 600 pt it is the phone design; from
  600 to 1400 a tablet with both columns; over that a desktop.
- `panels/mod.rs`: `panel_tabs(mode, waterfall)` and
  `digi_tabs(ui, mode, waterfall)`. A panel window passes `false`: its picture
  is the main window's, and a waterfall tab in it would select the two-column
  layout and undo the row. The main window passes `true` exactly as before.
  `every_mode_has_at_least_one_pane_and_a_waterfall_after_it` now checks both
  rows for every mode.
- `frame.rs`: the panel window and the floating panel draw `digi_tabs` when the
  published tier is Phone, above the panel body. `SdroxideApp::panel_window_pane`
  is the window's own pane, **swapped in** around the draw (the AUX SP pattern),
  so the main window and the window each keep their own, and a write from inside
  (answering a call switches to QSO) lands where the operator is looking. It is
  clamped to a real pane before the swap: a stale index from another mode, or
  the main window's waterfall tab, cannot ask a panel window for a picture.
- Numbers: `cargo check --workspace --all-targets` — **0 errors**, the same two
  pre-existing warnings in files this does not touch. `cargo test --release -p
  sdroxide-ui --lib` **1281 passed, 0 failed, 9 ignored** (one new test,
  `a_panel_window_that_draws_its_own_tabs_may_be_one_pane_at_a_time`).
  `cargo check --release --target wasm32-unknown-unknown -p sdroxide-ui` clean.
  `rustfmt --check` clean on every file touched.
- **Looked at rather than assumed** (`floating_panel_preview`):
  `/tmp/opencode/float-560-after.png` — one pane, DECODES / QSO chips, nothing
  past the window edge; `/tmp/opencode/float-wide-after.png` — the same one-pane
  panel at its 540 pt default in a 1600 pt window. Before the change, at 560 pt
  the two columns drew the QSO chips off the edge.

**Not done / blocked.**
- Still the detached OS window itself: the harness runs embedded viewports, so
  `--screenshot` cannot photograph it. This wants the operator's session, and
  the preview above is the floating panel, which draws the same body.
- One line of step 2 remains: **"in Classic they draw just the picture"** — the
  detached panadapter window still carries its DISP layer chip. It is the
  column's job in Classic and the chip should go, but whether the window looks
  bare without it is a judgement to make with his screen in front of us, so it
  was left alone rather than guessed.
- Nothing pushed; built and installed for him to click through
  (`~/.cargo/bin/sdroxide`, 14:14, `--version` says `SDR Oxide Brown 2.1.0_brown`).

**Next.** Handover step 3: the `▣` list that replaces the float/dock chips and
the radiotab windows button, with the three-state rows and the "bring it back"
row that makes a lost window recoverable.

## 2026-10-26 (windows: a child window answers for itself)

**In plain words.** A window that has been pulled out of the main one now lays
itself out from its own size instead of pretending it is still the big window.
That was the fault behind the photograph of chips running off the right edge: a
520 pt panel was drawing the arrangement meant for a 1920 pt screen. The radio
whose window it is also stops depending on which tab happened to be drawn last,
which mattered once there were two radios on screen. Nothing of this is on
`main` — it is on `local/windows`, where the operator asked for the window work
to be built and tested before it goes anywhere.

**Done.**
- Item: step 1 of [`WINDOWS-HANDOVER.md`](WINDOWS-HANDOVER.md) — "scope the
  state per viewport". Commit **`196827a3`** on `local/windows`.
- `crates/sdroxide-ui/src/layout.rs`: `ViewportScope` takes the frame's five
  published answers (tier, tight, short-tablet, the band dock's room, the radio
  salt), publishes the child window's, and puts the main window's back on `Drop`
  (it holds a cloned `Context`, which is a handle, so `Drop` needs no argument).
  `WindowShape::{Own, Desk}`; `tier_for_window` decides a window's tier from its
  **width only**, floored at `Tablet`; `tight_for_tier`/`short_tablet_for_tier`
  are the tier-already-decided forms of the two existing rules.
- The band dock's room is a **register** now (`layout::set_band_dock_room` /
  `band_dock_room`) instead of a field on the app: it was the one answer that
  could not be scoped by a guard, and it was also the one that let a control
  window's band chip hide a column in the main window.
- `crates/sdroxide-ui/src/app/frame.rs`: `detached_viewport` takes a
  `ChildWindow` and publishes the guard inside the viewport, so every module,
  every tool window and the floating panel go through one place. The three
  hand-written `set_tier`/restore dances (console, band selector, rotator) are
  gone, replaced by `ChildWindow::desk`.
- Numbers: `cargo check --workspace --all-targets` — compiles, **two warnings,
  both in files this does not touch** (`spectrum.rs:146` unused `mut`,
  `sstv_composer.rs:411` unused `release`; they are at `67a71118`). `cargo test
  --release -p sdroxide-ui --lib` **1280 passed, 0 failed, 9 ignored**.
  `cargo check --release --target wasm32-unknown-unknown -p sdroxide-ui` clean
  (280 warnings, as before). `rustfmt --check` clean on all four files touched.
- New tests: `a_child_window_is_measured_by_its_width_not_its_height`,
  `a_child_window_answers_for_itself_and_the_main_windows_answers_come_back`,
  `a_desk_window_stays_desktop_shaped_however_small_it_is_made`, and
  `a_detached_window_scopes_the_layout_answers_to_itself` (a source pin: no
  headless test can open a second OS window, so the one line that scopes the
  answers is read rather than run).
- **Looked at, not assumed**: new ignored tool `floating_panel_preview`
  (`cargo test -p sdroxide-ui --release --lib floating_panel_preview -- --ignored
  --nocapture`, `FLOAT_W`/`FLOAT_H`/`$FLOAT_PREVIEW`) renders the operating
  panel as a floating window. At 1600×1000 it is a legible 660 pt panel; at
  560×760 the QSO area's chips run past the panel's right edge.

**Not done / blocked.**
- The detached windows themselves **cannot be opened headlessly** — the
  screenshot harness runs embedded viewports, where `detached_viewport` draws
  nothing — so the guard, the policy and the call sites are tested and the
  windows themselves want the operator's Sway session. This is the handover's
  own caveat, not a new one.
- The 560 pt measurement above is a **real remaining fault**: a narrow panel
  window keeps its two columns because the phone arrangement (one pane at a
  time) needs the pane tab row, and that row is drawn by the main window. So
  step 2 of the handover — a child window is self-contained — is what fixes it,
  and the finding is written on `tier_for_window` so the next session starts
  from the picture rather than the guess.
- Nothing is pushed: `local/windows` is the branch the operator asked for. Built
  and installed for him to click through (`~/.cargo/bin/sdroxide`, 13:40,
  `--version` says `SDR Oxide Brown 2.1.0_brown`).

**Next.** Handover step 2: the detached panadapter and panel windows draw the
picture/panel and the way home, never a second strip — and the panel window
carries its own pane tabs when it is too narrow for two columns.

## 2026-10-25 (SSTV picture editor: the document on the wire, PR for F4NRM)

**In plain words.** The SSTV transmit side now has a real editor: a window over
the preset's picture with text you drag where you want it, in one of three
faces, at any size, optionally on a colour strip that can run across the picture
or down its side. Each of the five transmit slots keeps its own layout, the
radio's machine stores it, and every screen attached to that radio therefore
draws the same picture — including after a restart. A slot nobody has composed
still sends exactly what it sent before. It is built, installed and pushed as a
PR; nothing of it is on `main` because a panel of F4NRM's and the wire both are.

**Done.**
- Item: issue #237 (F4NRM) — drag and drop a picture, a face chooser, movable
  text, flexible banner placement, "the current images are not QSO-compliant",
  and a reply flow (his picture, your call over it).
- `crates/sdroxide-types/src/composer.rs` (new): `Composer`, `TextLayer`,
  `Strip`, `Face`; `ImageSlotInfo.composer` and `Command::ImageSetComposer`
  appended last. `PROTO_VERSION` 248 → **249**, `VERSION_BYTE` `0x44` → `0x45`,
  register line written.
- `sdroxide-radio`: `ImagePresetStore` keeps a document per slot, persists it in
  `sstv_composers.json` (via `sdroxide-config`), and does not forget it when the
  picture is replaced or cleared.
- `sdroxide-ui/src/sstv_composer.rs` (new): the renderer (fractions of the frame,
  quarter-turn text for a vertical line) and the claimed-working-copy hand-off
  (`shown`/`claim`/`pending`/`release`). `crates/sdroxide-ui/src/sstv.rs` grew a
  `TextSink` so one glyph loop paints both the picture and a rotated layer.
- `crates/sdroxide-ui/src/app/panels/sstv.rs`: the "Editor…" chip and the
  window; banner placement (top/bottom) and a text size on top of the strip
  height; a dropped picture (X11/Windows/macOS) and a pasted path (Wayland has
  no drag and drop at all).
- Commands and numbers: `cargo check --workspace --all-targets` silent (the one
  `classic.rs` unused-import warning predates this); `cargo test --release -p
  sdroxide-types` 796 passed; `-p sdroxide-proto` 17; `-p sdroxide-ui --lib
  sstv` 35; `-p sdroxide-radio --lib image_store` 11; `cargo check --release
  --target wasm32-unknown-unknown -p sdroxide-ui` silent; `rustfmt --check`
  clean on every file touched.
- Looked at rather than assumed: `/tmp/opencode/composer-sample.png` (renderer
  output), `/tmp/opencode/editor2.png` (the window at QSO size), and
  `/tmp/opencode/wire-doc.png` — a document written into a scratch
  `sstv_composers.json` the way the engine writes it, drawn by both the editor
  and the panel's transmit preview, which is the proof that the engine's
  document reaches the client and is composed.
- Commits on `fork/sstv-composer-wire` (off `main` at 70867db4): `58984bc2`
  (wire, store, engine), `aaa3fa6a` (renderer, editor, panel) and `c329abcb`
  (queue and journal). **PR #240**, and a comment on #237 pointing at it.

**Not done / blocked.**
- The editor's own **write** path is unit-tested but was not clicked through
  end to end: the `--screenshot` harness's synthetic clicks do not land on that
  window's buttons, so pressing "＋ Text" and watching `sstv_composers.json`
  appear stays the operator's test. Everything under it (send once, do not
  repeat before the echo, release on switch, persist, keep through a clear) is
  tested.
- The document is per slot, not a station-wide overlay: a call that should be on
  *every* picture is still the banner's job, and copying a layout is one press
  per slot. If F4NRM wants a station overlay, it is a document on `DigiConfig`
  and another wire bump.
- Nothing merged: **his PR to merge, ours not to touch** (AGENTS).

**Next.** Whatever the queue holds next; this item waits on F4NRM's review.
## 2026-10-26 (#220 is F4NRM's, checked)

**In plain words.** A question came up about PR #220 (the SESSION QSOs panel),
and the answer is that it is entirely his work: both commits are his
(`b8cd5cf2`, `1fb4621e`), he merged it himself on 2026-10-25, and
`session_qsos.rs` has had no commit from us since. Recorded here so the question
does not have to be asked twice.


## 2026-10-26 (evening: the review round, and the two-hour cap)

**In plain words.** F4NRM reviewed everything that was waiting and merged five
PRs himself — the sensitivity restoration, the test repair, the receive-only
transmitter guard, the two auto roles and the fax CTR fix. Two more are approved
and waiting for a hand merge (the picture composer and the logbook fix). The
unattended FT8 run got a continuous role, a retry delay and — on the
maintainer's instruction — a hard **two-hour ceiling**, because four hours is how
contests get cheated. The picture composer grew the station overlay, the layout
library (with rename and duplicate) and a mosaic of up to nine pictures.

**Done.**
- Merged by F4NRM: **#242**, **#243**, **#245**, **#246**, **#255**.
- On `main` directly, wire-neutral: **#244** (satellite TUNE follows the active
  VFO), **#254** (the scan gives the voice gate's dwell back), the PSK air
  loopback tool, the `ctr <span>` screenshot action.
- Approved, closed, waiting for a **hand merge**: **#240** (composer, wire 254)
  and **#241** (logbook fix, wire-neutral).
- Open: **#253** (AUTO CONT, retries, deferral, `PROTO_VERSION` 255, and the
  two-hour cap from `0c017ff4`).
- Issues closed: #244, #254. Tracking: #248–#252.
- Numbers: gate silent throughout; `sdroxide-ui --lib` 1275, `sdroxide-types` 815,
  `sdroxide-radio image_store` 14, `sdroxide-radio --test rx_only_never_keys` 3,
  `sdroxide-dsp --test wefax_real_recording` 5. Installed 10:28, combined build
  `PROTO_VERSION` 256.
- Bench: the RSP1 works; a 2 W PSK31 transmission on 27.415 MHz was captured with
  a flat envelope and clean BPSK31 spectrum, but the tool's own audio recovery is
  wrong — proven with a numpy-synthesised capture, so the rig is exonerated.
  `rspcap` streams at 2 MS/s whatever rate is asked, and the rig keys from card 3.

**Not done / blocked.**
- The loopback's `split` is broken (mix/decimate/shift); the synthetic capture in
  the example's header is the test. Until it works, the mode table's PSK rows
  cannot be re-measured.
- #240 and #241 need the maintainer's hand merge (closed, approved, `MERGEABLE`).

**Next.** Fix `split`, re-run the loopback, then the slow-speed PSK rows and the
SWL side of the mode programme.


## 2026-10-26 (composer features, the continuous auto run, and two bug fixes)

**In plain words.** The picture editor grew the three things F4NRM asked for — a
station overlay drawn under every slot, a library of named layouts, and a mosaic
of up to nine received pictures — and the unattended FT8 run gained a continuous
role with a retry budget and a session limit. Three reports are fixed on `main`:
the satellite window's TUNE followed VFO A instead of the one selected (#244),
the scan's voice gate left the listen time raised when switched off (#254), and
the fax CTR anchor centred the carrier rather than the chart (#255, open).

**Done.**
- **PRs open**: #240 composer (overlay, layouts, mosaic — wire 254), #241 the
  logbook fix, #253 AUTO CONT (wire 255), #255 CTR centring.
- **Merged at the operator's word**: #242 (Auto Rx/Auto CQ), #243 (receive-only
  transmitter guard), #245 (sensitivity restored), #246 (four red tests repaired).
- **On `main` directly, wire-neutral**: #244 (`tune_commands`, test fails on the
  old code) and #254 (`voice_dwell`, four cases pinned).
- **Tracking issues** #248–#252; #252 is the central-logbook design and asks the
  operator two questions.
- Commands: gate silent throughout; `sdroxide-ui --lib` 1256→1275 by branch,
  `sdroxide-types` 808, `sdroxide-radio image_store` 14, wasm check silent.
  Installed 22:31, 23:04, 00:36, 01:01, 07:01, 07:19 — the last with wire 256.
- Looked at rather than assumed: the editor in slot and overlay modes, twelve
  layers in the list, and a four-picture mosaic with the overlay on and off
  (`docs/images/sstv/`).

**Not done / blocked.**
- #240 waits on his approval; #241 he asked to leave open; the durable logbook
  (#252) waits on the operator's two answers.
- The nine demonstrations in #251 are decisions and are tested as such, except
  the three that need a band with real callers.

**Next.** The SWL side of the mode programme: STANAG 4285's payload, then the
half-done list in `MODE-PROGRESS.md`.


## 2026-10-25 (STANAG 4285: what the capture says, and what is left)

**In plain words.** The coded payload still does not read, but the fog is gone:
four of the five things it could have been are now measured and excluded, one real
bug is fixed, and the next step is named. Our own 24 October capture of 13031.2
kHz carries **283 frames** — far more than the interleaver needs at any rate — so
"we need more material" is finished as an answer. The chains fill, decode ten to
fourteen chunks each, and produce random bits. The scrambler, the deinterleaver
and the rate list are now verified against an independent GPL implementation,
which leaves the demodulated symbols themselves as the suspect.

**Done.**
- `IqSource`-style groundwork is not touched; this is all in
  `crates/sdroxide-dsp/src/stanag4285*.rs`.
- **A real bug fixed:** the QPSK coded chain was created *inside* a loop over
  `fec::CodedRate::BPSK` — the four BPSK rates — so `r == Bps1200` was never true
  and the QPSK branch never existed. A capture whose verdict is QPSK had nothing
  listening. Now built as an explicit list (`coded_chains`), twelve branches, and
  pinned by `every_coded_rate_has_a_branch`.
- **Instrumentation kept:** `S4285Receiver::chain_stats()` reports each branch's
  pushed soft values, decoded chunks, messages and SOM score, so "the chains
  never filled" is distinguishable from "they filled and the code failed". Two
  `#[ignore]`d probes use it on a capture passed in `SDROXIDE_S4285_SAMPLE`:
  `the_capture_chain_by_chain` (all four equaliser modes) and
  `the_capture_bit_stream_dump` (bit stream, SOM/EOM match, pair order, value
  shaping).

**Measured, on the 24 Oct capture (48 kHz stereo, averaged to mono).**

| | |
|---|---|
| frames | 283, locked, sync 0.90, references 0.971 / 0.994 / 0.988 |
| verdict | BPSK coded, i.e. one of 600/300/150/75 |
| chains | Bps600 fills and decodes 10 chunks (Bps300 4, Bps150 1) |
| decoded bits | 71 424, ones fraction 0.491, no positional bias at any period up to 32 |
| best SOM match | 22 of 32 as-is, 26 swapped pairs, 24 hard decisions, 22 tanh |

**Excluded against the GPL reference (`luwangg/signal-analysis`, GPL-3.0):**
- the scrambler — replayed their `s4285_gen_scramble.m` LFSR by hand, symbol for
  symbol, against ours: identical for the first 24 groups and the same taps;
- the deinterleaver — their `(9·i) & 31` output permutation, `increment·(31−i)`
  row lengths and `31·increment` fill rule are exactly our `IN_ORDER`, row
  lengths and fill, both in the `.hpp` and the `.m`;
- the rate list — see the bug above;
- soft-decision polarity, pair order and value shaping — all variants run, none
  frames a message.

**Then the reference was driven on the same audio, and it decodes.** A small
standalone C++ driver around its headers (no Octave here) is in
[`tools/s4285-reference/`](../../tools/s4285-reference/) with the commands that
built it. Its ITA2 reading of this capture is:

```
LTRS… AVUA FIG S CH  RYRYRYRYRYRYRYRY  SGSGSGSG  NNNN …
```

`RYRY` is the French Navy test tape. **So the capture is good, the payload is
readable, and the fault is ours.**

**And the two coding chains are not the same machine.** Feeding the reference's
own 128 soft decisions per frame (367 frames dumped from its equaliser) through
*our* deinterleaver and Viterbi and comparing bit for bit: 36.5 per cent agree
raw, 66.5 per cent at the best alignment — not the ~100 per cent that would mean
"same chain, different front end". Their metric is an XOR of saturated bytes
(`viterbi2.hpp`: `sym0 ^ 255`), a hard decision in disguise; ours is a soft
correlation. Both decode a clean signal; on a marginal one they diverge.

**Not done / blocked.** Two faults, in this order:
1. **The Viterbi/metric** — port the reference's exactly and A/B against its bit
   stream on its own soft decisions until the two agree; that is pure algorithm
   work with a ground truth already on disk (`ref_soft.bin`, `ref_bits_clean.txt`).
2. **The soft decisions** — ours come from a 11-tap DFE judged by *constellation
   concentration*, theirs from a 121-tap fractionally-spaced LMS DFE judged by
   whether the bits decode. On this recording the constellation metric prefers a
   filter that does not decode, which is this journal's own recurring trap: the
   instrument did not measure the thing.

**Next.** Do (1) first — it is bounded and has a pass/fail. Then (2): a longer
DFE trained on sync + references, swept against **decoded bits**, not
concentration.

**Then the equaliser was tried, and told us what it is not.** A sweep of the DFE
against decoded text (the probe `the_capture_equaliser_swept_against_text`, which
prints the text each shape produces) gave `RY` counts of 1, 0, 1, 2 and 3 for
filters from 11 to 81 taps with up to four passes — no better than the shortest,
and never a message. Carrying the weights between frames instead of restarting
them at zero every frame — which is what the reference does, and what ours did
not — moved nothing either, so that change was reverted rather than kept: an
untested improvement is churn.

What that leaves is a *structural* difference, not a tuning one: the reference
runs a 121-tap fractionally-spaced LMS filter on the 5-samples-per-symbol
baseband with four symbol taps, trained symbol by symbol through the frame, and
takes its soft decisions from a **log-likelihood ratio** quantised to 25..230.
Ours filters 11 taps, decides linearly on the real part, and sweeps the same
frame twice. Porting theirs is the remaining work — the header is 190 lines and
the ground truth (their bits, their soft decisions, and the text they produce) is
already on disk.

**And the coding chain was cleared against their decoder too.** The interleaver's
wiring is the one thing the annex leaves ambiguous, so this tree carries four;
`which_interleaver_wiring_matches_the_reference` runs the reference's own soft
decisions through each and asks which reproduces the bits that decoder produced:

| wiring | agreement |
|---|---|
| **announce** (the one we run) | **66.5 %** / 67.2 % |
| swapped | 54.0 % / 54.5 % |
| rows-flipped | 54.2 % / 55.3 % |
| swapped+flipped | 55.8 % / 54.3 % |

Three of the four sit at chance and ours is clearly ahead, so the wiring is right
and the deinterleaver's geometry is confirmed a second time. Also verified on the
capture: the chain is fed **one frame per frame** (feed spacing exactly 1280
samples, 559 feeds over 556 frames of audio), it is **never restarted** (resets
zero), and the material is five times the interleaver's fill need.

So every part of the receiver except the demodulated soft decisions has now been
checked against an independent implementation that decodes this transmission. The
remaining work is one thing: **the front end**. The reference filters the frame
with a 121-tap fractionally-spaced LMS equaliser plus four symbol taps, trained
symbol by symbol through the sync and the reference blocks, and takes
log-likelihood soft decisions from a running noise estimate — where this tree
filters eleven taps, applies a line fitted through the three reference blocks, and
decides on the real part.

**And then the text layer turned out to be throwing good messages away.** With the
reference's bits in hand — bits that plainly contain a French Navy tape — the
mode's own reader returned **nothing**:

```
message_text(reference bits) -> None
     …while the same bits read by hand are:
     AVQVA C  RYRYRYRYRYRYRYRY  SGSGSGSGSGSGSGSG  NNNN  ZNR UUUUU  ZUI TESTING
```

Three faults, each measured, each now fixed and pinned by `a_test_tape_reads_as_text`:

1. **The score was English prose.** `text_score` rewards E T A O I N S H R D U L
   and spaces; `RYRYRYRYRYRYRYRY` scored 0.23 against a 0.45 gate. A station
   testing a link sends a tape, not prose — so the mode's own test transmission
   was rejected by its own text filter. A `tape_score` now rewards a repeating
   pattern, measured in characters (a sixteen-character diddle is unmistakable;
   a *share* of the reading is not, and the first attempt scored this case at
   zero).
2. **The reader gave up at the first missing stop bit** instead of
   resynchronising, which the code's own doc claimed it did. One bad bit ended
   the reading and everything after it.
3. **The synchronous reading was gated on the framed one finding nothing**, so a
   wrong 5N1.5 alignment could block the right reading. Both are computed now and
   the framed one is preferred when it clears the gate, because the coded payload
   *is* a teleprinter stream.

With that fixed, the same reference bits read `RYRYRYRYRYRYRYRY`, `SGSGSG`,
`NNNN`, `ZNR UUUUU`, `ZUI TESTING`. **This is the first time this tree has read
the text of a real STANAG 4285 transmission** — not through our own chain, but
through the reference's bits feeding our message layer. The mode's remaining
fault is the soft decisions, exactly as (1) and (2) above say.



**In plain words.** Went through every open issue and discussion and closed the
ones that were finished — either because the answer had already concluded them or
because a merged fix had. Two issues and nine discussions went; six issues and
four discussions stay, each for a named reason, all written down below so nobody
has to guess later.

**Closed as fixed.** #216 (squelch hold-open) — merged as #218, told the reporter
what to look for and that it lands in the next pre-release, invited a reopen if
the build does not do it.

**Closed as answered.** #162 (NAVTEX 518 vs 516.6) — the preset already applies
the 1.7 kHz shift, DK8JV confirmed 518 decodes with it, and the remainder in that
report is a reference error at the reporter's end; the closer names the
calibration setting and the one condition that would make it ours again.
Discussion #92 (is PR #45 still in) — yes, answered. #79 (DAB frequency after an
update) — the reporter found it: a rescan. #144 (how to get an unmerged PR into
main) — the workflow answer stands, and Mark's two side questions were answered.
#169 (Brick2 for Jarvis) — the PR he asked for is merged (#170); #132 and #133
stay open for the missing radio. #125 (the 2.14 changelog) — that version shipped.
#56 (2.0.8 ready to cut) — already said "closing this out" in the thread months
of releases ago, and never pressed the button. #76 (an identity note). #200 (a
thank-you). #107 (AppImage idea) — duplicate of issue #105, which is where the
answer will land.

**Left open, and why.** #132/#133 Brick2 — code merged, needs the radio.
#153 Pluto — waiting on the diagnostic report that was asked for and not sent.
#146 Listen/TX controls — F4NRM deferred it himself ("too many security risks for
now"); it is his call, not a stale item. #91 WEEFAX — his, and it needs a ten-page
test. #105 AppImage — planned for a stable release. Discussions #165 (STANAG
recordings — waiting on material), #73 (the rules) and #2 (welcome) are standing
posts, and **#168 is now stale on purpose**: it tells people to use GitHub
Discussions with IDEA/FEED/REQ in the subject, while the group has moved to
groups.google.com/g/sdroxide. That one needs an update, not a close, and it is a
public announcement — the operator's to rewrite.

**Correction, same day, the operator's word.** #73 (the working rules) and #168
(the etiquette post) are **closed as well**, so the count below is nine plus two:
#73's rules are kept in `AGENTS.md`, which is the copy every session actually
reads, and #168's etiquette moved with the discussions to
groups.google.com/g/sdroxide. Both were listed above as "standing" and the
operator decided otherwise.

**Measured.** `gh issue list --state open` → 6; discussions closed by this work →
9; `cargo check` and the tests untouched by it. Numbers in `AGENTS.md` and
`TODO.md` re-measured from the queue (`AGENTS.md` said five issues and "no open
PRs", which was wrong twice: six issues, and #226 open).



**In plain words.** An Airspy HF+ listener reported that the waterfall at 768 kHz
shows the filter's roll-off at both edges, that zooming in and scrolling right
walks into it, and that the automatic contrast sometimes sits too high. The
picture was drawn from the radio's sample rate, and a decimation filter rolls
off inside that rate — about 660 of the 768 kHz is real. The display now uses
the usable part only, and the fit can no longer see the skirt. Built and tested,
but not looked at on an HF+, and not pushed: it changes the protocol, so it is
waiting as a pull request.

**Done.** Branch `fork/hfplus-usable-span`, commit `e91dfb9c`.
`IqSource::usable_frac()` (default 1.0) with the Airspy HF+ at 0.86 — 660/768,
Airspy's own figure and the listener's measurement, taken as constant across the
rates because it is the same filter scaled, and said so. `RadioState::usable_hz`
at the tail (`PROTO_VERSION` 246 → 247, `VERSION_BYTE` 0x42 → 0x43, register
line). `usable_span()` in the engine: rate × fraction, widened back to the whole
rate at decimation ≥ 2 because the engine's decimator is the sharper filter —
narrowing there would show less band than exists. UI: `dev_span()` feeds the
panadapter window, the zoom clamp, the pan bound and the fit key; the analyzer
keeps the raw rate so decoders and the skimmer still see everything.

Measured: `usable_span` (768 000 × 0.86 = 660 480, decimated keeps the whole
rate, 0.0 means no opinion, a fraction above one cannot widen the radio);
`display_span` (three cases); the `ServerMsg::State` postcard round-trip now
carries a real `usable_hz` rather than only its default. `cargo check
--workspace --all-targets` silent apart from the UI's pre-existing unused
import; `sdroxide-types` 17, `sdroxide-proto` 793, `sdroxide-ui` 1245,
`sdroxide-radio --lib` 203 — all green.

**Not done / blocked.** Not seen on an Airspy HF+ here: the arithmetic is
measured, the picture is not, and the fit half of the report needs Graeme's
answer about whether it still misbehaves when the skirt is out of view. Pushing
and opening the PR are the operator's.

**Next.** Answer the group thread with what changed, and take the next item in
`TODO.md`.



**In plain words.** We stop pressing merge on F4NRM's pull requests. He merges
his own, always, and we leave the button alone even when the change is green,
sensible and he is away. Nothing in the program changed; what changed is whose
hand is allowed on his work.

**Done.** `AGENTS.md`, "Pull requests to this fork": the rule that F4NRM's PRs
are "approved and merged without waiting (`gh pr merge <n> --merge --admin`)"
is replaced, on the operator's instruction of 2026-10-25: *"we will stop merging
his PR's as they were on auto merge because I trust his work blindly."* The
trust is the reason rather than despite it — a merge pressed by us puts his work
on `main` on our timing and under our hand, with no diff of our own for him to
read.

**Why it was worth writing down.** The audit the same day found three merges of
his PRs made by us (`#212`, `#213`, `#214`, all `mergedBy=madmedicnl`) and one
place where we had *edited* his code before merging it: commit `aea09ccc`, inside
his `#214` aircraft branch, adding a missing `#[test]` to a test that had never
run and deleting a helper nothing called — two lines, correct, and still his
branch. Our own PRs were all merged by him (`#218`, `#219`, `#222`, `#223`), and
`#226` is untouched and open. Nothing we wrote that day touched his modes, his
panels or the wire: fourteen commits, twelve files, all ours.

**Not done / blocked.** Nothing — it is a rule, not a build.

**Next.** The next time one of his PRs looks wrong, say it in a PR comment and
change nothing. The modes programme continues after ARDOP.



**In plain words.** The AIRCRAFT window now has CSV and JSON buttons that save the
tracked aircraft as a table: who they are, what they fly, where from and to, when they
were first and last heard, and whether each fact came from the radio, the online
lookup or both. If the station keeps its day of history, the file is that day;
otherwise it is what the program holds now, and a line says which you got. It never
holds a position or a photo address, and a callsign that a spreadsheet would run as a
formula is defused. Not yet opened in a real spreadsheet or seen on a real screen.
This was the last of the night's four tasks.

**Done.** Branch `claude/aircraft-export`. History crate: an `export` module (rows from
the history, CSV with quoting and formula guard, JSON, ISO UTC, `Format`). Server:
`GET /aircraft-export?format=csv|json` in the history module (sign-in, then the
history switch, then the format; downloaded as `aircraft-YYYYMMDD.ext`; added to
`is_root_route`). Client: a pure `aircraft_export` module (rows made from the live
cards and the relay's answers, the answer check that refuses a web page where a table
was asked for, the store with a 30 s wait and the fall-back to a local file), the
plumbing in the I/O code, two buttons and a note in the AIRCRAFT window. Tests:
history crate 42 passed (17 new), server lib 62 passed (5 new) and the station tests
`aircraft_history` 5 and `aircraft_history_default` 2 passed (3 new), `sdroxide-ui`
1173 passed (26 new, including four whole-app frame tests: both buttons drawn, a press
starts an export, the file made with no station, the relay's cards in it). Mutations
over the three layers; the survivors (heard-but-bare radio flag, a bare Ok answer
becoming a row, the click wiring, the relay's cards reaching the file) each got a test
and were then killed. One mutation, dropping the route from `is_root_route`, cannot be
told apart by any test: a path under a proxy prefix is reduced to its last segment by
the file rule anyway, so the entry is for the rule's sake. wasm check clean, rustfmt
clean on the touched files.

**How the server half was run.** As for the alerts: a throwaway `build.rs` for
`sdroxide-rade` and `--unresolved-symbols=ignore-all` on the server's test binaries;
restored with `git checkout` before the commit, in no commit.

**Not done / blocked.** Not seen on a real screen, not opened in a real spreadsheet.
The file dialog on save is the existing one (`download::save`) and was not exercised
in a test. The workspace check is CI's.

**Next.** Nothing queued; the night's four tasks are done and open as PRs.
## 2026-10-25 (aircraft alerts)

**In plain words.** You can now tell the AIRCRAFT window which callsigns or
registrations you care about. When one of them comes on the air — or comes back after
ten quiet minutes — the window shows a line, the AIRCRAFT chip gets a dot and a count,
and the Watch view lists the alerts with their age. The list lives on the station, so
the program and the browser share it. There is no sound and no mail. It has not been
looked at on a real screen. Next: the fleet export.

**Done.** Branch `claude/aircraft-alerts`. A `watch` module in the history crate (the
list, spelling, matching, the edit and the reply); a `watchlist` module in the server
(`GET`/`POST /aircraft-watch`, the file `aircraft-watch.json` written atomically, the
sign-in, and the route added to `is_root_route`); client: a pure `aircraft_watch`
module (the tracker, the store and the fetch gate), the POST path in the I/O code for
native and browser, the Watch view, the banner and the chip count. Tests: history crate
41 passed (16 new); server lib 67 passed (10 new, one of them the proxy-prefix test) and the
integration test `aircraft_watch` 3 passed; `sdroxide-ui` 1181 passed (34 new,
including two whole-app frame tests). 20 mutations over the three layers, each killed
by a test. wasm check clean, rustfmt clean on the touched files.

**How the server half was run.** `sdroxide-rade` cannot build here (opus 403), so for
the session I used a throwaway `build.rs` for it with hand-written bindings, plus
`-C link-arg=-Wl,--unresolved-symbols=ignore-all` on the server's own test binaries
(they never call RADE). It was restored with `git checkout` before every commit and is
in no commit. The whole-workspace check is still CI's.

**Not done / blocked.** Not seen on a real screen; the chip label *AIRCRAFT ●2* may be
tight in its 112 px slot. Alerts are memory only. The browser's POST (`fetch`) compiles
for wasm but was not run in a browser.

**Next.** The fleet export.
## 2026-10-25 (aircraft stats)

**In plain words.** The AIRCRAFT window has a new *Stats* button: five small tables
of what the sky is made of, with the radio's own counts first (the category an
aircraft reported, and its airline from the start of its callsign) and the relay's
models, operators and routes as a complement, each with its green or blue badge. It
counts what is already known and asks nothing unless the view is open. It has not
been looked at on a real screen. Next: alerts for chosen aircraft.

**Done.** Branch `claude/aircraft-stats`. History list: each line now also carries
category, type, operator and the route's two airport codes (optional, so an older list
still reads). UI: a pure `aircraft_stats` module (counting, sorting, merging live and
history facts, the airline code, the fetch gate) and a Stats view. `cargo test -p
sdroxide-ui`: 1177 passed (30 new); history crate 31 passed. Ten mutations; two
survived the first round (history winning the merge, a non-OK answer), tests added,
both then failed. The render test caught a long operator name widening the table; the
label is now cut with the whole of it in the hover. 4000 aircraft are counted in well
under 250 ms. wasm check clean, `rustfmt --check` clean.

**Not done / blocked.** The server was not compiled here (opus 403), but its list
route is unchanged: it serves the history crate's `Summary`, which grew optional
fields. Not seen on a screen; routes only count aircraft the relay has answered.

**Next.** Alerts for chosen aircraft.
## 2026-10-25 (aircraft history compression)

**In plain words.** The station's aircraft history file no longer grows without
limit on disk: past 5 MB it is saved compressed, under the same name, and read back
either way, so a file from before still opens. It stays off unless the operator turns
it on, and a crash while saving still leaves the old file. The compression itself is
tested here; the few lines that plug it into the server could not be compiled in this
sandbox, so CI has to confirm them. Next: aircraft statistics.

**Done.** Branch `claude/aircraft-history-compression`. A `gz` module in the history
crate (feature `gzip`, on only for the server): `encode(text, above)` gives gzip over
the threshold and plain text at or under it, `decode` tells them apart by the gzip
magic. Server: reads through `decode`, writes through `encode` on the flush thread
with `write_atomic_bytes`; the old `write_atomic` is kept for the existing tests.
`cargo test -p sdroxide-aircraft-history`: 33 passed (8 new). Five mutations (`<=`→`<`,
never compress, ignore the magic, no compression, swallow a bad stream) each failed
their test. wasm check clean, `rustfmt --check` clean on the touched files.

**Not done / blocked.** `cargo check -p sdroxide-server` and its tests need the opus
download (403 here): five new server tests (disk round trip, plain file from before,
damaged file, failed write, one threshold) are written but unrun. CI to cover.

**Next.** Aircraft statistics.

## 2026-10-25 (aircraft share)

**In plain words.** The aircraft card in the Cards list now has a *Copy summary*
button: it puts what is known about the aircraft on the clipboard as text, each line
saying whether the radio or the online lookup gave it, with the photo credit and
link. The position and the last message are left out on purpose. Not yet looked at
on a real screen or pasted into a real app.

The night's brief listed four PRs; the first three (online card, hover card,
search and filters) were already merged (#205, #206, #207), so only this one was
left to do. Branch `claude/aircraft-share`, a pure module `aircraft_share.rs` and one
button in `aircraft.rs`. Tests written with the code, then four mutations (position
allowed, missing lines exported, notes appended, thumbnail address exported): each
failed exactly the test meant for it.

- `cargo test -p sdroxide-ui`: 1147 passed, 0 failed, 8 ignored (8 new).
- wasm `cargo check --release`: clean for this module; `cargo check -p sdroxide-ui
  --all-targets`: clean; `rustfmt --check` clean on the new file and on `aircraft.rs`.
- Not run: `cargo check --workspace --all-targets` (the radio engine's opus fetch is
  refused by the sandbox proxy). The change touches only `sdroxide-ui`.
- Open: the real clipboard write (`copy_text`) is not exercised by a test, only that
  the card calls it once, through the pure function.

## 2026-10-25 (aircraft history) — the station remembers each aircraft for a day

**Summary.** The station now keeps a day of what its radios heard of every
aircraft: where it was, which callsigns and registrations it used, and the card the
relay gave for it. A new **Show history** button in the pinned card, and in the Cards
list, draws that aircraft's trail on the map — fainter the older it is — and adds
lines to the card saying when it was first and last heard and how high it went. It
is **off by default**: the station's operator switches it on with
`SDROXIDE_AIRCRAFT_HISTORY=1`. It asks nobody
for anything and records nobody who looks. It has not been seen on a real screen or
with a real station yet.

**Done.** Branch `claude/aircraft-history-server`. No wire change
(`PROTO_VERSION` untouched: it is plain JSON on two new routes like
`/aircraft-info`), no change to the relay's logic.
- A new pure crate `sdroxide-aircraft-history` (the model: which positions are kept,
  retention, the ceilings, the file's text), the server's `history.rs` (feeds it from
  the ADS-B and VDL2 events of every radio, serves `/aircraft-history` and
  `/aircraft-history/{icao}`, writes the file atomically at most once a minute and
  only when something changed), and the client's `aircraft_history.rs` (when to ask,
  the trail, the lines).
- The relay is touched by one line in `aircraft.rs`, after its answer is made, which
  hands the answer to the history; a test counts that there is exactly one. The
  relay crate itself is unchanged.
- The first version kept 400 000 points and was on by default; see "Changed after review" below.
- Tests written first against neutral stubs: 22 failed in the crate, 19 on the
  server, 23 in the UI (plus the map trail's two); the ones that passed vacuously
  were given positive controls. The server's wiring tests that came after the code
  (routes, root names, the pump reporting its events) were shown failing by removing
  those lines and putting them back.
- Numbers: `cargo test -p sdroxide-aircraft-history` 25 passed; `cargo test -p
  sdroxide-ui` 1139 passed / 0 failed / 8 ignored (1113 before); wasm `cargo check`
  clean (one old `QsoInfo` warning in `solar3d`); `rustfmt --check` clean on every
  file touched.
- **Server tests were run with `sdroxide-radio` taken out of the server's
  dev-dependencies, temporarily**: that crate pulls `sdroxide-rade`, whose build
  downloads opus from GitHub, which this sandbox's proxy refuses (403). With it out:
  `--lib` 57 passed, `--test aircraft_history` 3 passed, `--test
  aircraft_history_default` 1 passed, `--test aircraft_info` 3 passed. The manifest and `Cargo.lock` were put back, and the lock differs from
  `main`'s only by the new crate and the two lines that depend on it. The server's
  other integration tests (`multi_radio`, `session`, …) need the radio crate and were
  not run; `cargo check --workspace --all-targets` could not run for the same reason.
  Not claimed.
- The disk of this sandbox filled during the work; the build's `incremental` and
  `examples` folders were deleted to go on. Nothing of the repository.

**Changed after review (same day).** Opt-in instead of opt-out: unset or anything but
1/true/yes/on means nothing is read, kept, written or served, and a second
integration binary (`aircraft_history_default`) checks that a station with the switch
unset answers 503 and keeps nothing. The ceilings came down from 600 / 400 000 to 300
points per aircraft and 50 000 in all; a test fills a history to its ceilings with
synthetic data and measures it: 3846 aircraft, 49 998 points, 4 589 995 bytes of JSON,
asserted under 6 MB. A write now cannot start while another is still running, and tests
show a failed write leaves the old file whole and that what is written was pruned first.

**Not done / blocked.** The trail and the card on a real screen; a real station with
a real day of traffic (the 4.6 MB is measured on synthetic data at the ceilings, not on
real traffic); the server's other integration tests and the workspace check, which
need the opus download.

**Next.** The mode work.

---

## 2026-10-25 (aircraft search) — one search box over the table, the Cards list and the map

**Summary.** There is now a search box over the ADS-B table, the AIRCRAFT window's
Cards list and the map, and it is the same box in all three. Type an address, a
callsign, a registration, a type or an operator and the three narrow together; on the
map the aircraft that do not match disappear, and the view no longer stretches to fit
them. The aircraft whose card is open stays on the map whatever you type. The search
asks the internet nothing, so a type or an operator is found only for aircraft you
have already looked up, and the box says how many it could not look at. It has not
been seen on a real screen yet.

**Done.** Branch `claude/aircraft-search-filters`, no wire change, no relay change,
nothing stored in the browser.
- A pure module `aircraft_search` (words that must all match, `icao:` `call:` `reg:`
  `type:` `op:` prefixes, the note about missing online data, which card to show when
  the filter leaves the chosen one out) and one box drawn in three places. The old
  callsign and address filter of the table is replaced by it. The map has a
  `visible` set that its draw, hover, click and framing all read, with the pinned and
  selected aircraft always kept.
- A card shown only because the filter left it first is not asked about and its photo
  is not fetched: typing in the box must not cause a request.
- Tests written first against neutral stubs: 25 failed by assertion; one that passed
  vacuously (the VDL2 flight fallback) was given a negative assertion; two guards
  (`searching_asks_for_nothing`, the source scan) cannot fail before the code and
  say so. Then green: `cargo test -p sdroxide-ui` 1113 passed / 0 failed / 8 ignored
  (1083 before, 30 new). Wasm check: 0 errors (one `QsoInfo` warning in
  `solar3d`, not ours). `rustfmt --check` clean on the four `.rs` files touched.
- `cargo check --workspace --all-targets` could not run: `sdroxide-rade`'s build
  script downloads opus and this sandbox has no route to it. Not claimed.

**Not done / blocked.** Layout of the box in the narrow table column and above the
map, on a real screen. The note about missing online data is shown only for words
that begin `type:` or `op:`, as decided; a plain word such as `boeing` that finds
nothing says nothing about it.

**Next.** The mode work.

---

## 2026-10-25 (aircraft) — a click on an aircraft pins its card, with the Planespotters link

**Summary.** On the ADS-B map, clicking an aircraft now pins a small card beside
it that stays until you close it with the cross or Esc, or click another aircraft.
The Planespotters link in it can be clicked, and the photo shows if it was already
loaded. Clicking and hovering still ask the internet nothing; a new **Look up
online** button in the card asks your station's relay about that one aircraft. It
has not been seen on a real screen or against a real station yet; that is the
next thing to look at.

**Done.** Branch `claude/aircraft-pinned-card`, one PR to `main`, no wire change,
no relay change.
- Pure rules in `adsb_map` (what a click pins, what closes it, where the card
  sits, no hover card over the pinned aircraft); the card itself is the hover
  card's (`card_view`), so the two cannot disagree; `aircraft_pin` draws it and
  holds the only place that asks.
- Tests written first against neutral stubs: 23 failed by assertion (two that
  passed vacuously were given a positive control), then green. `cargo test -p
  sdroxide-ui` 1083 passed / 0 failed / 8 ignored. Wasm check: 0 errors. `cargo
  check -p sdroxide-ui --all-targets`: no warning from these files. `rustfmt
  --check` clean on the five `.rs` files touched.
- `cargo check --workspace --all-targets` could not run: `sdroxide-rade`'s build
  script downloads opus and this sandbox has no route to it. Not claimed.

**Not done / blocked.** Placement, the clickable link and the thumbnail on a real
screen; the lookup button against a station with the relay on. Both need a person
at a screen. After a failed lookup (offline, busy) the button does not come back
by itself; the AIRCRAFT window's back-off still decides when it is asked again.

**Next.** The mode work.

---

## 2026-10-25 (UI) — the operator's rule: our PRs are F4NRM's to merge

**Summary.** The operator set a rule while the panel work was being tested: PRs we
open are **not** ours to merge — F4NRM reviews them or merges them himself. Nothing
was merged by me. He also said the queue goes back to the modes list, so this closes
the UI round.

**What happened in the session, in order.** #218, #219, #222 and #223 were all merged
**by him or from his side** at 09:02–09:03 UTC, before the last three UI commits
existed. Those three — the narrow header that stops the floating panel drawing over
itself, the WINDOW chip inside the floating panel, and the FLOAT chip on the panel's
OS window — were pushed to `ui/pane-way-out` after that merge, so they went nowhere.
They are now **PR #226** on their own branch `ui/panel-float-and-header`, rebased onto
`main` after #224 (the English-only sweep) and green: `sdroxide-ui --lib` 1249 passed,
gate silent apart from the four NAVTEX warnings.

**The rule, recorded where a session will find it.** `AGENTS.md`, under "Pull requests
to this fork": **"Our own PRs are never merged by us"** — not with `gh pr merge`, not
with `--admin`, not "because it is green and he is away". A local merge for testing
(`local/*` plus a build into `~/.cargo/bin`) stays allowed, because that is how a fix
gets clicked before it is offered; **origin is theirs**. Same line added at the top of
`TODO.md`.

**Where the UI round stops.** #226 is the last of it. The queue is the **modes list**
again — `MODE-PHASE-4-PROMPT.md`, STANAG 4285's coded payload first among them.

**Not done.** Nothing merged, and nothing more on the screens: he has said the
screens are F4NRM's and the programme is the modes.

## 2026-10-25 (UI) — three fixes built into one local binary, for the operator to test

**Summary.** The operator's standing point, in his own words: *"I always need to test
what we fix."* So the three UI fixes of this session are folded into one local build
and installed, rather than left on branches waiting for a merge: the SQL label that
holds the squelch open, mode panels docking under the waterfall instead of opening in
windows, and the gear-only setup chip with a window icon that sends the pane to a
window of its own. What he has to test is listed below, because a fix nobody clicked
is not finished.

**Done.** Local branch `local/sql-and-panels`, merges `eab2cfcf` (#218),
`0f55df8f` (#219: #215 + #217) and `03192a69` (#222). Built and installed at
`~/.cargo/bin/sdroxide`, 09:26, checksum matched, `SDR Oxide Brown 2.1.0_brown`.
`sdroxide-ui --lib` **1246 passed, 0 failed**; workspace gate silent apart from the
four NAVTEX warnings, which are F4NRM's and are the subject of a report to him.

**What the operator is testing, and what would be wrong:**
- **SQL**: click the word → letters green, rail to *off* (**open** on a rig with its
  own squelch). Click again → the number he had. Dragging the rail while green →
  held state ends and the number he set stays. Wrong if the letters do not change
  colour, if the number does not come back exactly, or if the first start of this
  build still opens panels in windows.
- **Docking**: pick any mode → its panel is *in the pane below the waterfall*, no
  window appears. Windows are opened by Detachable in Settings or the icon below.
  Wrong if a panel still pops up on mode entry, or if a panel is missing entirely.
- **Gear and the way out**: the setup chip is a gear with no word, and a window icon
  sits at the right-hand end of that row; clicking it detaches the pane and the
  waterfall grows. Wrong if the icon covers a chip, if the pane does not come back
  (dock chip in the new window), or if the gear alone reads as nothing.

**Not done.** Merged on `origin`: **no** — #218, #219 and #222 are still PRs, and the
installed binary is ahead of `main`. Looked at on a screen: **no**, by me — which is
exactly why it is installed rather than reported as finished.

**Next.** His verdict on each of the three. Anything wrong goes back to the branch
that carries it; anything right, and the PRs get merged and the binary is rebuilt
from `main` instead.

## 2026-10-25 (UI) — mode panels docked by default (#217, and #215 with it)

**Summary.** The operator's own report: clicking a mode opened its panel in a
floating window instead of the pane below the waterfall. It is the same block that
caused the two SSTV windows, so both issues are one change: on the first frame of
every mode the program opened that mode's panel as a window *and* saved that into the
settings. The block is gone — a panel docks, and a window is opened by the operator —
and the bits the pre-release wrote are cleared once, so the fix actually arrives on
the installation that reported it. It is in **PR #219**, extended to cover both.

**Done.** Commit `4f1bbffb` on `fix/sstv-two-windows` (PR #219, which already carried
#215's fix `2a64eeb2`).
- **Removed the mode-entry auto-open** — twenty `if mode.is_x() { self.show_x = true;
  self.ui_settings.tool_windows_open |= TW_X; }` lines — so the docked branch in
  `mode_panel` draws again and no window opens. `last_mode_for_tools` existed only
  for that block and is gone.
- **`UiSettings::tool_windows_repaired`** (new field, `UiSettings` is not a wire type
  — the wire carries `ClientScreen`) and `repair_mode_panel_windows`, called once
  beside `migrate_detached()`. It clears `MODE_PANEL_WINDOWS` and **leaves every
  other window bit alone**: GRID, SAT, MEMORIES, EXTRAS and the rest stay as the
  operator left them.
- **#215's fix is subsumed.** `image_window_for` stays (the draw and the docked panel
  ask which single window a mode would use), but with no auto-open the duplicate
  cannot happen; #215's frame test was retired in favour of a sweep that asserts the
  stronger thing.
- **Tests.** `entering_a_mode_docks_its_panel_and_opens_no_window` enters twenty-odd
  modes, drives a frame for each, and asserts no window opened and no bit was
  written — verified failing with one auto-open line restored (*"Js8 opened ["JS8"]"*).
  `the_mode_panel_bits_are_repaired_once_and_only_once` covers the migration, GRID's
  survival and the second call being a no-op.
- **Checks:** `sdroxide-ui --lib` **1241 passed**; `sdroxide-types` **793**; gate
  silent apart from the four NAVTEX warnings that are not ours; wasm clean;
  `rustfmt` clean on what this adds (frame.rs's 24 pre-existing hunks are untouched).

**Not done / blocked.** Looked at on a screen. Docking is asserted from the app's own
state; "the panel is under the waterfall and no window appeared" is a thing for an
eye, and it is the operator's or F4NRM's to give.

**Next.** If F4NRM would rather keep the auto-open behind a preference ("panels open
in a window" on the Detachable page), that is the same change with one condition on
it — his call, since the screens are his.

## 2026-10-25 (SSTV) — two windows on one panel, fixed (PR #219, not merged)

**Summary.** The operator reproduced a report that opening SSTV puts two windows on
screen. It was not the decoder: SSTV has its own window, and a second generic one
called IMAGE was added later for the image modes that had none, with an auto-open
that matched every image mode — so SSTV opened both, and the second looked like an
FT8 receive pane because it is the same picture panel drawn twice. One decision now
picks the window; a test enters each image mode and counts what opened, and fails on
the released build with exactly the reported symptom. It is up as a PR because SSTV
is F4NRM's.

**Done.** Branch `fix/sstv-two-windows`, **PR #219**, commit `2a64eeb2`.
- `8cac8eb1` (F4NRM, "Convert … image panels to tool windows") added
  `if mode.is_image() { self.show_image = true; … }`, and `is_image()` is
  `Sstv | SstvFm | Rifp`, while `d3fcc48a` had already given SSTV its own window.
- `image_window_for(mode)` is now the single decision, asked by the **auto-open**, the
  **draw**, and the **docked panel's suppression** — the last of which also cures a
  latent fault: the old condition suppressed the docked panel on `!self.show_image`
  whatever the mode, so an `IMAGE` left open from RIFP would have blanked SSTV's
  panel rather than doubling it.
- **Tests.** `entering_an_image_mode_opens_one_window` drives a frame per image mode
  with a silent controller and counts the windows; verified failing on the old code
  with `Sstv opened [("SSTV", true), ("IMAGE", true)]`, passing after. A source guard
  (`nothing_gates_the_image_window_on_is_image_alone`) pins the bare `is_image()` out
  of both call sites, since a decision test cannot see an auto-open added beside it.
  One trap found by the test itself: each case needs **its own settings folder** —
  the strip syncs each window's open bit from the saved settings every frame, so a
  shared folder handed RIFP the window SSTV had just opened.
- **Checks:** `sdroxide-ui --lib` **1240 passed**; gate silent apart from the four
  NAVTEX warnings that are not ours; wasm clean. `frame.rs` has 24 pre-existing
  rustfmt hunks (his one-line-if style) and this change adds none — checked by
  formatting a copy and diffing for my own identifiers.

**Not done / blocked.** Looked at on a screen. The count is asserted from the app's
own state, which is the reported symptom, but one window titled `SSTV` with the panel
in it is a thing for an eye.

**Next.** A comment is on #215 saying it is fixed and in the next pre-release; if
F4NRM wants the two windows merged into one instead (IMAGE for everything), that is a
different shape and his call.

## 2026-10-25 (UI) — the SQL label holds the squelch open (PR #218, not merged)

**Summary.** The operator asked for a way to open the squelch for a moment without
dragging the rail down and back: a click on the word **SQL**, green when it is
holding the gate open, and a second click to put back the number that was there.
It is built and it costs no space — the letters are already drawn — and it is up as
a pull request rather than committed, because the screens are F4NRM's while he works
through the detachable windows. The one thing not done is looking at it: the window
opened behind the operator's own session, so the colour and the width are measured
from the drawn text headlessly and the gesture is unproven until a person clicks it.

**Done.** Branch `fork/sql-label-holds-open`, **PR #218**, commit `fde96c57`.
- The strip's `ui.label("SQL")` is now `Label::new(sql_label(held)).sense(Sense::click())`
  with a pointing hand and a hover that says what the click will do. Held, the ink is
  `theme::GREEN()`; the text and font are unchanged, so the width is unchanged.
- **Both rails.** `sql_hold_toggle` is one decision for the software gate
  (`SQUELCH_OPEN_DB`, already formatted *off*) and the radio's own squelch (`0.0`,
  already *open*). The scale is stored **with** the value, and a hold met on the
  other rail is dropped and nothing sent — an interface swapped under the window
  must not put a rig's 60 % onto a dBFS gate.
- **The number put back is the operator's own.** Dragging the rail while held takes
  the gate back instead. Nothing is persisted and nothing is on the wire.
- **Tests, three:** the toggle on both scales plus the stale rail;
  `the_sql_label_is_green_while_it_holds_the_gate_open_and_no_wider` draws both
  states and reads the galley's own section colour, asserting green and equal widths
  (0.01 px) — the "no additional space" condition measured; and a source guard that
  the strip's label is the clickable one and both rails route through one decision.
- **Checks:** `sdroxide-ui --lib` **1240 passed, 0 failed**; workspace gate silent
  apart from the four NAVTEX warnings that are not ours; wasm check clean;
  `rustfmt --check` clean. The installed binary is left on `main` — a PR build is
  not what belongs in `~/.cargo/bin`.

**Not done / blocked.** Looked at it on a screen. A screenshot of the running app
caught the operator's own window instead of the strip (it opened behind it), and
taking another would mean taking over their desktop, so the visual check is theirs
to make — five seconds of clicking the word.

**Next.** F4NRM reads the PR; if he wants the gesture or the colour different, that
is a line each.

## 2026-10-25 (aircraft) — F4NRM's three PRs merged, and two tests that were not running

**Summary.** The three aircraft pull requests that were conflicting are all in: a
**Stats** view, a **Watch** list that alerts on an aircraft you asked about, and
**CSV/JSON export** of the tracked fleet. They conflicted because all three add a
chip and a pane to the same window, so each branch was brought up to date with
`main` and the two features kept side by side — Watch, then Stats, then the cards.
Merging them turned up something worth more than the features: **two tests on the
tree were never running** because their `#[test]` had gone missing, and a route in
the reverse-proxy list was unreachable. All three are fixed and the installed build
has them.

**Done.**
- **#212 Aircraft statistics** — merged as `5f6cb912`'s parent line; conflicts were
  documentation only. Its own test
  `a_list_from_the_station_reaches_the_stats_store_and_frees_the_line` had **no
  `#[test]`** on `main`, so it had never run. Marked, and it passes.
- **#213 Aircraft alerts** (`5f6cb912`) — conflicts in `aircraft.rs` and
  `aircraft_online_io.rs` were the Watch and Stats chip/pane/method added to the
  same places; both kept, three exclusive views now (Watch, Stats, Cards) plus the
  map. The merge cost the watch test its `#[test]` (the attribute lived in the
  shared tail that had to be inlined for the other side); caught by the
  `--all-targets` gate as "function is never used" and put back.
- **#214 Aircraft export** (`a511c2c6`) — the largest: three structural conflicts
  where each side had rewritten the same poll/pane region, resolved to keep both
  features; the `Kind::Export`/`Kind::Stats` requests are reads, so they pass
  `None` where the watch store posts a body.
- **A bug the merge surfaced:** `is_root_route` ended up with **two identical match
  arms**, one listing `/aircraft-export` and the next `/aircraft-watch`, so the
  second was unreachable and `/aircraft-watch` would not have been recognised at
  the end of a reverse proxy's prefix. One arm lists both now.
- **Dead test helper removed:** `aircraft_history_default.rs` carried a JSON
  `body()` with no caller on either branch.
- **Checks:** `cargo check --workspace --all-targets` shows only the four NAVTEX
  dead-code warnings that are F4NRM's and are recorded, not touched;
  `sdroxide-ui --lib` **1237 passed**, `sdroxide-server --lib` **77**,
  `sdroxide-aircraft-history` **70**, `sdroxide-aircraft-online` **45**, none failed;
  `rustfmt --check` clean on every file touched. `main` is level with `origin/main`
  at `a511c2c6` and the release build is installed at `~/.cargo/bin/sdroxide`.

**Not done / blocked.** Nothing. The installed binary is `main` — the release tag
`v2.1.0.2.5_brown` plus these three features — so it is a development build, not
the tag; the tag is untouched.

**Next.** The operator's own queue: the modes, and STANAG 4285's coded payload,
which still wants a capture with ten continuous seconds of usable signal.

## 2026-10-25 (STANAG 4285) — a plain teleprinter transmission now reads, and the sweep is off for the storm

**Summary.** A STANAG 4285 station sending **uncoded** text — the simple 1200 bps
mode, no error correction — could never have been read by this program, however
strong the signal: the character layer was cutting the bits into the wrong size of
group, seven bits of a teleprinter character were being read five at a time, and the
alignment was thrown away at every frame. That is fixed and proved: a synthetic
transmission of the text a French Navy test tape actually sends now comes back
letter for letter, where the old code produced gibberish. Nothing else changed, and
nothing here needs a radio — which is why I did it tonight rather than leave the
receiver sitting under a thunderstorm.

**Done.**
- **`fec::Ita2Framer`** (`f3998d16`) — a streaming 5N1.5 framer: hunt for a start
  bit, take five data bits least significant first, require a stop bit before the
  next start. It runs across the whole transmission instead of per frame, so it is
  self-synchronising and it survives a noisy start.
- **`fec::ita2_bits`** — builds a 5N1.5 bit stream from text, for fixtures.
- **Tests, both verified failing on the old code:**
  - `an_uncoded_transmission_reads_its_text`: a noiseless 1200 bps transmission of
    "FAAA DE FUF ZNR UUUUU ZUI TESTING RYRYRYRYRYRYRYRY" reads back exactly. Old
    path: `CPWXNU?09/,8-9156,52?98#75$!62&4766:4'/#0&$!62&47#0&6!-9885!3-9&026!...`.
  - `the_ita2_framer_syncs_on_an_idle_line_and_recovers_from_junk`: an idle line is
    all ones, so the first zero frames the first character; arbitrary junk costs a
    character or two and then it settles.
- **Checks:** `cargo test --release -p sdroxide-dsp` **494 + 26 + 14 + 1 + 8 + 14 +
  8 + 5 passed, 0 failed**; `cargo check --workspace --all-targets` silent apart
  from the four NAVTEX warnings that are not ours; wasm check clean; `rustfmt
  --check` clean.

  **Finished after the operator went to bed, and worth writing down because the
  commit that landed last (`17578f9b`) honestly says "not claimed":** the full run
  that was still in flight came back **495 + 26 + 14 + 1 + 8 + 14 + 8 + 5 passed, 0
  failed**, `rustfmt --check` clean, and the workspace gate silent apart from the
  same four NAVTEX warnings. So the tree is green — the caveat was about *when* the
  check finished, not about its result.

**Not done / blocked.**
- **The local sweep is stopped, deliberately.** The operator reports a bad storm:
  the RSP1 is on an outdoor antenna, the band is static, and a capture is not worth
  taking or leaving the radio listening through it. Nothing was recorded.
- **The KiwiSDR route is blocked on one host name.** A receiver in France would not
  care about the storm here, and the tree already speaks KiwiSDR — but
  `kiwisdr.com/public/` is now behind a click-through gate that needs a browser, and
  the handover's example host (`f6abj-kiwihf.ddns.net:8073`) is dead. **One host:port
  from the listing is all it needs** — the operator can read one off
  `rx.kiwisdr.com` in a browser, and then a capture of 6348 or 5215.6 kHz through it
  is a first-class route.
- **The coded payload is still waiting on material**: 93 continuous frames above 0.9
  reference concentration, and no recording we have reaches it (see the entry
  below).

**Next.** If the operator supplies a KiwiSDR address, record a tape through it and
score it with the probe's `coded readiness` line. Otherwise the next no-radio item
is the standard's own start-of-message rule: Annex E says the **last 26 bits** of
the SOM are correlated against the decoder output, and this crate requires all 32 —
64 times less sensitive on a marginal signal.

## 2026-10-25 (STANAG 4285) — a real bug in the deinterleaver, and the tape is too short

**Summary.** I found and fixed a genuine fault in the part of the STANAG decoder that
undoes the station's error-correction interleaving: one decoded value in every 32 was
being thrown away, and because the encoder had the same flaw in mirror image, every
test passed and only a real transmission would have shown it. That is now pinned by a
test that fails on the old code. The bigger news for you is the honest one: I can now
say, from any recording, whether it is even possible to decode a message out of it —
and none of ours is. A message needs ten continuous seconds of solid signal and the
best any of our files has is seven and a half. The watcher is running again over eight
frequencies, and every file it writes now prints a one-line verdict saying whether it
is worth keeping, so the next good tape will not be missed and a marginal one will not
be mistaken for a decoder fault.

**Done.**
- **The deinterleaver dropped one soft decision in 32** (`2462fea6`). Its shortest row
  holds nothing at all — row 31 on the receive side, row 0 on the transmit side — and
  `push` interleaved the write and the read row by row, so that position read its own
  row before this segment's value had been written into it and emitted a zero. Every
  row is now written before any is read, both sides.
  - **How it was found:** built the independent implementation
    (`luwangg/signal-analysis`, GPL — read for comparison, nothing ported) as a
    deinterleaver + Viterbi over the *same* soft decisions and diffed the streams.
    Before: mismatched at exactly one position in every 32 (output position 7, whose
    row is empty). After: **1 mismatch in 130 689 values**, a rounding edge.
  - **Test:** `the_deinterleaver_matches_the_standard_machine_at_every_position`
    compares every position against a longhand model of the standard's machine.
    Verified failing on the old code: **"199 positions disagree"**, one per segment.
- **Our Viterbi is the independent one's equal**, measured the same way: on our own
  clean 600 bps signal the two agree to 1 % and ours is the one that frames the SOM
  and the EOM.
- **`coded_quality()`** — one value per frame, its mean reference concentration, zero
  where the frame was fed as erasures — and a `coded readiness` line in the probe that
  turns "can this recording carry a message" into a number.
  - A message needs the interleaver's span: **31·j segments / 4 per frame = 93 frames**
    for the long interleave (23 for the short).
  - Measured, usable = refs > 0.9: **FUF 24 Oct — 73 usable of 557, longest run 69**;
    **FUF 23 Oct — 204 of 561, run 32**; **our 6348 — 150 of 335, run 82**; **our own
    synthetic — 207 of 207, run 207, and it decodes**. Not one recording has 93.
- **The record corrected.** The previous entry's "the tape was never weak" was read
  off the probe's **last** frame. The tape's mean constellation over 217 frames is
  **0.344 plain / 0.451 equalised / 0.469 with the DFE**, with 49 / 71 / 57 frames
  above 0.7 — so the equaliser is worth +0.11 and 45 % more usable frames, and not a
  decode. The probe now prints the mean as well as the last frame.
- **Every convention a loopback cannot falsify was swept** with the SOM as the judge,
  via the new `s4285_search` example: 24 orderings of the frame's four data blocks ×
  block reversal × all four interleaver wirings × j = 3/6/12/24 × repeat factor
  1/2/4/8 × pair order × differential encoding. **Nothing frames a message** on either
  tape's good stretch. The frame layout itself was re-measured on the good frames: the
  reference blocks sit at exactly 32, 80 and 128 with concentrations
  **0.98 / 1.05 / 1.19** against 0.34–0.46 for the data windows.
- **Checks:** `cargo check --workspace --all-targets` silent; `cargo test --release
  -p sdroxide-dsp` **492 + 26 + 14 + 1 + 8 + 14 + 8 + 5 passed, 0 failed**; wasm check
  of `sdroxide-ui` clean; `rustfmt --check` clean on everything touched.
- New tools, committed: `s4285_deint_dump` and `s4285_bits_dump` (the side-by-side
  against the independent implementation) and `s4285_search` (the convention sweep);
  the probe gains `S4285_CODED_OUT`, `S4285_FRAME_LOG` and the readiness line.
- **The watcher says why it did not trigger.** It was printing "5.6x" against a 3.0x
  threshold and writing nothing, because the ratio is meaningless when both numbers
  are noise and the absolute floor is what refuses it. Each line now ends
  `no frame (acf under the floor)` or `no match (ratio under threshold)`. Restarted
  on eight frequencies at 23:15Z with the new build.
- **One warning in the tree that is not ours:** `crates/sdroxide-dsp/src/navtex.rs`
  has `constant SYNC_SCORE is never used`, left by the merged NAVTEX sensitivity
  work. NAVTEX is F4NRM's, so it is recorded here and not touched — a change there
  goes up as a PR.

**Not done / blocked.**
- **No coded message has been decoded, and none can be from the material on disk.**
  Not blocked on equipment — blocked on one thing: a recording with ten continuous
  seconds of it. The watcher is running again (`s4285watch` over 6348, 5215.6, 8467.8,
  8478.5, 4232, 12857, 13411.9 and 12873.9 kHz, gain 45, 6 s dwells); its first hours
  produced no trigger.
- **Written, measured and reverted:** an LLR weighting of the soft decisions by each
  frame's own `1/σ²`, from the reference concentration. On the tape it changed nothing
  (SOM 30/32 either way) and on a burst-fade synthetic it changed nothing, so it is not
  in the tree rather than sitting there unproven.
- **The short interleave is the one open question that a capture can settle:** all
  three recordings have more than the 23 frames the short interleave needs, and none of
  them framed a message, so either they are all long-interleaved (likely — the
  standard's own note is that real traffic uses the long one) or something in our chain
  is still wrong.

**Next.** Watch `~/s4285-captures/night2/` for a file whose readiness line says the
long interleave is in reach, and open its text. If a file with a 93-frame run still
frames nothing, the next thing to suspect is the interleave length itself, and the
short interleave is then the first hypothesis to run.

## 2026-10-24 (aircraft) — the map's hover card: radio first, online only if already fetched

**Summary.** Resting the pointer on an aircraft on the map now shows a small card
instead of a line of text: what the radio heard (address, flight, registration,
position, altitude, speed, track, range and bearing, climb, any emergency) with its
source and age, and under it, only if the aircraft's card has already been opened,
the type, operator and route from the station's relay. Green "radio" and blue
"online" badges mark each line and a badge at the top says "radio" or "radio +
online". Hovering never asks for anything and never starts a download, so an
aircraft nobody opened has no online lines, and that is intended. It has been
measured, not looked at: please look at it on a screen. Next: the live run of the
relay.

**Done.** Branch `claude/aircraft-hover-card-online` (from `d1bdfc2b`). New
`aircraft_hover.rs` (pure card builder, clipping, footer, photo block, and the
drawing); `adsb_map::show` takes an optional `hover_ui` and keeps the old text
(`tooltip_text`) when it has none; `adsb_map_pane` hands it a card that only reads
the registry and the online memory. The registry's `dead_code` allowance is gone
(its test-only accessor is `cfg(test)`). 22 new tests, red first against stubs
(17 of 21 failed; the others passed vacuously on an empty card and were checked
by mutation): putting the "Online" status row back, an ask or a browser-storage
word in the file, the picture drawn before it is loaded, a missing footer reason
and a label that is not truncated each fail one. Two are not caught by mutation
and are said so here: the 14-line cap equals the most lines the builder can
produce, and "building a card leaves the store as it found it" guards the shape of
the call (a card takes `&OnlineState`, so it cannot write to the store).

**Not done / blocked.** The Planespotters link in the card is named, not clickable:
a tooltip follows the pointer and goes when it moves. The link is clickable in the AIRCRAFT window's card.
Not looked at on a screen (the browser build and the native window); no live run
against a real station.

**Next.** A live run of the relay with the real services, then the screen check.

---

## 2026-10-24 (aircraft) — the clients read the relay: badges, photo credit, no storage

**Summary.** The aircraft card in the browser, the phone and the remote client now
shows what the station's relay knows — type, operator, route, a photo — under the
radio's own lines, each marked with a green "radio" or a blue "online" badge, and a
badge at the top that says "radio" or "radio + online". Nothing changes unless the
operator has switched the relay on at the station; if it is off the card says
"online indisponible" once and does not ask again. It is not proven against a real
station or the real services, and whether a browser may load the Planespotters
thumbnail depends on a header only their server can set — if not, the credit and
link show without the picture. Next: a live run.

**Done.** Branch `claude/aircraft-online-clients` (from `2520f406`). New
`aircraft_online.rs` (pure: which question, when, what a reply means, the rows and
badges, the photo view) and `aircraft_online_io.rs` (the sending: a thread with
`ureq` natively, `fetch` in the browser; thumbnail decoded in memory). The native
remote client signs in once with the password the socket already had and keeps the
cookie in memory. The native local engine has no station: `NoStation`, no request.
No client names adsbdb or Planespotters' API; only the thumbnail address the relay
returned, and only on `t.plnspttrs.net`. No `PROTO_VERSION` change; `Cargo.lock`
gains the edges for `serde_json`, `ureq` and the relay crate's pure half.
Numbers: 36 new tests; `cargo test -p sdroxide-ui` 1035 passed in the lib plus the
integration tests; `cargo check --release --target wasm32-unknown-unknown -p
sdroxide-ui` and `-p sdroxide-web` clean; rustfmt clean on the new files. Red
first: 26 of 29 logic tests failed on `todo!()` stubs; the three source scans
passed on stubs and were shown to fail by injecting `localStorage` and an adsbdb
address; the "nothing is saved" test was shown to fail by writing the online state
into `save`. The sixth transport test (no station, nothing asked) **cannot catch a
regression of its own guard** — the result is the same by another path — and says
so in a comment.

**Not done / blocked.** Not run: the full `cargo check --workspace --all-targets`
(`sdroxide-rade` fetches at build time), any request against a live station, the
native sign-in and cookie path, the browser `fetch`, and a look at the drawn
thumbnail (the preview shows the credit-only variant). The sandbox disk filled
twice; regenerable `target/` output was deleted.

**Next.** A live run with the relay on; then F4NRM looks at the card.

---

## 2026-10-24 (STANAG 4285) — the equaliser reads the tape, and the tape was never weak

> **Corrected 2026-10-25.** Everything here about the equaliser stands. "The tape was
> never weak" does **not**: it was read off the probe's *last* frame, and the tape's
> mean constellation over 217 frames is 0.344 plain / 0.451 equalised. The last frame
> of a fading recording is not the recording, and the probe now prints the mean. See
> the entry above for what the tape actually needed — ten continuous seconds, which
> none of our recordings has.

**Summary.** The one STANAG 4285 recording that was said to be too weak to finish
now demodulates cleanly, and the reason was our own filter, not the radio: a
receiver change takes the same minute of audio from an unreadable constellation to a
usable one and makes the hardest channel the standard allows decode at all. (The
claim that the tape was good enough — "a new capture is withdrawn" — was wrong; see
the correction above.) Also fixed: the program was labelling the *cleanest* BPSK
signal we have ever recorded as "unknown rate", and showing nothing for it.

**Done.**
- **`ls_equaliser`** replaces the T-spaced `ls_channel` + `zf_equaliser` pair in the
  coded and displayed paths. It is a direct least-squares filter over
  **half-symbol-spaced** samples whose target is the symbol sent at each instant, so
  it models the pulse as well as the path and its output is aligned without the
  three-shift group-delay search. `LS_FRAC_TAPS = 19` (swept 9/13/19/25/33/41),
  `LS_LOAD = 1e-2` (swept 0.003–0.3, moves the tape's constellation by 0.008).
- **Numbers, same files, same tunings** (`S4285_EQ=off|on` on
  `examples/s4285_probe`):
  - tape `FUF-13031.200-…150300Z`: constellation **0.046 → 0.790**, data blocks
    0.30/0.24/0.08/0.25 → **0.71/0.73/0.91/0.86**, refs 0.80/0.78/0.80 → 0.85/0.83/0.85.
  - our `6348.000-…045545Z`: **0.728 → 0.859**; `…044640Z` unchanged (Auto keeps the
    plain path); clean synthetic at 6 dB unchanged — the equaliser costs nothing.
  - synthetic sweep, first level that decodes: 1.5-symbol echo plain 15 dB /
    equaliser **8 dB**; two equal paths 2 ms apart plain never (18 dB) / equaliser
    **15 dB**.
- **Tests, and they fail without the fix.** `a_coded_message_survives_a_two_path_channel`
  now asserts both halves — the plain path reads **no message** at 10 dB and the
  equaliser reads it bit-exactly — and a new
  `a_coded_message_survives_the_two_equal_paths_the_standard_budgets_for` does the
  same at 4.8 symbols / 15 dB. Verified by making `ls_equaliser` return `None`:
  both fail ("the equaliser did not read a two-path message at 10 dB", "two equal
  paths 2 ms apart lost the message"), and pass again with it restored.
  `the_least_squares_equaliser_undoes_a_symbol_level_echo` rewritten for the new
  filter: 176 of 176 symbols, worst error 0.01.
- **The rate verdict was backwards** (`rate_from_concentrations`): the old rule asked
  the leading concentration to beat the runner by half again, which on a *clean*
  BPSK frame reads 0.933/0.760/0.357 — ratio 1.23 — and answered **`unknown`**,
  suppressing the uncoded text path. Now a cascade on which concentration stands up
  first. One file in the corpus changes, and it is the right one:
  `FUF-…125317Z` `unknown` → `1200 BPSK uncoded, or a coded BPSK rate`. Pinned by
  `the_rate_verdict_is_right_on_the_cleanest_bpsk_frame` (all numbers off the air)
  and `a_clean_synthetic_frame_is_reported_as_bpsk`.
- **Checks:** `cargo check --workspace --all-targets` silent; `cargo test --release
  -p sdroxide-dsp` **491 + 26 + 14 + 1 + 8 + 14 + 8 + 5 passed, 0 failed**;
  `cargo check --release --target wasm32-unknown-unknown -p sdroxide-ui` clean;
  `rustfmt --check` clean on both touched files; the off-air gate
  (`SDROXIDE_S4285_SAMPLE=… --ignored`) passes at mean refs **0.816 / 0.826 / 0.831**
  (was 0.61/0.70/0.57).
- **The record corrected:** the "tape is about 4 dB in band, needs a stronger capture"
  finding is **withdrawn** — it was the T-spaced fit's own failure read three ways.
  Both tapes now read data blocks at 0.71–0.96 with references at 0.97–0.99.

**Not done / blocked.**
- **No real coded transmission has been decoded.** Not blocked on the bench any more:
  both tapes are measured to *carry* the code (their descrambled bits are not ITA2 at
  any of 7 alignments × 2 polarities × 2 bit orders — 0.22 against a 0.23 chance —
  and show no rate-1/16 period), and both have a good eye. The fault is in the coded
  chain, and it is now the only suspect.
- **The interleaver/decoder comparison against the independent implementation** was
  not built in this session; it is the next measurement, and it needs no radio.
- **The uncoded character layer is still 5-bit chunking, not 5N1.5** — it cannot read
  ITA2 text from a 1200 bps uncoded transmission at all. Recorded, not fixed: neither
  tape is uncoded, so it is not on this path.

**Next.** Build `luwangg/signal-analysis` (GPL, C++/octave) and run it over
`FUF-13031.200-…125317Z`, then compare its decoded bit stream with ours symbol for
symbol — data-symbol order, bit order, and whether the interleaver works in 32-symbol
segments or bits. A framed message on the known text is the judge; the
metric-versus-shuffled-null statistic is invalid and must not be used.

---

## 2026-10-24 (aircraft) — an optional server relay for the aircraft card's online fields

**Summary.** The aircraft card can now be given its internet fields (type,
operator, route, and a photo's credit and link) by the station itself, so a
browser, a phone or a remote client never has to call those services. It is off
until the operator sets `SDROXIDE_AIRCRAFT_ONLINE=1` on the server, needs no key
or account, and the radio side of the card is untouched either way. Nothing in
the clients uses it yet: that is the second PR. What is not proven is the real
answers of adsbdb and Planespotters, because this sandbox cannot reach them.

**Done.** New crate `sdroxide-aircraft-online` (parsing, cache, relay, and an
HTTP part behind the `fetch` feature) and `GET /aircraft-info/{icao}` in the
server, behind the station's login cookie and shut when the switch is off. JSON
cache 24 h, failures remembered 5 min, 2000 entries per kind, one outbound
request per key, 30 questions a minute overall, fixed hosts only, ICAO exactly
six hex digits. The Planespotters photo is never fetched, stored or re-hosted:
only the thumbnail URL (unchanged), the credit and the page link are returned.
No `PROTO_VERSION` bump. Tests: 50 in the crate (45 without `fetch`), 30
server lib, 3 integration (`aircraft_info`); 23 of 23 hand mutations caught.

**Not done / blocked.** The two services' formats were written from their
documentation and **need one live run** from a machine with internet. The wasm
check of the crate is clean. `cargo check --workspace --all-targets` and the
radio-linked tests depend on `sdroxide-rade`, which fetches at build time.

**Next.** PR B: the clients, with the badges.

---

## 2026-10-24 (MT63) — the transmit fold, and the lock's acceptance is still the wall

**Summary.** MT63's transmit half was broken for two of its six variants and is now
right: our MT63-1K and MT63-500 signals were synthesised at a rate that put the whole
bank above Nyquist, so they folded into a mirrored signal that nothing — including
our own receiver — could read. All six variants now read their own transmission
back. The receive half is half done: five of the six corpus samples decode whole,
but only the batch decoder finds them, because the live lock still trusts the dial
and the samples sit 20–50 Hz away. The measured centre that would fix that also lets
the lock onto noise, and I could not separate the two in this session — that is the
next piece of work, and it is written down rather than half-shipped.

**Done.**
- **The fold** (`a558d624`, swept into a STANAG commit by a `git add -A` in a shared
  tree; named in `c8191c35`): the modulator synthesised at `variant.rate()` =
  `2 × bandwidth` while the bank sits at `centre ± bandwidth/2`. Fine for 2K at
  1500 Hz (500–1500 Hz at 4000 samples/s), broken for 1K (1000–1500 at 2000) and for
  500 (1250–1750 at 1000): the whole bank above Nyquist, folded. Now
  `max(2.2 × (centre + bandwidth/2), variant rate)` with the symbol length scaled to
  match. `a_synthetic_mt63_transmission_reads_back` covers all six.
- **The probes removed** (`c8191c35`): four debug functions and an orphaned gate that
  the same sweep carried, two of which no longer compiled as written, and which made
  the gate emit duplicate-attribute warnings. Plus a one-word `let mut` in
  `sdroxide-types/src/ui.rs` that was the only other warning.
- **Measured on the receive side:** with a 5 Hz centre sweep, 2KS 4/4 at 1665 Hz,
  500S 4/4 at 1690, 2KL 2/2 at 1680, 500L 2/2 at 1720, 1KS 2/2 at 1705, 1KL nothing
  anywhere. So the decoder handles the narrow banks; the *live* lock is the gap.
- **Experiments that did not ship, with their numbers.** A measured centre (the
  comb's outermost-carrier midpoint, `measure_centre`) plus text-first scoring took
  500S from not locking to 4/4 and 1KS to 3/4 — and let the lock onto noise. A
  character-agreement test between two decodes a hair apart separated them 71 %
  against 58 %, too thin. A shared-word test also failed, because both decodes of
  *noise* return the same wrong text — the nudge is smaller than the symbol, so it
  measures determinism, not signal. A lopsided-histogram test (real decodes are
  dominated by idle characters, noise is near-uniform) rejected real 2KL/1KS as well
  as noise. None of it shipped: the tree is green and these are recorded for the next
  attempt.

**Not done / blocked.** The live lock's acceptance. What it needs is a test that is
**temporal** — a decode that keeps reading as more audio arrives, versus one that
changes — because the one-shot statistics genuinely overlap at the first attempt
(both score textiness 48–76 and a Walsh median SNR near 3).

**Next.** That temporal acceptance, then the live corpus gate for all six.

---

## 2026-10-24 (STANAG) — the audio band was not the fault, and the data path is

**Summary.** The one coded tape we have — a French Navy FUF test transmission —
still does not decode here, but the reason we had written down was wrong, and that
is now measured rather than assumed. Everything up to the payload reads perfectly
through an audio chain as narrow as the tape's own, so the request for a wider
filter on the reporter's side would have been a wasted errand. His file is loud,
clean, correctly tuned and correctly framed; what it will not do is put its data
symbols on a constellation, and that points at the receiver's phase tracking
inside the frame rather than at the recording. Nothing has been read off the air
yet, and nothing is claimed to be.

**Done.**
- `stanag4285.rs`: `band_limit` (a 401-tap Hann band-pass) and
  `coded_round_trip_band`, so the whole coded chain can be run through an audio
  chain of any width, plus the test
  **`the_coded_chain_survives_a_truncated_audio_band`**.
- **The finding, and the numbers.** The bits come back exactly at
  **800–3000 Hz** (his tape's own band), and still at a **1000 Hz** high-pass and
  a **2600 Hz** low-pass; the standard's own band (300–3300) is the first
  assertion. The two corners are pinned so a front end that needs more fails in
  the test rather than on the air.
- **His file, re-measured** (60.9 s, 48 kHz): the two channels are identical, so
  the mono mix is not the fault; no carrier from 1400 to 2200 Hz beats 1800; SNR
  ≈ **70 dB** in 300–3400 Hz against 3.6–11 kHz; occupied ≈ 400–3000 Hz; frame
  period **106.73 ms**; locked with folded sync **0.781** (null 0.103), ACF
  **0.599**, 335 frames, references **0.626 / 0.697 / 0.575**, carrier +92 Hz.
- **The number that names the fault.** The descrambled data constellation is
  **0.234 / 0.070 / 0.080** where BPSK on the π/4 lattice should read about 1 at
  all three — so the data phases are spread continuously.
- **Where in the frame, and it decides the fix.** The same concentration per data
  block is **0.309 / 0.284 / 0.274 / 0.187** — all about the frame's own — so the
  smear is **inside a 32-symbol block (13 ms)**, not a drift across the frame. That
  rules out both cheap repairs (a line through the reference points is already
  there; per-block anchoring would not help). What smears a symbol in 13 ms while
  leaving sixteen identical reference symbols coherent is **delay spread of a
  symbol or more**, the impairment the standard's own tables allow for and the one
  a real receiver meets with an equaliser (Annex C/D).
- `STANAG4285-HANDOVER.md`: a new section with the table, the ruled-out list and
  the two things recorded but unexplained (an envelope line at 3607 Hz; the
  signal starting at 400 Hz). `MODE-PROGRESS.md` carries the same, and the mode
  table's row is corrected.
- `cargo test -p sdroxide-dsp --release --lib stanag` — **13 passed, 1 ignored**
  (the off-air gate); `rustfmt --edition 2024 --config-path rustfmt.toml` clean.

**Built: the closed-form equaliser, and where it stops.** The channel is now
estimated from the 80 known sync symbols by least squares and inverted by solving
`(R + σ²I)·w = p`; at the symbol level it recovers **176 of 176** symbols through a
1.5-symbol echo (worst error 0.09). On pulse-shaped audio it does not help yet
— its output makes the reference blocks collapse *worse* (2.81 → 2.02), so the gate
declines it and the frame reads as before. **The timing is ruled out**: the
sampling phase is now chosen by the least-squares residual over the known sync
(a proper error signal, a transition on almost every symbol) and it picks the same
instant the old criterion did, 0.60. What is short is the **model**: a five-tap
T-spaced fit does not describe a raised-cosine signal off a two-path channel well
enough to invert, so the next attempt estimates the channel including the pulse —
longer and fractionally spaced — which is a change to one function.

**Recorded: the adaptive equaliser does not converge.** An NLMS
fractionally-spaced FFE+DFE trained on the frame's 80 known sync symbols stalls at
the delay line's gain — training residual **0.88** against a unit target, whatever
the step size or tap count — and *lowers* the reference concentration (2.81 →
2.59). The inputs are not the reason: measured, the sync is recovered exactly at
the current timing (correlation 0.250 against a mean amplitude of 0.252, sign for
sign, clean and two-path alike). So the method is wrong, not the material: a loop
started from a delay line has to find a four-times gain and a fractional delay at
once on 80 symbols, and does not. The test that fails
(`a_coded_message_survives_a_two_path_channel`, ignored) is in place, and the way
in is **closed-form**: a least-squares channel from the known sync, then an
explicit inverse or a DFE seeded with it. The adaptive attempt was reverted rather
than left in, so nothing dead ships.

**Done, and the answer is the bench.** The offset was the real fault: it was read
off three reference blocks, which any echo bends into a ramp — 13 Hz wrong on the
two-path synthetic, +92 Hz wrong on his file. Refining it against the 80 **known**
sync symbols takes it to +1.5 Hz, the sampling phase to the correct 0.50, and the
data constellation from 0.234 to 0.347; the equaliser then decodes a two-path
synthetic end to end (`a_coded_message_survives_a_two_path_channel`, now passing
and no longer ignored).

But the same fit gives the tape's in-band SNR directly — **≈ 4 dB**, agreeing with
the constellation (0.35) and the reference blocks (0.65). A 4285 needs 10 dB; no
equaliser closes a 6 dB gap. **So STANAG is parked pending a stronger capture**,
and that is the one thing needed: the same tape on a quieter path, a better antenna
for 13 MHz, or more RF gain.

**The wider-filter request is withdrawn.** Mark measured the transmission himself
(#165, 12:47): at 4 kHz of SDR++ bandwidth he reads it as ~2.6 kHz wide at the
noise floor, about 500–3100 Hz — the tape, not his chain — and our own copy shows a
rolloff from 200 Hz rather than a wall at 800. The truncation test already says
800–3000 is enough. So the ask to him is strength, not width: a minute when the
signal is up (it is dropping in and out), more RF gain, or baseband I/Q if SDR++
can record it, which would leave no audio chain at all.

**Not done / blocked.** The payload is still unread, parked on the bench. Material
already on the disk, so no bench. A *second* capture of the same tape on a quiet,
steady path still needs the bench, and the watcher is the tool for it: a spread-out
channel is a bad witness for a modem.

**Next.** The phase tracker; then the rest of the mode queue.

---

## 2026-10-24 (stanag, eleventh pass) — segment interleaver landed, band limits measured

**Summary.** The interleaver now works in the standard's 32-symbol segments, so a
fade is spread over ten seconds instead of a third of one. The two impaired-channel
tests were re-baselined honestly (the impairment's cost comes out of the noise
budget), and the band the tape arrives in finally has a number: 800–3000 Hz costs
about 4 dB, and a low corner at 1000 Hz never decodes at any level. The captures
still do not frame a message, so what is left is the demodulation of real signals,
not the coding.

**Done.** `ConvInterleaver::push` gathers 32 symbols per commutator step; the
round-trip test measures the pipeline latency instead of assuming it; the flush in
the test transmitter is derived from the depth (32·31·j + 102) and the audio gets two
interleaver spans of margin; `coded_round_trip_noise` sets the noise budget, and the
band and two-path tests now spend theirs on their impairment.

**Numbers.** Span 11 904 symbols = 9.9 s at j = 12 (was 372 = 0.31 s). Round-trip
64/64. Band sweep at 10/14/18/22/26 dB: 300–3300 ✓10, 300–2600 ✓10, 800–3000 ✓14
(✗10), 1000–3300 ✗26. Captures: 0 messages, tape best SOM 28/32, 6348 capture 27/32.
Tests 16 passed, gate silent, fmt clean.

**Next.** The demodulation of real signals: our own signal reads 0.99 and decodes,
the captures read 0.73–0.82 and do not, and with the coding now verified end to end
the difference is in the symbols. The 0.93-reference capture is the test bed.

---

## 2026-10-24 (stanag, tenth pass) — the segment interleaver, written and measured, not landed

**Summary.** The interleaver is the wrong shape: the standard interleaves 32-symbol
data segments, this receiver advances its commutator every symbol, and the delays are
32 times shorter as a result (0.31 s against 9.9 s, the ten-second long interleave
every other source describes). I wrote the segment form and got two clear signals
that it is right — the round-trip returns 64 of 64 bits once the test looks for the
latency, and on the band-limited receiver test the framer matched the SOM **32 of
32**, which no earlier version ever came close to. It is not in the tree: the tests
whose audio is shorter than a ten-second interleaver stay red, and the lead-in and
flush plumbing has to be redone with it.

**Done.** The rewrite, twice, and the measurements above. Reverted to keep the suite
green; the design and the exact patch are recorded in `STANAG4285-HANDOVER.md`.

**Numbers.** Old form: max delay 31·12 = 372 symbols, 0.31 s. Segment form: 31·12
segments = 11 904 symbols, 9.9 s. Round-trip 64/64 at the found offset; SOM 32/32 on
the band-limited test with the segment form; no message closed because the test audio
is ~200 frames where ~290 are needed.

**Next, and it is mechanical.** Send the standard's ten-second lead-in from
`s4285_test_signal` and from `coded_round_trip_path`, derive the flush from the depth
(32·31·j + 102), give the tests a helper that measures the pipeline latency instead
of assuming it, then land the segment interleaver and re-run the captures. Until that
lands, nothing on the air can decode: this is the blocker.

---

## 2026-10-24 (stanag, ninth pass) — the scrambler was seeded wrong, and the judge never worked

**Summary.** Two findings, both big. First, our scrambler initialised its 9-stage
register to `1` instead of **all ones**, so on a real transmission the sync locked
normally (it is sent unscrambled) while every reference block and every data symbol
was descrambled into noise — reference blocks read 0.55 and the data constellation
0.396. Seeded correctly they read **0.76** and **0.750**, and 451 frames are
accepted instead of 242. Second, the statistic this whole effort has been judged by
— the decoder metric against the same values shuffled — reads **1.02× on our own
clean signal, which decodes**, so it never separated signal from noise and every
"ruled out" conclusion drawn with it is void. With a decision-feedback equaliser
added (0.396 → 0.823 data constellation) the tape is close but still frames no
message.

**Done.**
- `scrambling()` seeded `0x1FF`; the crate's generator test updated to the correct
  first symbol (phase index 7) with the reason on it.
- `EqMode::Dfe`: LMS DFE, 11 fractionally-spaced feedforward taps, 4 feedback taps,
  mu 0.05, trained on the sync and corrected on the reference blocks, two passes.
  Opt-in, because it spoils a clean signal.
- QPSK rates fed two soft bits per symbol in both bit orders; erasures and the
  trace follow the stride.
- Three diagnostic examples (interleaver conventions, commutator start phase, QPSK
  axes) and the `S4285_EQ` probe knob.

**Numbers.** refs 0.55 → 0.76 (0.95 best frames), constellation 0.396 → 0.750,
frames 242 → 451; with the DFE 0.823 and per-block 0.81–0.87; clean-signal control
under the DFE 0.995 → 0.736 (so not a default); the metric judge on that same clean
signal **1.02×** — the reason to distrust it.

**Next, in the order a second opinion recommends (and it agrees with both other
decoders).** Verify the data-symbol mapping and the coded-bit ordering against the
reference implementation — in particular whether the interleaver is fed bits (two
per QPSK symbol) or symbols, and the direction of the 9i commutator — then the
Viterbi framing. The equaliser has now had its turn.

---

## 2026-10-24 (stanag, eighth pass) — sensitivity measured, and the tape is not noise-limited

**Summary.** The chain's own limit is now a number per rate: it reads a message at
**-6 dB** in 3 kHz at 150 and 75 bps, with the reference blocks still collapsing at
**0.90**. Mark's tape collapses its reference blocks at **0.55**, and its symbols
scatter ±87° — which white noise at the same level does not do. So the tape's
problem is not weakness but a non-Gaussian impairment (fast phase wander or in-band
interference). That is the first statement in this work that is a bounded claim
about the tape rather than a cleared suspect, and it points at phase tracking, not
at coding, interleaving or the equaliser.

**Done.**
- `S4285Frame::symbols` (the 176 descrambled, channel-corrected symbols) and
  `S4285_SYMBOLS_OUT` in the probe, so a capture's frame structure can be measured
  instead of assumed.
- Noise ladder per rate (table in the handover).
- Tape analysed against it: per-position and per-block phase, sliding 16-symbol
  coherence over 242 frames.

**Numbers.** 600/2 decodes at 2 dB (constellation 0.921); 150/8 and 75/16 decode at
**-6 dB** (0.612 / 0.578) with references **0.905 / 0.906**. Tape: references
0.55 / 0.61 / 0.51, symbol scatter 86–91° sd, sliding coherence flat at 0.24–0.35.
Tests 16 passed 1 ignored, gate silent, rustfmt clean.

**Blocked, and it is the right block.** Which rate MultiPSK reads is a question for
Mark, and it decides the whole next move: at 75 bps we have 12 dB of coding gain to
spend and should be decoding this tape; at 600 we are 3 dB short in the
demodulator. It is in `TODO.md` under *Waiting on other people*.

**Next when the answer or a stronger capture arrives.** Phase tracking with a
per-symbol, decision-directed loop on the *reference blocks* first (a 16-symbol
block is short enough to follow wander that a per-frame line cannot), judged by
whether the reference concentration rises from 0.55.

---

## 2026-10-24 (stanag, seventh pass) — longer channel estimates are worse

**Summary.** One more equaliser variant measured and rejected in minutes: giving the
channel estimate more delay spread (up to 5 ms, which is what HF multipath can be)
makes the tape's constellation *worse*, not better. So whatever is closing the eye
is inside two symbols, and the short estimate stays.

**Numbers.** Data constellation 2-fold on Mark's file at 1850 Hz: default 5 taps /
9-tap equaliser **0.396**; 9 / 17 → **0.348**; 13 / 29 → **0.312**. Reverted; tests
green (16 passed, 1 ignored).

**Next.** Within-frame amplitude/phase wander: the reference blocks read 0.55–0.61
while the data read 0.396 in the same frames, and a timing error would hit both
equally. The next measurement is a **piecewise per-block** channel estimate
(sample-and-hold) in the coded path, judged first by the constellation and then by
the metric.

---

## 2026-10-24 (stanag, sixth pass) — the tape is not weak, it is smeared

**Summary.** The one question worth answering next was whether Mark's recording is
simply too weak for the chain. It is not: a generated signal with added noise still
decodes at 2 dB, where its constellation reads 0.921, while the tape reads 0.396.
Noise fills the eye in; this recording's eye is being closed by the channel. That
is a different fault, and it is the first time the search has had a measured
direction rather than a list of suspects.

**Done.** Calibration of the chain against deliberately added noise at six levels,
using `s4285_test_signal` and `s4285_probe`; the table is in the handover. Nothing
in the tree changed for it — the experiments (wider phase window, frame-score
trace, WAV dump from the loopback test) were all reverted, and the tree is clean.

**Numbers.** Constellation 2-fold: clean 0.995, 10 dB 0.981, 6 dB 0.959, 3 dB
0.934, **2 dB 0.921 and the message still decodes** (SOM 32/32). The tape: 0.396,
references 0.55. Bench trap recorded: the generator's default sub-carrier is
1500 Hz, the probe's default 1800 Hz, and probing at the wrong one reads 0 frames
with a 0.999 lock — which looks like a broken modulator.

**Next.** An equaliser, not another convention: the eye is closed by ISI and timing
wander. The concrete first step is per-block channel estimates with a short
fractionally-spaced equaliser trained on the 48 known reference symbols, judged by
the same metric-vs-shuffled-null test — and, because the tape's references read
0.55 rather than 0.99, by the constellation first.

---

## 2026-10-24 (stanag, fifth pass) — the judge was reading the wrong stream

**Summary.** The check that decides whether the code is in the recording was
reading a stream with holes in it. The decoder is fed every locked frame, including
the poor ones (as erasures, so the interleaver stays aligned), while the check was
reading only the frames good enough to report — and one hole shifts every soft
decision after it by a whole frame, which is exactly enough to hide real code.
That is now fixed, and the answer on the correct stream is still no: the trellis
sits at its null over 555 frames of real decisions. Reading the right stream also
showed how much of this recording is actually poor: 56 % of the locked frames fall
below the gate that decides whether a frame is worth reporting.

**Done.**
- `S4285Receiver::trace_coded_stream(on)` and `coded_stream()`, with the reason
  documented on the method, and an assertion in the round-trip tests that the
  traced stream is whole frames (128 per frame, erasures included).
- `s4285_fec_check` judges the traced stream and its SOM against the same
  statistic on shuffled values, through the receiver's own chunking.

**Numbers.** Mark's file, 1850 Hz: 242 reported frames, **71 168** traced
decisions → 556 frames fed, **314 of them erasures**. Best ratio 1.00× its null.
With `MIN_CODED_REFERENCE` lowered to 0.0: 555 frames, 71 040 values, ratio
**1.00×** — erasures are not the cause. SOM on the traced stream: real **31/32**
against **27/32** shuffled; no branch reaches 32 and no message closes, so it stays
a near-miss. Root-raised-cosine receive filter (α 0.2/0.35/0.5) breaks acquisition
(lock 0.900 → 0.465) and was reverted. The file is dual mono — L and R decode
identically.

**Commands.** `cargo test --release -p sdroxide-dsp --lib stanag` → 16 passed, 1
ignored; `cargo check --workspace --all-targets` → 0 warnings; `rustfmt --check`
clean on everything touched.

**Next.** The symbols themselves are what is wrong, not the alignment: 555 frames
of real decisions carry no trellis. The useful measurement now is *how bad the
channel is per frame* — the reference concentration distribution over all 556
frames, not just the reported ones — because if the tape is 3–4 dB poorer than the
chain's threshold, the honest answer is a better front end (or a stronger
recording), not another convention sweep.

---

## 2026-10-24 (stanag, fourth pass) — the channel path cleared, and a false statistic retired

**Summary.** The choice between the plain symbols and the equaliser is not what is
stopping the tape from decoding — all three settings sit at their own null on the
best recording. Two real things came out of asking: the equaliser could hand back
a frame of silence and nothing noticed, which is now fixed and tested; and the
"closest SOM 31/32" that the handover quoted as near-success is a number that
fires on noise too, so it has been withdrawn. The trellis metric — the one
statistic here with a proper null — still says there is no code visible.

**Done.**
- `EqMode::{Auto,Off,On}` on `S4285Receiver` (`set_eq_mode`), honoured by the coded
  path only; the displayed frame is untouched.
- Fixed the equaliser's fallback in `coded_symbols`: it started its group-delay
  comparison with a zero-filled buffer, so a frame could be read as 128 zeros.
- New test `a_coded_message_survives_every_channel_path`: a clean generated 600 bps
  message decodes its own bits with `Auto`, `Off` **and** `On`. Verified failing
  before the fallback fix (`no message with On`, reference blocks 0.000).
- `s4285_fec_check` now reports, for every branch, the closest SOM to the known
  pattern against the same statistic on shuffled values, using the receiver's own
  chunking (256 pairs, 64-pair overlap).

**Numbers.** Mark's file, 1850 Hz: 242 frames, 30 976 soft decisions in all three
modes; best ratio Auto **0.99×**, Off **0.99×**, On **1.00×**. SOM: real 24–28/32,
shuffled 24–28/32 on every branch — no discrimination.

**Commands.** `cargo test --release -p sdroxide-dsp --lib stanag` → 16 passed, 1
ignored. `cargo check --workspace --all-targets` → 0 warnings. `rustfmt --check`
on both files clean.

**Still open.** The receiver's own SOM (31/32) is measured on a stream that
includes erasure-filled frames, so its null is a different question and remains
unestablished. Next: null-test it inside the chain (or report it as a z-score
against the expected maximum for that many bits) before it is trusted — if it
survives, some frames are decoding and that is where to dig.

---

## 2026-10-24 (stanag, third pass) — three hypotheses eliminated with numbers

**Summary.** The French Navy tape still does not decode here, but the search is
much narrower: the phase tracker, every plausible interleaver wiring and depth,
and a one-symbol matched filter have all been measured against the new recording
and none of them is the answer. What is left is the channel equaliser, and the raw
baseband capture is the file to do it on.

**Done.**
- `Interleave` made selectable (`announce` / `swapped` / `rows-flipped` /
  `swapped+flipped`) and the depth swept (3/6/12/24/48 at 600 bps), in
  `s4285_fec_check`. On Mark's file at 1900 Hz: **0.94–0.99×** its shuffled null
  for all 40 combinations. Commit `a9999e57`.
- A one-symbol boxcar matched filter before the decision: lock **0.900 → 0.465**,
  constellation **0.396 → 0.211**. Reverted.
- Handover carries both, with the numbers.

**Not done / blocked.** Nothing blocked. The remaining candidate is a **channel
equaliser** (Annex C/D) — a linear phase fit across the frame is all we have, and
the data symbols' 0.40 concentration says there is more channel than that. The
423 MB CF32 baseband IQ (`…-134238Z.cf32.wav`) is the file for it: the whole
passband, so the equaliser is not fighting an audio filter as well.

**Next.** Open the IQ, extract the channel at 13031.2 kHz, and see whether the
constellation improves when the whole passband is available — that separates "the
audio file is missing something" from "our equaliser is not enough" before any
equaliser is written.

---

## 2026-10-24 (stanag, second pass) — the phase tracker is ruled out

**Summary.** The first idea for the French Navy tape — that our demodulator was
losing the code to a drifting phase — was built and measured, and it is wrong: the
tracker changes the code's own score by nothing at all. That is a real answer, and
it points the next attempt at the interleaver instead, on the best recording this
mode has ever had.

**Done.**
- A decision-directed phase tracker on the data symbols (sign decision, residual
  angle, gain 0.15) after the reference-block fit. Metric before and after:
  **0.96×** its shuffled null at 1850 Hz, **0.96–0.97×** at 1900 Hz — identical to
  four decimals on the strongest branch.
- Reverted: an unproven change that buys nothing does not stay in the tree.
- `STANAG4285-HANDOVER.md` records the negative and names the next measurement.

**Not done / blocked.** The interleaver-convention sweep, re-run against this file.
The old sweep was against a recording with 0.23–0.34 data concentration; this one
has 0.396 and a 0.899 lock, and a metric sitting exactly at its null is what a
*wrong interleaver* looks like as much as a noisy one.

**Next.** Parameterise the interleaver's commutator/row order in `CodedChain` and
sweep it on `FUF-13031.200kHz-20261024-125317Z.wav`, judged by the metric/nul
ratio; then the 423 MB baseband IQ, which is still unopened.

---

## 2026-10-24 (stanag) — the Mac recording measured, and a mistake to record

**Summary.** Mark sent a better recording of the French Navy tape — made with our
own program on his Mac, at the full 3.7 kHz — plus a raw baseband capture. Our
receiver locks it far better than the old file and the equaliser now gets the data
symbols to 0.40 where the old recording gave 0.23–0.34, but the error-correcting
code is still invisible to us, while MultiPSK reads the same file from midway. So
the fault is in our demodulator and not in anything he sends. The next step is
named and measurable: track the phase symbol by symbol after the frame's
correction, and judge it by whether the code's own metric lifts off its null.

**Done.**
- Both files kept in `~/s4285-captures/keepers/` (off-air, not in the tree).
- Measured: lock **0.899**, frame ACF **0.706**, 242 frames, data constellation
  **0.396**, references 0.55/0.61/0.51, FEC metric **0.96×** its shuffled null at
  1850 and 1900 Hz. SOM correlator **31/32** — the closest to a message yet.
- `STANAG4285-HANDOVER.md`: the table, the two findings and the next measurement.

**A mistake, recorded rather than hidden.** My commit `a558d624` swept up **25
lines of the other session's MT63 work** (`crates/sdroxide-dsp/src/mt63.rs`) under
a STANAG message, because I used `git add -A` in a tree another session is
editing. Their change is intact and unaltered in content — nothing was lost — but
it is in the wrong commit with the wrong message, and it is pushed. **In a shared
tree, `git add` takes paths and never `-A`.** They should be told.

**Not done / blocked.** The decision-directed tracker, and the baseband IQ not yet
opened (423 MB; the audio file was enough to answer today's question).

**Next.** The phase tracker on `FUF-13031.200kHz-20261024-125317Z.wav`, judged by
the trellis ratio.

---

## 2026-10-24 (map) — roads and internal boundaries, for the "too imprecise" report

**Summary.** A user on Reddit says our map is too imprecise and asks for
OpenStreetMap. The data was never the problem — it is already Natural Earth's
finest — so instead of an online map, the two layers that make a map read as a
place are now shipped: **roads** and **first-order internal boundaries**. They are
drawn only when you are zoomed in far enough to want them, so a continent view
looks exactly as it did. It is **PR #184** because it touches the map widget that
APRS also draws on.

**Done.**
- `make_earth_maps.py`: two more layers from the same source (`ne_10m_roads`,
  `ne_10m_admin_1_states_provinces_lines`), roads filtered to their major
  classes (`scalerank <= 7`), both on the finest level only. Railroads left out on
  purpose (1.7 MB, indistinguishable from roads at this map's zooms).
- `lines.bin` **1.6 MB → 4.2 MB**; `basemap::Lines` gains `roads` and `states`;
  an older two-layer file still loads.
- `worldmap`: both draw paths (stipple and painted) add them, gated at **40°** for
  boundaries and **8°** for roads.
- `survey_ground_probe` now draws a **close view**; looked at: present at 6° in
  all three styles, absent at 60°.
- Guard: `the_detail_layers_are_in_the_asset` (33k road parts, 43k boundary
  parts, roads on the finest level only).
- Gate: workspace check silent; basemap 9 and worldmap 12 green; rustfmt clean.

**Not done / blocked.** Higher land resolution (16384×8192): 88 MB of CPU
pyramid, 4× the startup decode and a texture size browsers and many GPUs refuse —
it would have to become tiles. Online OSM tiles stay out: Mercator against an
equirectangular pipeline, and the no-network / no-key rules.

**Next.** PR #184 waits; then transmit for the receive-only set, RS_ID/CALL_ID.

---

## 2026-10-24 (pause) — the debug suite, and the queue emptied into `main`

**Summary.** Three things are now on `main` that were not this morning. The UI
tests run again in debug — they had been aborting for everyone, taking 949
passing tests down with them, because of a contract in the graphics layer that a
dozen tests broke. The LISTEN tab really does hide the transmit controls now,
whichever way you open the band menu, and the reporter's issue is answered by a
build rather than by a promise. And 8PSK is in the program as a third modulation
beside BPSK and QPSK. Nothing is open: no PRs, no numbers claimed, and the wire
register sits at 243.

**Done.**
- **The debug UI suite** (`55269c28`, rebased as `5f932005`). `epaint`'s
  `TexturesDelta::drop` is a `debug_assert!` that every delta was applied; a test
  has no renderer, and twelve of them dropped a frame's `FullOutput` unread. The
  first — the DAB logo tile — panicked *inside a destructor*, which is
  non-unwinding, so the process aborted and the suite reported nothing.
  `crate::test_support::run_frame` is the idiom now: run the frame, take the
  deltas, return the rest. Twelve tests go through it, in `app/mod.rs`,
  `app/settings/general.rs`, `theme.rs` and `tooltip_copy.rs`.
  **Before: 12 failed and the binary aborted. After: 961 passed, 0 failed, 6
  ignored** in debug — the same as `--release`, which is why this had gone
  unseen: the release gate is the release one.
- **PR #183 merged** (LISTEN tab, issue #146), **PR #174 merged** (8PSK).
  #174 needed main folded in first: it conflicted in `CHANGELOG.md` and in the
  `PROTO_VERSION` register, where #177 had taken 243 under it. Resolution keeps
  both entries in order — 242 8PSK, 243 voice hunting — with `PROTO_VERSION` 243
  and `VERSION_BYTE` 0x3F, and both changelog lines.
- After the merges, on `main`: `cargo check --workspace --all-targets` silent;
  proto 18, types 792, digi 542, ui **961** all green.

**Not done / blocked.** Nothing. **Six issues are open**, and every one of them
waits on somebody else: #132/#133 (a Brick2 on the bench), #153 (a diagnostic
report), #162 (the reporter's own calibration), #105 (a stable release), and
**#146**, which is fixed and merged but deliberately left open — the PR said
*Addresses*, not *Closes*, because the man who reported it should be the one to
say it works.

**Next.** Nothing until the operator says so; he asked for a pause. The queue
stands where the modes left it — STANAG 4285's coded payload first.

---

## 2026-10-24 — issue #146: the LISTEN tab hid nothing from the popup

**Summary.** The reporter on v2.1.0.2 was right and the feature was broken. Picking
LISTEN in the band/mode menu is supposed to hide the transmit controls, and it
did that only for someone who had already opened the selector as a docked column
or a window — from the ordinary BAND chip popup, which is how everyone opens it,
nothing happened. The answer is now read from the tab itself rather than from a
flag that one of the three places drawing the menu remembered to set, so every
way in works, and the LISTEN tab says where the switch is while it is off. It is
**PR #178**, because the band/mode window is F4NRM's.

**Done.**
- `mod.rs`: `listen_tab_hides()` and the tab half of `swl_mode()` derived from
  `ui_settings.listen_tab_hides_tx && band_menu_tab == Listen`; the stored
  `listen_tab_swl` field is gone, so there is no flag left to forget. The
  remembering keeps one seam, `remember_band_tab()`, called once a frame, and it
  writes only when the answer has moved.
- `top_bar.rs`: the `before`/persist block deleted from `band_menu_body` — it was
  the only one of the three call sites that had it. `band_mode_menu` gains
  `listen_hides_tx` so the tab can say where the switch is.
- The LISTEN tab now carries one weak line while the setting is off: *"This tab
  can hide the transmit controls too: turn on "hide TX on the LISTEN tab" in
  Settings → UI"*. A hint and not a chip, because the switch asks once in
  Settings and a chip here would be a way round that confirmation.
- Manual §2.4 documents the setting, including the half that matters: it hides,
  it never grants, and a receive-only radio stays receive-only.
- **The test was the other half of the fault** and now moves the tab rather than
  setting a flag: `app.band_menu_tab = Listen` then `assert!(app.swl_mode())`.
  Verified to fail on the old behaviour ("picking LISTEN must hide them — nothing
  else called").

**Numbers.**
- `cargo test --release -p sdroxide-ui --lib` — **961 passed, 0 failed, 6
  ignored** (the whole crate, the release gate).
- `cargo test -p sdroxide-ui --lib the_listen_tab` — passes in debug and release.

**Found on the way, and not fixed: the debug UI suite does not run.** With egui
0.36, `epaint`'s `TexturesDelta::drop` is a `debug_assert!`; a dozen UI tests drop
an `egui::FullOutput` without applying or clearing its deltas, and the first of
them — the DAB logo tile — **aborts the whole test binary**, so 949 tests never
report. `cargo test --release` is green, which is why nobody has seen it: the
release gate is the release one. The fix is one `out.textures_delta.clear()` per
test that draws a frame. It is a sweep across F4NRM's UI test files, so it is in
`TODO.md` under *Now* waiting for a go-ahead rather than done here — and it is
why this entry's numbers are release numbers.

**Not done / blocked.** No click-through of the popup itself: the test drives the
tab the popup writes, not egui's pointer, and that seam is where the bug was.

**Next.** PR #178 for F4NRM to read, then back to the mode queue.

---

## 2026-10-24 (rules) — F4NRM's list is a workflow, and the RSP1 is ours

**Summary.** The list of things that belong to F4NRM has been read off the
discussion itself rather than from memory, and it was wrong in two ways. The
**RSP1 is not on it** — the operator struck it off in as many words, because it
is his own receiver and the only wideband one on the bench — so RSP1 work is ours
and needs nobody's permission. And the rest of the list is no longer a wall: a
change that has to touch one of those decoders or radios, or one of his panels,
goes up **as a PR** for him to read before it lands, which is what he asked for
in that same discussion. Nothing changes about the part that matters: no
unrequested refactor of a mode his radio group has confirmed.

**Done.**
- `AGENTS.md`: the section is renamed *"F4NRM's list — his to change, and a PR to
  touch"*, quotes the operator's own correction, drops RSP1, and states the PR
  rule with his sentence as the evidence: *"put any wire change or item that
  touches these to put up as a PR and not implement them directly without you
  having the chance to do look at them."*
- `AGENTS.md`'s stop-list for an unattended session: F4NRM's list, his panels and
  the wire now share one line — branch, tests, PR, carry on, never `main`.
- `AGENTS.md`'s focus section and `TODO.md`'s item 0 carry the same reading, so
  the queue cannot contradict the rules.
- Read from the discussion itself (GraphQL, `discussion(number:155)`), not from
  the paraphrase that was here.

**Not done / blocked.** Nothing. The list itself is still F4NRM's to change, and
this entry does not add to or remove from it beyond the operator's own correction.

**Next.** Back to issue #146 (the LISTEN tab), which is what the session was on.

---

## 2026-10-24 — voice hunting: the scanner stops on people, not on carriers

**Summary.** The scanner can now hunt voices. A **VOICE** switch in the SCAN
window makes it stop only on channels that sound like somebody talking, and go
past the unmodulated carriers, birdies, pagers and data bursts it used to stop
on. While it runs, the line under the status tells you what it made of the last
channel it looked at — *passed 27.5550 MHz · voice 18*, or *voice 82 · somebody
is talking* — so a strong signal going past is a number rather than a mystery.
The cost is time: a voice is a rhythm of syllables, so each channel is listened
to for at least half a second, and the window raises the dwell and says why.
Nothing is proven off the air yet — the detector is proven on synthetic speech
and on the engine's own bench, and a real band is the next thing it needs.

**Done.**
- **`sdroxide-dsp/src/vad.rs`** (new): `SpeechDetector`, a pure function of the
  audio handed to it — a 20 ms frame, then four measurements. **Structure**
  (spectral flatness and the share of the band's power in its strongest bin,
  which between them separate speech from noise and both from a tone),
  **voicing** (normalised autocorrelation over 60–400 Hz), **spread** (three
  sub-bands of 250–3500 Hz), and **syllables** (hysteresis rises of the frame
  loudness, 2–9 a second). Score 0–100; the verdict is **4 accepted frames in
  the last half second**, not one, and `ACCEPT` is 70 — above what structure,
  voicing and spread can add up to (65) — so a frame has to show a *rhythm*
  before it counts.
- **The two measurements that shaped it, both failures first.** A bare AM
  carrier scored **exactly 65.0 on one frame** — that is the DDC settling after
  a retune, and it made the scan stop on every hop. And a single frame at the
  threshold was enough for a verdict. Hence `ACCEPT = 70` and the 4-of-25
  window, and both have their own unit tests (`one_settling_thump_is_not_a_voice`,
  `a_steady_buzz_is_not_a_voice`).
- **Engine** (`crates/sdroxide-radio/src/engine.rs`): the chain scores its own
  demodulator audio *before* the AGC, notch, noise reduction and squelch — a
  voice is what arrived, not what the operator's settings did to it — and a
  demod-audio rig (CB on a sound card) is scored in `play_rx_audio` through a
  second detector. `scan_is_busy()` is now level **and** voice; the settle is
  `dwell.max(MIN_LISTEN_MS)` when the gate runs; the detectors reset when the
  offset moves or a candidate is retuned to; and `voice_gate()` has three
  answers — off, running, and *nothing to hear* (a mode that carries no speech
  is passed over, because the operator asked for voices).
- **Refusals, not silent fallbacks**: starting a voice hunt with no audio, or
  with nothing in the scan in a mode that carries speech, is refused with a
  notice, and the SCAN window greys the button and says the same thing before
  START is pressed.
- **Wire** (`crates/sdroxide-proto/src/lib.rs`): `ScannerConfig.voice_only` and
  `ScanState.voice` (`Option<ScanVoice>` — a whole-hertz frequency with the
  score, so the reading always names its channel), both appended,
  `#[serde(default)]`. **243 / 0x3F**, having found 242 already claimed by
  PR #174 (`fork/psk8`) — the later branch moves, as the register says.
- **Tests, with the numbers.**
  - `cargo test -p sdroxide-dsp --lib vad` — **15 passed**.
  - `cargo test -p sdroxide-radio --test scanner` — **15 passed**, three new:
    a bare carrier that stops the scan with the gate off and is passed over with
    it on (and its reading is on the wire, < 45); a voice-modulated carrier that
    stops it (score ≥ 60); and a voice hunt over `Mode::Ft8` refused with a
    notice.
  - Both new engine tests were run against the unfixed code and fail there:
    gate disabled → "an unmodulated carrier was taken for a voice"; refusal
    removed → "a voice hunt over a mode with no voice should not start".
  - `cargo check --workspace --all-targets` silent; `cargo check --release
    --target wasm32-unknown-unknown -p sdroxide-ui` clean (its 277 dead-code
    warnings are the pre-existing wasm-only ones).
  - `rustfmt --edition 2024 --config-path rustfmt.toml --check` clean on every
    file touched; `sdroxide-types/src/lib.rs` was already dirty in its module
    order and was left alone.
- **Not borrowed.** The idea came from VUURWERK (Quansheng UV-K5 firmware), but
  nothing was taken from it: its licence puts a commercial licence on its
  originals — the voice-probability scoring among them — and this fork takes no
  paid anything. This is textbook DSP written here, and a reviewer who compares
  the two will find no shared code.

**Not done / blocked.**
- **Nothing has been heard off the air yet.** The detector is proven on
  synthetic speech, a synthetic buzz, noise, AFSK and the engine's AM bench; a
  real band — 11 m at night, or a broadcast band full of carriers — is the test
  that would settle the thresholds, and it needs the bench, not a session.
- The **`audio_mode`/external-audio path is untested end to end** (the CB rig
  through its USB card). Its detector is fed and its guard is in place, but no
  test drives a demod-audio source through a voice hunt; that needs the SS9900v.
- Music with a vocal scores as voice, by design; documented in the manual rather
  than "fixed".

**Next.** The mode queue: STANAG 4285's coded payload is still the programme's
head, and the rest of the half-done list behind it. This item needs the operator
(or F4NRM) to merge the PR first, since it is the wire.

---


**Summary.** The four wide multi-carrier PSK modes are not going to be fixed for
now, and the reason is written down where it matters. They are still in the
picker, but their chips now say on the hover that they do not decode yet and why,
and the manual's PSK section names all four and tells you to reach for a narrower
bank instead. So nobody picks one, hears nothing, and blames the band.

**Done.**
- `panels/text_modem.rs`: the four wide chips carry the note — "this wide bank
  does not decode yet … the narrower banks do read their own recordings".
- Manual §3.3: a paragraph naming PSK125C12, PSK500C4, PSK500RC4, PSK1000C2 and
  pointing at PSK250C6 / PSK500C2 for a wide signal.
- `TODO.md` → **Parked, with the reason**, carrying the measurement, the ruled-out
  theory, the gate to use, and the condition for un-parking (somebody hears one
  on the air).
- Non-wire: no `PROTO_VERSION`, no mode change, nothing to merge.

**Not done / blocked.** The front-end rebuild itself, parked by the operator's
decision on the advice given: four rare wide banks are not worth risking the
twenty that work.

**Next.** The remaining mode work is transmit for the receive-only set and
RS_ID/CALL_ID; the rest of the unfinished list waits on recordings.

---

## 2026-10-24 (next item) — the four wide PSK banks: measured, and the fix is a refactor

**Summary.** The next item on the queue was the four wide multi-carrier PSK modes,
the last of the coded PSK family that does not read its own recording. They still
do not, and the reason is now narrower than it was: every bank wider than about
two kilohertz fails (0 of 3 known words) while every narrower one reads its whole
text, and making the per-carrier filter four times sharper changed **nothing** —
so it is not filter sharpness. What is left is the reference's own chain order
(decimate first, then filter), which is a rebuild of the receiver's front end
rather than a parameter change.

**Done.**
- Baseline, `cargo test --release -p sdroxide-dsp --lib the_corpus_pskc --
  --ignored --nocapture` (69 s): **PSK500C4 0/3, PSK1000C2 0/3, PSK125C12 0/3,
  PSK500RC4 0/3**; PSK500RC3 1/3 and PSK250RC5 1/3; everything narrower 3/3.
- The failing set is exactly the widest banks: 2.1 kHz (PSK125C12) and 2.8 kHz
  (the other three). PSK500C2 — same baud, same 700 Hz spacing, only two
  carriers — reads 3/3, so the spacing alone is not the fault.
- Tried a filter sharpened with bandwidth (tap count from `4·rate/half`, 129 →
  512 at 125 baud): **identical results**, reverted. It is not the filter's
  transition width.
- `psk.rs` restored to its committed state (`git status` clean before the next
  step).

**Not done / blocked.** The fix needs the front end rebuilt in the reference's
order — decimate each carrier to a few samples per symbol, *then* filter there —
instead of filtering at the 8 kHz input rate. That is a structural change in
`PskRx`/`PskCarrierBank` with the timing loop and the differential detector on
top, not a session-sized edit, and it should not be attempted as a tail-end
change to a long session.

**Next.** Rebuild the multi-carrier front end in that order, with the existing
`the_corpus_pskc_samples_are_measured` as the gate: the target is those four
banks moving off 0/3 without the 3/3 ones regressing.

---

## 2026-10-24 (last) — 8PSK is in the program, and the ALE watch is stopped

**Summary.** All three modes the operator named are now in the program. ALE and
CIS-11 were already wired; **8PSK** is now a third choice beside BPSK and QPSK on
the keyboard modes' modulation row — receive only, three bits per symbol. Its
proof is a synthetic, because the test recordings for it turned out not to hold a
clean 8PSK signal at all. The unattended ALE channel sweep was stopped as asked;
it had finished four rounds over eleven channels with nothing heard.

**Done.**
- `PskModulation::Psk8` appended, wired into the plain PSK receiver, refused on
  transmit with a reason, chip and hover text come from `ALL` so no layout
  changed. Commit `41c83c87`, branch `fork/psk8`, **PR #174** (wire change:
  **`PROTO_VERSION` 241 → 242, `VERSION_BYTE` 0x3D → 0x3E**).
- Manual mode-table row and changelog line say "receive only, proven on a
  synthetic".
- Gate: workspace check silent; types 792, digi 542, ui 961; rustfmt clean.
- ALE sweep: `pgrep -af "watch.py|rspcap"` → nothing. Its log's last four lines
  are `quiet` on every channel, four rounds, ~33 minutes.

**Not done / blocked.** 8PSK has no recording and no 8PSK modulator; the manual
says both. A reference recording would turn "proven on a synthetic" into proven.

**Next.** PR #173 (CIS-11) and #174 (8PSK) both wait on the operator's merge; the
next unwired item is the four wide `C` PSK banks, whose recordings are good
(0.83–0.85 of the bits) and whose fault is the demodulator's carrier separation.

---

## 2026-10-24 (later) — 8PSK: the instrument is sound, so the files are the fault

**Summary.** The question from the entry below is answered. Built a synthetic
8PSK of the right shape and measured it with the same instrument that reads the
test files: the synthetic scores **0.99** where the files score 0.44–0.58. So the
measurement is not what was wrong and neither is the width — **the four 8PSK test
recordings do not contain a clean differential-8PSK constellation**, and no
receiver written against them could have worked. The mode's slicer is proven on
our own synthetic; what it needs now is one genuine recording, or being shipped as
"proven synthetically" like the others.

**Done.**
- Synthetic 8PSK at 125 baud, symbols shaped by a 201-tap low-pass at 0.5 × baud
  (the α≈0 shape the corpus's own −3 dB width implies), measured by the same
  chain used on the corpus (mix, 300 Hz LPF, decimate to 8 sps, differential
  phase): **8-fold concentration 0.99**, 2-fold 0.04.
- The same chain on `samples/fldigi/8PSK125.wav`: 0.44–0.58 at every symbol
  phase, collapsing onto 0°/180° — a closed eye, not an 8PSK.
- Matched filtering (one-symbol box, raised cosine α = 0.35) does not help the
  file: 0.33 and 0.35 against 0.47 unfiltered. It is not a filter-order fault.

**Not done / blocked.** What the four files are, and whether our receiver reads
the **shaped** synthetic (the next run: write it to a WAV and feed `PskRx` with
`set_psk8(true)`; the existing test uses rectangular symbols and passes).

**Next.** Run the shaped synthetic through the real receiver, then wire 8PSK as a
mode on a branch — the wire change — with "proven synthetically, no working
recording" in the manual, exactly as ALE and CIS-11 are stated.

---

## 2026-10-24 — 8PSK: the recordings do not say what we thought

**Summary.** No change in the program. The mode that has been stuck — the
8-phase PSK family, which fldigi transmits and we cannot read — turned out to be
stuck for a reason we had wrong: the notes said those test recordings are only
30 Hz wide, which would mean they cannot carry the mode at all. They are not.
Measured properly they are exactly the width the mode should be, at a normal
level and the right speed, so the fault is not a band-limited recording. What is
still true is that their phase never settles for us, and our own decoder reads
nothing from them either. Before another receiver gets written, the next step is
to build the signal ourselves the way fldigi does and compare the two by the same
measurement — or to get one real recording of the mode.

**Done.**
- Re-measured the corpus 8PSK files (`samples/fldigi/8PSK*.wav`):
  **8PSK125 is 124.5 Hz wide at −3 dB** (BPSK125: 125.0 Hz), carrier 1806.6 Hz,
  in-band SNR ≈ 28 dB — the same level and SNR as the BPSK and QPSK files beside
  it. **Baud line 125.0 Hz**, harmonics at 250 and 374 Hz (spectrum of the phase
  derivative). The old "20–46 Hz wide" note was a faulty width measurement
  (thresholds read above the peak instead of beside it).
- Differential-phase concentration at the right rate: BPSK125 **0.87** (2-fold),
  QPSK125 **0.74** (4-fold), every 8PSK file **0.44–0.58** at any symbol phase.
- Our own BPSK decoder reads **nothing** from `8PSK125.wav` (PskRx at 125 baud over
  1650–1950 Hz), so the 0°/180° histogram is a **closed eye**, not a BPSK file.
- `docs/internal/MODE-PROGRESS.md` carries the corrected table and the two
  candidate explanations; commit `4e7abd93`.

**Not done / blocked.** What the 8PSK files actually are. Two answers would
settle it and neither is a guess this bench can make: a **synthetic 8PSK built the
way fldigi builds one**, compared against the file by the same statistic, or a
**reference recording** of the mode. Nothing is wired to the panel.

**Next.** Build the synthetic comparison (it is a Rust test next to the existing
`a_synthetic_8psk_transmission_reads_back`, which already proves the slicer on our
own waveform — the missing half is doing it under fldigi's shaping; if the
synthetic settles and the file does not, the file is the fault and the item waits
for a recording).

---

## 2026-10-24 — CIS-11: a new mode, measured from the recordings before it was built

**Summary.** There is a new mode in the DIGITAL row, **CIS-11** — the Russian
meteorological teleprinter that listeners meet between 4 and 18 MHz. Leave the
dial on a channel and it finds the two tones by itself and prints what it locked
to, with a count of how many characters passed their parity check; on a real
recording that is 121 of 123. What it cannot yet do is turn those characters into
readable letters — that layer of the protocol is not known, and the traffic on
these frequencies is encrypted anyway. It is receive only. It is on a branch as
PR #173 because it changes the protocol version, so it is waiting on your merge.

**Done.**
- The receiver (`sdroxide-dsp::cis11`): finds its own tones, tracks the character
  clock, decodes the 11-bit character. Commit `be6090ca` plus the wiring in
  `a0c56a2f`; branch `fork/cis11`; **PR #173**.
- The **framing measured, not assumed.** The two catalogues say "5 data +
  parity"; the idle word looks like 5 + 5 + 1. Scored every grouping from 4 to 16
  bits and every rotation by parity consistency: **11 bits at 85 %**, repeated
  character **13 %**, and the traffic's halves agree 5 % — so it is **ten data
  bits and one odd-parity bit**.
- **Off-air**, idle recording: tones 742/1242 Hz, **123 characters, 121
  parity-valid**; traffic: 199 characters, 123 clean. Command:
  `SDROXIDE_CIS11_SAMPLE=/tmp/opencode/cis11/idle.wav cargo test --release -p
  sdroxide-dsp --lib cis11 -- --ignored`.
- Wiring: `Mode::Cis11` (discriminant 62), `Cis11Controller`, the shared
  text-modem panel (no new UI), manual §3.30, changelog.
- Wire: **`PROTO_VERSION` 240 → 241, `VERSION_BYTE` 0x3C → 0x3D**, register entry.
- Gate: workspace check silent; types 792, digi 542, ui 961 green; rustfmt clean.
- Two pins fixed on the way: `DominoEx` and `Dtmf` were never pinned, and the
  `Mode::ALL` count test took its last discriminant from `Dtmf`.

**Not done / blocked.** The character layer: how the ten data bits divide into
letters. Needs a **clear** transmission (Russian meteo is sometimes sent in clear
M2 Cyrillic) or a reference decoder to compare against; the encrypted traffic
cannot settle it. Also unmeasured: the 400 Hz shift variant, which the receiver
searches for but no recording of it has been tested.

**Next.** PR #173 waits on the operator. Then the queue in `TODO.md`: the modes
that did not finish in 2.1.0 (8PSK's vestigial filter, the four wide PSK banks)
and ALE's recording ask.


## 2026-10-24 — The standing order for working unattended

**Summary.** Nothing in the program changed. The notes the agent works from now
say to keep going on its own when the goal and the boundaries are clear, to write
what happens in this journal as it happens, and to keep the TODO list and the
project board current. When an item is finished you will get a short plain
summary without having to ask for it.

**Done.**
- `AGENTS.md`: new section "Working on your own — the standing order" (the four
  things that make a task clear, the list of things that stop an item, the
  journal, the TODO/board updates, and the completion summary), and the header
  now names this journal.
- `docs/internal/AUTONOMY-JOURNAL.md`: this file, with its format.
- `docs/internal/README.md`: added to the file table.
- Docs only — no crate, no wire, no build. Nothing to run; `cargo check` not
  needed for a markdown change.

**Not done / blocked.** Nothing.

**Next.** The live queue is unchanged: the mode work, STANAG 4285 first —
[`MODE-PHASE-4-PROMPT.md`](MODE-PHASE-4-PROMPT.md) and
[`MODE-PROGRESS.md`](MODE-PROGRESS.md).

---

## 2026-10-27 (the DigiConfig wire fields are re-appended at the true tail)

**In plain words.** The five fields the composer and the auto run added to the
station's digital config — banner-bottom, banner text size, the station overlay,
auto-call retries and the auto session limit — were dropped into the *middle* of
that struct even though every one of their notes said "appended last". They now
sit at the real end, so the note and the code agree. It's a wire change, so the
wire number moved to 256.

**Done.**
- Branch `local/digiconfig-append-tail` (off `main`), commit `55e09550`.
- Moved `sstv_banner_bottom`, `sstv_banner_text_pct`, `station_overlay`,
  `auto_call_retries`, `auto_session_min` to after `wefax_rx_auto` in
  `DigiConfig` (`crates/sdroxide-types/src/digi.rs`), so `sstv_rx_sensitivity` /
  `sstv_rx_auto` / `wefax_rx_sensitivity` / `wefax_rx_auto` no longer shift down
  in the positional postcard layout.
- Bumped the wire: `PROTO_VERSION` 255 → 256, `VERSION_BYTE` 0x4B → 0x4C, with a
  v256 register entry in `crates/sdroxide-proto/src/lib.rs`.
- Green: `cargo test -p sdroxide-proto --lib roundtrip` (14/14), `cargo test -p
  sdroxide-types --lib` (815/815), `cargo check --workspace --all-targets`
  (silent apart from pre-existing warnings), and
  `cargo check --release --target wasm32-unknown-unknown -p sdroxide-ui`.

**Not done / blocked.** It is a wire change, so it waits for the freeze to lift
and goes up as a PR (never straight to `main`). Not pushed.

**Next.** When the freeze lifts, push the branch and open the PR; F4NRM reviews
it like any wire change.
