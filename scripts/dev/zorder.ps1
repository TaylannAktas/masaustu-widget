# Gömmenin doğru olup olmadığını gösterir: Progman'ın çocukları üstten alta ve bir noktaya tıklanınca kimin alacağı.
# Beklenen (Windows 11 24H2+): SHELLDLL_DefView > Tauri Window (host) > WorkerW ; isabet: SHELLDLL_DefView (tıklama ikonlara gider).
# Ör: .\scripts\dev\zorder.ps1 -X 300 -Y 200
param([int]$X = 300, [int]$Y = 200)
Add-Type @'
using System; using System.Text; using System.Runtime.InteropServices;
public static class ZW {
 [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindowExW(IntPtr p, IntPtr a, string c, string t);
 [DllImport("user32.dll")] public static extern IntPtr GetWindow(IntPtr h, uint c);
 [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, StringBuilder s, int n);
 [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, StringBuilder s, int n);
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
 [StructLayout(LayoutKind.Sequential)] public struct P { public int x, y; }
 [DllImport("user32.dll")] public static extern IntPtr RealChildWindowFromPoint(IntPtr p, P pt);
 public static string Name(IntPtr h) { var c = new StringBuilder(128); var t = new StringBuilder(128); GetClassNameW(h, c, 128); GetWindowTextW(h, t, 128); return c + (t.Length > 0 ? " '" + t + "'" : "") + (IsWindowVisible(h) ? "" : " (gizli)"); }
}
'@
$pm = [ZW]::FindWindowExW([IntPtr]::Zero, [IntPtr]::Zero, "Progman", [NullString]::Value)
"Progman çocukları (üstten alta):"
$h = [ZW]::GetWindow($pm, 5)
while ($h -ne [IntPtr]::Zero) { "  " + [ZW]::Name($h); $h = [ZW]::GetWindow($h, 2) }
$pt = New-Object ZW+P; $pt.x = $X; $pt.y = $Y
"($X,$Y) noktasına tıklama: " + [ZW]::Name([ZW]::RealChildWindowFromPoint($pm, $pt))
