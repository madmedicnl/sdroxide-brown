# TODO — SDR Oxide Brown

Working queue. Rules in AGENTS.md. Last updated 2026-10-31.

State: `main` at crate 2.2.0, PROTO_VERSION 258 (0x4E; 259 / 0x4F on the TX band list branch). Latest release
v2.2.0.3_brown. Open PRs: 4 (#318, #339, #341, #358). Open issues: 15 (re-measured 2026-10-29).

## Task tracking rules

- Every task started: add to **Now** (one line: what, why, where from) and board
  **In Progress** before starting.
- Every task finished: move to **Recently done** (today's date, one line) and
  board **Achieved**; remove from In Progress.
- Every task blocked: move to **Waiting on other people** with the single question.
- Issues take priority over development.
- Each task must name who added it and who owns it:
  `Item (added by X, owned by Y) — description.`
- Re-measurable numbers (crate version, PROTO_VERSION, open issues/PRs) must be
  kept current in the state line.

## Now

- PROGRAMS window layout and persistence (added by F4NRM, owned by Claude) — branch `f4nrm/stoic-fermi-a0z0zp`, PR pending: columns, Sort menus, chip-framed colour, two-row bar, saved filters. Wacht op F4NRM's review.
- SAT Doppler for CAT rigs (added by Roy, owned by Roy/SP9TKN) — step 1+2a+2b+2c
  implemented on branch `fork/sat-doppler-cat`. MSI build available. Wacht op
  SP9TKN's testresultaten. Design doc: `docs/SAT-DOPPLER-CAT-PLAN.md`.
- NAVTEX restore (added by Roy, owned by Roy) — PR #317 open, mergeable. Wacht
  op F4NRM's test.
- ADS-B overlay (added by Roy, owned by Roy) — PR #312 open. Wacht op F4NRM's
  review (relay rule gefixt).
- Airspy HF+ per-rate usable span (added by Roy, owned by Roy) — PR #309 open.
  Wacht op Graeme's test.
- Antenna calculator (added by Roy, owned by Roy) — PR #304 open.
- FT8 auto queue UI (added by Roy, owned by Roy) — PR #323 open (not
  live-tested).
- NAVTEX sync-drop (added by Roy, owned by Roy) — PR #308 open.
- UDP feed VDL2/ACARS/DSC/HFDL (added by Roy, owned by Roy) — PR #318 open.
  Rolling-window fix toegepast.
- APRS panel: all messages heard, 500 kept, composer note, RX/TX engine tests (added by madmedicnl, owned by Roy; touches F4NRM's APRS area, PR for his review) — decoder fault #259 not reproduced in the engine; needs a real capture.
- Per-band TX permission (added by madmedicnl, owned by Roy; touches the radio engine and every radio tab, PR for F4NRM's review) — PR #358 open, not merged. Every band has a YES/NO switch in Settings; new install = all off, upgrade = bands carried over but TX blocked until the operator confirms once per radio; not applied on SWL/listen screens. PROTO_VERSION 259 (0x4F). Needs a hands-on test with a real radio.

## Waiting on other people

- ~~STANAG 4285~~ **finished** (2026-10-29) — Mark confirmed decode with video.
  Coded payload decoding intentionally not implemented (restricted by law in
  several countries). Rust equaliser still does not converge.
- ALE — never decoded a real burst. Needs recording.
- S-meter jumping with mouse — question to reporter: number or needle?
- #105 AppImage — planned for stable release.
- #265 "outside receive range" — partially fixed (notice logged, not shown).
  Second message (2.941 MHz) still open.
- #239 WEEFAX — not fixed.
- #262 SSTV editor freeze — not fixed.
- #263 Zoom — not fixed.
- #264 Public SDR — not fixed.
- #259 APRS — not fixed.
- #257 NAVTEX — PR #317 open, wacht op test.
- #132 Wrong frequency display — not fixed.
- #256 SSTV overlay preview — enhancement.
- #252 Central logbook — enhancement.
- #251 FT8 continuous operation — core logic built, queue UI added (PR #323).
  9 demonstrations not verifiable without test.
- #249 SSTV layout templates — enhancement.
- #302 Memories paste-knop — F4NRM oppakt het.
- #283 Settings question — gesloten (fix da78715b).

## Parked

- 4 brede PSK-banken (PSK125C12, PSK500C4, PSK500RC4, PSK1000C2) — 0/3
  words. Geparkeerd tot iemand ze op de band hoort.
- Antenna modelling tool — branch fork/antenna-tool, MoM solver.
- Satellite telemetry decoders — ISS gedekt, Fox/QO-100 BPSK, per-mission
  layouts nodig.
- FT4 console, SSDV across passes, Codec2 playback — ideeën, niet gescopeerd.
- FST4W — do not ship as wired.
- SDRuno look — parked, operator's pet project.
- DMR — out (patent).
- ATS Mini / DiamondRadio — closed/unlicensed (research only).
- AppImage — beslissing nodig.

## Recently done
- 2026-10-31 — Airport card (PR 2 of 4): the pinned card lists up to 4 runways and 6 frequencies from embedded OurAirports tables (offline, dated), lengths in the chosen units, and "aircraft within" reaches 100 km instead of 25 (added by madmedicnl, owned by Roy)
- 2026-10-31 — Aircraft online relay is ON by default and one visible "Online" switch sits at the top of the AIRCRAFT window (was off, hidden under a selected card, 4 clicks away). The station keeps an "off" mark (`aircraft-online.off`); `SDROXIDE_AIRCRAFT_ONLINE=0` holds it off, `=1` on. First of 4 PRs for the airport card (runways/frequencies, Wikipedia thumbnail, local time/temperature follow) (added by madmedicnl, owned by Roy; changes a privacy default, PR for F4NRM's review)
- 2026-10-30 — Aircraft photo: the station keeps 10 thumbnails for 30 s (was 1), loads several aircraft at once (up to 3, same aircraft once) instead of answering 429, frees its place even if a download panics, a closing card releases only its own picture, and a failed thumbnail is asked again (3 times) while the card stays open (added by madmedicnl, owned by Roy)
- 2026-10-29 — Every message log keeps at most 200 (ACARS, DSC, APRS, VDL2 were 300/300/300/2000), VDL2 list newest first (added by madmedicnl, owned by Roy; touches F4NRM's DSC/NAVTEX/APRS caps, PR for his review)
- 2026-10-29 — ACARS channel chips tune the VFO in use, light the real channel, grey out unreachable ones (added by madmedicnl, owned by Roy)
- 2026-10-29 — Aircraft online read: a failing or slow card no longer blocks the others (3 at once, 15 s limit, retry 30 s, failure says so) (added by madmedicnl, owned by Roy)
- 2026-10-29 — Aircraft photo a quarter larger: map card 84→106 px, Cards view 140→175 px (added by madmedicnl, owned by Roy)
- 2026-10-29 — Language selector in Settings → UI with Apply button (PR #339, #331)
- 2026-10-29 — S-meter calibration removed from UI and reading (PR #341, #340)
- 2026-10-29 — SSTV received-image viewer nav + composer text transfer (PR #333)
- 2026-10-29 — UDP feed rolling-window unit tests added (PR #318)

- 2026-10-29 — ADS-B overlay (adsb.lol, spearheads, relay fix, manual)
- 2026-10-29 — NAVTEX restore to F4NRM's PR #157/#161
- 2026-10-29 — NAVTEX sync-drop (DK8JV suggestion)
- 2026-10-29 — UDP feed VDL2/ACARS/DSC/HFDL + rolling-window fix
- 2026-10-29 — Airspy HF+ per-rate usable span
- 2026-10-29 — Antenna calculator (MoM solver, presets, 3D, Yagi optimizer)
- 2026-10-29 — FT8 auto queue UI
- 2026-10-29 — Grey-line gradient confirmed
- 2026-10-29 — S-meter troubleshooting (wireless mouse RFI) added to manual
- 2026-10-29 — STANAG 4285 finished (coded payload not implemented — legal)
- 2026-10-29 — Olivia proven (RX+TX)
- 2026-10-29 — S-meter still an issue
- 2026-10-31 — Airport card (PR 4 of 4): the pinned card shows the airport's local time and temperature, online only — the station asks Open-Meteo (`/airport-weather?lat=&lon=`, only the airport's position rounded to ~1 km, kept 30 s in memory, same sign-in/switch/ceiling as the other online routes, credit shown); local time is the client's UTC clock moved by the zone's offset (summer time included), 12 h/°F in Imperial and 24 h/°C otherwise; offline it reads "UTC hh:mm (local time: offline)", never an invented local time (added by madmedicnl, owned by Roy)
- 2026-10-28 — NAVTEX false-sync fix
- 2026-10-28 — WEFAX negative auto-correct
- 2026-10-28 — #265 "outside receive range" notice fixed (logged, not shown)
- 2026-10-27 — Aircraft window clean-up (10 PRs)
- 2026-10-27 — #283 settings question resolved (da78715b)
- 2026-10-31 — Airport card (PR 3 of 4): the pinned card shows the airport's Wikipedia thumbnail, asked of the station (`/airport-photo?lang=&title=`, hosts fixed to `*.wikipedia.org` and `upload.wikimedia.org`), held in memory for the open card only and dropped when it closes, credit links to the article; `PhotoStore` is now generic on its key (added by madmedicnl, owned by Roy)
