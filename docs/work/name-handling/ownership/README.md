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
