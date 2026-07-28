# Reflect writes its own Start Menu shortcut on Windows

**Superseded on 28 July 2026.**
The premise below did not survive measurement: the stub toast activator made no difference to anything Reflect depends on, and the reminder stayed in the notification centre on the strength of the application identity alone.
What was run, and how far it reaches, is recorded under [What was actually measured](#what-was-actually-measured); the reasoning is kept as written so that the mistake is legible rather than quietly deleted.

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

An aside, consistent with that: the notification centre also held fourteen reminders from earlier development runs, filed under Windows PowerShell's identity, at a time when Reflect had no Start Menu shortcut of its own at all.
Those persisted too — though PowerShell's own shortcut was not inspected, so this corroborates rather than proves.

The finding is scoped to what was run, and the gaps matter as much as the result.
Windows 11 build 26200, unpackaged, protocol activation, four runs on one machine.
No installer was run: the shortcut was written by hand, so the reach from here to an installed Reflect rests on the bundler writing the same `System.AppUserModel.ID` onto its own shortcut — which is what it is documented to do, and what the decision above already took for granted.
The retired Microsoft text may well have been true of Windows 10, and nothing here says otherwise; it says only that Reflect cannot claim the activator as its reason today.

## What this reopens

The reason recorded above for writing the shortcut is void.
The reminder survives its own banner without an activator, so the persistence problem this decision existed to solve is not a problem, and the NSIS-plugin difficulty it was manoeuvring around does not arise.

What does not follow is that the shortcut is pointless.
A `cargo run` still has no shortcut and no identity of its own, which is why `app_user_model_id()` borrows PowerShell's and why development reminders arrive wearing another app's name — user stories 21 and 22 are about exactly that, and nothing measured here touches them.
That case was deliberately set aside: these runs used a release build precisely so that the identity under test was Reflect's own rather than the borrowed one.
So the argument that survives is both smaller than the one this decision was made on and the one the experiment did not examine.

This decision is therefore reopened rather than replaced, and no successor is recorded here.
Issue #25 asks for a shortcut carrying both the application identity and the stub activator; the activator half of that has lost the justification written above, and the identity half now stands on different and narrower ground.
What Reflect should do instead is a new decision, and it wants evidence covering the developer build — the case this experiment set aside and the only one still in play.
