//! Port of the Python `tests/test_bigip_code_actions.py`
//! ("BIG-IP code-action provider").
//!
//! ----------------------------------------------------------------------------
//! Background / what the Python suite covers
//! ----------------------------------------------------------------------------
//! The Python module `server.features._bigip_code_actions` exposes two
//! entry points:
//!
//!   * `get_bigip_code_actions(source, uri, range_)` — walks the parsed
//!     BIG-IP config for the stanza covering the cursor and emits:
//!       - "Rename <full-path>…"  → `editor.action.rename` command
//!       - "Rename partition '<n>'…" (only on `auth partition`, never
//!         `/Common`) → `tclLsp.renamePartition` command with arguments
//!         `[uri, <bare-name>]`, and NO pre-baked workspace edit.
//!   * `run_rename_partition(...)` — drives the `rename_partition(old,new)`
//!     query-DSL expression through the F5 query engine to build the
//!     `WorkspaceEdit` (with cascade + Common-refusal + name validation +
//!     post-rewrite reparse guard).
//!
//! ----------------------------------------------------------------------------
//! Parity-audit context (docs/.../python-rust-parity-audit-2026-06-22.md §3 #8)
//! ----------------------------------------------------------------------------
//! Before this port the Rust BIG-IP code-actions fell back to generic-Tcl:
//! there was NO `bigip_code_actions` equivalent (the audit's gap #8). The
//! BIG-IP *data* layer is, however, largely ported — `tcl-lsp-core`'s own
//! self-contained stanza parser (`src/bigip.rs`, already powering BIG-IP
//! documentSymbols) decomposes every `module object-type identifier { … }`
//! stanza with its source range. That made `get_bigip_code_actions`
//! buildable as a SMALL provider arm over existing data (category (b)).
//!
//! IMPLEMENTATION ADDED (minimal, noted): `tcl_lsp_core::code_actions::
//! bigip_code_actions(source, range, uri)` — a faithful port of the Python
//! `get_bigip_code_actions`, plus a `string_args: Vec<String>` field on
//! `ActionCommand` so a command can carry the `uri` / bare-partition-name
//! textual arguments the BIG-IP commands require (the pre-existing
//! integer-position rename command is unaffected; `string_args` is empty
//! there). A `pub(crate) parse_stanzas` helper was added to `src/bigip.rs`
//! reusing the existing `extract_blocks` / `parse_header` machinery.
//!
//! ----------------------------------------------------------------------------
//! Classification of the 8 Python tests (a = generic provider already;
//! b = small arm over existing data; c = large unported feature)
//! ----------------------------------------------------------------------------
//!   1. `test_no_actions_for_non_bigip_text`          → (b) — covered here
//!   2. `test_rename_action_for_object_at_cursor`      → (b) — covered here
//!   3. `test_rename_partition_action_on_partition`…   → (b) — covered here
//!   4. `test_no_partition_rename_action_for_common`…  → (b) — covered here
//!   8. `test_no_actions_when_cursor_outside_any`…     → (b) — covered here
//!
//!   5. `test_run_rename_partition_drives_through_query_engine`  → (c) GAP
//!   6. `test_run_rename_partition_safely_quotes_query_arguments` → (c) GAP
//!   7. `test_run_rename_partition_refuses_common_renames`        → (c) GAP
//!
//! Tests 5–7 exercise the `run_rename_partition` LSP *wrapper*, which drives
//! the F5 query engine's `rename_partition` builtin cascade. The engine
//! itself IS ported in the *separate* `tcl-bigip-query` crate
//! (`builtins/rename.rs::bi_rename_partition` — same name-validation
//! "partition names must match [A-Za-z0-9_.-]+" and both-direction
//! `/Common` refusal as Python), with its own parity coverage in
//! `f5-cli/tests/query_mutation_parity.rs`. The unported piece is the LSP
//! *glue* (`run_rename_partition`: JSON-quote args → `run_query` → reparse
//! guard → single-file `WorkspaceEdit`), which belongs in `tcl-lsp-server`
//! (it would force `tcl-lsp-core` to depend on `tcl-bigip-query`), not in a
//! minimal `code_actions.rs` arm. So those three are GAPs *for this
//! provider file* — see the `// GAP:` markers near the end — not absent
//! engine functionality.
//!
//! ----------------------------------------------------------------------------
//! Presentation vs semantic split
//! ----------------------------------------------------------------------------
//! A code ACTION is an editor-presentation / LSP-wire artefact: its TITLE,
//! its KIND, its COMMAND id + arguments, and whether it carries an EDIT are
//! a structural contract, not a Tcl/tclsh runtime value. Everything here is
//! asserted STRUCTURALLY and directly — there is no Tcl to run (a BIG-IP
//! `.conf` is config, not a script), so no tclsh oracle applies.

