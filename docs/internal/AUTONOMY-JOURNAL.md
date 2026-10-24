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

## 2026-10-24 (aircraft) — the clients read the relay: badges, photo credit, no storage

**Summary.** The aircraft card in the browser, the phone and the remote client now
shows what the station's relay knows — type, operator, route, a photo — under the
radio's own lines, each marked with a green "radio" or a blue "online" badge, and a
badge at the top that says "radio" or "radio + online". Nothing changes unless the
operator has switched the relay on at the station; if it is off the card says
"online indisponible" once and does not ask again. It is not proven against a real
station or the real services, and whether a browser may load the Planespotters
thumbnail depends on a header only their server can set — if not, the credit and
link show without the picture. Next: a live run.

**Done.** Branch `claude/aircraft-online-clients` (from `2520f406`). New
`aircraft_online.rs` (pure: which question, when, what a reply means, the rows and
badges, the photo view) and `aircraft_online_io.rs` (the sending: a thread with
`ureq` natively, `fetch` in the browser; thumbnail decoded in memory). The native
remote client signs in once with the password the socket already had and keeps the
cookie in memory. The native local engine has no station: `NoStation`, no request.
No client names adsbdb or Planespotters' API; only the thumbnail address the relay
returned, and only on `t.plnspttrs.net`. No `PROTO_VERSION` change; `Cargo.lock`
gains the edges for `serde_json`, `ureq` and the relay crate's pure half.
Numbers: 36 new tests; `cargo test -p sdroxide-ui` 1035 passed in the lib plus the
integration tests; `cargo check --release --target wasm32-unknown-unknown -p
sdroxide-ui` and `-p sdroxide-web` clean; rustfmt clean on the new files. Red
first: 26 of 29 logic tests failed on `todo!()` stubs; the three source scans
passed on stubs and were shown to fail by injecting `localStorage` and an adsbdb
address; the "nothing is saved" test was shown to fail by writing the online state
into `save`. The sixth transport test (no station, nothing asked) **cannot catch a
regression of its own guard** — the result is the same by another path — and says
so in a comment.

**Not done / blocked.** Not run: the full `cargo check --workspace --all-targets`
(`sdroxide-rade` fetches at build time), any request against a live station, the
native sign-in and cookie path, the browser `fetch`, and a look at the drawn
thumbnail (the preview shows the credit-only variant). The sandbox disk filled
twice; regenerable `target/` output was deleted.

**Next.** A live run with the relay on; then F4NRM looks at the card.

---

## 2026-10-24 (STANAG 4285) — the equaliser reads the tape, and the tape was never weak

**Summary.** The one STANAG 4285 recording that was said to be too weak to finish
now demodulates cleanly, and the reason was our own filter, not the radio: a
receiver change takes the same minute of audio from an unreadable constellation to a
usable one and makes the hardest channel the standard allows decode at all. So the
request for a new capture is withdrawn — what is left is a fault in the part that
undoes the error correction, and both tapes are good enough to hunt it on. Nothing
new needs the bench. Also fixed: the program was labelling the *cleanest* BPSK
signal we have ever recorded as "unknown rate", and showing nothing for it.

