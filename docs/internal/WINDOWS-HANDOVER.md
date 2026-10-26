# Windows, docking and the layout state — handover

Issue: the operator (Roy) cannot control windows from his tiling WM (no
titlebars), which is why chips were bolted onto panels, and those chips are ugly.
He wants panels detachable to a **floating** state or a **separate window**, with
**no more chips** (icons are fine), and he asked for the work to be built and
tested on a local branch before anything is pushed.

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
- Decisions taken: **the in-app floating window is not the destination** —
  separate windows are (the WM tiles them beside the main one); floating is a
  peek. **A detached panadapter window should be just the picture** (readout,
  spectrum, waterfall, its panel, no tools): in Classic the column *is* the
  control surface, and a second one on another monitor is a second place to
  change the same thing.

## The agreed design

1. **One icon, one list.** A `▣` beside the gear opens a list of every detachable
   module (`sdroxide_types::DetachableModule`), each row showing its state as
   three icons — docked / floating in this window / its own window — and its key.
   The list **is** the recovery: a window lost on another workspace or behind the
   main one is still a row, and "bring it back" does not depend on the window
   being visible. Every float/dock chip and the radiotab's windows button fold
   into it.
2. **Two detach targets, two mechanisms.** Floating = an egui window inside our
   frame (no WM can tile or lose it). Separate = an OS viewport, which carries a
   **header we draw** (title, dock icon, close) precisely because a tiling WM
   draws none.
3. **One shortcut, not N.** `Ctrl+Alt+W` opens the list anywhere, with a filter
   box; arrows move, `D`/`F`/`S` set docked/floating/separate, `Esc` leaves.
   `Ctrl+Alt+F` toggles float/dock for the panel in focus. Two keys in total.
4. **Workspaces are the real answer for a keyboard WM.** The app already saves
   workspaces; each should be bindable (`Ctrl+Alt+1…9`), because a tiling user
   wants "my listening layout" on a key, not per-panel nudging.

## The architectural fault underneath it

The per-frame UI state is published **once per frame on the shared
`egui::Context`** and read by every window:

- `layout::set_tier` (frame.rs ~695) — from the **main window's** rect.
- `SdroxideApp::classic_column_this_frame` (frame.rs 3129) — Classic UI *and*
  "the column fits this width"; only the main window ever draws the column.
- `band_dock_room`, `set_radio_salt` — same pattern.

A detached viewport shares that context, so it answers those questions with the
main window's geometry: a small panel window lays itself out as though it were
desktop-wide (the strip's chips clip at the right edge — seen in the operator's
photograph), and Classic's arrangement is half-applied. **The fix is to scope
that state to the viewport being drawn** (`set` on entry, restore on exit), so
each window answers for itself.

## Done on `local/windows` (not pushed)

- `classic_column_this_frame` now consults `controls_detached()`
  (frame.rs 3155): with the Controls module detached, the main window keeps its
  own strip controls instead of drawing a readout-only strip that expects a
  column living in another window. Test:
  `a_detached_column_hands_the_controls_back_to_the_strip` (app/mod.rs).
