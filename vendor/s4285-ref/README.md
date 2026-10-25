# STANAG 4285 — the reference decoder, vendored

`luwangg/signal-analysis`'s STANAG 4285 decoder: a frame detector with a PLL, an
LMS decision-feedback equaliser, a convolutional deinterleaver and a rate-½ K=7
Viterbi. GPL-3.0 (`COPYING`), which this tree already is, and credited in the
README's Acknowledgements.

**Why it is vendored rather than ported.** This tree has its own STANAG 4285
receiver (`crates/sdroxide-dsp/src/stanag4285*.rs`) which reads the sync, the
reference blocks, the constellation and the *uncoded* text, and has never read the
coded payload off the air. The reference decoder reads this station's own 24
October capture of 13031.2 kHz — its ITA2 reading is `RYRYRYRY…`, `ZNR UUUUU`,
`ZUI TESTING` — where ours produces noise, and the difference was measured to be
the demodulated symbols themselves: theirs sit at ±1 with a small quadrature,
ours at half magnitude with quadrature as large.

A port of that equaliser to Rust is in the tree (`EqMode::Lms`) and does not
converge yet. Until it does, this is the working decoder, and it is the same
answer [`vendor/rade_c`](../rade_c) gives for RADE: the reference is C, the tree
is Rust, and a working mode beats a pure-Rust mode that decodes nothing.

**What is here is exactly upstream and unmodified** — `include/` and `COPYING`,
at `UPSTREAM-COMMIT`. Theirs is not ours to relicense. Everything this tree adds
is in [`crates/sdroxide-s4285`](../../crates/sdroxide-s4285): a build script, a C
shim that drives their classes without Octave, and a safe wrapper.

Upstream: <https://github.com/luwangg/signal-analysis>
