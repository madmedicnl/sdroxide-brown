# Antenna reference values — from dx-antennas.com

> Concrete numbers to **validate the solver** and seed **presets**, gathered from
> <https://dx-antennas.com/> (operator, 2026-10-11). The designs are by
> **19DX348** unless noted; the site is the source, so **attribute it** if any of
> this is carried into the program or the manual.

## Golden values for the far field / impedance

| Design | Gain | Z | F/B | Fit for our solver |
|---|---|---|---|---|
| Half-wave dipole (freespace) | **2.14 dBi** (0 dBd — the reference) | ~70 Ω (20–100 with height/polarisation) | — | Straight single wire — **already validated (~2.1 dBi)** |
| **4-element OWA Yagi** @ 27.555 MHz | **8.42 dBi** | 50 Ω | <22 dB | Parallel straight elements, no bends — **ideal acceptance test** |
| 2-element Moxon @ 27.555 MHz | 5.94 dBi | 50 Ω | <37 dB (H) | Folded tips = bends, so it waits on the bend model |

`dBi = dBd + 2.14`. The site prefers **dBi** precisely because dBd can be gamed.

### 4-element OWA Yagi (the one to test against)

- Boom **3.82 m**, 27.555 MHz, gain **8.42 dBi**, F/B and F/R < 22 dB, 50 Ω.
- Elements 22 mm centre tube + 18 mm tips.
- Distances: reflector→radiator **1.329 m**, reflector→director1 **1.834 m**,
  reflector→director2 **3.82 m**.
- Half-lengths: reflector **2.736 m**, radiator **2.657 m**, director1
  **2.523 m**, director2 **2.4062 m** (double for the full element).
- A second (inch) version at 27.555 gives 8.32 dBi; a third at 27.205 MHz
  (boom 3.87 m) gives 8.34 dBi.

### 2-element Moxon

- Boom **1.445 m**, 27.555 MHz, gain **5.94 dBi**, F/B < 37 dB, F/R < 27 dB, 50 Ω.
- 2 mm (AWG #12) wire. Reflector length 3.956 m with 0.740 m tips; radiator
  3.956 m with 0.592 m tips; tip gap 11.3 cm.

## The dipole

- λ(m) = **299.8 / f(MHz)**; the dipole is λ/2 — 5.70 m for 26.285 MHz. The site
  builds to the full half-wave and **trims to best SWR** (start ~0.05 m long).
- Inverted-V, sloping dipole, horizontal and vertical are the same wire in a
  different shape; the **inverted-V is still horizontally polarised**.
- "Gain 0.0 dB" on the dipole's own EZNEC plot is *0 dBd* — it **is** the dBd
  reference; in dBi it is **2.14**.

## Ground, height and take-off angle (for the ground model)

- A dipole above **real earth gains up to ~6 dB** over freespace, depending on
  height, polarisation and ground — this is the ground model's whole point.
- **Very good ground** (near sea/fresh water) favours low angles when the
  vertical is **low**; raising it above ~5–6 m is *worse* for DX. **Poor ground**
  (city/industrial/rock) wants the vertical **higher than λ/2**. So there is no
  single "one wavelength" rule — the site stresses the perfect-ground plots are
  misleading for this reason.
- Antenna **tip height** is what matters for local coverage, not element length.
- Term to model and show: the **take-off angle** (TOA) — the elevation of peak
  gain — and the **−3 dB beamwidth**.

## Stacking (milestone D)

- Stacking gives **~2.0–2.7 dB**, not the often-quoted 3 dB, because side lobes
  must stay down.
- Spacing formula, in wavelengths: **`57 / (−3 dB beamwidth°)`** — e.g. a 66°
  beamwidth → ~0.86 λ.
- Stacking vertically narrows the **elevation** beamwidth; horizontally narrows
  the **azimuth** beamwidth.
- Stacking can add a high-angle dip (fewer nearby high-angle stations) — the
  point is switching between take-off angles, not only gain.

## Pattern vocabulary (for the plot labels)

- **Azimuth** = looking down from above (360°); **elevation** = from the side
  (180°, earth blocks the rest).
- **Front-to-back (F/B)** = suppression straight out the back; **front-to-rear
  (F/R)** = suppression over the whole back 180°. F/R is the more honest figure.
- **Side lobes**, **−3 dB beamwidth**, and the fact that gain in one direction
  is paid for by loss elsewhere (a focused balloon).

## Presets worth adding (dimensioned on their pages)

Wire antennas: **2el Moxon**, **3el Moxon**, **Hentenna**, **Double Extended
Zeppelin**, **Saturn "Bi Square"**, **Loops**, **3el Skyrex**, **3el Skypper**.
Verticals: **J-Pole (Dutch / HPSD)**, **End Fed (Fuchs)**, **Extended double
Zepp**. The Yagi/quad catalogues carry dozens more with a dBi figure in the title.
