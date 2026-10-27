# **How far behind is the sound, and how well does the reader hold its backlog?**
#
#   .\tools\web-audio-latency\run.ps1
#
# Two pages, both served from a scratch directory so nothing is written inside
# the tree. `tests.html` drives the real worklet file with the audio-thread
# globals stubbed and asserts its internal invariants deterministically.
# `page.html` drives the real bridge and worklet through a headless browser's
# own AudioContext, at 48 kHz and again at 44.1 kHz. See `harness.js` for what
# each assertion means; this script only builds the scratch copy, serves it,
# and reports what the pages said.
#
# `$env:LONG = "1800"` adds a thirty-minute drift phase to the 48 kHz pass.
# `$env:JITTER = "1"` hands audio over on a 20-50 ms cadence instead of 30 ms.
#
# Needs python and a Chromium-family browser (Edge or Chrome).
$ErrorActionPreference = "Stop"

$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$root = (Resolve-Path (Join-Path $here "..\..")).Path
$port = if ($env:PORT) { $env:PORT } else { "8733" }
$build = Join-Path $env:TEMP "sdroxide-web-audio-latency-$port"
$raw = Join-Path $env:TEMP "sdroxide-web-audio-latency-$port.txt"
$long = if ($env:LONG) { $env:LONG } else { "0" }
$jitter = if ($env:JITTER) { $env:JITTER } else { "0" }
# Which context rates to run. By default both, with the long phase on 48 kHz
# only; `RATES="48000"` (or "44100") runs one of them, and then a LONG applies
# to it -- which is how a thirty-minute run is taken per rate.
$rates = if ($env:RATES) { $env:RATES.Split(" ") } else { @("48000", "44100") }
$perRateLong = [bool]$env:RATES

$browser = @(
    "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
    "C:\Program Files\Microsoft\Edge\Application\msedge.exe",
    "C:\Program Files\Google\Chrome\Application\chrome.exe",
    "C:\Program Files (x86)\Google\Chrome\Application\chrome.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $browser) {
    Write-Error "no Chromium-family browser found (Edge or Chrome)"
    exit 2
}

Remove-Item $build -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item $raw -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $build | Out-Null
Copy-Item (Join-Path $root "crates\sdroxide-web\assets\audio_bridge.js") $build
Copy-Item (Join-Path $root "crates\sdroxide-web\assets\pcm_worklet.js") $build
Copy-Item (Join-Path $here "page.html") $build
Copy-Item (Join-Path $here "harness.js") $build
Copy-Item (Join-Path $here "tap_worklet.js") $build
Copy-Item (Join-Path $here "tests.html") $build
Copy-Item (Join-Path $here "worklet_tests.js") $build
Copy-Item (Join-Path $root "tools\web-audio-recovery\serve.py") $build

$server = Start-Process python -ArgumentList "serve.py", $port, $raw `
    -WorkingDirectory $build -PassThru -WindowStyle Hidden
Start-Sleep 2

# One pass, one browser of its own: a browser left over from an earlier run
# holds the same profile directory open, and a second launch against it is
# handed to the process that is already there instead of loading the page.
function Run-Pass([string]$page, [string]$query, [int]$wait) {
    $profile = Join-Path $build ("profile-" + [guid]::NewGuid().ToString("N"))
    $client = Start-Process $browser -ArgumentList @(
        "--headless=new", "--disable-gpu", "--no-first-run", "--no-default-browser-check",
        # Edge otherwise opens its own sign-in/sync panel instead of the page.
        "--disable-sync", "--disable-extensions",
        "--autoplay-policy=no-user-gesture-required",
        "--user-data-dir=$profile",
        "http://127.0.0.1:$port/$page$query"
    ) -PassThru
    Start-Sleep $wait
    taskkill /PID $client.Id /T /F 2>&1 | Out-Null
}

try {
    Run-Pass "tests.html" "" 8
    foreach ($r in $rates) {
        $L = if ($perRateLong -or $r -eq "48000") { $long } else { "0" }
        $wait = if ([int]$L -gt 0) { 120 + [int]$L } elseif ($r -eq "48000") { 90 } else { 70 }
        Run-Pass "page.html" "?rate=$r&long=$L&jitter=$jitter" $wait
    }
} finally {
    Stop-Process -Id $server.Id -Force -ErrorAction SilentlyContinue
}

echo "--- the pages said ---"
if (Test-Path $raw) { Get-Content $raw } else { echo "(nothing)" }

$lines = if (Test-Path $raw) { Get-Content $raw } else { @() }
$failed = @($lines | Where-Object { $_ -like "line=FAIL*" })
$tests = @($lines | Where-Object { $_ -like "line=PASS worklet tests done" })
$done = @($lines | Where-Object { $_ -like "line=PASS suite done" })
if ($failed.Count -gt 0) {
    echo "--- FAIL ($($failed.Count)) ---"
    exit 1
}
if ($tests.Count -lt 1 -or $done.Count -lt $rates.Count) {
    echo "--- FAIL: a pass did not run to the end (tests $($tests.Count), suites $($done.Count)/$($rates.Count)) ---"
    exit 1
}
echo "--- PASS ---"
exit 0
