# Source variable value frames

The exact direct C Tcl probe applies a lambda read through the same relative qualified variable `Target::lambda` in root, current and caller frames. C8.5.19, C8.6.18, C9.0.4 and C9.1.0 returned `ROOT_TARGET`, `LOCAL_TARGET` and `CALLER_TARGET`. A procedure cannot read the bare `lambda` stored in the root frame or another procedure's local frame; both controls return an absent-variable error. Each probe exits0 with no stderr. C8.4.20 reports no `apply` command and does not execute those controls.

These observations constrain conditional source value-map consumers, including dynamically supplied lambda bodies. A tracked value and source span alone do not provide a runtime cell, executed body or original command reference. Binary, probe and matching upstream initialization-file hashes are recorded in `results.json`. No Tcltest is used. No Rust consumer, JimTcl or F5 acceptance is claimed.
