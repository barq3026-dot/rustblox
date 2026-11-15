param(
    [string[]]$GameArgs = @(),
    [string]$Tag = "grab"
)

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class W {
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, int dx, int dy, uint d, UIntPtr e);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, System.Text.StringBuilder s, int n);
}
"@

function Focus($h) {
    [W]::SetForegroundWindow($h) | Out-Null
    Start-Sleep -Milliseconds 250
    # A click is the most reliable way to hand focus to the window on Windows.
    [W]::SetCursorPos(640, 360) | Out-Null
    Start-Sleep -Milliseconds 120
    [W]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 60
    [W]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 400
}

$exe = Join-Path $PSScriptRoot "target\release\rustblox.exe"
$log = "$env:LOCALAPPDATA\Temp\opencode\input_$Tag.txt"
Remove-Item $log -ErrorAction SilentlyContinue

$allArgs = @("--input-log", $log) + $GameArgs
$p = Start-Process -FilePath $exe -ArgumentList $allArgs -PassThru
Start-Sleep -Seconds 3

$p.Refresh()
$hwnd = $p.MainWindowHandle
"hwnd: $hwnd"
if ($hwnd -ne 0) { Focus $hwnd }

$fg = [W]::GetForegroundWindow()
$sb = New-Object System.Text.StringBuilder 256
[W]::GetWindowText($fg, $sb, 256) | Out-Null
"foreground: $($sb.ToString())"

# Move the mouse to the right, then down.
for ($i = 0; $i -lt 20; $i++) {
    [W]::mouse_event(0x0001, 20, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 40
}
for ($i = 0; $i -lt 10; $i++) {
    [W]::mouse_event(0x0001, 0, 15, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 40
}

Start-Sleep -Milliseconds 400
if (-not $p.HasExited) { $p.Kill() }
Start-Sleep -Milliseconds 300

$lines = @(Get-Content $log -ErrorAction SilentlyContinue)
$playing = @($lines | Where-Object { $_ -match "playing=true" })
$yaws = @()
$pitches = @()
$maxdx = 0.0
foreach ($l in $lines) {
    if ($l -match "d=\((-?[0-9.]+),(-?[0-9.]+)\)") {
        $dx = [math]::Abs([double]$matches[1])
        if ($dx -gt $maxdx) { $maxdx = $dx }
    }
    if ($l -match "yaw=(-?[0-9.]+) pitch=(-?[0-9.]+)") {
        $yaws += [double]$matches[1]
        $pitches += [double]$matches[2]
    }
}
"log lines: $($lines.Count)  playing: $($playing.Count)  max |dx|: $maxdx"
if ($yaws.Count -gt 0) {
    "yaw first/last: $($yaws[0]) / $($yaws[$yaws.Count-1])"
    "pitch first/last: $($pitches[0]) / $($pitches[$pitches.Count-1])"
}
"last lines:"
$lines | Select-Object -Last 2
