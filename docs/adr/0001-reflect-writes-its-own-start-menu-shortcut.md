# Reflect writes its own Start Menu shortcut on Windows

**Superseded on 28 July 2026 — Reflect does not write a Start Menu shortcut.**
The premise below did not survive measurement.
Neither the stub toast activator nor the shortcut carrying it is what keeps a reminder in the notification centre, and a reminder is delivered even with no Start Menu shortcut present at all; the shortcut supplies the name and the icon and nothing else.
What was run, and how far it reaches, is under [What was actually measured](#what-was-actually-measured), and what replaces the decision is under [The decision that replaces it](#the-decision-that-replaces-it).
The reasoning below is kept as written so that the mistake is legible rather than quietly deleted.

## The decision as it was made

Windows will not keep a reminder in the notification centre once its banner has faded unless the sending app has a Start Menu shortcut carrying both an `System.AppUserModel.ID` and a `System.AppUserModel.ToastActivatorCLSID`.
Tauri's bundlers write the AUMID and never the CLSID, so a shipped Reflect's reminder would be clickable only while it happened to be on screen — which for an app whose entire purpose is one quiet nudge a day means anyone away from their desk at nine simply loses that day.
Rather than patch the installer, Reflect creates and maintains that shortcut itself at startup, through `IShellLink` and `IPropertyStore` on the `windows` crate it already depends on.

### Considered options

**An NSIS post-install hook.**
This is what Microsoft recommends — the installer owns the shortcut.
Rejected for two reasons.
Tauri's NSIS helper is AUMID-specific, so writing an arbitrary property key would likely mean shipping a third-party NSIS plugin binary inside an already-unsigned installer.
More importantly it only helps people who run the installer: anyone who clones and builds Reflect, which the MIT licence and user story 22 actively invite, would still get a reminder that vanishes silently.

**Upstreaming a `toastActivatorClsid` option to Tauri.**
The right long-term fix and still worth doing, but it blocks a release on someone else's review cycle and needs a local workaround in the meantime regardless.

### Consequences

The app now writes outside its own install directory, which it did not before.
It must patch the shortcut the installer already placed rather than leave a second one beside it, and uninstalling has to remove what the app created.

The stub CLSID is a fixed GUID with no COM server behind it.
That is a documented Microsoft pattern rather than a trick — their words, in *Activating toast notifications from desktop apps*: "specify a CLSID on your shortcut. That can be any random GUID. Don't add the COM server/activator. You're adding a 'stub' COM CLSID, which will cause Action Center to persist the notification."
A real `INotificationActivationCallback` server is needed only for toast inputs, in-process activation, and the `foreground`/`background` activation types, none of which Reflect uses — it activates by protocol, through the `reflect://write` link the toast already carries.

That page has since been retired from Microsoft Learn, which now redirects it to Windows App SDK documentation that says nothing about the unpackaged case; the text above survives at commit `dfc8a9c` of `MicrosoftDocs/windows-dev-docs`, in `hub/apps/design/shell/tiles-and-notifications/toast-desktop-apps.md`.
No live Microsoft page states the rule today.
That is why the rule was measured rather than relied on, and measuring it is what superseded this decision.

One simplification falls out.
`app_user_model_id()` currently hands debug builds the AUMID of Windows PowerShell so that a `cargo run` toast is visible at all, which is why development reminders arrive attributed to PowerShell.
Once Reflect writes its own shortcut, a developer build has a real identity and that branch can go.

## What was actually measured

On 28 July 2026, on Windows 11 Pro 10.0.26200, against a local release build — release rather than debug so that the toast went out under `com.alexfrankcodes.reflect` rather than the PowerShell identity a debug build borrows.

A Start Menu shortcut was written by hand at `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Reflect.lnk`, through `IShellLink` and `IPropertyStore`, carrying `System.AppUserModel.ID` and — in half the runs — `System.AppUserModel.ToastActivatorCLSID` set to a random GUID with no COM server behind it.
Both properties were read back off the saved link before every run, so that a `VT_CLSID` that had failed to marshal could not be mistaken for an activator that had no effect.
Each time they read back exactly as written: the identity as `com.alexfrankcodes.reflect` (`VT_LPWSTR`), and the activator either as `{6F3F41C9-1E36-4B7D-9A2E-5E4A0C8B2D71}` (`VT_CLSID`) or, in the identity-only runs, as absent (`VT_EMPTY`).

The daily time was then set three minutes out, the reminder was allowed to fire, the banner was watched until it faded, and the notification centre was opened and the entry clicked.
Four runs, alternating identity-only, identity-and-activator, identity-only, identity-and-activator.
Alternating matters: a single before-and-after pair cannot tell a real effect from a shell that has cached the shortcut it saw first.

| Run | Shortcut | Fired | Arrived as | Still in the notification centre | Click there opened the notes window |
|---|---|---|---|---|---|
| 1 | identity only | 10:54 | Reflect's own name and icon | yes — 10:55, still there at 10:57 | yes |
| 2 | identity + activator | 11:01 | Reflect's own name and icon | yes — 11:02 | yes |
| 3 | identity only | 11:06 | Reflect's own name and icon | yes — 11:07 | yes |
| 4 | identity + activator | 11:12 | Reflect's own name and icon | yes — 11:14 | not retested |

Two entries in that table are weaker than the rest, and are worth saying plainly.
In run 2 an unrelated notification took the banner slot in the same moment, so that run's name and icon were read off the notification centre entry rather than off the banner; runs 1, 3 and 4 were read off the banner itself.
Run 4 was fired to capture the banner that run 2 lost, and its click was not repeated once the centre had been checked.

Every run that was made behaved the same way, with the activator and without it.
The banner faded after its twenty-five seconds and the reminder stayed put; clicking it in the centre opened the notes window in the process already running, by way of the `reflect://write` protocol activation rather than any COM callback.

So the stub activator changes nothing that Reflect depends on.

### Whether the shortcut is needed at all

Those four runs answered the ticket's question and left a larger one standing: every one of them had a shortcut.
Two further runs asked whether one is needed at all.

The shortcut was removed entirely and the same release build fired a reminder at 11:51 and again at 11:59.
The toast was not dropped.
It appeared, it was still in the notification centre after its banner had faded, and clicking it there opened the notes window — but it arrived attributed to the raw string `com.alexfrankcodes.reflect`, with no icon.

That identity had been registered on this machine by the earlier runs, which could have been what let it through.
So a second probe used an identity Windows had never seen — no Start Menu shortcut, and no key under `HKCU\Software\Microsoft\Windows\CurrentVersion\Notifications\Settings` — sent through the same `ToastNotificationManager` call `notify.rs` makes.
It appeared too, likewise attributed to the raw string and likewise without an icon.

Neither run is clean on both axes: the first had Reflect as the sender but a pre-registered identity, the second an unregistered identity but PowerShell as the sender.
Taken together they are strong, and they are reported here as a pair rather than as one clean measurement.

| Shortcut | Delivered | Name and icon | Persists after the banner | Click there opens the notes window |
|---|---|---|---|---|
| none at all | yes | the raw identifier, no icon | yes | yes |
| identity only | yes | Reflect's own | yes | yes |
| identity + stub activator | yes | Reflect's own | yes | yes |

So a Start Menu shortcut is not what makes a reminder appear, and not what keeps it in the notification centre.
It supplies the name and the icon, and nothing else.

Two more claims in this repo fall with that.
`notify.rs` says that without a shortcut carrying its identity "Windows drops the toast silently and `Show` still reports success", and `reminder.rs` repeats it as "Windows silently drops a toast whose app id it can't resolve".
Both are false here, and with them the stated reason for handing debug builds PowerShell's identity — "purely so that a toast can be seen at all".
A toast can be seen without it.

An aside, consistent with all of this: the notification centre also held fourteen reminders from earlier development runs, filed under Windows PowerShell's identity, at a time when Reflect had no Start Menu shortcut of its own at all.
Those persisted too — though PowerShell's own shortcut was not inspected, so this corroborates rather than proves.

The finding is scoped to what was run, and the gaps matter as much as the result.
Windows 11 build 26200, unpackaged, protocol activation, on one machine: four runs with a shortcut, two without, and one probe under an identity Windows had never seen.
No installer was run: the shortcut was written by hand, so the reach from here to an installed Reflect rests on Tauri's bundler writing the same `System.AppUserModel.ID` onto the shortcut it installs.
That is asserted by the decision above and by every ticket beneath it, and it has been verified by nobody — the bundler lives in the Tauri CLI, which is not installed on this machine, so its template was not read either.
It is named here as an assumption rather than repeated as a fact, because this document exists to record what came of trusting one of those.
Issue #26 builds the first installer this project has ever produced, and that is where it holds or falls.
The retired Microsoft text may well have been true of Windows 10, and nothing here says otherwise; it says only that Reflect cannot claim the activator as its reason today.

## The decision that replaces it

**Reflect does not write a Start Menu shortcut.**

The reason recorded above is void twice over.
The reminder survives its own banner without an activator, so the persistence problem this decision existed to solve is not a problem, and the NSIS-plugin difficulty it was manoeuvring around does not arise.
And the shortcut is not what delivers the reminder either, so the app has no functional reason to write outside its own directories at all — which was this decision's stated cost, now paid for nothing.

What is genuinely lost is the name and the icon, and only for someone running a Reflect they built rather than installed.
Their reminder says `com.alexfrankcodes.reflect` instead of Reflect, and carries no icon.
That is a real cost and it is accepted: it falls on people who cloned a repository and ran `cargo build`, it is legible rather than misleading, and it is not worth an app that edits the user's Start Menu at every startup and an uninstaller that has to clean up after it.

One simplification survives the reversal, on better grounds than the ones recorded above.
`app_user_model_id()` hands debug builds Windows PowerShell's identity, and its stated reason — that a toast could not otherwise be seen — is false.
The branch does nothing but attribute development reminders to another application, so it goes whether or not a shortcut is ever written.
That, and correcting the comments this experiment falsified, is now the whole of issue #25.

This rests on one thing not measured: that Tauri's bundler writes the identity onto the shortcut it installs, so that an installed Reflect still shows its own name and icon.
No installer has ever been built for this project.
Issue #26 builds the first one and already asks that a reminder from the installed build arrive under Reflect's own name and icon — if that fails, the bundler does not write the identity, this decision is wrong about installed users too, and writing the shortcut comes back on evidence rather than on assumption.