use tcl_lsp_core::code_actions::{ActionKind, CodeAction, bigip_code_actions};
use tcl_lsp_core::definition::LspRange;

// ---------------------------------------------------------------------------
// Harness — mirror the Python `_range(line)` helper: a zero-width range at
// the start of `line` (BIG-IP code actions key on `range.start.line`).
// ---------------------------------------------------------------------------

/// A zero-width LSP range at `(line, 0)`.
fn range_at(line: u32) -> LspRange {
    LspRange {
        start_line: line,
        start_character: 0,
        end_line: line,
        end_character: 0,
    }
}

const URI: &str = "file:///t.conf";

/// Find the single action whose title starts with `prefix`.
fn action_titled<'a>(actions: &'a [CodeAction], prefix: &str) -> Option<&'a CodeAction> {
    actions.iter().find(|a| a.title.starts_with(prefix))
}

// ===========================================================================
// (b) test_no_actions_for_non_bigip_text
// ===========================================================================

/// Plain comment text parses to zero stanzas → no actions.
/// Python: `get_bigip_code_actions("# not bigip\n", …) == []`.
#[test]
fn no_actions_for_non_bigip_text() {
    let actions = bigip_code_actions("# not bigip\n", range_at(0), URI);
    assert!(
        actions.is_empty(),
        "non-BIG-IP text must yield no actions, got: {actions:?}"
    );
}

// ===========================================================================
// (b) test_rename_action_for_object_at_cursor
// ===========================================================================

/// Cursor inside a parsed stanza → a `Rename <full-path>…` action whose
/// command is the editor's standard `editor.action.rename`.
/// Python asserts: some title starts with "Rename /`Common/web_pool`" and the
/// rename action's command is `editor.action.rename`.
#[test]
fn rename_action_for_object_at_cursor() {
    let source = "ltm pool /Common/web_pool { }\n";
    let actions = bigip_code_actions(source, range_at(0), URI);

    let rename = action_titled(&actions, "Rename /Common/web_pool")
        .expect("expected a 'Rename /Common/web_pool…' action");
    // RefactorRewrite kind (Python: `CodeActionKind.RefactorRewrite`).
    assert_eq!(rename.kind, ActionKind::RefactorRewrite);
    let cmd = rename
        .command
        .as_ref()
        .expect("rename action must carry a command");
    assert_eq!(cmd.command, "editor.action.rename");
    // The uri is forwarded so the editor knows which document to rename in.
    assert_eq!(cmd.string_args, vec![URI.to_string()]);
    // It is a "trigger the rename UI" action — no pre-baked edit.
    assert!(
        rename.edits.is_empty(),
        "rename-object action must not carry a pre-baked edit"
    );
}

