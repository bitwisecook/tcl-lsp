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

use super::*;

#[test]
fn empty_links_for_non_source_commands() {
    assert_eq!(
        document_links(
            "set x 1\n",
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            None
        ),
        [] as [crate::document_links::DocumentLink; 0]
    );
    assert_eq!(
        document_links(
            "puts hello\n",
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            None
        ),
        [] as [crate::document_links::DocumentLink; 0]
    );
}

/// Links for a pack at `/home/user/project/.tcl-lsp/mylib.tclspec`, given
/// the full context the server supplies — the include anchor is the
/// document's own path, so `document_links` alone cannot express one.
///
/// `workspace_root` and `home` are populated deliberately: an include that
/// reached for either would resolve to a *different* file, so the tests
/// below can tell the loader's rule from the `source` resolver's.
fn pack_links(src: &str, dialect: &'static tcl_dialect::DialectProfile) -> Vec<DocumentLink> {
    document_links_in_context(
        src,
        dialect,
        &LinkContext {
            imported_constants: None,
            workspace_root: Some("/home/user/project/.tcl-lsp"),
            home: Some("/home/user"),
            script_path: Some("/home/user/project/.tcl-lsp/mylib.tclspec"),
        },
    )
}

#[test]
fn pack_include_navigation_uses_original_wrapped_unicode_names_and_first_pack() {
    let src = "speclib p 2.0 {include {shared α.tclspec}}; speclib q 2.0 {include ignored}";
    let links = pack_links(src, crate::profile_for_dialect("spectcl"));
    assert_eq!(links.len(), 1, "{links:?}");
    assert_eq!(
        links[0].target,
        "file:///home/user/project/.tcl-lsp/shared%20%CE%B1.tclspec"
    );
    let start = u32::try_from(src.find("shared").unwrap()).unwrap();
    let end = start + u32::try_from("shared α.tclspec".len()).unwrap();
    let index = LineIndex::new(src);
    assert_eq!(
        links[0].start_character,
        index.position_at_utf16(start, src).character.get()
    );
    assert_eq!(
        links[0].end_character,
        index.position_at_utf16(end, src).character.get()
    );
}

#[test]
fn pack_include_navigation_withholds_dynamic_and_nested_file_rows() {
    for row in [
        "include $file",
        "include [set file]",
        "command c {include hidden}",
        "include from",
    ] {
        let src = format!("speclib p 2.0 {{{row}}}");
        assert!(
            pack_links(&src, crate::profile_for_dialect("spectcl")).is_empty(),
            "{row}"
        );
    }
}

/// A pack's `include NAME` row links to the sibling file the loader would
/// read, anchored on the name alone.
#[test]
fn a_pack_include_row_links_to_its_sibling_file() {
    let src = "speclib mylib 1 {\n    include shared.tclspec\n}\n";
    let links = pack_links(src, crate::profile_for_dialect("spectcl"));
    assert_eq!(links.len(), 1, "{links:?}");
    assert_eq!(
        links[0].target,
        "file:///home/user/project/.tcl-lsp/shared.tclspec"
    );
    assert_eq!(links[0].start_line, 1);
    assert_eq!(links[0].tooltip.as_deref(), Some("shared.tclspec"));
}

/// The link goes where the loader reads, and the loader does not expand
/// `~`: `include ~` makes it read a sibling literally named `~`, so that
/// is the file the link has to name — not the user's home directory.
#[test]
fn a_pack_include_does_not_expand_a_tilde() {
    let links = pack_links(
        "speclib mylib 1 {\n    include ~\n}\n",
        crate::profile_for_dialect("spectcl"),
    );
    assert_eq!(links.len(), 1, "{links:?}");
    assert_eq!(links[0].target, "file:///home/user/project/.tcl-lsp/~");
}

/// A name carrying path structure is dropped by `loader::include_name`
/// before the resolver sees it, so it names no file to navigate to.
#[test]
fn a_pack_include_with_path_structure_is_not_a_link() {
    for row in ["include ../outside.tclspec", "include sub/shared.tclspec"] {
        let src = format!("speclib mylib 1 {{\n    {row}\n}}\n");
        assert!(
            pack_links(&src, crate::profile_for_dialect("spectcl")).is_empty(),
            "{row}"
        );
    }
}

