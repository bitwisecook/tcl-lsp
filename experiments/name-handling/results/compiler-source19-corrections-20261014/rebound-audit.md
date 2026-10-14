# Rebound candidate consumer

`ModuleCommandBindings::record_proc_rebound_candidates` maintains a conservative roster that withdraws procedure-binding trust after a mutation. It does not allocate command objects, select procedure bodies, or grant callable targets. Original typed slots retain command-table lookup and mutation identity.

Authored namespace coordinates use `tcl_syntax::naming::bareword_resolution_candidates`, then the existing mutation-label projection. Both bare and relative qualified written names include current-namespace and global candidates. Rooted names retain their single rooted candidate. Runtime-selected unqualified contexts remain opaque. Native and reached source namespace keys continue through `source_keys_checked` without string reconstruction.

The bounded consumer test checks relative qualified, bare and rooted candidates and asserts that candidate collection produces no binding rows. Its compiler execution is pending. The shared syntax owner already carries the name-resolution geometry contract; this consumer adds no independent colon parser.

## Validation receipt

The narrow patch parses with rustfmt and passed Main `git apply --check` before integration. Compiler19 failed before executing source controls because of fixture imports and old map/tuple accesses; no source drift was observed. `typing-03.patch` and `test-source-input-04.patch` retain the original assertions and actual body/source ownership. No Cargo command was run by this lane for these changes.
