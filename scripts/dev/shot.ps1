# Masaüstünü kısa süre gösterip (Win+D gibi) ana ekranın görüntüsünü alır, sonra pencereleri geri getirir.
# -HideIcons: masaüstü simgelerini çekim süresince gizler (README görselleri için; simge etiketleri kişisel olabilir).
# Ör: .\scripts\dev\shot.ps1 -Name saat ; çıktı yolunu yazar, görüntüyü okuyup kontrol et.
param([string]$Name = "shot", [string]$Out = "$env:TEMP\masaustu-widget-test", [switch]$HideIcons, [switch]$Half)
Add-Type -AssemblyName System.Drawing, System.Windows.Forms
Add-Type @'
using System; using System.Runtime.InteropServices;
public static class ShotW {
 [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindowExW(IntPtr p, IntPtr a, string c, string t);
 [DllImport("user32.dll")] public static extern IntPtr SendMessageW(IntPtr h, uint m, IntPtr w, IntPtr l);
}
'@
New-Item -ItemType Directory -Force $Out | Out-Null
$b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$sh = New-Object -ComObject Shell.Application
$sh.ToggleDesktop(); Start-Sleep -Milliseconds 1500
$dv = [IntPtr]::Zero
if ($HideIcons) {
  $pm = [ShotW]::FindWindowExW([IntPtr]::Zero, [IntPtr]::Zero, "Progman", [NullString]::Value)
  $dv = [ShotW]::FindWindowExW($pm, [IntPtr]::Zero, "SHELLDLL_DefView", [NullString]::Value)
  [ShotW]::SendMessageW($dv, 0x0111, [IntPtr]0x7402, [IntPtr]::Zero) | Out-Null; Start-Sleep -Milliseconds 1200  # simgeleri aç/kapa
}
$bmp = New-Object System.Drawing.Bitmap $b.Width, $b.Height
[System.Drawing.Graphics]::FromImage($bmp).CopyFromScreen($b.X, $b.Y, 0, 0, $bmp.Size)
if ($HideIcons) { [ShotW]::SendMessageW($dv, 0x0111, [IntPtr]0x7402, [IntPtr]::Zero) | Out-Null; Start-Sleep -Milliseconds 500 }
$sh.ToggleDesktop()
if ($Half) { $bmp = New-Object System.Drawing.Bitmap $bmp, ($b.Width / 2), ($b.Height / 2) }
$path = Join-Path $Out "$Name.png"
$bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
$path