/// `include from … into …` composes a surface out of loaded packs and
/// names no file, so it must not be offered as one.
#[test]
fn a_surface_roster_include_is_not_a_file_link() {
    let src = "speclib mylib 1 {\n    include from vendor into mylib { a b }\n}\n";
    assert_eq!(
        pack_links(src, crate::profile_for_dialect("spectcl")),
        [] as [crate::document_links::DocumentLink; 0]
    );
}

/// The gate is the document's own grammar: an ordinary Tcl script calling
/// a proc of its own named `include` is not a pack.
#[test]
fn an_ordinary_tcl_include_call_is_not_a_link() {
    let src = "speclib mylib 1 {\n    include shared.tclspec\n}\n";
    assert_eq!(
        pack_links(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        ),
        [] as [crate::document_links::DocumentLink; 0]
    );
}

#[test]
fn absolute_path_surfaces_as_link() {
    let src = "source /usr/lib/tcl/init.tcl\n";
    let links = document_links(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        None,
    );
    assert_eq!(links.len(), 1, "{links:?}");
    assert_eq!(links[0].target, "file:///usr/lib/tcl/init.tcl");
}

pub(super) fn path_analysis(
    source: &str,
    profile: &'static tcl_dialect::DialectProfile,
    config: tcl_lexer::LexerConfig,
) -> tcl_compiler::analyser::AnalysisResult {
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::context_for_profile(profile),
        config,
    );
    tcl_compiler::analyser::Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name)
}

#[test]
fn original_namespace_source_links_use_typed_jim_local_scope_without_export() {
    // naming.navigation.retained-path-source-inventory
    // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
    let source = "namespace eval N {set dir /FIRST; source $dir/a.tcl}; namespace eval N {source $dir/b.tcl}; source $dir/c.tcl";
    let profile = tcl_registry::model::ingress::resolve_environment("jim").analyser_profile();
    assert_eq!(
        tcl_native_model::InvocationDialect::of_profile(profile)
            .authored_name_policy()
            .unwrap()
            .recipe(),
        tcl_syntax::naming::NativeNameProtocol::Jim084
    );
    let analysis = path_analysis(
        source,
        profile,
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    );
    let context = LinkContext {
        workspace_root: Some("/workspace"),
        ..LinkContext::default()
    };
    // Public Native links need current values independently of source advice.
    assert_eq!(
        document_links_from_analysis(source, &analysis, &context),
        [] as [crate::document_links::DocumentLink; 0]
    );
    let inventory =
        tcl_compiler::auto_path_eval::constant_path_assignments_from_analysis(source, &analysis)
            .unwrap();
    let constants = tcl_compiler::auto_path_eval::fold_constant_assignments(&inventory, None);
    assert!(constants.exported().is_empty());
    // This direct helper is explicit source simulation, not public Native lookup.
    let links = lexical_document_links_in_context(source, &analysis, &context);
    assert_eq!(links.len(), 1, "{links:?}");
    assert_eq!(links[0].target, "file:///FIRST/a.tcl");
    let unknown = tcl_dialect::DialectProfile::projected_from_point(
        "source-path-jim079",
        &[],
        "Unknown Jim source path recipe",
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
    )
    .intern();
    let unavailable = path_analysis(
        source,
        unknown,
        tcl_lexer::LexerConfig::for_profile(Some(unknown)),
    );
    assert!(
        tcl_compiler::auto_path_eval::constant_path_assignments_from_analysis(source, &unavailable)
            .is_none()
    );
    assert_eq!(
        lexical_document_links_in_context(source, &unavailable, &context),
        [] as [crate::document_links::DocumentLink; 0]
    );
    assert_eq!(
        document_links_from_analysis(source, &unavailable, &context),
        [] as [crate::document_links::DocumentLink; 0]
    );
}

