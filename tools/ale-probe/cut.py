"""Cut N seconds of 8 kHz audio out of a wideband IQ capture, at one offset.

An ALE burst's eight tones sit 750–2500 Hz above the suppressed carrier, so the
detected block's centre is ~1625 Hz above the channel: the carrier to mix down
to is `offset - 1625`.
"""
import sys
import numpy as np
from scipy.signal import butter, sosfilt, resample_poly

path, secs, offset, dur, out = sys.argv[1], float(sys.argv[2]), float(sys.argv[3]), float(sys.argv[4]), sys.argv[5]
rate = 192_000.0
carrier = offset - 1625.0

raw = np.memmap(path, dtype=np.float32, mode="r")
iq = raw[0::2] + 1j * raw[1::2]
start = int(secs * rate)
n = int(dur * rate)
seg = iq[start : start + n].astype(np.complex64)
t = np.arange(len(seg), dtype=np.float64) / rate
seg = seg * np.exp(-2j * np.pi * carrier * t)

# Low-pass at the input rate, then decimate 24:1. Filtering after the mixer is
# what the handover calls out — a mixer without a filter leaves the mirror at
# -2·carrier inside the decimated band and half the symbols are gone.
sos = butter(6, 3200.0, "lowpass", fs=rate, output="sos")
x = sosfilt(sos, seg.real)
a = resample_poly(x, 1, 24)
a = (a / (np.abs(a).max() + 1e-9) * 0.9).astype(np.float32)
a.tofile(out)
print(f"{out}: {len(a)} samples = {len(a)/8000:.1f} s at 8 kHz, carrier {carrier:.0f} Hz")
