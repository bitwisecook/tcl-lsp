# Namespace fallback and compiler lookup

The original source compares ordinary known-command lookup, a dead missing
command visited only by the compiler, and entered missing-command fallback.
C Tcl 8.5–9.1 invokes the namespace unknown prefix only after ordinary lookup
fails at execution. The callback also receives a rooted missing command;
a rooted existing command and a compiler-only missing command invoke none.
C Tcl 8.4 and current Jim record their unsupported namespace-unknown surface.

The manifest retains source/binary/output hashes and original interpreter argv.
Each native process has a 60-second timeout.
