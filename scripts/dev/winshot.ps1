# Başlığı verilen pencereyi öne getirip yalnızca onun görüntüsünü alır.
# Başlıkta Türkçe karakter varsa parametre olarak ver: .\scripts\dev\winshot.ps1 -Title "Masaüstü Widget" -Name yonetim
# Kaydedilmemiş değişiklik varsa başlık "● Masaüstü Widget" olur; o da denenir.
param([Parameter(Mandatory)][string]$Title, [string]$Name = "win", [string]$Out = "$env:TEMP\masaustu-widget-test")
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System; using System.Runtime.InteropServices;
public static class WinShotW {
 [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindowW(string c, string t);
 [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
 [StructLayout(LayoutKind.Sequential)] public struct R { public int l, t, r, b; }
 [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr h, int a, out R r, int s);
}
'@
$h = [WinShotW]::FindWindowW([NullString]::Value, $Title)
if ($h -eq [IntPtr]::Zero) { $h = [WinShotW]::FindWindowW([NullString]::Value, ([char]0x25CF + " " + $Title)) }
if ($h -eq [IntPtr]::Zero) { Write-Error "pencere yok: $Title"; exit 1 }
[WinShotW]::SetForegroundWindow($h) | Out-Null; Start-Sleep -Milliseconds 800
$r = New-Object WinShotW+R; [WinShotW]::DwmGetWindowAttribute($h, 9, [ref]$r, 16) | Out-Null  # DWMWA_EXTENDED_FRAME_BOUNDS
$bmp = New-Object System.Drawing.Bitmap ($r.r - $r.l), ($r.b - $r.t)
[System.Drawing.Graphics]::FromImage($bmp).CopyFromScreen($r.l, $r.t, 0, 0, $bmp.Size)
New-Item -ItemType Directory -Force $Out | Out-Null
$path = Join-Path $Out "$Name.png"; $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png); $path
