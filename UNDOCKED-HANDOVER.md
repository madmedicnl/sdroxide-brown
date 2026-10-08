# Undocked mode — scope and first workflow

**Status: scoped, not started (2026-10-08).** A new layout where chosen modules
live in their **own OS windows** — like GIMP's and SDRuno's detachable panels —
so the main window can give its space to the controls and decoders while the
waterfall sits on a second monitor.

This is the gap it fills: today every `LayoutMode` (Desktop / Tablet / Small /
Phone / Auto) stacks every module into the **one** window. Operators have asked
repeatedly for the waterfall detached, and the fork has the mechanism already.

## What already exists (the groundwork — this is why it is not from scratch)

- **Multi-viewport is proven on this codebase.** The solar-system view is a real
  separate OS window, built with `ctx.show_viewport_deferred(vid, builder, …)`
  and an `egui::ViewportBuilder` — `crates/sdroxide-ui/src/solar3d/mod.rs`
  (`viewport_id`, around `:346`–`:366`). Any widget can be rendered into such a
  viewport; that is the whole of the "own window" mechanism.
- **Geometry persistence has a pattern.** `UiSettings::solar3d_window:
  Option<Solar3dWindow>` (`crates/sdroxide-types/src/ui.rs`) carries size and
  position, seeded into the builder only on the **rebuild frame**
  (`cumulative_pass_nr_for(vid) == 0`) so a live resize is never fought.
- **Per-radio window identity exists.** `layout::radio_salt(ctx)`
  (`crates/sdroxide-ui/src/layout.rs:275`) salts the viewport id per radio;
  `solar3d::viewport_id(salt)` builds it.
- **The one-window/one-owner rule exists.** The 3D window's `keep_alive` and
  owner cap (`solar3d/mod.rs:417`, and the AGENTS note "The 3D window in the
  multi-radio shell") are exactly the problem a detached panadapter has: a
  second window owned by a per-radio app, in a shell that draws only the visible
  tab. Read that note before touching any of it.
- **The layout enum is where a mode slots in.** `LayoutMode`
  (`sdroxide-types/src/ui.rs:244`) is `Desktop / Tablet / Phone / Small / Auto`;
  `layout::tier_for` (`:80`) maps it and the viewport size to a `Tier`. A new
  mode is appended **before `Auto`**, which must stay last because it carries
  `#[serde(other)]`.

## The known caveat, stated up front: Wayland, and **Niri**

A Wayland client **cannot set its own window position**. `viewport().inner_rect`
and `outer_rect` are `None` there (the solar3d notes record this) — only the
size comes back. So on Wayland "put it on monitor 2" is **best-effort**: size is
exact, placement is the compositor's. X11, Windows and macOS return real
geometry and honour a position. Say this in the UI copy rather than pretending.

**Niri is the operator's WM, and it is already a named target** in the "3D
window in the multi-radio shell" note (a remapped window is a *new* window to a
tiling compositor like niri/sway, so `keep_alive` is what holds its place).
Undocked mode meets Niri the same way, and the path is clearer than it looks:

- A detached viewport is a **new toplevel**, and Niri **tiles** it like anything
  else. To make it **float** — the operator's wish — is a Niri **window rule**,
  not an app call:
  ```kdl
  window-rule {
      match app-id="sdroxide-panadapter"
      open-floating true
      // and put it where the operator wants it:
      // open-on-output "HDMI-A-1"
      default-column-width { fixed 1200; }
      default-window-height { fixed 720; }
  }
  ```
- That is why the detached window must set **`with_app_id`** (and a title): it
  is the only handle a compositor rule has. Niri can also match on **title**, so
  even the 3D window (title only) is routable today — but a stable app-id is the
  better key.
- **`with_position` is ignored on Wayland/Niri.** Placement is `open-on-output`
  / `open-on-workspace` in the operator's rule, which is *better* for the
  "second monitor" case than anything the app could do.
- **`with_window_level`** (always-on-top) is compositor-dependent; do not rely
  on it. Niri floats and layers via its rules.
- **Emit the window every frame.** Niri destroys a window that stops being
  emitted and recompositor-places it on return — the `keep_alive` lesson, and
  the reason a detached panadapter (like solar3d) must stay alive while its tab
  is hidden.

Net: the app's job is to **expose the identity and the size**; *floating and
placing it on a chosen monitor is the operator's one-line rule*. That is the
honest division, and what the manual should say.

## Scope

**In (the first slice, below):** a detach toggle, the **panadapter** as one
detached window, its geometry persisted, and the main window reclaiming the
space.

**Out (later):** a general docking framework (any module, arbitrary splits and
tabbing); guaranteed multi-monitor placement under Wayland; floating in tiling
WMs; carrying the detached geometry on the wire (decide when it matters).

