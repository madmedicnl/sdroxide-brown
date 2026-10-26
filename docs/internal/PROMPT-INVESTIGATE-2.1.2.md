# Prompt — find what makes 2.1.2 "buggy" against v2.1.1.1

Paste everything below the line into a fresh session (any model). It is written to
be self-contained: the tree, the reports, what has already been measured and ruled
out, and the leads that are worth the first hour. Everything it needs is local; no
radio, no browser and no network are required except where it says so.

---

You are investigating a regression in the SDR receiver/decoder program in this
repository (`sdroxide-brown`, a fork of `sdroxide`). Read `AGENTS.md` first — it is
the operator's rulebook and several of its rules bear directly on this work — then
this brief. Work read-only until you have a mechanism; then produce the smallest
patch on a **local branch**.

## The question

**Between the build tagged `v2.1.1.1` and `main`, what changed that makes the
program behave wrongly?** The operator runs both builds himself, keeps the older
one, and one of his testers ("F4NRM") is hitting several faults on the newer one
that we cannot yet reproduce. Find the mechanism, name the commit, and prove it
with a test that fails on the old code.

Faults reported (all on the newer build unless noted):

1. **The panadapter will not pan or zoom.** F4NRM, issue #263, verbatim: *"Not
   work. Always at this place. Moving ou zooming is impossible. (all modes). CTR
   are now problem."* Two screenshots: one at 127.550 MHz in AIR-AM with the
   visible span about **1.124 MHz**; one at 7.156 MHz in SSTV with the visible
   span **48 kHz**. In both, the picture is live (timestamps advance) and the
   gestures do nothing.
2. **Clicking Copy in the SSTV panel froze the whole window** (same session, same
   client). The SSTV panel and its picture composer are the work merged as **#240**.
3. **SWR reads 3.5 while his external tuner shows 1.2.**
4. **After he downgraded to `v2.1.1.1`, FT8 no longer syncs.**
5. **The operator's own, brand new:** on his bench, native Linux, he switched his
   transceiver's panadapter to the **RSP1 lent as that transceiver's panadapter**
   (`PanadapterConfig::source_radio`) and **the zooming is weird**. This is the only
   live reproduction we have — start here.

**Two variables differ between the reporter and the bench, and they matter:** his
client is the **wasm build in Chrome on Windows** (installed PWA, "full remote"
against a server on **Debian**); ours is **native on Linux**. Anything that exists
in only one of those (clipboard, pointer/wheel semantics, device pixel ratio,
single-threaded wasm, event timing, the pairing of two radios) is a candidate.
His station: radio 1 is a **Kenwood TS-2000 on CAT with the rig's own audio**,
radio 2 is a **HackRF lent as radio 1's panadapter**. He also owns an RSP1A (on a
Mac), a PlutoSDR and an RTL-SDR.

## The range, exactly

```
main                    f0678166   crate 2.1.2, PROTO_VERSION 255 / VERSION_BYTE 0x4B
working build  tag      v2.1.1.1   2fc129fc   (2026-10-26 00:31)
the whole suspect diff: git diff v2.1.1.1..main
```

Six pull requests landed in that range. What each one touched, measured with
`git diff --stat v2.1.1.1..main`:

| PR | what it is | files |
|----|------------|-------|
| #240 | SSTV/RIFP picture composer, its document on the wire (249→254) | `engine.rs` (+41: six `Image…` command arms), `image_store.rs`, `Command`, `ImageSlotInfo`, `DigiConfig`, `panels/sstv.rs` (+1257), `sstv_composer.rs` (+898), `sstv.rs` |
| #253 | AUTO RX / AUTO CQ roles, a bound on a CQ run (wire 255) | `auto.rs`, `digi.rs` (`auto_call_retries`, `auto_session_min`), `auto_mode.rs`, `panels/decodes.rs`, `panels/setup.rs` |
| #261 | windows for a tiling WM, module-window list, workspaces | `frame.rs` (+648), `mod.rs` (+469), `top_bar.rs` (+308), `layout.rs` (+338), `chrome.rs` (+104), `multi.rs` (+39), `ui.rs` (+16) |
| #255 | one press centres a detuned weather chart (the fax CTR anchor) | `frame.rs` (`centres_on_picture`), UI only |
| #254 | the scan's voice gate gives the dwell back | `scanner.rs`, UI only |
| #241 | the shared logbook is not wiped when there is no store to re-read | `persist.rs`, UI only |