**Done.**
- **`ls_equaliser`** replaces the T-spaced `ls_channel` + `zf_equaliser` pair in the
  coded and displayed paths. It is a direct least-squares filter over
  **half-symbol-spaced** samples whose target is the symbol sent at each instant, so
  it models the pulse as well as the path and its output is aligned without the
  three-shift group-delay search. `LS_FRAC_TAPS = 19` (swept 9/13/19/25/33/41),
  `LS_LOAD = 1e-2` (swept 0.003–0.3, moves the tape's constellation by 0.008).
- **Numbers, same files, same tunings** (`S4285_EQ=off|on` on
  `examples/s4285_probe`):
  - tape `FUF-13031.200-…150300Z`: constellation **0.046 → 0.790**, data blocks
    0.30/0.24/0.08/0.25 → **0.71/0.73/0.91/0.86**, refs 0.80/0.78/0.80 → 0.85/0.83/0.85.
  - our `6348.000-…045545Z`: **0.728 → 0.859**; `…044640Z` unchanged (Auto keeps the
    plain path); clean synthetic at 6 dB unchanged — the equaliser costs nothing.
  - synthetic sweep, first level that decodes: 1.5-symbol echo plain 15 dB /
    equaliser **8 dB**; two equal paths 2 ms apart plain never (18 dB) / equaliser
    **15 dB**.
- **Tests, and they fail without the fix.** `a_coded_message_survives_a_two_path_channel`
  now asserts both halves — the plain path reads **no message** at 10 dB and the
  equaliser reads it bit-exactly — and a new
  `a_coded_message_survives_the_two_equal_paths_the_standard_budgets_for` does the
  same at 4.8 symbols / 15 dB. Verified by making `ls_equaliser` return `None`:
  both fail ("the equaliser did not read a two-path message at 10 dB", "two equal
  paths 2 ms apart lost the message"), and pass again with it restored.
  `the_least_squares_equaliser_undoes_a_symbol_level_echo` rewritten for the new
  filter: 176 of 176 symbols, worst error 0.01.
- **The rate verdict was backwards** (`rate_from_concentrations`): the old rule asked
  the leading concentration to beat the runner by half again, which on a *clean*
  BPSK frame reads 0.933/0.760/0.357 — ratio 1.23 — and answered **`unknown`**,
  suppressing the uncoded text path. Now a cascade on which concentration stands up
  first. One file in the corpus changes, and it is the right one:
  `FUF-…125317Z` `unknown` → `1200 BPSK uncoded, or a coded BPSK rate`. Pinned by
  `the_rate_verdict_is_right_on_the_cleanest_bpsk_frame` (all numbers off the air)
  and `a_clean_synthetic_frame_is_reported_as_bpsk`.
- **Checks:** `cargo check --workspace --all-targets` silent; `cargo test --release
  -p sdroxide-dsp` **491 + 26 + 14 + 1 + 8 + 14 + 8 + 5 passed, 0 failed**;
  `cargo check --release --target wasm32-unknown-unknown -p sdroxide-ui` clean;
  `rustfmt --check` clean on both touched files; the off-air gate
  (`SDROXIDE_S4285_SAMPLE=… --ignored`) passes at mean refs **0.816 / 0.826 / 0.831**
  (was 0.61/0.70/0.57).
- **The record corrected:** the "tape is about 4 dB in band, needs a stronger capture"
  finding is **withdrawn** — it was the T-spaced fit's own failure read three ways.
  Both tapes now read data blocks at 0.71–0.96 with references at 0.97–0.99.

**Not done / blocked.**
- **No real coded transmission has been decoded.** Not blocked on the bench any more:
  both tapes are measured to *carry* the code (their descrambled bits are not ITA2 at
  any of 7 alignments × 2 polarities × 2 bit orders — 0.22 against a 0.23 chance —
  and show no rate-1/16 period), and both have a good eye. The fault is in the coded
  chain, and it is now the only suspect.
- **The interleaver/decoder comparison against the independent implementation** was
  not built in this session; it is the next measurement, and it needs no radio.
- **The uncoded character layer is still 5-bit chunking, not 5N1.5** — it cannot read
  ITA2 text from a 1200 bps uncoded transmission at all. Recorded, not fixed: neither
  tape is uncoded, so it is not on this path.

**Next.** Build `luwangg/signal-analysis` (GPL, C++/octave) and run it over
`FUF-13031.200-…125317Z`, then compare its decoded bit stream with ours symbol for
symbol — data-symbol order, bit order, and whether the interleaver works in 32-symbol
segments or bits. A framed message on the known text is the judge; the
metric-versus-shuffled-null statistic is invalid and must not be used.

---

## 2026-10-24 (aircraft) — an optional server relay for the aircraft card's online fields

**Summary.** The aircraft card can now be given its internet fields (type,
operator, route, and a photo's credit and link) by the station itself, so a
browser, a phone or a remote client never has to call those services. It is off
until the operator sets `SDROXIDE_AIRCRAFT_ONLINE=1` on the server, needs no key
or account, and the radio side of the card is untouched either way. Nothing in
the clients uses it yet: that is the second PR. What is not proven is the real
answers of adsbdb and Planespotters, because this sandbox cannot reach them.

**Done.** New crate `sdroxide-aircraft-online` (parsing, cache, relay, and an
HTTP part behind the `fetch` feature) and `GET /aircraft-info/{icao}` in the
server, behind the station's login cookie and shut when the switch is off. JSON
cache 24 h, failures remembered 5 min, 2000 entries per kind, one outbound
request per key, 30 questions a minute overall, fixed hosts only, ICAO exactly
six hex digits. The Planespotters photo is never fetched, stored or re-hosted:
only the thumbnail URL (unchanged), the credit and the page link are returned.
No `PROTO_VERSION` bump. Tests: 50 in the crate (45 without `fetch`), 30
server lib, 3 integration (`aircraft_info`); 23 of 23 hand mutations caught.

**Not done / blocked.** The two services' formats were written from their
documentation and **need one live run** from a machine with internet. The wasm
check of the crate is clean. `cargo check --workspace --all-targets` and the
radio-linked tests depend on `sdroxide-rade`, which fetches at build time.

**Next.** PR B: the clients, with the badges.

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

## 2026-10-24 (stanag, eleventh pass) — segment interleaver landed, band limits measured

**Summary.** The interleaver now works in the standard's 32-symbol segments, so a
fade is spread over ten seconds instead of a third of one. The two impaired-channel
tests were re-baselined honestly (the impairment's cost comes out of the noise
budget), and the band the tape arrives in finally has a number: 800–3000 Hz costs
about 4 dB, and a low corner at 1000 Hz never decodes at any level. The captures
still do not frame a message, so what is left is the demodulation of real signals,
not the coding.

**Done.** `ConvInterleaver::push` gathers 32 symbols per commutator step; the
round-trip test measures the pipeline latency instead of assuming it; the flush in
the test transmitter is derived from the depth (32·31·j + 102) and the audio gets two
interleaver spans of margin; `coded_round_trip_noise` sets the noise budget, and the
band and two-path tests now spend theirs on their impairment.

**Numbers.** Span 11 904 symbols = 9.9 s at j = 12 (was 372 = 0.31 s). Round-trip
64/64. Band sweep at 10/14/18/22/26 dB: 300–3300 ✓10, 300–2600 ✓10, 800–3000 ✓14
(✗10), 1000–3300 ✗26. Captures: 0 messages, tape best SOM 28/32, 6348 capture 27/32.
Tests 16 passed, gate silent, fmt clean.

**Next.** The demodulation of real signals: our own signal reads 0.99 and decodes,
the captures read 0.73–0.82 and do not, and with the coding now verified end to end
the difference is in the symbols. The 0.93-reference capture is the test bed.

---

## 2026-10-24 (stanag, tenth pass) — the segment interleaver, written and measured, not landed

**Summary.** The interleaver is the wrong shape: the standard interleaves 32-symbol
data segments, this receiver advances its commutator every symbol, and the delays are
32 times shorter as a result (0.31 s against 9.9 s, the ten-second long interleave
every other source describes). I wrote the segment form and got two clear signals
that it is right — the round-trip returns 64 of 64 bits once the test looks for the
latency, and on the band-limited receiver test the framer matched the SOM **32 of
32**, which no earlier version ever came close to. It is not in the tree: the tests
whose audio is shorter than a ten-second interleaver stay red, and the lead-in and
flush plumbing has to be redone with it.

**Done.** The rewrite, twice, and the measurements above. Reverted to keep the suite
green; the design and the exact patch are recorded in `STANAG4285-HANDOVER.md`.

**Numbers.** Old form: max delay 31·12 = 372 symbols, 0.31 s. Segment form: 31·12
segments = 11 904 symbols, 9.9 s. Round-trip 64/64 at the found offset; SOM 32/32 on
the band-limited test with the segment form; no message closed because the test audio
is ~200 frames where ~290 are needed.

**Next, and it is mechanical.** Send the standard's ten-second lead-in from
`s4285_test_signal` and from `coded_round_trip_path`, derive the flush from the depth
(32·31·j + 102), give the tests a helper that measures the pipeline latency instead
of assuming it, then land the segment interleaver and re-run the captures. Until that
lands, nothing on the air can decode: this is the blocker.

---

## 2026-10-24 (stanag, ninth pass) — the scrambler was seeded wrong, and the judge never worked

**Summary.** Two findings, both big. First, our scrambler initialised its 9-stage
register to `1` instead of **all ones**, so on a real transmission the sync locked
normally (it is sent unscrambled) while every reference block and every data symbol
was descrambled into noise — reference blocks read 0.55 and the data constellation
0.396. Seeded correctly they read **0.76** and **0.750**, and 451 frames are
accepted instead of 242. Second, the statistic this whole effort has been judged by
— the decoder metric against the same values shuffled — reads **1.02× on our own
clean signal, which decodes**, so it never separated signal from noise and every
"ruled out" conclusion drawn with it is void. With a decision-feedback equaliser
added (0.396 → 0.823 data constellation) the tape is close but still frames no
message.

**Done.**
- `scrambling()` seeded `0x1FF`; the crate's generator test updated to the correct
  first symbol (phase index 7) with the reason on it.
- `EqMode::Dfe`: LMS DFE, 11 fractionally-spaced feedforward taps, 4 feedback taps,
  mu 0.05, trained on the sync and corrected on the reference blocks, two passes.
  Opt-in, because it spoils a clean signal.
- QPSK rates fed two soft bits per symbol in both bit orders; erasures and the
  trace follow the stride.
- Three diagnostic examples (interleaver conventions, commutator start phase, QPSK
  axes) and the `S4285_EQ` probe knob.

**Numbers.** refs 0.55 → 0.76 (0.95 best frames), constellation 0.396 → 0.750,
frames 242 → 451; with the DFE 0.823 and per-block 0.81–0.87; clean-signal control
under the DFE 0.995 → 0.736 (so not a default); the metric judge on that same clean
signal **1.02×** — the reason to distrust it.

**Next, in the order a second opinion recommends (and it agrees with both other
decoders).** Verify the data-symbol mapping and the coded-bit ordering against the
reference implementation — in particular whether the interleaver is fed bits (two
per QPSK symbol) or symbols, and the direction of the 9i commutator — then the
Viterbi framing. The equaliser has now had its turn.

---

## 2026-10-24 (stanag, eighth pass) — sensitivity measured, and the tape is not noise-limited

**Summary.** The chain's own limit is now a number per rate: it reads a message at
**-6 dB** in 3 kHz at 150 and 75 bps, with the reference blocks still collapsing at
**0.90**. Mark's tape collapses its reference blocks at **0.55**, and its symbols
scatter ±87° — which white noise at the same level does not do. So the tape's
problem is not weakness but a non-Gaussian impairment (fast phase wander or in-band
interference). That is the first statement in this work that is a bounded claim
about the tape rather than a cleared suspect, and it points at phase tracking, not
at coding, interleaving or the equaliser.

**Done.**
- `S4285Frame::symbols` (the 176 descrambled, channel-corrected symbols) and
  `S4285_SYMBOLS_OUT` in the probe, so a capture's frame structure can be measured
  instead of assumed.
- Noise ladder per rate (table in the handover).
- Tape analysed against it: per-position and per-block phase, sliding 16-symbol
  coherence over 242 frames.

**Numbers.** 600/2 decodes at 2 dB (constellation 0.921); 150/8 and 75/16 decode at
**-6 dB** (0.612 / 0.578) with references **0.905 / 0.906**. Tape: references
0.55 / 0.61 / 0.51, symbol scatter 86–91° sd, sliding coherence flat at 0.24–0.35.
Tests 16 passed 1 ignored, gate silent, rustfmt clean.

**Blocked, and it is the right block.** Which rate MultiPSK reads is a question for
Mark, and it decides the whole next move: at 75 bps we have 12 dB of coding gain to
spend and should be decoding this tape; at 600 we are 3 dB short in the
demodulator. It is in `TODO.md` under *Waiting on other people*.

**Next when the answer or a stronger capture arrives.** Phase tracking with a
per-symbol, decision-directed loop on the *reference blocks* first (a 16-symbol
block is short enough to follow wander that a per-frame line cannot), judged by
whether the reference concentration rises from 0.55.

---

## 2026-10-24 (stanag, seventh pass) — longer channel estimates are worse

**Summary.** One more equaliser variant measured and rejected in minutes: giving the
channel estimate more delay spread (up to 5 ms, which is what HF multipath can be)
makes the tape's constellation *worse*, not better. So whatever is closing the eye
is inside two symbols, and the short estimate stays.

**Numbers.** Data constellation 2-fold on Mark's file at 1850 Hz: default 5 taps /
9-tap equaliser **0.396**; 9 / 17 → **0.348**; 13 / 29 → **0.312**. Reverted; tests
green (16 passed, 1 ignored).

**Next.** Within-frame amplitude/phase wander: the reference blocks read 0.55–0.61
while the data read 0.396 in the same frames, and a timing error would hit both
equally. The next measurement is a **piecewise per-block** channel estimate
(sample-and-hold) in the coded path, judged first by the constellation and then by
the metric.

---

## 2026-10-24 (stanag, sixth pass) — the tape is not weak, it is smeared

**Summary.** The one question worth answering next was whether Mark's recording is
simply too weak for the chain. It is not: a generated signal with added noise still
decodes at 2 dB, where its constellation reads 0.921, while the tape reads 0.396.
Noise fills the eye in; this recording's eye is being closed by the channel. That
is a different fault, and it is the first time the search has had a measured
direction rather than a list of suspects.

**Done.** Calibration of the chain against deliberately added noise at six levels,
using `s4285_test_signal` and `s4285_probe`; the table is in the handover. Nothing
in the tree changed for it — the experiments (wider phase window, frame-score
trace, WAV dump from the loopback test) were all reverted, and the tree is clean.

**Numbers.** Constellation 2-fold: clean 0.995, 10 dB 0.981, 6 dB 0.959, 3 dB
0.934, **2 dB 0.921 and the message still decodes** (SOM 32/32). The tape: 0.396,
references 0.55. Bench trap recorded: the generator's default sub-carrier is
1500 Hz, the probe's default 1800 Hz, and probing at the wrong one reads 0 frames
with a 0.999 lock — which looks like a broken modulator.

**Next.** An equaliser, not another convention: the eye is closed by ISI and timing
wander. The concrete first step is per-block channel estimates with a short
fractionally-spaced equaliser trained on the 48 known reference symbols, judged by
the same metric-vs-shuffled-null test — and, because the tape's references read
0.55 rather than 0.99, by the constellation first.

---

## 2026-10-24 (stanag, fifth pass) — the judge was reading the wrong stream

**Summary.** The check that decides whether the code is in the recording was
reading a stream with holes in it. The decoder is fed every locked frame, including
the poor ones (as erasures, so the interleaver stays aligned), while the check was
reading only the frames good enough to report — and one hole shifts every soft
decision after it by a whole frame, which is exactly enough to hide real code.
That is now fixed, and the answer on the correct stream is still no: the trellis
sits at its null over 555 frames of real decisions. Reading the right stream also
showed how much of this recording is actually poor: 56 % of the locked frames fall
below the gate that decides whether a frame is worth reporting.

**Done.**
- `S4285Receiver::trace_coded_stream(on)` and `coded_stream()`, with the reason
  documented on the method, and an assertion in the round-trip tests that the
  traced stream is whole frames (128 per frame, erasures included).
- `s4285_fec_check` judges the traced stream and its SOM against the same
  statistic on shuffled values, through the receiver's own chunking.

**Numbers.** Mark's file, 1850 Hz: 242 reported frames, **71 168** traced
decisions → 556 frames fed, **314 of them erasures**. Best ratio 1.00× its null.
With `MIN_CODED_REFERENCE` lowered to 0.0: 555 frames, 71 040 values, ratio
**1.00×** — erasures are not the cause. SOM on the traced stream: real **31/32**
against **27/32** shuffled; no branch reaches 32 and no message closes, so it stays
a near-miss. Root-raised-cosine receive filter (α 0.2/0.35/0.5) breaks acquisition
(lock 0.900 → 0.465) and was reverted. The file is dual mono — L and R decode
identically.

**Commands.** `cargo test --release -p sdroxide-dsp --lib stanag` → 16 passed, 1
ignored; `cargo check --workspace --all-targets` → 0 warnings; `rustfmt --check`
clean on everything touched.

**Next.** The symbols themselves are what is wrong, not the alignment: 555 frames
of real decisions carry no trellis. The useful measurement now is *how bad the
channel is per frame* — the reference concentration distribution over all 556
frames, not just the reported ones — because if the tape is 3–4 dB poorer than the
chain's threshold, the honest answer is a better front end (or a stronger
recording), not another convention sweep.

---

## 2026-10-24 (stanag, fourth pass) — the channel path cleared, and a false statistic retired

**Summary.** The choice between the plain symbols and the equaliser is not what is
stopping the tape from decoding — all three settings sit at their own null on the
best recording. Two real things came out of asking: the equaliser could hand back
a frame of silence and nothing noticed, which is now fixed and tested; and the
"closest SOM 31/32" that the handover quoted as near-success is a number that
fires on noise too, so it has been withdrawn. The trellis metric — the one
statistic here with a proper null — still says there is no code visible.

**Done.**
- `EqMode::{Auto,Off,On}` on `S4285Receiver` (`set_eq_mode`), honoured by the coded
  path only; the displayed frame is untouched.
- Fixed the equaliser's fallback in `coded_symbols`: it started its group-delay
  comparison with a zero-filled buffer, so a frame could be read as 128 zeros.
- New test `a_coded_message_survives_every_channel_path`: a clean generated 600 bps
  message decodes its own bits with `Auto`, `Off` **and** `On`. Verified failing
  before the fallback fix (`no message with On`, reference blocks 0.000).
- `s4285_fec_check` now reports, for every branch, the closest SOM to the known
  pattern against the same statistic on shuffled values, using the receiver's own
  chunking (256 pairs, 64-pair overlap).

**Numbers.** Mark's file, 1850 Hz: 242 frames, 30 976 soft decisions in all three
modes; best ratio Auto **0.99×**, Off **0.99×**, On **1.00×**. SOM: real 24–28/32,
shuffled 24–28/32 on every branch — no discrimination.

**Commands.** `cargo test --release -p sdroxide-dsp --lib stanag` → 16 passed, 1
ignored. `cargo check --workspace --all-targets` → 0 warnings. `rustfmt --check`
on both files clean.

**Still open.** The receiver's own SOM (31/32) is measured on a stream that
includes erasure-filled frames, so its null is a different question and remains
unestablished. Next: null-test it inside the chain (or report it as a z-score
against the expected maximum for that many bits) before it is trusted — if it
survives, some frames are decoding and that is where to dig.

---

## 2026-10-24 (stanag, third pass) — three hypotheses eliminated with numbers

**Summary.** The French Navy tape still does not decode here, but the search is
much narrower: the phase tracker, every plausible interleaver wiring and depth,
and a one-symbol matched filter have all been measured against the new recording
and none of them is the answer. What is left is the channel equaliser, and the raw
baseband capture is the file to do it on.

**Done.**
- `Interleave` made selectable (`announce` / `swapped` / `rows-flipped` /
  `swapped+flipped`) and the depth swept (3/6/12/24/48 at 600 bps), in
  `s4285_fec_check`. On Mark's file at 1900 Hz: **0.94–0.99×** its shuffled null
  for all 40 combinations. Commit `a9999e57`.
- A one-symbol boxcar matched filter before the decision: lock **0.900 → 0.465**,
  constellation **0.396 → 0.211**. Reverted.
- Handover carries both, with the numbers.

**Not done / blocked.** Nothing blocked. The remaining candidate is a **channel
equaliser** (Annex C/D) — a linear phase fit across the frame is all we have, and
the data symbols' 0.40 concentration says there is more channel than that. The
423 MB CF32 baseband IQ (`…-134238Z.cf32.wav`) is the file for it: the whole
passband, so the equaliser is not fighting an audio filter as well.

**Next.** Open the IQ, extract the channel at 13031.2 kHz, and see whether the
constellation improves when the whole passband is available — that separates "the
audio file is missing something" from "our equaliser is not enough" before any
equaliser is written.

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
