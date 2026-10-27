# Web audio latency harness

**How far behind is the sound, and how well does the reader hold its backlog?**

    ./tools/web-audio-latency/run.sh          # or run.ps1 on Windows

    LONG=1800 RATES=48000 ./tools/web-audio-latency/run.sh   # one thirty-minute run
    JITTER=1 ./tools/web-audio-latency/run.sh               # 20-50 ms hand-overs

It runs `tests.html` first — the worklet's internal invariants, driven
deterministically with the audio-thread globals stubbed — and then the harness
at 48 kHz and again at 44.1 kHz.

The harness page is driven in a headless browser against the **real** bridge
scripts (`crates/sdroxide-web/assets/audio_bridge.js` and `pcm_worklet.js`) so
that what is measured is the code the client runs, not a copy of it. Audio is
handed over the way the wasm client hands it over: a lump per "frame", carrying
everything the audio clock says is due since the last one — which is what
`RemoteController::poll_event` does with `rx_pcm`. A frame that took longer than
usual therefore produces a bigger lump, as it does in the client.

Two passes, 48 kHz and 44.1 kHz, because the reader assumes a rate and the
browser need not give it one. `RATES` narrows that to one, and then `LONG`
applies to it; without `RATES`, `LONG` applies to the 48 kHz pass only.

## What is measured

- **Backlog** — what the player still holds, from the counter it posts every
  ~170 ms. `queued` is in **source frames at 48 kHz**: the same unit as before
  the reader was rewritten, so the two can be compared. This is the reader's own
  delay — audio handed over, not yet played.
- **Continuity** — `tap_worklet.js`, connected to the player's output beside the
  destination and ending in a muted gain, counts the frames that reached the
  graph and the sample-to-sample discontinuities in them. A 1 kHz sine at 0.3
  has a second difference of about 0.005 at 48 kHz; an unfaded splice is an
  order of magnitude larger. This is a proxy for a click, not a measurement of
  one, and it is disarmed while the swept tone changes frequency — a change of
  slope is not a splice.
- **The audio band** — the tone's frequency and level at 300 Hz, 1, 3, 6, 10 and
  15 kHz, measured on the same tap; 18 and 20 kHz are measured and reported.
- **Conversion products** — an `AnalyserNode` on the same output: a tone at
  21 kHz (inside the output Nyquist) must stay there, and one at 23 kHz (outside
  it on a 44.1 kHz context) shows where it folds back and by how much.
- **The device's share** — `AudioContext.baseLatency` and `outputLatency`.

## Deterministic tests

`tests.html` loads `pcm_worklet.js` with the audio-thread globals stubbed and
drives `process()` and the port directly: no audio clock, no delivery jitter,
no device. It asserts the reader's internal state - the **raw `w - r`**, before
any clamp the telemetry applies - across the interpolation window crossing the
ring's boundary (a ramp must come back as itself), reading at the very end of
what is available, a message larger than the ring, a write during an edit, a
write during a resync, a reset during playback, and an underrun and the resume
after it.

