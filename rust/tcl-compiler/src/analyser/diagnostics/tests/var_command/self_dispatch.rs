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

//! W308 on `[self]` receivers and the bareword `my` dispatch.

use super::*;

// `[self]` / `[self object]` used as a dispatch head (`[self] method`) is
// `TclOO`'s own same-object spelling: it must resolve like `my method` for
// W308 ("unknown method") too, not just for the LSP-layer consumers
// (highlighting, go-to-def, references, code lens). Treating *any*
// self-dispatch/introspection head — `self`'s own included — as an opaque,
// unresolvable-class object handle makes `emit_cmd_command_diagnostics`
// abstain silently, a false negative on invalid code.

#[test]
fn analyse_w308_tp_1324_bare_self_receiver_unknown_method() {
    // `[self] nosuchmethod`.
    let src = "\
oo::class create Test1324A {
    method animTick {} {
        return {}
    }
    method anim {} {
        [self] nosuchmethod
        return {}
    }
}
";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W308 && d.message.contains("nosuchmethod")),
        "W308 expected for `[self] nosuchmethod`; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w308_fp_1324_bare_self_receiver_real_method_silent() {
    // FP guard: `[self] animTick` dispatches to a real method on the
    // enclosing class — must NOT fire W308.
    let src = "\
oo::class create Test1324B {
    method animTick {} {
        return {}
    }
    method anim {} {
        [self] animTick
        return {}
    }
}
";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W308),
        "W308 must not fire for `[self] animTick`, a real method; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w308_tp_1324_self_object_receiver_unknown_method() {
    // `[self object]` is `self`'s explicit-word spelling of the same
    // receiver `[self]` (bare) denotes — must resolve identically.
    let src = "\
oo::class create Test1324C {
    method animTick {} {
        return {}
    }
    method anim {} {
        [self object] nosuchmethod
        return {}
    }
}
";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W308 && d.message.contains("nosuchmethod")),
        "W308 expected for `[self object] nosuchmethod`; got {:?}",
        r.diagnostics,
    );
}

// Bareword `my <method>` is recorded as a dispatch site, so W308 ("unknown
// method") fires for `TclOO`'s commonest same-object spelling; and every
// `CmdCommandSite`- / `VarCommandSite`-anchored diagnostic reports an exact
// span.
//
// The span assertions are deliberately *exact ranges*: these diagnostics fire
// whether or not the span is right, so a test that checked the code alone
// would pass against a span bug.

/// Analyse `src` on both the whole-file and the per-item (incremental) path
/// and return the `(code, text-under-span)` pairs for the dispatch codes,
/// asserting the two paths agree.
///
/// The agreement check is the point: a dispatch symptom can appear only on
/// the per-item path, which is the one the LSP serves from, so a
/// whole-file-only assertion would miss it entirely.
fn dispatch_diags_both_paths(src: &str) -> Vec<(String, String, Span)> {
    let collect = |r: &crate::analyser::types::AnalysisResult| -> Vec<(String, String, Span)> {
        r.diagnostics
            .iter()
            .filter(|d| matches!(d.code, DiagCode::W307 | DiagCode::W308 | DiagCode::E001))
            .map(|d| {
                let text = src
                    .get(d.span.start() as usize..d.span.end() as usize)
                    .unwrap_or("<out of bounds>")
                    .to_string();
                (d.code.as_str().to_string(), text, d.span)
            })
            .collect()
    };
    let whole = collect(&crate::analyser::Analyser::new().analyse(src, "tcl8.6"));
    let per_item = collect(&crate::analyser::Analyser::new().analyse_per_item(src, "tcl8.6"));
    assert_eq!(
        whole, per_item,
        "whole-file and per-item paths must report identical dispatch \
         diagnostics and spans (issue #1330) for:\n{src}"
    );
    whole
}

/// Just the codes, for the presence/absence assertions.
fn dispatch_codes(src: &str) -> Vec<String> {
    dispatch_diags_both_paths(src)
        .into_iter()
        .map(|(code, _, _)| code)
        .collect()
}

/// A class whose only declared members are `animTick` and the method body
/// under test — the ticket's own shape.
fn cls_1329(body: &str) -> String {
    format!(
        "oo::class create Test1329 {{\n    method animTick {{}} {{ return {{}} }}\n    \
         method anim {{}} {{\n{body}    }}\n}}\n"
    )
}

#[test]
fn analyse_w308_tp_1329_bareword_my_unknown_method() {
    // TP — `my nosuchmethod` must fire W308, anchored on
    // the method word alone.
    let src = cls_1329("        my nosuchmethod\n");
    let diags = dispatch_diags_both_paths(&src);
    assert_eq!(
        diags.len(),
        1,
        "exactly one dispatch diagnostic expected; got {diags:?}"
    );
    assert_eq!(diags[0].0, "W308");
    assert_eq!(
        diags[0].1, "nosuchmethod",
        "W308 must underline the method word alone; got {diags:?}"
    );
}

