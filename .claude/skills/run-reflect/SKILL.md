---
name: run-reflect
description: Build, launch, drive and screenshot Reflect, the tray-only Tauri journalling app. Use when asked to run, start, or screenshot Reflect, to open its notes/settings/browse windows, to confirm a change works in the real app rather than in tests, or to reproduce a UI bug end to end. Windows only.
---

# Running Reflect

Reflect is a tray-only Tauri app: it opens **no windows at startup** (`app.windows` is `[]` in `tauri.conf.json`), so there is no window for a runner to attach to and no CLI to invoke.
The tray icon is the only way in.

`driver.ps1` is that way in.
It works the tray menu through UI Automation and the keyboard, screenshots windows by their own rectangles, and seeds or clears the entries folder.
Drive the app through it rather than reaching for `Start-Process` and hand-written coordinates.

Paths below are relative to the repo root.
**Windows only** — the driver is UI Automation and Win32, and the macOS build has never been run by anyone (CI type-checks it and nothing more; see `.github/workflows/ci.yml`).

## Build

```powershell
cargo build -p reflect-app
```

The binary is `target\debug\reflect.exe` — **not** `reflect-app.exe`.
The package is `reflect-app`, but `src-tauri/Cargo.toml` sets `[[bin]] name = "reflect"`.

The frontend is not served, it is **embedded into the binary at build time** (`frontendDist: "../src"`).
Editing anything under `src/` — `.html`, `.css`, `.js` — does nothing until you `cargo build` again.
There is no dev server and no reload.

## Run (agent path)

```powershell
$d = ".\.claude\skills\run-reflect\driver.ps1"

& $d launch                                # start it, wait for the tray icon
& $d seed -Date 2026-07-22 -Text "A day."  # give it something to browse
& $d menu -Item browse                     # tray -> Browse Entries
& $d shot -Window Entries -Out "$env:TEMP\browse.png"   # just that window
& $d quit
& $d reset                                 # clear the entries again
```

Screenshots go outside the repo: `.gitignore` doesn't cover PNGs, and a run that leaves one at the root turns up in the next `git add`.

| Verb | What it does |
|---|---|
| `launch` | Starts the app, waits up to 30s for the tray icon. Says "already running" rather than starting a second copy. |
| `menu -Item <name>` | Opens the tray menu and picks a row. Names: `write`, `settings`, `browse`, `reveal`, `quit`. |
| `windows` | Lists Reflect's open windows with their screen rectangles. |
| `shot -Out <path>` | Screenshots the whole desktop. Add `-Window <title>` to crop to one window, `-Scale n` to enlarge (default 2, which makes 13px UI type legible). |
| `pick -Nth <n>` | Clicks the nth day in the browse window's left column, 1 being the most recent. |
| `type -Text <s>` | Types into the focused window. Escapes the characters `SendKeys` would otherwise eat (`+ ^ % ~ ( ) { } [ ]`). |
| `close` | Alt+F4 the focused window. For the notes window this **is** the save — there is no save button. |
| `quit` | Tray -> Quit Reflect, then confirms the process is gone. |
| `seed -Date <YYYY-MM-DD> [-Text s]` | Writes an entry file for a day. |
| `entries` | Lists the entries folder. |
| `reset` | Deletes every entry. Leaves `settings.txt` and `last-reminder.txt` alone. |

Window titles, for `-Window`: notes is **`Reflect`**, settings is **`Settings`**, browse is **`Entries`**.

**Always look at the screenshot you took.** A window that opened but rendered nothing is the failure mode worth catching, and it is invisible from the exit code.

### A full flow that works

Writing an entry and reading it back, which is most of what the app does:

```powershell
$d = ".\.claude\skills\run-reflect\driver.ps1"
& $d launch
& $d menu -Item write
& $d type -Text "Rain all afternoon. I liked it."
& $d close                    # closing saves
& $d menu -Item browse
& $d shot -Window Entries -Out "$env:TEMP\entry.png"
& $d quit; & $d reset
```

## Run (human path)

`cargo run -p reflect-app`, then use the tray icon.
No window appears at startup — that is correct, not a hang.

## Test

```powershell
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all --check
```

All the decision-worthy logic is in `crates/reflect-core` and is unit-tested there.
`src-tauri` and `src/*.js` are deliberately untested glue, verified by running the app — which is what this skill is for.

