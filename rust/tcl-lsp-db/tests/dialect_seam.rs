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

//! Document grammar and analysis configuration survive the Salsa query boundary.
//!
//! Semantic tokens and folding use the current file's resolved analysis,
//! profile and Registry together. These checks compare plain Tcl with hosted
//! object roles and Expect pattern regions, whose source grammar differs.

use salsa::Setter as _;
use tcl_compiler::analyses::LatticeValue;
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::compiler_checks::DiagCode;
use tcl_lsp_db::{
    AnalyserConfig, Project, SourceFile, TclDatabase, document_compilation_unit_for,
    file_analysis_incremental, folding_ranges, semantic_tokens, semantic_tokens_project,
    set_overlay_epoch,
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
    overlay_config(db, 0)
}

/// A config whose only setting is the workspace's pack overlay.
fn overlay_config(db: &TclDatabase, overlay: u64) -> AnalyserConfig {
    AnalyserConfig::new(
        db,
        Vec::new(),
        tcl_compiler::analyser::NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        overlay,
        Vec::new(),
        Vec::new(),
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
    data.as_chunks::<5>()
        .0
        .iter()
        .filter(|t| t[3] == kind)
        .count()
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
    let cfg = config(&db);

    let fold_lines = |file| {
        let mut v: Vec<(u32, u32)> = folding_ranges(&db, file, cfg)
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

/// A pack command that writes two variables through its argument roles.
const VENDOR_PACK: &str = "speclib vendor 2.0 {\n    \
     command vendor::unpack {\n        \
         arity 1..\n        \
         arg 0 -role Value\n        \
         arg 1 -role VarWrite\n        \
         arg 2 -role VarWrite\n    \
     }\n\
 }\n";

/// What the unit's lattice holds for `a`'s first version in `::p`, when it
/// holds anything: the write `vendor::unpack`'s `VarWrite` role declares is
/// what defines it, so it is there if and only if the unit resolved the
/// pack's command.
fn defined_by_the_pack(unit: &CompilationUnit) -> Option<LatticeValue> {
    let p = unit.procedures.get("::p").expect("::p");
    let symbol = p.ssa.var_symbol("a").expect("`return $a` names it");
    p.sccp.values.get(&(symbol, 1)).cloned()
}

/// A workspace pack's command resolves in the compilation unit the editor's
/// queries share: the unit is built against the overlay's registry, so the
/// variables the command writes are defined. Retire the pack and the unit is
/// rebuilt without it, never kept from the overlay it was built under.
#[test]
fn the_compilation_unit_sees_the_packs_commands() {
    const SRC: &str =
        "proc p {} {\n    set l {1 2 3}\n    vendor::unpack $l a b\n    return $a\n}\n";
    let packs = tcl_spectcl::pack::load_in_memory(vec![(
        tcl_spectcl::PackFile {
            tier: tcl_spectcl::Tier::Workspace,
            path: std::path::PathBuf::from("/workspace/.tcl-lsp/vendor.tclspec"),
            origin: tcl_spectcl::discovery::Origin::DotDir,
            dependency_tier: None,
        },
        VENDOR_PACK.to_owned(),
    )]);
    assert!(packs.notices.is_empty(), "{:#?}", packs.notices);
    assert_ne!(packs.key, 0);
    let _registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl9.0", &packs);

    let mut db = TclDatabase::default();
    let file = SourceFile::new(&db, SRC.to_owned(), "tcl9.0".to_owned(), None);
    let config = overlay_config(&db, packs.key);
    let unit = document_compilation_unit_for(&db, file, config).expect("the pack is installed");
    assert_eq!(
        defined_by_the_pack(&unit),
        Some(LatticeValue::Overdefined),
        "the pack command's `VarWrite` role defines `a`"
    );

    // Retired: no overlay any more. The unit is built again, without the
    // pack's command, and does not keep the overlay it was memoised under.
    config.set_spec_pack_key(&mut db).to(0);
    let unit = document_compilation_unit_for(&db, file, config).expect("no overlay always builds");
    assert_eq!(
        defined_by_the_pack(&unit),
        None,
        "without the pack `vendor::unpack` is an unknown command and defines nothing"
    );

    // And the pack again: keyed by its overlay, the unit sees it once more.
    config.set_spec_pack_key(&mut db).to(packs.key);
    let unit = document_compilation_unit_for(&db, file, config).expect("installed");
    assert_eq!(defined_by_the_pack(&unit), Some(LatticeValue::Overdefined));
}

/// A pack command that only this test's analysis knows: another pack, so
/// another overlay key from the one the unit test installs.
const SPREAD_PACK: &str = "speclib spread 2.0 {\n    \
     command spreadout {\n        \
         arity 1..\n        \
         arg 0 -role Value\n        \
         arg 1 -role VarWrite\n    \
     }\n\
 }\n";

/// The analysis reads the plain registry while the packs are not installed
/// and runs again once they are, with the document untouched: `spreadout`
/// is an unknown command to the analyser until the overlay its key names is
/// installed and the host moves the overlay epoch, and a known one after. The
/// rerun rides on the unit query, which reads the epoch and which the analysis
/// reads.
#[test]
fn the_analysis_reads_the_packs_once_they_install() {
    const SRC: &str = "spreadout {1 2} a\nputs $a\n";
    let packs = tcl_spectcl::pack::load_in_memory(vec![(
        tcl_spectcl::PackFile {
            tier: tcl_spectcl::Tier::Workspace,
            path: std::path::PathBuf::from("/workspace/.tcl-lsp/spread.tclspec"),
            origin: tcl_spectcl::discovery::Origin::DotDir,
            dependency_tier: None,
        },
        SPREAD_PACK.to_owned(),
    )]);
    assert!(packs.notices.is_empty(), "{:#?}", packs.notices);

    let mut db = TclDatabase::default();
    let file = SourceFile::new(&db, SRC.to_owned(), "tcl9.0".to_owned(), None);
    let config = overlay_config(&db, packs.key);
    let unknown = |db: &TclDatabase| {
        file_analysis_incremental(db, file, config)
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W123 && d.message.contains("spreadout"))
    };
    assert!(
        unknown(&db),
        "the packs are keyed but not installed: the plain registry does not know the command"
    );

    let _registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl9.0", &packs);
    // The host installs the packs and moves the overlay epoch; the document
    // and its config are untouched.
    assert!(set_overlay_epoch(&mut db, tcl_registry::overlay_epoch()));
    assert!(
        !unknown(&db),
        "installed, the next revision's analysis knows the pack's command"
    );
}
