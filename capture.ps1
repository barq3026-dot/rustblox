param(
    [string]$Out = "$env:LOCALAPPDATA\Temp\opencode\menu.png",
    [string[]]$GameArgs = @(),
    [int]$DelayMs = 3000,
    [switch]$Click
)

Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class W2 {
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, int dx, int dy, uint d, UIntPtr e);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
"@

$exe = Join-Path $PSScriptRoot "target\release\rustblox.exe"
if ($GameArgs -and $GameArgs.Count -gt 0) {
    $p = Start-Process -FilePath $exe -ArgumentList $GameArgs -PassThru
} else {
    $p = Start-Process -FilePath $exe -PassThru
}
Start-Sleep -Milliseconds $DelayMs
$p.Refresh()
$h = $p.MainWindowHandle
[W2]::SetForegroundWindow($h) | Out-Null
Start-Sleep -Milliseconds 600

if ($Click) {
    $r0 = New-Object W2+RECT
    [W2]::GetWindowRect($h, [ref]$r0) | Out-Null
    $cx = [int](($r0.Left + $r0.Right) / 2)
    $cy = [int](($r0.Top + $r0.Bottom) / 2)
    [W2]::SetCursorPos($cx, $cy) | Out-Null
    Start-Sleep -Milliseconds 150
    [W2]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 60
    [W2]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 900
}

$r = New-Object W2+RECT
[W2]::GetWindowRect($h, [ref]$r) | Out-Null
$w = $r.Right - $r.Left
$hh = $r.Bottom - $r.Top
$bmp = New-Object System.Drawing.Bitmap $w, $hh
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($r.Left, $r.Top, 0, 0, $bmp.Size)
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose()
$bmp.Dispose()
"rect: $($r.Left),$($r.Top) ${w}x${hh}"
"saved: $Out"
if (-not $p.HasExited) { $p.Kill() }
