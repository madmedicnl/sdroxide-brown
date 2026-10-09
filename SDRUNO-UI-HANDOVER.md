# SDRuno-style UI — the window map and the todo (handover)

**Fresh starting point (2026-10-09), read this before the SDRuno-shaped UI work.**
The operator wants this program's window layout to read like **SDRuno**: a set of
windows, each its own module, that the operator arranges. The undocked mode
(`UNDOCKED-HANDOVER.md`) is the machinery; this file is the **shape** being aimed
at and what is left. It exists because the operator expects to **switch models
when the work reaches MAIN SP** — so note the split below.

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
| RX Control | the **Controls** module (`sdroxide-controls`) | built, iterating |
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

## TODO (in order)

1. **The band pad — the aligned grid is in (2026-10-09); the SDRuno keypad is
   next.** `band_chip` is now a free function and `band_pad` lays the chips out
   in **fixed columns where there is room** and a wrapped row where there is not
   (a plain `Grid` never shrinks, so it overflowed the narrow dock). What is
   still missing is SDRuno's actual **keypad**: digits 0–9 laid out
   calculator-style, each also naming a band (7,8,9 / 4,5,6 / 1,2,3 / 0, with
   **Bands** and **MHz** above and **Clear**/**Enter** below), the digits tuning
   to a band and, in MHz mode, entering a frequency. It is wider than the dock,
   so it belongs **in the console** (its own widget, not the shared
   `band_mode_menu`). Decide then whether to keep the full band list beside it or
   move it behind a **MORE BANDS ▾** dropdown like the digital modes.
2. **MAIN SP — switch model here.** Give the undocked **panadapter** window an
   SDRuno-style **toolbar**: the frequency readout, SP controls, and the toggles
   SP1 keeps. This is where the operator expects a different model to take over.
3. **AUX SP (SP2)** — a **second** panadapter window (a second spectrum/waterfall
   on its own receiver or the same one). Needs a second `DetachableModule` or a
   `Panadapter`-with-index; the shell-owned manager already handles the plumbing.
4. **The MAIN window's buttons** — SDRuno's MAIN has SP1/SP2/RX/SCANNER/REC/MEM
   buttons. Our shell's radio-tab strip is the MAIN; consider a small **Windows**
   menu that opens each window (SP1/SP2/RX/SCANNER/RECORDER/MEMORIES), mirroring
   Settings → UI → Undocked but on the strip.
5. **The rest of the tool windows** — extract each body to `_body` and call
   `self.tool_window(...)`: schedule, logbook, spots, awards, grid tracker,
   public SDRs, recordings, SWL log, memories, voice keyer, and the per-mode
   setup windows.
6. **Workspace save/recall** — SDRuno records up to ten named workspaces
   (Ctrl+W / SAVE WS). Ours persists geometry per module but has no named
   workspaces and no "save the whole arrangement". A `UiSettings` list of named
   window layouts is the shape.

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