#[test]
fn analyse_w308_tn_1329_bareword_my_real_method_silent() {
    // TN — `my animTick` dispatches a method the enclosing class declares.
    let src = cls_1329("        my animTick\n");
    assert_eq!(
        dispatch_codes(&src).len(),
        0,
        "`my animTick` must be silent; got {:?}",
        dispatch_diags_both_paths(&src),
    );
}

#[test]
fn analyse_w308_fp_1329_my_reaches_unexported_object_builtins() {
    // FP guard — `my` bypasses export filtering, so it reaches `oo::object`'s
    // unexported members. tclsh 9.0.4 and 8.6.16 both list them as
    // `<cloned> destroy eval unknown variable varname`, and `my variable v` is
    // the single most common line in idiomatic TclOO. None may draw W308.
    let src = cls_1329(
        "        my variable v\n        set v [my varname v]\n        my eval {set q 1}\n",
    );
    assert_eq!(
        dispatch_codes(&src).len(),
        0,
        "`my` must reach oo::object's unexported members; got {:?}",
        dispatch_diags_both_paths(&src),
    );
}

#[test]
fn analyse_w308_tp_1329_object_command_cannot_reach_unexported_builtins() {
    // The other side of that rule — it is not "allow `variable`
    // everywhere": the object's own command really does *not* expose it
    // (tclsh 9.0.4: `$obj varname v` -> `unknown method "varname"`), so this
    // stays a true positive.
    let src =
        "oo::class create C {\n    method m {} { return 1 }\n}\nC create obj\nobj varname v\n";
    assert_eq!(
        dispatch_codes(src),
        vec!["W308".to_string()],
        "`$obj varname` is a real error — only `my` reaches it",
    );
}

#[test]
fn analyse_w308_fp_1329_my_outside_any_class_body() {
    // FP guard — no enclosing class means no receiver to check against.
    // A `my` at top level, or in a plain proc, must abstain rather than
    // invent a class.
    for src in [
        "my nosuchmethod\n",
        "proc p {} {\n    my nosuchmethod\n}\n",
        "namespace eval ns {\n    my nosuchmethod\n}\n",
    ] {
        assert_eq!(
            dispatch_codes(src).len(),
            0,
            "`my` outside a class body must abstain; got {:?} for {src:?}",
            dispatch_diags_both_paths(src),
        );
    }
}

#[test]
fn analyse_w308_fp_1329_inherited_and_forwarded_and_unknown() {
    // FP guards for the three ways a method can exist without being declared
    // on the dispatching class itself.
    let inherited = "oo::class create B1329 {\n    method inherited {} {}\n}\n\
         oo::class create D1329 {\n    superclass B1329\n    method go {} {\n        \
         my inherited\n    }\n}\n";
    let forwarded = "oo::class create F1329 {\n    forward fwd my other\n    \
         method other {} {}\n    method go {} {\n        my fwd\n    }\n}\n";
    let unknown_handler = "oo::class create U1329 {\n    method unknown {args} {}\n    \
         method go {} {\n        my anything\n    }\n}\n";
    for src in [inherited, forwarded, unknown_handler] {
        assert_eq!(
            dispatch_codes(src).len(),
            0,
            "a resolvable-by-other-means method must not draw W308; got {:?} for {src:?}",
            dispatch_diags_both_paths(src),
        );
    }
}

#[test]
fn analyse_w308_fp_1329_unprovable_method_set_abstains() {
    // FP guard — a mixin or superclass outside the local index makes the
    // method set a lower bound, never a complete one. W308 must abstain.
    let mixin = "oo::class create M1329 {\n    mixin ::ext::Unindexed\n    \
         method go {} {\n        my whatever\n    }\n}\n";
    let superclass = "oo::class create S1329 {\n    superclass ::ext::Unindexed\n    \
         method go {} {\n        my whatever\n    }\n}\n";
    for src in [mixin, superclass] {
        assert_eq!(
            dispatch_codes(src).len(),
            0,
            "an unprovable method set must abstain; got {:?} for {src:?}",
            dispatch_diags_both_paths(src),
        );
    }
}

#[test]
fn analyse_w308_fp_1329_computed_method_word_abstains() {
    // FP guard — the dispatched name is chosen at run time, so no static
    // method set can contradict it.
    let src = cls_1329("        set m animTick\n        my $m\n        my get$m\n");
    assert_eq!(
        dispatch_codes(&src).len(),
        0,
        "a computed method word must abstain; got {:?}",
        dispatch_diags_both_paths(&src),
    );
}

