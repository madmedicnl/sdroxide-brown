# TODO — SDR Oxide Brown

What this fork is working on, kept here so it survives a session and is visible
on the repo. Last updated **2026-10-25** by madmedicnl's agent.

State: `main` at crate **2.1.0**, `PROTO_VERSION` **243** (`VERSION_BYTE`
`0x3F`) — 242 was 8PSK and 243 voice hunting, both merged on 2026-10-24, and
the aircraft online work (PR A and PR B) is merged and the hover card is next to
be read. Latest release **v2.1.0.2_brown** (with **v2.1.0.2.1_brown** as the
current pre-release), five open issues. The rules are
in [`AGENTS.md`](AGENTS.md); the history is in
[`docs/internal/SESSIONS.md`](docs/internal/SESSIONS.md).

Items marked `#NNN` are **upstream** requests this fork may pick up; they are the
maintainer's queue there, not a promise.

---

## Now

0. **Mode panels docked by default, and no window on mode entry (#215, #217)** —
   **PR #219**, awaiting review. On the first frame of every mode the program opened
   that mode's panel as a window *and* saved the choice into `tool_windows_open`, so
   (a) the pane below the waterfall was always suppressed and (b) SSTV opened twice,
   because `is_image()` covers Sstv/SstvFm/Rifp and SSTV already had a window. The
   block is gone; a panel docks and a window is the operator's (Detachable page, or
   the dock chip in a window). The bits 2.1.0.x wrote are cleared **once**
   (`UiSettings::tool_windows_repaired`), leaving non-panel windows alone. Two frame
   tests: entering twenty-odd modes opens nothing, and the repair clears the mode
   bits once and only once. **SSTV and the screens are F4NRM's**, so it is a PR.

0. **SSTV opened two windows (#215) — fixed by the same PR as #217.** Reported by
   `drdatapi` against 2.1.0.2.5 and reproduced by the operator: entering SSTV or
   SSTV-FM opened both the `SSTV` window and a generic `IMAGE` one on the same
   panel, because `IMAGE`'s auto-open was written against `is_image()`, which is
   `Sstv | SstvFm | Rifp`. One decision (`image_window_for`) now picks the window
   for the auto-open, the draw and the docked panel, and a frame test enters each
   image mode and counts what opened — it fails on the release with the report's own
   symptom. Also cured a latent blanking: an `IMAGE` left open from RIFP would have
   hidden SSTV's panel entirely. **SSTV is F4NRM's**, so it is a PR.

0. **Squelch hold-open from the SQL label — PR #218, awaiting review.** Asked for
   by the operator 2026-10-25: a click on the word **SQL** holds the squelch open
   (green letters) and a second click puts back the operator's own number, with no
   chip and no extra width. It is **UI work, so it is a PR** rather than a commit to
   `main` — the screens are F4NRM's. Built and tested here (1240 UI tests, the ink
   and the width measured headlessly); **nobody has looked at it on a screen yet**,
   which is the one thing left.

0. **⛔ Our own PRs are never merged by us.** They wait for **F4NRM to review or
   merge them himself** — his instruction through the operator, 2026-10-25. A local
   merge for testing (`local/*` + a build into `~/.cargo/bin`) is fine; origin is
   theirs. The rule now lives in [`AGENTS.md`](AGENTS.md) under "Pull requests".

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

0b. **Aircraft online info — merged (PR A relay, PR B clients).** Left: a live run
   against a station with the relay on, adsbdb and Planespotters answering, and a
   look at whether the CDN lets a browser load the thumbnail. The screen is
   F4NRM's to look at. Asked for by the operator.

