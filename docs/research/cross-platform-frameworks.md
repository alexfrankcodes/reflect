# Research findings: cross-platform lightweight native frameworks for Reflect

Type: research findings
Status: complete
Feeds: [Decide: tech stack/framework for Reflect v1](https://github.com/alexfrankcodes/reflect/issues/3)

This is a fact-finding document only.
It does not rank or recommend a candidate.
Each claim below is cited to the primary source that owns it — official documentation, source code, or first-party API reference.
Where no primary source states a fact (e.g. concrete size/memory numbers that vendors don't publish), that gap is called out explicitly rather than filled with a blog estimate presented as fact.

Candidates covered in depth: **Tauri**, **Electron**, **Native-per-platform** (Swift/AppKit on macOS + WinUI 3/WPF on Windows).
Candidates covered briefly, with inclusion/exclusion rationale: **Flutter desktop**, **Qt**, **.NET MAUI**, **wxWidgets**.

---

## 1. Tauri

Tauri is a Rust-based toolkit that renders the UI in the OS's native webview (WebView2 on Windows, WKWebView on macOS) rather than bundling a browser engine, with application logic split between a Rust core and a JS/TS frontend.

### 1.1 Bundle size / memory footprint

- Tauri's own docs claim only qualitatively that it "provides very small binaries" by default, and describe size-reduction levers (release Cargo profiles, an ACL-based `removeUnusedCommands` option to strip unused command bindings) — but publish **no first-party numeric size or memory benchmark**. [Tauri: App Size](https://v2.tauri.app/concept/size/)
- The architectural reason Tauri binaries are small is documented: Tauri does not ship a bundled browser engine — it uses whatever native webview is already on the OS (WebView2 on Windows, WKWebView via WebKit on macOS, WebKitGTK on Linux), so the app binary only needs to contain the Rust runtime and app code, not a browser. [Tauri: App Size](https://v2.tauri.app/concept/size/)
- No official Tauri source publishes a memory-footprint figure for a baseline app. Third-party benchmarks (not primary sources, cited here only to flag they exist, not as fact) commonly claim installers in the single-digit-MB range and idle RSS in the tens-of-MB range, contrasted with Electron; these are **not from tauri.app or the Tauri GitHub org** and should be independently re-benchmarked before being relied on in the decision ticket.

### 1.2 Native UI/feel

- The webview surface renders whatever HTML/CSS/JS the app author supplies, so "native feel" of the content area is entirely up to the frontend code, not the framework — unlike a widget-toolkit approach, Tauri does not supply native controls itself. This follows directly from the "native webview" architecture above; it is a corollary of the documented rendering model rather than a separately stated doc claim.
- Window chrome, system tray, and native menu integration are provided via Tauri's own Rust APIs (see 1.4), which do use real native window/menu/tray objects under the hood on each OS.

### 1.3 System notifications (macOS + Windows)

- The official `@tauri-apps/plugin-notification` supports Windows, Linux, macOS, Android, and iOS. On Windows specifically, the plugin only shows the correct app name/icon for **installed** apps — during development it "shows powershell name & icon in development." [Tauri: Notification Plugin](https://v2.tauri.app/plugin/notification/)
- **Click-to-action gap (desktop):** the plugin's "Actions" feature (interactive buttons/inputs registered via `registerActionTypes()`, consumed via `onAction()`) is documented as **"only available on mobile platforms"** (Android/iOS) — not Windows/macOS. [Tauri: Notification Plugin](https://v2.tauri.app/plugin/notification/)
- There is **no documented generic "notification clicked" event** for desktop in the official plugin API. Inspecting the plugin's own JS surface confirms the only interaction events are `onNotificationReceived()` and `onAction()` — no plain click/open event. [tauri-apps/plugins-workspace, notification plugin guest-js source](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/notification/guest-js/index.ts)
- This gap is a known, open community request, not a documentation oversight: a feature request for a desktop notification click event is open against Tauri core ([tauri-apps/tauri#4770](https://github.com/tauri-apps/tauri/issues/4770)) and again against the plugin repo, where a user notes they run the app from the tray and "couldn't subscribe to any notification onclick event" ([tauri-apps/plugins-workspace#2150](https://github.com/tauri-apps/plugins-workspace/issues/2150)).
- Net effect for Reflect's "click a notification to open the notes window" requirement: the first-party plugin does not currently document a supported way to do this on desktop; it would likely require dropping to platform-specific code (e.g. calling `UNUserNotificationCenter` on macOS or the Windows App SDK's `AppNotificationManager` directly from Rust) rather than relying purely on the notification plugin.

### 1.4 System tray / menu bar icon

- Tauri ships a first-party tray API (Rust + JS) supporting icon creation, attached menus, and mouse events (click/double-click/enter/move/leave), configurable to show a menu on left-click, right-click, or both. Requires enabling the `tray-icon` Cargo feature. [Tauri: System Tray](https://v2.tauri.app/learn/system-tray/)
- Documented platform caveat is for **Linux**, not macOS/Windows: on Linux the click/hover mouse events are not emitted even though the icon and its context menu still show. This implies macOS and Windows are the platforms where the full mouse-event API is expected to work as documented. [Tauri: System Tray](https://v2.tauri.app/learn/system-tray/)

### 1.5 Background scheduling

- Tauri core has **no first-party "run at wall-clock time" scheduling API**. Options surfaced in the ecosystem are all third-party/community plugins, e.g. `tauri-plugin-schedule-task` (cron-style scheduling of Rust closures) and `tauri-plugin-background-service` (manages a long-lived background service/keepalive across platforms) — neither is an official `tauri-apps` package. [crates.io: tauri-plugin-schedule-task](https://crates.io/crates/tauri-plugin-schedule-task), [crates.io: tauri-plugin-background-service](https://crates.io/crates/tauri-plugin-background-service)
- Practically, a Tauri app must keep its own process resident (e.g. in the tray) and run its own in-process timer/async task to fire at a given time — there is no documented mechanism for the OS to wake a *not-running* Tauri process at a scheduled time the way a native local notification trigger can (see 3.5). A single-instance plugin exists to prevent duplicate tray processes, which is consistent with the "one long-lived resident process" model. [Tauri: Single Instance Plugin](https://v2.tauri.app/plugin/single-instance/)

### 1.6 Packaging / build story for OSS contributors

- **macOS build prerequisites:** Xcode (Mac App Store or Apple Developer site) — full Xcode is only required for mobile targets; Xcode Command Line Tools suffice for desktop-only builds — plus Rust. [Tauri: Prerequisites](https://v2.tauri.app/start/prerequisites/)
- **Windows build prerequisites:** Microsoft C++ Build Tools (with the "Desktop development with C++" workload), WebView2 Runtime (preinstalled on Windows 10 1803+ and later; otherwise installed via the Evergreen Bootstrapper), Rust, and — only for MSI installer output — the VBScript optional Windows feature. [Tauri: Prerequisites](https://v2.tauri.app/start/prerequisites/)
- **macOS local/unsigned builds:** code signing is documented as required for App Store listing and to avoid a "damaged/broken" Gatekeeper warning **on apps downloaded through a browser**; ad-hoc signing (pseudo-identity `"-"`) is available without any Apple account, but ad-hoc signed apps still make users go through the Privacy & Security "allow anyway" flow if the binary picks up a quarantine attribute. [Tauri: macOS code signing](https://v2.tauri.app/distribute/sign/macos/) A locally built binary that a contributor compiles and runs on their own machine is not downloaded through a browser and so does not normally acquire the quarantine attribute that triggers Gatekeeper's "unidentified developer" prompt in the first place — this is a general property of Gatekeeper (it acts on the `com.apple.quarantine` extended attribute set by quarantine-aware apps like browsers/mail clients, not by the compiler), documented by Apple only in end-user terms. [Apple Support: Safely open apps on your Mac](https://support.apple.com/en-us/102445)
- **Windows local/unsigned builds:** Tauri's own docs state signing "is not required to execute your application on Windows, as long as your end user is okay with ignoring the SmartScreen warning or your user does not download via the browser" — i.e. a contributor building and running locally hits no signing requirement at all. [Tauri: Windows code signing](https://v2.tauri.app/distribute/sign/windows/)
- **License:** dual MIT / Apache-2.0. [tauri-apps/tauri (GitHub)](https://github.com/tauri-apps/tauri)

### 1.7 Maintenance / community health

- Governance: Tauri describes itself as "a Programme within the Commons Conservancy," aiming to be "a sustainable collective," with CrabNebula named as a primary sponsor/partner and Open Collective used for funding. [tauri-apps/tauri (GitHub README)](https://github.com/tauri-apps/tauri)
- Activity: ~109k GitHub stars; release notes show continued point releases through mid-2026 across the core ecosystem (e.g. v2.10.1 in March 2026, v2.11.5 on July 1, 2026), indicating active maintenance as of this research date (July 2026). [tauri-apps/tauri Releases](https://github.com/tauri-apps/tauri/releases), [Tauri Ecosystem Releases](https://v2.tauri.app/release/)
- License: MIT / Apache-2.0 (see 1.6).

---

## 2. Electron

Electron bundles Chromium and Node.js so the entire app — UI and backend logic — runs as a full browser + Node runtime.

### 2.1 Bundle size / memory footprint

- Electron's own documentation does **not publish a baseline size or memory figure**. Its performance guide's explicit stance is "Measure, Measure, Measure" — it declines to give universal benchmarks and puts the burden of profiling on each app's developer, citing VS Code and Slack as apps that reached acceptable performance only through iterative, app-specific profiling. [Electron: Performance](https://www.electronjs.org/docs/latest/tutorial/performance)
- The structural reason Electron is heavier than a webview-based approach is architectural and undisputed: every Electron app embeds its own full copy of Chromium and Node.js rather than sharing an OS-provided runtime, which is the direct opposite of Tauri's model (1.1). This follows from Electron's documented architecture (it packages Chromium + Node) rather than from a specific numeric claim in the docs.
- As with Tauri, any specific MB/RAM numbers found in circulation are third-party blog benchmarks, not Electron project figures, and should be re-verified rather than cited as vendor fact.

### 2.2 Native UI/feel

- Because the entire UI renders inside Chromium, visual "native feel" on both macOS and Windows is entirely a function of the CSS/JS the app author writes — Electron supplies no native widget set of its own. Native-looking chrome (title bar style, vibrancy, traffic-light position, etc.) is available only through specific opt-in APIs (e.g. `BrowserWindow` vibrancy/`titleBarStyle` options), which are visual approximations layered onto a Chromium surface, not real AppKit/WinUI controls.

### 2.3 System notifications (macOS + Windows)

- Electron's `Notification` module is supported cross-platform. On **macOS**, the docs state the app "will need to be code-signed in order for notification events to emit correctly" — an unsigned binary emits a `failed` event instead, and body text is capped at 256 bytes. [Electron: Notifications](https://www.electronjs.org/docs/latest/tutorial/notifications)
- On **Windows**, basic notifications work through the same `Notification` module but require an AppUserModelID and, for advanced/interactive templates, a `ToastActivatorCLSID`; richer interactive notifications need the third-party `electron-windows-notifications` / `electron-windows-interactive-notifications` modules, not first-party Electron code. [Electron: Notifications](https://www.electronjs.org/docs/latest/tutorial/notifications)
- **Click-to-action:** supported and simple in the renderer process — `new Notification(...).onclick = () => { ... }` — so clicking a notification can trigger in-app behavior (e.g. focusing/opening the notes window) via a documented, first-party API. [Electron: Notifications](https://www.electronjs.org/docs/latest/tutorial/notifications)

### 2.4 System tray / menu bar icon

- Electron's `Tray` API is first-party and cross-platform, with platform-specific extras documented explicitly:
  - **macOS-only:** drag-and-drop events, `mouse-up`/`mouse-down` events, `setTitle()`/`getTitle()` (text next to the icon), `setIgnoreDoubleClickEvents()`, `setPressedImage()`. Template image icons are recommended (16x16 @72dpi / 32x32 @144dpi @2x).
  - **Windows-only:** balloon notifications (`displayBalloon()`/`removeBalloon()`, `balloon-show`/`balloon-click`/`balloon-closed` events), `middle-click`, `focus()`. ICO icons are recommended for best visual results.
  [Electron: Tray](https://www.electronjs.org/docs/latest/api/tray)

### 2.5 Background scheduling

- Electron ships a `powerMonitor` module that emits system power events (`suspend`, `resume`, etc.) but this is about reacting to sleep/wake, not scheduling future work. [Electron: powerMonitor](https://www.electronjs.org/docs/latest/api/power-monitor)
- There is no first-party Electron scheduling API; the documented community pattern is to combine `powerMonitor` with a Node scheduling library (`node-schedule`, `node-cron`) while keeping the app resident via the tray so the scheduled job can still fire — i.e. the whole Chromium+Node process must stay running in the background for a wall-clock-time trigger to work, there is no OS-level "wake this app later" primitive analogous to a native local notification trigger.

### 2.6 Packaging / build story for OSS contributors

- Electron itself is just the runtime; a real project also needs a packager. The two dominant first-party-adjacent tools are Electron Forge and electron-builder (community-maintained, widely used, not covered by Electron's own docs in depth, so not further detailed here as "primary source" material — flagged as a fact-finding gap).
- **macOS code signing/notarization:** Electron's own docs state plainly that "both Windows and macOS prevent users from running unsigned applications" for distribution, and that shipping on macOS requires (a) code signing and (b) notarization, which in turn requires enrolling in the paid Apple Developer Program and having Xcode installed. The docs are explicit that this guidance targets **packaging and distributing**, and do not state whether a contributor's local, non-distributed build needs any of this. [Electron: Code Signing](https://www.electronjs.org/docs/latest/tutorial/code-signing)
- **Windows code signing:** Electron's docs note that, as of a mid-2023 Microsoft policy change, Windows code signing effectively requires an Extended Validation (EV) certificate stored on FIPS 140 Level 2 / Common Criteria EAL4+ hardware for older Authenticode certs to retain their trust benefit; again this section is written entirely in terms of shipping a signed release, with no stated exemption or requirement for local dev builds. [Electron: Code Signing](https://www.electronjs.org/docs/latest/tutorial/code-signing)
- Net effect: Electron's own docs do not distinguish "local build for personal use" from "public release" the way Tauri's docs explicitly do (2.6 vs 1.6) — a contributor cloning the repo would need to infer, or a project's own build docs would need to spell out, that unsigned local builds are fine to run on your own machine even though the official guidance is written for distribution.

### 2.7 Maintenance / community health

- License: MIT. [electron/electron (GitHub)](https://github.com/electron/electron)
- Governance: Electron is a project under the OpenJS Foundation. [electron/electron (GitHub)](https://github.com/electron/electron)
- Activity: ~122k GitHub stars, 30,000+ commits on the main branch — a long-running, heavily active project.

---

## 3. Native-per-platform (Swift/AppKit on macOS + WinUI 3/WPF on Windows)

This candidate means two separate native codebases (one per OS) for UI, potentially sharing non-UI business logic (notes storage/format, scheduling logic) through a portable core (e.g. a Rust library called from both, or hand-ported logic).

### 3.1 Bundle size / memory footprint

- There is no "framework" bundle to measure here — the app links directly against OS-provided frameworks (AppKit, Windows App SDK/WPF) that already ship with the OS or as a small redistributable runtime, so there is no bundled browser engine or extra language runtime to inflate size the way Electron's does. This is a structural consequence of the approach (no embedded browser/engine) rather than a numeric claim from a single doc page; no single "official" size figure exists because the number depends entirely on the app's own code and assets, same as any native Mac/Windows app.
- If a shared Rust core is used for business logic, its contribution to size/memory is the same order of magnitude as any small compiled Rust library — negligible relative to the OS-provided UI frameworks.

### 3.2 Native UI/feel

- By construction this approach uses the OS's own UI toolkit (AppKit on macOS, WinUI 3 or WPF on Windows), so controls, window chrome, animations, accessibility, and platform HIG conformance are handled by the platform itself rather than approximated.
- On Windows specifically, Microsoft's own guidance is that **WinUI 3 is "the latest native UI framework for Windows app development"** and is the recommended choice for new native Windows apps, combining "the flexibility of the Win32 app model with the richness of modern Windows UX," whereas **WPF** is described as a "well-established framework" still fully supported via .NET, using the same XAML markup model, with full access to .NET APIs. [Microsoft Learn: Overview of framework options](https://learn.microsoft.com/en-us/windows/apps/get-started/) [Microsoft Learn: Migrate WPF app patterns to WinUI 3](https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/migrate-to-windows-app-sdk/wpf-patterns-winui3)

### 3.3 System notifications (macOS + Windows)

**macOS — UserNotifications framework:**
- Local notifications are scheduled via `UNUserNotificationCenter` with a trigger object. `UNCalendarNotificationTrigger` fires at a specific date/time built from `DateComponents`, and supports `repeats: true` to recur (e.g. specifying only hour/minute repeats it daily at that time). [Apple Developer Documentation: UNCalendarNotificationTrigger](https://developer.apple.com/documentation/usernotifications/uncalendarnotificationtrigger), [init(dateMatching:repeats:)](https://developer.apple.com/documentation/usernotifications/uncalendarnotificationtrigger/init(datematching:repeats:))
- Click handling: implementing `UNUserNotificationCenterDelegate` and its `userNotificationCenter(_:didReceive:withCompletionHandler:)` method lets the app respond when a user interacts with a delivered notification; the `response.actionIdentifier` (including the built-in `UNNotificationDefaultActionIdentifier` for "user opened the app from the notification") tells the app what the user did. [Apple Developer Documentation: UNUserNotificationCenterDelegate](https://developer.apple.com/documentation/usernotifications/unusernotificationcenterdelegate), [userNotificationCenter(_:didReceive:withCompletionHandler:)](https://developer.apple.com/documentation/usernotifications/unusernotificationcenterdelegate/usernotificationcenter(_:didreceive:withcompletionhandler:))
- These are first-party, OS-level APIs — clicking a scheduled notification is a fully documented, supported flow, unlike Tauri's current gap (1.3).

**Windows — Windows App SDK app notifications:**
- Toast/app notifications are built with `AppNotificationBuilder` and shown via `AppNotificationManager.Default.Show(...)`. Arguments attached at build time (`.AddArgument(...)`) are delivered back to the app on click. [Microsoft Learn: Quickstart — Send and Handle App Notifications](https://learn.microsoft.com/en-us/windows/apps/develop/notifications/app-notifications/app-notifications-quickstart)
- Click handling: register `AppNotificationManager.Default.NotificationInvoked` and call `AppNotificationManager.Default.Register()` before checking `AppInstance.GetActivatedEventArgs()`; if the app wasn't running, Windows launches it via COM activation and the notification's arguments are delivered through the same mechanism, letting the app distinguish "launched normally" from "launched by clicking a notification." [Microsoft Learn: Quickstart — Send and Handle App Notifications](https://learn.microsoft.com/en-us/windows/apps/develop/notifications/app-notifications/app-notifications-quickstart)
- Constraint: "App notifications aren't supported for elevated (admin) apps" — `Show()` fails silently if the app is running elevated. [Microsoft Learn: Quickstart — Send and Handle App Notifications](https://learn.microsoft.com/en-us/windows/apps/develop/notifications/app-notifications/app-notifications-quickstart)
- Registering for notification-click activation (`ToastActivatorCLSID`, COM server registration in the app manifest) is documented in terms of an MSIX-packaged `Package.appxmanifest`; this is the packaged-app story. [Microsoft Learn: Quickstart — Send and Handle App Notifications](https://learn.microsoft.com/en-us/windows/apps/develop/notifications/app-notifications/app-notifications-quickstart)
- Native scheduling: the Windows App SDK's `AppNotificationBuilder`/`AppNotificationManager` API itself has **no built-in scheduling** — but the legacy `Windows.UI.Notifications.ScheduledToastNotification` class can still be used (with an `AppNotificationBuilder`-built payload) to schedule a notification for a future time regardless of whether the app is running then. Scheduled notifications have a 5-minute delivery window: if the machine is off for longer than that at the scheduled time, the notification is dropped as no longer relevant; Microsoft recommends a background task with a time trigger instead if guaranteed delivery is required. [Microsoft Learn: Schedule an app notification](https://learn.microsoft.com/en-us/windows/apps/develop/notifications/app-notifications/scheduled-toast)
- **If using WPF instead of WinUI 3:** WPF has no built-in toast API of its own. The practical first-party-adjacent path is Microsoft's own `Microsoft.Toolkit.Uwp.Notifications` NuGet package (from the Windows Community Toolkit, authored by Microsoft), which explicitly supports WPF, WinForms, UWP, and console apps, including **unpackaged** (non-MSIX) Win32 apps with no Start-menu-shortcut requirement — `new ToastContentBuilder().AddText("Hello toast!").Show();`. A known caveat: on Windows 10 builds ≤19042, toasts from unpackaged apps using this package show as "greyed out" in Settings → Notifications & actions. [NuGet: Microsoft.Toolkit.Uwp.Notifications](https://www.nuget.org/packages/Microsoft.Toolkit.Uwp.Notifications/), [CommunityToolkit/WindowsCommunityToolkit#3870](https://github.com/CommunityToolkit/WindowsCommunityToolkit/issues/3870)

### 3.4 System tray / menu bar icon

- **macOS:** `NSStatusBar`/`NSStatusItem` are first-party AppKit APIs for adding an item to the system-wide menu bar; `NSStatusBar.system` is the shared bar, and an `NSStatusItem`'s `button` property is an `NSStatusBarButton` (a thin `NSButton` wrapper) used to configure its appearance and menu. macOS Ventura introduced `MenuBarExtra`, a SwiftUI-native struct for the same purpose. [Apple Developer Documentation: NSStatusBar](https://developer.apple.com/documentation/appkit/nsstatusbar), [NSStatusItem](https://developer.apple.com/documentation/appkit/nsstatusitem)
- Running with **no Dock icon** (menu-bar-only presence) is a documented, first-party Info.plist key: `LSUIElement` — "a Boolean value indicating whether the app is an agent app that runs in the background and doesn't appear in the Dock." An `LSUIElement` app may still show an `NSStatusItem`. [Apple Developer Documentation: LSUIElement](https://developer.apple.com/documentation/bundleresources/information-property-list/lsuielement)
- **Windows:** `System.Windows.Forms.NotifyIcon` is the standard first-party API for a taskbar notification-area icon — it works from WPF apps (via a WinForms interop reference) as well as WinForms apps, exposing `Icon`, `ContextMenu`/`ContextMenuStrip`, `Text` (tooltip), `Visible`, and `Click`/`DoubleClick` events. [Microsoft Learn: NotifyIcon Class](https://learn.microsoft.com/en-us/dotnet/api/system.windows.forms.notifyicon)

### 3.5 Background scheduling

- This is the strongest structural argument for the native approach: on macOS, a `UNCalendarNotificationTrigger` is delivered by the OS's notification daemon, not by the app process — the app does not need to be kept running for a scheduled local notification to fire at its target wall-clock time (see 3.3; this is the documented purpose of a *local* notification as opposed to requiring the requesting process to remain alive). On Windows, the same is true of a `ScheduledToastNotification` (see 3.3), modulo the documented 5-minute delivery-window caveat if the machine was off.
- This contrasts directly with Tauri (1.5) and Electron (2.5), neither of which documents an OS-level "wake and fire at time X without a resident process" mechanism — both currently rely on the app's own process staying alive with an in-process timer/tray presence.

### 3.6 Packaging / build story for OSS contributors

- **macOS:** a contributor needs Xcode (or Xcode Command Line Tools) and opens/builds the `.xcodeproj`/`.xcworkspace` via Xcode or `xcodebuild`; no third-party SDK is required beyond what Apple ships. Local, unsigned builds compiled directly by the contributor do not carry the `com.apple.quarantine` attribute that triggers Gatekeeper's "unidentified developer" prompt, since that attribute is set by quarantine-aware apps (browsers, mail clients) on downloaded files, not by the compiler — Apple's own guidance on this flow is written for the download case. [Apple Support: Safely open apps on your Mac](https://support.apple.com/en-us/102445)
- **Windows (WinUI 3):** Microsoft's own docs describe new WinUI 3 apps as packaged by default via **single-project MSIX**, but also document an explicit unpackaged distribution path "useful for enterprise scenarios where MSIX deployment isn't available, or for developers who prefer a traditional folder-based install" — trading away some package-identity-gated features (e.g. certain background task types) for a simpler, install-free build/run flow. [Microsoft Learn: Packaging overview](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/packaging/), [Microsoft Learn: Distribute an unpackaged WinUI 3 app](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/unpackage-winui-app)
- **Windows (WPF):** WPF apps build as plain Win32 executables via the standard .NET SDK/`dotnet build` — no MSIX packaging step is required at all for local build/run, which is the simplest "clone and run" path of the Windows-native options, at the cost of first-party notification support (needing the Microsoft.Toolkit.Uwp.Notifications NuGet package, see 3.3) rather than a built-in WinUI-native API.
- **Windows local/unsigned builds:** running a freshly compiled, unsigned `.exe` locally triggers no signing requirement; Microsoft SmartScreen's "Windows protected your PC" warning is specifically about running or installing files that Windows flags as unrecognized/unsigned when downloaded — a contributor who builds locally and runs the resulting exe directly is in the same position as running any other locally compiled program (Windows SmartScreen's App/Browser control primarily screens downloaded/unrecognized files); there's no first-party doc stating a hard requirement to sign purely-local builds. [Microsoft Q&A: Windows protected your PC](https://learn.microsoft.com/en-us/answers/questions/3770830/windows-protected-your-pc)
- **Cross-platform contributor overhead specific to this approach:** two separate native codebases/toolchains must each be maintained, reviewed, and kept building — Xcode/Swift on one side, Visual Studio/.NET on the other — which is qualitatively more OSS contributor surface area than a single Tauri or Electron codebase, even though each individual platform's build is simple in isolation.

### 3.7 Maintenance / community health

- Swift and AppKit are maintained by Apple; Swift itself is open-source (Apache License 2.0 with a Runtime Library Exception) and developed in the open at swift.org / github.com/swiftlang/swift, but AppKit is a closed-source, OS-bundled framework documented only through Apple's developer documentation (i.e. there is no separate "release history" to check the way there is for Tauri/Electron — it ships and versions with macOS itself).
- WinUI 3/Windows App SDK and .NET/WPF are maintained by Microsoft. The Windows App SDK is developed in the open at github.com/microsoft/WindowsAppSDK; WPF for modern .NET is developed at github.com/dotnet/wpf (MIT-licensed, part of the open-source .NET project). Both receive regular releases tied to .NET/Windows App SDK release trains.
- Because "the framework" here is really "the OS vendor's own SDKs," maintenance risk is best framed as platform-lifecycle risk (Apple's and Microsoft's own long-term support commitments to AppKit/WinUI/WPF) rather than the open-source-project-health question that applies to Tauri/Electron/Flutter/Qt/wxWidgets.

---

## 4. Other candidates considered and why they are/aren't covered in depth

### 4.1 Flutter desktop — included briefly

- Flutter officially supports compiling native Windows, macOS, and Linux desktop apps from the same codebase used for mobile. [Flutter Docs: Desktop](https://docs.flutter.dev/platform-integration/desktop)
- Reason for lighter treatment: Flutter draws its entire UI itself (via its own rendering engine/Skia-based graphics), rather than using native OS widgets — the docs describe "compiling a native... app" in the sense of producing a native binary, not in the sense of using native controls. This makes "how native does it feel" a matter of how well a custom-drawn UI mimics each platform's look, structurally similar to Electron/Qt in that respect, but with a much smaller compiled-app footprint than Electron since there's no bundled browser engine.
- Notifications and tray are **not first-party**: `flutter_local_notifications` (supports macOS via UserNotifications and Windows via a C++/WinRT Toast Notifications implementation) and `tray_manager` (desktop tray icon support for Windows/macOS/Linux) are both third-party community packages, not maintained by the Flutter/Google team. [GitHub: MaikuB/flutter_local_notifications](https://github.com/MaikuB/flutter_local_notifications), [GitHub: leanflutter/tray_manager](https://github.com/leanflutter/tray_manager)
- License: BSD-3-Clause; backed by Google. [flutter/flutter (GitHub)](https://github.com/flutter/flutter) ~178k GitHub stars.
- Verdict for inclusion: kept as a lightweight mention because it's a legitimate, actively maintained, genuinely cross-platform (including Linux, though out of scope here) desktop option — but its "native feel" comes from mimicking platforms, not using their real widgets, and its notification/tray/scheduling story for desktop all runs through third-party packages rather than first-party APIs, unlike Tauri/Electron/native.

### 4.2 Qt — included briefly

- Qt is dual-licensed: open-source (GPLv2, GPLv3, LGPLv3) and commercial, developed by The Qt Company; the LGPLv3 option permits closed-source app code under its terms, while the commercial license removes any LGPL obligations. [Qt: Licensing](https://doc.qt.io/qt-6/licensing.html)
- Qt Widgets can render with each platform's native style, and Qt ships its own tray (`QSystemTrayIcon`) and notification-adjacent APIs; a full first-party citation pass on Qt's notification-click and packaging story was out of scope for this pass given the ticket's "briefly justify" instruction for secondary candidates.
- Repo: mirrored/developed at code.qt.io (qtbase), current stable release cited as 6.11.1 (13 May 2026) — actively maintained. [Qt Licensing docs](https://doc.qt.io/qt-6/licensing.html)
- Verdict for inclusion: a serious, mature, actively maintained contender for "native-feeling" cross-platform desktop UI, but its C++ toolchain and dual-license model (LGPL compliance obligations, or a paid commercial license to avoid them) add meaningfully more contributor friction for a simple open-source clone-and-build project than Tauri, Electron, or plain native code — flagged here rather than dropped, for the decision ticket to weigh.

### 4.3 .NET MAUI — included briefly

- .NET MAUI targets Android, iOS, iPadOS, macOS, and Windows from one shared codebase; the macOS target is **Mac Catalyst** (Apple's technology for running an iOS-style UIKit app on the Mac), not native AppKit. [dotnet/maui (GitHub)](https://github.com/dotnet/maui)
- Mac Catalyst apps run in an App Sandbox with capabilities/entitlements managed the same way as an iOS App Store submission (provisioning profiles, code signing, entitlements.plist) even for local development in some flows — a meaningfully heavier setup than plain AppKit. [Microsoft Learn: Mac Catalyst capabilities](https://learn.microsoft.com/en-us/dotnet/maui/mac-catalyst/capabilities), [Microsoft Learn: Mac Catalyst entitlements](https://github.com/dotnet/docs-maui/blob/main/docs/mac-catalyst/entitlements.md)
- License: MIT; backed by Microsoft/.NET Foundation. [dotnet/maui (GitHub)](https://github.com/dotnet/maui) ~23.3k GitHub stars.
- Verdict for inclusion: worth flagging as "one shared .NET codebase across both OSes" middle ground between Electron/Tauri and the fully-native-per-platform approach, but Mac Catalyst is a materially different, sandboxed, iOS-lineage UI technology rather than true native AppKit — the ticket's "native-feeling" bar should account for that distinction rather than assume MAUI-on-Mac equals AppKit-quality native feel.

### 4.4 wxWidgets — included briefly

- wxWidgets is a C++ GUI toolkit that wraps **actual native controls** on each platform (unlike Qt's own-drawn-but-styled widgets or Flutter's fully custom rendering) — the project describes itself as a toolkit "for writing advanced GUI applications using native controls." [wxWidgets/wxWidgets (GitHub)](https://github.com/wxWidgets/wxWidgets)
- License: a modified LGPL that explicitly permits not distributing an app's own source even under static linking — a materially more permissive stance than plain LGPL for a proprietary or simple open-source app that just wants to link the library.
- Latest stable release cited as 3.3.1 (21 July 2025) — actively maintained, long-running project (originally dates to the 1990s).
- Verdict for inclusion: a serious "actually native widgets" C++ contender, structurally similar in spirit to the native-per-platform approach but through one shared C++ codebase instead of two platform-specific ones — flagged because it's a genuine lightweight/native option, though it lacks first-party notification/tray/scheduling APIs of its own (these would be hand-rolled per-platform inside a wxWidgets app much like plain native code would need, reducing its "one codebase" advantage for exactly the OS-integration features Reflect needs most).

---

## Sources

**Tauri**
- [Tauri: App Size](https://v2.tauri.app/concept/size/)
- [Tauri: Notification Plugin](https://v2.tauri.app/plugin/notification/)
- [Tauri: System Tray](https://v2.tauri.app/learn/system-tray/)
- [Tauri: Single Instance Plugin](https://v2.tauri.app/plugin/single-instance/)
- [Tauri: Prerequisites](https://v2.tauri.app/start/prerequisites/)
- [Tauri: macOS code signing](https://v2.tauri.app/distribute/sign/macos/)
- [Tauri: Windows code signing](https://v2.tauri.app/distribute/sign/windows/)
- [Tauri Ecosystem Releases](https://v2.tauri.app/release/)
- [tauri-apps/tauri (GitHub)](https://github.com/tauri-apps/tauri)
- [tauri-apps/tauri Releases](https://github.com/tauri-apps/tauri/releases)
- [tauri-apps/plugins-workspace, notification plugin guest-js source](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/notification/guest-js/index.ts)
- [tauri-apps/tauri#4770 (notification click event request)](https://github.com/tauri-apps/tauri/issues/4770)
- [tauri-apps/plugins-workspace#2150 (notification onclick request)](https://github.com/tauri-apps/plugins-workspace/issues/2150)
- [crates.io: tauri-plugin-schedule-task](https://crates.io/crates/tauri-plugin-schedule-task)
- [crates.io: tauri-plugin-background-service](https://crates.io/crates/tauri-plugin-background-service)

**Electron**
- [Electron: Notifications](https://www.electronjs.org/docs/latest/tutorial/notifications)
- [Electron: Tray](https://www.electronjs.org/docs/latest/api/tray)
- [Electron: Code Signing](https://www.electronjs.org/docs/latest/tutorial/code-signing)
- [Electron: Performance](https://www.electronjs.org/docs/latest/tutorial/performance)
- [Electron: powerMonitor](https://www.electronjs.org/docs/latest/api/power-monitor)
- [electron/electron (GitHub)](https://github.com/electron/electron)

**Native (macOS)**
- [Apple Developer Documentation: UNCalendarNotificationTrigger](https://developer.apple.com/documentation/usernotifications/uncalendarnotificationtrigger)
- [Apple Developer Documentation: init(dateMatching:repeats:)](https://developer.apple.com/documentation/usernotifications/uncalendarnotificationtrigger/init(datematching:repeats:))
- [Apple Developer Documentation: UNUserNotificationCenterDelegate](https://developer.apple.com/documentation/usernotifications/unusernotificationcenterdelegate)
- [Apple Developer Documentation: userNotificationCenter(_:didReceive:withCompletionHandler:)](https://developer.apple.com/documentation/usernotifications/unusernotificationcenterdelegate/usernotificationcenter(_:didreceive:withcompletionhandler:))
- [Apple Developer Documentation: NSStatusBar](https://developer.apple.com/documentation/appkit/nsstatusbar)
- [Apple Developer Documentation: NSStatusItem](https://developer.apple.com/documentation/appkit/nsstatusitem)
- [Apple Developer Documentation: LSUIElement](https://developer.apple.com/documentation/bundleresources/information-property-list/lsuielement)
- [Apple Support: Safely open apps on your Mac](https://support.apple.com/en-us/102445)

**Native (Windows)**
- [Microsoft Learn: Quickstart — Send and Handle App Notifications](https://learn.microsoft.com/en-us/windows/apps/develop/notifications/app-notifications/app-notifications-quickstart)
- [Microsoft Learn: Schedule an app notification](https://learn.microsoft.com/en-us/windows/apps/develop/notifications/app-notifications/scheduled-toast)
- [Microsoft Learn: NotifyIcon Class](https://learn.microsoft.com/en-us/dotnet/api/system.windows.forms.notifyicon)
- [Microsoft Learn: Overview of framework options](https://learn.microsoft.com/en-us/windows/apps/get-started/)
- [Microsoft Learn: Migrate WPF app patterns to WinUI 3](https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/migrate-to-windows-app-sdk/wpf-patterns-winui3)
- [Microsoft Learn: Packaging overview](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/packaging/)
- [Microsoft Learn: Distribute an unpackaged WinUI 3 app](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/unpackage-winui-app)
- [Microsoft Q&A: Windows protected your PC](https://learn.microsoft.com/en-us/answers/questions/3770830/windows-protected-your-pc)
- [NuGet: Microsoft.Toolkit.Uwp.Notifications](https://www.nuget.org/packages/Microsoft.Toolkit.Uwp.Notifications/)
- [CommunityToolkit/WindowsCommunityToolkit#3870](https://github.com/CommunityToolkit/WindowsCommunityToolkit/issues/3870)

**Other candidates**
- [Flutter Docs: Desktop](https://docs.flutter.dev/platform-integration/desktop)
- [flutter/flutter (GitHub)](https://github.com/flutter/flutter)
- [GitHub: MaikuB/flutter_local_notifications](https://github.com/MaikuB/flutter_local_notifications)
- [GitHub: leanflutter/tray_manager](https://github.com/leanflutter/tray_manager)
- [Qt: Licensing](https://doc.qt.io/qt-6/licensing.html)
- [dotnet/maui (GitHub)](https://github.com/dotnet/maui)
- [Microsoft Learn: Mac Catalyst capabilities](https://learn.microsoft.com/en-us/dotnet/maui/mac-catalyst/capabilities)
- [Microsoft Learn: Mac Catalyst entitlements (docs-maui repo)](https://github.com/dotnet/docs-maui/blob/main/docs/mac-catalyst/entitlements.md)
- [wxWidgets/wxWidgets (GitHub)](https://github.com/wxWidgets/wxWidgets)

## Known gaps for the decision ticket to be aware of

- No vendor (Tauri or Electron) publishes an official bundle-size or memory-footprint benchmark; any specific MB/RAM numbers used in the decision ticket should come from a benchmark the team runs itself against a minimal "hello world" build of each, not from third-party blog posts.
- Qt's notification-click-to-action behavior and packaging/build-from-source friction were not run through the same depth of primary-source verification as Tauri/Electron/native in this pass (time-boxed per the ticket's "briefly justify" instruction for secondary candidates) — worth a follow-up pass if Qt becomes a serious contender in the decision ticket.
- This document does not benchmark actual notarization/signing turnaround cost or CI setup complexity for a *release* build of any candidate — it only covers what a third-party contributor needs to clone and run a local build, per the ticket's scope.
