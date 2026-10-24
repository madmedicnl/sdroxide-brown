# Autonomy journal

The running record of work done without the operator at the screen. **Newest
first**, one entry per work item, written when the item ends — a session can be
closed without warning, so nothing is held back for a tidy write-up at the end.

The rules for working this way are under "Working on your own" in
[`../../AGENTS.md`](../../AGENTS.md). The queue is [`../../TODO.md`](../../TODO.md)
and the view is [`PROJECT-BOARD.md`](PROJECT-BOARD.md). This file is not a
narrative and not a changelog: it is the state of play, so the next session (or
the operator after a night away) can start from it in one read.

Each entry, in this order:

- **Summary** — three or four plain sentences for the operator: what changed for
  him, what he can look at, what is still not done, what comes next. No crate
  names, no file paths, no jargon.
- **Done** — the item, the commits, and the commands with their numbers.
- **Not done / blocked** — what stopped, and the single question or dependency.
- **Next** — the first step of the next item.

---

## 2026-10-24 (MT63) — the transmit fold, and the lock's acceptance is still the wall

**Summary.** MT63's transmit half was broken for two of its six variants and is now
right: our MT63-1K and MT63-500 signals were synthesised at a rate that put the whole
bank above Nyquist, so they folded into a mirrored signal that nothing — including
our own receiver — could read. All six variants now read their own transmission
back. The receive half is half done: five of the six corpus samples decode whole,
but only the batch decoder finds them, because the live lock still trusts the dial
and the samples sit 20–50 Hz away. The measured centre that would fix that also lets
the lock onto noise, and I could not separate the two in this session — that is the
next piece of work, and it is written down rather than half-shipped.

**Done.**
- **The fold** (`a558d624`, swept into a STANAG commit by a `git add -A` in a shared
  tree; named in `c8191c35`): the modulator synthesised at `variant.rate()` =
  `2 × bandwidth` while the bank sits at `centre ± bandwidth/2`. Fine for 2K at
  1500 Hz (500–1500 Hz at 4000 samples/s), broken for 1K (1000–1500 at 2000) and for
  500 (1250–1750 at 1000): the whole bank above Nyquist, folded. Now
  `max(2.2 × (centre + bandwidth/2), variant rate)` with the symbol length scaled to
  match. `a_synthetic_mt63_transmission_reads_back` covers all six.
- **The probes removed** (`c8191c35`): four debug functions and an orphaned gate that
  the same sweep carried, two of which no longer compiled as written, and which made
  the gate emit duplicate-attribute warnings. Plus a one-word `let mut` in
  `sdroxide-types/src/ui.rs` that was the only other warning.
- **Measured on the receive side:** with a 5 Hz centre sweep, 2KS 4/4 at 1665 Hz,
  500S 4/4 at 1690, 2KL 2/2 at 1680, 500L 2/2 at 1720, 1KS 2/2 at 1705, 1KL nothing
  anywhere. So the decoder handles the narrow banks; the *live* lock is the gap.
