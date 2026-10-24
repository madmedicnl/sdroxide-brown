# TODO — SDR Oxide Brown

What this fork is working on, kept here so it survives a session and is visible
on the repo. Last updated **2026-10-24** by madmedicnl's agent.

State: `main` at crate **2.1.0**, `PROTO_VERSION` **243** (`VERSION_BYTE`
`0x3F`) — 242 was 8PSK and 243 voice hunting, both merged on 2026-10-24, and
**no PRs are open**. Latest release **v2.1.0.2_brown** (with
**v2.1.0.2.1_brown** as the current pre-release), five open issues. The rules are
in [`AGENTS.md`](AGENTS.md); the history is in
[`docs/internal/SESSIONS.md`](docs/internal/SESSIONS.md).

Items marked `#NNN` are **upstream** requests this fork may pick up; they are the
maintainer's queue there, not a promise.

---

## Now

0. **Focus, set by the operator 2026-10-24: no UI work, and the modes are the
   programme.** F4NRM is working through the detachable windows and the layout,
   so the screens are his; the only UI we touch is a new mode's own panel, or
   something **very off**. Our work is **STANAG 4285 first** — the coded payload
   has never been read off the air — then the modes that did not finish in
   2.1.0. **F4NRM's list still holds** (Olivia, RTTY, FT8, NAVTEX, WEFAX, SSTV,
   DAB, APRS, WFM, DSC, RDS), variants included — and a change that has to touch
   one of those, or one of his panels, goes up **as a PR** for him to read, never
   straight to `main`. **RSP1 is not on the list** (the operator struck it off on
   2026-10-24: it is the bench's receiver and his only wideband one).

1. **The mode work — the live queue, and it has its own brief.**
   [`docs/internal/MODE-PHASE-4-PROMPT.md`](docs/internal/MODE-PHASE-4-PROMPT.md):
   finish the missing modes and get TX working for everything fldigi can
   transmit, and **vendor a mode when the port and the bench have both been
   exhausted**. [`MODE-PROGRESS.md`](docs/internal/MODE-PROGRESS.md) is the
   journal and the measurements; the five-column mode table at the end of the
   brief is the deliverable.
   - **STANAG 4285**: the receiver locks real captures, Annex E (the code, the
     interleaver, SOM/EOM, an ITA2 reading) is built and proven on a loopback,
     and what is missing is a **coded off-air transmission**. The frequencies and
     the expected text are in
     [`STANAG4285-HANDOVER.md`](docs/internal/STANAG4285-HANDOVER.md); an RSP1
     capture of a French Navy test tape closes it.
   - What is left is mostly **measurement**, not code: THOR25/THOR44 and a
     second MFSK8 recording, a known-good FSQ recording, a BPSK1000 capture for
     the lock-time question, an ARDOP recording off air, and a THOR/DominoEX
     tone-step comparison against a real fldigi. The list with reasons is the
     "Needs me" section at the end of the journal.
   - **8PSK**: re-measured 2026-10-24 and the old "30 Hz wide" note was wrong —
     the files are 124.5 Hz wide at −3 dB with a 125 Hz baud line, right level,
     right SNR. Their constellation still does not settle (0.44–0.58 against
     0.87 for BPSK125 and 0.74 for QPSK125), and our BPSK decoder reads nothing
     from them either. **Blocked on one of two answers**: a synthetic built the
     way fldigi builds one (next step, a Rust test) or a reference recording.
   - **The four wide `C` PSK banks** need the demodulator's carrier separation
     (the recording is not the fault).
   - **JTTY: take upstream's rewrite by hand.** Upstream's #597 was our port;
     they then fixed it properly in six commits (`0d24a5a9`, `25350c0f`,
     `6771f6fa`, `33f6a8fa`, `53085277` and the 1500 Hz centre before them) —
     772 lines across `jtty_controller.rs` and `dsp/jtty.rs`: TX refusal reason,
     the TX row kept on screen, slow AGC, the frequency search, four-letter words
     as text, the reference waveform timing, a non-ASCII TX panic, burst tail
     loss and the stop button. A mode we ship, so it is this programme's work.

2. **A decoded-text feed over UDP** — asked on Reddit (2026-10-24) and worth
   having. What exists today is the **WSJT-X broadcast** (Settings → Servers):
   the FT8 family's decodes, status and logged QSOs, in WSJT-X's dialect. What
   does **not** exist is a generic feed for the text modes (RTTY, PSK, Olivia,
   THOR, MFSK, DominoEX, Hell, CW, NAVTEX, STANAG) or the wideband rows. Shape:
   one stored address per radio, one JSON line per decode (UTC, dial, mode,
   audio offset, text), off by default, next to the UDP audio sink that already
   exists in the engine. A `RadioConfig` field means a `PROTO_VERSION` bump.

3. **Brick2** — the code half is merged (#170); the radio half is a report away.
   [`docs/internal/BRICK2-HANDOVER.md`](docs/internal/BRICK2-HANDOVER.md).
   - **#132 wrong frequency display** (Thetis 7.183 vs ours 7.223, 40 kHz, with
     the spectrum going unstable under a MIDI knob). **One answer needed** —
     already asked on the issue: is the number the dial readout or the spectrum
     centre? If it is the axis, measure the rate the board actually streams
     against the rate it was asked for.
   - **#133 LSB needs USB on 40 m** — the switch is Settings → Radio → HPSDR →
     Invert spectrum, and one-switch-per-radio is now enforced (#170), so the
     generic Swap I/Q can no longer cancel it.

4. **#162 NAVTEX frequency** — the reporter had to tune 516.6 kHz instead of
   518. The presets already tune the dial **1.710 kHz below the channel**
   (516.290 and 488.290), and that rule is pinned by a test, so this is his
   receiver's calibration (`ppm`) or the station's own offset. **Answer it, do
   not change the rule.**

5. **#153 Pluto socket stalls** — APRS reception stops for a second or two and a
   socket is being replaced mid-stream. Waiting on the **diagnostic report**
   (Settings → Radio → Copy Diagnostic Report) and the precise setup.

6. **Detachable panels are F4NRM's** (discussion #166). Our part: any panel we
   touch gets made detachable and added to the Detachable page in Settings —
   APRS was the last one (#167).

## Waiting on other people

- **ALE — a recording of one real burst.** The receiver, the FEC and the tone map
  are built and round-trip a synthesized word exactly; no off-air burst has ever
  decoded. The ask is now in the manual (§3.29) and in the changelog's *Needs
  feedback* section, with the frequencies (11175 / 8992 / 15016 kHz USB, or the
  amateur channels near 14109.5 kHz). One minute of WAV finishes the mode.
- **SYNOP off air** — built and tested against four published messages, BETA
  marked, and **nobody has decoded one from a station**. One report takes the
  BETA marks off or finds the fault.
- **The grey-line gradient** — verified only on a path entirely in night. It
  needs one run over a **partly daylit** path, which is the case it exists for.
- **The S-meter jumping with the mouse** — one question is on the thread: does
  the *number* jump too, or only the needle? Number → an engine fault; needle
  only → the animation. Do not fix from the symptom.
- **#105 AppImage** — answered: planned for a stable release, not the
  pre-releases.

## Not in a release yet

On `main`, after **v2.1.0.2.1**: the Brick2 HPSDR work (paced transmit, the
Hermes-emulation PA fix, one I/Q switch per radio), the **detachable APRS decoder
panel** with the Detachable settings page, the **decoded-text UDP feed**, the
**JTTY** rebuild, the **CIS-11** mode (PR #173, `PROTO_VERSION` 241) and
**voice hunting** (PR #177, 243 / 0x3F). Open: **PR #174** (8PSK, 242 / 0x3E) and
**PR #183** (#146's LISTEN tab, no wire change).

## Parked, with the reason

- **The four wide PSK banks** — PSK125C12, PSK500C4, PSK500RC4 and PSK1000C2 read
  0 of 3 words where every narrower bank reads 3 of 3. **Measured 2026-10-24**:
  sharpening the per-carrier filter changes nothing, so it is not transition
  width; the reference decimates first and filters at the low rate, which is a
  rebuild of the multi-carrier front end shared by all 24 banks (20 of them
  working). Parked until somebody hears one of these on the air, and **named on
  the chips and in the manual** so nobody picks one and thinks the band is dead.
  Gate when it is done: `the_corpus_pskc_samples_are_measured`, those four off
  0/3 with no regression.
- **Antenna modelling tool** — branch `fork/antenna-tool`, a validated MoM solver
  (dipole, monopole over perfect ground, delta loop, Yagi). Next would be the
  source-between-terminals junction, real Fresnel ground and lumped loads. See
  `docs/antenna-tool-ideas.md`. **Do not delete this branch on a tidy-up.**
- **#657 OpenStreetMap background** — parked after scoping: the maps are already
  baked 8192×4096, our projection is equirectangular where OSM is Web Mercator,
  and a fetch stack is the wrong trade (and forbidden: no paid or keyed service,
  and the program works offline).
- **Satellite telemetry decoders** — the ISS is already covered (AFSK + AX.25,
  interoperable with DireWolf) and the Fox/QO-100 side is BPSK in
  `sdroxide-qo100`; what is left is per-mission field layouts, each needing a
  published table and a real capture. See `docs/skyroof-notes.md`.
- **FT4 console, SSDV across passes, Codec2 playback** — ideas, not scoped.

## Backlog — upstream requests this fork may pick up

**Fork-relevant** — #458 (KML export done; the bulk tour/QRZ lookup is open) ·
#322 PC keyboard as a Morse key · #536 RX audio to the rig's own speaker (HPSDR
P1 EP2) · #653 split screen · #624 FTP upload · #442 external band filters and
transverters · #521 IC-7610 internal ATU · #261 auto-tune with the rig's tuner ·
#270 ADS-B remote web view · #480 MCP server · #229 TX interlock across radios

**Smaller, hardware, or large and low** — #250 ELAD FDM-S2 · #344 Dragonfly
RX-103 · #383 RS-HFIQ · #406 remote tuner for the HL2 IO board · #448 RSPDuo +
RSP1 as a third tuner · #525 Pluto Sky R2 · #595 Perseus · #639 KV4P HT · #325
band-decoder output · #327 CI-V GPS · #571 TS-480 CAT · #591 RGO ONE · #227 USB
relay · #289 link VFO · #413 x-band repeater · #589 RPM packaging · #237 native
C-driver SDRs · #267 OpenWebRX · #416 multi-RX HL2 / Red Pitaya · #446 Ettus B210
· #449 M17 · #652 QO-100 full duplex

**Answered, kept so nobody re-opens them** — #607 DMR/YSF/D-Star: **decided
against** (the AMBE+2 patent, US 8,359,197, until 2028-05-20) · #212 NAVTEX
time-sync · #355 spectrum smoothing · #594 UDP audio · #617 HFDL frequency chips
· #623 satellite passes · #638 PSTRotator (taken from upstream) · #641 SWL
reception log · #642 S-meter calibration.

---

## Recently done

### 2026-10-24
- **Map detail** — roads and first-order internal boundaries from Natural Earth
  1:10m, drawn from 8° and 40° of span (asset +2.7 MB). **PR #184**; asked for on
  Reddit as "the map is too imprecise".
- **The debug UI suite runs again** — `cargo test -p sdroxide-ui --lib` aborted
  on a `debug_assert!` inside `epaint`'s `TexturesDelta::drop`, which a dozen UI
  tests tripped by dropping a frame unread; the abort took the other 949 tests
  with it. One `test_support::run_frame` helper now takes the deltas, and the
  twelve go through it: **961 passed, 0 failed** in debug, the same as release.
  Test-only, straight to `main`.
- **Issue #146 fixed and merged (#183)** — the LISTEN tab hid the transmit
  controls only from the docked column and the detached window, because the
  answer lived in a flag that just one of the three places drawing the band/mode
  selector set. It is derived from the tab now, the tab says where the switch is
  while it is off, and the manual documents it.
- **Voice hunting in the scanner** — a **VOICE** switch in the SCAN window stops
  the scan only on channels that sound like somebody talking, passing over
  carriers, birdies, pagers and data bursts instead of stopping on them. The
  reading is shown as it runs (*passed 27.5550 · voice 18*), so a strong signal
  going by is explained rather than looking like a deaf scan. Independent
  implementation from standard DSP — no code from VUURWERK, whose licence puts a
  commercial licence on its originals. **Merged as #177** (243 / 0x3F, renumbered
  from 242 which #174 claimed first).
- **8PSK in the program** — a third modulation choice beside BPSK and QPSK
  (receive only, three bits per symbol), proven on a synthetic because the
  corpus's 8PSK recordings carry no clean constellation. **Merged as #174**
  (242 / 0x3E).
- **CIS-11 built** — the Russian meteorological teleprinter, receive only. Framing
  measured from the recordings (ten data bits + one odd parity, not the
  catalogues' "5 + parity"), 121 of 123 characters parity-valid on the off-air
  idle file, and the model wired as `Mode::Cis11` behind the existing text panel.
  **PR #173** because it is the wire change (241 / 0x3D).
- **A decoded-text feed over UDP** — one JSON line per decode for the FT8 family,
  WSPR and every keyboard mode, on Settings → Servers (240 / 0x3C).
- **HD Radio is confirmed on the air by users** (reported on r/ShortwavePlus) —
  it moves off the "no station heard here" list and out of the feedback asks.
- **ALE takes its place on that list**: the manual now documents the mode with
  an honest *not yet proven off the air* and asks for one recording of a burst.
- **Brick2** (#170): Protocol 2 transmit is paced at the stream's own rate
  (choppy SSB on a Brick2), an HL2 on Hermes-emulation gateware is recognised so
  its PA is enabled, and **one I/Q switch per radio** — the generic Swap I/Q no
  longer cancels an interface's own inversion.
- **Detachable APRS decoder panel**, and a Detachable page in Settings (#167).

### 2026-10-23 — the 2.1.0 release
- **The mode release**: THOR (all fifteen variants, RX and TX), MFSK and MFSK8/16,
  Contestia, MT63, DominoEX (nine), thirty-two coded PSK waveforms with QPSK
  selectable, DTMF, and **STANAG 4285** with Annex E.
- **Wavelog** (your own logbook server), an **Aoki schedule from a file**,
  **SSTV pictures to an FTP server**, the **FDM-DUO's own power steps**, a
  **WAV recording** beside the MP3, **receive sensitivity** for SSTV and WEFAX,
  and **per-memory label colours**.
- **Three UI fixes from screenshots**: the mode parameter chips no longer draw
  over each other, a digital mode's readout no longer moves the dial (#156), and
  the LISTEN tab can hide the transmit controls and never grant them (#146).
- NAVTEX reads real signals (the discriminator was inverted) and gains clear_rx,
  persistence, noise reduction and a large detachable panel; DSC decodes MF/HF
  with the real M.493 code.

### 2026-10-22
- **The phase-3 PSK work finishes**: the coded families reach the panel, the
  wide `C`/`RC` banks and the `R` set are measured against recordings, THOR is
  found to have been wired to a receiver that read nothing (four faults), and
  DominoEX reads text. The 8PSK slicer is proven on a synthetic and shown to
  need its vestigial filter. STANAG 4285 locks real captures and its payload
  chain is built (a coded off-air capture is still needed to read a message).

### 2026-10-17/18
- **Classic UI** — every control in one column down the left, SDR++-style, with
  sections you can reorder, a rail to fold onto, and a guard that the column
  never draws past its own width.
- **ADS-B map**: airports and navaids baked in from public-domain data, three
  styles, hover labels, and a double-clicked station tunes the dial the mode does
  not own.
- **Settings page** rebuilt onto grid rows: no orphaned rows, no caps that walk
  the cursor backwards, notes elided with the hover, and the `SDROXIDE_SETTINGS_TRACE`
  tool for the overlap reports that could not be reproduced.
- **No release publishes by itself** — the nightly cron is gone, and the
  `nightly` release the workflow had made was removed.
- **WEFAX**: the rephase rule no longer cuts a page inside its own phasing
  signal (F4NRM's complementary start-tone fix is in the same builds), plus
  automatic start on audio tones, CLEAR RX and SAVE.
- FT8's CB CQ path restored (the streaming merge dropped the decode hook) and
  confirmed by a loopback and then a real station; the FT8/FT4/FT2 **slot bar
  follows the frame as it is received**; the web client's sound comes back after
  a long hidden spell; the playout backlog is bounded on every radio.

### 2026-10-16/17
- **S-meter calibration** back at ±5 S-units, per radio, after a widening made on
  a reading from the wrong build.
- **The ATS/RadioScript answer**: research only, nothing built — the firmware is
  unlicensed and the board has no I/Q.
- **No paid service, no API key, no account** — written down after the map-tile
  question.
- The **grey line** on the flat maps, **IBP beacons**, **Kp history**,
  **local solar time** on the schedule, and the **Brick2 report** that opened
  the HPSDR work.