#[test]
fn original_source_links_use_actual_selected_navigation_and_decline_shadows_or_stale_input() {
    // naming.navigation.retained-path-source-inventory
    // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let context = LinkContext {
        workspace_root: Some("/workspace"),
        ..LinkContext::default()
    };
    for source in [
        "::source /tmp/é.tcl",
        "interp alias {} load {} source; load /tmp/é.tcl",
    ] {
        let analysis = path_analysis(source, profile, config);
        let links = lexical_document_links_in_context(source, &analysis, &context);
        assert_eq!(links.len(), 1, "{source}: {links:?}");
        assert_eq!(links[0].target, "file:///tmp/%C3%A9.tcl");
        assert_eq!(links[0].tooltip.as_deref(), Some("/tmp/é.tcl"));
        assert_eq!(
            lexical_document_links_in_context(&source.replace("é", "x"), &analysis, &context),
            [] as [crate::document_links::DocumentLink; 0]
        );
        let mut stale = analysis.clone();
        stale.body_lexer_config = Some(tcl_lexer::LexerConfig {
            braced_var: tcl_dialect::BracedVarStyle::FirstClose,
            ..config
        });
        assert_eq!(
            lexical_document_links_in_context(source, &stale, &context),
            [] as [crate::document_links::DocumentLink; 0]
        );
    }
    // tclsh 8.4-9.1: `rename source load` fails with "can't rename to
    // \"load\": command already exists", so nothing is sourced.
    for source in [
        "rename source load; load /tmp/é.tcl",
        "proc source {path} {}; source /tmp/a.tcl",
        "proc file {args} {return /BAD}; set d [file join /tmp sub]; source $d/a.tcl",
        "interp alias {} load {} source /tmp/fixed.tcl; load",
    ] {
        let analysis = path_analysis(source, profile, config);
        assert!(
            lexical_document_links_in_context(source, &analysis, &context).is_empty(),
            "{source}"
        );
    }
}

#[test]
fn relative_path_resolves_against_workspace_root() {
    let src = "source helper.tcl\n";
    let links = document_links(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        Some("/home/user/project"),
    );
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].target, "file:///home/user/project/helper.tcl");
}

#[test]
fn relative_path_without_root_produces_no_link() {
    let src = "source helper.tcl\n";
    let links = document_links(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        None,
    );
    assert!(links.is_empty(), "{links:?}");
}

#[test]
fn encoding_flag_skipped_before_path() {
    let src = "source -encoding utf-8 /tmp/foo.tcl\n";
    let links = document_links(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        None,
    );
    assert_eq!(links.len(), 1, "{links:?}");
    assert_eq!(links[0].target, "file:///tmp/foo.tcl");
}

#[test]
fn link_range_anchors_at_path_argument() {
    // `source ` is 7 chars; the path starts at col 7.
    let src = "source /tmp/foo.tcl\n";
    let links = document_links(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        None,
    );
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].start_character, 7);
    // End col covers the path (12 chars: `/tmp/foo.tcl`).
    assert_eq!(links[0].end_character, 19);
}

#[test]
fn braced_and_quoted_path_range_excludes_delimiter() {
    // `source {/tmp/foo.tcl}` must underline the path, not
    // the opening `{` — the range starts at the content, past the delimiter.
    for (src, want_start) in [
        ("source {/tmp/foo.tcl}\n", 8u32), // `source ` = 7, then `{` at 7, content at 8
        ("source \"/tmp/foo.tcl\"\n", 8u32),
    ] {
        let links = document_links(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            None,
        );
        assert_eq!(links.len(), 1, "{src:?} → {links:?}");
        assert_eq!(links[0].target, "file:///tmp/foo.tcl");
        assert_eq!(
            links[0].start_character, want_start,
            "range must start at the path content for {src:?}",
        );
    }
}

#[test]
fn dynamic_path_produces_no_link() {
    // `source $somevar` — variable substitution, not a
    // literal path.  Skipped.
    let src = "source $somevar\n";
    let links = document_links(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        None,
    );
    assert!(links.is_empty(), "{links:?}");
}

// Computed `source` paths, exercised through
// the code path a real editor takes: `workspace_root = Some(...)` *and*
// `script_path = Some(...)`.  The `dynamic_path_produces_no
// _link` test above passes `None` for both, which short-circuits
// `resolve_path` on `let root = workspace_root?` before the
// `carries_substitution`
// gate is reached at all — so it never covered the bug.

/// TP — the positive side nothing pinned before: the corpus idiom
/// `set dir [file dirname [info script]]; source [file join $dir x.tcl]`
/// resolves to the **correct** target, not merely to no bogus one.
///
/// Oracle (tclsh 8.6.16 / 9.0.4): running `/proj/test/caller.tcl` loads
/// `/proj/test/helper.tcl`, since `[info script]` is the sourcing file.
#[test]
fn a_computed_source_path_resolves_to_its_real_target_923_idx41() {
    let src = "set dir [file dirname [info script]]\nsource [file join $dir helper.tcl]\n";
    let links = document_links_in_context(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &LinkContext {
            imported_constants: None,
            workspace_root: Some("/proj"),
            home: None,
            script_path: Some("/proj/test/caller.tcl"),
        },
    );
    assert_eq!(links.len(), 1, "expected exactly one link: {links:?}");
    assert_eq!(
        links[0].target, "file:///proj/test/helper.tcl",
        "the computed path must resolve against `[info script]`'s own \
             directory: {links:?}",
    );
}

