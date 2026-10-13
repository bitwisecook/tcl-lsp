# Enter-trace replacement of the procedure definer

All five C Tcl interpreters8.4.20 through9.1.0 run the same program successfully: the `proc` enter trace renames stock proc and replaces the original head with a user procedure. The interrupted definition returns SHADOW and installs no `:audit`; the subsequent definition also returns SHADOW and installs no `:later`. Both original operands remain source inputs to the actual replacement. A compiled-definition candidate must not bind its entry or publish a stock procedure after this trace-induced selection change.

The captured Jim binary refuses `trace` at line7 with invalid command name and exits1. That real provider limitation is retained; the C trace protocol is not assumed on Jim. Each run retains original argv, raw outputs, input and before/after executable hashes. This is a direct provider observation, not a Compiler/Runtime/WASM native-entry acceptance receipt. Tcltest was not used and Jim upstream freshness was not reverified here.

The BIG-IP-access chat can execute the unchanged input and report the provider version, exact completion/stdout/stderr, interrupted-definition result and installed commands, and subsequent-definition result. In particular, determine whether the replacement chosen by the enter trace handles the interrupted call. The F5 answer is independent of these C/Jim records.
