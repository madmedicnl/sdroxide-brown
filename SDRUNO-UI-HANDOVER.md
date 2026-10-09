# SDRuno-style UI — the window map and the todo (handover)

**Fresh starting point (2026-10-09), read this before the SDRuno-shaped UI work.**
The operator wants this program's window layout to read like **SDRuno**: a set of
windows, each its own module, that the operator arranges. The undocked mode
(`UNDOCKED-HANDOVER.md`) is the machinery; this file is the **shape** being aimed
at and what is left. It exists because the operator expects to **switch models
when the work reaches MAIN SP** — so note the split below.

**Where the work stands: the band keypad (item 1) is built, tested and gated, and
the next item is MAIN SP — the model-switch seam. Nothing has been started past
it, deliberately.**

## SDRuno's windows, from the manual (v1.4, ICAS)

Source: `https://icas.to/sdrplay/SDRuno/sdruno-usermanual-jpn-1a-140.htm`
(Japanese; the operator translates it). The window set:

1. **MAIN** — controls all of SDRuno. Its buttons open the rest: **SP1**, **SP2**,
   **RX**, **SCANNER**, **REC PANEL**, **MEM PAN**, plus SETT., PLUGINS, SAVE WS.
2. **RX Control** — frequency readout, mode, filter, NB/notch, AGC, squelch,
   volume, the **band keypad**, and SETT.
3. **MAIN SP (SP1)** — the main spectrum + waterfall.
4. **AUX SP (SP2)** — a second spectrum + waterfall.
5. **PLUGINS** — the decode plugins.
6. **RECORDER** — record/playback.
7. **MEM. PANEL** — memories.
8. **SCANNER**.

## Our mapping

| SDRuno | ours | state |
|---|---|---|
| MAIN | the `MultiApp` shell + the radio tab strip | exists (no separate window) |
| RX Control | the **Controls** module (`sdroxide-controls`) | built: strip, band keypad, band/mode list |
| MAIN SP (SP1) | the **Panadapter** module (`sdroxide-panadapter`) | built, **no toolbar yet** |
| AUX SP (SP2) | a second panadapter window | not built |
| SCANNER / RECORDER / MEM. PANEL / … | the **tool windows** with the ⇱ WINDOW chip | mechanism done; ~13 tools converted, the rest need their bodies extracted |
| PLUGINS | the decoders are modes, not plugins | n/a |

## What is done (the machinery)

- **Module registry + shell-owned window manager** (`UNDOCKED-HANDOVER.md`): four
  docked modules — Panadapter, Panel, Controls — each with a `detached_spec`,
  emitted by the shell; geometry persisted; close-to-dock.
- **Tool windows**: a general `tool_window(ctx, id, title, size, open, body)`
  draws an egui window or its own OS window; a ⇱ WINDOW/DOCK chip toggles it.
  Converted: scanner, DRM, morse, signal id, contest, bands, known stations, RDS,
  HD Radio, Enigma, ISM, mail, satellite.
- **The Controls console** is SDRuno's RX control in shape: desktop strip (never
  the phone strip) + the band/mode selector beneath it; the digital modes are one
  **DIGITAL ▾** dropdown; the emptied main window shows a plain face, not the
  selector a second time.
