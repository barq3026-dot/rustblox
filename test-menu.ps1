param(
    [string]$Dir = "$env:LOCALAPPDATA\Temp\opencode"
)

Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public class MW {
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte sc, uint f, UIntPtr e);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, int dx, int dy, uint d, UIntPtr e);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
"@

$exe = Join-Path $PSScriptRoot "target\release\rustblox.exe"
$p = Start-Process -FilePath $exe -PassThru
Start-Sleep -Seconds 3
$p.Refresh()
$h = $p.MainWindowHandle
[MW]::SetForegroundWindow($h) | Out-Null
Start-Sleep -Milliseconds 500

function Key([byte]$vk) {
    [MW]::keybd_event($vk, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 80
    [MW]::keybd_event($vk, 0, 2, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 350
}

function Shot([string]$name) {
    $r = New-Object MW+RECT
    [MW]::GetWindowRect($h, [ref]$r) | Out-Null
    $w = $r.Right - $r.Left
    $hh = $r.Bottom - $r.Top
    $bmp = New-Object System.Drawing.Bitmap $w, $hh
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($r.Left, $r.Top, 0, 0, $bmp.Size)
    $out = Join-Path $Dir "$name.png"
    $bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose()
    $bmp.Dispose()
    "shot: $out"
}

Shot "m1-main"

# Main menu -> Settings (second item)
Key 0x28   # Down
Key 0x0D   # Enter
Shot "m2-settings"

# Bump sensitivity and FOV
Key 0x27
Key 0x27
Key 0x28
Key 0x27
Shot "m3-settings-changed"

# Back to main menu
Key 0x1B
Shot "m4-back"

# Play
Key 0x0D
Start-Sleep -Milliseconds 1200
Shot "m5-play"

# Pause
Key 0x1B
Shot "m6-pause"

if (-not $p.HasExited) { $p.Kill() }
