# The recovery "known-command generality" package fixture
# (`errorRecovery.test.ts`): `mypkg`'s sole command, resolved via
# `package require mypkg` against this workspace-scanned `pkgIndex.tcl`.
proc mypkg_helper {x} {return $x}
