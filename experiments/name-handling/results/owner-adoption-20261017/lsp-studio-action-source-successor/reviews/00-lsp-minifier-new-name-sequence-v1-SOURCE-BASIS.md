# Source basis and purpose

Exact audited unit: `lsp-minifier-new-name-sequence`, catalogue area LSP,
`rust/tcl-lsp-core/src/minify.rs::NameGenerator / next_unused_name`.
The requirement separates new compacted variable/procedure/alias candidates
from existing-name lookup, calls for a shared generated-name owner and retains
partial Logical collision inventories as partial. Scope-label identity570 and
symbol-report serialization are independent and unchanged.

The existing shared owners examined are Syntax naming/scaffold (other generated
suggestion policies), naming/variable_spelling::scalar_source_reference, and
word_rules::lexical_ascii_source_word. No compacted length-first sequence or
its observed-collision/attempt-budget policy existed there. The new private
naming module extends this same Syntax new-source construction axis; it does
not copy a Tcl lookup/resolver or Native name projection.

Actual adopter map:

| Existing use | Actual supplied configuration | Preserved attempts |
| --- | --- | --- |
| Isolated procedure names in compact_names | input.lexer_config() | unbounded |
| Isolated Logical formals | input.lexer_config() | 1000 per formal |
| General local/formal scope candidates | ScopeCtx copied from actual input.lexer_config() | 1000 per request |
| Command aliases | actual input.lexer_config() through alias_repeated_commands/alias_by_uses | unbounded |
| Argument aliases | actual input.lexer_config() through alias_repeated_arguments/alias_by_uses | unbounded |
| String aliases | input.lexer_config() | unbounded |

All predicates retain the same exact observed String-set membership; the
shared owner advances through collisions with the identical candidate order.
Rejected cost/length proposals remain consumed as previously. Claiming and all
existing census/barrier/formal/reference/alpha guards are unchanged. The extra
source-word/scalar grammar check may explicitly decline an unrepresentable
candidate; no missing/foreign grammar fallback occurs. No false predicate is
an unused-name or complete-inventory receipt. Aggressive output still authors
observable new variables, independently of unresolved collision debt.

The shared public API/owner row, consumer matrix and name-handling contract
are included. All old Core test text from its original test module through its
old end is byte-identical; a separate new adopter module is appended. New
Syntax controls are fixed deterministic cases, not a fuzz campaign. Source
preflight is only three Rust parse/fmt checks, exact captured before/after
hashes and fresh zero-fuzz zero-offset patch forward/inverse. No builds,
Cargo, tests, native probes, main edits or count/acceptance advances occurred.

Suggested Root-owned validation after an actual new source binding:

- `cargo test --locked -p tcl-syntax --lib authored_compact_names_`
- `cargo test --locked -p tcl-lsp-core --lib minify::authored_compact_candidate_tests::`
- Retain the current actual minify cohorts and independent Native alpha controls.

The Syntax library test artifact is a separate required actual artifact; it
must not be represented by an existing Compiler/Core executable or guessed hash.
