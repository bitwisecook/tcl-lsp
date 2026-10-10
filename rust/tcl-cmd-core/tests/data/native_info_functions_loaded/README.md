# Native info function and loaded-path source controls

`cases.tcl` contains twelve complete source-file controls. Six independently
captured shell receipts retain actual patchlevel output, executable hashes,
source/runner hashes, process status and complete stdout/stderr. Result columns
are decimal `string index`/`scan %c` units, not resident object bytes.

The function controls distinguish Tcl 8.4's fixed table from later caller-local
and global function scripts and the selected `apply` helper. Loaded controls
retain root, nested list path, binary-produced child name, malformed list and
missing child. Jim rejects both info subcommands; its two child-constructor
failures occur before `info loaded` and supply no loaded-path answer.

Run `python3 replay.py --verify-only` for exact retained associations. The copied
`capture.py` requires its explicitly recorded external shell environment and
source paths for execution; build/library/header/configure attribution and
native executables are not retained in this fixture. Binary format creates
runtime values that undergo string conversion, so these controls establish no
raw counted-zero argv, physical cache, CPP or Normal capability.
