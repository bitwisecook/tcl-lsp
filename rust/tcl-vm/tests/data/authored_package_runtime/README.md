# Authored package controls

The fixtures contain exact scripts and results from C Tcl 8.4.20 and 9.0.4.
Result bytes are hex encoded; process status is independent of the guest catch
code. Binary and input SHA-256 values identify each capture.

These controls distinguish the explicit authored Tcl84 package grammar from
its physical C9 host. They cover abbreviations, ambiguity, arity, core package
visibility, custom versions/loaders, forgetting and namespace keyword selection.
They do not identify an F5 native compiler, package object ABI or table order.
