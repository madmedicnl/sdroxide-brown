# RSR-DF Analyzer manual — notes

Notes from the **RSR200B / RSR-DF Analyzer** user manual (BETA 2.4, 2026-09-13),
which the operator linked from <https://dx-antennas.com/>. It is a **separate
Windows/WPF SDR + direction-finding + medium-wave DX program** (personal use
only) — we copy **no code or text**, only the ideas and the physics below, which
are standard. Source attributed.

## What it is (for context, not for us to clone)

A Windows SDR app built around **coherent multi-channel reception**, weak-signal
analysis, long-recording review and MW DX. It credits **Linrad** (SM5BSZ),
**WavViewDX** and the **MWList** station database as its references. Core pieces:
LIVE RSR200B workspace + RSRMW (recording review), spectrum/waterfall/meters, a
fine carrier view, **coherent two-channel direction finding** with calibration,
an **Azimuth Map**, and **MWList** station identification.

## The physics worth keeping (for stacking and any array/DF work)

- **Two-antenna phase DF:** `Δphase = 360° × (d / λ) × sin(angle from broadside)`,
  with `λ = c / f`. A single straight baseline gives a **primary and a mirrored
  ambiguity**; resolving it needs a directional pattern, geography, or a second
  baseline.
- **Baseline spacing vs behaviour** (express as `d/λ`, not metres):

  | `d/λ` | behaviour |
  |---|---|
  | 0.02–0.05 | compact, weak angular sensitivity |
  | 0.05–0.10 | moderate |
  | **0.10–0.25** | **recommended compromise** |
  | 0.25–0.50 | high sensitivity, geometry/calibration matter |
  | > 0.50 | extra 360° phase wraps → **ambiguous** |

  Concrete: a 60 m baseline is 0.106 λ at 531 kHz, 0.162 λ at 810 kHz, 0.200 λ
  at 1 MHz, 0.300 λ at 1.5 MHz. (A 0.25 λ baseline is 141 m at 531 kHz but only
  0.75 m at 100 MHz.)
- **Angular accuracy from 1° of phase error at broadside:** 0.025 λ → 6.37°,
  0.05 λ → 3.18°, 0.10 λ → 1.59°, 0.20 λ → 0.80°, 0.25 λ → 0.64°, 0.50 λ →
  0.32°. Best near broadside, worse toward endfire.

**Why this is in *our* notes:** the `d/λ` spacing rule and the ambiguity limit
are exactly what milestone **D (stacking)** needs, and the phase relation is the
basis of any future coherent/array feature. It is standard array theory, stated
cleanly.

## Feature ideas that map onto our work

- **Azimuth Map** — receiver-centred, **great-circle azimuthal-equidistant**
  projection (direction = initial great-circle bearing, radius = distance), with
  range rings and graticule. Crucially it draws the operator's **antenna heading
  and −3 dB beamwidth as a lobe sector** — which is precisely what our antenna
  tool's pattern could feed into a map, i.e. the **RX nice-to-have** (predicted
  pattern/take-off overlaid on the receive side). It also plots station
  candidates (logged green / not-logged blue, details on hover).
- **MWList workflow** — an external MW station database used to match a received
  carrier to candidate stations by bearing/distance, with day/night power. A
  possible **SWL/MW listening** feature (station identification), which is
  listener-side and on-fork.
- **Long-recording review** (WavViewDX-style) and a **fine carrier view** — we
  already do time-shift replay and scheduled recordings; the carrier-view idea
  is a possible refinement.

## Caveats

- It is another program's manual: **no licence to reuse its code**; the physics
  and the `d/λ` table are standard and re-derivable.
- The DF material is **medium-wave, ground-wave, two-channel coherent** — it is
  an SDR-receiver technique, not an antenna-*modelling* technique, so it informs
  the RX/array side rather than the MoM solver.
