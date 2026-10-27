#!/bin/sh
# **How far behind is the sound?**
#
#   ./tools/web-audio-latency/run.sh
#
# The page is driven in a headless browser against the *real* bridge scripts,
# served from a scratch directory so nothing is written inside the tree. See
# `harness.js` for what it does and what it asserts; this script only builds the
# scratch copy, serves it, runs the suite at 48 kHz and again at 44.1 kHz, and
# reports what the page said.
#
# `LONG=1800 ./tools/web-audio-latency/run.sh` adds a thirty-minute drift phase
# to the 48 kHz pass.
#
# Needs python3 and a Chromium-family browser (chrome, chromium or msedge).
set -eu

here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
build="${TMPDIR:-/tmp}/sdroxide-web-audio-latency"
raw="${TMPDIR:-/tmp}/sdroxide-web-audio-latency.txt"
port="${PORT:-8733}"
long="${LONG:-0}"

browser=""
for c in google-chrome google-chrome-stable chromium chromium-browser msedge microsoft-edge; do
    if command -v "$c" >/dev/null 2>&1; then browser="$c"; break; fi
done
if [ -z "$browser" ]; then
    echo "no Chromium-family browser found (chrome, chromium or msedge)" >&2
    exit 2
fi

rm -rf "$build" "$raw"
mkdir -p "$build"
cp "$root/crates/sdroxide-web/assets/audio_bridge.js" "$build/"
cp "$root/crates/sdroxide-web/assets/pcm_worklet.js" "$build/"
cp "$here/page.html" "$here/harness.js" "$here/tap_worklet.js" "$build/"
cp "$root/tools/web-audio-recovery/serve.py" "$build/"

python3 "$build/serve.py" "$port" "$raw" &
server=$!
trap 'kill $server 2>/dev/null || true' EXIT
sleep 2

# One pass, one browser of its own: --disable-sync, or Edge opens its own
# sign-in panel instead of the page and nothing runs.
run_pass() {
    query="$1"
    wait="$2"
    "$browser" \
        --headless=new --disable-gpu --no-first-run --no-default-browser-check \
        --disable-sync --disable-extensions \
        --autoplay-policy=no-user-gesture-required \
        --user-data-dir="$build/profile.$$.$query" \
        "http://127.0.0.1:$port/page.html$query" >/dev/null 2>&1 &
    client=$!
    sleep "$wait"
    kill $client 2>/dev/null || true
    pkill -f "user-data-dir=$build/profile" 2>/dev/null || true
}

run_pass "?rate=48000&long=$long" $((52 + long))
run_pass "?rate=44100" 40

kill $server 2>/dev/null || true

echo "--- the page said ---"
cat "$raw" 2>/dev/null || echo "(nothing)"

if grep -q "^line=FAIL" "$raw" 2>/dev/null; then
    echo "--- FAIL ---" >&2
    exit 1
fi
if [ "$(grep -c '^line=PASS suite done' "$raw" 2>/dev/null || true)" -lt 2 ]; then
    echo "--- FAIL: a pass did not run to the end ---" >&2
    exit 1
fi
echo "--- PASS ---"
