# Where we stopped — night of 26 October 2026

Read this first. It is the state of play after a long session, written because the
operator had to stop mid-thread. `AGENTS.md` is still the rules; this is the
position.

## The one-paragraph version

The windows work merged (#261), a full release was cut and then **aborted** because
Windows had no installer, `main` now carries the two build fixes and **no v2.1.2
release exists on origin**. The live bug is **#263** — F4NRM's Chrome client cannot
pan or zoom, on a TS-2000 listening through a lent HackRF — and its diagnosis is
pinned to the view taking its span from the *listening* radio's state instead of
from the spectrum frames it is drawing. That fix is specified below and not
started. The WEFAX multi-page puzzle is separately unreproduced because every
recording we have holds one page.

## State, measured

- `main` at **`a5bbb0dd`**, clean, pushed. Wire **255** / `VERSION_BYTE` `0x4B`.
  Crate version **2.1.2** (the bump is on `main`; no tag carries it).
- **No v2.1.2 release on origin.** `v2.1.2_brown` and `v2.1.2.1_brown` pages and
  tags were deleted at the operator's instruction ("abort the release and remove
  the files"); the `v2.1.2.2_brown` run was cancelled before it published. Latest
  release is the **v2.1.1.1** pre-release. Do not re-cut any of those tags.
- `~/.cargo/bin/sdroxide` is **the 17:28 test build** from `local/windows` — the
  operator is using it on purpose. Nothing was installed since.
- Open: **PR #258** (ours, WEFAX centring, waiting for F4NRM), **issue #263**
  (new, F4NRM), and the ten issues that were already open.

## On `main` and worth keeping

- **`s4285: M_PI for Windows`** (`8c0bc8e2`): the vendored STANAG 4285 decoder's
  headers use `M_PI`, which the runner's MinGW does not define; `build.rs` now
  defines it for Windows targets only. Before this, **Windows had no build since
  v2.1.1.1** and neither page said so.
- **`windows: the shortcuts belong to the executable, machine-wide`** (`9f2c4dac`):
  WiX 5.0.2 refuses the old `main.wxs` (ICE38/43/57) because the Start Menu and
  Desktop shortcuts were components of their own keyed on HKLM inside the
  **per-user** `ProgramMenuFolder`/`DesktopFolder`. They are now children of the
  executable's `File` with `CommonProgramsFolder`/`CommonDesktopFolder`. **The
  Windows job is the judge**: WiX cannot be installed on this machine (no .NET
  SDK), and this fix has never been through a build.
- A **re-release** therefore needs: a fresh `v2.1.2.N_brown` (fourth point; never
  a re-cut), the changelog heading renamed to match (the workflow reads
  `## [<tag>]`), and `wix msi validate` green on the standard Windows job. Windows
  is the operator's most important group — *"no windows is our most important
  group.. fix it until it works"*.

## #263 — "Big Pb with zoom" (F4NRM, 26 Oct)

His words: *"Not work. Always at this place. Moving ou zooming is impossible. (all
modes). CTR are now problem."* Two screenshots, no version, no comments.

