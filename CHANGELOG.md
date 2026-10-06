# Changelog

Notable changes to **SDR Oxide Brown**, newest first. The headings are the
release tags (`vX.Y.Z_brown`); the crate version is the same number without the
`_brown` suffix. The format loosely follows
[Keep a Changelog](https://keepachangelog.com/).

This file starts at **1.9.5**. The release history was reset on 2026-09-30, so
1.9.3 and 1.9.4 have no surviving tag; the fork's full feature set (and
everything before 1.9.5) is described in the
[User Manual](https://github.com/madmedicnl/sdroxide-brown/blob/main/docs/USER_MANUAL.md)
and the [README](https://github.com/madmedicnl/sdroxide-brown#readme).

## [Unreleased]

## [1.9.21_brown] - 2026-10-06

A panadapter station can decode again, a phone can reach its scope, and the
browser stops losing the screen when it sleeps.

### A transceiver's audio reaches the decoders again

Reported upstream as **#640** by ct7cht, on a TS-440 with a DigiRig for CAT and
RX audio and an RTL-SDR on the 45 MHz IF as a panadapter — and **the fault was
ours too, in all four places they found**. Where an attached receiver paints the
picture and the transceiver supplies the audio, there is **no main audio chain at
all**, and that turned out to be the one configuration in which the digital modes
could not hear anything:

- The stage that takes the transceiver's audio and hands it to the decoders sat
  behind a check for a main chain, in both the sequential and the pooled path,
  with its own early return when there was no main to read an output rate from.
  **The main chain is now the only conditional part**, and the rate falls back to
  the engine's configured output rate. FT8 and CW decode from a DigiRig again.
- The digital **channel analyzer was created with no main to feed it from** and
  chosen by frame selection anyway, so FT8 showed a seeded, frozen 3.7 kHz window
  with the live wide panadapter sitting behind it. It now requires a main chain.
- **And the test was asserting the frozen behaviour.** The panadapter test
  harness built its engine with every default, which means no audio — and no
  audio means no main chain — so the file had been running exactly this broken
  configuration while claiming to test an ordinary station. It builds a real
  station now, and the no-main case has its own test, verified to fail against
  the old code.

### The scope on a phone

- **A phone held sideways gets its spectrum back.** Hiding the spectrum was
  decided by *"the tier is a phone"*, but the reason is about a narrow window —
  *"a spectrum trace in a 360 pt-wide window…"* — and a phone in landscape is
  **852 pt wide**. The rule is now about the width, read from the window, so a
  rotation follows it.
- **And the phone's layer switches now do something.** `SHOW WATERFALL` was drawn
  inside a block skipped on exactly the tier that needed it, and the panadapter
  overrode the operator's choice on top. The waterfall-only view is now a
  *default*: hiding the waterfall is how a phone reaches the scope, which is the
  trade the SPEC popup offers on every other screen. Showing both at once on a
  narrow window is deliberately not done yet — that is the landscape rework.
- **A menu chip that opens a window no longer leaves the menu stacked over it.**
  `MEM`, `GRID`, `⚙ SETTINGS` and `? HELP` all did, because the popup closed only
  on a click outside itself.

### The browser stops losing the screen

- **The client asks the browser to keep its storage.** Sleeping the phone and
  coming back to default settings was the browser evicting this site's storage —
  where the client keeps its whole screen — and it had already cost one operator
  his control bindings on a different thread. It should have been asked for from
  the start. A browser can refuse, so saving the screen to the profile remains the
  other answer, and the settings row says which is in force.
- **In a browser the send button says it opens the mail**, because it does. It
  read "Save picture" while saving *and* opening a mail composer, with a tooltip
  that already admitted it.

**Not fixed:** the 1.9.17 phone crash is untouched, and the landscape *layout*
for a short wide screen is still open. No Olivia transmission has yet been copied
by fldigi or MultiPSK — the probe stays the way to settle that.


## [1.9.20_brown] - 2026-10-06

The CW keyer review, and the end of the one thing on Olivia nobody had been able
to test. Six defects the maintainer found in the engine-side CW keyer are fixed,
**all six of which were live in this build too** — the review was written against
a smaller version of the same code.

- **A paddle press from idle now starts an over.** It did not: the keyer's first
  element was generated and there was nothing to carry it. `set_cw_contacts` had
  **no test at all**, which is why it went unnoticed — every keyer test drove the
  block renderer, the one path that cannot show this. A press also restarts the
  idle countdown and clears the transmit watchdog, as it always should have.
- **A client that disconnects mid-press no longer keys a carrier by itself.**
  The straight key and the paddle are different things — one is a down *edge*,
  the other is *closed contacts* — and only one was being released. Because the
  lost-key-up cap deliberately does not apply to a keyer (holding a paddle
  *should* send indefinitely), that was not a truncated over; it was a carrier
  keyed by a client that no longer existed.
- **An abort releases the contacts it was holding**, so the keyer cannot begin
  the next over with no press at all. The keyer itself stays armed: it is a
  setting, not a transmission, and a real iambic keyer is never "disarmed"
  between words. Turning **hand-keying off** — the KEY chip, or a change of speed
  or iambic mode — *does* now put it down, which is what stops the straight key
  going dead once a paddle has been used.
- **A rig that keys itself is refused before the keyer is built**, not after.
  Arming first left a keyer standing on a rig that will never key it, held by
  whatever contact arrived, with the only code that releases a key being the
  straight path that rig had just refused.
- **A quick tap is no longer lost.** A press is an event and the keyer is
  sampled once per audio sample, so a press and release arriving together — a
  tap, which is the element a CW operator sends most often — was sampled only in
  its released state. The edge is now latched where the change happens, and a
  transmitted reset clears it so an abort cannot re-key what it released.
- **Iambic mode B is full squeeze memory**, which turns out to be a claim about
  the *release* and not the press. Letting go of one paddle mid-squeeze now
  leaves it remembered, so the trailing element is sent instead of the squeeze
  degenerating into dits for ever. Mode A is deliberately unchanged.

### Olivia can be tested without an over

- **Olivia's opening bracket is transmitted**: the pair of band-edge tones fldigi
  brackets every transmission with, on by default there, followed by one idle
  character. The tones sit **half a tone spacing outside** the tone bank, so they
  bracket it, each quarter is ramped so the four tone changes do not click, and
  the length comes from fldigi's own block size rather than from our sample rate.
- **And so is its closing bracket** — the same tones again, then a short
  silence. This needed a change to when the transmit path stops asking for audio,
  because anything generated after the last character was never played at all.
- **A probe writes out exactly what we would send.**
  `cargo run -p sdroxide-dsp --example olivia_tx_probe -- "CQ CQ DE W1AW" out.wav`
  Open that in **fldigi** or **MultiPSK** and what it decodes is what a station
  would have copied — with nobody's air time spent and nobody else needed at the
  other end. **Receiving was already confirmed off the air** (discussion #5);
  this is how the other direction gets tested.

### Housekeeping

- **The QSL card is gone.** A card is what a broadcaster sends *back*; composing
  one duplicated the half of the loop we already had, and did it on the wrong
  side of it.
- Olivia's transmitter no longer reports a start tone as sent text, and a
  configuration write that changes nothing the keyer sends no longer disturbs it.

**Not fixed, and not claimed to be:** the 1.9.17 phone crash is still open, and
no Olivia transmission has yet been copied by fldigi or MultiPSK — the probe is
how that gets answered, and it is answered off the air.


### Fixed

- **Buttons that open a link now open it on the desktop.** eframe implements
  `open_url` **only in its web target** — the native backend never reads the
  command, so `Context::open_url` was silently dropped. Nothing this program
  opened with it did anything in the desktop build: the reception report and the
  SSTV picture's send button, the signal-identification window's sigidwiki links,
  and the manual's own cross-reference links in **HELP**. This is not a Wayland or
  compositor matter — there was no native handler to reach in the first place.
  A real opener is used now (`xdg-open`, `open`, `start`), and an empty or
  `-`-leading URL is refused rather than passed to a shell.
- **A reception report is never lost to a missing mail handler.** Both send paths
  now put the text on the **clipboard** whatever happens and say so when the mail
  client could not be opened, because the report is the thing the operator typed
  and losing it to a silent no-op is the failure worth designing against.

- **A received SSTV picture can be sent back out, saved where you choose, or
  emailed as a QSL** (fork discussion #7). Three chips on an opened picture:
  - **Re-upload** loads the picture into the selected transmit slot so it can be
    sent again for the stations who could not copy it. It loads the slot and
    stops there — sending is still a deliberate press of **TX**, as everywhere
    else — and is greyed with a reason on a receive-only radio.
  - **Save image as…** writes a copy where you choose. Renamed from *Save
    picture…*, which read as "it has saved one"; a picture's worth of clicks
    going somewhere unexpected is how that goes wrong.
  - **Share picture…** sends someone the picture. In a **browser** it hands the
    picture to the device's own share sheet, so WhatsApp, Telegram, Signal and
    Mail are one tap away with the picture *in* the message — a `mailto:` on a
    phone opens a blank message with nothing in it, which is not sharing a
    picture. Everywhere else it saves the picture and opens a message with a
    short reception note and room for your own sentence.
    SSTV has no QSL audience: the stations on it are amateurs and 11 m operators,
    reached through QRZ and the DX communities, so this is a friend being sent a
    funny picture rather than a report to a station.
- **Note on browsers and attachments.** Where the share sheet is not available
  the picture is saved and a message opens with the text filled in, because a web
  page is not permitted to attach a file to a message. The window says which of
  the two you have before you press anything, rather than opening a composer with
  nothing in it.
- **A reception report you can mail to the station (SWL log).** A new **MAIL
  REPORT…** in the LISTEN window's entry form writes the reception report and opens
  it in your mail client. It is available when the entry carries an email address,
  which is what a **known station** means here — one the schedule holds a report
  contact for, filled in when you log from the schedule or land on a scheduled
  frequency. Otherwise the chip is greyed and says why: a report with nowhere to go
  is not a report.
  The report is built from the form **as it stands**, so a SINPO you improved and
  then mailed without saving still goes out as you typed, and from the same
  conversion SAVE uses so the two cannot disagree. **A field you left blank is
  left out entirely** — never blanked, never dashed — and an unjudged reception
  states no figures at all rather than five defaulted ones nobody heard. A
  transmission marked **Pirate** is reported plainly, as a fact the station can act
  on.
  **It sends nothing.** Two new settings under **Settings → UI** (SWL section) say
  what a report carries: **Report picture** — a path on this computer, your own
  card or a photograph, one image used for every report — and **Report note**,
  your standing note. The picture is written out and named in the message, and
  attaching it (or a soundclip from **REC**'s 30-second quick clip) is your own
  click: a program cannot put a file in a message without being a mail client,
  which is a different program with its own security to consider. Both settings
  are optional; a plain-text report is a perfectly good report.
  The picture setting is a **file path**, so it lives in its own
  `swl_report.json` beside the reception log rather than in a config that travels
  to a browser tab or a second station — a path is not an identity, and
  `swl_id` is the part of a report that is. The settings row is absent in a
  browser tab for the same reason.
- **The chip row on a received picture can no longer overflow its window.** At
  360 pt the chips and the label beside them needed about 368 pt of a 344 pt
  window, and nothing wrapped it because the row did not wrap — so this was
  already broken before any of these chips existed. It wraps now, and no single
  chip can be wider than the narrowest window.

## [1.9.19_brown] - 2026-10-05

### Fixed

- **A phone no longer draws its radios side by side.** With several radios
  open on a phone, the main area was split into equal columns — three radios on
  a 360 pt screen became 116 pt columns each, so the frequency readout was
  truncated, the S-meter unreadable and a radio's name wrapped to a single
  letter per line (fork discussion #9). The split view is not drawn on the
  phone at all now, and the radio you are working takes the window alone; the
  radios you had open are kept, so widening the window again gives the split
  back exactly as it was. Two things beyond drawing it: the radio strip has to
  read the set actually on screen, or a phone could look at one radio and have
  no way to change which, and the split button is greyed on a phone with the
  reason in its hover rather than left able to open a split nothing would show.
  Every other screen size is unchanged, including a two-way split in a
  1250 pt desktop window.
- **A renamed sound card no longer silently becomes the default.** Matching an
  audio device compared a name that the system reports differently once it is
  plugged into a different socket, so a card the operator had chosen fell back
  to "system default" — and picking the default again matched every card at
  once. Devices are now matched on their stable identifiers, with the name only
  as a last resort.
- **A stored profile's control bindings are offered instead of being dropped.**
  A remote client asking for the settings of the profile it signed in as was
  given that profile's key bindings by the server and then silently ignored
  them, so a station set up once was never actually set up. They are now
  offered, and taking them is a separate click rather than something that
  happens because a profile was loaded.

### Changed

- **The Retro Radio faceplate is gone.** It was a listener's skin over the same
  engine, with a wooden panel, one big tuning scale and a needle. It is
  removed rather than reworked, and the normal workspace is the only layout
  again. Nothing was lost with it: the faceplate held no station state of its
  own, so every control it had is still in the workspace.
- **A received SSTV picture can be sent back out.** A **Re-upload** chip beside
  **Save picture…** loads the picture into the selected transmit slot, for
  sending it back out for the stations who could not copy it (fork discussion
  #7). It loads the slot and stops there — sending is still a deliberate press
  of TX, as it is everywhere else. On a radio that cannot transmit the chip is
  greyed and says so.
- **`mfsk-core`'s FT8 signal subtraction no longer needs a whole slot.** A short
  buffer used to cancel the last part of the subtraction instead of all of it,
  so a weak signal under a stronger neighbour could be left uncancelled while
  the decode list looked perfectly normal. The upstream fix is vendored: the
  transform is now sized from the frame rather than from the buffer that
  happened to be handed in.

### Not fixed

- **The 1.9.17 phone crash is still open.** The *layout* fault behind it is
  addressed (see the phone entry above), but the crash itself has never been
  reproduced on the bench. If it recurs, the browser console output when it
  happens is what is needed — please send it.
- **The skimmer does not read a callsign off a CW signal** in its own test, and
  has not done so for several releases. The station is spotted at the right
  frequency — the window following the view works — but the callsign never
  arrives with it. This is a pre-existing failure, not something this release
  introduced, and it is left alone rather than patched blind.

## [1.9.18_brown] - 2026-10-04

### Fixed

- **The SAVE chip on a received SSTV picture now appears as soon as the
  picture does.** It showed up only after the session had been closed and
  reopened (fork discussion #7). The chip needs the picture's bytes, but the
  fetch was asked for only while the full-size *texture* was missing — and a
  picture can already have its texture (promoted from the copy that just
  arrived, or fetched before its bytes were displaced) while the bytes are
  absent or name a different picture. No request went out, so the chip stayed
  hidden until a new session re-listed and re-fetched everything. The ask now
  follows the bytes rather than the texture, still once per picture name, so a
  failed fetch is not a request every frame.
- **The phone layout keeps its controls on the screen.** On a phone the strip's
  chips are fixed-width and were measured against a desktop row, so at 360 pt
  the whole set ran past the edge of the display: the top row overlapped into a
  mess, the layout was not centred, and the controls that lost that race were
  simply not there (fork discussion #9; the base layout fault is shared with
  upstream, issue #516). A phone now keeps three chips on the row — the
  receiver, the display, and one **☰** — and gets a single nested menu behind
  the ☰, grouped by subject: **BAND**, **MODE**, **SYSTEM** (the receiver, the
  transmitter and the radio), **DECODE WINDOWS** and **EXTRAS**. Every chip the
  fork draws on a phone remains reachable; nothing is removed, it is one level
  deeper. The menu borrows the band/mode menu, the window list and the very same
  control bodies the individual chips open, so a control edited in one is edited
  in the other and the two cannot drift apart. Desktop and tablet are unchanged.
  **The crash reported against 1.9.17 is not this**, and is not claimed to be
  fixed: whole-frame regression tests at both reported geometries (360×800 and
  1440×3200) run clean on this code, and it has not been seen on the bench. If
  it happens again, the browser console output is what is needed to find it.

### Added

- **An Enigma machine, and a solver that breaks a ciphertext you copied.** An
  **ENIGMA** chip in the free-text keyboard panels (PSK, RTTY, Olivia, THOR,
  FSQ and Hell — a listener copying cipher-like text is exactly who wants a
  decryption toy) opens a forest-green Wehrmacht faceplate: the Steckerbrett
  across the top, rotor windows with Modell / Rotor / Ring pickers, and the
  QWERTZ lampboard and keyboard. Typing or clicking keys enciphers live, clicking
  a rotor window advances it, and clicking two sockets runs a cable between them.
  It is the real machine — wheels I–VIII, the M4's thin Beta and Gamma,
  reflectors B and C, the double-step gear, a first-class plugboard — and it
  keeps the flaw that makes it *this* machine, that a letter never enciphers to
  itself, which is what lets the solver prune. Give the solver a crib and it
  recovers the rotors and the start exactly and adopts them onto the faceplate;
  blind, it recovers rotors, start and rings by index of coincidence. A blind
  solve *with* a plugboard is the Bombe's problem and is deliberately not
  attempted, and the panel says which case you are in. Nothing on the air: it
  holds no station state and touches nothing but its own faceplate. Fork-only,
  no dependencies, and it works in the browser build.

### Changed

- **The FTx failures in a browser session are not the server's.** Bench-measured
  on 1.9.17: the `--server` engine produced 25 decode batches with no client
  attached and 26 batches / 280 stations in about two and a half minutes with a
  client watching, so the decode path is sound. Discussion #8's symptom is
  browser-specific or environmental — which matches the reporter's own
  minimise-and-reload clue. No code change here; it narrows where the next
  test goes.

**Wire:** unchanged (`PROTO_VERSION` 192). A client and server on any recent
version can talk to each other.

## [1.9.17_brown] - 2026-10-04

### Fixed

- **The auto-notch no longer runs in the image and tone modes.** With it on, an
  SSTV picture had its low end — around 200–290 Hz, where the sync pulses and
  dark video sit — cancelled to a dead band in the waterfall and the spectrum.
  The notch is an adaptive tone-canceller, and an SSTV line is made of steady
  tones, so it was cancelling the picture. It is now offered only in the modes
  it is meant for; the image panel (SSTV / SSTV-FM / RIFP), WEFAX, Hell and RF
  Paint are all excluded. The keyboard and data modes are unchanged.
- **A front end that is overloading now says so where you are looking.** A
  receiver driven past full scale does not decode — the audio is loud but
  carries no signal — and every panel only ever said "hunting". The SSTV,
  NAVTEX and WEFAX panels now show a clear warning to reduce the RF/LNA gain
  when the receiver is clipping, on any front end, not just an SDRplay.

### Added

- **Olivia tunes to its published calling frequency per band.** Choosing the
  mode moves the dial onto the band's Olivia centre (14.1075 MHz on 20 m) the
  same way SSTV and the slotted modes already did, instead of leaving it
  wherever the last mode put it. A dial already on one of the mode's
  frequencies is never moved.

### Changed

- **Clearer help for the two newcomer traps in NAVTEX and WEFAX.** Both are
  timed broadcasts, and both quote a *channel* frequency that the radio is
  tuned below, so "no decode" is most often the wrong dial or an empty slot,
  not a fault. The panel hovers and the manual now say so plainly: do not type
  the advertised number, click the channel button; a quiet band is normal; and
  518 kHz is a night band.

**Wire:** unchanged (`PROTO_VERSION` 192). This is a fix-and-help release.

## [1.9.16_brown] - 2026-10-04

### Fixed

- **FT8 and FTx now decode normally in a `--server` + browser session.** The
  decoder ran on a worker thread that **crashed** on the first receive slot of
  a server session (the radio attaches a moment after the engine starts, so the
  first slot can be shorter than a full FT8 frame). The crash was an
  out-of-bounds in the FT8 signal-subtraction code of the decoder library
  (`mfsk-core`), present in every published version, so **every release from
  1.9.10 to 1.9.15** decoded almost nothing in a browser session — one or two
  stations at most — while the native app was fine and the audio itself was
  good. Once the worker died the station decoded nothing at all for the rest of
  the session, with no warning on screen and no recovery from CLEAR RX; only a
  restart cleared it. Reported by kevin2008-01 (fork discussion #8) and
  reproduced on the bench. The fork now carries the library fix (reported
  upstream as mfsk-core issue #567).
- **Olivia receive decodes on the air.** The Walsh transform, and with it the
  scrambler, interleaver and the character bit order, now match fldigi's
  reference (`pj_mfsk.h`), so a real Olivia transmission copies instead of
  fragmenting into `CQ`/`ET`/`LEE`. Verified against an off-air Avalon SW Net
  recording.

### Added

- **Selectable FT8 decode depth** (Fast / Normal / Deep), on the FT8 setup
  window. Deep is the default and matches WSJT-X: it runs the checkpointed
  signal subtraction, which recovers weak signals masked by a stronger
  neighbour — measured 22 decodes against 12 on a busy reference slot, for
  about a second of decode time. Fast is the plain single pass.
- **FT8 decode in two stages**, so an auto-sequenced reply is decided from the
  quick pass inside the transmit offset and still goes out on time; the deeper
  subtraction returns after.
- **A contest logger** (fork-only): a single-operator logger opened from
  LOGBOOK → CONTEST, with CQ WW / CQ WPX / ARRL DX / EU VHF / a generic text
  contest, multipliers, a live score and rate, the session's log and Cabrillo
  export. FT8 contacts log themselves with the running session's exchange;
  hand-typed contacts go into the logbook too. The two FT8 contest layouts
  (EU VHF and the RTTY Roundup shape) pack through the decoder library's own
  unpacker.
- **NAVTEX automatic frequency control**: the decoder now tracks a small
  tuning error instead of failing to decode when the receiver is slightly off.
- **LimeSDR Mini lower transmit rates**, to clear the underruns the Mini hits
  at its higher rates.

### Changed

- **FT4's targeted pass, and FST4/FST4W's DDC**, came in with the upstream
  merge of the weak-signal modes' current decoder library. No wire-visible
  change beyond the appended `DigiConfig` fields below.

**Wire:** `PROTO_VERSION` **191 → 192** (`DigiConfig` gained `ft8_depth` and
the contest layout's `RttyRoundup`, both appended). A remote client and server
must be on the same version.

## [1.9.15_brown] - 2026-10-03

### Fixed

- **A remote client's screen settings are saved **under the profile it signed
  in as**, not as the station's shared default. On a server **with a password**
  the settings landed in the `default` bucket and `profiles` stayed empty, so a
  named profile was never created and the screen came up on defaults every
  session (fork discussion #4). The cause: on connect the server offered the set
  for the signed-in profile, `for_profile` fell back to `default` when that
  profile was empty and reported `profile: None`, and the client therefore saved
  `None` — straight back into `default`. The server now names the signed-in
  profile in the offer and the save echo, and the reply carries `has_stored` so a
  client can learn its profile name without adopting a placeholder look it has
  not saved yet. Requires a client and server on the same wire version
  (`PROTO_VERSION` 191).

## [1.9.14.1_brown] - 2026-10-02

### Fixed

- **The browser (webAssembly) build compiles again.** The engine-side CW keyer
  left one read of the local paddle in the CW panel ungated, and that field
  exists only on native Linux — so `1.9.14_brown`'s `web client (wasm)` job
  failed and nothing was published. The read is now gated exactly as the
  straight-key read beside it, so the browser reports both contacts open and the
  (portable) engine side simply sends nothing. `cargo check --target
  wasm32-unknown-unknown -p sdroxide-ui` is the check that would have caught it;
  run it for any UI change. Same release otherwise.

## [1.9.14_brown] - 2026-10-02

### Changed

- **CW keying timing now comes from the engine, so a USB paddle sends clean
  dits and dahs instead of frame-quantised ones.** The iambic keyer used to run
  in the interface, over the paddle's contacts, and publish a key-down flag; the
  CW panel turned that into `CwKey(down)` once a frame and the engine applied it
  once per transmit block. At 15 wpm a dit is 120 ms, so each element landed on
  whatever block boundary it happened to cross — audible raggedness, and a held
  paddle that behaved like a straight key instead of repeating. The contacts now
  go to the engine and the elements are made next to the transmitter, one sample
  at a time.
  A **straight key wired into a paddle box** now keys from the middle contact
  *or* the dit contact, whichever the box reports — a paddle box wires a single
  key to one contact, and reading only the middle left every straight key in a
  plain two-contact box dead and silent.
  On a **radio that keys itself from text** (a working CAT link, CW keying set to
  "CAT") hand-keying is still refused, by design: the text goes over the control
  port and the rig times the elements itself, so there is nothing between the
  hand and the air for a key to drive — a paddle no less than a straight key. The
  KEY chip now says so and names the way out instead of the refusal being
  silent. Set **Settings → Radio → CW keying = Sound card (MCW)**: the rig is
  held on a sideband and the keyer's own sidetone is transmitted as audio, which
  is the route a paddle and a straight key both drive. That route is unchanged by
  this — it was already the confirmed working one.
  Requires a client and server on the same wire version (`PROTO_VERSION` 190).

- **The Morse trainer's SEND pane reads back what you sent from the monitor
  tone**, through the same decoder the transmit path uses, instead of a separate
  decode that existed only for the trainer. Its "n elements" counter is now the
  characters actually decoded.

### Fixed

- **A client's screen settings are stored under the profile it signed in as, not
  as the station's shared default.** On a server **with a password** the settings
  were written into the `default` bucket, so a named profile was never created —
  `profiles` stayed empty — and the next person to sign in inherited the previous
  operator's theme, layout and waterfall. The web client appeared to revert to
  "the default configuration saved for everyone".
  The cause was that a client only knows the profile it was *offered*, and a
  profile that is still empty falls back to the station default *and reports that
  fallback* — so a save could only ever reach `default`, which is why no profile
  is ever created and the loop could not terminate. The server now keys the store
  on the authenticated identity; this also stops a client writing another
  profile's settings by naming it. It was invisible without a password, because
  there the station default is the correct bucket. No wire change.
  (Report and diagnosis by Roy / F6KIM in discussion #4.)

- **MCW needs the radio on a sideband, not in CW.** Where CW is transmitted as
  keyed audio through the sound card (the Sound card (MCW) route), a radio put in
  CW ignores that sound card entirely: its own keyer is selected and the audio
  input is out of the path, so nothing is modulated and nothing goes out while
  everything on screen looks correct — the tone sounds, the send is decoded, and
  the only missing thing is reaching the air. Leave the radio on **USB**, as for
  FT8. This has bitten before on another rig (#119).

- **The SSTV panel no longer says "waiting for a signal…" while one is
  present.** The LIVE pane's placeholder, shown until a picture appears, read the
  same whether there was no audio at all or a strong signal whose header had not
  locked — so the one state an operator is in when a transmission is up and no
  picture shows was the one it denied. It now says **"audio present · no SSTV
  header yet"** when the level meter is above the floor and **"no / low audio"**
  below it. No wire change, and no change to the decoder: a live scan of the 20 m,
  40 m and 80 m SSTV dials and a controlled test of the receiver found it healthy
  (a full picture to ±125 Hz carrier offset, good to ~8 dB SNR, level-independent),
  so what was missing was the panel telling the operator which case they are in.

## [1.9.14_brown] - 2026-10-02

**The consolidated notes.** 1.9.11 and 1.9.12 were tagged but never published —
their builds failed — so this is everything a user of **1.9.10** has been missing,
in one place. It replaces the per-release entries below for anyone reading it now;
those are kept for the record.

### Added

- **A KNOWN window lists the callsigns your hashes can currently resolve** — a new
  **KNOWN** chip in the general decode's filter row opens a window showing every
  callsign a hashed `<...>` on this receiver can name, newest first and with its
  country. An FT8 message carries a one-way hash of the station it addresses, so
  `<...>` cannot be read backwards by any program; the only way to name a station
  is to have heard it spelled out, and that set was previously invisible. On 11 m
  the country is the point: `19DC797` reads Netherlands and `4CB04` Argentina.
- **Grid tracker** — a GRID window drawing worked Maidenhead squares on a flat
  map, with a HEARD layer for the live decode list and a **COUNTRY** mode that
  maps worked DXCC entities, so a CB log — which has no locators — has a map too.
- **Contest logger** — a mode-agnostic session with a live score and a Cabrillo
  export, auto-filled from the FT8 side.
- **Morse trainer** — a reference table, a practice player and a Koch drill.
- **Retro Radio** faceplate for listeners: one big tuning scale and a needle,
  BAND and MODE chips, VOLUME, SQUELCH, the receive tone shelves, an S-meter,
  SCAN/SEEK and PRESET buttons.
- **FSK441 transmit** — the message loops for the length of the over.
- **Editable message buttons** for the keyboard modes (PSK / RTTY / Olivia / Thor),
  with **F1–F9** hotkeys on them.
- **Tune the radio from the 3D pass window's frequency table**, and **remember the
  3D window's size and place** across a rebuild.
- **NAVTEX** tracks the tuning error with an AFC loop instead of decoding it, so a
  dial a couple of hundred hertz off no longer turns a message into asterisks.
- **LimeSDR Mini** — the lower transmit sample rates that clear its USB underruns.
- **#600** an honest ALC reading; **#605** per-band gain memory.
- **A session ignore list** for the FT8/FT4/FT2 decode list, and a **SAVE** chip
  on every text panel so decoded text can be kept.
- **Decodes feed the propagation field**, so the 3D globe's BANDS OPEN chart is no
  longer empty on a band RBN does not carry.
- WSJT-CB **modifier suffixes** on 11 m calls (`/P`, `/QRP`, `/MM`, `/F1`), with
  the eleven-character ceiling enforced in the grammar, plus an experimental wider
  callsign grammar (off by default).
- **Screen settings follow your profile** when you are a remote client of
  `--server` — theme, layout, waterfall and spectrum, fonts, Simple UI, Retro
  Radio, the map layers — so a new browser session no longer starts on defaults.
  **Saved when you press the button, not on every change**, so one operator's
  theme cannot become the next one's on a shared station.

### Changed

- **On 11 m, a CQ run is now bounded by the unanswered-call count and not by the
  transmit watchdog.** The watchdog runs off "time since the operator acted", so
  during a CQ run — where no reply is the *expected* state — it was a clock
  cutting a run that was working normally. **11 m only**; a stalled exchange is
  still cut there deliberately. The "Give up after" hover now says what the count
  comes to at the mode's own slot length.
- **Choosing SSTV now lands on that band's SSTV frequency** — on 11 m that is
  **27.700 MHz**, the community's picture frequency — instead of sending you to the
  **⇵ FREQ** chip for a manual pick. A dial already on one of the mode's own
  frequencies is never moved.
- **11 m's SSTV band buttons now reach 27.700** rather than 27.255 (channel 23).
- The grid tracker's popup chips read **GRID/HAM** and **COUNTRY/CB**, so a
  listener who does not know callsign etiquette can see that no CB calls land on
  the grid and no ham calls land on the country list.
- The **AUTO chip's hover now shows the engine's current step**, which is the one
  fact that decides whether a station answering a CQ can be taken.

### Fixed

- **A station calling you is no longer discarded in silence.** An FT8 reply is only
  taken when the sequencer is in the state that expects one; in any other state a
  message carrying your callsign was thrown away with nothing said. It now writes a
  line in the transcript naming the station, the state and the contact in hand,
  and points at **REPLY** — said once per station, and never for a bare 73/RR73.
- **The screen settings a remote client stores are no longer behind a hidden
  gate**, and the row says where the look in force came from and what the last
  save did.
- **A hand-picked sign-off is not undone by the DX repeating** theirs.
- A **multi-byte character in a locator** can no longer panic the grid tally.
- The **CAT/Audio IQ probe** reads the opened sound card's own stream, so a stereo
  I/Q input is no longer called mono.
- The **REC popup's Quick clip row** (30 s, 1 min), and the grey line now shades
  the continents as well as the sea.
- A signed alert can be **spoken**, so a new DXCC can be heard and not only rung.

### Known limitations

- **Olivia does not interoperate yet.** The tone counts and bandwidths are named
  after the real modes and the block coding follows the protocol, but the decoder
  is not yet bit-compatible: it does not read an Olivia station and one does not
  read it. MultiPSK or fldigi can copy the same signal. The chip's hover says so.
- **Control bindings on the server profile** remain an **opt-in, off by default and
  not recommended**: on a station where the keyboard is shared, a profile carrying
  keys can rebind another operator's PTT or tuning keys. Kept because it is asked
  for, behind an explicit acknowledgement.

## [1.9.13_brown] - 2026-10-01

### Added

- **A KNOWN window lists the callsigns your hashes can currently resolve** — a new
  **KNOWN** chip in the general decode's filter row opens a window showing every
  callsign a hashed `<...>` on this receiver can name, newest first and with its
  country. An FT8 message carries a one-way hash of the station it addresses, so
  `<...>` cannot be read backwards by any program; the only way to name a station
  is to have heard it spelled out, and that set was previously invisible. On 11 m
  the country is the point: `19DC797` reads Netherlands and `4CB04` Argentina,
  resolved through WSJT-CB's own numbering. A **Find** box filters on callsign,
  country or prefix, **REFRESH** re-asks, and the header says "newest 200 of 340"
  when the decoder knows more than one reply carries. If the list is empty the
  window says which half is missing, because "nobody yet" and "this mode keeps no
  callsign table" are different facts.

### Changed

- **On 11 m, a CQ run is now bounded by the unanswered-call count and not by the
  transmit watchdog.** The watchdog runs off "time since the operator acted",
  and a station answering a CQ is expected inside about 30 seconds — so during a
  CQ run, where no reply is the *expected* state, it was a clock cutting a run
  that was working normally. It was measured cutting a run in which a station
  then answered four times over ninety seconds, from +2 dB to +12 dB, its
  callsign resolved in the clear; the answer was discarded because the watchdog
  had already stepped the sequencer back to idle, and the adopt path needs the
  calling state. **11 m only** — every other band, and the station queue, behave
  exactly as before. A **stalled exchange is still cut on 11 m**, deliberately:
  propagation can drop mid-QSO, and because 11 m transmissions are free text,
  nothing but another station's 73 on that frequency can end it. The "Give up
  after" hover now says what the count comes to at the current mode's own slot
  length, so 10 reads as about 5 minutes on FT8 and about 1 on FT2.
- **The AUTO chip's hover now shows the engine's current step.** It is the one
  fact that decides whether a station answering a CQ can be taken, and it was
  invisible.
- **Choosing SSTV now lands on that band's SSTV frequency** — pick the band, pick
  SSTV, and the dial goes where the pictures are, instead of leaving you where the
  last mode left it and sending you to the **⇵ FREQ** chip for a manual pick. On
  11 m that is **27.700 MHz**, the community's picture frequency. This is the
  existing rule that already moves the dial for FT8, FT4, FT2, JS8 and WSPR,
  extended to SSTV and SSTV-FM; it only ever rescues a dial that is nowhere
  useful, because **a dial already on one of the mode's own frequencies is never
  moved**. A memory, a band-stack entry or a frequency you set yourself is
  untouched.

### Fixed

- **A station calling you is no longer discarded in silence.** An FT8 reply is
  only taken when the sequencer is in the state that expects one; in any other
  state a message carrying your callsign in the addressee field was thrown away
  with nothing said. It now writes a line in the QSO transcript naming the
  station, the state and the contact in hand, and points at **REPLY**. Reported
  on 11 m: a station answered a CQ four times over ninety seconds, from +2 dB to
  +12 dB, with the operator's callsign resolved in the clear, and no contact was
  ever made. Said once per station rather than once per repeat, and never for a
  bare 73/RR73 — somebody finishing is not somebody calling. The commonest cause
  is now named in the notice: the **transmit watchdog** fires mid-CQ-run and
  forces the sequencer back to idle, and a station answering after that is
  discarded *by design* — an unattended station must stop transmitting, and one
  that resumed the instant somebody called would be unattended and transmitting.
  It was only ever indistinguishable from "nobody answered".
- **11 m's SSTV band buttons now reach 27.700** rather than 27.255 (channel 23).
  All three of the band's SSTV entries carried a note, so the "the plain calling
  frequency" rule found none and fell back to the lowest of the three. 27.700 is
  the unannotated primary now — the highest of the three, which is the point of it
  being freeband.

## [1.9.12_brown] - 2026-10-01

### Added

- **Control bindings can be kept on the server, behind an opt-in** — a remote
  client may store its keyboard and mouse bindings with the profile it signed in
  as, so they follow the login between browsers and devices, as the screen
  settings already do. **Off by default and marked not recommended**, because on a
  station other people use the keyboard is shared: turning it **on** asks for an
  explicit acknowledgement first, and a client that has not opted in ignores
  stored bindings entirely. Settings → UI, under "Screen settings on".

## [1.9.11_brown] - 2026-10-01

### Added

- **Grid tracker** — a GRID window that draws the log's worked Maidenhead
  squares on a flat map (amber worked, green confirmed), with a HEARD layer for
  the live decode list. A **COUNTRY** mode maps worked DXCC entities instead, so
  a CB log — which has no locators — has a map too.
- **LimeSDR Mini** — the lower transmit sample rates (100–750 ksps) that clear
  its USB underruns, offered on the Mini only.

### Fixed

- **NAVTEX** tracks the tuning error with an AFC loop instead of decoding it, so
  a dial a couple of hundred hertz off no longer turns a message into
  asterisks.
- **#600** an honest ALC reading; **#605** per-band gain memory.

## [1.9.10_brown] - 2026-09-30

### Fixed

- A bare CB call is remembered, so the identity pair that follows resolves.

### Documentation

- The user manual is published to the GitHub wiki.

## [1.9.9_brown] - 2026-09-30

### Added

- **A remote client's screen settings can live on the server**, against the
  profile it signs in as, so a new browser session no longer starts on defaults.

## [1.9.8_brown] - 2026-09-30

### Added

- **Contest logger** — a mode-agnostic session with a live score and a Cabrillo
  export.
- **FT8 decode depth** — Fast / Normal / Deep, to trade the last weak decodes
  for time.
- **Retro Radio** faceplate for listeners.
- Tune the radio from the 3D pass window's frequency table.

## [1.9.7_brown] - 2026-09-29

### Added

- **ALE** (MIL-STD-188-141A) receive, and a transmit primitive.
- **WSJT-CB modifier suffixes** on 11 m calls (`/P`, `/QRP`, `/MM`, `/F1`), with
  the eleven-character ceiling enforced in the grammar. An experimental wider
  callsign grammar is available, off by default.
- A session **ignore list** for the FT8/FT4/FT2 decode list.

### Changed

- The CW straight key is polled every frame while it is armed.

## [1.9.6_brown] - 2026-09-29

### Added

- **JTTY** receive and transmit, the WSJT-X 3.2 asynchronous text mode.
- **FSK441 transmit**.
- **Spoken alerts** — a new DX can be heard, not only rung.
- The 3D window's size and place are remembered across a rebuild.
- This station's own decodes feed the propagation field.
- A DOCK chip for a band/mode column beside the waterfall.

### Fixed

- Hermes-Lite 2 SWR telemetry for Protocol 1.
- A hand-picked sign-off is no longer undone by the DX repeating.
- The recording silence gate keeps running on a hidden tab, and its own stop is
  not read as a manual one.

### Changed

- The window's 3D geometry is no longer clamped, so a window on a second monitor
  comes back there.

## [1.9.5_brown] - 2026-09-28

### Added

- **SWL reception log** — report-sent and QSL-received tracking, an at-a-glance
  header, a single SWL LOG chip, and a station pre-fill from the schedule.
- **Brown identity** — icons and a menu entry that name the fork.
- **FT8 checkpointed signal subtraction**, like FT4 and WSJT-X, so a weak signal
  inside a stronger neighbour's bandwidth is decoded.

### Fixed

- The mono/stereo IQ stream probe reads the opened PCM's own stream, not always
  stream 0 (#588).
- The Windows MSI has its own product identity, so it no longer replaces an
  upstream install.
- The RX reset chip is drawn once, not twice.

[Unreleased]: https://github.com/madmedicnl/sdroxide-brown/compare/v1.9.12_brown...main
[1.9.12_brown]: https://github.com/madmedicnl/sdroxide-brown/compare/v1.9.11_brown...v1.9.12_brown
[1.9.11_brown]: https://github.com/madmedicnl/sdroxide-brown/compare/v1.9.10_brown...v1.9.11_brown
[1.9.10_brown]: https://github.com/madmedicnl/sdroxide-brown/compare/v1.9.9_brown...v1.9.10_brown
[1.9.9_brown]: https://github.com/madmedicnl/sdroxide-brown/compare/9cf31d2ef5198ef98d04a11c0d2ec984f22f8a1f...v1.9.9_brown
[1.9.8_brown]: https://github.com/madmedicnl/sdroxide-brown/compare/2af23541384b98c735eaadcb8727486f914ae73a...9cf31d2ef5198ef98d04a11c0d2ec984f22f8a1f
[1.9.7_brown]: https://github.com/madmedicnl/sdroxide-brown/compare/0dbfa1935f182e12c1e947531311009158a5eba9...2af23541384b98c735eaadcb8727486f914ae73a
[1.9.6_brown]: https://github.com/madmedicnl/sdroxide-brown/compare/fed08486990fd8711a4f415a86c82cac4c9a4bb5...0dbfa1935f182e12c1e947531311009158a5eba9
[1.9.5_brown]: https://github.com/madmedicnl/sdroxide-brown/compare/999a1ce70b86f24bbc48273a6537de213f7d6e59...fed08486990fd8711a4f415a86c82cac4c9a4bb5

<!-- Boundary commits are used above for 1.9.5–1.9.9: their tags were removed
     on 2026-09-30 (see AGENTS.md → "Cutting a release"), and a compare link to
     a tag that no longer exists is a 404. -->