/// TN — a shape the evaluator cannot fold, so `dir` cannot be
/// constant-propagated.  The whole expression must abstain rather than
/// fall through to percent-encoding its own raw text into a `file://`
/// URI (`file:///proj/%5Bfile%20join%20$dir%20helper.tcl%5D`).
///
/// `file readlink` is the unmodellable command here: the subset does not
/// model it, and cannot without touching the filesystem.  A `file
/// normalize` wrapper does fold, and
/// `a_normalized_computed_source_path_resolves_775` below pins its
/// target.
#[test]
fn an_unfoldable_computed_source_path_produces_no_link_923_idx41() {
    for src in [
        "set dir [file readlink [file dirname [info script]]]\n\
             source [file join $dir helper.tcl]\n",
        "source [file join $undeclared .. pkgfile.tcl]\n",
        "source $other\n",
    ] {
        let links = document_links_in_context(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            &LinkContext {
                imported_constants: None,
                workspace_root: Some("/proj"),
                home: None,
                script_path: Some("/proj/test/caller.tcl"),
            },
        );
        assert!(
            links.is_empty(),
            "an unresolvable computed path must produce no link at all, \
                 got {links:?} for {src:?}",
        );
    }
}

/// A computed path that folds to a *relative* result is anchored by this
/// provider on `workspace_root`, exactly as the literal path beside it —
/// never on the language server process's own working directory.
///
/// Tcl resolves such a path against the interpreter's run-time working
/// directory (tclsh 8.6.16 / 9.0.4: `cd other && tclsh ../sub/main.tcl`
/// with `set v helper.tcl; source $v` loads `other/helper.tcl`), which is
/// unknowable statically; what must never happen is the two spellings
/// disagreeing, or the answer depending on how the editor was launched.
///
/// The computed spelling is `[file join $dir helper.tcl]` rather than
/// `[file join $v]` (with `set v helper.tcl`) because a substitution must
/// end in a literal word to have anything to anchor its link on, and
/// `[file join $v]` ends in a variable.  The claim under test — a
/// *relative* computed result anchoring on `workspace_root` like the
/// literal beside it — is unaffected, since `dir` folds to `.`.
#[test]
fn a_relative_computed_source_path_anchors_like_the_literal_beside_it_923_idx41() {
    let ctx = LinkContext {
        imported_constants: None,
        workspace_root: Some("/proj"),
        home: None,
        script_path: Some("/proj/test/caller.tcl"),
    };
    let literal = document_links_in_context(
        "source helper.tcl\n",
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &ctx,
    );
    let computed = document_links_in_context(
        "set dir .\nsource [file join $dir helper.tcl]\n",
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &ctx,
    );
    assert_eq!(literal.len(), 1, "{literal:?}");
    assert_eq!(computed.len(), 1, "{computed:?}");
    assert_eq!(
        computed[0].target, literal[0].target,
        "a computed relative path must anchor exactly like the literal one",
    );
    assert_eq!(literal[0].target, "file:///proj/helper.tcl");
}

// A `source` link must never span the code inside a
// command substitution.  The reporter's file is
// georgtree/SpiceGenTcl's `test/arbitaryTest.tcl` shape: `set
// currentDir [file … [info script]]` then `source [file join
// $currentDir <name>.tcl]`.  The argument's own semantic tokens were
// always correct (that is what the earlier fix pinned, in
// `tests/e2e/semantic_tokens.rs`); what the reporter actually sees is
// this link, painted flat over `file join $currentDir <name>.tcl`.

