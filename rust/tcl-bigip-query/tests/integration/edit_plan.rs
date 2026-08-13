// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Engine-level tests for the edit-plan path (`run_query` mutation): scalar
//! assignments, `|=` transforms, `+=` list materialisation, pool-member field
//! edits, no-op detection, and identity-field rename routing.
//!
//! Self-contained: builds a tiny synthetic SCF config inline and drives the
//! pure `run_query` runner, asserting the rewritten source and the
//! `edits_per_file` / `has_mutation` bookkeeping.

use tcl_bigip_query::{QueryOptions, run_query};

const CONF: &str = "\
ltm pool /Common/p1 {
    monitor /Common/old_mon
    load-balancing-mode round-robin
}

ltm virtual /Common/v1 {
    destination /Common/10.0.0.1:443
    pool /Common/p1
    rules {
        /Common/r1
    }
}
";

fn sources() -> Vec<(String, String)> {
    vec![("file:///test.conf".to_owned(), CONF.to_owned())]
}

fn opts() -> QueryOptions {
    QueryOptions::default()
}

#[test]
fn scalar_assign_rewrites_single_value_span() {
    let result = run_query(
        r#".ltm.pool[] | .monitor = "/Common/new_mon""#,
        &sources(),
        &opts(),
    )
    .expect("run");
    assert!(result.has_mutation);
    let (_, applied) = &result.edits_per_file[0];
    assert_eq!(applied.field_edits, 1);
    assert!(applied.new_source.contains("monitor /Common/new_mon"));
    assert!(!applied.new_source.contains("old_mon"));
    // Untouched lines survive verbatim.
    assert!(
        applied
            .new_source
            .contains("load-balancing-mode round-robin")
    );
}

#[test]
fn pipe_assign_transforms_current_value() {
    let result = run_query(
        r#".ltm.virtual[] | .destination |= sub(., ":[0-9]+$", ":0")"#,
        &sources(),
        &opts(),
    )
    .expect("run");
    let (_, applied) = &result.edits_per_file[0];
    assert!(
        applied
            .new_source
            .contains("destination /Common/10.0.0.1:0")
    );
}

#[test]
fn list_append_materialises_block() {
    let result = run_query(
        r#".ltm.virtual[] | .rules += "/Common/r2""#,
        &sources(),
        &opts(),
    )
    .expect("run");
    let (_, applied) = &result.edits_per_file[0];
    // The existing `rules { ... }` block is overwritten in place.
    assert!(
        applied
            .new_source
            .contains("rules { /Common/r1 /Common/r2 }")
    );
}

#[test]
fn noop_assignment_leaves_source_unchanged() {
    // Assigning the existing value queues an edit but the spliced text is
    // identical — `has_mutation` is still true, `new_source == original`.
    let result = run_query(
        r#".ltm.pool[] | .monitor = "/Common/old_mon""#,
        &sources(),
        &opts(),
    )
    .expect("run");
    assert!(result.has_mutation);
    let (_, applied) = &result.edits_per_file[0];
    assert_eq!(applied.new_source, applied.original);
}

#[test]
fn select_nothing_queues_no_edit() {
    // The predicate drops everything: no edit op queued, not a mutation.
    let result = run_query(
        r#".ltm.pool[] | select(.name == "nope") | .monitor = "x""#,
        &sources(),
        &opts(),
    )
    .expect("run");
    assert!(!result.has_mutation);
    assert!(result.edits_per_file.is_empty());
}

#[test]
fn identity_field_write_routes_through_rename() {
    // `.name = "/Common/p2"` rewrites the pool header *and* the virtual's
    // `pool /Common/p1` reference (token-bounded), and records a RenameReport.
    let result = run_query(r#".ltm.pool[] | .name = "/Common/p2""#, &sources(), &opts())
        .expect("identity rename should succeed");
    assert!(result.has_mutation);
    let (_, applied) = &result.edits_per_file[0];
    assert_eq!(applied.field_edits, 0);
    assert!(applied.new_source.contains("ltm pool /Common/p2 {"));
    assert!(applied.new_source.contains("pool /Common/p2"));
    assert!(!applied.new_source.contains("/Common/p1"));
    assert_eq!(applied.rename_reports.len(), 1);
    let rep = &applied.rename_reports[0];
    assert_eq!(rep.old, "/Common/p1");
    assert_eq!(rep.new, "/Common/p2");
    assert_eq!(rep.occurrences, 2);
}

#[test]
fn identity_field_inc_dec_still_rejected() {
    // `+=` / `-=` on an identity field is nonsensical and stays an error.
    let err = run_query(r#".ltm.pool[] | .name += "x""#, &sources(), &opts())
        .expect_err("identity += must error");
    assert!(
        err.to_string()
            .contains("assignment += to identity field 'name' is not supported"),
        "unexpected error: {err}"
    );
}

#[test]
fn multi_statement_chain_sees_post_edit_source() {
    // The second statement should overwrite the first's result, leaving only
    // the final value in the source.
    let result = run_query(
        r#".ltm.pool[] | .monitor = "/Common/a"; .ltm.pool[] | .monitor = "/Common/b""#,
        &sources(),
        &opts(),
    )
    .expect("run");
    let (_, applied) = &result.edits_per_file[0];
    assert!(applied.new_source.contains("monitor /Common/b"));
    assert!(!applied.new_source.contains("/Common/a"));
    assert_eq!(applied.field_edits, 2);
}
