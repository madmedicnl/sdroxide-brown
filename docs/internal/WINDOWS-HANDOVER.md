# Windows, docking and the layout state — handover

**Merged as PR #261** (`16d4339a`, merge `3ed50c4c`, 2026-10-26) on top of the
extraordinary circumstance that the operator asked for the merge himself; F4NRM
had not reviewed it. Everything below describes `main` as it stands after that.

The issue: the operator (Roy) cannot control windows from his tiling WM — there
are no titlebars, so no close box and nowhere to put window controls — which is
why chips were bolted onto panels, and those chips are ugly.

## What the operator said, in his words

- "Tiling WM's are mostly managed by keyboard. So instead of chips we could use
  shortcuts … but then we run into another issue: there are quite some panels to
  detach and it is impossible to remember all those shortcuts."
- "Panels should be detachable to a floating state or a separate window."
- "Floating windows … still obstruct the view on the waterfall or the controls
  depending where you drag them. Whereas in a window, on my WM, it shrinks my main
  window and places it next to them … and gives a nice orderly view."
- "I wanted to work with workspaces you can save — you spend time setting up your
  preferred view, save it, and call it back any time."
- The bug that started this: "I run Classic UI, the controls are in the left
  column. If I detach a panel or window it also starts showing the controls
  underneath the frequency (the regular layout), so it starts mixing two layouts."
- And the complaint that shaped the last pass: "the amount of chips … they are not
  logical, esp for simple controls. Icons would do … window controls have standard
  icons allover the world yet we somehow can't seem to use those."

## What the program does now — the grammar

One icon per verb, always in the same place, words only for nouns:

| state | control | where |
|---|---|---|
| docked panel | ⇱ *float* | the panel's own header row, all panels that have one |
| floating panel | roll-up · ⤢ *out to a window* · ✕ *close (docks)* | its **own title bar**, drawn by us |
| its own OS window | ⇲ *dock* | one glyph, top right of the window's content |
| any screen | ▣ *module list* | beside the gear; and `Ctrl+Alt+W` anywhere |

- **The float's title bar is ours** — `title_bar(false)` plus a row: the mode
  name at the left, `WindowIcon::Out`, roll-up and `Close` at the right, in that
  order. egui has no hook for a fourth title-bar button (`title_ui` is private and
  draws only collapse, title and close). With no title bar egui falls back to
  dragging from anywhere on the window, so the bar's ground and the body's empty
  space move it and controls keep their own drags. **Close docks**: a panel is the
  mode's control surface, not a tool that can be shut.
- **Roll-up** (session state, `panel_rolled`) is the answer to floating on a
  laptop: the body is not drawn, `min_height` is 0, so the float is its bar.
  `float_operating_panel()` unrolls, because a float remembered rolled up would
  hand back a bar and nothing else.
