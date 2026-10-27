#!/bin/sh
# **How far behind is the sound, and how well does the reader hold its backlog?**
#
#   ./tools/web-audio-latency/run.sh
#
# Two pages, both served from a scratch directory so nothing is written inside
# the tree. `tests.html` drives the real worklet file with the audio-thread
# globals stubbed and asserts its internal invariants deterministically.
# `page.html` drives the real bridge and worklet through a headless browser's
# own AudioContext, at 48 kHz and again at 44.1 kHz. See `harness.js` for what
# each assertion means; this script only builds the scratch copy, serves it,
# and reports what the pages said.
#
# `LONG=1800 ./tools/web-audio-latency/run.sh` adds a thirty-minute drift phase
# to the 48 kHz pass. `JITTER=1` hands audio over on a 20-50 ms cadence.
#
# Needs python3 and a Chromium-family browser (chrome, chromium or msedge).
set -eu

here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
build="${TMPDIR:-/tmp}/sdroxide-web-audio-latency-$port"
raw="${TMPDIR:-/tmp}/sdroxide-web-audio-latency-$port.txt"
port="${PORT:-8733}"
long="${LONG:-0}"
jitter="${JITTER:-0}"
# Which context rates to run. By default both, with the long phase on 48 kHz
# only; `RATES="48000"` (or "44100") runs one of them, and then a LONG applies
# to it -- which is how a thirty-minute run is taken per rate.
rates="${RATES:-48000 44100}"

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
cp "$here/tests.html" "$here/worklet_tests.js" "$build/"
cp "$root/tools/web-audio-recovery/serve.py" "$build/"

python3 "$build/serve.py" "$port" "$raw" &
server=$!
trap 'kill $server 2>/dev/null || true' EXIT
sleep 2

# One pass, one browser of its own: --disable-sync, or Edge opens its own
# sign-in panel instead of the page and nothing runs.
run_pass() {
    page="$1"
    query="$2"
    wait="$3"
    "$browser" \
        --headless=new --disable-gpu --no-first-run --no-default-browser-check \
        --disable-sync --disable-extensions \
        --autoplay-policy=no-user-gesture-required \
        --user-data-dir="$build/profile.$$.$page$query" \
        "http://127.0.0.1:$port/$page$query" >/dev/null 2>&1 &
    client=$!
    sleep "$wait"
    kill $client 2>/dev/null || true
    pkill -f "user-data-dir=$build/profile" 2>/dev/null || true
}

run_pass "tests.html" "" 8
for r in $rates; do
    if [ -n "${RATES:-}" ] || [ "$r" = "48000" ]; then L="$long"; else L=0; fi
    if [ "$L" -gt 0 ]; then w=$((120 + L)); elif [ "$r" = "48000" ]; then w=90; else w=70; fi
    run_pass "page.html" "?rate=$r&long=$L&jitter=$jitter" "$w"
done

kill $server 2>/dev/null || true

echo "--- the pages said ---"
cat "$raw" 2>/dev/null || echo "(nothing)"

if grep -q "^line=FAIL" "$raw" 2>/dev/null; then
    echo "--- FAIL ---" >&2
    exit 1
fi
if [ "$(grep -c '^line=PASS worklet tests done' "$raw" 2>/dev/null || true)" -lt 1 ]; then
    echo "--- FAIL: the worklet tests did not run to the end ---" >&2
    exit 1
fi
n_rates=$(set -- $rates; echo $#)
if [ "$(grep -c '^line=PASS suite done' "$raw" 2>/dev/null || true)" -lt "$n_rates" ]; then
    echo "--- FAIL: a suite did not run to the end ---" >&2
    exit 1
fi
echo "--- PASS ---"
