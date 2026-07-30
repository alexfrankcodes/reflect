<p align="center">
  <img src="assets/reflect.svg" alt="" width="104" height="104">
</p>

<h1 align="center">Reflect</h1>

<p align="center">
  A tray-only desktop journal that interrupts you once a day, at a time you chose, and offers a blank page.<br>
  Everything it keeps is a plain text file on your own machine.
</p>

---

Reflect lives in the system tray and does nothing until the time you set.
Then it nudges you once, offers a prompt and somewhere to write, and gets out of the way.
There is no account, no sync, no database, and nothing leaves your machine.

## Installing

Reflect is a Windows app in practice.
The macOS build is type-checked by CI and has never been run by anyone, so there is no macOS release — see [ADR 0002](docs/adr/0002-unsigned-windows-only-first-release.md).

Releases are published on the [Releases page](https://github.com/alexfrankcodes/reflect/releases) as an unsigned NSIS installer.
It installs per-user, so there is no administrator prompt.
Because it is unsigned, Windows SmartScreen shows an "unrecognised app" warning — click **More info**, then **Run anyway**.
There is no self-updater: you upgrade by downloading the next release.

[Building from source](#building-from-source) is the other way in, and it is two commands.

## Using Reflect

### There is no window at startup

Reflect opens nothing when it launches.
That is correct, not a hang — the tray icon is the whole of its persistent interface.
Click it for the menu:

| Menu item | What it does |
|---|---|
| **Write Today's Reflection** | Opens today's page, whether or not the reminder has been and gone. |
| **Settings…** | The daily time, writing prompts, and starting at login. |
| **Browse Entries** | Read back everything you have written. |
| **Reveal Entries Folder** | Opens your entries in Explorer. This is the whole of export. |
| **Quit Reflect** | Saves an open page first, then exits. |

If you cannot see the icon, it is probably behind the chevron in the notification area — drag it out to keep it visible.

### The reminder

One reminder a day, at the time you chose, defaulting to 9pm.
Clicking it opens today's page.
Dismissing it is the end of it for that day: Reflect does not ask again.

If your machine was asleep or off at the appointed hour, the reminder arrives when it wakes.
A reminder that late is let go entirely once it would land within an hour of the next day's — two nudges almost back to back are worse than a missed one.

### Writing

The page shows the day's prompt above a blank area, and that is all it shows.

**Closing the window is the save.**
There is no save button, no keyboard shortcut, and never a "discard your changes?" dialog.
Close the window and what you wrote is on disk.

A few things follow from that, all deliberate:

- A page you close without writing anything leaves no file at all — an unwritten day simply has no entry.
- A page holding nothing but the prompt Reflect put there also counts as nothing.
- Reopening today's page picks up after what you already wrote, so you can come back to a day as many times as you like.
- Clearing the page and closing is not a delete — the day's file stays as it was, because Reflect only ever adds to the record.

### Browsing

**Browse Entries** lists every day you have written on, most recent first, and shows the day you pick.

Past entries are read-only.
They are a record of what you wrote, not a draft to revise — if you want to change one, it is a text file and Notepad will open it.

### Settings

Changes apply the moment you make them.
There is no Save button here either; the line at the bottom of the window tells you what Reflect is now going to do.

| Setting | Default | |
|---|---|---|
| **Daily reflection time** | 21:00 | One time a day, every day — not a schedule, and not per-weekday. |
| **Writing prompts** | On | Off means no prompt line at all, not a blank one: Reflect as a plain journal. |
| **Start Reflect at login** | On | Windows only. The row is not shown on platforms that do not support it. |

Reflect ships with a fixed library of 30 prompts.
The prompt is chosen from the date, so a given day always shows the same one, and consecutive days walk the whole library before any of them comes round again.

## Where your writing lives

One plain text file per day, in the folder Windows keeps per-app data in:

```
%APPDATA%\com.alexfrankcodes.reflect\
├── entries\
│   ├── 2026-07-24.txt
│   └── 2026-07-25.txt
├── settings.txt
└── last-reminder.txt
```

There is no database and no index, because the folder is the index.
Back it up, sync it, grep it, or put it in a git repository — it is yours, and Reflect will read back whatever you leave there.

`settings.txt` appears the first time you change a setting.
It is one `key = value` per line and is meant to be readable:

```
daily-time = 21:00
show-prompts = on
start-at-login = on
```

Edit it by hand if you prefer.
A line Reflect does not recognise is stepped over rather than treated as a broken file, so a note at the top of the file costs you nothing.

## Building from source

No Node, no bundler, no dev server — Reflect is pure Cargo.
You need a [Rust toolchain](https://rustup.rs/) and the MSVC build tools.
WebView2 is needed too, and Windows 11 already has it.

```powershell
cargo run -p reflect-app
```

Then use the tray icon.

The built binary is `target\debug\reflect.exe` — **not** `reflect-app.exe`.
The package is `reflect-app`, but `src-tauri/Cargo.toml` names the binary `reflect`.

The frontend under `src/` is embedded into the binary at build time rather than served, so editing any `.html`, `.css`, or `.js` there does nothing until you build again.

To produce an installer you also need the Tauri CLI, which is not a project dependency:

```powershell
cargo install tauri-cli
cargo tauri build          # writes to target\release\bundle\
```

That writes the same unsigned NSIS installer a release ships.

One difference worth knowing about a Reflect you built rather than installed: its reminder arrives attributed to the raw string `com.alexfrankcodes.reflect`, with no icon.
The name and icon come from the Start Menu shortcut the installer writes, and Reflect deliberately does not write one itself.
[ADR 0001](docs/adr/0001-reflect-writes-its-own-start-menu-shortcut.md) records what was measured to establish that, and what it cost to find out.

### Checks

```powershell
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all --check
```

All the decision-worthy logic lives in `crates/reflect-core` and is unit-tested there.
`src-tauri` and `src/*.js` are deliberately untested glue, verified by running the app.

## Layout

```
crates/reflect-core/   entries, schedule, prompts, settings — the rules, tested
src-tauri/             the Tauri shell: tray, windows, notifications, autostart
src/                   the three pages — notes, settings, browse
docs/adr/              decisions, including the ones that were reversed
CONTEXT.md             the vocabulary this project uses, and what it avoids
```

If you are going to change anything here, [`CONTEXT.md`](CONTEXT.md) is the place to start.
It defines what an *entry*, a *reminder*, an *occurrence* and a *skip* mean in this codebase, and the code holds itself to those words.

## Licence

[MIT](LICENSE).
