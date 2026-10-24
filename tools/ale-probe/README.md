# ALE probe — finding a 2G-ALE burst in a wideband capture

Two small Python scripts, beside `tools/rsp1-capture`, for the one thing the ALE
decoder is still missing: **a real burst**. They are measurement tools, not part
of the build, and nothing here is needed to run the program.

An ALE burst is eight FSK tones 250 Hz apart between 750 and 2500 Hz above the
suppressed carrier — a ~2 kHz block, one tone at a time, 125 baud, for a few
seconds. That shape is what separates it from voice (bursty, not flat-topped)
and from a carrier (one line).

## 1. Capture

```sh
cd ../rsp1-capture && cargo build --release
./target/release/rspcap 8992000 192000 600 /tmp/opencode/ale-8992.f32 25
```

- **192 kHz is accepted** by an RSP1 (unlike 48 kHz, which it refuses while
  keeping its own rate). 600 s is 0.9 GB, and the slice is ±96 kHz — enough for
  the channel, its neighbours and any dial error.
- **Gain 25.** At 40 the capture clipped (peak 0.999) on 11 MHz in the morning;
  the burst is not the loudest thing in the slice, so leave it headroom.
- Close sdroxide first: the SDRplay API is single-client.
- Channels worth trying, USB: **11175, 8992, 15016 kHz** (military/government
  networks) and the amateur 2G channels around **14109.5 kHz**.

## 2. Scan for the shape

```sh
python3 scan.py /tmp/opencode/ale-8992.f32
```

Builds a spectrogram of the whole slice and scores every 2 kHz-wide, 8-sub-band
window against that frame's own median. A burst scores 10 dB and up; a carrier
scores in one bin and does not fill the width. Prints `t=` (seconds into the
capture) and `offset=` (Hz above the dial, the centre of the tone block).

## 3. Cut one candidate and decode it

```sh
python3 cut.py /tmp/opencode/ale-8992.f32 123.0 47_000 8 /tmp/opencode/cand.f32
SDROXIDE_ALE_SAMPLE=/tmp/opencode/cand.f32 \
  cargo test -p sdroxide-dsp --release --lib ale:: -- --ignored --nocapture
```

`cut.py` mixes the offset down (the carrier is the block centre **minus
1625 Hz**, because that is where the eight tones sit relative to it), filters at
the input rate, decimates to 8 kHz and scales — the format the decoder's
off-air test reads.

## What has been tried

- **2026-10-24, 11175 kHz, 600 s, 07:26–07:36 UTC**: nothing. No 2 kHz-wide
  burst anywhere in ±96 kHz, and a 15 s look at the dial band showed flat noise
  with one narrow carrier.
- The capture settings that do work are above; the earlier failures in
  `docs/internal/ALE-HANDOVER.md` were a short-capture bug in the tool and a
  front end set by gain rather than left at default.
