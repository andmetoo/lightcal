# Windows only. Opens and closes Settings three times in a real Lightcal window by posting
# mouse messages to it (the cursor never moves) and prints the native moves and resizes each
# step caused. Needs a debug build (`cargo build -p lightcal`), which prints them when
# LIGHTCAL_TRACE_WINDOW is set, and no other Lightcal running. Uses a temporary data folder.
# Click positions are logical: the header gear and the Settings ×.
param(
    [string]$Exe = (Join-Path $PSScriptRoot "../target/debug/lightcal.exe"),
    [string]$Log = (Join-Path ([IO.Path]::GetTempPath()) "lightcal-view-switch.log"),
    [string]$DataDir = (Join-Path ([IO.Path]::GetTempPath()) ("lightcal-view-switch-" + [guid]::NewGuid()))
)
Add-Type -Namespace M -Name W -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
[DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
'@
$env:LIGHTCAL_TRACE_WINDOW = "1"
$env:LIGHTCAL_DATA_DIR = $DataDir
New-Item -ItemType Directory -Force $DataDir | Out-Null
$p = Start-Process $Exe -ArgumentList "--synthetic-calendars" -RedirectStandardError $Log -PassThru
try {
    $hwnd = [IntPtr]::Zero
    for ($i = 0; $i -lt 40 -and $hwnd -eq [IntPtr]::Zero; $i++) {
        Start-Sleep -Milliseconds 250
        $p.Refresh()
        $hwnd = $p.MainWindowHandle
    }
    if ($hwnd -eq [IntPtr]::Zero) { throw "no Lightcal window" }
    Start-Sleep -Milliseconds 800
    $scale = [M.W]::GetDpiForWindow($hwnd) / 96.0
    function Click([double]$x, [double]$y) {
        $l = [IntPtr](([int]($y * $scale) -shl 16) -bor [int]($x * $scale))
        [M.W]::PostMessage($hwnd, 0x0200, [IntPtr]0, $l) | Out-Null   # WM_MOUSEMOVE
        Start-Sleep -Milliseconds 60
        [M.W]::PostMessage($hwnd, 0x0201, [IntPtr]1, $l) | Out-Null   # WM_LBUTTONDOWN
        [M.W]::PostMessage($hwnd, 0x0202, [IntPtr]0, $l) | Out-Null   # WM_LBUTTONUP
    }
    function Lines { @(Get-Content $Log -ErrorAction SilentlyContinue | Where-Object { $_ -like "window*" }) }
    # Logical positions: the gear in the calendar header, the × in the Settings caption.
    $steps = @(@("open", 296, 22), @("close", 425, 16))
    for ($round = 1; $round -le 3; $round++) {
        foreach ($step in $steps) {
            $before = (Lines).Count
            Click $step[1] $step[2]
            Start-Sleep -Milliseconds 700
            $new = @(Lines | Select-Object -Skip $before)
            "{0} {1}: {2} native change(s)" -f $step[0], $round, $new.Count
            $new | ForEach-Object { "    $_" }
        }
    }
} finally {
    Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
}
