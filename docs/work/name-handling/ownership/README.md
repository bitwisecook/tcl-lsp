# Tcl grammar and semantic ownership

Run `cargo xtask check` to check the ownership rules below. The same checks run
as part of `make xtask-gates`.

[Grammar owners](grammars.tsv) lists the reader and writer entry points. A
writer may produce a canonical word, a source fragment or a typed grammar
descriptor, depending on the grammar. Use the selected dialect when an entry
point requires it. The Tcl list reader, for example, takes the release's
escape policy. The `TCLLIBPATH` reader exposes both discovery consensus and
external-build consensus through one module.

[Walker facts](walker-facts.tsv) lists the intended owners and migration
targets. [Secondary walkers and resolvers](walkers.tsv) records existing
source scans, walk declarations and name-resolution declarations in the
analysis and editor crates. Migrate these consumers to the owner's proof for
the same site before removing their rows.

[Analysis memos](memos.tsv) tracks independent caches until the memo is
deleted or its query moves into the database. [Release tables](release-tables.tsv) tracks potential
hand-written release facts outside `tcl-dialect` and the registry. The dependency
check reads the protected crate layers from the wrapper lists in `deny.toml`. [cargo-deny wrapper configuration](https://embarkstudios.github.io/cargo-deny/checks/bans/cfg.html#wrappers)

[Module layers](module-layers.tsv) maps each `tcl-compiler` module to its
target crate and rank for the crate split
([design](../../../design/compiler/crate-split/01-module-layers.md)). A row
may name a submodule, or an item by the path that defines it, when a module's
parts go to different crates. [Back-edges](module-back-edges.tsv) counts the
production `crate::` references from a module to one of an equal or higher
rank, after following `pub use` re-exports. The `# mode:` line selects
`reporting`, which prints the ledger's diff and never fails, or `enforcing`,
which fails on any difference. `cargo xtask module-layers --write`
regenerates the back-edge ledger and adds new modules as `unassigned`, which
fails until a target is chosen. The same gate fails in both modes on
[upward dev-dependencies](dev-edges.tsv) and on
[test bodies behind `test-support`](test-support-bodies.tsv) beyond their
known lists, which may only shrink.

[Native model](native-model.tsv) assigns each registry `native_*` module, and
the runtime-protocol items split out of other core modules, to its destination
for item 117a ([design](../../../design/registry/registry-split/117-native-model.md)):
`tcl-native-model`, or `tcl-registry` where the item reads specs or the core's
word view. `pending` rows have not moved yet; an extension-trait row is a
selector over a core type that the orphan rule keeps out of an inherent impl.

[Registry test moves](registry-test-moves.tsv) counts, per crate and test
target, the tests that read the command registry without installing surfaces
(item 264a): run with `TCL_REGISTRY_TEST_INSTALL=` empty, they panic with
`NOT_INSTALLED`. A row shrinks as its tests move to an `-it` crate that
installs the surfaces it reads, and goes when it reaches zero.

[Host primitives](grammar-bypasses.tsv) records existing uses of host string
operations in Tcl grammar consumers. The scanner works on Rust source tokens
and uses the operation to identify the relevant grammar. Some recorded uses
read prose, paths, or tool output. Keep such uses confined to their existing
sites. A checked-in entry identifies the grammar owner by key, and an inline
`// grammar-bypass-ok: <grammar> — <reason>` comment, when present, must name
that same owner. An inline comment alone grants no additional allowance.

The ledgers pin a count for a declaration and operation in one source file.
A new site or increased count fails. Remove stale rows and reduce counts as
sites disappear. `cargo xtask <gate> --report` prints the observed sites for
review. The checks use syntactic recognisers, so calling a shared grammar
owner remains the responsibility of code review when new code uses a different
spelling or data path.

## Test cleanup survey

[Test cleanup](test-cleanup.tsv) is the per-crate review queue for item 499.
`cargo xtask test-cleanup` reports candidate test modules and counts;
`--write` creates the initial ledger or lowers it, and `--check` verifies it.
Neither write nor check accepts a new row or an increased count. A renamed
module must be reviewed before it moves; a rename cannot silently reset the
allowance. This survey is not a new prerequisite for the crate extractions.

Each row names the crate, source file and inline module, then the candidate
function-body count and the counts of private-field reads, internal-item
paths and private-method calls. The scanner parses Rust, including expression
arguments of assertions, and ignores quoted Rust and Tcl. It follows declared
`cfg(test)` source files, inline test modules and test-support bodies.
Ordinary Cargo integration tests are outside the library's privacy boundary:
matching their external receiver fields against a library-private name would
be a false positive. They are excluded. Source embedding with `include!`,
indirect private-state exposure through public helpers, and external test
harnesses need manual review; exported test bodies in production source remain
inventoried there. Generated tables and recorded fixture data are excluded.

This is a syntactic review inventory, not Rust name resolution or a finding
that a test should be deleted. Private names are collected per crate; a name
that is also declared public is ambiguous and is omitted. External receivers
can share private field or method names, and statement/pattern macros and
indirect helper calls need manual review. Counts cannot establish the absence
of implementation assertions. Review each module against the public API or
oracle, retain an unobservable invariant only with its reason, and regenerate
the ledger as the owning split rewrites its tests.

## Contract claims

Add each behavioural claim with a link to its test declaration or vector row.

- **Claim** A source walker or resolver added outside its owner is detected by the walker gate. [Evidence](../../../../rust/xtask/src/semantic_ownership/source.rs#finds_new_parallel_walkers_and_resolvers)
- **Claim** A dependency alias cannot evade the crate-layer check. [Evidence](../../../../rust/xtask/src/semantic_ownership/layering.rs#an_upward_edge_is_rejected_even_through_a_dependency_alias)
- **Claim** A new nested analysis memo is detected outside the database. [Evidence](../../../../rust/xtask/src/semantic_ownership/source.rs#finds_new_memos_outside_the_database)
- **Claim** A hand-written release-keyed operator table is detected. [Evidence](../../../../rust/xtask/src/semantic_ownership/source.rs#finds_hand_written_operator_tables)
- **Claim** The grammar check recognises multiline method calls and ignores comments and quoted Rust examples. [Evidence](../../../../rust/xtask/src/semantic_ownership/source.rs#finds_tcl_host_primitives_across_lines_and_ignores_literals)
- **Claim** A baseline entry cannot cover an additional site with the same declaration and operation. [Evidence](../../../../rust/xtask/src/semantic_ownership.rs#ratchet_rejects_a_second_site_and_requires_a_removed_site_to_be_deleted)
- **Claim** A module naming a module of an equal or higher rank is a back-edge, also through a `pub use` re-export, and fails only in enforcing mode. [Evidence](../../../../rust/xtask/src/semantic_ownership/module_layers.rs#a_lower_module_naming_a_higher_one_is_a_back_edge_through_a_re_export)
- **Claim** A new upward dev-dependency fails the module-layers gate. [Evidence](../../../../rust/xtask/src/semantic_ownership/module_layers.rs#a_new_upward_dev_dependency_fails)
- **Claim** A new test body behind `test-support` fails the module-layers gate. [Evidence](../../../../rust/xtask/src/semantic_ownership/module_layers.rs#a_new_test_body_behind_test_support_fails)
- **Claim** A contract claim with no linked test or vector fails validation. [Evidence](../../../../rust/xtask/src/semantic_ownership/evidence.rs#an_unlinked_claim_fails_even_if_the_other_claims_have_evidence)
- **Claim** A waiver applies only to its own tag in an attached comment. [Evidence](../../../../rust/xtask/src/util/waivers.rs#site_tags_are_exact_and_comment_scoped)
- **Claim** Literal SslicTcl words with unmatched braces keep the enclosing script balanced. [Evidence](../../../../rust/tcl-sslictcl/src/emit.rs#words_keep_the_enclosing_document_balanced)
- **Claim** The version 2.0 autoload writer preserves literal names and filenames. [Evidence](../../../../rust/tcl-lsp-core/src/package_resolver/tests.rs#tcl_index_writer_preserves_literal_names_and_paths)
- **Claim** Only the first byte of an open filename selects the pipeline grammar. [Evidence](../../../../rust/tcl-syntax/src/pipeline.rs#only_the_first_byte_selects_the_open_pipeline_grammar)
- **Claim** External `TCLLIBPATH` discovery reads braced directories as single list elements. [Evidence](../../../../rust/tcl-pkg/src/venv.rs#inherited_tcllibpath_uses_tcl_list_boundaries_for_external_builds)

## Verification on work

The six ownership checks, the 411 xtask unit tests, the cargo-deny bans check
and the focused SslicTcl, Tcl autoload, `TCLLIBPATH` and pipeline tests passed
on 24 November 2026. The autoload differential used Tcl 8.6.18.

`make prep-pr` passed formatting, code generation, the installer check and the
editor lint and type checks. The runner killed `rustc` with SIGKILL while it
compiled the `tcl-compiler` test harness. The single-job retry using the
targeted Cargo smoke runner reached the same failure. The workspace smoke
tests remain unexecuted.

`make rust-check-drift` passed, including all 46 xtask drift gates. The
workspace Clippy leg of `make rust-check` was killed with SIGKILL on the
`tcl-compiler` library test target, including on a single-job retry.
Targeted xtask Clippy passed. The workspace lint recipe stopped before its
offline X.509 and standalone runtime checks.

## Item 267 test assertion review

The 44 test names are retained across public integration tests and the
compiler-only reporting-guard unit test. These implementation assertions are
removed from the moved files; the public counterparts are described in the
[compiler test boundary](../../../design/compiler/crate-split/00-test-dependencies.md#public-test-boundary).

| Former source group | Removed assertions |
|---|---|
| `original_logical_jim_body_roles_tests` | Private authored-class receipt and parent-body access; public class/inheritance facts remain. |
| `finite_foreach_source::controls` | Name-receipt fields, replay lookup variants, private config/scope mutation and plan admission. Public declarations/references are checked across source and namespace changes. Guard storage unwinding and depth refusal stay in a compiler-only unit fixture. |
| `original_proc_source_layout_tests` | Private source-word selector calls with mutated argument vectors, swapped tokens and a replaced analyser source field. Public body spans and invocation inventories are checked across source replacement and expansion. |
| `called_static_layouts::tests` | Private layout issuers, body-frame pointers, snapshot equality and entered/result inventory equality. Public source geometry and scope/diagnostic effects remain. |
| `declaration_preview::static_read_child::tests` | Fabricated SSA child views, private frame/producer fields, missing/reissued layout rows and receipt-record identity. Public W210 cases exercise nested reads, scopes, aliases, replaced callees, redefinition, computed bodies and changed source. |
| `jim_stock_class::tests` | Private publication maps, generated name records, allocation counters and alias-owner structure. Public command presence, inherited variables and method source metadata remain. |
| `jim_stock_class::reference_prefix::tests` | Private fresh-prefix presence and analysis-owner argument flag. Public global incoming-slot queries must grant neither body command absence nor execution authority. |
| `jim_stock_class::source_advice::controls` | Private effect/publication vectors and partial, duplicated, reordered or foreign records. Public member association and refusal after unknown/replaced dependencies remain. |
| `jim_wrapper_body::tests` | Private lookup snapshot/opaque-domain and baseline fields. Public future inventories and authored namespace labels remain; the API explicitly withholds a native namespace identity. |
| `jim_wrapper_body::scope_body::tests` | Calls to the private first-entry presence worker with foreign text. Public W123 spans and refusal after effects, substitutions and uncertain history remain. |
| `source_transition_advice::source_jim_member::tests` | Corrupted recipe records, pointer ownership and withdrawn recipe state. Public class-method association, source replacement and rebinding refusals remain. |
| `original_native_static_read_owner_tests` | Private receipt queries with cloned functions, equal keys from another unit and manually wrapped/relabelled key fields. Public diagnostics distinguish each source's static, formal, nested and global reads. An equal structural key is not a public unit-allocation identity. |

Validation is recorded in the active naming snapshot. The full compiler
integration baseline before this rewrite is 182 passed / 127 failed; later
shared-branch codegen changes must be distinguished from this test-only rewrite
when comparing the combined tree. Before the registry harness move, the
rewrite gave 184 passes / 124 failures and zero new failures. The three fixed
names are the two wrapper namespace-identity assertions (authored layouts
withhold native identity) and the foreign-unit structural-key assertion;
the replacement public observations pass. These are owner-authorised test
cleanup, not production behaviour changes.
