# TODO — SDR Oxide Brown

What this fork is working on, kept here so it survives a session and is visible
on the repo. Last updated **2026-10-27** by madmedicnl's agent.

State: `main` at crate **2.2.0**, `PROTO_VERSION` **256** (`VERSION_BYTE`
`0x4C`) — 256 is the DigiConfig field-order fix. Latest release
**v2.2.0.2_brown** (full: AppImage, Windows MSI 2.2.0, Apple Silicon + Intel
Mac). Two PRs of ours open: **#284** (static-link the MinGW C++ runtime in every
C++ crate) and **#285** (embed the Brown icon in the exe). The rules are in
[`AGENTS.md`](AGENTS.md); the history is in
[`docs/internal/SESSIONS.md`](docs/internal/SESSIONS.md).

Items marked `#NNN` are **upstream** requests this fork may pick up; they are the
maintainer's queue there, not a promise.

---

## Now

0. **Web client: the sound lags the waterfall and the lag grows** (operator,
   2026-10-27 — branch **ix/web-audio-playout**, PR **#286**, based on 5cbce00d).
   Playing received audio immediately on arrival in RemoteController (without
   waiting for render frames) prevents progressive latency/stutters and keeps
   audio playing in background tabs; frame-gate feeding remains preserved.

0. **CW/TCO and the flood buffer** (issue #254, F4NRM, 2026-10-27 — his
   area) — three things fixed or tracked. The effect of a weak signal on
   slow_loggers was a hidden bug: CW keying at timing-critical instant is
   logged via the telemetry/log queue, so the key shuts down for logging while
   being timed-critical — not only slow but dangerous. Fixed with keying paths
   that take care not to put keying tasks out of the direct class of the logging
   queue, preventing
 origin/main

0. **Read `docs/internal/HANDOVER-NOW.md` first** (written 2026-10-26 night): the
   release is aborted and removed, #263's fix is specified but not started, and
   the WEFAX multi-page puzzle is unreproduced. It carries the state, the three
   next actions and the traps.

0. **Windows, for a tiling WM** (the operator, 2026-10-25/26 —
   [`WINDOWS-HANDOVER.md`](docs/internal/WINDOWS-HANDOVER.md), branch
   **`local/windows`, not pushed** and not to be pushed until he has clicked it)
   — **the float has its own title bar now** (`151828df`): out · roll-up · close,
   with close docking and roll-up turning the float into a bar. That pass also
   **removed** the floating panel's chips row, the ▣ from every panel header and
   the two-button chip machinery; the panel's OS window carries one dock glyph.
   **To test**: drag by the bar, roll up/down, out to a window and back.
   Open: panels with no header chip row (SSTV, WSPR, NAVTEX…) have no ⇱ float
   chip, so they float with `Ctrl+Alt+F` or the list; the rest of step 5 (the band
   popup's chips, the radio strip's WINDOWS menu) is untouched.

0. **The picture composer's three features, and the continuous auto run** (issues
   #237/#236/#251, F4NRM, 2026-10-25/26 — his area, all **PRs**) — the station
   overlay, the layout library (save/load/rename/duplicate/forget) and the mosaic
   of up to nine received pictures are in **#240** (wire 254); the AUTO CONT role
   with retries, busy deferral and a session limit is **#253** (wire 255); the fax
   CTR anchor is **#255**. Tracking: #248, #249, #250, #251. What is left in
   those is what the tracking issues say and nothing more — new ideas wait for
   their own discussion.

0. **FT8 Auto Rx and Auto CQ are two roles, and a CQ run has a bound** (issue
   #236, F4NRM, 2026-10-25 — his area + a wire change → **PR**) — one AUTO chip
   did both jobs, so the operator could not say "only listen" or "only call".
   Now `AUTO RX` answers a CQ and never calls one, `AUTO CQ` calls and works
   whoever answers and never chases another station's CQ, and an Auto CQ run has
   `DigiConfig::auto_cq_repeats` calls before it switches to Auto Rx or stops
   (0 = no limit of its own). The count is read from the engine's exchange log,
   because the sequencer repeats the CQ by itself. `PROTO_VERSION` 250 (249 is
   PR #240's). Tests in `sdroxide_types::auto`. Not done: band rotation, which
   moves the dial unattended and needs its own decision.
0. **A receive-only mode showed a transmitter in the strip** (issue #239, F4NRM,
   2026-10-25) — the top strip asked `tx_capable()` ("can this radio transmit?")
   and not "does this mode transmit at all?", so WEFAX, NAVTEX, DSC, ADS-B,
   ACARS, VDL2 and AIS inherited the transceiver's PTT/Drive/Tune/XIT/DUPLEX. On
   3855 kHz that is a key-down with nothing to modulate on a broadcast
   frequency. Fixed with `tx_chrome_ok()` and a scan test; **PR #243**. The rest
   of his list is his WEFAX PR #164, **reverted on 2026-10-25 13:28**
   (`5e87cd34`) — the sensitivity/"false signal detector" and the automatic
   phasing calibration — which is the operator's decision to restore or explain.
   Two more findings to act on: the CTR anchor treats only SSTV as a picture
   mode, so a fax sits 1900 Hz off centre; and detaching is native-only, which
   is why the browser has no float arrow.
0. **A multi-radio browser station exported a logbook of one QSO** (issue #236
   comment, F4NRM, 2026-10-25) — the export itself is innocent. With more than
   one radio the logbook is shared, so a logged contact re-reads it first; in a
   browser there is no file to re-read, the reload answered "empty", and the log
   was replaced by the one contact just logged. Fixed in
   `crates/sdroxide-ui/src/app/{frame,persist}.rs` — `reload_qso_log` returns
   `None` when there is no store and `after_shared_log` keeps the log in hand —
   with a test that fails on the old behaviour, and pushed as a PR. Still open in
   the same area: a browser tab that is not the station writer never persists
   the logbook, which is the argument for putting the log on the wire the way
   the picture document now is.

0. **SSTV/RIFP picture composer: the document is on the wire** (#237, F4NRM,
   2026-10-25, his area + a wire change → **PR, waiting for the operator's word**)
   — the editor built earlier drew its layers from client memory, so a second
   screen composed a different picture and a restart lost the layout. Each
   transmit preset now carries its own `Composer` in `ImageSlotInfo.composer`,
   set with `Command::ImageSetComposer` and persisted in `sstv_composers.json`
   by the engine; the panel edits a claimed working copy and writes it when an
   edit settles. `PROTO_VERSION` 249, `VERSION_BYTE` 0x45. Pushed as **PR #240**
   (`fork/sstv-composer-wire`), waiting for F4NRM.



0. **TCI receive audio: a server that carries baseband audio only gets silence**
   (#group, Rick Abshier KQ4Y, 2026-10-25) — reported against upstream 1.6.10, an
   Icom 7300 MK2 through Remote Ham Radio's TCI platform with LocalMate: TCI CAT
   works, receive audio does not, and CW keys the transmitter but sends nothing.
   RHR's answer is right: their stream carries **baseband audio**, and our TCI
   receive asks for **IQ** only.

   Checked in the code: `crates/sdroxide-tci/src/protocol.rs` parses
   `DataType::RxAudio`, and `net.rs`'s binary dispatch handles **only**
   `DataType::Iq` and `DataType::TxChrono` — an `RxAudio` packet is dropped.
   `Ctrl::Attach` sends `iq_start`; `audio_start` is sent only for transmit, and
   `tci_caps` sets `audio_mode: false` unconditionally. (Transmit is already raw
   audio here, so "keys but sends nothing" is 1.6.10's limit, not the design.)

   **The fix, wire-neutral:** detect the transport at the handshake, and when
   there is no IQ stream publish `caps.audio_mode = true` at the audio rate, send
   `audio_start`, and route `RxAudio` frames into the receiver's ring as real
   samples. `audio_mode` and the audio-source path already exist for CAT rigs.

0. **Airspy HF+ Discovery: the skirt on the waterfall, and the fit on a dead
   band** (#group, Graeme Davidson, 2026-10-25) — reported on 2.1.0.2.5_brown.
   Two faults, one report, both built: the display was drawn on the radio's sample
   rate, so 14 % of a 768 kSPS picture was filter skirt and the view could be
   panned into it; and when a band measures only a few dB from top to bottom the
   auto-fit **centred** its window on the noise, putting an empty band half way up
   the colour map (his own experiment settled that second one — at 22 kHz, skirts
   off screen, FIT still landed warm).

   **PR #234** — `IqSource::usable_frac` (the HF+ 0.86, measured at 768 kSPS),
   `RadioState::usable_hz`, `PROTO_VERSION` 247 → 248, the UI's `dev_span` for the
   window, zoom clamp, pan bound and fit key, and the fit anchored at the floor
   instead of the middle. Two commits, six files. **Waiting for F4NRM to review
   and merge.** Not looked at on an HF+.

0. **⛔ Our own PRs are never merged by us** — they wait for F4NRM to review or
   merge, per the operator's instruction of 2026-10-25. Open: **#234** only.

0. **Focus, set by the operator 2026-10-24: no UI work, and the modes are the
   programme.** The half-done list is in
   [`MODE-PROGRESS.md`](docs/internal/MODE-PROGRESS.md): STANAG 4285's coded
   payload (**done** — it decodes through the vendored reference decoder), the
   **8PSK receive** corpus (0/3 — our front end smears what the reference reads),
   the four wide `C`/`RC` PSK banks (**done** — they transmit), the speeds that
   lose a word, and the MT63/Contestia/DominoEX variants. The five-column table
   the operator asked for is finished and is the deliverable.

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

## Now

0. **WEFAX detection still wrong: "the receiver is triggered without a signal"**
   (issue #91, F4NRM, 2026-10-26) — after the sensitivity restore (#245) and the
   band-table fix he reports detection and reception still wrong and false
   triggering. **Blocked on one recording** (frequency, UTC, audio or IQ) to
   reproduce; asked for exactly that on the issue. Not a guess to be made.



- **2026-10-26** — WEFAX on the bench: 150 s each on 3855 kHz (gain 40) and
  13 882.5 kHz (gain 20). A strong **carrier** on both, no modulated chart —
  the panel stayed "listening", which is what the code is meant to do with a
  standing carrier. Not a decoder fault; nothing scheduled was on the air at
  10:40 UTC. 40 dB on the long wire showed broadband speckle; 20 dB was clean.



- **2026-10-26** — the review round: F4NRM merged #242, #243, #245, #246, #255;
  #240 (composer, wire 254) and #241 (logbook fix) are approved and closed,
  waiting for a hand merge; #253 (AUTO CONT, wire 255) is open with the
  **two-hour cap** at the maintainer's instruction. #244 and #254 fixed on `main`
  and closed. Combined install 10:28, wire 256.
- **2026-10-26** — the PSK air-loopback tool (`crates/sdroxide-dsp/examples/
  psk_air_loopback.rs`): self-test 10/10, air path not yet, and the fault is the
  tool's own audio recovery (proven with a synthetic capture).



- **2026-10-26** — scan: the voice gate gave the raised listen time back (issue
  #254), `voice_dwell` a pure function with four cases tested. On `main`
  (`3731da4d`).
- **2026-10-26** — the notes now say how to read a machine-drafted review: short
  answers, suggestions only, one bounded scope per item (AGENTS.md).

## Recently done

- **2026-10-27** — the Aircraft window clean-up: ten PRs open, none merged
  (#266 English and no browser translation, #267 units, #268 duplicates, #269
  columns, #270 map-style buttons, #271 airport card, #272 mode buttons, #273
  VDL2/ACARS views, #274 online-lookup switch, #276 button column scrolls at the
  smallest size). Not yet seen in a picture; #274 and
  #272 were tested with a stubbed RADE crate, not on a radio. Waiting on review.
- **2026-10-26** — the float's **own title bar** (`151828df`, on `local/windows`,
  not pushed): out to a window · roll up · close (which docks), with the mode name
  on the left. Roll-up is the laptop answer — the float becomes a bar. And the
  first subtraction of the thread: the floating panel's chips row is gone, the
  panel's OS window carries one dock picture, and the ▣ is out of every panel
  header (the list is the ▣ beside the gear and `Ctrl+Alt+W` again).
- **2026-10-26** — the ▣'s list opens where it was clicked (`f52099af`): an aside
  popup anchored to the chip. **Superseded** by `151828df`, which removed the
  two-button chip and the popup with it.
- **2026-10-26** — the Classic strip, and the ▣ where the panels are
  (`7574093c`, on `local/windows`): in Classic the strip re-derived "is the column
  here?" from the width left *after* the column took its share, so a Classic
  window with a panel detached drew the **full strip under the frequency** beside
  a column holding the same controls. One answer now, published where the column
  is drawn, and a child window publishes `false` so the console keeps its
  controls. The ▣ is now in every panel header that has a chip row (SSTV, the FT8
  family, JS8, AtCHAT, packet, ADS-B) and in the floating/detached panel rows; the
  panels with no chip row (WSPR, PI4, NAVTEX, FSQ, JTTY, CW, RADE, RF Paint, text
  modems) need a row before they can carry it — **waiting on the operator**.
- **2026-10-26** — windows, step 4 (`1dd73489`, `a8a2a842`, on `local/windows`):
  **`Ctrl+Alt+W`** opens the module-windows list anywhere (it is a window of its
  own now, so the key has somewhere to open it with every module docked), the
  same chord closes it, **`Ctrl+Alt+F`** floats or docks the operating panel, and
  **`Ctrl+Alt+1…9`** / **`0`** call back a saved workspace — an empty slot says so
  on the banner. Applying a workspace docks the floating panel, because a
  workspace stores windows and not floats. Not built: the list's filter box and
  arrow-key navigation.
- **2026-10-26** — windows, step 3 (`aa716e97`, on `local/windows`): a **▣**
  beside the gear opens **MODULE WINDOWS** — one row per detachable module with
  the three places it can be (here / floating over this window / its own
  window), the one in force lit, a press to move it, and DOCK ALL / UNDOCK ALL.
  The list is the recovery: a window left on another workspace is still a row.
  Floating is the operating panel's only, and its slot is empty on the other
  rows. The chip is on the strip **only while a module is out** — as a permanent
  ninth chip it cost the widest 1600 pt strip a row. Preview:
  `module_windows_preview`.
- **2026-10-26** — windows, step 2 (`3a5b1d9e`, on `local/windows`): a panel
  window under 600 pt wide draws its own **DECODES / QSO** row (no waterfall
  tab — the picture is the main window's) and shows one pane at a time, so a
  narrow window no longer cuts its QSO chips off at the edge. Its pane is the
  window's own, swapped in around the draw; the floating panel gets the same and
  stops growing to fit two columns. Widening past 600 brings both back. Looked
  at both widths. Still owed in step 2: in Classic the detached panadapter
  should draw no tools at all (its DISP chip remains).
- **2026-10-26** — windows, step 1 of the handover (`196827a3`, on
  `local/windows`): a detached window answers the layout questions for itself —
  its own width earns the tier, it has no band column to dock, and its ids are
  the focused radio's — and the main window's answers come back when its draw
  ends. The band dock's room moved from an app field to a context register. New
  `floating_panel_preview` tool; at 560 pt the FT8 panel still overflows, which
  is step 2's pane tabs.
- **2026-10-26** — AppImage for #105: the build step existed but its job
  (`linux-x86_64-compat`) was not in a release matrix, so no AppImage was ever
  produced. The compat job now comes to every release, `cat` only; a release
  matrix is nine jobs. `08e2e125`.



### 2026-10-25
- **STANAG 4285 decodes off the air** — the vendored GPL reference decoder
  (`vendor/s4285-ref`, driven from `crates/sdroxide-s4285`) runs beside our own
  receiver and the console shows its text. This station's 24 Oct capture of
  13031.2 kHz reads a French Navy tape: `FAAA`, `DE FUF`, `ZNR UUUUU`,
  `ZUI TESTING`, `RYRYRYRY…`. Our own chain still does not read the coded
  payload; the Rust port of that equaliser does not converge yet.
- **Airspy HF+ waterfall span** — PR #234, wire 247 → 248: the display uses the
  usable band rather than the sample rate, so the filter skirt is not drawn, not
  pannable and not fitted. Not looked at on an HF+.


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
