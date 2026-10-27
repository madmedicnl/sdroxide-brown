// **How far behind is the sound, and how well does the reader hold its
// backlog?**
//
// The page drives the real bridge the way the wasm client does: a lump per
// "frame", carrying everything the audio clock says is due, which is exactly
// what `RemoteController::poll_event` hands over. A frame that took longer than
// usual therefore produces a *bigger* lump, as it does in the client.
//
// What is asserted is the behaviour wanted of a reader, not the numbers any
// particular one happens to produce:
//
//   * in ordinary running the backlog stays under CEILING_MS, the tone comes out
//     at its own frequency at both context rates, and nothing underruns;
//   * six tones across the audio band come out at their own frequency and their
//     own level, which is what a 48 -> 44.1 kHz conversion has to preserve;
//   * a 2 s block, a stalled frame, a flood of 60 s, an underrun and a reset are
//     each recovered from within a bounded time, and the edits are faded rather
//     than cut;
//   * the backlog never goes below zero and never exceeds the ring;
//   * over a longer run the level does not creep upward.
//
// Two meters, and they answer different questions:
//
//   * `tap_worklet.js`, connected to the player's output, counts the frames
//     that actually reached the graph, their energy, the discontinuities in
//     them, and the tone's frequency. It is independent of the code under test;
//   * the player's own port says how much it still holds -- the backlog -- and
//     what it had to do to hold it: underruns, edits, resyncs, resets, wraps.
//
// Neither is the speaker latency. `ageMs()` is the worklet's own backlog, and
// `baseLatency`/`outputLatency` are reported separately for the device's share.
(function () {
    const SRC = 48000; // the rate the server's PCM is in
    const RATE = window.__requestedRate || 48000;
    // The unit the player's `queued` counter is in: source frames at 48 kHz.
    // Kept from the shipped worklet so the two can be compared.
    const QUEUE_RATE = 48000;
    const TONE = 1000;
    // The reader's backlog ceiling, in milliseconds. Below the 250 ms the
    // first reader settled at, and below the point at which a waterfall and a
    // CW note are visibly out of step.
    const CEILING_MS = 150;
    const SWEEP = [300, 1000, 3000, 6000, 10000, 15000, 18000, 20000];
    // The widest band this asserts. 15 kHz is the widest *analog* mode the
    // server carries (WFM); above it the tone is measured and reported, not
    // asserted, because a DAB service can reach 20 kHz and the 8-point kernel
    // is not flat there.
    const SWEEP_BAND = 15000;
    const params = new URLSearchParams(location.search);
    const LONG_MS = Number(params.get("long") || 0) * 1000;
    const JITTER = params.get("jitter") === "1";
    const FULL = RATE === 48000;

    const say = (l) => window.__report(l);
    const wait = (ms) => new Promise((r) => setTimeout(r, ms));
    const now = () => performance.now();

    let phase = 0;
    let freq = TONE;
    let due = 0;
    let last = 0;
    let timer = null;
    let tap = null;
    // Every backlog reading, for the one property that must hold everywhere.
    const ages = [];

    function setFreq(f) {
        // Phase-continuous on purpose: a swept tone is still a tone, and a
        // change of slope is not a splice.
        freq = f;
    }

    function block(n) {
        const b = new Float32Array(n);
        const d = (2 * Math.PI * freq) / SRC;
        for (let i = 0; i < n; i++) {
            b[i] = 0.3 * Math.sin(phase);
            phase += d;
        }
        return b;
    }

    function arm() {
        const period = JITTER ? 20 + Math.random() * 40 : 30;
        timer = setTimeout(() => {
            const t = now();
            due += (t - last) * 0.001 * SRC;
            last = t;
            const n = Math.floor(due);
            if (n >= 128) {
                due -= n;
                window.sdroxideAudio.pushPcm(block(n));
            }
            arm();
        }, period);
    }

    function startPush() {
        if (timer) clearTimeout(timer);
        last = now();
        due = 0;
        arm();
    }

    function stopPush() {
        if (timer) clearTimeout(timer);
        timer = null;
    }

    function ageMs() {
        const r = window.__playerReport;
        if (!r || typeof r.queued !== "number") return null;
        return r.queued / QUEUE_RATE * 1000;
    }

    function snap() {
        const t = window.__tap || {
            frames: 0, energy: 0, splices: 0, crossings: 0, longestZeroRun: 0,
        };
        const p = window.__playerReport || {};
        const age = ageMs();
        if (age !== null) ages.push(age);
        return {
            t: now(),
            age,
            frames: t.frames,
            energy: t.energy,
            splices: t.splices,
            crossings: t.crossings,
            longestZeroRun: t.longestZeroRun,
            maxStep: t.maxStep || 0,
            peak: t.peak || 0,
            underruns: p.underruns || 0,
            edits: p.edits || 0,
            resyncs: p.resyncs || 0,
            resets: p.resets || 0,
            wraps: p.wraps || 0,
            oversized: p.oversized || 0,
            overwritten: p.overwritten || 0,
            driftPpm: p.driftPpm || 0,
            cap: p.capSrc || 0,
            target: p.targetSrc || 0,
            primed: !!p.primed,
        };
    }

    const d = (a, b, key) => b[key] - a[key];

    async function sampleFor(ms, step) {
        step = step || 100;
        const out = [];
        const t0 = now();
        while (now() - t0 < ms) {
            const s = snap();
            if (s.age !== null) out.push(s);
            await wait(step);
        }
        return out;
    }

    function pct(xs, q) {
        if (!xs.length) return null;
        const s = [...xs].sort((a, b) => a - b);
        return s[Math.min(s.length - 1, Math.floor(q * s.length))];
    }

    function freqHz(a, b) {
        const df = b.frames - a.frames;
        if (df <= 0) return 0;
        return (b.crossings - a.crossings) / 2 / (df / RATE);
    }

    function rms(a, b) {
        const df = b.frames - a.frames;
        if (df <= 0) return 0;
        return Math.sqrt(Math.max(0, b.energy - a.energy) / df);
    }

    function check(name, ok, detail) {
        say((ok ? "PASS " : "FAIL ") + name + " -- " + detail);
    }

    function note(name, detail) {
        say("note " + name + " -- " + detail);
    }

    function stall(ms) {
        const t = now();
        while (now() - t < ms) { /* one long task, as a busy frame is */ }
    }

    async function attachTap(ctx) {
        await ctx.audioWorklet.addModule("tap_worklet.js");
        tap = new AudioWorkletNode(ctx, "pcm-tap");
        tap.port.onmessage = (ev) => { window.__tap = ev.data; };
        window.__player.connect(tap);
        const mute = ctx.createGain();
        mute.gain.value = 0;
        tap.connect(mute);
        mute.connect(ctx.destination);
        // A spectrum tap on the same output, because a conversion product is a
        // line at the wrong frequency, not a level: no amount of tone gain tells
        // a fold-back from the tone that caused it.
        const an = ctx.createAnalyser();
        an.fftSize = 16384;
        an.smoothingTimeConstant = 0;
        window.__player.connect(an);
        an.connect(mute);
        window.__analyser = an;
        while (!window.__tap) await wait(20);
    }

    function spectrumPeak() {
        const an = window.__analyser;
        if (!an) return null;
        const bins = new Float32Array(an.frequencyBinCount);
        an.getFloatFrequencyData(bins);
        let best = 2;
        for (let i = 3; i < bins.length; i++) if (bins[i] > bins[best]) best = i;
        return { hz: best * (an.context.sampleRate / an.fftSize), db: bins[best] };
    }

    async function main() {
        window.dispatchEvent(new KeyboardEvent("keydown", { code: "KeyA", bubbles: true }));
        window.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));

        const t0 = now();
        while (now() - t0 < 6000
               && !(window.__player && window.__contexts.length
                    && window.__contexts[0].state === "running")) {
            await wait(50);
        }
        const ctx = window.__contexts[0];
        if (!ctx || ctx.state !== "running") {
            return finish("FAIL no running playback context (" + (ctx ? ctx.state : "none") + ")");
        }
        note("context", ctx.sampleRate + " Hz (asked " + RATE + "), baseLatency "
             + (ctx.baseLatency || 0) + " s, outputLatency " + (ctx.outputLatency || 0) + " s");

        // The client's cadence, started the way the app starts it: as soon as
        // the node exists, so the queue is loaded while the audio thread is
        // still coming up. The tap is attached beside it.
        startPush();
        await attachTap(ctx);
        await wait(2500);
        note("reader", "target " + snap().target + " source frames ("
             + Math.round((snap().target / SRC) * 1000) + " ms), ring " + snap().cap
             + " source frames" + (JITTER ? ", jittered cadence" : ""));

        // --- ordinary running -------------------------------------------------
        const steadyMs = FULL ? 20000 : 8000;
        let a = snap();
        const steady = await sampleFor(steadyMs);
        let b = snap();
        const steadyAges = steady.map((s) => s.age);
        const p95 = pct(steadyAges, 0.95);
        check("steady: backlog under ceiling", p95 !== null && p95 <= CEILING_MS,
              "p95 " + Math.round(p95) + " ms, max " + Math.round(pct(steadyAges, 1))
              + " ms (ceiling " + CEILING_MS + " ms)");
        check("steady: no underrun", d(a, b, "underruns") === 0,
              d(a, b, "underruns") + " underruns");
        check("steady: no splice", d(a, b, "splices") === 0,
              d(a, b, "splices") + " discontinuities");
        const hz = freqHz(a, b);
        check("steady: tone frequency", Math.abs(hz - TONE) <= TONE * 0.01,
              hz.toFixed(1) + " Hz (expected " + TONE + " Hz)");
        {
            // The mean, not the median: the backlog is a sawtooth (it is a lump
            // in, and a trickle out), so a median of single readings depends on
            // where the sampling happens to land in the saw. The mean is the
            // level.
            const mean = (xs) => xs.reduce((a, b) => a + b, 0) / Math.max(1, xs.length);
            const first = steady.slice(0, Math.floor(steady.length / 4)).map((s) => s.age);
            const end = steady.slice(-Math.floor(steady.length / 4)).map((s) => s.age);
            const p = mean(first), q = mean(end);
            check("steady: no upward drift", q <= p + 20,
                  "mean " + Math.round(p) + " ms at the start, " + Math.round(q) + " ms at the end");
        }
        note("steady", "edits " + d(a, b, "edits") + ", resyncs " + d(a, b, "resyncs")
             + ", wraps " + d(a, b, "wraps") + ", drift " + b.driftPpm + " ppm");

        // --- the audio band ---------------------------------------------------
        // A 1000 Hz tone proves the pitch; it does not prove the converter. The
        // app plays broadcast and DAB audio too, so the whole band the server
        // can send is measured: 20 dB below 300 Hz to 15 kHz would be an
        // audible dullness, not a test artefact.
        if (tap.port) tap.port.postMessage({ splices: false });
        for (const f of SWEEP) {
            setFreq(f);
            await wait(300);
            const s = snap();
            await wait(1200);
            const e = snap();
            const gain = 20 * Math.log10(rms(s, e) / (0.3 / Math.SQRT2));
            const got = freqHz(s, e);
            if (f <= SWEEP_BAND) {
                check("sweep: " + f + " Hz level", Math.abs(gain) <= 1.0, gain.toFixed(2) + " dB");
            } else {
                note("sweep", f + " Hz level " + gain.toFixed(2) + " dB (outside the declared band)");
            }
            check("sweep: " + f + " Hz pitch", Math.abs(got - f) <= f * 0.01,
                  got.toFixed(1) + " Hz");
        }
        if (tap.port) tap.port.postMessage({ splices: true });
        setFreq(TONE);
        await wait(400);

        // --- above the output Nyquist -----------------------------------------
        // At 44.1 kHz the output cannot represent anything above 22.05 kHz: a
        // tone above it either disappears (an anti-alias filter) or folds back
        // (an interpolator). The app's sources are band-limited below it -- the
        // README names where -- so this measures the worst case, not a station.
        {
            let ref = null;
            for (const f of [21000, 23000]) {
                setFreq(f);
                await wait(500);
                const pk = spectrumPeak();
                if (!pk) { note("spectrum", "no analyser node"); break; }
                note("spectrum", f + " Hz in, peak at " + pk.hz.toFixed(0) + " Hz, "
                     + pk.db.toFixed(1) + " dBFS");
                if (f === 21000) {
                    ref = pk;
                    check("spectrum: 21 kHz stays where it is", Math.abs(pk.hz - 21000) < 200,
                          pk.hz.toFixed(0) + " Hz");
                } else if (ref) {
                    note("spectrum", "a tone above the output Nyquist, relative to the 21 kHz one: "
                         + (pk.db - ref.db).toFixed(1) + " dB");
                }
            }
            setFreq(TONE);
            await wait(400);
        }

        // --- a block larger than the reader's own capacity --------------------
        const BIG_MS = 2000;
        if (tap.port) tap.port.postMessage({ resetPeak: true });
        a = snap();
        window.sdroxideAudio.pushPcm(block((SRC * BIG_MS) / 1000));
        await wait(1000);
        const oversize = await sampleFor(1000);
        b = snap();
        const overP95 = pct(oversize.map((s) => s.age), 0.95);
        check("oversize: backlog under ceiling", overP95 !== null && overP95 <= CEILING_MS,
              "p95 " + Math.round(overP95) + " ms after a " + BIG_MS + " ms block");
        check("oversize: the edit is faded", d(a, b, "splices") <= 1,
              d(a, b, "splices") + " discontinuities");
        note("oversize", "edits " + d(a, b, "edits") + ", resyncs " + d(a, b, "resyncs")
             + ", oversized " + Math.round(b.oversized || 0) + ", peak step " + b.peak);
        await wait(1500);

        // --- the sound stopping, then starting again --------------------------
        a = snap();
        stopPush();
        const silenceStart = a.frames;
        await wait(600);
        const silenceFrames = (window.__tap ? window.__tap.frames : 0) - silenceStart;
        b = snap();
        startPush();
        const underrun = await sampleFor(2000);
        const c = snap();
        check("underrun: playback resumes", c.crossings > b.crossings,
              c.frames - b.frames + " frames heard after the gap");
        const underP95 = pct(underrun.map((s) => s.age), 0.95);
        check("underrun: backlog under ceiling", underP95 !== null && underP95 <= CEILING_MS,
              "p95 " + Math.round(underP95) + " ms");
        note("underrun", silenceFrames + " frames rendered during a 600 ms stop, "
             + d(b, c, "underruns") + " underruns counted, "
             + d(a, c, "splices") + " discontinuities");

        // --- the reset the receive side will send when it drops a backlog -----
        a = snap();
        if (window.__player.port) window.__player.port.postMessage({ reset: true });
        await wait(800);
        const reset = await sampleFor(1500);
        b = snap();
        const resetP95 = pct(reset.map((s) => s.age), 0.95);
        check("reset: playback resumes", b.crossings > a.crossings,
              b.frames - a.frames + " frames heard after the reset");
        check("reset: backlog under ceiling", resetP95 !== null && resetP95 <= CEILING_MS,
              "p95 " + Math.round(resetP95) + " ms");
        note("reset", d(a, b, "resets") + " reset(s) counted, "
             + d(a, b, "splices") + " discontinuities");

        // --- stalled frames ---------------------------------------------------
        a = snap();
        let discarded = 0;
        for (let i = 0; i < 3; i++) {
            const before = ageMs();
            stall(400);
            const after = await sampleFor(300, 25);
            const lowest = after.length ? Math.min(...after.map((s) => s.age)) : null;
            if (before !== null && lowest !== null) {
                discarded = Math.max(discarded, before - lowest);
            }
            await wait(600);
        }
        const recovery = await sampleFor(1500);
        b = snap();
        const stallP95 = pct(recovery.map((s) => s.age), 0.95);
        check("stall: backlog returns under ceiling", stallP95 !== null && stallP95 <= CEILING_MS,
              "p95 " + Math.round(stallP95) + " ms after three 400 ms stalls");
        check("stall: the discarded audio is bounded", discarded <= 400 + CEILING_MS,
              Math.round(discarded) + " ms discarded for a 400 ms stall");
        note("stall", d(a, b, "splices") + " discontinuities (max step " + b.maxStep + "), " + d(a, b, "edits")
             + " edits, " + d(a, b, "resyncs") + " resyncs over three stalls");

        // --- half a second handed over at once --------------------------------
        // The largest burst the ring can hold, and what a stalled frame of that
        // length produces. It must come back to the target without a cut.
        if (tap.port) tap.port.postMessage({ resetPeak: true });
        a = snap();
        window.sdroxideAudio.pushPcm(block(SRC / 2));
        await wait(1000);
        const burst = await sampleFor(1000);
        b = snap();
        const burstP95 = pct(burst.map((s) => s.age), 0.95);
        check("burst: backlog under ceiling", burstP95 !== null && burstP95 <= CEILING_MS,
              "p95 " + Math.round(burstP95) + " ms after a 500 ms block");
        check("burst: the edit is faded", d(a, b, "splices") <= 1,
              d(a, b, "splices") + " discontinuities");
        note("burst", "edits " + d(a, b, "edits") + ", resyncs " + d(a, b, "resyncs") + ", peak step " + b.peak);
        await wait(1000);

        // --- an overrun: a hidden tab's sixty seconds, all at once -----------
        // A producer faster than real time destroys audio the reader has not
        // played yet, and no fade can be applied to samples the writer has
        // already overwritten. What is asserted is that the reader gives up on
        // the backlog and comes back to the target; the discontinuities that
        // leaves are counted and reported, not asserted. The receive side is not
        // supposed to hand this over: it discards the backlog and sends a reset
        // (Lot B).
        a = snap();
        for (let i = 0; i < 3000; i++) {
            window.sdroxideAudio.pushPcm(block(960));
        }
        await wait(1000);
        const flood = await sampleFor(1000);
        b = snap();
        const floodP95 = pct(flood.map((s) => s.age), 0.95);
        check("overrun: backlog bounded within 1 s", floodP95 !== null && floodP95 <= CEILING_MS,
              "p95 " + Math.round(floodP95) + " ms after 60 s of audio at once");
        note("overrun", "edits " + d(a, b, "edits") + ", resyncs " + d(a, b, "resyncs")
             + ", discontinuities " + d(a, b, "splices") + " (reported, not asserted)"
             + ", " + d(a, b, "overwritten") + " source frames dropped");

        if (LONG_MS > 0) {
            if (tap.port) tap.port.postMessage({ resetPeak: true });
            a = snap();
            const long = await sampleFor(LONG_MS, 500);
            b = snap();
            const xs = long.map((s) => s.age);
            const mean = (v) => v.reduce((q, r) => q + r, 0) / Math.max(1, v.length);
            const quarter = Math.max(1, Math.floor(xs.length / 4));
            const start = mean(xs.slice(0, quarter));
            const end = mean(xs.slice(-quarter));
            const drifts = long.map((s) => s.driftPpm);
            const lowest = xs.length ? Math.min(...xs) : 0;
            const highest = xs.length ? Math.max(...xs) : 0;
            const p95l = pct(xs, 0.95);
            const secs = Math.round(LONG_MS / 1000);
            check("long run: backlog under ceiling", p95l !== null && p95l <= CEILING_MS,
                  "p95 " + Math.round(p95l) + " ms over " + secs + " s");
            check("long run: no upward drift", end <= start + 20,
                  "mean " + Math.round(start) + " ms at the start, " + Math.round(end)
                  + " ms at the end");
            check("long run: no underrun", d(a, b, "underruns") === 0,
                  d(a, b, "underruns") + " over " + secs + " s");
            note("long run", secs + " s, " + xs.length + " readings: mean "
                 + Math.round(mean(xs)) + " ms, p95 " + Math.round(p95l) + ", min "
                 + Math.round(lowest) + ", max " + Math.round(highest)
                 + "; start " + Math.round(start) + " -> end " + Math.round(end)
                 + "; underruns " + d(a, b, "underruns") + ", edits " + d(a, b, "edits")
                 + ", resyncs " + d(a, b, "resyncs") + ", refused "
                 + d(a, b, "overwritten") + " source frames; drift "
                 + Math.min(...drifts) + ".." + Math.max(...drifts) + " ppm; discontinuities "
                 + d(a, b, "splices") + " (peak step " + b.peak + ")");
        }

        // --- properties that must hold everywhere -----------------------------
        const lowest = ages.length ? Math.min(...ages) : null;
        check("backlog: the counter never goes negative", lowest !== null && lowest >= 0,
              "lowest " + Math.round(lowest) + " ms over " + ages.length + " readings");
        const capMs = snap().cap ? (snap().cap / QUEUE_RATE) * 1000 : 0;
        const highest = ages.length ? Math.max(...ages) : null;
        check("backlog: never beyond the ring", capMs > 0 && highest !== null && highest <= capMs,
              "highest " + Math.round(highest) + " ms, ring " + Math.round(capMs) + " ms");

        finish(null);
    }

    let done = false;
    function finish(error) {
        if (done) return;
        done = true;
        stopPush();
        if (error) {
            say(error);
            say("FAIL suite did not run to the end");
        } else {
            say("PASS suite done");
        }
        fetch("/result?done=1");
    }

    setTimeout(() => finish("FAIL ran out of time"), 240000 + LONG_MS);
    main().catch((e) => finish("FAIL harness error: " + (e && e.stack ? e.stack : e)));
})();
