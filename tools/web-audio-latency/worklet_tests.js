// Deterministic tests of the playback reader's internals.
//
// The worklet is loaded on this page with the audio-thread globals stubbed, so
// `process()` and the port can be driven directly: no audio clock, no delivery
// jitter, no device. What is asserted is the reader's own state between calls,
// the *raw* backlog `w - r` -- before any clamp the telemetry applies -- and,
// above all, **which stream the samples come from**: a wrap makes an address
// valid, it does not make the data behind it belong to this stream.
//
// Two conventions make stale data visible rather than plausible:
//
//   * wherever the harness writes a stream itself, the sample value equals its
//     absolute position, so every output has a computable reference;
//   * every slot that holds no stream is filled with a sentinel (-999, or a
//     previous generation at +0.5), so reading an old generation is a value
//     out of range rather than a NaN that any arithmetic hides.
(function () {
    const say = (l) => window.__report(l);
    const P = window.__processors["pcm-player"];
    let failures = 0;

    function check(name, ok, detail) {
        if (!ok) failures++;
        say((ok ? "PASS " : "FAIL ") + name + " -- " + detail);
    }

    function note(name, detail) {
        say("note " + name + " -- " + detail);
    }

    // The invariant every test re-checks, on the raw fields.
    function inv(p, where) {
        const raw = p.w - p.r;
        check("invariant: raw backlog >= 0 (" + where + ")", raw >= 0,
              "w - r = " + raw.toFixed(3));
        check("invariant: backlog within the ring (" + where + ")",
              raw <= p.cap + 1e-9, "w - r = " + raw.toFixed(3) + ", cap " + p.cap);
    }

    function feedRamp(p, from, count) {
        const a = new Float32Array(count);
        for (let i = 0; i < count; i++) a[i] = from + i; // value == position
        p.port.onmessage({ data: a });
    }

    function feedValue(p, value, count) {
        const a = new Float32Array(count);
        a.fill(value);
        p.port.onmessage({ data: a });
    }

    function process(p, frames) {
        const out = new Float32Array(frames);
        p.process([], [[out]]);
        return out;
    }

    const fresh = () => new P();

    // The blend a crossfade must produce when both branches carry the ramp:
    // old at `rOld`, new at `rNew`, equal power, `xfLen` frames.
    function blendRef(rOld, rNew, xfLen, i) {
        const g = ((i / xfLen) * Math.PI) / 2;
        return Math.cos(g) * (rOld + i) + Math.sin(g) * (rNew + i);
    }

    function worstVs(out, ref, from) {
        let worst = 0;
        for (let i = 0; i < out.length; i++) {
            worst = Math.max(worst, Math.abs(out[i] - ref(i + (from || 0))));
        }
        return worst;
    }

    if (!P) {
        say("FAIL pcm-player was not registered");
        say("FAIL worklet tests did not run");
        fetch("/result?done=1");
        return;
    }

    // --- 1. the interpolation window, across the ring's boundary -------------
    {
        const p = fresh();
        const cap = p.cap;
        p.ring.fill(NaN);
        const total = cap + 400;
        for (let k = 0; k < total; k++) p.ring[k % cap] = k * 1e-3;
        p.w = total;
        p.r = cap - 3.5;
        p.primed = true;
        p.avg = p.target;
        const out = process(p, 64);
        let worst = 0;
        let nan = 0;
        for (let i = 0; i < out.length; i++) {
            if (Number.isNaN(out[i])) { nan++; continue; }
            worst = Math.max(worst, Math.abs(out[i] - (cap - 3.5 + i) * 1e-3));
        }
        check("wrap: a ramp comes back as itself", nan === 0 && worst < 5e-4,
              "worst error " + worst.toExponential(2) + ", " + nan + " NaN");
        inv(p, "wrap");
    }

    // --- 2. reading at the very end of what is available ---------------------
    {
        const p = fresh();
        p.ring.fill(NaN);
        for (let k = 0; k < 1000; k++) p.ring[k % p.cap] = k * 1e-3;
        p.w = 1000;
        p.r = 997.5;
        p.primed = true;
        p.avg = p.target;
        const out = process(p, 128);
        let nan = 0;
        for (let i = 0; i < 3; i++) if (Number.isNaN(out[i])) nan++;
        const tail = out.subarray(3).every((x) => x === 0);
        check("end of data: what is there is played, the rest is silence",
              nan === 0 && tail, nan + " NaN, tail silent: " + tail);
        check("end of data: an underrun is counted and playback stops",
              p.underruns === 1 && p.primed === false,
              "underruns " + p.underruns + ", primed " + p.primed);
        const raw = p.w - p.r;
        check("end of data: the read position never passes the write position", raw >= 0 && raw < 1,
              "w - r = " + raw.toFixed(3));
        inv(p, "end of data");
    }

    // --- 3. a position before the stream began is silence, not a wrap --------
    {
        const p = fresh();
        p.ring.fill(12345);
        p.w = 100;
        p.wLimit = 100;
        check("before the start: no sample is invented", p.sample(-1) === 0 && p.sample(-1000) === 0,
              "sample(-1) = " + p.sample(-1));
    }

    // --- 4. a message larger than the ring ----------------------------------
    {
        const p = fresh();
        const n = p.cap * 2 + 100;
        const a = new Float32Array(n);
        for (let i = 0; i < n; i++) a[i] = i;
        p.port.onmessage({ data: a });
        const raw = p.w - p.r;
        check("oversize: what is kept fits the ring", raw > 0 && raw <= p.cap,
              "kept " + raw + " of " + n);
        check("oversize: the newest is what is kept",
              p.ring[(p.w - 1) % p.cap] === n - 1 && p.oversized === 1,
              "last sample " + p.ring[(p.w - 1) % p.cap] + ", oversized " + p.oversized);
        inv(p, "oversize");
    }

    // --- 5. an edit near the start of the stream, with a computed reference --
    // The old branch of the crossfade plays the position the reader was about
    // to play, and the new branch the one after the skip. With a ramp, both are
    // known numbers, so the blend is compared to a reference rather than merely
    // checked for being finite.
    {
        const p = fresh();
        p.ring.fill(-999); // anything unwritten is out of range
        feedRamp(p, 0, p.target);
        process(p, 128); // primes at r = 4 and plays
        feedRamp(p, p.target, 6000);
        const rOld = p.r;
        const out = process(p, 128); // over the edit threshold: a crossfade
        check("edit near the start: it is blended", p.edits === 1 && p.xf > 0,
              "edits " + p.edits + ", xf " + p.xf);
        const rNew = rOld + p.skip;
        const worst = worstVs(out, (i) => blendRef(rOld, rNew, p.xfadeLen, i));
        check("edit near the start: both branches are this stream", worst < 1e-2,
              "worst error " + worst.toExponential(2));
        check("edit near the start: nothing comes from outside it",
              out.every((x) => x > -998), "lowest " + Math.min(...out).toFixed(1));
        inv(p, "edit near the start");
    }

    // --- 6. a write while the crossfade is running --------------------------
    // The writer appends; it must not move either branch. The remaining half of
    // the fade is still compared to the same reference.
    {
        const p = fresh();
        p.ring.fill(-999);
        feedRamp(p, 0, p.target);
        process(p, 128);
        feedRamp(p, p.target, 6000);
        const rOld = p.r;
        const out1 = process(p, 128);
        const rNew = rOld + p.skip;
        const xf = p.xf;
        feedRamp(p, p.target + 6000, 960); // a write in the middle of the fade
        const out2 = process(p, 128);
        let worst = 0;
        for (let j = 0; j < 112; j++) {
            const i = 128 + j;
            worst = Math.max(worst, Math.abs(out2[j] - blendRef(rOld, rNew, p.xfadeLen, i)));
        }
        check("write during the fade: the fade keeps the same two branches",
              p.xf === xf - 112 && worst < 1e-2,
              "worst error " + worst.toExponential(2));
        check("write during the fade: nothing comes from outside the stream",
              out1.every((x) => x > -998) && out2.every((x) => x > -998),
              "lowest " + Math.min(...out1, ...out2).toFixed(1));
        // After the fade the reader is on the new branch, advancing from there.
        let after = 0;
        for (let j = 112; j < 128; j++) {
            after = Math.max(after, Math.abs(out2[j] - (rNew + 240 + (j - 112))));
        }
        note("boundary", "out2[109..116] "
             + Array.from(out2.slice(109, 117)).map((x) => x.toFixed(1)).join(","));
        note("write during the fade", "after the fade: actual "
             + Array.from(out2.slice(112, 116)).map((x) => x.toFixed(1)).join(",")
             + " expected " + [0, 1, 2, 3].map((j) => (rNew + 240 + j).toFixed(1)).join(","));
        check("write during the fade: playback continues on the new branch", after < 1e-2,
              "worst error " + after.toExponential(2));
        inv(p, "write during the fade");
    }

    // --- 7. after several full turns of the ring ----------------------------
    // Value == position still, so the reference survives the wraps.
    {
        const p = fresh();
        feedRamp(p, 0, p.target);
        process(p, 128); // primes
        let next = p.target;
        // One block in, one quantum out: the level stays put, so no edit or
        // resync fires and the ring turns over cleanly, value == position.
        for (let i = 0; i < 1000; i++) {
            feedRamp(p, next, 128);
            next += 128;
            process(p, 128);
        }
        const turns = p.w / p.cap;
        p.avg = p.target;
        feedRamp(p, next, p.target + 6000);
        const rOld = p.r;
        const out = process(p, 128);
        check("after several turns: still blended", p.edits >= 1 && p.xf > 0,
              "edits " + p.edits + ", xf " + p.xf + ", w " + Math.round(p.w));
        const rNew = rOld + p.skip;
        const worst = worstVs(out, (i) => blendRef(rOld, rNew, p.xfadeLen, i));
        check("after several turns: both branches are the right samples", worst < 1e-2,
              "worst error " + worst.toExponential(2) + " after " + turns.toFixed(1) + " turns");
        inv(p, "after several turns");
        note("turns", turns.toFixed(2) + " ring lengths written");
    }

    // --- 8. no history behind the read position -----------------------------
    // A state the reader's own prime makes unreachable, driven by hand: at
    // r = 0 there is nothing behind the position to blend with, so the
    // transition is the fade through silence -- not a wrapped read.
    {
        const p = fresh();
        p.ring.fill(-999);
        feedRamp(p, 0, p.target + 6000);
        p.primed = true;
        p.avg = p.target;
        const out = process(p, 128);
        check("no history: it fades through silence, it does not crossfade",
              p.resyncs === 1 && p.xf === 0 && p.outFade > 0,
              "resyncs " + p.resyncs + ", xf " + p.xf + ", outFade " + p.outFade);
        let worst = 0;
        for (let i = 0; i < 128; i++) {
            const expected = i * ((p.fadeLen - 1 - i) / p.fadeLen);
            worst = Math.max(worst, Math.abs(out[i] - expected));
        }
        check("no history: the fade plays this stream and nothing else", worst < 1e-2,
              "worst error " + worst.toExponential(2));
        check("no history: nothing comes from outside it", out.every((x) => x > -998),
              "lowest " + Math.min(...out).toFixed(1));
        inv(p, "no history");
    }

    // --- 9. a reset, then a different generation ----------------------------
    // Generation A is +0.5, generation B is -0.5. The ring still holds A in the
    // slots the new stream has not reached; no A may appear once the reset's
    // transition is over.
    {
        const p = fresh();
        feedValue(p, 0.5, p.target);
        process(p, 128);
        process(p, 128);
        const playing = process(p, 128);
        check("reset: generation A is what plays", playing.every((x) => Math.abs(x - 0.5) < 1e-6),
              "first " + playing[0].toFixed(3));
        p.port.onmessage({ data: { reset: true } });
        process(p, 128);
        process(p, 128);
        check("reset: the ring is dropped and the reader waits",
              p.resets === 1 && p.w === 0 && p.r === 0 && p.primed === false,
              "resets " + p.resets + ", w " + p.w + ", r " + p.r + ", primed " + p.primed);
        feedValue(p, -0.5, p.target + 960);
        const post = [];
        for (let i = 0; i < 4; i++) post.push(process(p, 128));
        const flat = post.flatMap((q) => Array.from(q));
        note("reset post", post.map((q) => q[0].toFixed(2) + ".." + q[q.length - 1].toFixed(2)).join(" "));
        note("reset nan", post.map((q) => {
            const bad = [];
            for (let i = 0; i < q.length; i++) if (Number.isNaN(q[i])) bad.push(i);
            return bad.length ? bad[0] + "+" + bad.length : "none";
        }).join(" "));
        check("reset: no sample of the old generation comes back",
              flat.every((x) => x <= 0.001), "highest " + Math.max(...flat).toFixed(4));
        check("reset: the new stream is what plays",
              post[3].every((x) => Math.abs(x + 0.5) < 1e-6), "last " + post[3][0].toFixed(3));
        inv(p, "reset");
    }

    // --- 10. an underrun, then the stream comes back ------------------------
    {
        const p = fresh();
        feedRamp(p, 0, p.target);
        p.primed = false;
        let out = process(p, 128);
        check("resume: the prime waits for the target",
              p.primed === true && out.some((x) => x !== 0), "primed " + p.primed);
        for (let i = 0; i < 40; i++) out = process(p, 128);
        check("resume: running out is counted", p.underruns >= 1 && p.primed === false,
              "underruns " + p.underruns + ", primed " + p.primed);
        inv(p, "underrun");
        feedRamp(p, 10000, p.target + 960);
        out = process(p, 128);
        check("resume: it primes again and plays", p.primed === true && out.some((x) => x !== 0),
              "primed " + p.primed);
        inv(p, "resume");
    }

    // --- 11. a resync, and a write while it is fading -----------------------
    {
        const p = fresh();
        feedRamp(p, 0, p.target + 20000);
        p.primed = true;
        p.avg = p.target;
        const out = process(p, 128);
        check("resync: it fades through silence", p.resyncs === 1 && p.outFade > 0,
              "resyncs " + p.resyncs + ", outFade " + p.outFade);
        check("resync: the fade is finite", out.every(Number.isFinite), "finite");
        const before = { outFade: p.outFade, skip: p.pendingSkip };
        feedRamp(p, 300000, 960);
        check("write during a resync: the fade and its skip are untouched",
              p.outFade === before.outFade && p.pendingSkip === before.skip,
              "outFade " + p.outFade + "/" + before.outFade + ", skip "
              + p.pendingSkip + "/" + before.skip);
        inv(p, "write during a resync");
    }

    // --- 12. an overrun: sixty seconds pushed as fast as they can be --------
    {
        const p = fresh();
        for (let i = 0; i < 3000; i++) feedRamp(p, i * 960, 960);
        const raw = p.w - p.r;
        check("overrun: what is kept fits the ring", raw > 0 && raw <= p.cap,
              "kept " + Math.round(raw) + " source frames");
        check("overrun: the excess is counted as refused", p.overwritten > 0,
              p.overwritten + " source frames refused");
        inv(p, "overrun");
        p.primed = true;
        p.avg = p.target;
        process(p, 128);
        process(p, 128);
        process(p, 128);
        const after = p.w - p.r;
        check("overrun: the reader gives the backlog up", after >= 0 && after <= p.target + 3000,
              "w - r = " + Math.round(after) + ", target " + p.target);
        check("overrun: a resync is what did it", p.resyncs >= 1, "resyncs " + p.resyncs);
        inv(p, "overrun");
    }

    say(failures === 0 ? "PASS worklet tests done" : "FAIL worklet tests done (" + failures + " failures)");
    fetch("/result?done=1");
})();