- **Step 1, the per-viewport state, is done (`196827a3`).** `layout::ViewportScope`
  holds the five answers the frame publishes (tier, tight, short-tablet, the
  band dock's room, the radio salt), publishes the child window's, and puts the
  main window's back when it drops. Every module, every tool window and the
  floating panel enter it through `detached_viewport`, which now takes a
  `ChildWindow` — the type is what stops a call site being written without
  saying whether it is a window in its own right (`Own`) or a desk tool
  (`Desk`, the console, the band selector and the rotator, as before).
  - A child window's tier comes from its **width alone**
    (`layout::tier_for_window`): the height rules describe a screen, and a
    window tiled short is not a phone. Floored at `Tablet`, because the phone
    design is one pane at a time behind a tab row and that row is the main
    window's.
  - The band dock's room moved from an app field to a context register
    (`layout::band_dock_room`), which is what let it be scoped at all — and
    fixed a control window's band chip hiding a column in the main window.
- Built and installed for testing (`~/.cargo/bin/sdroxide`, 13:15 on the 25th).
- **Found by looking, and it is the next step's work**: at 560 pt the floating
  FT8 panel still draws its two columns and the QSO area's chips run past the
  window's right edge (`floating_panel_preview`, new ignored tool). The phone
  arrangement is what fits there, and it needs the panel window's **own** pane
  tabs first — which is step 2 below. The finding is on `tier_for_window`.
- **Step 2, the narrow panel window, is done (`3a5b1d9e`).** Under 600 pt a
  panel window (detached or floating) draws its own **DECODES / QSO** row —
  `panel_tabs(mode, waterfall: false)`, so no waterfall tab: the picture is the
  main window's — and shows one pane at a time. `WindowShape::Panes` is the one
  shape allowed past `tier_for_window`'s Tablet floor, because it carries the
  row that makes one pane reachable. The window's pane is its own
  (`panel_window_pane`, swapped in around the draw the way AUX SP swaps its
  view), clamped to a real pane first. Widening past 600 brings both columns
  back. Looked at: `/tmp/opencode/float-560-after.png` (one pane, nothing past
  the edge) and `/tmp/opencode/float-wide-after.png`.

## Next, in order

1. ~~**Scope the state per viewport**~~ — **done**, `196827a3`.
2. ~~**A child window is self-contained and minimal**~~ — **done for the panel
   window**, `3a5b1d9e`: it draws its panel, its own pane row and the way home.
   **Classic, done (`7574093c`)**: the strip had been re-deriving "is the column
   here?" from the width left *after* the column took its share, so a Classic
   window with a panel detached drew the full strip under the frequency beside a
   column holding the same controls. One answer now, published where the column
   is drawn (`layout::classic_column`), and a child window publishes `false` for
   its own draw so the console keeps every control. **Still owed**: in Classic a
   detached *panadapter* should draw no tools at all — it still carries its DISP
   chip.
3. ~~**The `▣` list**~~ — **done**, `aa716e97`: `ModuleWindow` on the app, the
   list at the end of the System box and the phone's ☰, three icon slots per row
   with the current one lit, DOCK ALL / UNDOCK ALL, and the state round-tripping
   in tests. **The chip is on the row only while a module is out** (or the list
   is open) — as a permanent ninth chip it cost the widest 1600 pt strip a row,
   which is issue #294's rule. Preview: `module_windows_preview`.
   Notes for the step that follows:
   - **The ▣ is in every panel header that has a chip row** (`7574093c`): SSTV,
     the FT8 family, JS8, AtCHAT, packet, ADS-B, plus the floating panel's own
     row and the detached panel window's. The panels with **no chip row** — WSPR,
     PI4, NAVTEX, FSQ, JTTY, CW, RADE, RF Paint, the text modems — have nowhere to
     put it without being given a row, which is a height change on F4NRM's
     panels; the operator has to say which way he wants that.
   - The list is a **window of its own** now (`module_windows_window`), opened by
     the ▣ chip and by `Ctrl+Alt+W`; step 4 took it out of the SYS menu because a
     popup is anchored to the chip that opened it and the key has no chip.
   - So `Ctrl+Alt+W` (step 4) needs somewhere to put the same list: either an
     in-window `egui::Window` drawn from `module_windows_list` (which is
     `pub(in crate::app)` and needs nothing but a `Ui`), or a popup anchored to
     the strip. Prefer the window: it is what "does not depend on any window
     being visible" means, and it reuses the body as it stands.
   - `set_module_window_state` is the one write path; the keys should call it too.
4. ~~**`Ctrl+Alt+W` / `Ctrl+Alt+F`**, and workspaces bound to `Ctrl+Alt+1…9`~~ —
   **done**, `1dd73489` (the keys; the list is a window now so the key has
   somewhere to open) and `a8a2a842` (workspaces on the number row, `0` for the
   tenth, and an empty slot names itself on the banner). **Still owed from the
   design**: a filter box and arrow-key navigation inside the list (`D`/`F`/`S`
   per row). The rows are pointer-driven for now.
5. Delete the chips: `super::window_chip` call sites, the float/dock icons inside
   the detached Panel window, and the windows button on the radiotab row.

## Not verifiable headlessly

The screenshot harness runs with **embedded viewports**, so a *detached* window
cannot be photographed by `--screenshot`; only the main window and the in-app
floating panel can. Whoever picks this up should verify the detached states on
the bench (the operator's Sway session) — and the harness could gain a
`--screenshot` action that raises a detached module if that is worth the wiring.
