// An independent witness of what the playback worklet actually rendered.
//
// It is connected to the player's output, beside the destination, and ends in
// a muted gain so the graph pulls it without adding a second audible copy.
// Nothing here is part of the client: this is the harness measuring the code
// under test rather than asking it about itself.
//
// It reports, per render quantum group:
//   frames          input frames seen since it was connected (== rendered)
//   energy          sum of the squares of every sample, for a band check
//   splices         sample-to-sample discontinuities (a proxy for a click)
//   maxStep         the largest such discontinuity
//   longestZeroRun  the longest run of exact zeros (priming, underrun)
//   crossings       zero crossings, for the tone's frequency
//
// The splice detector assumes the harness's own tone: a 1 kHz sine at 0.3 has
// a second difference of about 0.005 at 48 kHz, and an unfaded splice or a
// block dropped out of the middle of it is an order of magnitude larger. It is
// a proxy for a click, not a measurement of one, and it is disarmed by the
// harness while a swept tone changes frequency -- a change of slope is not a
// splice.
class PcmTap extends AudioWorkletProcessor {
    constructor() {
        super();
        this.frames = 0;
        this.energy = 0;
        this.splices = 0;
        this.maxStep = 0;
        this.peak = 0; // the largest second difference since the last reset
        this.zeroRun = 0;
        this.longestZeroRun = 0;
        this.crossings = 0;
        this.prev1 = 0;
        this.prev2 = 0;
        this.warm = 0;
        this.report = 0;
        this.checkSplices = true;
        this.port.onmessage = (ev) => {
            const m = ev.data;
            if (!m) return;
            if (typeof m.splices === "boolean") this.checkSplices = m.splices;
            if (m.resetPeak) this.peak = 0;
        };
    }

    counters() {
        return {
            frames: this.frames,
            energy: this.energy,
            splices: this.splices,
            maxStep: +this.maxStep.toFixed(3),
            peak: +this.peak.toFixed(3),
            longestZeroRun: this.longestZeroRun,
            crossings: this.crossings,
        };
    }

    process(inputs, _outputs) {
        const ch = inputs[0] && inputs[0][0];
        if (!ch) {
            this.port.postMessage(this.counters());
            return true;
        }
        for (let i = 0; i < ch.length; i++) {
            const x = ch[i];
            this.frames++;
            this.energy += x * x;
            if (this.warm >= 2) {
                const d2 = Math.abs(x - 2 * this.prev1 + this.prev2);
                if (d2 > this.peak) this.peak = d2;
                if (this.checkSplices && d2 > 0.05) {
                    this.splices++;
                    if (d2 > this.maxStep) this.maxStep = d2;
                }
            } else {
                this.warm++;
            }
            if ((this.prev1 <= 0 && x > 0) || (this.prev1 >= 0 && x < 0)) this.crossings++;
            if (x === 0) {
                this.zeroRun++;
                if (this.zeroRun > this.longestZeroRun) this.longestZeroRun = this.zeroRun;
            } else {
                this.zeroRun = 0;
            }
            this.prev2 = this.prev1;
            this.prev1 = x;
        }
        if (++this.report >= 8) {
            this.report = 0;
            this.port.postMessage(this.counters());
        }
        return true;
    }
}

registerProcessor("pcm-tap", PcmTap);