## What has already been measured — do not re-derive

- **`v2.1.1.1` already contains #234** (the Airspy HF+ display-span change,
  `433a2d74`): `git merge-base --is-ancestor 433a2d74 v2.1.1.1` is true. The
  handover at the time blamed that merge for the pan/zoom fault; it cannot be the
  cause, because the build the tester calls fine has it.
- **The panadapter's own code is byte-identical between the two builds.** This is
  empty, and it is the single most important fact here:

  ```
  git diff --stat v2.1.1.1..main -- \
      crates/sdroxide-ui/src/widgets/ crates/sdroxide-ui/src/app/spectrum.rs \
      crates/sdroxide-ui/src/view.rs
  ```

  So the pan, zoom, clamp, CTR and fit arithmetic is unchanged. If the panadapter
  misbehaves now and worked then, either the numbers it is fed changed, or
  something *around* it changed (layout, overlay, input), or the fault was always
  there and this build is the first to expose it.
- **No changes at all** in the range to: `crates/sdroxide-cat` (the whole rig
  protocol, including the Kenwood meter curves), `src/panadapter_source.rs` (the
  two-radio pairing), `crates/sdroxide-radio/src/source.rs`, `device.rs`,
  `crates/sdroxide-types/src/state.rs`, `caps.rs`, `crates/sdroxide-server`,
  `crates/sdroxide-web`, `src/`.