**His configuration** (which is why the operator could not reproduce it, and why
it is not Chrome's fault): radio 1 is a **Kenwood TS-2000 on CAT with the rig's
audio**, radio 2 is a **HackRF lent as radio 1's panadapter**
(`PanadapterConfig { source_radio }`, `sdroxide-types/src/radio.rs`). The picture
is radio 2's; the state the view is built from is radio 1's, whose "sample rate"
is the audio chain's.

**The fault**, in today's merge #234 (the Airspy HF+ display span): `pan_view`
begins with

```rust
if view.span() >= dev_span { /* nothing to pan */ }
```

and `dev_span` is computed from the listening radio's state
(`spectrum.rs` `display_span(sample_rate, usable_hz)`; the widget still reads
`state.sample_rate` at `widgets/spectrum_view.rs:1425`). On a lent panadapter the
view is wider than that span before the operator touches anything, so pan and
zoom-out are refused — in every mode, because the path is shared. CTR then centres
on the TS-2000's dial rather than on the HackRF's actual centre.

**#234 cannot be reverted whole**: it added `usable_hz` to `RadioState` and bumped
247 → 248, and four wire bumps sit on top. Removing the field is the append-only
rule's one prohibition. Its HF+ wins stay.

**The fix, specified and not started.** `SpectrumFrame` already carries
`center_hz` and `span_hz` — the frames know their own centre and span, whoever
produced them. So:

1. `draw_panadapter` (`app/frame.rs`) builds its `WindowPan`, its fit key and its
   clamp from `frame.center_hz` / `frame.span_hz` when a frame exists, falling back
   to `state.center_hz` / `display_span(sample_rate, usable_hz)` only when there is
   none (silent radio, playback, a sweep).
2. Thread that span into `spectrum_view::show_ext` so the widget uses the same
   number as the clamps instead of reading `state.sample_rate` — **one span for
   drawing, fitting and clamping** is the invariant that is broken today.
3. Test that fails on the old code: a synthetic frame whose `span_hz` differs from
   `state.sample_rate` (a CAT rig with a lent panadapter) must pan and zoom, and
   must not hit the `view.span() >= dev_span` exit on the first frame.
4. **Reproduce first, bench-free**: pair an IQ-file source as the panadapter of an
   audio radio natively and watch the two numbers the client receives. No radio
   needed.

**The CTR half** may be the fax merge `4437b72b` (`centres_on_picture`, UI only,
no wire field): if his "CTR are now problem" is not explained by the span fix,
that one reverts cleanly and nothing after it touches that function.

**Ask him exactly one thing**, when a reply goes out: the build's version from the
title bar. Nothing else — no re-test, no recording.

## WEFAX multi-page — the other puzzle

The claim: **"only the first one or two pages decode"** (F4NRM today; his 18 Oct
report says three pages then the rest broken and *"then the reading function
deactivates itself"*; markjfine, a different station, 24 Oct: *"charts have
prematurely stopped after roughly 500 lines … Autostart engaged, Autostop not
engaged, and they still quit mid-way"*). The carrier standing on the channel
corrupts rows (253 of 2153 in his own file) — that is a level problem he has
confirmed himself, not a trigger problem.

**What has been proved with his own files** (`crates/sdroxide-dsp/examples/wefax_probe.rs`):

- PR #258 attachment, 70 s, 15:10 today: no start tone; **phased 35.8 s**; 68 lines
  before the clip ends.
- The Drive file ("real full signal from DWD 3855", 1555 s): start tone at 42.7 s,
  **complete at 1121.6 s, 2151 lines, ended by its stop tone**, one picture,
  1809×2153. Nothing in the 433 s after it.
- **Both hold exactly one page, so neither can reproduce a multi-page failure** —
  not as his audio and not as our state machine.

**What would settle it**: one short **audio** clip spanning a page boundary (last
minute of one chart, the stop tone, the gap, the next chart's start and first
phasing lines) — a few megabytes, the same shape as the 6.7 MB file he already
sent. Then the probe prints, per page, whether the second start came from tone or
phasing, where its pulse landed, and how many lines it got.

**Suspects once we have it**: `IDLE_HOLD_S` (15 s in which phasing alone may not
begin a page), `BLANK_END_S` (60 s of white carrier ending a page), the re-arm
after `COMPLETE`, and a passband that moves between pages — the decoder's
black/white windows are re-measured only from a phasing signal.

## The listening-screen trap (cost an hour tonight)

Transmit controls vanished for the operator. Three independent switches hide them,
and the Source fold says only "listening screen", naming none:

- `config.toml start_swl = true` — *Settings → UI "start with SWL mode on"*, read
  **once at launch** (`swl_start`), so it holds the screen in SWL for the session;
- `config.toml listen_tab_hides_tx = true` — the LISTEN tab hides the transmitter;
- `radio.json hide_tx` — the per-radio *Settings → Radio "hide them all (SWL mode)"*,
  which takes effect on **Apply** only.

Wanted: the fold names which switch is doing it. Small, honest, not started.

## Other things not to re-derive

- `--server` on this build serves a placeholder page: our binary has no embedded
  web client (`--features embed-web`), so it needs `--web-root
  crates/sdroxide-web/dist` from a `trunk build`. It also opens the same radios, so
  the GUI must be closed first. Verified healthy otherwise: HTTP 200, `/ws/1`
  upgrades with 101, no panic.
- PR #261 was merged **on the operator's explicit word** (he asked for the merge
  and the release); the rule that only F4NRM merges our PRs is his and the
  exception is recorded in the journal. Our PRs are otherwise never merged by us.
- A release page with a failed job still publishes (that is how two partial pages
  happened): delete both the page and the tag when it goes wrong, as was done.

## First three actions next session

1. Reproduce #263 with an IQ-file panadapter, then make the frame's own
   `center_hz`/`span_hz` the one span for drawing, fitting and clamping — PR,
   because it touches a shared view.
2. Re-release: `v2.1.2.3_brown` with the changelog heading renamed, watching the
   Windows job's `wix msi validate`; Windows first, since it is the operator's most
   important group.
3. Reply to #263 with the one question (the build version), and to #258 with the
   single-page finding and the ask for a clip that spans two pages.
