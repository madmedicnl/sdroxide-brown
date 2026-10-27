// **How far behind is the sound?**
//
// The page drives the real bridge the way the wasm client does: a lump per
// "frame", carrying everything the audio clock says is due, which is exactly
// what `RemoteController::poll_event` hands over. A frame that took longer than
// usual therefore produces a *bigger* lump, as it does in the client.
//
// What is asserted is the behaviour wanted of the reader, not the numbers the
// broken one happens to produce:
//
//   * the reader never sits more than CEILING_MS of audio behind what it was
//     given, in ordinary running;
//   * a tone comes out at the tone's frequency, whatever the context rate;
//   * an incident (a stalled frame, an oversized block, a flood, an underrun)
//     is recovered from in a bounded time, and the transition is faded rather
//     than cut;
//   * the level does not creep upward over a longer run.
//
// The measured values are reported beside every line, pass or fail: they are
// the reference the fix has to be compared with, not a target copied from the
// broken build.
//
// Two meters, and they answer different questions:
//
//   * `tap_worklet.js`, connected to the player's output, counts the frames
//     that actually reached the graph, the discontinuities in them, and the
//     tone's frequency. It is independent of the code under test;
//   * the player's own port says how much it still holds -- the backlog.
//
// Neither is the speaker latency. `ageMs()` is the worklet's own backlog, and
// `baseLatency`/`outputLatency` are reported separately for the device's share.
// The tap cannot measure the backlog: the player renders silence while it
// primes and while it underruns, so "handed over minus rendered" counts dropped
// audio as though it were still queued.
(function () {
    const SRC = 48000;               // the rate the server's PCM is in
    const RATE = window.__requestedRate || 48000;
    // The unit the player's own `queued` counter is in. The shipped worklet
    // counts source samples at 48 kHz; a worklet that counts output frames must
    // change this to RATE, and Lot A does. The harness says which it is
    // assuming rather than guessing from a number.
    const QUEUE_RATE = 48000;
    const TONE = 1000;
    // The reader's backlog ceiling, in milliseconds. Below the 250 ms the
    // shipped worklet settles at, and below the point at which a waterfall and
    // a CW note are visibly out of step; the target the fix aims for is lower
    // still and is decided by measurement, not by this number.
    const CEILING_MS = 150;
    const LONG_MS = Number(new URLSearchParams(location.search).get("long") || 0) * 1000;
    const FULL = RATE === 48000;

    const say = (l) => window.__report(l);
    const wait = (ms) => new Promise((r) => setTimeout(r, ms));
    const now = () => performance.now();

    let phase = 0;
    let due = 0;
    let last = 0;
    let timer = null;
    let tap = null;
    // Every backlog reading, for the one property that must hold everywhere:
    // a backlog is a count of samples still to play, so it cannot go below
    // zero. (The shipped reader can, because its drop loop subtracts a whole
    // block when only part of it was still unplayed.)
    const ages = [];

    function block(n) {
        const b = new Float32Array(n);
        for (let i = 0; i < n; i++) {
            b[i] = 0.3 * Math.sin((2 * Math.PI * TONE * (phase + i)) / SRC);
        }
        phase += n;
        return b;
    }

    function startPush(period) {
        if (timer) clearInterval(timer);
        last = now();
        due = 0;
        timer = setInterval(() => {
            const t = now();
            due += (t - last) * 0.001 * SRC;
            last = t;
            const n = Math.floor(due);
            if (n >= 128) {
                due -= n;
                window.sdroxideAudio.pushPcm(block(n));
            }
        }, period);
    }

    function stopPush() {
        if (timer) clearInterval(timer);
        timer = null;
    }

    // The reader's backlog, in milliseconds: what the player says it still
    // holds. This is the worklet's own share only -- the device adds the
    // context's `baseLatency` and `outputLatency`, reported separately, and
    // neither is a measurement of the delay to the speakers.
    //
    // The tap cannot supply this number: it counts frames rendered, and the
    // player renders silence while it primes and while it underruns, so
    // `handed over minus rendered` counts dropped audio as if it were still
    // queued. The tap's job is continuity and the tone's frequency.
    function ageMs() {
        const r = window.__playerReport;
        if (!r) return null;
        return r.queued / QUEUE_RATE * 1000;
    }

    function snap() {
        const t = window.__tap || { frames: 0, splices: 0, crossings: 0, longestZeroRun: 0 };
        const age = ageMs();
        if (age !== null) ages.push(age);
        return {
            t: now(),
            frames: t.frames,
            splices: t.splices,
            crossings: t.crossings,
            longestZeroRun: t.longestZeroRun,
            age,
        };
    }

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
        return ((b.crossings - a.crossings) / 2) / (df / RATE);
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

    const failed = [];

    async function attachTap(ctx) {
        await ctx.audioWorklet.addModule("tap_worklet.js");
        tap = new AudioWorkletNode(ctx, "pcm-tap");
        tap.port.onmessage = (ev) => { window.__tap = ev.data; };
        window.__player.connect(tap);
        const mute = ctx.createGain();
        mute.gain.value = 0;
        tap.connect(mute);
        mute.connect(ctx.destination);
        while (!window.__tap) await wait(20);
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
        // still coming up -- which is where a session's first backlog comes
        // from. The tap is attached beside it; its job starts a moment later.
        startPush(30);
        await attachTap(ctx);
        await wait(2500);         // let the reader settle, whatever it settles at

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
        check("steady: no splice", b.splices - a.splices === 0,
              (b.splices - a.splices) + " discontinuities");
        const hz = freqHz(a, b);
        check("steady: tone frequency", Math.abs(hz - TONE) <= TONE * 0.01,
              hz.toFixed(1) + " Hz (expected " + TONE + " Hz)");
        {
            // The level at the end must not be above the level at the start,
            // once settled. At a context rate that is not the source rate the
            // reader is fed faster than it drains, which is exactly this.
            const first = steady.slice(0, Math.floor(steady.length / 4)).map((s) => s.age);
            const end = steady.slice(-Math.floor(steady.length / 4)).map((s) => s.age);
            const p = pct(first, 0.5), q = pct(end, 0.5);
            check("steady: no upward drift", p !== null && q !== null && q <= p + 20,
                  "median " + Math.round(p) + " ms at the start, " + Math.round(q) + " ms at the end");
        }

        // --- a block larger than the reader's own capacity --------------------
        const BIG_MS = 2000;
        a = snap();
        window.sdroxideAudio.pushPcm(block(SRC * BIG_MS / 1000));
        await wait(1000);
        const oversize = await sampleFor(1000);
        b = snap();
        const overP95 = pct(oversize.map((s) => s.age), 0.95);
        check("oversize: backlog under ceiling", overP95 !== null && overP95 <= CEILING_MS,
              "p95 " + Math.round(overP95) + " ms after a " + BIG_MS + " ms block");
        check("oversize: the edit is faded", b.splices - a.splices <= 1,
              (b.splices - a.splices) + " discontinuities");
        await wait(1500);

        // --- the sound stopping, then starting again --------------------------
        a = snap();
        stopPush();
        const silenceStart = a.frames;
        await wait(600);
        const silenceFrames = (window.__tap ? window.__tap.frames : 0) - silenceStart;
        b = snap();
        startPush(30);
        const underrun = await sampleFor(2000);
        const c = snap();
        check("underrun: playback resumes", c.crossings > b.crossings,
              (c.frames - b.frames) + " frames heard after the gap");
        const underP95 = pct(underrun.map((s) => s.age), 0.95);
        check("underrun: backlog under ceiling", underP95 !== null && underP95 <= CEILING_MS,
              "p95 " + Math.round(underP95) + " ms");
        note("underrun", silenceFrames + " frames rendered during a 600 ms stop");

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
        note("stall", (b.splices - a.splices) + " discontinuities over three stalls");

        // --- the return from a hidden tab: already-decoded audio, all at once --
        a = snap();
        for (let i = 0; i < 3000; i++) {
            window.sdroxideAudio.pushPcm(block(960));
        }
        await wait(1000);
        const flood = await sampleFor(1000);
        b = snap();
        const floodP95 = pct(flood.map((s) => s.age), 0.95);
        check("flood: backlog bounded within 1 s", floodP95 !== null && floodP95 <= CEILING_MS,
              "p95 " + Math.round(floodP95) + " ms after 60 s of audio at once");
        check("flood: the edit is faded", b.splices - a.splices <= 3,
              (b.splices - a.splices) + " discontinuities");

        if (LONG_MS > 0) {
            a = snap();
            const long = await sampleFor(LONG_MS, 500);
            b = snap();
            const lp = pct(long.slice(0, 10).map((s) => s.age), 0.5);
            const lq = pct(long.slice(-10).map((s) => s.age), 0.5);
            check("long run: no upward drift", lp !== null && lq !== null && lq <= lp + 20,
                  "median " + Math.round(lp) + " ms at the start, " + Math.round(lq) + " ms after "
                  + Math.round(LONG_MS / 1000) + " s");
        }

        const lowest = ages.length ? Math.min(...ages) : null;
        check("backlog: the counter never goes negative", lowest !== null && lowest >= 0,
              "lowest " + Math.round(lowest) + " ms over " + ages.length + " readings");

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

    setTimeout(() => finish("FAIL ran out of time"), 120000);
    main().catch((e) => finish("FAIL harness error: " + (e && e.stack ? e.stack : e)));
})();
