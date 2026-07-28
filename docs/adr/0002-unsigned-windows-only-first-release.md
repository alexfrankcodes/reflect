# The first release is unsigned, Windows-only, and has no updater

Reflect ships its first release publicly on GitHub Releases as an unsigned NSIS installer for Windows alone.
Code signing is deferred because an Apple Developer account and a Windows certificate are a recurring cost and a standing secret-management burden for an app with no users yet, and an OV certificate would accrue SmartScreen reputation slowly enough that the first downloads would meet a warning regardless.
macOS is deferred because nobody has ever run that build — CI type-checks it and its own comment says it "buys compilation, not behaviour" — and publishing an unrun binary invites bug reports that cannot be reproduced from a Windows machine.

## Consequences

Users meet a SmartScreen "unrecognised app" warning and have to click through it.
The README has to say so plainly rather than let people discover it and conclude the download is unsafe.

There is no self-updater.
Tauri's updater needs a minisign keypair whose private half lives in CI and must not be lost, and an unsigned binary that silently replaces itself on disk is the exact shape antivirus heuristics are built to distrust — so adding one would have made the trust problem worse, not better.
People upgrade by downloading the next release.

`bundle.targets` is set explicitly to NSIS rather than left at `"all"`, which was also producing an MSI nobody had chosen to support.
NSIS installs per-user by default, so there is no administrator prompt on an unsigned binary.

None of this is hard to undo — it is deferral, not exclusion.
macOS wants its own release once someone has run it, and signing wants revisiting when there are enough users for the warning to cost more than the certificate.