#[test]
fn analyse_w308_fp_1329_disturbed_self_dispatch_keyword_abstains() {
    // FP guard — if the document shadows, renames or deletes `my`, a bare
    // `my` may not be TclOO's dispatcher at all. Order-independent: the
    // `rename` written *after* the class body still governs, because the
    // method body only runs later.
    let shadowed = format!(
        "proc my {{args}} {{}}\n{}",
        cls_1329("        my nosuchmethod\n")
    );
    let renamed = format!("{}rename my mine\n", cls_1329("        my nosuchmethod\n"));
    let aliased = format!(
        "{}interp alias {{}} my {{}} puts\n",
        cls_1329("        my nosuchmethod\n")
    );
    for src in [shadowed, renamed, aliased] {
        assert_eq!(
            dispatch_codes(&src).len(),
            0,
            "a disturbed `my` must abstain; got {:?} for {src:?}",
            dispatch_diags_both_paths(&src),
        );
    }
}

#[test]
fn analyse_w308_1329_next_and_self_name_no_method() {
    // `next` / `nextto` re-invoke the *currently executing* method and `self`
    // introspects — the registry classifies neither as SelfDispatch, so no
    // word of theirs is validated as a method name.
    let src = "oo::class create N1329 {\n    method go {} {\n        next\n        \
         self class\n        self namespace\n    }\n}\n";
    assert_eq!(
        dispatch_codes(src).len(),
        0,
        "`next` / `self` name no method; got {:?}",
        dispatch_diags_both_paths(src),
    );
}

#[test]
fn analyse_w308_1329_dialect_gate_no_tcloo_before_86() {
    // `my` is TCL86_PLUS registry data, so an 8.4 / 8.5 document has no
    // self-dispatch keyword at all and the site is never recorded.
    let src = cls_1329("        my nosuchmethod\n");
    for dialect in ["tcl8.4", "tcl8.5"] {
        let r = crate::analyser::Analyser::new().analyse(&src, dialect);
        assert!(
            !r.diagnostics.iter().any(|d| d.code == DiagCode::W308),
            "no TclOO under {dialect}, so no W308; got {:?}",
            r.diagnostics,
        );
    }
}

#[test]
fn analyse_w308_1330_cmd_site_spans_are_exact() {
    // Every `CmdCommandSite`-anchored diagnostic, with its exact span: the two
    // head-anchored ones (E001, W307) must cover the closing `]`, and the
    // method-anchored one (W308) must report absolute offsets, not the body
    // fragment's own, on the per-item path.
    let dog = "oo::class create Dog {\n    method bark {} { return {} }\n}\n";

    // W308, method-word anchored, inside a proc body (the per-item shape).
    let in_proc = format!("{dog}proc handler {{}} {{\n    [Dog new] badmethod\n}}\n");
    assert_eq!(
        dispatch_diags_both_paths(&in_proc)
            .into_iter()
            .map(|(c, t, _)| (c, t))
            .collect::<Vec<_>>(),
        vec![("W308".to_string(), "badmethod".to_string())],
    );

    // E001, head anchored: the whole `[Dog new]`, closing bracket included.
    let bare = format!("{dog}[Dog new]\n");
    assert_eq!(
        dispatch_diags_both_paths(&bare)
            .into_iter()
            .map(|(c, t, _)| (c, t))
            .collect::<Vec<_>>(),
        vec![("E001".to_string(), "[Dog new]".to_string())],
    );

    // W307, head anchored, same rule.
    assert_eq!(
        dispatch_diags_both_paths("[getcmd] doit\n")
            .into_iter()
            .map(|(c, t, _)| (c, t))
            .collect::<Vec<_>>(),
        vec![("W307".to_string(), "[getcmd]".to_string())],
    );
}

#[test]
fn analyse_w308_1330_quick_fix_span_matches_the_diagnostic() {
    // The "did you mean" fix rewrites `method_span`. Un-rebased, it would
    // splice the replacement into unrelated bytes elsewhere in the file —
    // a silent document corruption, not just a misplaced squiggle. Pin the
    // fix span to the diagnostic's own on the per-item path.
    let src = "oo::class create Dog {\n    method bark {} { return {} }\n}\n\
         proc handler {} {\n    [Dog new] bakr\n}\n";
    let r = crate::analyser::Analyser::new().analyse_per_item(src, "tcl8.6");
    let d = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W308)
        .unwrap_or_else(|| panic!("W308 expected; got {:?}", r.diagnostics));
    let fix = d
        .fixes
        .first()
        .unwrap_or_else(|| panic!("a `did you mean` fix expected; got {d:?}"));
    assert_eq!(fix.span, d.span, "fix must rewrite the word it points at");
    assert_eq!(
        &src[fix.span.start() as usize..fix.span.end() as usize],
        "bakr"
    );
    assert_eq!(fix.new_text, "bark");
}