- **The band keypad** (2026-10-09) — the console's left column: **Bands**/**MHz**
  above, ten keys in calculator order, **Clear**/**Enter** below, with the full
  band list beside it. The manual describes it under **Undocked**.

## TODO (in order)

1. ~~**The band keypad.**~~ **BUILT (2026-10-09)** — `top_bar::band_keypad` and
   the console's left column, with the reasoning below.
2. ~~**MAIN SP — switch model here.**~~ **BUILT (2026-10-09)** — the panadapter
   window now carries an SDRuno-style **toolbar**: the frequency readout (with the
   mode and S-meter) in a bar across the top instead of floating over the middle
   of the waterfall, a **DISP** chip opening the same layer menu the main window's
   SPEC chip does, and a **DOCK** chip that brings the window home. See the note
   below. The operator's "switch models here" was honoured by handing this slice
   back as a self-contained commit rather than a sprawling one.
3. ~~**AUX SP (SP2)** — a second panadapter window.~~ **BUILT (2026-10-09)** —
   `DetachableModule::AuxPanadapter`, its own app-id, its own window, its own
   SP1 toolbar. See the note below.
4. ~~**The MAIN window's buttons.**~~ **BUILT (2026-10-09)** — the **🪟 WINDOWS**
   chip on the radio-tab strip: the four module windows by their SDRuno names,
   each a toggle, plus **DOCK ALL WINDOWS**. See the note below.
5. **The rest of the tool windows** — extract each body to `_body` and call
   `self.tool_window(...)`: schedule, logbook, spots, awards, grid tracker,
   public SDRs, recordings, SWL log, memories, voice keyer, and the per-mode
   setup windows.
6. **Workspace save/recall** — SDRuno records up to ten named workspaces
   (Ctrl+W / SAVE WS). Ours persists geometry per module but has no named
   workspaces and no "save the whole arrangement". A `UiSettings` list of named
   window layouts is the shape.

## The band keypad as built (2026-10-09) — and where it deliberately stops

- **A widget of its own in the console**, not a part of `band_mode_menu`: the
  popup and the dock are narrow and the pad is not. It is drawn by
  `show_detached_module`'s `M::Controls` arm, beside the band list, and it is
  **console-only** (`#[cfg(not(target_arch = "wasm32"))]` throughout) — the
  browser has no console.
- **Digits, calculator order** (`KEYPAD_ROWS`): `7 8 9 / 4 5 6 / 1 2 3`, then `0`
  centred alone under them. **The bands rise with the digits** — `1` is 160 m and
  `9` is 11 m — so the pad reads up the way the dial does.
- **The ten bands, and what is not on the pad.** A calculator grid has ten keys.
  They are the harmonic HF allocations; **60 m** (a 15 kHz secondary allocation)
  and everything above 6 m are not on it. 11 m is, because it is this program's
  band — it took the `9` key, which is why 60 m rather than 11 m is the one left
  off.
- **The digit-to-band table is ours.** SDRuno's own assignment was not published
  in anything we could read, and a table invented and then described as theirs
  would be a claim nobody checked; what is documented here is the *ordering*
  rule, which is the part a muscle memory forms on.
- **MHz mode types a frequency in kilohertz**: `14074` is 14.074 MHz, six digits
  deep, `ENTER` sends it and hands the keys back to the bands, `CLEAR` empties it
  and stays in MHz. ENTER is refused, with the range named, when the radio
  publishes a receive range the typed frequency falls outside.
- **The band list stays beside the pad, and that was the decision to make.** It
  was **not** moved behind a **MORE BANDS ▾** dropdown: our list carries 28
  bands — CB, the VHF/UHF allocations, the microwave bands, the broadcast
  services — and hiding them behind a dropdown to put ten keys where they were
  would make them *harder* to reach than they are today. The **DIGITAL ▾**
  dropdown exists because three dozen chips overflowed a row, not because a
  keypad replaced them. The two stack below `keypad_side_by_side_w(ui)`, which is
  the list's own wrap threshold — the same constant `band_pad` reads, so the two
  cannot drift apart.
- **One press, one meaning**: `band_jump_target` is now the single place that
  decides what pressing a band does in the current mode, and both the list's
  chip and the keypad's key go through it.
- **Tests** (nine, `top_bar::tests`): one band per digit and one digit per band;
  the bands rise with the digits and the rows are a calculator's; a key is a band
  in one mode and a digit in the other; the entry is kilohertz and six digits;
  ENTER tunes and returns the pad to the bands, an empty ENTER tunes nothing;
  CLEAR empties without leaving the mode; the readout says which of the two it
  is; no key is narrower than its own label (measured against the style — the
  Terminal theme's brackets are 14 pt of key width apiece); and **a render test**
  that ten keys land in `3,3,3,1` rows with `0` centred and nothing painted past
  the column. The render test and the width test were both verified failing on
  wrong code (`KEYPAD_KEY_AIR = -40`, and `0` moved to the left column).

## MAIN SP as built (2026-10-09) — the toolbar

- **The bar.** `sp1_toolbar` (in `frame.rs`, console-free, native-only) draws a
  fixed-height (`SP1_TOOLBAR_H = 32 pt`) header across the top of the undocked
  panadapter window, and the spectrum is drawn **below** it. The frequency readout
  that used to float over the middle of the waterfall — `centred_detached_readout`,
  now deleted — sits in the bar with the mode and the S-meter beside it.
- **DISP** opens the **same** layer menu the main window's SPEC chip does
  (`layers_button` → `panadapter_controls`), so SP1's toggles are the program's
  own rather than a second copy. It needed one visibility change:
  `layers_button` is now `pub(in crate::app)`.
- **DOCK** sets `DetachableModule::Panadapter` back to docked and persists, so the
  operator does not have to hunt for the window's close box to send it home.
- **The readout stays display-only** — a click or drag on the spectrum still tunes
  and the tuning strip stays in the controls window — preserving the single-owner
  invariant for the frequency.
- **Test** `the_sp1_toolbar_is_a_header_and_not_a_share_of_the_picture`: the bar is
  tall enough for the 21 pt readout and under a tenth of the window SP1 opens at.
- **Not seen in a running window** (the detached viewport no-ops under the headless
  harness), and the bar reserves height the spectrum then draws into.

## AUX SP as built (2026-10-09) — the second spectrum, and what it is not

- **`DetachableModule::AuxPanadapter`**, appended (slot 3), app-id
  `sdroxide-panadapter-aux`, window title *aux panadapter*, opening size the same
  as SP1. A Settings → UI row appears for it by itself: that list walks
  `DetachableModule::ALL`, and the hover names its app-id for a window rule.
- **One draw, two windows.** `M::Panadapter | M::AuxPanadapter` is a single match
  arm, so the two cannot drift apart in anything but their id. `module_window_wanted`
  is the one predicate for both ("undocked, focused, and the layers are on"), so a
  second spectrum window never appears without a spectrum in it.
- **The toolbar takes the module it is in**, so AUX SP's **DOCK** sends *AUX SP*
  home rather than SP1 — the bug a shared toolbar would otherwise have.
- **It is a second view of the *same* receiver**, and the doc says so. Both
  windows render one station's shared spectrum frame and the same `ViewState`, so
  they show the same band and the same zoom. Pointing AUX SP at another radio
  needs the shell to own a window per (module, radio) rather than per module,
  which is a bigger change than this slice and is left for whoever wants it.
- **The registry is now proven to extend safely** — that was the real work here.
  Adding a module grows `UiSettings::detached`, and a derived array deserializer
  would have rejected every saved config over one extra slot; `Settings::load`
  **quarantines** a file it cannot parse and answers `Settings::default()`, so
  the cost would have been the operator's theme, fonts and layout as well as
  their window geometry. `detached_slots` reads the field at any length now, and
  the tests are in `crates/sdroxide-config/tests/detached_slots_survive_a_module_count.rs`
  (through the real loader) and `ui::tests::the_detached_list_loads_at_any_length`.

## The MAIN window's buttons as built (2026-10-09) — WINDOWS on the strip

- **On the radio-tab strip, not the top bar.** That was tried first and the strip
  layout tests refused it: the System box's width is priced from its two chip
  rows and the top row is the wider of the two (435 pt against 427), so an eighth
  chip there grew the box by ~85 pt and the desktop strip needed **three** rows
  instead of two. The radio-tab strip is also the better home on the handover's
  own reading — it *is* the MAIN window, and every other control on it is about
  the windows and the radio together.
- **The chip** (`MultiApp::windows_button`) is lit while any module is out of the
  main window, and opens one row per module named as an SDRuno operator looks for
  them — **SP1 — the spectrum**, **SP2 (AUX) — a second spectrum**, **Operating
  panel**, **RX control** — each with a **DOCKED** / **IN ITS OWN WINDOW**
  toggle, and **DOCK ALL WINDOWS** when any is out. The toggle closes the menu on
  **DOCK ALL** rather than leaving a list of rows that are all docked.
- **It acts on the focused radio**, and reads its settings rather than the app
  mutably: the strip is drawn from `&MultiApp` and a split pane must not undock
  the radio it is not looking at. So the change goes through
  `StripAction::{SetDetached, DockAll}` and lands in
  `SdroxideApp::{set_module_detached, dock_all_windows}` — the same two writers
  Settings → UI uses, so the two cannot drift in whether they persist.
- **The browser draws no chip.** It has one window and keeps every module in it,
  so there is nothing to arrange; the strip's calls are `#[cfg]`-gated to keep
  that honest rather than leaving a dead control.
- **Test** `the_windows_menu_offers_every_module_by_name`: one row per module, no
  two rows the same, and SP1/SP2 called by those names.
- The module names live in one function (`module_window_name`), so a module added
  to the registry gets a row whether or not anyone remembered.

## House notes for whoever picks this up

- The wire type is untouched by all of this: `UiSettings` is local `config.toml`.
  A new `DetachableModule` variant is fine; a new **tool** id is fine. Only a
  `ClientScreen` field or a new `LayoutMode` variant needs a `PROTO_VERSION` bump.
- The module registry: a new docked module is a `DetachableModule` variant, its
  `detached_spec`, a `show_detached_module` arm, and a `*_window_wanted`
  predicate where it is not always present.
- A new tool: extract its body and call `self.tool_window(ctx, id, title, size,
  open, |me, ui| …)` from its `*_window` method.
- Read `UNDOCKED-HANDOVER.md` for the machinery and the SDRuno/Niri window-rule
  caveat; read `AGENTS.md` for the house rules.
