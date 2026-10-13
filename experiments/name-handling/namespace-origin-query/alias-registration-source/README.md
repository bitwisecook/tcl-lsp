# Alias registration coordinates

The original immutable `generic/tclInterp.c` files are byte-equal to the corresponding provisioned C references. `provenance.json` records commits, URLs, Git blob digests and full source digests.

C8.4 `AliasCreate` (lines 1230–1254) retains the original C-string registration coordinate in `aliasTable`. Reusing it deletes the prior actual command token, including a command moved by `rename`. C8.5 (lines 1502–1535) chooses a distinct alias-record token when the coordinate collides. These records remain independent of the command’s namespace slot and of its counted report spelling.

The unchanged `../input.tcl` fixture observes that difference at `alias-rename-setup`, `alias-replacement-setup` and `renamed-alias-query`. C8.4 reports the moved alias missing after replacement; C8.5–9.1 retain it. The selected registration owner must preserve this distinction without reconstructing a token from a report.

The shared table stores backend-owned tokens. Backends validate actual generation and current physical placement, retire the prior token using their existing lifecycle owner after successful alias-loop checking, retry collisions introduced by callbacks, and forget actual deleted tokens at alias implementation retirement. Collision selection borrows the retained receipt; it does not remove the coordinate before callbacks. Relocation retains the registration coordinate. Pure raw-NUL coordinate tests check byte geometry only; this ASCII oracle fixture supplies no raw-NUL or native allocation result.
