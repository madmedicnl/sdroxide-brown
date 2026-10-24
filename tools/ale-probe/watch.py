"""Hop the chain of 2G-ALE channels and keep anything that looks like a burst.

One process, one radio: capture a short slice per channel with `rspcap`, score it
with `scan.py`'s detector, and only keep a file that scored — so an hour of
listening costs a directory of nothing instead of tens of gigabytes.

    python3 watch.py 8            # eight rounds; ~9 min a round with 45 s dwell

The channel order starts with the amateur HFLINK data/sounding channels, because
those sound about once an hour and are the predictable traffic; the military
channels go last because their traffic is not on a schedule.
"""
import os
import subprocess
import sys
import time
from datetime import datetime, timezone

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
RSPCAP = os.path.normpath(os.path.join(HERE, "..", "rsp1-capture", "target", "release", "rspcap"))
OUT = "/tmp/opencode/ale-watch"

# kHz, in the order to visit them.
CHANNELS = [
    14109.0, 18106.0, 21096.0, 28146.0, 7102.0, 10145.5, 3596.0, 24926.0,
    11175.0, 8992.0, 15016.0,
]
DWELL = 45.0     # seconds per channel
RATE = 192_000.0
GAIN = "25"


def score(path, rate=RATE, nfft=8192, band=1000.0, width=8):
    """Best 2 kHz-wide, 8-sub-band block against each frame's own median."""
    raw = np.memmap(path, dtype=np.float32, mode="r")
    iq = raw[0::2]
    frames = (len(iq) - nfft) // nfft
    if frames <= 0:
        return []
    win = np.hanning(nfft).astype(np.float32)
    sub = int(band / (rate / nfft) + 0.5)
    nb = len(np.fft.rfftfreq(nfft, 1.0 / rate)) // sub
    if nb < width:
        return []
    hits = []
    for i in range(frames):
        seg = np.asarray(iq[i * nfft : i * nfft + nfft], dtype=np.float32) * win
        p = np.abs(np.fft.rfft(seg)) ** 2
        p = p[: nb * sub].reshape(nb, sub).mean(axis=1)
        med = float(np.median(p)) + 1e-30
        db = 10.0 * np.log10(p / med + 1e-30)
        for j in range(nb - width + 1):
            s = float(db[j : j + width].mean())
            if s >= 8.0:
                hits.append(((i + 0.5) * nfft / rate, (j + width / 2) * band, s))
    return hits


def main():
    rounds = int(sys.argv[1]) if len(sys.argv) > 1 else 4
    os.makedirs(OUT, exist_ok=True)
    print(f"watching {len(CHANNELS)} channels, {DWELL:.0f} s each, {rounds} round(s)", flush=True)
    for r in range(rounds):
        for khz in CHANNELS:
            tmp = os.path.join(OUT, "last.f32")
            hz = f"{khz:.1f}"
            subprocess.run(
                [RSPCAP, str(int(khz * 1000)), str(int(RATE)), str(DWELL), tmp, GAIN],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                check=False,
            )
            if not os.path.exists(tmp):
                print(f"{hz:>9} kHz: no capture", flush=True)
                continue
            hits = score(tmp)
            if hits:
                stamp = datetime.now(timezone.utc).strftime("%Y%m%d-%H%M%S")
                keep = os.path.join(OUT, f"hit-{hz}kHz-{stamp}.f32")
                os.replace(tmp, keep)
                best = max(hits, key=lambda h: h[2])
                print(
                    f"{hz:>9} kHz: {len(hits)} candidate frame(s), best t={best[0]:.1f}s "
                    f"offset={best[1]:.0f} Hz score={best[2]:.1f} dB -> {keep}",
                    flush=True,
                )
            else:
                os.remove(tmp)
                print(f"{hz:>9} kHz: quiet", flush=True)
    print("done", flush=True)


if __name__ == "__main__":
    main()