/// The link on a computed path anchors on the file name alone, leaving
/// the rest of the substitution — `file`, `join`, `$currentDir` — free to
/// carry its own highlighting.
#[test]
fn a_computed_source_link_anchors_on_the_file_name_only_775() {
    let src = "set currentDir [file dirname [info script]]\n\
                   source [file join $currentDir esd_pulse_circuit.tcl]\n";
    let links = document_links_in_context(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &LinkContext {
            imported_constants: None,
            workspace_root: Some("/proj/test"),
            home: None,
            script_path: Some("/proj/test/main.tcl"),
        },
    );
    assert_eq!(links.len(), 1, "{links:?}");
    let line = src.lines().nth(1).expect("the source line");
    let covered = &line[links[0].start_character as usize..links[0].end_character as usize];
    assert_eq!(
        covered, "esd_pulse_circuit.tcl",
        "the link must cover the file name, not the substitution: {links:?}",
    );
    // Navigation is kept, not traded away for the highlighting.
    assert_eq!(links[0].target, "file:///proj/test/esd_pulse_circuit.tcl");
}

/// The reporter's file, verbatim in shape: georgtree/SpiceGenTcl's
/// `test/arbitaryTest.tcl` wraps its directory in `file normalize`, which
/// the evaluator now folds, so the link both appears and stays off the
/// code.  Every `source` in that project is spelled this way.
///
/// Oracle (tclsh 8.6.16 / 9.0.4): running `/proj/test/arbitaryTest.tcl`
/// loads `/proj/test/testUtilities.tcl`.
#[test]
fn a_normalized_computed_source_path_resolves_775() {
    let src = "set currentDir [file normalize [file dirname [info script]]]\n\
                   source [file join $currentDir testUtilities.tcl]\n";
    let links = document_links_in_context(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &LinkContext {
            imported_constants: None,
            workspace_root: Some("/proj"),
            home: None,
            script_path: Some("/proj/test/arbitaryTest.tcl"),
        },
    );
    assert_eq!(links.len(), 1, "{links:?}");
    assert_eq!(links[0].target, "file:///proj/test/testUtilities.tcl");
    let line = src.lines().nth(1).expect("the source line");
    assert_eq!(
        &line[links[0].start_character as usize..links[0].end_character as usize],
        "testUtilities.tcl",
        "{links:?}",
    );
}

/// A directory reached through an intermediate resolves like a direct
/// one — georgtree/SpiceGenTcl's own `SpiceGenTcl.tcl` shape, where every
/// one of seventeen `source` lines goes through `$sourceDir`.
///
/// Oracle (tclsh 8.6.16 / 9.0.4): running `/proj/SpiceGenTcl.tcl` loads
/// `/proj/src/generalClasses.tcl`.
#[test]
fn a_chained_computed_source_path_resolves_775() {
    let src = "set dir [file dirname [file normalize [info script]]]\n\
                   set sourceDir [file join $dir src]\n\
                   source [file join $sourceDir generalClasses.tcl]\n\
                   source [file join $sourceDir ngspice netlistParserClassNgspice.tcl]\n";
    let links = document_links_in_context(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &LinkContext {
            imported_constants: None,
            workspace_root: Some("/proj"),
            home: None,
            script_path: Some("/proj/SpiceGenTcl.tcl"),
        },
    );
    let targets: Vec<&str> = links.iter().map(|l| l.target.as_str()).collect();
    assert_eq!(
        targets,
        vec![
            "file:///proj/src/generalClasses.tcl",
            "file:///proj/src/ngspice/netlistParserClassNgspice.tcl",
        ],
        "{links:?}",
    );
    // Still anchored on the file name, chained or not.
    let line = src.lines().nth(2).expect("the first source line");
    assert_eq!(
        &line[links[0].start_character as usize..links[0].end_character as usize],
        "generalClasses.tcl",
    );
}

/// The same narrowing for a fully literal `[file join …]`: the joined
/// path's *name* is the link, not the `file join` call that builds it.
#[test]
fn a_literal_file_join_link_anchors_on_the_file_name_only_775() {
    let src = "source [file join lib helper.tcl]\n";
    let links = document_links(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        Some("/proj"),
    );
    assert_eq!(links.len(), 1, "{links:?}");
    let covered = &src[links[0].start_character as usize..links[0].end_character as usize];
    assert_eq!(covered, "helper.tcl", "{links:?}");
    assert_eq!(links[0].target, "file:///proj/lib/helper.tcl");
}

