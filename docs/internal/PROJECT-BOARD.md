# SDR Oxide Brown — project board content

Three columns: Achieved → **Done**, Open → **In Progress**, Roadmap → **Todo**.
This is the view; the working queue is [`../../TODO.md`](../../TODO.md) and the
rules are [`../../AGENTS.md`](../../AGENTS.md).

Scripted via `gh project item-create` + `item-edit` once the `project` scope is
granted (`gh auth refresh -h github.com -s project`).

Last brought up to date **2026-10-24** (crate 2.1.0, `PROTO_VERSION` 241 on
`main`; 242 claimed by PR #174 and 243 by the voice-hunting PR).

## Achieved (Done)

**Releases**
- **2.1.0_brown** and its point releases (2.1.0.1, .2), with 2.1.0.2.1 as the
  current pre-release. The big mode release.
- 2.0.13 and its point releases; the Brown rename, the config directory split,
  the Windows MSI identity, the site at `sdrbrown.info`.

**Modes**
- **The mode-3 work**: PSK plain, coded (`BPSK63F` + the `R` set) and the
  multi-carrier `C`/`RC` banks, MFSK (eleven), Contestia (nineteen), MT63,
  DominoEX (nine), THOR (fifteen), DTMF, and **STANAG 4285** with Annex E.
- Interoperable and field-confirmed: FT8/FT4/FT2 with WSJT-CB callsigns and the
  wide grammar, JS8, JT65/JT9, FST4, Q65, MSK144, FSK441 (RX and TX), JTTY,
  UVPacket, ALE (RX), Olivia (RX proven on air), RTTY, Hell, SYNOP/SHIP (BETA),
  NAVTEX, DSC (MF/HF), WEFAX, SSTV/RIFP, ACARS, VDL2, HFDL, ADS-B, AIS, APRS,
  DAB/DAB+, DRM, HD Radio, RADE, packet.

**Radio and station**
- HPSDR (Protocol 1 and 2), the Hermes-Lite 2 family and the **Brick2** — the
  field audit, the paced transmit stream and the Hermes-emulation PA fix.
- One I/Q switch per radio (#170), Swap I/Q for SDR sources, per-memory colours,
  the ATS Mini, ELAD FDM-DUO power steps, transverter tables, rotator (manual,
  EasyComm/GS-232, PSTRotator), Wavelog, eQSL QTH, QRZ/LoTW/ClubLog uploads.

**UI**
- **Classic UI** (SDR++'s left column, reorderable sections, rail to fold onto).
- **The detachable window set** — the SDRuno-shaped windows F4NRM is taking
  further: SP1/AUX SP, the band window, the tool windows with remembered
  geometry, workspaces, Settings, and the decoders as they become detachable.
- **ADS-B** map with baked-in airports and navaids, three styles, hover names and
  double-click tuning; grey line, meteor calendar, IBP beacons, Kp history, local
  solar time, SIG ID.
- The settings page on grid rows, with `SDROXIDE_SETTINGS_TRACE` for the overlap
  reports; the phone/tablet layout and the anti-touch padlock.

**Listener and CB**
- The SWL reception log with QSL tracking and CSV/ADIF export, ECSS, the receive
  tone, time-shift replay, scheduled recordings, broadcast scanning, the EiBi and
  Aoki schedules, local solar time.
- CB: 11 m used in full (voice and all the digital modes), WSJT-CB
  interoperability, the channel plans, SSTV on 27.700, CB-first band handling.
- **Voice hunting in the scanner** — a VOICE switch that stops a scan only where
  somebody is talking, with the reading shown as it runs (in the PR, 243).

**Health**
- The gate is silent; the mode/band tables, `PROTO_VERSION` register, `Action`
  and `NrLevel` discriminant tests all hold.
- Branch and disk hygiene: 38 branches pruned and 900 GB of build artifacts
  removed; the site, the issue form with a required version field, and the
  offline-by-default rule.

## Open (In Progress)

- **Issue #146, reopened** — LISTEN in the band/mode menu hid nothing from the
  BAND chip's popup: the answer lived in a flag that only one of the three places
  drawing the selector set. **PR #183** derives it from the tab instead, adds a
  hint where the switch is, and documents it in the manual §2.4. It needs the
  reporter to confirm before the issue closes.
- **The debug UI suite does not run** — `cargo test -p sdroxide-ui --lib` aborts
  on `epaint`'s `TexturesDelta` `debug_assert!`; `--release` is green (961). One
  `out.textures_delta.clear()` per drawing test, across UI test files.
- **The mode work is the queue.** [`MODE-PHASE-4-PROMPT.md`](MODE-PHASE-4-PROMPT.md)
  — finish the modes, TX for everything fldigi can transmit, vendor when the port
  and the bench are exhausted. Journal and measurements:
  [`MODE-PROGRESS.md`](MODE-PROGRESS.md). Open: the 8PSK vestigial filter, the
  four wide `C` banks' carrier separation, and the bench recordings the journal's
  "Needs me" list names.
- **CIS-11** — built and measured; **PR #173** open because it is the wire
  change (`PROTO_VERSION` 241). What it still needs is a clear transmission to
  settle how the ten data bits divide into letters.
- **Brick2** — [`BRICK2-HANDOVER.md`](BRICK2-HANDOVER.md): the code half is
  merged; **#132** (`40 kHz` display) waits on one answer, **#133** (LSB as USB)
  is the HPSDR Invert-spectrum switch.
- **Detachable panels** — F4NRM's project (#166): every new panel goes in
  detachable and lands on the Detachable page in Settings.
- **#162 NAVTEX frequency** — an answer, not a change: the presets already tune
  1.710 kHz below the channel; his 516.6 is calibration or the station.
- **#153 Pluto socket stalls** — waiting on the diagnostic report.
- **#105 AppImage** — for a stable release, not the pre-releases.
- **SYNOP off air** — BETA until somebody decodes a real station.
- **The grey line over a partly daylit path** — the case it exists for, not yet
  seen.
- **The S-meter and the mouse** — one question on the issue, and the answer
  decides whether it is an engine fault or the needle's animation.

## Roadmap (Todo)

- **The mode table** at the end of the phase-4 brief: every mode we ship, five
  columns — name, rx, tx, vendored, tested. The operator's deliverable.
- **Performance pass** — per-frame paths, measured rather than guessed; the
  decode-row loop is the prime suspect and the ignore list is *not* a problem
  (7.6 µs/frame, measured). Method and what is ruled out are in
  [`SESSIONS.md`](SESSIONS.md).
- **M17** (RX first) and **ALE Selcall**; **DSC**'s audio front end needs a real
  burst; **Inmarsat** (upstream #187).
- Decoder candidates: POCSAG/FLEX, ARDOP (modem proven on reference WAVs, needs
  a station), FLARM/OGN, UAT 978.
- Listener: **D-RAP absorption map**, the **azimuthal map**, solar-wind/Bz
  sparklines, the solar-cycle chart, broadcast noise reduction.
- **CW reception reporting** to PSK Reporter.
- **The antenna modelling tool** — branch `fork/antenna-tool`, WIP, kept.
- Upstream requests this fork may pick up: the list is [`../../TODO.md`](../../TODO.md).
