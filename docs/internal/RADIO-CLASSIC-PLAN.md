# Radio Classic — the plan

A new panel in EXTRAS: a classic-radio face for broadcast listening, with the FM
radio (WFM + RDS) as the first thing it does well. Asked for by madmedicnl on
2026-10-28, from a screenshot he liked; the previews (v1 → v17, Classic and
Amber Phosphor themes) were validated by him the same day. The plan below was
approved with its three decisions.

## What it is

One framed panel, **docked by default** under the waterfall exactly like the DAB
panel (waterfall above, a draggable handle, the panel full width, 46 % of the
column by default), in three columns of equal height:

| column | content |
| --- | --- |
| left | bands (the existing SWL band plan), OL/OM, FM 87.5–108 and **FM ext 64–87.5**, OC, modes WFM/FM/AM/USB/LSB/CW, **MEM** and **SCHEDULE** (one click retunes band, mode, frequency and filter) |
| middle | the selected mode, centred, over a mini spectrum + waterfall; a segmented S-meter; BW / STEP / AGC / LNA; the DAB stereo VU (also the volume control) |
| right | LOC and UTC clocks; the VFO readout (9 digits, interactive); the station name; RadioText; RDS chips; audio bars |

On FM the name is the RDS PS; on every other band it is the name of the memory
or of the schedule entry. The VFO is the existing `freq_display::show` with 9
digits (no GHz column); the VU is the existing `widgets::vu_meter`. The theme
colours come from `theme.rs`, so the panel follows the theme in force.

## Decisions (approved 2026-10-28)

1. **Wire.** The VU and the bars need the main receiver's audio level, which
   reaches the client today only from the DAB status (`audio_rms_db`). A new
   message carries it, appended last. **A PR, never `main`**:
   `PROTO_VERSION` 256 → 257, `VERSION_BYTE` 0x4C → 0x4D, a register line, a
   postcard round-trip test.
2. **Digital modes keep the slot.** A mode that has its own docked panel (FT8,
   DAB…) keeps the space under the waterfall; Radio Classic is then used
   detached.
3. **SCHEDULE** shows the Aoki/EiBi broadcasts on the air at the current UTC
   hour on the chosen band.

## Steps

| # | step | done when |
| --- | --- | --- |
| 0 | bookkeeping: `TODO.md`, the board, this file | committed |
| 1 | audio levels: the engine sends L/R level and ~48 bands of the main receiver at ~30 Hz; wire bump | round-trip test, shown to fail without the fix |
| 2 | skeleton: `panels/radio_classic.rs`, `TW_RADIO_CLASSIC` (1 << 50), state in `UiSettings` (serde default), an EXTRAS chip in a free slot | the chip opens an empty panel |
| 3 | docking in the voice branch of `frame.rs` (handle, 46 %), digital modes and CW unchanged | a test of the split, the DAB and CW layouts untouched |
| 4 | left column | a click on a MEM retunes everything |
| 5 | middle column | the VU follows the audio |
| 6 | right column (VFO gestures, RDS block, bars) | the gestures work, RDS chips hide when not sent |
| 7 | detachable: a line and a switch in Settings → Tool windows, remembered geometry | it detaches and docks back |
| 8 | render the real panel to a PNG (an ignored test, like `dab_panel_preview`) and compare with the previews, both themes | looked at, not just measured |
| 9 | gate: `cargo check --workspace --all-targets` silent, tests of the touched crates, `rustfmt` on what is touched, the wasm check if it can be run, the journal | all green, or the blocker said |
| 10 | the PR on `f4nrm/determined-dirac-yyf856`, not merged by us | open |

## Rules that apply

- F4NRM's detachable-window work is in the same file as the docking: the PR
  keeps that change small and isolated and leaves his panels alone.
- No new `Command` is planned: tuning, mode, band and memory recall go through
  the commands a normal radio panel already sends. If one turns out to be
  needed it is a wire change and goes in the same PR, under the same number.
- A fix carries a test verified to fail on the unfixed code; anything drawn is
  rendered and looked at.
