# Native procedure recompilation

The producer runs four independent cases on Tcl 8.4.20, 8.5.19, 8.6.18,
9.0.4 and 9.1.0. It captures 100 procedure, body, default and frame ownership
snapshots and 40 completion results. Every snapshot captures identity,
reference counts, primary type and resident-string presence before a result
or string observer runs. `manifest.json` retains source, binary, library and
output hashes, command lines and process results.

The cases retain a sole declaration, an actual `procbody` object, an active
procedure frame during recursive recompilation, or the original dynamic
local-name object. Invalidation uses a real core command rename and replacement;
the original native handler stays registered throughout forwarding.

`S` records contain case, window, original procedure/body/default identity
comparisons, procedure and body references, body type and string presence,
default type/references/string presence, original key type/references,
current-frame identity/references, compiled local count and compile epoch.
`R` records contain case, window, completion code and exact result bytes in hex.

Tcl 8.4 and 8.5 replace a shared procedure during recompilation. Tcl 8.6 and
later retain the procedure and body identities. A Tcl 8.4 `localVarName`
primary owns the procedure; later releases retain the selected local-name
table instead. Jim has no C `Proc` or `procbody` producer in this corpus.