The twelve cases check **which stream the samples come from**, not merely that
the output is finite. Where the harness writes a stream itself the sample value
equals its absolute position, so every output has a computed reference; slots
that hold no stream are filled with a sentinel (-999, or a previous generation
at +0.5), so a stale read is an out-of-range value rather than a NaN that any
arithmetic hides. The cases include a crossfade near the start of the stream,
one after more than five full turns of the ring, a write while a fade is
running (both branches must keep their positions), a forced state with no
history behind the read position (it must fade through silence, not wrap), and
a reset followed by a different generation (no sample of the old generation may
reappear once the reset's transition is over). The history rule the crossfade
relies on - `r - 3 >= max(0, w - cap)` - is therefore exercised, not assumed.

## What is *not* measured

- **The delay to the speakers.** The backlog plus `baseLatency` and
  `outputLatency` is an estimate, not a measurement: there is no capture of the
  acoustic output here.
- **The ewebsock → wake → receive path.** This harness calls
  `sdroxideAudio.pushPcm` directly, which is the *end* of that path. Whether a
  hidden tab still receives, decodes and plays is a property of the compiled
  client and needs the built page in a real browser (Lot B).
- **Background behaviour.** `page.html` is always in front, and the browser's
  handling of a hidden page is not reproducible in a headless run.

## What is asserted

The assertions are the behaviour wanted of a reader, not the numbers the shipped
one happens to produce:

- in ordinary running the backlog stays under **150 ms** and nothing underruns,
  with no discontinuity; a tone comes out at its own frequency; the level does
  not creep upward;
- the whole band the server can send is measured: within **1 dB** to 15 kHz, at
  both context rates;
- a 2 s block, a 500 ms burst, a stalled frame, an underrun and a reset are each
  recovered from within a bounded time, and the edits are faded rather than cut;
- the backlog never goes below zero and never exceeds the ring.

One scenario is measured but **not** asserted: sixty seconds of audio pushed
synchronously. A producer faster than real time destroys audio the reader has
not played yet, and no fade can be applied to samples the writer has already
overwritten, so discontinuities there are inherent and are reported rather than
required. The receive side must not hand that over — it discards the backlog and
sends a reset (Lot B).

`$env:LONG = "1800"` (or `LONG=1800 ./run.sh`) adds a thirty-minute phase to the
48 kHz pass for a drift hunt.

## The reader it shipped against

The measured values of the reader that preceded this harness. **Reference only**
— they are what the fix is compared with, not values to impose.

Recorded from `pcm_worklet.js` as it stood, Edge headless, 2026-10-27:

| | 48 kHz | 44.1 kHz |
|---|---|---|
| steady backlog, p95 | 229 ms (max 230) | 242 ms (max 247) |
| steady discontinuities | 0 | 38 |
| steady tone | 1000.0 Hz | **919.8 Hz** |
| a 2 s block | backlog 80 ms, **6 discontinuities** | backlog **−243 ms**, 4 |
| underrun, 600 ms stop | back to 78 ms; 618 ms rendered in silence | back to 165 ms |
| three 400 ms stalls | 61 ms after; 143 ms discarded; 17 discontinuities | 244 ms after |
| 60 s flood | **247 ms**, 36 discontinuities | **237 ms**, 38 |
| lowest backlog reading | **−67 ms** | **−413 ms** |

Three things in that table were the defects: the backlog settled at the reader's
ceiling instead of coming back down, the edits that got it there were hard cuts,
and a context that was not 48 kHz played the stream at the wrong rate. The
negative backlog was the drop loop subtracting a whole block from its counter
when only part of it was still unplayed.

## After the rewrite

Same machine, same day. The steady and incident rows are one run at the final
settings; the thirty-minute rows are separate runs, one per rate, each with the
final 80 ms target and nothing else on the machine. The jittered row is a third
run.

**Timing of the thirty-minute runs.** Those runs were recorded before the last
three fade-transition corrections (negative-index wrap in the linear fallback,
the history-validity guard on the crossfade, and cancelling a stale prime
fade-in at the start of a crossfade). They recorded **0 edits and 0 resyncs**,
so none of those corrected paths executed during them, and the stationary
figures (mean/p95/min/max backlog, drift, discontinuities) are kept as
representative of the steady state. The deterministic suite
(`worklet_tests.js`, 12 cases with computed references, including a
two-generation reset and a write during a fade) and the short incident suites
at both rates were re-run on the final code: PASS. The corrected paths are
therefore covered by tests, not by the long runs.

| | 48 kHz | 44.1 kHz |
|---|---|---|
| steady backlog, p95 | 75 ms (max 78) | 77 ms (max 89) |
| steady underruns / discontinuities | 0 / 0 | 0 / 0 |
| steady mean level, start → end | 57 → 62 ms | 64 → 70 ms |
| steady tone | 1000.0 Hz | 999.7–999.8 Hz |
| 300 Hz – 10 kHz, level | 0.00 dB | 0.00 dB |
| 15 kHz, level | 0.00 dB, or **−0.50 dB** when correcting | **−0.51 dB** |
| 18 kHz / 20 kHz (reported only) | 0.00 / 0.00 dB | −1.4 / −2.4 dB |
| a 2 s block | 0 discontinuities | 0 |
| 500 ms burst | 0 | 0 |
| underrun / reset | 0 / 0 | 0 / 0 |
| three 400 ms stalls | 0 | 0 |
| jittered cadence (20–50 ms) | 0 underruns, p95 101 ms | 0 underruns, p95 95 ms |
| 21 kHz tone in / out | 21.0 / 21.0 kHz | 21.0 / 21.0 kHz |
| 23 kHz tone in / out | 23.0 / 23.0 kHz | 23.0 / **21.1 kHz, −2.0 to −2.6 dB** |
| **30 minutes at 80 ms** | mean 71, p95 86, min 43, max 92 ms; start 70 → end 71; 0 underruns, 0 edits, 0 resyncs, 0 refused; drift −223..0 ppm; 0 discontinuities | mean 70, p95 86, min 45, max 92 ms; start 69 → end 69; 0 underruns, 0 edits, 0 resyncs, 0 refused; drift −204..0 ppm; 0 discontinuities |
| overrun, 60 s at once | 18–61 discontinuities (reported) | 48 |
| lowest backlog reading | 0 ms | 0 ms |
| highest backlog reading | 90 ms | 90 ms |

The 48 kHz column reads 0.00 dB at every tone when the correction's smoothed
error sits inside its dead band: the reader then copies whole frames and does
no interpolation at all. When a clock disagreement takes it outside that band
the same 8-point kernel applies there as at 44.1 kHz, and 15 kHz measures
−0.50 dB.

The 80 ms target is what the jittered cadence chose: at 60 ms the same run
underran two or three times in twenty seconds (and eight at 44.1 kHz), at 80 ms
it did not. The worst second difference measured inside the oversize and burst
phases was 0.006–0.007, against the detector's 0.05 threshold.

## The band the conversion has to carry

At 44.1 kHz the output Nyquist is **22.05 kHz**; anything above it folds back.
What the sources put there, by code:

- SSB and CW: the demodulator's own passband, about 3 kHz.
- AM: about 6 kHz.
- WFM: a 383-tap 15 kHz low-pass ahead of the decimation — `WfmDemod` in
  `crates/sdroxide-dsp/src/demod.rs` (`WFM_LPF_TAPS`, `RealFirDecim::new(…,
  15_000.0, …)`), whose comment puts the pilot 111 dB down.
- DAB/DAB+ and DRM: the AAC / xHE-AAC decoders' own bandwidth, about 20 kHz
  (`sdroxide-dab`, `sdroxide-drm`); the tree does not filter the audio again.

So nothing is *expected* above 22.05 kHz, and the kernel is chosen for its
passband. It is not an anti-alias filter, and the harness measures what that
means: a tone pushed at 23 kHz comes out at **21.1 kHz** on a 44.1 kHz context,
at **−2.2 dB** relative to a 21 kHz tone that passes properly. That is the worst
case, not a station — but it is a real limit, written down rather than assumed.

Measured levels at 44.1 kHz: 0.00 dB to 10 kHz, −0.51 dB at 15 kHz, −1.4 dB at
18 kHz, **−2.4 dB at 20 kHz**. The last is a known degradation of the top
octave of a DAB service, which can reach 20 kHz. The 15 kHz figure is where the
harness *asserts*; it is not a claim that the product's sources stop there, and
it should not be read as one. Flattening 20 kHz would need a windowed-sinc of
several dozen taps, which is a separate decision with its own cost.

The 48 kHz column reads 0.00 dB at every tone because its start-up backlog sits
inside the correction's dead band, so the reader copies whole frames and does no
interpolation at all. When a real sound card's clock disagrees enough to leave
the dead band, the same 8-point kernel applies there as at 44.1 kHz.

## Saturation: what is refused, what is kept

The ring holds 500 ms of source audio, and the writer leaves 20 ms of it unread
(`reserve`) so that a fade-out always has untouched samples ahead of it.

- `write` never modifies a sample in `[r, w)` — the audio the reader is about to
  play. It accepts the **newest** `min(n, free)` samples of a block, where
  `free = cap - ceil(w - r) - reserve`, and appends them at `w`. The rest is
  **refused**, and `overwritten` counts those source frames: it is input that
  never entered the ring, not ring content that was replaced. `r` moves only by
  playback, and by the edits and resyncs the reader itself decides on.
- A block larger than the ring keeps its newest `cap` first, then the same rule
  applies.
- When the backlog passes target + 300 ms the reader resyncs: 5 ms fade-out, a
  skip of `w - r - target`, 5 ms fade-in. That is where unread audio is given
  up — deliberately, and on a counter (`resyncs`), never by overwriting it.

So sixty seconds handed over at once is deterministic rather than lucky: the
first ~480 ms is accepted, the rest refused, one resync leaves the target, and
the latency is bounded from then on. The *programme* has a hole where the
refused audio was — an interruption no reader can avoid — but nothing is played
out of a half-overwritten block and the fade across the hole is continuous. The
receive side must not hand this over: it discards the backlog and sends a reset
(Lot B).

## Reading the output

One line per assertion, `PASS`/`FAIL` with the measured value, then `PASS suite
done`. The runner exits non-zero if any line failed or a pass did not finish.
`note` lines carry the context rate, the device latencies, the edits and
resyncs, and the frames rendered in silence.

## The unit of `queued`

The reader counts **source frames at 48 kHz**, and the harness divides by that
(`QUEUE_RATE`). It is the same unit the shipped reader used, so the two tables
are comparable; a reader that reports output frames instead must change that
constant, and the change is part of that fix, not of this harness.
