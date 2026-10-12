# Where does the original Logical hint control first lose its int/double facts?

Read-only proposal; not executed. Current compiler sources are being edited.
The [retained result](frozen-build-2/result.json) and [raw log](frozen-build-2/logical.stdout-stderr.log)
measure `[]` at the first positive int-label assertion, not its cause.
Keep the [existing question](logical-hint-first-barrier-proposal.md) and its uncertainty.

Preserve the exact `namespace eval Ω` / `:pair` / `pair` source, `x=42` / `x=3.5`,
`Analyser::analyse(source, "tcl")`, retained profile/registry/config, whole-document
range, type hints `true`, parameter hints `false`, and every existing assertion.
Add observation only after coordinator grants source ownership and a build lease.

Owners, relative to the repository root:
- `rust/tcl-lsp-core/src/inlay_hints.rs`: `inlay_hints_in_program`, `collect_type_hints`,
  `type_display`, `normalise_fn_name`, `walk_scope_type_hints`.
- `rust/tcl-lsp-core/src/original_invocation.rs`: `source_compilation_unit`.
- `rust/tcl-compiler/src/compilation_unit.rs`: read-only `FunctionUnit` / `functions()` API.

Record each reached stage; label unreached stages explicitly:
1. Before each hint-entry return, report analysis/config presence, exact-image match,
   lexical-advice result, retained profile/registry presence and the unchanged hint flags.
2. In `source_compilation_unit`, report `current_analysis` admission, retained input
   presence, registry semantic-key equality, and successful unit return independently.
3. On the Logical branch, report each function's exact/normalised name; for SSA `x`,
   report symbol/version, lattice kind, `type_display` and final map membership.
   Distinguish no function, no `x` entry, Unknown/Overdefined, and an empty display map.
4. In the scope walker, report Proc scope exact/normalised key and selected-map hit,
   scoped `x` row/name/span, type-map lookup and requested-range admission before emit.
   Stop interpretation at the first refusal/absence; a key mismatch requires both sides.

Leased invocation from repository root, using the recorded `/tmp/name-env.sh` environment:
`CARGO_TARGET_DIR=tmp/name-target-issues cargo test --offline -j 2 -p tcl-lsp-core --lib inlay_hints::retained_hint_context_tests::logical_type_hints_keep_constructed_colon_scopes_and_original_source -- --exact --nocapture`
Capture exact argv/environment, source-before plus observation diff, binary/log hashes,
outcome, and restored-source-after; record actual dependency scope without an all-Rust claim.
