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

use tcl_compiler::analyser::Analyser;

/// A fully qualified original array root retains its namespace naming
/// geometry independently of an unknown index. Readonly alias templates
/// relate the local declaration and lappend operand in the same genuine
/// procedure body. The info-exists operand and sibling lexical read are
/// independent original root anchors; no entered link or target element
/// is established by this navigation result.
#[test]
fn a_fully_qualified_upvar_target_navigates_from_word_and_read() {
    // Implementation contract: naming.variable.original-alias-source-template
    // docs/design/analysis/name-resolution-proofs/original-alias-source-template.md
    let src = "\
proc SetFocusGrab {grab focus} {
    set index \"$grab,$focus\"
    upvar ::tk::FocusGrab($index) data
    lappend data one
}
proc RestoreFocusGrab {grab focus} {
    set index \"$grab,$focus\"
    if {[info exists ::tk::FocusGrab($index)]} {
        set data2 $::tk::FocusGrab($index)
        unset ::tk::FocusGrab($index)
    }
}
";
    let analysis = Analyser::new().analyse(src, "tcl9.0").clone();
    for (label, needle, extra) in [
        ("upvar-othervar", "upvar ::tk::FocusGrab", 8usize),
        ("read", "$::tk::FocusGrab($index)", 3usize),
    ] {
        let off = src.find(needle).unwrap() + extra;
        let line = u32::try_from(src[..off].matches('\n').count()).unwrap();
        let line_start = src[..off].rfind('\n').map_or(0, |i| i + 1);
        let col = u32::try_from(off - line_start).unwrap();
        let hover = crate::hover::hover(
            src,
            line,
            col,
            &analysis,
            Some(tcl_registry::model::ingress::static_context_for("tcl9.0").commands()),
        )
        .unwrap_or_else(|| panic!("{label}: expected a hover"));
        assert!(
            hover.value.contains("::tk::FocusGrab"),
            "{label}: the card must name the cell: {}",
            hover.value
        );
        let defs = crate::definition::definition(src, line, col, &analysis);
        assert_eq!(defs.len(), 1, "{label}: one definition: {defs:?}");
        let refs = crate::references::references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            line,
            col,
            &analysis,
            true,
        );
        let spans: Vec<(u32, u32)> = refs
            .iter()
            .map(|r| (r.start_line, r.start_character))
            .collect();
        assert_eq!(
            spans,
            vec![(2, 10), (2, 34), (3, 12), (7, 21), (8, 18)],
            "{label}: three genuine namespace-root anchors and two readonly local alias-name templates: {refs:?}"
        );
    }
}

/// The idx-58 shape: `$dataset` is created by the callee's `upvar`, and a
/// sibling `TclOO` **method** happens to share the name.
const SRC: &str = "\
proc gridlayoutHasDataSetObj {dts} {
    upvar 1 $dts dataset
    set dataset MY-SHARED-DATASET
}
oo::class create chart {
    constructor {} {
        gridlayoutHasDataSetObj dataset
        set _dataset $dataset
    }
    method dataset {} { return 1 }
}
";

fn analysis() -> tcl_compiler::analyser::AnalysisResult {
    Analyser::new().analyse(SRC, "tcl9.0").clone()
}

/// Line/character of the `dataset` inside `set _dataset $dataset`.
fn read_position() -> (u32, u32) {
    let line = SRC
        .lines()
        .position(|l| l.contains("set _dataset $dataset"))
        .expect("read line present");
    let col = SRC.lines().nth(line).unwrap().find("$dataset").unwrap() + 2;
    (u32::try_from(line).unwrap(), u32::try_from(col).unwrap())
}

/// Line/character of the bare `dataset` argument at the call site.
fn call_site_position() -> (u32, u32) {
    let line = SRC
        .lines()
        .position(|l| l.contains("gridlayoutHasDataSetObj dataset"))
        .expect("call line present");
    let text = SRC.lines().nth(line).unwrap();
    let head = "gridlayoutHasDataSetObj ";
    let col = text.find(head).unwrap() + head.len() + 2;
    (u32::try_from(line).unwrap(), u32::try_from(col).unwrap())
}

/// TP + the idx-58 FP fix in one: hover on the caller-frame read names the
/// variable and its creating callee — and, critically, never renders the
/// same-named **method**'s card.
#[test]
fn hover_on_a_caller_frame_read_names_the_creating_call() {
    let analysis = analysis();
    let (line, character) = read_position();
    let hover = crate::hover::hover(
        SRC,
        line,
        character,
        &analysis,
        Some(tcl_registry::model::ingress::static_context_for("tcl9.0").commands()),
    )
    .expect("caller-frame hover");
    assert!(
        hover.value.contains("Caller-frame variable"),
        "expected a caller-frame card: {}",
        hover.value
    );
    assert!(
        hover.value.contains("gridlayoutHasDataSetObj"),
        "the card must name the creating callee: {}",
        hover.value
    );
    assert!(
        !hover.value.contains("method"),
        "a `$`-led read must never resolve to a same-named method: {}",
        hover.value
    );
}

