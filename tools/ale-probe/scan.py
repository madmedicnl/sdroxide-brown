"""Find 2G-ALE-shaped bursts in a wideband IQ capture.

An ALE burst is eight FSK tones 250 Hz apart between 750 and 2500 Hz above the
suppressed carrier — a ~2 kHz block, one tone at a time, 125 baud, for a few
seconds at a time. In a 192 kHz slice that is a narrow, flat-topped block of
energy that lasts seconds and then is gone, and nothing else on an HF band looks
quite like it: voice is bursty but not 2 kHz-flat, carriers are one line.

Scans a grid of offsets and prints the best candidates as (seconds, offset Hz),
so only a handful of audio extractions need the decoder.
"""
import sys
import numpy as np

path = sys.argv[1] if len(sys.argv) > 1 else "/tmp/opencode/ale-11175.f32"
rate = 192_000.0
nfft = 8192          # 23.4 Hz bins
hop = 8192           # 42.7 ms per frame
band = 1000.0        # 1 kHz sub-bands for the width test

raw = np.memmap(path, dtype=np.float32, mode="r")
iq = raw[0::2]  # real part is enough: the slice is 192 kHz and the signal is
# within 3 kHz of some offset in it; the mirror is folded but a single-sideband
# signal at an offset the operator would tune to is clean.
frames = (len(iq) - nfft) // hop
if frames <= 0:
    sys.exit("capture too short")

win = np.hanning(nfft).astype(np.float32)
freqs = np.fft.rfftfreq(nfft, 1.0 / rate)
sub = int(band / (rate / nfft) + 0.5)  # bins per 1 kHz sub-band
nb = len(freqs) // sub

# dB per (frame, 1 kHz sub-band), relative to that frame's median
mat = np.empty((frames, nb), dtype=np.float32)
for i in range(frames):
    seg = np.asarray(iq[i * hop : i * hop + nfft], dtype=np.float32) * win
    p = np.abs(np.fft.rfft(seg)) ** 2
    p = p[: nb * sub].reshape(nb, sub).mean(axis=1)
    med = np.median(p) + 1e-30
    mat[i] = 10.0 * np.log10(p / med + 1e-30)

# An ALE burst occupies ~8 sub-bands (2.5 kHz). Score every window of 8 and keep
# the best per frame, then group frames that are close in time and offset.
W = 8
if nb < W:
    sys.exit("slice too narrow for the width test")
score = np.empty((frames, nb - W + 1), dtype=np.float32)
for j in range(nb - W + 1):
    score[:, j] = mat[:, j : j + W].mean(axis=1)

best = score.argmax(axis=1)
peak = score.max(axis=1)
cands = []
for i in range(frames):
    if peak[i] < 7.0:  # nothing worth extracting
        continue
    off = (best[i] + W / 2) * band  # centre of the window, Hz above the dial
    cands.append((i * hop / rate, off, float(peak[i])))

# **A burst, not a broadcast.** The width test alone is satisfied by any strong
# wide signal that lasts — a shortwave broadcast is 5 kHz of flat energy for the
# whole capture, and it tripped this detector on the first watch (with the
# candidate frames covering 99 % of the file). ALE is an FSK burst: it appears,
# runs a few seconds and goes. So count, per 1 kHz offset bucket, how many frames
# scored at all, and drop any bucket that is present for more than half the
# capture.
bucket_frames = {}
for i in range(frames):
    bucket_frames[int((best[i] + W / 2) * band) // 1000] = (
        bucket_frames.get(int((best[i] + W / 2) * band) // 1000, 0) + 1
    )
if frames:
    busy = {k for k, n in bucket_frames.items() if n > frames * 0.5}

# Group by (time within 15 s, offset within 3 kHz) and keep the strongest.
cands.sort(key=lambda c: -c[2])
picked = []
for t, off, s in cands:
    # **The channel, not the band.** The capture is centred on a known ALE
    # channel and the tones sit 750-2500 Hz above the suppressed carrier, so a
    # candidate anywhere else in the 192 kHz slice is a broadcaster or a carrier
    # that happens to be wide. Every false positive on the first watch was 18-74
    # kHz away.
    if not (500.0 <= off <= 3000.0):
        continue
    if int(off) // 1000 in busy:
        continue  # continuous: a broadcast or a carrier, not a burst
    if any(abs(t - t2) < 15.0 and abs(off - o2) < 3000.0 for t2, o2, _ in picked):
        continue
    picked.append((t, off, s))
    if len(picked) >= 12:
        break

print(f"{frames} frames, {frames * hop / rate:.1f} s scanned")
if busy:
    print(f"  {len(busy)} offset bucket(s) were continuous (broadcast/carrier) and are ignored")
for t, off, s in sorted(picked):
    print(f"  t={t:7.1f} s  offset={off:8.0f} Hz  score={s:5.1f} dB")
if not picked:
    print("  nothing that looks like a 2 kHz-wide burst")