/// The full-path in the title is exactly the stanza identifier (the
/// partition-qualified path), not the bare leaf name.
#[test]
fn rename_action_title_uses_full_path() {
    let source = "ltm pool /Common/web_pool { }\n";
    let actions = bigip_code_actions(source, range_at(0), URI);
    assert!(
        action_titled(&actions, "Rename /Common/web_pool\u{2026}").is_some(),
        "title should be the full path with an ellipsis, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

/// A non-partition object (a pool) offers ONLY the object-rename action —
/// no partition-rename action leaks onto it.
#[test]
fn pool_stanza_offers_only_object_rename() {
    let source = "ltm pool /Common/web_pool { }\n";
    let actions = bigip_code_actions(source, range_at(0), URI);
    assert_eq!(
        actions.len(),
        1,
        "pool: exactly one action, got: {actions:?}"
    );
    assert!(
        actions
            .iter()
            .all(|a| !a.title.contains("Rename partition")),
        "a pool stanza must not offer a partition rename"
    );
}

// ===========================================================================
// (b) test_rename_partition_action_on_partition_stanza
// ===========================================================================

/// An `auth partition` stanza gets an extra `Rename partition '<name>'…`
/// action that triggers `tclLsp.renamePartition` with `[uri, name]` and
/// carries NO pre-baked edit (the user supplies the real name; the cascade
/// runs through the query engine on accept).
#[test]
fn rename_partition_action_on_partition_stanza() {
    let source =
        "auth partition Tenant_A { default-route-domain 0 }\nltm pool /Tenant_A/web_pool { }\n";
    let actions = bigip_code_actions(source, range_at(0), URI);

    let partition = action_titled(&actions, "Rename partition")
        .expect("expected a 'Rename partition …' action on the partition stanza");
    // No pre-baked workspace edit — the action triggers a command.
    assert!(
        partition.edits.is_empty(),
        "partition rename must not carry a pre-baked edit (Python: edit is None)"
    );
    let cmd = partition
        .command
        .as_ref()
        .expect("partition action must carry a command");
    assert_eq!(cmd.command, "tclLsp.renamePartition");
    // Arguments mirror Python exactly: `[uri, "Tenant_A"]`.
    assert_eq!(
        cmd.string_args,
        vec![URI.to_string(), "Tenant_A".to_string()]
    );
    assert_eq!(partition.kind, ActionKind::RefactorRewrite);
}

/// The bare partition name (no leading slash) is exact in the title.
#[test]
fn partition_action_title_uses_bare_name() {
    let source = "auth partition Tenant_A { default-route-domain 0 }\n";
    let actions = bigip_code_actions(source, range_at(0), URI);
    assert!(
        action_titled(&actions, "Rename partition 'Tenant_A'\u{2026}").is_some(),
        "expected bare-name partition title, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

/// On a partition stanza BOTH actions appear: the generic object rename
/// AND the partition rename (the partition is also a renameable object).
#[test]
fn partition_stanza_offers_both_actions() {
    let source = "auth partition Tenant_A { default-route-domain 0 }\n";
    let actions = bigip_code_actions(source, range_at(0), URI);
    assert_eq!(actions.len(), 2, "partition: two actions, got: {actions:?}");
    // The generic rename uses the editor rename command + the partition
    // identifier as its full path.
    let obj_rename =
        action_titled(&actions, "Rename Tenant_A\u{2026}").expect("object-rename action");
    assert_eq!(
        obj_rename.command.as_ref().unwrap().command,
        "editor.action.rename"
    );
    // The partition rename uses the dedicated command.
    let part_rename = action_titled(&actions, "Rename partition").expect("partition-rename action");
    assert_eq!(
        part_rename.command.as_ref().unwrap().command,
        "tclLsp.renamePartition"
    );
}

// ===========================================================================
// (b) test_no_partition_rename_action_for_common_partition
// ===========================================================================

/// `rename_partition` refuses `/Common`, so the provider does not offer a
/// partition-rename action on the `Common` stanza (the generic object
/// rename is still offered).
#[test]
fn no_partition_rename_action_for_common_partition() {
    let source = "auth partition Common { description default }\nltm pool /Common/web_pool { }\n";
    let actions = bigip_code_actions(source, range_at(0), URI);
    assert!(
        actions
            .iter()
            .all(|a| !a.title.contains("Rename partition")),
        "no partition rename for /Common, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
    // The plain object rename IS still present on the Common stanza.
    assert!(
        action_titled(&actions, "Rename Common\u{2026}").is_some(),
        "object rename should still be offered on the Common partition stanza"
    );
}

// ===========================================================================
// (b) test_no_actions_when_cursor_outside_any_stanza
// ===========================================================================

/// Cursor on a blank line between stanzas → no actions (no stanza range
/// covers that line). Python: `source = "\n\nltm pool /Common/p { }\n"`,
/// cursor line 0 → `[]`.
#[test]
fn no_actions_when_cursor_outside_any_stanza() {
    let source = "\n\nltm pool /Common/p { }\n";
    let actions = bigip_code_actions(source, range_at(0), URI);
    assert!(
        actions.is_empty(),
        "cursor on a blank line must yield no actions, got: {actions:?}"
    );
}

/// Sanity counter-check: the SAME document, with the cursor moved onto the
/// pool's line (line 2), DOES yield the object-rename action — proving the
/// emptiness above is about cursor position, not a parse failure.
#[test]
fn cursor_on_the_stanza_line_does_yield_an_action() {
    let source = "\n\nltm pool /Common/p { }\n";
    let actions = bigip_code_actions(source, range_at(2), URI);
    assert!(
        action_titled(&actions, "Rename /Common/p\u{2026}").is_some(),
        "cursor on the pool line should offer its rename, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

// ===========================================================================
// Extra structural robustness (no Python sibling; guard the new arm)
// ===========================================================================

/// Empty source → no actions, no panic.
#[test]
fn empty_source_yields_no_actions() {
    assert!(bigip_code_actions("", range_at(0), URI).is_empty());
}

/// An out-of-range cursor line (past the end of the document) yields no
/// actions and does not panic.
#[test]
fn out_of_range_cursor_is_safe() {
    let source = "ltm pool /Common/web_pool { }\n";
    let actions = bigip_code_actions(source, range_at(9999), URI);
    assert!(actions.is_empty(), "out-of-range cursor: no actions");
}

/// A nameless global singleton (`auth password-policy { … }`, empty
/// identifier) is not a rename target — there is no path to rename — so it
/// yields no actions even with the cursor on it.
#[test]
fn nameless_singleton_is_not_a_rename_target() {
    let source = "auth password-policy {\n    lockout-duration 10\n}\n";
    let actions = bigip_code_actions(source, range_at(0), URI);
    assert!(
        actions.is_empty(),
        "nameless singleton must not offer a rename, got: {actions:?}"
    );
}

/// A multi-line stanza covers every interior line: cursor on the closing
/// brace line still resolves to the enclosing object.
#[test]
fn cursor_inside_multiline_stanza_resolves_object() {
    let source = "ltm pool /Common/web_pool {\n    members {\n        1.2.3.4:80 { }\n    }\n}\n";
    // Line 4 is the stanza's closing brace.
    let actions = bigip_code_actions(source, range_at(4), URI);
    assert!(
        action_titled(&actions, "Rename /Common/web_pool\u{2026}").is_some(),
        "cursor on the closing-brace line should still resolve the pool, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

// ===========================================================================
// (c) GAPs — run_rename_partition (the query-engine-backed LSP wrapper)
// ===========================================================================
//
// The three Python tests below are NOT ported here. They drive
// `run_rename_partition`, the LSP wrapper that builds a WorkspaceEdit by
// running the `rename_partition(old, new)` query-DSL expression through the
// F5 query engine. That wrapper has no `tcl-lsp-core` home: it would force
// a dependency on the `tcl-bigip-query` crate and reproduce the
// JSON-quote → `run_query` → reparse-guard → WorkspaceEdit glue, which is
// LSP-server territory, not a `code_actions.rs` arm.
//
// The underlying ENGINE is already ported and tested elsewhere:
//   * `tcl-bigip-query/src/builtins/rename.rs::bi_rename_partition` performs
//     the same name validation ("partition names must match [A-Za-z0-9_.-]+")
//     and both-direction `/Common` refusal as the Python builtin.
//   * `f5-cli/tests/query_mutation_parity.rs` covers the cascade rewrite.
//
// GAP: test_run_rename_partition_drives_through_query_engine — the
//      `run_rename_partition` wrapper (DSL build + run_query + reparse guard
//      + single-file WorkspaceEdit) is unported in the LSP layer.
//      Size: small–medium, belongs in tcl-lsp-server (needs a
//      tcl-bigip-query dependency + a `tclLsp.renamePartition`
//      execute_command verb).
//
// GAP: test_run_rename_partition_safely_quotes_query_arguments — same
//      wrapper; the safe JSON-quoting of hostile arguments is part of the
//      unported wrapper (the engine's name validation, which ultimately
//      rejects the injection, is ported in tcl-bigip-query).
//
// GAP: test_run_rename_partition_refuses_common_renames — same wrapper; the
//      `/Common` refusal surfacing as a user error is the wrapper's job
//      (the refusal itself is enforced by the ported engine builtin).
//
// Also unported (parity-audit §3 #8, beyond this pytest's scope, noted for
// completeness): the `tclLsp.renamePartition` execute_command verb in
// tcl-lsp-server that the emitted action's command targets, plus the
// server-side forwarding of `ActionCommand.string_args` into the lifted LSP
// command arguments (the server currently forwards only the integer
// `args`). The provider arm + its command payload are complete and tested
// above; wiring them end-to-end through the server is the remaining step.
