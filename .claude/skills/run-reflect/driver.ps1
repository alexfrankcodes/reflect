<#
.SYNOPSIS
  Drive Reflect: launch it, work its tray menu, screenshot its windows.

.DESCRIPTION
  Reflect is a tray-only Tauri app. It opens no windows at startup, so the
  tray icon is the only way in and there is nothing for a test runner to
  attach to. This script is that way in.

  Everything here was arrived at by fighting the Windows shell; read the
  Gotchas section of SKILL.md before changing any of it. In particular the
  tray menu is a native '#32768' popup that does not appear in the UI
  Automation tree at all, so its items are reached by keyboard, not by name.

  Keep this file ASCII-only. Windows PowerShell 5.1 reads a .ps1 with no
  byte-order mark as ANSI, so a stray em dash becomes mojibake and the
  script fails to parse.

  Run one verb per invocation:

    launch            start the app and wait for its tray icon
    menu   -Item x    open the tray menu and choose an item
    windows           list Reflect's open windows and their rectangles
    shot   -Out p     screenshot the desktop, or one window with -Window
    type   -Text s    type into whatever window has focus
    pick   -Nth n     click the nth day in the browse window's left column
    close             Alt+F4 the focused window (how the notes window saves)
    quit              tray Quit Reflect, then confirm the process is gone
    seed   -Date d    write an entry for a day, -Text for its contents
    entries           list what is in the entries folder
    reset             delete every entry (leaves the folder)

.EXAMPLE
  .\driver.ps1 launch
  .\driver.ps1 menu -Item browse
  .\driver.ps1 shot -Window Entries -Out .\browse.png
#>
[CmdletBinding()]
param(
  [Parameter(Mandatory, Position = 0)]
  [ValidateSet('launch', 'menu', 'windows', 'shot', 'pick', 'type', 'close', 'quit', 'seed', 'entries', 'reset')]
  [string]$Do,

  # For 'pick': which day in the browse column, counting from the top (1 is
  # the most recent day).
  [int]$Nth = 1,

  # For 'menu'. One of the names below.
  [ValidateSet('write', 'settings', 'browse', 'reveal', 'quit')]
  [string]$Item,

  # For 'shot': where the PNG goes, and which window to crop to.
  [string]$Out = "$PWD\reflect.png",
  [string]$Window,

  # For 'type' and 'seed'.
  [string]$Text = 'Written by the driver.',

  # For 'seed', as YYYY-MM-DD. Defaults to today.
  [string]$Date,

  # For 'shot': enlarge the crop, which makes small type legible to a model.
  [int]$Scale = 2
)

$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, System.Windows.Forms, System.Drawing

Add-Type @'
using System;
using System.Runtime.InteropServices;
public class Shell {
  // Without this the process sees a scaled desktop while CopyFromScreen
  // captures physical pixels, so screenshot coordinates and UI Automation's
  // disagree by the display's scale factor.
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern IntPtr FindWindow(string cls, string name);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] static extern void mouse_event(uint f, uint dx, uint dy, uint d, IntPtr e);

  public static void ClickAt(int x, int y) {
    SetCursorPos(x, y);
    System.Threading.Thread.Sleep(300);
    mouse_event(0x0002, 0, 0, 0, IntPtr.Zero);
    System.Threading.Thread.Sleep(80);
    mouse_event(0x0004, 0, 0, 0, IntPtr.Zero);
  }
}
'@
[void][Shell]::SetProcessDPIAware()

$Auto = [System.Windows.Automation.AutomationElement]
$Descendants = [System.Windows.Automation.TreeScope]::Descendants

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
$Exe = Join-Path $RepoRoot 'target\debug\reflect.exe'
$DataDir = Join-Path $env:APPDATA 'com.alexfrankcodes.reflect'
$EntriesDir = Join-Path $DataDir 'entries'

# The tray menu in the order TrayAction::ALL builds it. A keyboard walk is the
# only way in, so these are row numbers, and they move if that array does.
$MenuRows = @{ write = 1; settings = 2; browse = 3; reveal = 4; quit = 5 }

# Every window Reflect can open, by the title its Rust builder gives it.
$WindowTitles = @('Reflect', 'Settings', 'Entries')

function Find-ReflectWindow([string]$Title) {
  $byName = New-Object System.Windows.Automation.PropertyCondition($Auto::NameProperty, $Title)
  $byType = New-Object System.Windows.Automation.PropertyCondition(
    $Auto::ControlTypeProperty, [System.Windows.Automation.ControlType]::Window)
  $both = New-Object System.Windows.Automation.AndCondition($byName, $byType)
  return $Auto::RootElement.FindFirst($Descendants, $both)
}

function Invoke-Element($Element) {
  $Element.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
}