/// A substitution with no literal word to anchor on emits no link at all,
/// rather than falling back to a range that spans code.
#[test]
fn a_computed_source_path_with_no_literal_word_produces_no_link_775() {
    let src = "set dir [file dirname [info script]]\n\
                   set name helper.tcl\n\
                   source [file join $dir $name]\n";
    let links = document_links_in_context(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &LinkContext {
            imported_constants: None,
            workspace_root: Some("/proj"),
            home: None,
            script_path: Some("/proj/main.tcl"),
        },
    );
    assert!(
        links.is_empty(),
        "no anchor means no link, never a link over code: {links:?}",
    );
}

/// A plain literal path is unaffected by the narrowing — the whole word
/// stays the link, since it is the file name.
#[test]
fn a_literal_source_link_still_covers_the_whole_word_775() {
    let src = "source lib/helper.tcl\n";
    let links = document_links(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        Some("/proj"),
    );
    assert_eq!(links.len(), 1, "{links:?}");
    let covered = &src[links[0].start_character as usize..links[0].end_character as usize];
    assert_eq!(covered, "lib/helper.tcl", "{links:?}");
}

#[test]
fn double_dash_terminator_skipped() {
    // `source` takes no `--`: tclsh 8.4-8.6 raise "wrong # args" and
    // 9.0/9.1 "bad option \"--\": must be -nopkg", so nothing is sourced.
    let src = "source -- /tmp/x.tcl\n";
    let links = document_links(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        None,
    );
    assert!(links.is_empty(), "{links:?}");
}

#[test]
fn trailing_slash_on_workspace_root_handled() {
    let src = "source helper.tcl\n";
    let links = document_links(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        Some("/home/user/"),
    );
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].target, "file:///home/user/helper.tcl");
}

#[test]
fn tilde_expansion_uses_supplied_home() {
    let src = "source ~/lib/init.tcl\n";
    let links = document_links_with_home(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        None,
        Some("/test-home"),
    );
    assert_eq!(links.len(), 1, "{links:?}");
    assert_eq!(links[0].target, "file:///test-home/lib/init.tcl");
}

#[test]
fn tilde_without_home_produces_no_link() {
    let src = "source ~/lib/init.tcl\n";
    let links = document_links_with_home(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        None,
        None,
    );
    assert!(links.is_empty(), "{links:?}");
}

#[test]
fn bare_tilde_expands_to_home() {
    let src = "source ~\n";
    let links = document_links_with_home(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        None,
        Some("/test-home"),
    );
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].target, "file:///test-home");
}

#[test]
fn source_links_keep_shared_host_base_and_literal_parent_path_policy() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    for root in ["", "relative"] {
        let links = document_links_with_home("source helper.tcl\n", profile, Some(root), None);
        assert!(links.is_empty(), "{root}: {links:?}");
    }
    let root = std::env::temp_dir().join("document-link-root");
    let source = "source ./link/../helper.tcl\n";
    let links = document_links_with_home(source, profile, root.to_str(), None);
    assert_eq!(links.len(), 1, "{links:?}");
    assert_eq!(
        links[0].target,
        crate::file_uri::from_path(
            &root.join("./link/../helper.tcl"),
            crate::file_uri::FileUriEscaping::PathCharacters
        )
        .unwrap()
    );
    for home in ["", "relative"] {
        assert_eq!(
            document_links_with_home("source ~/helper.tcl\n", profile, root.to_str(), Some(home)),
            [] as [crate::document_links::DocumentLink; 0]
        );
    }
}

// URI escaping

#[test]
fn percent_encode_path_passes_unreserved_through() {
    // Letters, digits, and the unreserved punctuation set
    // pass through unchanged, as does the path separator.
    assert_eq!(
        percent_encode_path("/usr/local/lib/init.tcl"),
        "/usr/local/lib/init.tcl",
    );
}

#[test]
fn percent_encode_path_escapes_spaces_and_specials() {
    assert_eq!(
        percent_encode_path("/path/with spaces/file.tcl"),
        "/path/with%20spaces/file.tcl",
    );
    // `#` is a fragment delimiter in URIs.
    assert_eq!(
        percent_encode_path("/has/#hash/file.tcl"),
        "/has/%23hash/file.tcl",
    );
    // `%` is the escape introducer — it itself needs escaping.
    assert_eq!(
        percent_encode_path("/has/50%off/file.tcl"),
        "/has/50%25off/file.tcl",
    );
}