0c. **Aircraft hover card — merged (#205).** Resting the pointer on an aircraft on
   the map shows a compact card: the radio's lines first (green), then the online
   ones the open card has already fetched (blue), the badge at the top, the
   picture only if it is already in memory. It asks for nothing. Asked for by the
   operator.

0d. **Aircraft pinned card — merged (#206).** A
   click on an aircraft on the map pins a small card beside it, with a clickable
   Planespotters link and the thumbnail if it is already in memory. It stays until
   the ✕ or Esc, or until another aircraft is clicked; a click on empty map does not
   close it. A click or a hover asks for nothing; a "Look up online" button in the
   card asks the station's relay for that one aircraft. Seen on a real screen with a
   real station. Asked for by the operator.

0e. **Aircraft search and filters — merged (#207).**
   One search box over the ADS-B table, the Cards list and the map: ICAO, callsign,
   registration, type, operator (`reg:` `type:` `op:` limit a word to a field). The
   map hides what does not match and the pinned or selected aircraft stays. It reads
   memory only and asks for nothing, so type and operator are found only once an
   aircraft has been looked up. Asked for by the operator.

0k. **Aircraft export — open for review (branch `claude/aircraft-export`).** **CSV** and
   **JSON** buttons save the tracked fleet: the station's 24 h history table
   (`/aircraft-export`) when it keeps one, else a table made from the cards held. No
   positions, no photo or address, no session; spreadsheet formulas made inert. Left: a
   look at the buttons on a real screen and a file opened in a real spreadsheet.
   Asked for by the operator.
0j. **Aircraft alerts — open for review (branch `claude/aircraft-alerts`).** A **Watch**
   view in the AIRCRAFT window: a list of callsigns and registrations kept by the
   station (`aircraft-watch.json`, `/aircraft-watch`), and an alert, a line and a count
   on the AIRCRAFT chip, when one appears or comes back. No sound, no mail. Left: a look
   at it on a real screen with a real station, and the same list from the browser.
   Asked for by the operator.
0i. **Aircraft statistics — open for review (branch `claude/aircraft-stats`).** A
   **Stats** view in the AIRCRAFT window: top categories and airlines from the radio,
   top models, operators and routes from the relay's cards, most first or A–Z. The
   station's history list now carries the card facts so the last 24 h count too. Left:
   a look at it on a real screen with a real station. Asked for by the operator.
0h. **Aircraft history compression — open for review (branch
   `claude/aircraft-history-compression`).** `aircraft-history.json` is written gzipped,
   same name, once it passes 5 MB; either form is read. Still opt-in, still atomic.
   Left: the server wiring and its tests need the opus download (CI); a look at a
   real, big file. Asked for by the operator.

0g. **Copy summary on the aircraft card — merged (#210).** A button in the Cards list copies the card as text, each
   line marked radio / online / station history, with the photo credit and page link.
   Position, last message and the thumbnail address are withheld. Left: a look at the
   button on a real screen and a paste into a real app. Asked for by the operator.

0f. **Aircraft history on the station — merged (#208).** The station keeps a day of what its radios
   heard of each aircraft — positions, callsigns, registrations and the relay's card —
   in `aircraft-history.json`, **off by default** (`SDROXIDE_AIRCRAFT_HISTORY=1` turns it
   on), served on `/aircraft-history`. **Show history** in the pinned card and the
   Cards list draws the trail on the map and adds the record to the card. Left: a
   look at it on a real screen with a real station; the server's other integration
   tests and the workspace check need the opus download. Asked for by the operator.

1. **The mode work — the live queue, and it has its own brief.** The table is
   **done** (operator, 2026-10-25: "that table is already done a few sessions ago"),
   so it is not the next task. **What is left, checked against the code on
   2026-10-25** — every line read out of `Mode::is_rx_only`, the refusal texts in
   `text_modem.rs`, and `MODE-PROGRESS.md`'s table:
   - **STANAG 4285's coded payload** — the programme's head. Receiver measured;
     the code has never been read off the air. Needs a capture with 93 continuous
     usable frames; none of ours has more than 82.
   - **TX where fldigi sends and we do not** — the coded PSK families (**34
     waveforms**, "the coded PSK transmitter is not written" — the largest single
     gap in the tree), **8PSK** ("no eight-phase modulator"), **DominoEX**,
     **DTMF**, **ARDOP** (no ARQ), **STANAG 4285** (no modulator), and
     **JT65 / JT9 / FST4 / Q65 / MSK144** ("transmit is not wired in this build").
     - **The coded PSK transmitter, in hand (2026-10-25).** The pieces are all
       there and the shape is settled: `ConvEncoder::new(ConvCode::PSK_R)` gives
       the two coded bits per data bit, `mfsk_rx::Interleaver::new(2, depth, true)`
       is already the **transmit** direction (the receiver's mirror is
       `forward: false`), `mfsk_word` gives the varicode, and `PskCoded` carries
       baud/carriers/depth/fec per variant. **`psk.rs`'s own test module already
       renders a correct coded signal** (`pskr_audio`, proven by
       `the_pskr_chain_round_trips_its_own_bits` at depths None/40/80/160) — so
       the first step is to promote that into a real `PskrTx`, not to invent a
       modulator. Then: the single-carrier coded five (BPSK63F + the four `R`)
       first, the multi-carrier `C`/`RC` next (bits distributed round-robin over
       carriers `1.4 × baud` apart, each with its own differential state), and a
       loopback per variant as the test. `text_modem.rs`'s refusal narrows to the
       families not yet wired, so the honest message stays on the chip.
   - **RX faults on modes we ship** — PSK 31/63 lose a word, BPSK1000 locks from
     its second in; PSK500C4/1000C2/500RC4 **0/3** and four RC banks 1–2 of 3;
     8PSK **0/4**; Hell five of seven at 0.33–0.48; MFSK8/MFSK11; DominoEX 3 of 9
     short; THOR 4 of 15; Contestia 64/500; MT63 2KL partial and 1KS/1KL/500L not
     offered; FSQ unproven; PI4 and UVPacket read nothing.
   - **No decoder at all, recordings already in `samples/fldigi`** — **OFDM (3),
     THROB (3), SCAMP (6), SITOR-B (1)**.
   - **Needs the bench, not a session** — ALE (one burst), ARDOP (a station),
     STANAG (a coded tape), 8PSK and FSQ (a clean recording).
   - The line below still holds, and F4NRM's list is still his.
   [`docs/internal/MODE-PHASE-4-PROMPT.md`](docs/internal/MODE-PHASE-4-PROMPT.md):
   finish the missing modes and get TX working for everything fldigi can
   transmit, and **vendor a mode when the port and the bench have both been
   exhausted**. [`MODE-PROGRESS.md`](docs/internal/MODE-PROGRESS.md) is the
   journal and the measurements; the five-column mode table at the end of the
   brief is the deliverable.
   - **STANAG 4285 — the demodulator is finished; the recordings are too short.**
     The equaliser is measured and kept (mean constellation 0.344 → 0.451, usable
     frames +45 %), and a real bug came out of comparing against the independent
     implementation: the **deinterleaver dropped one soft decision in 32** (its
     shortest row was read before it was written) — invisible to a loopback, now
     pinned position by position. Every convention a loopback cannot falsify has
     been swept and nothing frames a message, and **no recording has the ten
     continuous seconds a message needs**: the longest run of usable frames is
     **69 / 32 / 82** against **93** needed, while our own synthetic runs 207 and
     decodes. So the fault is the material, and it is now a number: a capture with
     **93 consecutive frames above 0.9 reference concentration**. The watcher is
     running again; `s4285_probe` prints a `coded readiness` line for every file it
     writes.
     See [`STANAG4285-HANDOVER.md`](docs/internal/STANAG4285-HANDOVER.md).
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

- **STANAG 4285 — one question for Mark: what rate does MultiPSK report?** Both
  tapes now demodulate to a good BPSK eye here (data blocks 0.71–0.96, references
  0.97–0.99) and neither frames an Annex E message, so the fault is in the coded
  chain and not in the signal. MultiPSK's status line names the rate it is reading
  (600 / 300 / 150 / 75 bps), which cuts the branch search from four to one. It is
  no longer a question about the recording's strength — that was answered.
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

- **The Linux AppImage is missing from releases, and it is a decision to revisit.**
  The AppImage step still works, but it lives on the `linux-x86_64-compat` job,
  which was **dropped from normal releases at 1.9.20** on a download measurement
  (21 of 31 assets never fetched once, all of them ARM or `-compat`). So a
  release has no compat job and no AppImage. Three ways out, for the operator to
  choose when he has looked at the workflow:
  1. move the AppImage onto the regular `linux-x86_64` job — free, ships every
     release, but inherits that build's **glibc 2.39** floor (breaks 22.04,
     Debian 12, Mint 21 — the audience an AppImage is for);
  2. **a small job of its own** in the `ubuntu:22.04` container that packages
     only the AppImage — the recommended one;
  3. attach one to an existing release with `gh release upload`, built **from
     that tag** rather than `main`.
  Nothing is started; this is a note so the finding is not re-derived.

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

### 2026-10-25
- **The five slotted weak-signal modes transmit** — JT65, JT9, Q65, FST4 and
  MSK144 take WSJT-X's three words in the message box and put one burst out in
  the first armed slot, at the mode's own delay into it. Each is verified by the
  decoder this build already had, at both ends of its family where it has one
  (Q65-15A/Q65-60E, FST4-15/FST4-60A), and a message that will not pack is
  refused by name before the radio is keyed. **Not tested on the air.**
- **ARDOP's modulator** — `ardop::encode_frame` is the exact inverse of the
  demodulator and round-trips all sixteen data modes (4PSK/8PSK/16QAM at 1, 2, 4
  and 8 carriers, 4FSK at 50 and 100 baud). **Not wired to a transmit row**:
  ARDOP's over is an ARQ session, and the state machine is the remaining work.
- **The coded PSK banks all transmit** — the multi-carrier `C` and `RC` families
  were refused because their output came back mangled even on one carrier. The
  fault was the **varicode table**: `PskMultiRx` ends in `VaricodeRx` (PSK31's)
  and the coded families in `MfskVaricodeRx`, so a `C` transmission carrying the
  MFSK table decoded as near neighbours — right character count, every letter
  wrong. All 31 coded variants key now.
- **DominoEX transmits** — nine speeds, one mode, no modulator before this. It is
  THOR's front end with the code and the interleaver taken out: one to three
  nibbles per character, one per symbol, bit 3 continuing. All nine read back, and
  the controller reads the two ends of the family.
- **DTMF transmits** — the text box is the keypad: 100 ms press, 50 ms gap, the
  phone standard, every key read back once.
- **8PSK transmits** — 125/250/500 read back through our own receiver and the
  controller; 1000 baud has two characters of twenty wrong and is written down as
  open. Its receiver now matches fldigi's cross-faded waveform (before, only an
  exactly aligned symbol phase decoded at all), but the corpus recordings still
  read 0/3 — the remaining difference is the signal's amplitude envelope.
- **STANAG 4285's uncoded text reads** — the character layer cut each frame's bits
  into groups of five, but a teleprinter character is **seven** bits (start, five
  data, stop), so an uncoded 1200 bps transmission could never be read however clean
  it was. A streaming 5N1.5 framer now runs across the whole transmission, and a
  synthetic of the text a French Navy tape sends reads back letter for letter where
  the old path produced gibberish. Both tests verified failing on the old code.

### 2026-10-24
- **Aircraft hover card** — the map's tooltip over an aircraft is now a compact
  card: ICAO, flight, registration, position, altitude, speed, track, range and
  bearing, vertical rate and any emergency, each with its source and age, then
  type, operator, origin and destination when the AIRCRAFT window's card has
  already fetched them. Green "radio" and blue "online" badges, the same badge
  at the top, a grey "—" with its reason for a gap, the Planespotters credit
  always, the picture only if the card already loaded it. **It asks for nothing**
  (no request, no download), keeps nothing in the browser, and clips every value.
  Without the card the old text is shown unchanged. Wire-neutral. **Not looked at
  on a screen**: the layout is measured, not seen.
- **Aircraft online clients (PR B)** — the AIRCRAFT window's Cards view now asks
  its own station's `GET /aircraft-info/{icao}` (web, Chrome, native remote; the
  native local engine has no station and says so). Green "radio" and blue
  "online" badges, a "radio" / "radio + online" badge at the top, the radio's own
  data first and its registration winning; the thumbnail loaded in memory from
  Planespotters' host with the credit and link kept when it cannot be. Disabled
  or unsigned stops the questions for the session; a failure waits five minutes.
  No browser storage, no wire change. **Not run against a real station or the
  real services**; the full workspace check needs the network.
- **Aircraft online relay (PR A)** — an optional server-side relay for an aircraft
  card's online fields (type, operator, route from adsbdb; a photo's URL, credit
  and link from Planespotters). Off unless `SDROXIDE_AIRCRAFT_ONLINE=1`; no key,
  no account; JSON cached 24 h; the photo itself is never proxied or stored.
  Wire-neutral. **Not yet checked against the real services** (the formats were
  written from their documentation; this sandbox cannot reach them). PR B wires
  the web, Chrome and remote clients to it.
- **STANAG 4285's equaliser** — a half-symbol-spaced least-squares equaliser
  replaces the T-spaced estimate-plus-inverse, which could not describe a pulse and
  so never beat the plain symbols on any real audio. Measured over frames, not on
  the last one: mean constellation **0.344 → 0.451**, usable frames **+45 %**, and
  a two-path synthetic decodes **6 dB** weaker than the plain path can. The rate
  verdict is a cascade now, so the cleanest BPSK capture we have is no longer
  reported `unknown` (which had also suppressed its text path).
- **STANAG 4285's deinterleaver lost one soft decision in 32** — its shortest row
  holds nothing at all and was read before it was written, so every 32nd decision
  went into the trellis as a zero. The encoder carried the mirror of the same
  quirk, so the loopback passed and every test was green; it was found by diffing
  our deinterleaved stream against the independent GPL implementation's on the same
  input. Pinned by a test that fails on the old code with **199 positions
  disagree**.
- **STANAG 4285: why nothing decodes, as a number.** Every convention a loopback
  cannot falsify has been swept (24 block orders, four interleaver wirings,
  j = 3/6/12/24, every repeat factor, pair order, differential encoding) against
  the SOM, and nothing frames. The reason is the recordings: a message needs ten
  continuous seconds of usable signal (**93 frames**), and the longest run in each
  is **69 / 32 / 82**, while our own synthetic runs **207** and decodes.
  `coded_quality()` and a `coded readiness` line in the probe make that measurable
  per file, and the watcher is running again for a capture that has it.
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