function Find-TrayIcon {
  # Not merely "the element named Reflect": the notes window is titled Reflect
  # too, and a Window element has no InvokePattern to call.
  $byName = New-Object System.Windows.Automation.PropertyCondition($Auto::NameProperty, 'Reflect')
  $byClass = New-Object System.Windows.Automation.PropertyCondition(
    $Auto::ClassNameProperty, 'SystemTray.NormalButton')
  $both = New-Object System.Windows.Automation.AndCondition($byName, $byClass)

  $icon = $Auto::RootElement.FindFirst($Descendants, $both)
  if ($icon) { return $icon }

  # Not on the taskbar proper, so it is behind the chevron. Opening that
  # flyout puts it in the tree.
  $chevron = New-Object System.Windows.Automation.PropertyCondition($Auto::NameProperty, 'Show Hidden Icons')
  $found = $Auto::RootElement.FindFirst($Descendants, $chevron)
  if ($found) {
    Invoke-Element $found
    Start-Sleep -Milliseconds 1000
  }
  return $Auto::RootElement.FindFirst($Descendants, $both)
}

function Save-Screen([string]$Path) {
  $bounds = [System.Windows.Forms.SystemInformation]::VirtualScreen
  $bitmap = New-Object System.Drawing.Bitmap($bounds.Width, $bounds.Height)
  $canvas = [System.Drawing.Graphics]::FromImage($bitmap)
  $canvas.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)
  $bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
  $canvas.Dispose(); $bitmap.Dispose()
  return $bounds
}

function Save-Crop([string]$From, [string]$To, $Rect, [int]$Factor) {
  $source = [System.Drawing.Image]::FromFile($From)
  try {
    $width = [int]$Rect.Width
    $height = [int]$Rect.Height
    $take = [System.Drawing.Rectangle]::new([int]$Rect.X, [int]$Rect.Y, $width, $height)
    $put = [System.Drawing.Rectangle]::new(0, 0, $width * $Factor, $height * $Factor)

    $crop = [System.Drawing.Bitmap]::new($width * $Factor, $height * $Factor)
    $canvas = [System.Drawing.Graphics]::FromImage($crop)
    $canvas.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $canvas.DrawImage($source, $put, $take, [System.Drawing.GraphicsUnit]::Pixel)
    $crop.Save($To, [System.Drawing.Imaging.ImageFormat]::Png)
    $canvas.Dispose(); $crop.Dispose()
  } finally { $source.Dispose() }
}