- **Experiments that did not ship, with their numbers.** A measured centre (the
  comb's outermost-carrier midpoint, `measure_centre`) plus text-first scoring took
  500S from not locking to 4/4 and 1KS to 3/4 — and let the lock onto noise. A
  character-agreement test between two decodes a hair apart separated them 71 %
  against 58 %, too thin. A shared-word test also failed, because both decodes of
  *noise* return the same wrong text — the nudge is smaller than the symbol, so it
  measures determinism, not signal. A lopsided-histogram test (real decodes are
  dominated by idle characters, noise is near-uniform) rejected real 2KL/1KS as well
  as noise. None of it shipped: the tree is green and these are recorded for the next
  attempt.

**Not done / blocked.** The live lock's acceptance. What it needs is a test that is
**temporal** — a decode that keeps reading as more audio arrives, versus one that
changes — because the one-shot statistics genuinely overlap at the first attempt
(both score textiness 48–76 and a Walsh median SNR near 3).

**Next.** That temporal acceptance, then the live corpus gate for all six.

---

## 2026-10-24 (STANAG) — the audio band was not the fault, and the data path is

**Summary.** The one coded tape we have — a French Navy FUF test transmission —
still does not decode here, but the reason we had written down was wrong, and that
is now measured rather than assumed. Everything up to the payload reads perfectly
through an audio chain as narrow as the tape's own, so the request for a wider
filter on the reporter's side would have been a wasted errand. His file is loud,
clean, correctly tuned and correctly framed; what it will not do is put its data
symbols on a constellation, and that points at the receiver's phase tracking
inside the frame rather than at the recording. Nothing has been read off the air
yet, and nothing is claimed to be.

**Done.**
- `stanag4285.rs`: `band_limit` (a 401-tap Hann band-pass) and
  `coded_round_trip_band`, so the whole coded chain can be run through an audio
  chain of any width, plus the test
  **`the_coded_chain_survives_a_truncated_audio_band`**.
- **The finding, and the numbers.** The bits come back exactly at
  **800–3000 Hz** (his tape's own band), and still at a **1000 Hz** high-pass and
  a **2600 Hz** low-pass; the standard's own band (300–3300) is the first
  assertion. The two corners are pinned so a front end that needs more fails in
  the test rather than on the air.
- **His file, re-measured** (60.9 s, 48 kHz): the two channels are identical, so
  the mono mix is not the fault; no carrier from 1400 to 2200 Hz beats 1800; SNR
  ≈ **70 dB** in 300–3400 Hz against 3.6–11 kHz; occupied ≈ 400–3000 Hz; frame
  period **106.73 ms**; locked with folded sync **0.781** (null 0.103), ACF
  **0.599**, 335 frames, references **0.626 / 0.697 / 0.575**, carrier +92 Hz.
- **The number that names the fault.** The descrambled data constellation is
  **0.234 / 0.070 / 0.080** where BPSK on the π/4 lattice should read about 1 at
  all three — so the data phases are spread continuously.
- **Where in the frame, and it decides the fix.** The same concentration per data
  block is **0.309 / 0.284 / 0.274 / 0.187** — all about the frame's own — so the
  smear is **inside a 32-symbol block (13 ms)**, not a drift across the frame. That
  rules out both cheap repairs (a line through the reference points is already
  there; per-block anchoring would not help). What smears a symbol in 13 ms while
  leaving sixteen identical reference symbols coherent is **delay spread of a
  symbol or more**, the impairment the standard's own tables allow for and the one
  a real receiver meets with an equaliser (Annex C/D).
- `STANAG4285-HANDOVER.md`: a new section with the table, the ruled-out list and
  the two things recorded but unexplained (an envelope line at 3607 Hz; the
  signal starting at 400 Hz). `MODE-PROGRESS.md` carries the same, and the mode
  table's row is corrected.
- `cargo test -p sdroxide-dsp --release --lib stanag` — **13 passed, 1 ignored**
  (the off-air gate); `rustfmt --edition 2024 --config-path rustfmt.toml` clean.

**Built: the closed-form equaliser, and where it stops.** The channel is now
estimated from the 80 known sync symbols by least squares and inverted by solving
`(R + σ²I)·w = p`; at the symbol level it recovers **176 of 176** symbols through a
1.5-symbol echo (worst error 0.09). On pulse-shaped audio it does not help yet
— its output makes the reference blocks collapse *worse* (2.81 → 2.02), so the gate
declines it and the frame reads as before. **The timing is ruled out**: the
sampling phase is now chosen by the least-squares residual over the known sync
(a proper error signal, a transition on almost every symbol) and it picks the same
instant the old criterion did, 0.60. What is short is the **model**: a five-tap
T-spaced fit does not describe a raised-cosine signal off a two-path channel well
enough to invert, so the next attempt estimates the channel including the pulse —
longer and fractionally spaced — which is a change to one function.

**Recorded: the adaptive equaliser does not converge.** An NLMS
fractionally-spaced FFE+DFE trained on the frame's 80 known sync symbols stalls at
the delay line's gain — training residual **0.88** against a unit target, whatever
the step size or tap count — and *lowers* the reference concentration (2.81 →
2.59). The inputs are not the reason: measured, the sync is recovered exactly at
the current timing (correlation 0.250 against a mean amplitude of 0.252, sign for
sign, clean and two-path alike). So the method is wrong, not the material: a loop
started from a delay line has to find a four-times gain and a fractional delay at
once on 80 symbols, and does not. The test that fails
(`a_coded_message_survives_a_two_path_channel`, ignored) is in place, and the way
in is **closed-form**: a least-squares channel from the known sync, then an
explicit inverse or a DFE seeded with it. The adaptive attempt was reverted rather
than left in, so nothing dead ships.

**Done, and the answer is the bench.** The offset was the real fault: it was read
off three reference blocks, which any echo bends into a ramp — 13 Hz wrong on the
two-path synthetic, +92 Hz wrong on his file. Refining it against the 80 **known**
sync symbols takes it to +1.5 Hz, the sampling phase to the correct 0.50, and the
data constellation from 0.234 to 0.347; the equaliser then decodes a two-path
synthetic end to end (`a_coded_message_survives_a_two_path_channel`, now passing
and no longer ignored).

But the same fit gives the tape's in-band SNR directly — **≈ 4 dB**, agreeing with
the constellation (0.35) and the reference blocks (0.65). A 4285 needs 10 dB; no
equaliser closes a 6 dB gap. **So STANAG is parked pending a stronger capture**,
and that is the one thing needed: the same tape on a quieter path, a better antenna
for 13 MHz, or more RF gain.

**The wider-filter request is withdrawn.** Mark measured the transmission himself
(#165, 12:47): at 4 kHz of SDR++ bandwidth he reads it as ~2.6 kHz wide at the
noise floor, about 500–3100 Hz — the tape, not his chain — and our own copy shows a
rolloff from 200 Hz rather than a wall at 800. The truncation test already says
800–3000 is enough. So the ask to him is strength, not width: a minute when the
signal is up (it is dropping in and out), more RF gain, or baseband I/Q if SDR++
can record it, which would leave no audio chain at all.

**Not done / blocked.** The payload is still unread, parked on the bench. Material
already on the disk, so no bench. A *second* capture of the same tape on a quiet,
steady path still needs the bench, and the watcher is the tool for it: a spread-out
channel is a bad witness for a modem.

**Next.** The phase tracker; then the rest of the mode queue.

---

## 2026-10-24 (stanag, second pass) — the phase tracker is ruled out

**Summary.** The first idea for the French Navy tape — that our demodulator was
losing the code to a drifting phase — was built and measured, and it is wrong: the
tracker changes the code's own score by nothing at all. That is a real answer, and
it points the next attempt at the interleaver instead, on the best recording this
mode has ever had.

**Done.**
- A decision-directed phase tracker on the data symbols (sign decision, residual
  angle, gain 0.15) after the reference-block fit. Metric before and after:
  **0.96×** its shuffled null at 1850 Hz, **0.96–0.97×** at 1900 Hz — identical to
  four decimals on the strongest branch.
- Reverted: an unproven change that buys nothing does not stay in the tree.
- `STANAG4285-HANDOVER.md` records the negative and names the next measurement.

**Not done / blocked.** The interleaver-convention sweep, re-run against this file.
The old sweep was against a recording with 0.23–0.34 data concentration; this one
has 0.396 and a 0.899 lock, and a metric sitting exactly at its null is what a
*wrong interleaver* looks like as much as a noisy one.

**Next.** Parameterise the interleaver's commutator/row order in `CodedChain` and
sweep it on `FUF-13031.200kHz-20261024-125317Z.wav`, judged by the metric/nul
ratio; then the 423 MB baseband IQ, which is still unopened.

---

## 2026-10-24 (stanag) — the Mac recording measured, and a mistake to record

**Summary.** Mark sent a better recording of the French Navy tape — made with our
own program on his Mac, at the full 3.7 kHz — plus a raw baseband capture. Our
receiver locks it far better than the old file and the equaliser now gets the data
symbols to 0.40 where the old recording gave 0.23–0.34, but the error-correcting
code is still invisible to us, while MultiPSK reads the same file from midway. So
the fault is in our demodulator and not in anything he sends. The next step is
named and measurable: track the phase symbol by symbol after the frame's
correction, and judge it by whether the code's own metric lifts off its null.

**Done.**
- Both files kept in `~/s4285-captures/keepers/` (off-air, not in the tree).
- Measured: lock **0.899**, frame ACF **0.706**, 242 frames, data constellation
  **0.396**, references 0.55/0.61/0.51, FEC metric **0.96×** its shuffled null at
  1850 and 1900 Hz. SOM correlator **31/32** — the closest to a message yet.
- `STANAG4285-HANDOVER.md`: the table, the two findings and the next measurement.

**A mistake, recorded rather than hidden.** My commit `a558d624` swept up **25
lines of the other session's MT63 work** (`crates/sdroxide-dsp/src/mt63.rs`) under
a STANAG message, because I used `git add -A` in a tree another session is
editing. Their change is intact and unaltered in content — nothing was lost — but
it is in the wrong commit with the wrong message, and it is pushed. **In a shared
tree, `git add` takes paths and never `-A`.** They should be told.

**Not done / blocked.** The decision-directed tracker, and the baseband IQ not yet
opened (423 MB; the audio file was enough to answer today's question).

**Next.** The phase tracker on `FUF-13031.200kHz-20261024-125317Z.wav`, judged by
the trellis ratio.

---

## 2026-10-24 (map) — roads and internal boundaries, for the "too imprecise" report

**Summary.** A user on Reddit says our map is too imprecise and asks for
OpenStreetMap. The data was never the problem — it is already Natural Earth's
finest — so instead of an online map, the two layers that make a map read as a
place are now shipped: **roads** and **first-order internal boundaries**. They are
drawn only when you are zoomed in far enough to want them, so a continent view
looks exactly as it did. It is **PR #184** because it touches the map widget that
APRS also draws on.

**Done.**
- `make_earth_maps.py`: two more layers from the same source (`ne_10m_roads`,
  `ne_10m_admin_1_states_provinces_lines`), roads filtered to their major
  classes (`scalerank <= 7`), both on the finest level only. Railroads left out on
  purpose (1.7 MB, indistinguishable from roads at this map's zooms).
- `lines.bin` **1.6 MB → 4.2 MB**; `basemap::Lines` gains `roads` and `states`;
  an older two-layer file still loads.
- `worldmap`: both draw paths (stipple and painted) add them, gated at **40°** for
  boundaries and **8°** for roads.
- `survey_ground_probe` now draws a **close view**; looked at: present at 6° in
  all three styles, absent at 60°.
- Guard: `the_detail_layers_are_in_the_asset` (33k road parts, 43k boundary
  parts, roads on the finest level only).
- Gate: workspace check silent; basemap 9 and worldmap 12 green; rustfmt clean.

**Not done / blocked.** Higher land resolution (16384×8192): 88 MB of CPU
pyramid, 4× the startup decode and a texture size browsers and many GPUs refuse —
it would have to become tiles. Online OSM tiles stay out: Mercator against an
equirectangular pipeline, and the no-network / no-key rules.

**Next.** PR #184 waits; then transmit for the receive-only set, RS_ID/CALL_ID.

---

## 2026-10-24 (pause) — the debug suite, and the queue emptied into `main`

**Summary.** Three things are now on `main` that were not this morning. The UI
tests run again in debug — they had been aborting for everyone, taking 949
passing tests down with them, because of a contract in the graphics layer that a
dozen tests broke. The LISTEN tab really does hide the transmit controls now,
whichever way you open the band menu, and the reporter's issue is answered by a
build rather than by a promise. And 8PSK is in the program as a third modulation
beside BPSK and QPSK. Nothing is open: no PRs, no numbers claimed, and the wire
register sits at 243.

**Done.**
- **The debug UI suite** (`55269c28`, rebased as `5f932005`). `epaint`'s
  `TexturesDelta::drop` is a `debug_assert!` that every delta was applied; a test
  has no renderer, and twelve of them dropped a frame's `FullOutput` unread. The
  first — the DAB logo tile — panicked *inside a destructor*, which is
  non-unwinding, so the process aborted and the suite reported nothing.
  `crate::test_support::run_frame` is the idiom now: run the frame, take the
  deltas, return the rest. Twelve tests go through it, in `app/mod.rs`,
  `app/settings/general.rs`, `theme.rs` and `tooltip_copy.rs`.
  **Before: 12 failed and the binary aborted. After: 961 passed, 0 failed, 6
  ignored** in debug — the same as `--release`, which is why this had gone
  unseen: the release gate is the release one.
- **PR #183 merged** (LISTEN tab, issue #146), **PR #174 merged** (8PSK).
  #174 needed main folded in first: it conflicted in `CHANGELOG.md` and in the
  `PROTO_VERSION` register, where #177 had taken 243 under it. Resolution keeps
  both entries in order — 242 8PSK, 243 voice hunting — with `PROTO_VERSION` 243
  and `VERSION_BYTE` 0x3F, and both changelog lines.
- After the merges, on `main`: `cargo check --workspace --all-targets` silent;
  proto 18, types 792, digi 542, ui **961** all green.

**Not done / blocked.** Nothing. **Six issues are open**, and every one of them
waits on somebody else: #132/#133 (a Brick2 on the bench), #153 (a diagnostic
report), #162 (the reporter's own calibration), #105 (a stable release), and
**#146**, which is fixed and merged but deliberately left open — the PR said
*Addresses*, not *Closes*, because the man who reported it should be the one to
say it works.

**Next.** Nothing until the operator says so; he asked for a pause. The queue
stands where the modes left it — STANAG 4285's coded payload first.

---

## 2026-10-24 — issue #146: the LISTEN tab hid nothing from the popup

**Summary.** The reporter on v2.1.0.2 was right and the feature was broken. Picking
LISTEN in the band/mode menu is supposed to hide the transmit controls, and it
did that only for someone who had already opened the selector as a docked column
or a window — from the ordinary BAND chip popup, which is how everyone opens it,
nothing happened. The answer is now read from the tab itself rather than from a
flag that one of the three places drawing the menu remembered to set, so every
way in works, and the LISTEN tab says where the switch is while it is off. It is
**PR #178**, because the band/mode window is F4NRM's.

**Done.**
- `mod.rs`: `listen_tab_hides()` and the tab half of `swl_mode()` derived from
  `ui_settings.listen_tab_hides_tx && band_menu_tab == Listen`; the stored
  `listen_tab_swl` field is gone, so there is no flag left to forget. The
  remembering keeps one seam, `remember_band_tab()`, called once a frame, and it
  writes only when the answer has moved.
- `top_bar.rs`: the `before`/persist block deleted from `band_menu_body` — it was
  the only one of the three call sites that had it. `band_mode_menu` gains
  `listen_hides_tx` so the tab can say where the switch is.
- The LISTEN tab now carries one weak line while the setting is off: *"This tab
  can hide the transmit controls too: turn on "hide TX on the LISTEN tab" in
  Settings → UI"*. A hint and not a chip, because the switch asks once in
  Settings and a chip here would be a way round that confirmation.
- Manual §2.4 documents the setting, including the half that matters: it hides,
  it never grants, and a receive-only radio stays receive-only.
- **The test was the other half of the fault** and now moves the tab rather than
  setting a flag: `app.band_menu_tab = Listen` then `assert!(app.swl_mode())`.
  Verified to fail on the old behaviour ("picking LISTEN must hide them — nothing
  else called").

**Numbers.**
- `cargo test --release -p sdroxide-ui --lib` — **961 passed, 0 failed, 6
  ignored** (the whole crate, the release gate).
- `cargo test -p sdroxide-ui --lib the_listen_tab` — passes in debug and release.

**Found on the way, and not fixed: the debug UI suite does not run.** With egui
0.36, `epaint`'s `TexturesDelta::drop` is a `debug_assert!`; a dozen UI tests drop
an `egui::FullOutput` without applying or clearing its deltas, and the first of
them — the DAB logo tile — **aborts the whole test binary**, so 949 tests never
report. `cargo test --release` is green, which is why nobody has seen it: the
release gate is the release one. The fix is one `out.textures_delta.clear()` per
test that draws a frame. It is a sweep across F4NRM's UI test files, so it is in
`TODO.md` under *Now* waiting for a go-ahead rather than done here — and it is
why this entry's numbers are release numbers.

**Not done / blocked.** No click-through of the popup itself: the test drives the
tab the popup writes, not egui's pointer, and that seam is where the bug was.

**Next.** PR #178 for F4NRM to read, then back to the mode queue.

---

## 2026-10-24 (rules) — F4NRM's list is a workflow, and the RSP1 is ours

**Summary.** The list of things that belong to F4NRM has been read off the
discussion itself rather than from memory, and it was wrong in two ways. The
**RSP1 is not on it** — the operator struck it off in as many words, because it
is his own receiver and the only wideband one on the bench — so RSP1 work is ours
and needs nobody's permission. And the rest of the list is no longer a wall: a
change that has to touch one of those decoders or radios, or one of his panels,
goes up **as a PR** for him to read before it lands, which is what he asked for
in that same discussion. Nothing changes about the part that matters: no
unrequested refactor of a mode his radio group has confirmed.

**Done.**
- `AGENTS.md`: the section is renamed *"F4NRM's list — his to change, and a PR to
  touch"*, quotes the operator's own correction, drops RSP1, and states the PR
  rule with his sentence as the evidence: *"put any wire change or item that
  touches these to put up as a PR and not implement them directly without you
  having the chance to do look at them."*
- `AGENTS.md`'s stop-list for an unattended session: F4NRM's list, his panels and
  the wire now share one line — branch, tests, PR, carry on, never `main`.
- `AGENTS.md`'s focus section and `TODO.md`'s item 0 carry the same reading, so
  the queue cannot contradict the rules.
- Read from the discussion itself (GraphQL, `discussion(number:155)`), not from
  the paraphrase that was here.

**Not done / blocked.** Nothing. The list itself is still F4NRM's to change, and
this entry does not add to or remove from it beyond the operator's own correction.

**Next.** Back to issue #146 (the LISTEN tab), which is what the session was on.

---

## 2026-10-24 — voice hunting: the scanner stops on people, not on carriers

**Summary.** The scanner can now hunt voices. A **VOICE** switch in the SCAN
window makes it stop only on channels that sound like somebody talking, and go
past the unmodulated carriers, birdies, pagers and data bursts it used to stop
on. While it runs, the line under the status tells you what it made of the last
channel it looked at — *passed 27.5550 MHz · voice 18*, or *voice 82 · somebody
is talking* — so a strong signal going past is a number rather than a mystery.
The cost is time: a voice is a rhythm of syllables, so each channel is listened
to for at least half a second, and the window raises the dwell and says why.
Nothing is proven off the air yet — the detector is proven on synthetic speech
and on the engine's own bench, and a real band is the next thing it needs.

**Done.**
- **`sdroxide-dsp/src/vad.rs`** (new): `SpeechDetector`, a pure function of the
  audio handed to it — a 20 ms frame, then four measurements. **Structure**
  (spectral flatness and the share of the band's power in its strongest bin,
  which between them separate speech from noise and both from a tone),
  **voicing** (normalised autocorrelation over 60–400 Hz), **spread** (three
  sub-bands of 250–3500 Hz), and **syllables** (hysteresis rises of the frame
  loudness, 2–9 a second). Score 0–100; the verdict is **4 accepted frames in
  the last half second**, not one, and `ACCEPT` is 70 — above what structure,
  voicing and spread can add up to (65) — so a frame has to show a *rhythm*
  before it counts.
- **The two measurements that shaped it, both failures first.** A bare AM
  carrier scored **exactly 65.0 on one frame** — that is the DDC settling after
  a retune, and it made the scan stop on every hop. And a single frame at the
  threshold was enough for a verdict. Hence `ACCEPT = 70` and the 4-of-25
  window, and both have their own unit tests (`one_settling_thump_is_not_a_voice`,
  `a_steady_buzz_is_not_a_voice`).
- **Engine** (`crates/sdroxide-radio/src/engine.rs`): the chain scores its own
  demodulator audio *before* the AGC, notch, noise reduction and squelch — a
  voice is what arrived, not what the operator's settings did to it — and a
  demod-audio rig (CB on a sound card) is scored in `play_rx_audio` through a
  second detector. `scan_is_busy()` is now level **and** voice; the settle is
  `dwell.max(MIN_LISTEN_MS)` when the gate runs; the detectors reset when the
  offset moves or a candidate is retuned to; and `voice_gate()` has three
  answers — off, running, and *nothing to hear* (a mode that carries no speech
  is passed over, because the operator asked for voices).
- **Refusals, not silent fallbacks**: starting a voice hunt with no audio, or
  with nothing in the scan in a mode that carries speech, is refused with a
  notice, and the SCAN window greys the button and says the same thing before
  START is pressed.
- **Wire** (`crates/sdroxide-proto/src/lib.rs`): `ScannerConfig.voice_only` and
  `ScanState.voice` (`Option<ScanVoice>` — a whole-hertz frequency with the
  score, so the reading always names its channel), both appended,
  `#[serde(default)]`. **243 / 0x3F**, having found 242 already claimed by
  PR #174 (`fork/psk8`) — the later branch moves, as the register says.
- **Tests, with the numbers.**
  - `cargo test -p sdroxide-dsp --lib vad` — **15 passed**.
  - `cargo test -p sdroxide-radio --test scanner` — **15 passed**, three new:
    a bare carrier that stops the scan with the gate off and is passed over with
    it on (and its reading is on the wire, < 45); a voice-modulated carrier that
    stops it (score ≥ 60); and a voice hunt over `Mode::Ft8` refused with a
    notice.
  - Both new engine tests were run against the unfixed code and fail there:
    gate disabled → "an unmodulated carrier was taken for a voice"; refusal
    removed → "a voice hunt over a mode with no voice should not start".
  - `cargo check --workspace --all-targets` silent; `cargo check --release
    --target wasm32-unknown-unknown -p sdroxide-ui` clean (its 277 dead-code
    warnings are the pre-existing wasm-only ones).
  - `rustfmt --edition 2024 --config-path rustfmt.toml --check` clean on every
    file touched; `sdroxide-types/src/lib.rs` was already dirty in its module
    order and was left alone.
- **Not borrowed.** The idea came from VUURWERK (Quansheng UV-K5 firmware), but
  nothing was taken from it: its licence puts a commercial licence on its
  originals — the voice-probability scoring among them — and this fork takes no
  paid anything. This is textbook DSP written here, and a reviewer who compares
  the two will find no shared code.

**Not done / blocked.**
- **Nothing has been heard off the air yet.** The detector is proven on
  synthetic speech, a synthetic buzz, noise, AFSK and the engine's AM bench; a
  real band — 11 m at night, or a broadcast band full of carriers — is the test
  that would settle the thresholds, and it needs the bench, not a session.
- The **`audio_mode`/external-audio path is untested end to end** (the CB rig
  through its USB card). Its detector is fed and its guard is in place, but no
  test drives a demod-audio source through a voice hunt; that needs the SS9900v.
- Music with a vocal scores as voice, by design; documented in the manual rather
  than "fixed".

**Next.** The mode queue: STANAG 4285's coded payload is still the programme's
head, and the rest of the half-done list behind it. This item needs the operator
(or F4NRM) to merge the PR first, since it is the wire.

---


**Summary.** The four wide multi-carrier PSK modes are not going to be fixed for
now, and the reason is written down where it matters. They are still in the
picker, but their chips now say on the hover that they do not decode yet and why,
and the manual's PSK section names all four and tells you to reach for a narrower
bank instead. So nobody picks one, hears nothing, and blames the band.

**Done.**
- `panels/text_modem.rs`: the four wide chips carry the note — "this wide bank
  does not decode yet … the narrower banks do read their own recordings".
- Manual §3.3: a paragraph naming PSK125C12, PSK500C4, PSK500RC4, PSK1000C2 and
  pointing at PSK250C6 / PSK500C2 for a wide signal.
- `TODO.md` → **Parked, with the reason**, carrying the measurement, the ruled-out
  theory, the gate to use, and the condition for un-parking (somebody hears one
  on the air).
- Non-wire: no `PROTO_VERSION`, no mode change, nothing to merge.

**Not done / blocked.** The front-end rebuild itself, parked by the operator's
decision on the advice given: four rare wide banks are not worth risking the
twenty that work.

**Next.** The remaining mode work is transmit for the receive-only set and
RS_ID/CALL_ID; the rest of the unfinished list waits on recordings.

---

## 2026-10-24 (next item) — the four wide PSK banks: measured, and the fix is a refactor

**Summary.** The next item on the queue was the four wide multi-carrier PSK modes,
the last of the coded PSK family that does not read its own recording. They still
do not, and the reason is now narrower than it was: every bank wider than about
two kilohertz fails (0 of 3 known words) while every narrower one reads its whole
text, and making the per-carrier filter four times sharper changed **nothing** —
so it is not filter sharpness. What is left is the reference's own chain order
(decimate first, then filter), which is a rebuild of the receiver's front end
rather than a parameter change.

**Done.**
- Baseline, `cargo test --release -p sdroxide-dsp --lib the_corpus_pskc --
  --ignored --nocapture` (69 s): **PSK500C4 0/3, PSK1000C2 0/3, PSK125C12 0/3,
  PSK500RC4 0/3**; PSK500RC3 1/3 and PSK250RC5 1/3; everything narrower 3/3.
- The failing set is exactly the widest banks: 2.1 kHz (PSK125C12) and 2.8 kHz
  (the other three). PSK500C2 — same baud, same 700 Hz spacing, only two
  carriers — reads 3/3, so the spacing alone is not the fault.
- Tried a filter sharpened with bandwidth (tap count from `4·rate/half`, 129 →
  512 at 125 baud): **identical results**, reverted. It is not the filter's
  transition width.
- `psk.rs` restored to its committed state (`git status` clean before the next
  step).

**Not done / blocked.** The fix needs the front end rebuilt in the reference's
order — decimate each carrier to a few samples per symbol, *then* filter there —
instead of filtering at the 8 kHz input rate. That is a structural change in
`PskRx`/`PskCarrierBank` with the timing loop and the differential detector on
top, not a session-sized edit, and it should not be attempted as a tail-end
change to a long session.

**Next.** Rebuild the multi-carrier front end in that order, with the existing
`the_corpus_pskc_samples_are_measured` as the gate: the target is those four
banks moving off 0/3 without the 3/3 ones regressing.

---

## 2026-10-24 (last) — 8PSK is in the program, and the ALE watch is stopped

**Summary.** All three modes the operator named are now in the program. ALE and
CIS-11 were already wired; **8PSK** is now a third choice beside BPSK and QPSK on
the keyboard modes' modulation row — receive only, three bits per symbol. Its
proof is a synthetic, because the test recordings for it turned out not to hold a
clean 8PSK signal at all. The unattended ALE channel sweep was stopped as asked;
it had finished four rounds over eleven channels with nothing heard.

**Done.**
- `PskModulation::Psk8` appended, wired into the plain PSK receiver, refused on
  transmit with a reason, chip and hover text come from `ALL` so no layout
  changed. Commit `41c83c87`, branch `fork/psk8`, **PR #174** (wire change:
  **`PROTO_VERSION` 241 → 242, `VERSION_BYTE` 0x3D → 0x3E**).
- Manual mode-table row and changelog line say "receive only, proven on a
  synthetic".
- Gate: workspace check silent; types 792, digi 542, ui 961; rustfmt clean.
- ALE sweep: `pgrep -af "watch.py|rspcap"` → nothing. Its log's last four lines
  are `quiet` on every channel, four rounds, ~33 minutes.

**Not done / blocked.** 8PSK has no recording and no 8PSK modulator; the manual
says both. A reference recording would turn "proven on a synthetic" into proven.

**Next.** PR #173 (CIS-11) and #174 (8PSK) both wait on the operator's merge; the
next unwired item is the four wide `C` PSK banks, whose recordings are good
(0.83–0.85 of the bits) and whose fault is the demodulator's carrier separation.

---

## 2026-10-24 (later) — 8PSK: the instrument is sound, so the files are the fault

**Summary.** The question from the entry below is answered. Built a synthetic
8PSK of the right shape and measured it with the same instrument that reads the
test files: the synthetic scores **0.99** where the files score 0.44–0.58. So the
measurement is not what was wrong and neither is the width — **the four 8PSK test
recordings do not contain a clean differential-8PSK constellation**, and no
receiver written against them could have worked. The mode's slicer is proven on
our own synthetic; what it needs now is one genuine recording, or being shipped as
"proven synthetically" like the others.

**Done.**
- Synthetic 8PSK at 125 baud, symbols shaped by a 201-tap low-pass at 0.5 × baud
  (the α≈0 shape the corpus's own −3 dB width implies), measured by the same
  chain used on the corpus (mix, 300 Hz LPF, decimate to 8 sps, differential
  phase): **8-fold concentration 0.99**, 2-fold 0.04.
- The same chain on `samples/fldigi/8PSK125.wav`: 0.44–0.58 at every symbol
  phase, collapsing onto 0°/180° — a closed eye, not an 8PSK.
- Matched filtering (one-symbol box, raised cosine α = 0.35) does not help the
  file: 0.33 and 0.35 against 0.47 unfiltered. It is not a filter-order fault.

**Not done / blocked.** What the four files are, and whether our receiver reads
the **shaped** synthetic (the next run: write it to a WAV and feed `PskRx` with
`set_psk8(true)`; the existing test uses rectangular symbols and passes).

**Next.** Run the shaped synthetic through the real receiver, then wire 8PSK as a
mode on a branch — the wire change — with "proven synthetically, no working
recording" in the manual, exactly as ALE and CIS-11 are stated.

---

## 2026-10-24 — 8PSK: the recordings do not say what we thought

**Summary.** No change in the program. The mode that has been stuck — the
8-phase PSK family, which fldigi transmits and we cannot read — turned out to be
stuck for a reason we had wrong: the notes said those test recordings are only
30 Hz wide, which would mean they cannot carry the mode at all. They are not.
Measured properly they are exactly the width the mode should be, at a normal
level and the right speed, so the fault is not a band-limited recording. What is
still true is that their phase never settles for us, and our own decoder reads
nothing from them either. Before another receiver gets written, the next step is
to build the signal ourselves the way fldigi does and compare the two by the same
measurement — or to get one real recording of the mode.

**Done.**
- Re-measured the corpus 8PSK files (`samples/fldigi/8PSK*.wav`):
  **8PSK125 is 124.5 Hz wide at −3 dB** (BPSK125: 125.0 Hz), carrier 1806.6 Hz,
  in-band SNR ≈ 28 dB — the same level and SNR as the BPSK and QPSK files beside
  it. **Baud line 125.0 Hz**, harmonics at 250 and 374 Hz (spectrum of the phase
  derivative). The old "20–46 Hz wide" note was a faulty width measurement
  (thresholds read above the peak instead of beside it).
- Differential-phase concentration at the right rate: BPSK125 **0.87** (2-fold),
  QPSK125 **0.74** (4-fold), every 8PSK file **0.44–0.58** at any symbol phase.
- Our own BPSK decoder reads **nothing** from `8PSK125.wav` (PskRx at 125 baud over
  1650–1950 Hz), so the 0°/180° histogram is a **closed eye**, not a BPSK file.
- `docs/internal/MODE-PROGRESS.md` carries the corrected table and the two
  candidate explanations; commit `4e7abd93`.

**Not done / blocked.** What the 8PSK files actually are. Two answers would
settle it and neither is a guess this bench can make: a **synthetic 8PSK built the
way fldigi builds one**, compared against the file by the same statistic, or a
**reference recording** of the mode. Nothing is wired to the panel.

**Next.** Build the synthetic comparison (it is a Rust test next to the existing
`a_synthetic_8psk_transmission_reads_back`, which already proves the slicer on our
own waveform — the missing half is doing it under fldigi's shaping; if the
synthetic settles and the file does not, the file is the fault and the item waits
for a recording).

---

## 2026-10-24 — CIS-11: a new mode, measured from the recordings before it was built

**Summary.** There is a new mode in the DIGITAL row, **CIS-11** — the Russian
meteorological teleprinter that listeners meet between 4 and 18 MHz. Leave the
dial on a channel and it finds the two tones by itself and prints what it locked
to, with a count of how many characters passed their parity check; on a real
recording that is 121 of 123. What it cannot yet do is turn those characters into
readable letters — that layer of the protocol is not known, and the traffic on
these frequencies is encrypted anyway. It is receive only. It is on a branch as
PR #173 because it changes the protocol version, so it is waiting on your merge.

**Done.**
- The receiver (`sdroxide-dsp::cis11`): finds its own tones, tracks the character
  clock, decodes the 11-bit character. Commit `be6090ca` plus the wiring in
  `a0c56a2f`; branch `fork/cis11`; **PR #173**.
- The **framing measured, not assumed.** The two catalogues say "5 data +
  parity"; the idle word looks like 5 + 5 + 1. Scored every grouping from 4 to 16
  bits and every rotation by parity consistency: **11 bits at 85 %**, repeated
  character **13 %**, and the traffic's halves agree 5 % — so it is **ten data
  bits and one odd-parity bit**.
- **Off-air**, idle recording: tones 742/1242 Hz, **123 characters, 121
  parity-valid**; traffic: 199 characters, 123 clean. Command:
  `SDROXIDE_CIS11_SAMPLE=/tmp/opencode/cis11/idle.wav cargo test --release -p
  sdroxide-dsp --lib cis11 -- --ignored`.
- Wiring: `Mode::Cis11` (discriminant 62), `Cis11Controller`, the shared
  text-modem panel (no new UI), manual §3.30, changelog.
- Wire: **`PROTO_VERSION` 240 → 241, `VERSION_BYTE` 0x3C → 0x3D**, register entry.
- Gate: workspace check silent; types 792, digi 542, ui 961 green; rustfmt clean.
- Two pins fixed on the way: `DominoEx` and `Dtmf` were never pinned, and the
  `Mode::ALL` count test took its last discriminant from `Dtmf`.

**Not done / blocked.** The character layer: how the ten data bits divide into
letters. Needs a **clear** transmission (Russian meteo is sometimes sent in clear
M2 Cyrillic) or a reference decoder to compare against; the encrypted traffic
cannot settle it. Also unmeasured: the 400 Hz shift variant, which the receiver
searches for but no recording of it has been tested.

**Next.** PR #173 waits on the operator. Then the queue in `TODO.md`: the modes
that did not finish in 2.1.0 (8PSK's vestigial filter, the four wide PSK banks)
and ALE's recording ask.


## 2026-10-24 — The standing order for working unattended

**Summary.** Nothing in the program changed. The notes the agent works from now
say to keep going on its own when the goal and the boundaries are clear, to write
what happens in this journal as it happens, and to keep the TODO list and the
project board current. When an item is finished you will get a short plain
summary without having to ask for it.

**Done.**
- `AGENTS.md`: new section "Working on your own — the standing order" (the four
  things that make a task clear, the list of things that stop an item, the
  journal, the TODO/board updates, and the completion summary), and the header
  now names this journal.
- `docs/internal/AUTONOMY-JOURNAL.md`: this file, with its format.
- `docs/internal/README.md`: added to the file table.
- Docs only — no crate, no wire, no build. Nothing to run; `cargo check` not
  needed for a markdown change.

**Not done / blocked.** Nothing.

**Next.** The live queue is unchanged: the mode work, STANAG 4285 first —
[`MODE-PHASE-4-PROMPT.md`](MODE-PHASE-4-PROMPT.md) and
[`MODE-PROGRESS.md`](MODE-PROGRESS.md).