/// Go-to-definition reaches the call-site word that creates the variable —
/// the nearest thing the frame has to a declaration.
#[test]
fn definition_on_a_caller_frame_read_reaches_the_call_site_word() {
    let analysis = analysis();
    let (line, character) = read_position();
    let locs = crate::definition::definition(SRC, line, character, &analysis);
    assert_eq!(locs.len(), 1, "one definition: {locs:?}");
    let (call_line, call_col) = call_site_position();
    assert_eq!(locs[0].start_line, call_line, "{locs:?}");
    assert!(
        locs[0].start_character <= call_col && locs[0].end_character >= call_col,
        "definition must cover the call-site word: {locs:?}"
    );
}

/// Find-All-References links both halves of the idiom.
#[test]
fn references_link_the_call_site_word_and_the_read() {
    let analysis = analysis();
    let (line, character) = read_position();
    let refs = crate::references::references(
        SRC,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        line,
        character,
        &analysis,
        true,
    );
    assert_eq!(refs.len(), 2, "call-site word + read: {refs:?}");
    // …and from the bare call-site word too.
    let (cl, cc) = call_site_position();
    let from_call = crate::references::references(
        SRC,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        cl,
        cc,
        &analysis,
        true,
    );
    assert_eq!(from_call, refs, "both anchors give one reference set");
}

/// **Finding 1 at provider level.** With the callee's `upvar` aimed
/// anywhere but the caller's frame, hover / go-to-definition /
/// find-references must all abstain — not navigate a variable this frame
/// never gains.  (`upvar 0` aliases the callee's own frame, `#0` the
/// global one, `2` the caller's caller; pinned identical on tclsh 9.0.4
/// and 8.6.14.)
#[test]
fn a_non_caller_frame_upvar_level_draws_no_navigation() {
    for level in ["0", "\\#0", "2"] {
        let src = format!(
            "proc setdef {{d}} {{ upvar {level} $d dst; set dst 1 }}\n\
                 proc caller {{}} {{\n    setdef shared\n    puts $shared\n}}\n"
        );
        let analysis = Analyser::new().analyse(&src, "tcl9.0").clone();
        let line = u32::try_from(
            src.lines()
                .position(|l| l.contains("puts $shared"))
                .unwrap(),
        )
        .unwrap();
        let col = u32::try_from(
            src.lines()
                .nth(line as usize)
                .unwrap()
                .find("$shared")
                .unwrap()
                + 2,
        )
        .unwrap();
        assert!(
            crate::hover::hover(
                &src,
                line,
                col,
                &analysis,
                Some(tcl_registry::model::ingress::static_context_for("tcl9.0").commands())
            )
            .is_none(),
            "`upvar {level}` must draw no caller-frame hover"
        );
        assert!(
            crate::references::references(
                &src,
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
                line,
                col,
                &analysis,
                true
            )
            .is_empty(),
            "`upvar {level}` must report no references"
        );
        assert!(
            crate::definition::definition(&src, line, col, &analysis).is_empty(),
            "`upvar {level}` must report no definition"
        );
    }
}

/// **Finding 4.** `include_declaration = false` drops the *creating* call
/// site only.  A call whose callee upvar-**reads** through the alias
/// (`peek x`) creates nothing — tclsh 9.0.4 / 8.6.14 both report `info
/// exists` = 0 in the caller after `peek w` alone — so that site is an
/// ordinary reference and must survive.
#[test]
fn read_only_call_sites_survive_without_declarations() {
    let src = "\
proc make {d} { upvar 1 $d dst; set dst SET }
proc peek {d} { upvar 1 $d src; return $src }
proc caller {} {
    make shared
    peek shared
    puts $shared
}
";
    let analysis = Analyser::new().analyse(src, "tcl9.0").clone();
    let line = u32::try_from(
        src.lines()
            .position(|l| l.contains("puts $shared"))
            .unwrap(),
    )
    .unwrap();
    let col = u32::try_from(
        src.lines()
            .nth(line as usize)
            .unwrap()
            .find("$shared")
            .unwrap()
            + 2,
    )
    .unwrap();
    let with_decls = crate::references::references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        line,
        col,
        &analysis,
        true,
    );
    assert_eq!(
        with_decls.len(),
        3,
        "make + peek + the read: {with_decls:?}"
    );
    let without = crate::references::references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        line,
        col,
        &analysis,
        false,
    );
    assert_eq!(
        without.len(),
        2,
        "the writer `make shared` is the declaration and drops out; the \
             read-only `peek shared` is a reference and stays: {without:?}"
    );
    let make_line =
        u32::try_from(src.lines().position(|l| l.contains("make shared")).unwrap()).unwrap();
    assert!(
        !without.iter().any(|r| r.start_line == make_line),
        "the creating call site must be excluded: {without:?}"
    );
    let peek_line =
        u32::try_from(src.lines().position(|l| l.contains("peek shared")).unwrap()).unwrap();
    assert!(
        without.iter().any(|r| r.start_line == peek_line),
        "the read-only call site must be retained: {without:?}"
    );
}