- **The bar's numbers are one place**: `chrome::{BAR_H, BAR_CHIP, BAR_FONT,
  BAR_INSET}` = 16 / 14 / 12 / 8, and `chip_window_icon_sized` draws the pictures
  at an exact box with a fixed 2 pt inset (the theme's chip padding is sized for
  buttons and leaves an 8 pt chip holding a 6 pt picture).
- **One drawn family** for every window button: `WindowIcon::{Out, In, Dock,
  Collapse, Unfold, Close}`, all painted at the same 1.3 stroke and the same
  fractions of the box. The font glyphs (`×`, `▾`) were three weights beside one
  painted arrow and the operator spotted it immediately.
- **The panel's OS window wears the same bar**, on the panel's own background
  (`theme::PANEL()`, which is what egui's `window_fill` is), so the window does
  not read as a chip strip pasted over a black panel.
- **The browser has no `out` button**: the floating panel is shared code, so the
  web client wears the same bar — but detaching is native-only, and an out button
  there would *dock* the panel instead of giving it a window.

## The module list, and the keys

- `▣` beside the gear (System box and the phone's ☰) lists every
  `DetachableModule`: name, three icon slots — docked / floating / its own window
  — the current one lit, an **empty slot** where a module cannot go (floating is
  the operating panel's alone), and DOCK ALL / UNDOCK ALL. Rows write through
  `set_module_window_state`, the one place the state changes.
- **The list is the recovery**: a window left on another workspace is still a row.
  It is a window of its own (`module_windows_window`) — a popup is anchored to the
  chip that opened it and the key has no chip to hang one on — so `Ctrl+Alt+W`
  reaches it with every module docked.
- The **▣ chip is on the strip only while a module is out** (or the list is open):
  as a permanent ninth chip it took the widest 1600 pt arrangement from two rows
  to three, which is issue #294's rule (`the_cessb_rail_costs_the_desktop_strip_no_row`).
- **Keys**: `Ctrl+Alt+W` list, `Ctrl+Alt+F` float/dock the operating panel,
  `Ctrl+Alt+1…9` and `0` recall a saved workspace (`workspace_slot`, a pure
  mapping; an empty slot names itself on the notice banner). Read where F1 is, so
  a text field never eats them, and from the **events**, so a chord pressed and
  released inside one frame still arrives and a repeat cannot flicker a window.
- The two conditions are separate and both live on the app:
  `module_windows_chip_wanted()` (is anything out, so is the ▣ on the strip) and
  `panels::window_chip` (the docked panel's ⇱, drawn by the panels that have a
  header chip row).

## The layout state, scoped per window

The per-frame UI answers were published once on the shared `egui::Context` and
read by every window, so a detached viewport answered with the **main window's**
geometry — a 520 pt panel laid itself out as though it were desktop-wide, its
chips running past its right edge.

`layout::ViewportScope` now holds tier, tight, short-tablet, the band dock's room,
the radio salt **and** the Classic answer, publishes the child window's, and puts
the main window's back on `Drop`. `detached_viewport` takes a `ChildWindow` —
whose radio, which layout override, which `WindowShape` — so no call site can be
written without saying what kind of window it is.

- `WindowShape::{Own, Desk, Panes}`: `Own` measures the window's **width alone**
  (`tier_for_window`, height rules are about a screen and a window tiled short is
  not a phone) and floors at Tablet, because the phone design is one pane at a
  time behind a tab row and that row is the main window's. `Desk` is the console,
  the band selector and the rotator, desktop-shaped whatever their height.
  `Panes` is a panel window: the one shape allowed past the floor, because it
  draws the tab row itself.
- **A narrow panel window** (under 600 pt) draws `digi_tabs(ui, mode, false)` — no
  waterfall tab, because the picture is the main window's — and shows one pane at
  a time. Measured: at 560 pt the two columns put the QSO area's chips past the
  window's edge. Its pane is its own (`panel_window_pane`, swapped in around the
  draw the way AUX SP swaps its view), clamped to a real pane first.
- **Classic has one answer.** The strip used to re-derive "is the column here?"
  from the width left *after* the column took its share, so a Classic window with
  a panel detached drew the full strip under the frequency beside a column holding
  the same controls. It is now decided once, where the column is drawn, from the
  window's whole width, and **published** (`layout::classic_column`); a child
  window publishes `false` for its own draw, so the RX control console keeps every
  control.

## Still owed

- **Step 5, the chips that remain**: the band popup still carries its own DOCK
  (the docked band column) and WINDOW (the selector's own window) icons, and the
  radio strip still has the `🪟 WINDOWS` menu (multi.rs); both now overlap the
  list. `panels::window_chip` stays on purpose — it is the docked panel's ⇱, the
  only control that *starts* a float.
- **The list's filter box and arrow-key navigation** (`D`/`F`/`S` per row) from
  the design were not built; the rows are pointer-driven.
- **Panels with no chip row** cannot carry ⇱: NAVTEX and JTTY have no `&mut self`
  in their header functions, and RF Paint's and the text modems' first chip row is
  a *list* header rather than the panel's. They float with `Ctrl+Alt+F` or the
  list. Giving them a chip means giving them a row, which is a height change on
  F4NRM's panels.
- **In Classic, a detached panadapter still carries its DISP chip**; the agreed
  design says it should draw just the picture.
- Declaring the switch that is holding a radio in the listening screen: the Source
  fold says "listening screen" without naming which of `start_swl`,
  `listen_tab_hides_tx` or the per-radio `hide_tx` is doing it.

## Not verifiable headlessly

The screenshot harness runs with **embedded viewports**, so a *detached* window
cannot be photographed by `--screenshot`; only the main window and the in-app
floating panel can (`floating_panel_preview`, `module_windows_preview`). The
detached states are the operator's Sway session — where they have been used, and
where the bar's drag and roll-up want an eye.
