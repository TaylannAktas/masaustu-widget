# Uygulama pencerelerini UI Automation ile sürmek için yardımcılar. Kullanım: . .\scripts\dev\uia.ps1
# Ör: $w = Win "Masaüstü Widget"; ClickBtn $w "+ Yeni*"; SetVal ($w.FindFirst("Descendants", (Cond AutomationIdProperty "x"))) 300
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
$script:A = [System.Windows.Automation.AutomationElement]
function Cond($prop, $val) { New-Object System.Windows.Automation.PropertyCondition($script:A::$prop, $val) }
function Win($title) {
  for ($i = 0; $i -lt 20; $i++) {
    foreach ($t in @($title, ([char]0x25CF + " " + $title))) {
      $w = $script:A::RootElement.FindFirst("Children", (Cond NameProperty $t))
      if ($w) { return $w }
    }
    Start-Sleep -Milliseconds 300
  }
}
function ByName($root, $name) {
  for ($i = 0; $i -lt 20; $i++) { $e = $root.FindFirst("Descendants", (Cond NameProperty $name)); if ($e) { return $e }; Start-Sleep -Milliseconds 200 }
  throw "bulunamadı: $name"
}
function Click($root, $name) { (ByName $root $name).GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke(); Start-Sleep -Milliseconds 300 }
function Edits($root) { $root.FindAll("Descendants", (Cond ControlTypeProperty ([System.Windows.Automation.ControlType]::Edit))) }
function SetVal($el, $v) { $el.SetFocus(); $el.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).SetValue([string]$v); Start-Sleep -Milliseconds 200 }
function Status($root) {
  $f = $root.FindAll("Descendants", [System.Windows.Automation.Condition]::TrueCondition) | Where-Object { $_.Current.Name -match "Kaydedil|widget|aktar|eklendi|Hata" -and $_.Current.ControlType.ProgrammaticName -eq "ControlType.Text" }
  ($f | ForEach-Object { $_.Current.Name }) -join " | "
}
function ClickBtn($root, $like) {
  for ($i = 0; $i -lt 15; $i++) {
    $b = $root.FindAll("Descendants", (Cond ControlTypeProperty ([System.Windows.Automation.ControlType]::Button))) | Where-Object { $_.Current.Name -like $like } | Select-Object -First 1
    if ($b) { PressEl $b; return $true }
    Start-Sleep -Milliseconds 200
  }
  return $false
}

Add-Type -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetCursorPos(int x,int y); [DllImport("user32.dll")] public static extern void mouse_event(int f,int x,int y,int d,int e);' -Name Mou -Namespace UiaH -ErrorAction SilentlyContinue
function PressEl($b) {
  $pats = $b.GetSupportedPatterns() | ForEach-Object { $_.ProgrammaticName }
  if ($pats -match '^InvokePattern') { $b.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke() }
  elseif ($pats -match '^ExpandCollapse') { $p = $b.GetCurrentPattern([System.Windows.Automation.ExpandCollapsePattern]::Pattern); if ($p.Current.ExpandCollapseState -eq 'Collapsed') { $p.Expand() } else { $p.Collapse() } }
  else { $r = $b.Current.BoundingRectangle; [UiaH.Mou]::SetCursorPos([int]($r.X + $r.Width/2), [int]($r.Y + $r.Height/2)); [UiaH.Mou]::mouse_event(2,0,0,0,0); [UiaH.Mou]::mouse_event(4,0,0,0,0) }
  Start-Sleep -Milliseconds 350
}

# Fareyle gerçek sürükleme (düzenleme modu testi). Koordinatlar fiziksel piksel.
function Drag([int]$x1, [int]$y1, [int]$x2, [int]$y2, [int]$steps = 20) {
  [UiaH.Mou]::SetCursorPos($x1, $y1); Start-Sleep -Milliseconds 300; [UiaH.Mou]::mouse_event(2,0,0,0,0)
  for ($i = 1; $i -le $steps; $i++) { [UiaH.Mou]::SetCursorPos($x1 + ($x2-$x1)*$i/$steps, $y1 + ($y2-$y1)*$i/$steps); Start-Sleep -Milliseconds 25 }
  Start-Sleep -Milliseconds 100; [UiaH.Mou]::mouse_event(4,0,0,0,0)
}
