# The reference STANAG 4285 decoder, driven without Octave

`luwangg/signal-analysis` is a GPL-3.0 STANAG 4285 implementation with the same
shape as ours: a frame detector, an LMS decision-feedback equaliser, a
convolutional deinterleaver and a rate-1/2 K=7 Viterbi. Its own driver is
Octave glue (`src/s4285_cc.cc`), which is no use on a machine with no Octave —
so this is a small standalone driver around the *same* headers. Nothing of the
decoder is changed, and its licence is not ours to relicense: it is cloned, not
vendored.

**What it settled (2026-10-25).** On this station's 24 October capture of
13031.2 kHz the reference decodes the payload — its ITA2 reading is
`RYRYRYRYRYRYRYRY`, the French Navy test tape — while this tree's receiver
produces noise from the same audio. So the capture is good and the payload is
readable; the fault is ours. Feeding the reference's own soft decisions through
*our* deinterleaver and Viterbi still disagrees with its output on 63 per cent of
bits, which says the two coding chains are not the same machine either: its
metric is an XOR of saturated bytes (`viterbi2.hpp`), a hard decision in
disguise, ours is a soft correlation.

## Use

```sh
git clone --depth 1 https://github.com/luwangg/signal-analysis /tmp/sa
cp tools/s4285-reference/driver.cc /tmp/sa/
cd /tmp/sa
ffmpeg -loglevel error -y -i capture.wav -ac 1 -ar 12000 -f f32le cap.raw
g++ -O2 -std=c++17 -Iinclude \
    -include cstdint -include cassert -include iostream \
    -include algorithm -include vector -include array -include cmath \
    driver.cc -o driver
./driver cap.raw 2 1800 >/dev/null      # 2 = BPSK; the carrier is mixed to 0 Hz
```

It writes `/tmp/opencode/ref_bits_clean.txt` (its decoded bit stream) and
`/tmp/opencode/ref_soft.bin` (128 soft decisions per frame — its equaliser's
output, which is what the A/B test in the DSP crate consumes).

`-include` is not tidiness: the reference's headers assume an Octave prelude and
do not include what they use.

## Comparing

In the tree, with those two files in place:

```sh
cargo test -p sdroxide-dsp --release --lib \
    the_reference_soft_decisions_through_our_chain -- --ignored --nocapture
```

That runs our deinterleaver and Viterbi over the reference's soft decisions and
prints the agreement, the reference's bit count and ours. It is a measurement,
not a gate.