switch ($Do) {

  'launch' {
    if (Get-Process reflect -ErrorAction SilentlyContinue) {
      # The single-instance plugin turns a second launch into "open the notes
      # window of the copy already running", which is not what was asked for.
      'already running'
      break
    }
    if (-not (Test-Path $Exe)) {
      throw "no build at $Exe. Run: cargo build -p reflect-app"
    }

    Start-Process $Exe
    foreach ($attempt in 1..30) {
      Start-Sleep -Milliseconds 1000
      if (Find-TrayIcon) { "launched, tray icon up after $attempt s"; break }
      if ($attempt -eq 30) { throw 'started, but no tray icon appeared within 30s' }
    }
  }

  'menu' {
    if (-not $Item) { throw "menu needs -Item: $($MenuRows.Keys -join ', ')" }
    $icon = Find-TrayIcon
    if (-not $icon) { throw 'no tray icon. Is Reflect running?' }

    # Invoked through UI Automation rather than clicked: a synthetic click at
    # the icon's reported rectangle lands on whichever neighbour the tray has
    # reflowed into that spot.
    Invoke-Element $icon
    Start-Sleep -Milliseconds 1500

    if ([Shell]::FindWindow('#32768', $null) -eq [IntPtr]::Zero) {
      throw 'the tray menu did not open'
    }

    foreach ($step in 1..$MenuRows[$Item]) {
      [System.Windows.Forms.SendKeys]::SendWait('{DOWN}')
      Start-Sleep -Milliseconds 200
    }
    [System.Windows.Forms.SendKeys]::SendWait('{ENTER}')
    Start-Sleep -Seconds 3
    "chose row $($MenuRows[$Item]) ($Item)"
  }

  'windows' {
    $open = @()
    foreach ($title in $WindowTitles) {
      $found = Find-ReflectWindow $title
      # Tauri keeps an off-screen helper window titled like the app even when
      # nothing is open; it reports an infinite rectangle and is not a window
      # anyone can see.
      if ($found -and -not [double]::IsInfinity($found.Current.BoundingRectangle.Width)) {
        $r = $found.Current.BoundingRectangle
        $open += [pscustomobject]@{
          Title = $title
          X = [int]$r.X; Y = [int]$r.Y
          Width = [int]$r.Width; Height = [int]$r.Height
        }
      }
    }
    if ($open) { $open | Format-Table -AutoSize } else { 'no Reflect windows are open' }
  }

  'shot' {
    if (-not $Window) {
      $bounds = Save-Screen $Out
      "saved $Out ($($bounds.Width)x$($bounds.Height))"
      break
    }

    # Captured to a file of its own first: GDI+ holds a read lock on the image
    # it loaded, so cropping cannot write back over the same path.
    $whole = [System.IO.Path]::ChangeExtension($Out, '.whole.png')
    $bounds = Save-Screen $whole

    $found = Find-ReflectWindow $Window
    if (-not $found) { throw "no window titled '$Window'. Try: .\driver.ps1 windows" }
    $r = $found.Current.BoundingRectangle
    if ([double]::IsInfinity($r.Width)) {
      throw "'$Window' has no on-screen rectangle. Minimised?"
    }

    # UI Automation gives absolute screen pixels; the capture starts at the
    # virtual desktop's origin, which is negative when a monitor sits above or
    # left of the primary one.
    $rect = [pscustomobject]@{
      X = $r.X - $bounds.X; Y = $r.Y - $bounds.Y
      Width = $r.Width; Height = $r.Height
    }
    Save-Crop $whole $Out $rect $Scale
    Remove-Item $whole -Force
    "saved $Out. $Window at $([int]$r.X),$([int]$r.Y), scaled ${Scale}x"
  }

  'pick' {
    $found = Find-ReflectWindow 'Entries'
    if (-not $found) { throw "the browse window is not open. Run: .\driver.ps1 menu -Item browse" }
    $r = $found.Current.BoundingRectangle
    if ([double]::IsInfinity($r.Width)) { throw 'the browse window has no on-screen rectangle' }

    # There are no clickable elements to find: the page is a webview, and its
    # buttons are DOM, not UI Automation. So the row is worked out from the
    # window's own geometry. The browse window is built 720 CSS px wide, which
    # is where the display scale comes from without asking about monitors.
    # The rest are browse.css: .days padding-top, and a day button's box.
    $scale = $r.Width / 720.0
    $caption = 32 * $scale
    $navTop = 19 * $scale
    $pitch = 31 * $scale
    $middle = 15 * $scale

    $x = [int]($r.X + 100 * $scale)
    $y = [int]($r.Y + $caption + $navTop + ($Nth - 1) * $pitch + $middle)

    # A click on a window that isn't in front only activates it: the webview
    # swallows that one, and the day is not selected. So the activating click
    # is spent on empty reading pane, where nothing is listening, and the real
    # click lands on an already-focused window.
    [void][Shell]::SetForegroundWindow([Shell]::FindWindow($null, 'Entries'))
    [Shell]::ClickAt([int]($r.X + $r.Width * 0.75), [int]($r.Y + $r.Height * 0.8))
    Start-Sleep -Milliseconds 400

    [Shell]::ClickAt($x, $y)
    Start-Sleep -Seconds 1
    "clicked day $Nth at $x,$y (scale $([math]::Round($scale, 2)))"
  }

  'type' {
    # SendKeys treats these as its own syntax; a journal entry should not.
    $escaped = $Text -replace '([+^%~(){}\[\]])', '{$1}'
    [System.Windows.Forms.SendKeys]::SendWait($escaped)
    Start-Sleep -Milliseconds 500
    "typed $($Text.Length) characters"
  }

  'close' {
    # The notes window has no save button: closing it IS the save.
    [System.Windows.Forms.SendKeys]::SendWait('%{F4}')
    Start-Sleep -Seconds 2
    'closed the focused window'
  }

  'quit' {
    & $PSCommandPath menu -Item quit
    Start-Sleep -Seconds 2
    # Quit goes through the notes window's save, so a window refusing to close
    # keeps the app up. Say so rather than leave a process nobody expects.
    if (Get-Process reflect -ErrorAction SilentlyContinue) {
      'still running. A window may be refusing to close; Stop-Process to force it'
    } else { 'quit' }
  }

  'seed' {
    if (-not $Date) { $Date = (Get-Date).ToString('yyyy-MM-dd') }
    if ($Date -notmatch '^\d{4}-\d{2}-\d{2}$') { throw "-Date must be YYYY-MM-DD, got '$Date'" }
    New-Item -ItemType Directory -Force $EntriesDir | Out-Null
    # Reflect trims and appends a newline, so a seeded file matches one it
    # wrote. An empty file is deliberately not listed by the browse window.
    Set-Content -Path (Join-Path $EntriesDir "$Date.txt") -Encoding utf8 -Value $Text
    "seeded $Date"
  }

  'entries' {
    if (-not (Test-Path $EntriesDir)) { "no entries folder yet: $EntriesDir"; break }
    $files = Get-ChildItem $EntriesDir
    if ($files) { $files | Select-Object Name, Length | Format-Table -AutoSize }
    else { "no entries in $EntriesDir" }
  }

  'reset' {
    # Only the entries: settings.txt and last-reminder.txt are left alone, so a
    # reset does not silently move the user's daily reminder.
    if (Test-Path $EntriesDir) { Remove-Item (Join-Path $EntriesDir '*.txt') -Force }
    "entries cleared from $EntriesDir"
  }
}
