# Web audio latency harness

**How far behind is the sound, and how well does the reader recover from an
incident?**

    ./tools/web-audio-latency/run.sh          # or run.ps1 on Windows

The page is driven in a headless browser against the **real** bridge scripts
(`crates/sdroxide-web/assets/audio_bridge.js` and `pcm_worklet.js`) so that what
is measured is the code the client runs, not a copy of it. Audio is handed over
the way the wasm client hands it over: a lump per "frame", carrying everything
the audio clock says is due since the last one — which is what
`RemoteController::poll_event` does with `rx_pcm`. A frame that took longer than
usual therefore produces a bigger lump, as it does in the client.

Two passes, 48 kHz and 44.1 kHz, because the reader assumes a rate and the
browser need not give it one.

## What is measured

- **Backlog** — what the player still holds, from the counter it already posts
  (`{ rendered, queued }`, every ~170 ms). This is the reader's own delay: audio
  handed over, not yet played.
- **Continuity** — `tap_worklet.js`, connected to the player's output beside the
  destination and ending in a muted gain, counts the frames that reached the
  graph and the sample-to-sample discontinuities in them. A 1 kHz sine at 0.3
  has a second difference of about 0.005 at 48 kHz; an unfaded splice or a block
  dropped out of the middle of it is an order of magnitude larger. This is a
  proxy for a click, not a measurement of one.
- **Frequency** — the tap's zero crossings, against the tone that was pushed.
- **The device's share** — `AudioContext.baseLatency` and `outputLatency`, as
  the browser reports them.

## What is *not* measured

- **The delay to the speakers.** The backlog plus `baseLatency` and
  `outputLatency` is an estimate, not a measurement: there is no capture of the
  acoustic output here.
- **The ewebsock → wake → receive path.** This harness calls
  `sdroxideAudio.pushPcm` directly, which is the *end* of that path. Whether a
  hidden tab still receives, decodes and plays is a property of the compiled
  client and needs the built page in a real browser (see the plan's Lot B).
- **Background behaviour.** `page.html` is always in front, and the browser's
  handling of a hidden page is not reproducible in a headless run.

## What is asserted

The assertions are the behaviour wanted of a reader, not the numbers the shipped
one happens to produce:

- in ordinary running, the backlog stays under **150 ms** — below the 250 ms the
  shipped worklet settles at and below the point where a waterfall and a CW note
  are visibly out of step. The target a fix aims for is lower still and is
  decided by measurement, not by this number;
- a tone comes out at its own frequency, at both context rates;
- the backlog never goes below zero (it is a count of samples still to play);
- a 2 s block, a stalled frame, a flood of 60 s, and an underrun are each
  recovered from within a bounded time, and the edit is faded rather than cut;
- over a longer run the level does not creep upward.

`$env:LONG = "1800"` (or `LONG=1800 ./run.sh`) adds a thirty-minute phase to the
48 kHz pass for a drift hunt. The automatic suite keeps 20 s so it stays usable
in a session.

## The reader it shipped against

Recorded from `pcm_worklet.js` as it stands, Edge headless, 2026-10-27. These
are the reference the fix is compared with — **not** values to impose.

| | 48 kHz | 44.1 kHz |
|---|---|---|
| steady backlog, p95 | 229 ms (max 230) | 242 ms (max 247) |
| steady discontinuities | 0 | 38 |
| steady tone | 1000.0 Hz | **919.8 Hz** |
| steady drift, median start → end | 215 → 216 ms | 217 → 226 ms |
| a 2 s block | backlog 80 ms, **6 discontinuities** | backlog **−243 ms**, 4 |
| underrun, 600 ms stop | back to 78 ms; 618 ms rendered in silence | back to 165 ms |
| three 400 ms stalls | 61 ms after; 143 ms discarded; 17 discontinuities | 244 ms after |
| 60 s flood | **247 ms**, 36 discontinuities | **237 ms**, 38 |
| lowest backlog reading | **−67 ms** | **−413 ms** |

Three things in that table are the defects, and the harness fails on all three:
the backlog settles at the reader's ceiling instead of coming back down; the
edits that get it there are hard cuts; and a context that is not 48 kHz plays the
stream at the wrong rate. The negative backlog is the drop loop subtracting a
whole block from its counter when only part of it was still unplayed.

## Reading the output

One line per assertion, `PASS`/`FAIL` with the measured value, then `PASS suite
done`. The runner exits non-zero if any line failed or a pass did not finish.
`note` lines carry the context rate, the device latencies and the frames
rendered in silence.

## The unit of `queued`

The shipped worklet counts **source samples at 48 kHz** (`QUEUE_RATE` in
`harness.js`). A reader that counts output frames instead — which is what fixing
the rate handling means — must change that constant, and the change is part of
that fix, not of this harness.
