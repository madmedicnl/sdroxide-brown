# **How far behind is the sound?**
#
#   .\tools\web-audio-latency\run.ps1
#
# The page is driven in a headless browser against the *real* bridge scripts
# (`crates/sdroxide-web/assets/audio_bridge.js` and `pcm_worklet.js`), served
# from a scratch directory so nothing is written inside the tree. See
# `harness.js` for what it does and what it asserts; this script only builds the
# scratch copy, serves it, runs the suite at 48 kHz and again at 44.1 kHz, and
# reports what the page said.
#
# `$env:LONG = "1800"` adds a thirty-minute drift phase to the 48 kHz pass.
#
# Needs python and a Chromium-family browser (Edge or Chrome).
$ErrorActionPreference = "Stop"

$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$root = (Resolve-Path (Join-Path $here "..\..")).Path
$build = Join-Path $env:TEMP "sdroxide-web-audio-latency"
$raw = Join-Path $env:TEMP "sdroxide-web-audio-latency.txt"
$port = if ($env:PORT) { $env:PORT } else { "8733" }
$long = if ($env:LONG) { $env:LONG } else { "0" }

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
Copy-Item (Join-Path $root "tools\web-audio-recovery\serve.py") $build

$server = Start-Process python -ArgumentList "serve.py", $port, $raw `
    -WorkingDirectory $build -PassThru -WindowStyle Hidden
Start-Sleep 2

# One pass, one browser of its own: a browser left over from an earlier run
# holds the same profile directory open, and a second launch against it is
# handed to the process that is already there instead of loading the page.
function Run-Pass([string]$query, [int]$wait) {
    $profile = Join-Path $build ("profile-" + [guid]::NewGuid().ToString("N"))
    $client = Start-Process $browser -ArgumentList @(
        "--headless=new", "--disable-gpu", "--no-first-run", "--no-default-browser-check",
        # Edge otherwise opens its own sign-in/sync panel instead of the page.
        "--disable-sync", "--disable-extensions",
        "--autoplay-policy=no-user-gesture-required",
        "--user-data-dir=$profile",
        "http://127.0.0.1:$port/page.html$query"
    ) -PassThru
    Start-Sleep $wait
    taskkill /PID $client.Id /T /F 2>&1 | Out-Null
}

try {
    Run-Pass "?rate=48000&long=$long" (52 + [int]$long)
    Run-Pass "?rate=44100" 40
} finally {
    Stop-Process -Id $server.Id -Force -ErrorAction SilentlyContinue
}

echo "--- the page said ---"
if (Test-Path $raw) { Get-Content $raw } else { echo "(nothing)" }

$lines = if (Test-Path $raw) { Get-Content $raw } else { @() }
$failed = @($lines | Where-Object { $_ -like "line=FAIL*" })
$done = @($lines | Where-Object { $_ -like "line=PASS suite done" })
if ($failed.Count -gt 0) {
    echo "--- FAIL ($($failed.Count)) ---"
    exit 1
}
if ($done.Count -lt 2) {
    echo "--- FAIL: a pass did not run to the end ($($done.Count)/2) ---"
    exit 1
}
echo "--- PASS ---"
exit 0