#[test]
fn percent_encode_path_escapes_non_ascii() {
    // UTF-8 bytes for `é` are 0xC3 0xA9.
    assert_eq!(
        percent_encode_path("/caf\u{00E9}/x.tcl"),
        "/caf%C3%A9/x.tcl"
    );
}

#[test]
fn file_uri_for_unix_absolute_path() {
    // `/foo/bar.tcl` → `file:///foo/bar.tcl` (one slash
    // from `file://` + the leading `/` of the path).
    assert_eq!(file_uri_for_path("/foo/bar.tcl"), "file:///foo/bar.tcl");
}

#[test]
fn file_uri_for_windows_drive_letter_path() {
    // `C:/Users/me/file.tcl` → `file:///C:/Users/me/file.tcl`
    // — `file_uri_for_path` must insert the third slash
    // so the host component is empty and the path starts
    // at the drive letter.
    assert_eq!(
        file_uri_for_path("C:/Users/me/file.tcl"),
        "file:///C:/Users/me/file.tcl",
    );
}

#[test]
fn file_uri_normalises_windows_backslashes() {
    assert_eq!(
        file_uri_for_path(r"C:\Users\me\file.tcl"),
        "file:///C:/Users/me/file.tcl",
    );
}

#[test]
fn source_with_spaces_in_path_produces_escaped_uri() {
    // End-to-end: a literal path arg containing spaces or
    // hashes surfaces as a properly percent-encoded
    // `file://` URI rather than a raw concatenation.
    let src = "source /path/with spaces.tcl\n";
    let links = document_links(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        None,
    );
    // The literal-path arg may or may not parse cleanly
    // through the segmenter; if it does, the URI must be
    // escaped.  If not, the test is informational only.
    if let Some(link) = links.first() {
        assert!(
            !link.target.contains(' '),
            "URI target must not contain raw spaces: {target}",
            target = link.target,
        );
    }
}

// literal `[file join …]`

#[test]
fn file_join_joins_literal_segments() {
    // `[file join lib core init.tcl]` → `lib/core/init.tcl`.
    assert_eq!(
        tcl_compiler::auto_path_eval::evaluate_auto_path_expr(
            "[file join lib core init.tcl]",
            None
        ),
        Some("lib/core/init.tcl".to_owned()),
    );
}

#[test]
fn file_join_handles_quoted_and_braced_segments() {
    assert_eq!(
        tcl_compiler::auto_path_eval::evaluate_auto_path_expr(
            r#"[file join "lib" {core} init.tcl]"#,
            None
        ),
        Some("lib/core/init.tcl".to_owned()),
    );
}

#[test]
fn file_join_absolute_segment_resets_accumulator() {
    // Per Tcl's `file join` semantics, an absolute path
    // resets the joined accumulator.
    assert_eq!(
        tcl_compiler::auto_path_eval::evaluate_auto_path_expr(
            "[file join /etc /opt/foo bar]",
            None
        ),
        Some("/opt/foo/bar".to_owned()),
    );
}

#[test]
fn file_join_returns_none_for_variable_segments() {
    assert!(
        tcl_compiler::auto_path_eval::evaluate_auto_path_expr("[file join $dir foo]", None)
            .is_none()
    );
    assert!(
        tcl_compiler::auto_path_eval::evaluate_auto_path_expr("[file join [pwd] foo]", None)
            .is_none()
    );
}

#[test]
fn file_join_returns_none_for_non_file_join_subst() {
    assert!(tcl_compiler::auto_path_eval::evaluate_auto_path_expr("[exec ls]", None).is_none());
    assert_eq!(
        tcl_compiler::auto_path_eval::evaluate_auto_path_expr("[file dirname /foo]", None)
            .as_deref(),
        Some("/")
    );
}

#[test]
fn source_with_literal_file_join_surfaces_link() {
    let src = "source [file join lib helper.tcl]\n";
    let links = document_links(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        Some("/home/user/project"),
    );
    assert_eq!(links.len(), 1, "{links:?}");
    assert_eq!(links[0].target, "file:///home/user/project/lib/helper.tcl",);
}

#[test]
fn source_with_absolute_file_join_segment_surfaces_link() {
    let src = "source [file join /usr/local/lib tcl init.tcl]\n";
    let links = document_links(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        None,
    );
    assert_eq!(links.len(), 1, "{links:?}");
    assert_eq!(links[0].target, "file:///usr/local/lib/tcl/init.tcl");
}