/// The literal target at provider level: a
/// callee that binds `upvar name name` creates `name` in this frame,
/// with no call-site word to point at — hover / go-to-definition /
/// find-references must answer from the call itself rather than
/// silently miss it.
#[test]
fn navigation_resolves_a_literal_upvar_target_to_the_creating_call() {
    let src = "\
proc NameProcess {arguments object} {
    upvar name name
    set name W1
}
proc build {} {
    NameProcess x obj
    set out $name
}
";
    let analysis = Analyser::new().analyse(src, "tcl9.0").clone();
    let line = u32::try_from(
        src.lines()
            .position(|l| l.contains("set out $name"))
            .unwrap(),
    )
    .unwrap();
    let col = u32::try_from(
        src.lines()
            .nth(line as usize)
            .unwrap()
            .find("$name")
            .unwrap()
            + 2,
    )
    .unwrap();
    let hover = crate::hover::hover(
        src,
        line,
        col,
        &analysis,
        Some(tcl_registry::model::ingress::static_context_for("tcl9.0").commands()),
    )
    .expect("caller-frame hover for a literal target");
    assert!(
        hover.value.contains("Caller-frame variable"),
        "expected a caller-frame card: {}",
        hover.value
    );
    assert!(
        hover.value.contains("NameProcess"),
        "the card must name the creating callee: {}",
        hover.value
    );
    let locs = crate::definition::definition(src, line, col, &analysis);
    assert_eq!(locs.len(), 1, "one definition: {locs:?}");
    let call_line = u32::try_from(
        src.lines()
            .position(|l| l.contains("NameProcess x obj"))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        locs[0].start_line, call_line,
        "definition must reach the creating call: {locs:?}"
    );
    let refs = crate::references::references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        line,
        col,
        &analysis,
        true,
    );
    assert!(
        refs.iter().any(|r| r.start_line == call_line),
        "the creating call is part of the reference set: {refs:?}"
    );
    assert!(
        refs.iter().any(|r| r.start_line == line),
        "the read is part of the reference set: {refs:?}"
    );
}

/// A bare cursor on the callee's command word keeps resolving as the
/// **command**, never as the variable its literal `upvar` creates.
#[test]
fn the_call_head_word_still_navigates_as_a_command() {
    let src = "\
proc NameProcess {arguments object} {
    upvar name name
    set name W1
}
proc build {} {
    NameProcess x obj
    set out $name
}
";
    let analysis = Analyser::new().analyse(src, "tcl9.0").clone();
    let call_line = u32::try_from(
        src.lines()
            .position(|l| l.contains("NameProcess x obj"))
            .unwrap(),
    )
    .unwrap();
    let col = u32::try_from(
        src.lines()
            .nth(call_line as usize)
            .unwrap()
            .find("NameProcess")
            .unwrap()
            + 2,
    )
    .unwrap();
    let locs = crate::definition::definition(src, call_line, col, &analysis);
    assert_eq!(locs.len(), 1, "one definition: {locs:?}");
    let def_line = u32::try_from(
        src.lines()
            .position(|l| l.contains("proc NameProcess"))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        locs[0].start_line, def_line,
        "the command word must reach the proc definition: {locs:?}"
    );
}

/// TN — a `$`-led read that nothing binds abstains rather than falling
/// through to a same-named method (the idx-58 wrong-kind conflation).
#[test]
fn an_unbound_dollar_read_never_resolves_to_a_same_named_method() {
    let src = "\
oo::class create widget {
    constructor {} { puts $thing }
    method thing {} { return 1 }
}
";
    let analysis = Analyser::new().analyse(src, "tcl9.0").clone();
    let line = u32::try_from(src.lines().position(|l| l.contains("puts $thing")).unwrap()).unwrap();
    let col = u32::try_from(
        src.lines()
            .nth(line as usize)
            .unwrap()
            .find("$thing")
            .unwrap()
            + 2,
    )
    .unwrap();
    assert!(
        crate::hover::hover(
            src,
            line,
            col,
            &analysis,
            Some(tcl_registry::model::ingress::static_context_for("tcl9.0").commands())
        )
        .is_none(),
        "an unbound `$`-led read must draw no hover at all"
    );
    assert!(
        crate::references::references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            line,
            col,
            &analysis,
            true
        )
        .is_empty(),
        "an unbound `$`-led read must report no references"
    );
    assert!(
        crate::definition::definition(src, line, col, &analysis).is_empty(),
        "an unbound `$`-led read must report no definition"
    );
}