- A headless probe of the real app (capabilities, a snapshot, a frame, then a real
  Shift+wheel and a real 120-point left-drag on the panadapter found by its own
  background colour) was run over ten span configurations — plain SDR zoomed in
  and out, HF+ 0.86 usable span, a 48 kHz state against a 1.1 MHz frame, an audio
  rig, a CAT rig at 4.8 kHz, a wide lane over narrow I/Q, a `usable_hz` larger
  than the rate, and no state at all. **Pan and zoom worked in every configuration
  but the no-state one** (a zero-width view). The probe is a throwaway; a copy is
  at `/tmp/opencode/probe_263_keep.rs` (315 lines, a single test named
  `probe_263_pan_and_zoom` in `crates/sdroxide-ui/src/app/mod.rs`'s test module).
  Reuse it rather than rebuilding it — but do not assume it answers the pairing
  case, which is the one the operator now has on his bench.
- The one question already asked of the reporter: *when he rolls the wheel over
  the waterfall, does the frequency change?* That separates "the gesture never
  reaches the widget" from "the widget receives it and refuses". No answer yet.

## Leads worth the first hour, in order

1. **The lent-panadapter zoom path (the operator's own reproduction).** With a CAT
   rig as the control radio and an RSP1/HackRF/IQ-file lent as its panadapter, the
   caps say `audio_mode = false`, `center_is_dial = false`, so `WindowPan.movable`
   is true and a pan at full zoom-out is handed to the *lent receiver's* centre —
   while the engine also pushes the VFO through
   `PanadapterSource::set_if_offset`, which moves the **transceiver's dial** when
   `track` is on. Look at what a zoom or a continued drag does to: the rig's dial,
   `keep_vfo_in_span`, `follow_view_into_the_band`, `ctr_settle`,
   `center_on_dial`, and the `usable_hz` the lent receiver reports (a SoapySDR
   source reports no `usable_frac`, so `usable_hz == sample_rate`). Reproduce it
   natively first: any CAT or audio source plus an IQ-file source as its
   panadapter; no radio needed for the arithmetic.
2. **The SSTV Copy freeze (#240).** Trace the Copy button in
   `crates/sdroxide-ui/src/app/panels/sstv.rs` and `sstv_composer.rs`. Two shapes
   to check: (a) a command → event → write-back loop, because
   `ImageSetComposer`/`ImageSetOverlay`/`ImageSaveTemplate` all call
   `Engine::emit_image_presets`, which sends **every preset with its whole
   document** back to every client — if the panel writes on what it receives, two
   sides can ping-pong; (b) per-frame cost in the panel (rebuilding or
   re-rasterising a document every frame). Both are worse in a single-threaded
   wasm client.
3. **#261's layout registers.** `layout::set_band_dock_room` / `classic_column` /
   `tier` / `ViewportScope` moved answers from app fields onto the egui context.
   This is the only new code that decides *what is drawn where*, it is shared with
   the browser, and it is the prime suspect for a gesture never reaching the
   panadapter (something drawn over it, or a stale register). Check, specifically,
   that every register set in a frame is either reset or re-derived in the same
   frame on every path, including the browser's.
4. **Wheel/modifier semantics in a browser.** `spectrum_view` reads the frame's
   modifier state (`ui.input(|i| i.modifiers.shift)`), not the wheel event's own.
   In Chrome a Shift+wheel is a horizontal page gesture. The widget code has not
   changed, but anything new that consumes scroll before it (a scroll area, a
   panel) would kill zoom while leaving pans working — or vice versa.
5. **SWR (3.5 against a tuner's 1.2).** The reading is the rig's own `RM` meter
   bars converted through a per-model curve chosen from its `ID;` reply
   (`crates/sdroxide-cat/src/kenwood.rs`: TS-2000 = `(12 bars → 3.0)`, TS-590 and
   TS-890 differ by 2×). None of that changed in the range, so if this is real it
   is either a *crossed reply* on the wire (two pollers on one port) or not a
   regression at all. Do not fix from the symptom; establish whose meter it is.
6. **FT8 not syncing after he downgraded.** A downgrade that reverts the binary
   and leaves a fault means the fault is outside the binary: the browser client
   Chrome caches (the server refuses a wire mismatch by name — `session.rs`
   compares `PROTO_VERSION` on `Hello`), the station's own config files, or the
   audio path. Note for the record: config files are JSON and unknown fields are
   ignored, so a newer `digi.json` does not break an older build — but a file the
   old reader cannot parse is quarantined and the program starts on defaults
   (`load_json_migrated`), silently.

## Tools that exist, so you do not rebuild them

- Headless full-app harness: `SdroxideApp::new_tab(&ctx, None, None, controller, 0,
  true)` with a `RadioController` whose `poll_event` hands back `RadioEvent`s, then
  `crate::test_support::run_frame(&ctx, RawInput { … }, |ui| app.ui(ui, &mut
  eframe::Frame::_new_kittest()))`. `egui::Event::ModifiersChanged` is how you set
  Shift/Alt for a frame (a `RawInput` field does not exist). Existing examples:
  `crates/sdroxide-ui/src/app/mod.rs`'s test module.
- Offscreen rasteriser for looking at a screen: `crate::test_raster` plus the
  `#[ignore]`d `*_preview` tests (see `module_windows_preview`). The rule is: for
  anything drawn rather than computed, render it and look at it.
- `cargo test -p sdroxide-ui --release --lib <name> -- --nocapture` is the fast
  loop; name the test. `cargo check --workspace --all-targets` must be silent, and
  is the gate. There is **no browser on this machine**: the wasm client can only be
  `cargo check --release --target wasm32-unknown-unknown -p sdroxide-ui`ed, so any
  browser-only fault must be argued from the code and, where possible, emulated in
  the native app (the UI code is the same).

## Rules that bind this work

- **Nothing merges, nothing is released, no PR is opened.** The repository is
  frozen on the operator's instruction while this runs; work on a local branch
  (`local/…`) and stop there. The 2.1.2 download has been withdrawn already.
- **The wire is append-only and a wire change is a PR**, never a direct push —
  `PROTO_VERSION`/`VERSION_BYTE` in `crates/sdroxide-proto/src/lib.rs` is one
  counter shared with another person's sessions.
- **A passing test is evidence about the test.** Every fix carries a test you have
  watched fail on the unfixed code.
- **A measurement that disagrees with a report is the first suspect.** Ask in this
  order and stop at the first "no": is it visible in the reporter's words taken
  literally; does the instrument measure the thing reported; does anything in the
  tree already have a number that disagrees; is something else fighting the fix.
- **Ask the reporter one thing at a time**, and do not re-open his decoders or his
  panels as tidying work — a *report* against one is a new report, to be
  reproduced.
- `rg -n`, never `rg -rn`. No repo-wide `cargo fmt`. Do not touch `vendor/`.

## What to hand back

A written finding, short and concrete: **the mechanism, the commit that
introduced it, the failing test, the fix, and what you did not test.** If the
mechanism turns out to be *outside* the binary — the client Chrome caches, the
config files, the audio path — say so with the evidence that rules the code out,
and say what measurement would settle it. A clean "not in these files, here is
why" is a result; a guess dressed as a fix is not.
