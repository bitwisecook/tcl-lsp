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

//! The document dialect must survive the salsa seam.
//!
//! `semantic_tokens`, `semantic_tokens_project` and `folding_ranges` each read
//! `file.dialect(db)` and resolve it to a **profile** before handing off to
//! `tcl-lsp-core`. Nothing else in this crate's suite discriminated on that
//! resolution: hardcoding the profile argument to plain Tcl at any of the three
//! call sites left the whole `tcl-lsp-db` suite green.
//!
//! Getting a *discriminating* pin here needs care. Each of these queries hands
//! `tcl-lsp-core` a registry alongside the profile, and that registry is built
//! separately from the same `file.dialect(db)`. So the obvious test — tokenise
//! one source under two dialects and assert the answers differ — passes even
//! with the profile pinned to plain Tcl, because the registries still differ
//! and carry the whole difference on their own. Such a test pins the *query's*
//! dialect handling in aggregate but says nothing about the profile argument.
//!
//! The tests below therefore assert on token/fold content that only the
//! profile can produce:
//!
//! - `collect_entries` gates its BIG-IP object-reference pass on
//!   `profile.is_irules()`, so an `object` token on `pool /Common/web_pool`
//!   exists if and only if the profile reached it.
//! - `folding::folding_ranges` reads `dialect.grammar` for the lexer config and
//!   `dialect.name` for the analyser, so an `expect` pattern arm folds only
//!   under the `expect` profile.
//!
//! Every case was mutation-verified by pinning its call site to
//! `profile_for_dialect("tcl")` and confirming the test fails.

use tcl_lsp_db::{
    AnalyserConfig, Project, SourceFile, TclDatabase, folding_ranges, semantic_tokens,
    semantic_tokens_project,
};

/// An `expect` document whose `-brace` pattern list and `default` arm are
/// foldable regions only the `expect` grammar exposes.
const EXPECT_DOC: &str = concat!(
    "expect -brace {\n",          // 0
    "    default {\n",            // 1
    "        return FOLDED\n",    // 2
    "        puts unreachable\n", // 3
    "    }\n",                    // 4
    "}\n",                        // 5
);

/// An iRules document carrying a BIG-IP **object reference**
/// (`pool /Common/web_pool`) — the shape whose tokenisation is gated directly
/// on `profile.is_irules()`.
const IRULES_DOC: &str = concat!(
    "when HTTP_REQUEST {\n",
    "    set u [HTTP::uri]\n",
    "    pool /Common/web_pool\n",
    "    HTTP::respond 200\n",
    "}\n",
);

fn config(db: &TclDatabase) -> AnalyserConfig {
    AnalyserConfig::new(
        db,
        Vec::new(),
        tcl_compiler::analyser::NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
    )
}

/// Index of the `object` token type in the LSP legend.
fn object_token_type() -> u32 {
    u32::try_from(
        tcl_lsp_core::semantic_tokens::legend_token_types()
            .iter()
            .position(|t| *t == "object")
            .expect("the legend must carry an `object` token type"),
    )
    .expect("legend index fits in u32")
}

/// How many tokens of `kind` the packed LSP stream carries.
///
/// The wire form is five `u32` per token — `deltaLine`, `deltaStart`,
/// `length`, `tokenType`, `tokenModifiers` — so the type is every fourth
/// element starting at index 3.
fn count_of_type(data: &[u32], kind: u32) -> usize {
    data.chunks_exact(5).filter(|t| t[3] == kind).count()
}

/// `semantic_tokens` (single-file) must resolve the document's dialect to the
/// profile it hands `tcl-lsp-core`, not just to the registry.
#[test]
fn semantic_tokens_carry_the_profile_across_the_seam() {
    let db = TclDatabase::default();
    let cfg = config(&db);
    let object = object_token_type();

    let irules = SourceFile::new(&db, IRULES_DOC.to_owned(), "f5-irules".to_owned(), None);
    let tokens = semantic_tokens(&db, irules, cfg);

    assert!(
        count_of_type(&tokens.data, object) > 0,
        "`pool /Common/web_pool` must produce a BIG-IP `object` token: that pass \
         is gated on `profile.is_irules()`, so its absence means the profile did \
         not survive the salsa seam"
    );

    // FP guard: the same text under plain Tcl is not an iRules document and
    // must not attract object tokens. Without this, a profile hardcoded to
    // *iRules* would pass the assertion above just as well.
    let plain = SourceFile::new(&db, IRULES_DOC.to_owned(), "tcl8.6".to_owned(), None);
    assert_eq!(
        count_of_type(&semantic_tokens(&db, plain, cfg).data, object),
        0,
        "a plain-Tcl document must not attract BIG-IP object tokens"
    );
}

/// `semantic_tokens_project` reads the dialect through the same shape as
/// `semantic_tokens` but was left unmutated by the adversary sweep, so it gets
/// its own pin rather than riding on its sibling's.
#[test]
fn project_semantic_tokens_carry_the_profile_across_the_seam() {
    let db = TclDatabase::default();
    let cfg = config(&db);
    let object = object_token_type();

    let irules = SourceFile::new(&db, IRULES_DOC.to_owned(), "f5-irules".to_owned(), None);
    let plain = SourceFile::new(&db, IRULES_DOC.to_owned(), "tcl8.6".to_owned(), None);
    let project = Project::new(&db, vec![irules, plain]);

    assert!(
        count_of_type(
            &semantic_tokens_project(&db, irules, project, cfg).data,
            object
        ) > 0,
        "the project-wide token query must carry the profile across the seam too"
    );
    assert_eq!(
        count_of_type(
            &semantic_tokens_project(&db, plain, project, cfg).data,
            object
        ),
        0,
        "a plain-Tcl document must not attract BIG-IP object tokens"
    );
}

/// `folding_ranges` must fold under the document's own dialect — the `expect`
/// `-brace` list and its `default` arm are regions the plain-Tcl grammar
/// cannot see.
#[test]
fn folding_ranges_carry_the_profile_across_the_seam() {
    let db = TclDatabase::default();

    let fold_lines = |file| {
        let mut v: Vec<(u32, u32)> = folding_ranges(&db, file)
            .iter()
            .map(|r| (r.start_line, r.end_line))
            .collect();
        v.sort_unstable();
        v
    };

    let expect = SourceFile::new(&db, EXPECT_DOC.to_owned(), "expect".to_owned(), None);
    let plain = SourceFile::new(&db, EXPECT_DOC.to_owned(), "tcl8.6".to_owned(), None);

    let under_expect = fold_lines(expect);
    let under_plain = fold_lines(plain);

    assert!(
        under_expect.contains(&(0, 4)),
        "the `-brace` pattern list must fold under the `expect` profile: \
         {under_expect:?}"
    );
    // FP guard, and the half that fails if the profile is pinned to `expect`
    // rather than dropped: plain Tcl sees an ordinary command with a braced
    // word and must not produce the arm fold.
    assert!(
        !under_plain.contains(&(0, 4)),
        "plain Tcl must not fold an `expect` pattern list: {under_plain:?}"
    );
}