## Gotchas

- **The driver writes to your real journal.** There is no way to redirect it: Tauri's `app_data_dir` resolves through `SHGetKnownFolderPath`, which ignores the `APPDATA` environment variable, so you cannot point the app at a scratch folder. Everything lands in `%APPDATA%\com.alexfrankcodes.reflect\entries`. **Check what is there before you seed, and `reset` after.** If the folder had real entries in it, `reset` will delete them too.
- **The tray menu is not in the UI Automation tree.** It is a native `#32768` popup; searching for `MenuItem` elements finds nothing even while the menu is plainly on screen. Rows are reached by `{DOWN}` × n then `{ENTER}`, which means **the row numbers in `$MenuRows` must track `TrayAction::ALL`** in `crates/reflect-core/src/tray_menu.rs`. Reorder that array and the driver picks the wrong thing silently.
- **Don't click the tray icon; invoke it.** A synthetic click at the icon's reported `BoundingRectangle` lands on whichever neighbour the tray has reflowed into that spot — this reliably opened Razer's menu instead. UI Automation's `InvokePattern` is what works.
- **The tray icon may be behind the chevron.** A freshly built app goes into the hidden-icons overflow. The driver invokes "Show Hidden Icons" first when it can't find the icon directly.
- **Searching for the name "Reflect" finds the notes window,** which is titled `Reflect` too — and a `Window` element has no `InvokePattern`, so the call throws `Unsupported Pattern`. The tray button must be matched on `ClassName = SystemTray.NormalButton` as well.
- **The first click on an unfocused window is swallowed.** It activates the window; the webview never sees it. `pick` spends an activating click on empty reading pane first, then clicks the day for real.
- **PowerShell is DPI-unaware by default,** so `CopyFromScreen` captures physical pixels while `SystemInformation.VirtualScreen` reports logical ones, and UI Automation rectangles line up with neither. The driver calls `SetProcessDPIAware()` before anything else. It also subtracts the virtual desktop's origin, which goes **negative** when a monitor sits above or left of the primary one.
- **Keep `driver.ps1` ASCII-only.** Windows PowerShell 5.1 reads a `.ps1` with no byte-order mark as ANSI. A single em dash in a comment turns into mojibake and the whole file fails to parse with a cascade of "Unexpected token" errors that point nowhere near the real problem.
- **GDI+ locks the file it loaded from,** so a crop cannot be written back over the path it was read from ("A generic error occurred in GDI+"). The driver captures to a `.whole.png` alongside the target and deletes it after.
- **Don't drive the app while someone is using the machine.** Synthetic clicks and `SendKeys` go to whatever is in the foreground. If windows start coming forward on their own or the display arrangement changes mid-run, stop.
- **`Quit Reflect` is not `Stop-Process`.** Quit runs the notes window's save first, so a window that refuses to close keeps the app alive; the `quit` verb says so instead of pretending. `Stop-Process -Name reflect -Force` is the blunt fallback and skips the save.
- **A second `Start-Process` does not start a second copy.** The single-instance plugin routes it into "open the notes window of the copy already running" — which is also how a clicked reminder gets in.

## Troubleshooting

| Symptom | Cause and fix |
|---|---|
| `no build at ...\reflect.exe` | Not built yet: `cargo build -p reflect-app`. Note the binary is `reflect.exe`, not `reflect-app.exe`. |
| `the tray menu did not open` | Usually the app is still starting. Re-run `launch` (it will say "already running"), then retry `menu`. |
| `Unsupported Pattern` from `GetCurrentPattern` | Something named `Reflect` that isn't the tray button — almost always the notes window being open. Close it, or check the `ClassName` condition in `Find-TrayIcon`. |
| `Cannot convert value "∞" to type "System.Int32"` | The window is minimised or is Tauri's off-screen helper. `windows` filters those out; `shot -Window` reports it instead. |
| Menu opens but the wrong window appears | `$MenuRows` has drifted from `TrayAction::ALL` in `crates/reflect-core/src/tray_menu.rs`. |
| A CSS or JS edit has no effect | The frontend is embedded at build time. `cargo build -p reflect-app` again. |
| `pick` clicks but nothing selects | The window wasn't focused, or the browse layout moved. `pick` derives rows from `.days` padding and the day-button box in `src/browse.css`; change those and update the constants in the `pick` branch. |