## First slice — the detached PANADAPTER

The single most-requested case, and the one that proves the pattern. Do not
build a framework first.

1. **State.** Add `UiSettings::panadapter_window: Option<DetachedWindow>` where
   `DetachedWindow { size: [f32; 2], pos: Option<[f32; 2]> }` — appended last,
   mirroring `Solar3dWindow`, and reused later for other modules. Plus a
   `panadapter_detached: bool` (on `UiSettings` if it should persist/travel, on
   the app if it is machine-local). **Appending to `UiSettings`/`ClientScreen`
   is a `PROTO_VERSION` bump.**
2. **The toggle.** A `DETACH` action in the panadapter's DISP row, with hover
   text saying where the window goes and the Wayland caveat. Turning it off (or
   closing the window — handle `ViewportEvent::Close`) re-attaches.
3. **The main window reclaims the space.** The digital split already computes
   `(waterfall_h, panel_h)` summing to the area (`frame.rs`, the `digi_split` /
   panadapter region). Detached ⇒ `waterfall_h = 0` and the operating panel
   takes the whole column. This is the part worth a **pure test**.
4. **The detached window.** When detached, `ctx.show_viewport_deferred(
   panadapter_vid(radio_salt), ViewportBuilder::default()
   .with_app_id("sdroxide-panadapter").with_title(…)
   .with_inner_size(size)[.with_position(pos)], move |ui, _| draw the
   panadapter)` — reusing the existing panadapter draw (`frame.rs` /
   `widgets/spectrum_view.rs`), not a copy of it. **The `with_app_id` is not
   decoration**: it is what a Wayland window rule matches on to float and place
   the window (see "Niri" below). solar3d sets only a title today; a detached
   window should carry its own id.
5. **Persist geometry.** Capture `inner_rect` / `outer_rect` on the rebuild
   frame only (the solar3d pattern), so a drag is not undone each frame.
6. **Native only.** `#[cfg(not(target_arch = "wasm32"))]` throughout the detach
   path; the browser keeps the panadapter in-window.

**The oracle.** The detached window itself cannot be asserted headlessly, so the
test is the **main-window layout**: with the panadapter detached, the operating
panel takes the full height and still stays inside its column — the same
both-edges check as `the_operating_panel_stays_inside_its_column`
(`crates/sdroxide-ui/src/app/mod.rs`). Plus a pure unit test of the split
arithmetic (`waterfall_h == 0`).

## Workflow and house rules (do not skip)

- **Gate before any tag:** `cargo check --workspace --all-targets` **silent**,
  then `cargo test --release --workspace`.
- **The wasm check is mandatory** whenever a wasm-reachable file changes:
  `cargo check --release --target wasm32-unknown-unknown -p sdroxide-ui`. The
  detach must not pull a native-only path into the browser build — the DAB
  radio-tab move (`5bef39e7`) broke exactly this and the check caught it.
- **Wire:** `UiSettings` and `ClientScreen` ride the wire whole; an appended
  field or a new `LayoutMode` variant is a `PROTO_VERSION` bump plus a register
  line in `crates/sdroxide-proto/src/lib.rs`. `LayoutMode` is appended **before
  `Auto`**.
- **A new `LayoutMode` variant** is also a `ClientScreen` change and is
  validated by the "catch-all last" rule — read `Auto`'s own doc first.
- **Multi-radio is the risk area.** Reuse the owner-cap/`keep_alive` model from
  solar3d; do not leave N radios each emitting a window.

## Later, when the slice works

- **Generalise to any detachable module** (decoders, band/mode dock) on the same
  `DetachedWindow` type, rather than a boolean per module.
- **The shell-owned window manager.** The AGENTS note already names the real
  fix: "a shell-owned window manager with stable viewport ids and an explicit
  owner, emitted every frame regardless of focus". A general undocked mode is
  the natural home for it.
- **Floating in tiling WMs** (the operator runs **Niri**): a tiling compositor
  tiles a new toplevel; making it float, and pinning it to a monitor, is a
  **Niri `window-rule`** the operator sets (see the Niri section above) — not
  something a Wayland client can assert. The app's contribution is the
  **app-id** the rule matches on. Worth a short manual section; not an app
  feature.
- **Whether the detached geometry travels** on `ClientScreen` (a remote client
  that wants its own arrangement) — a decision, not a default.

## Bench checks before committing to it

1. **Does a second viewport render the wgpu waterfall cleanly, and at what
   cost?** The panadapter is the GPU-heavy widget; the 3D window is the only
   precedent and it is a different render path.
2. **Multi-radio:** who owns the detached panadapter, and what a tab switch
   does (the solar3d owner-cap problem, exactly).
3. **Wayland:** confirm on niri/sway that a detached window can at least be
   resized and moved by the operator, and that the app reads a sane size back.
