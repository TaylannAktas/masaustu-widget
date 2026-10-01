# Uygulamanın (masaustu-widget.exe + tüm WebView2 alt süreçleri) CPU ve bellek kullanımını ölçer.
# Bellek = aktif özel çalışma kümesi (Görev Yöneticisi'nin "Bellek" sütunu). PrivateMemorySize Chromium'da ~2 kat şişkin gösterir, kullanma.
# Ör: .\scripts\dev\olc.ps1 -Sec 15 -Label "7 widget" -Detail
# Not: ölçüm sırasında süreç kapanırsa CPU negatif çıkabilir; o ölçümü yok say.
param([int]$Sec = 10, [string]$Label = "", [switch]$Detail)
$all = Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId, Name, CommandLine
$root = ($all | Where-Object Name -eq "masaustu-widget.exe" | Select-Object -First 1).ProcessId
if (-not $root) { Write-Error "masaustu-widget.exe çalışmıyor"; exit 1 }
$ids = @($root); $grew = $true
while ($grew) { $grew = $false; foreach ($p in $all) { if ($ids -contains $p.ParentProcessId -and -not ($ids -contains $p.ProcessId)) { $ids += $p.ProcessId; $grew = $true } } }

$t0 = (Get-Process -Id $ids -ErrorAction SilentlyContinue | Measure-Object CPU -Sum).Sum
Start-Sleep $Sec
$t1 = (Get-Process -Id $ids -ErrorAction SilentlyContinue | Measure-Object CPU -Sum).Sum
$pct = ($t1 - $t0) / $Sec / [Environment]::ProcessorCount * 100

$perf = Get-CimInstance Win32_PerfFormattedData_PerfProc_Process | Where-Object { $ids -contains $_.IDProcess }
$mem = ($perf | Measure-Object WorkingSetPrivate -Sum).Sum / 1MB
"{0}  CPU {1:N2}% (tüm çekirdekler)  bellek {2:N1} MB  süreç {3}" -f $Label, $pct, $mem, $ids.Count

if ($Detail) {
  foreach ($p in ($all | Where-Object { $ids -contains $_.ProcessId })) {
    $type = if ($p.CommandLine -match "--type=([\w-]+)") { $Matches[1] } elseif ($p.Name -eq "msedgewebview2.exe") { "browser" } else { "uygulama" }
    if ($p.CommandLine -match "--utility-sub-type=([\w.]+)") { $type += " (" + $Matches[1].Split('.')[-1] + ")" }
    "  {0,-34} {1,7:N1} MB" -f $type, (($perf | Where-Object IDProcess -eq $p.ProcessId).WorkingSetPrivate / 1MB)
  }
}
