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

//! W135 and W136: a command, option, subcommand or argument value below the required floor, and its lifecycle.

use super::*;

#[test]
fn original_lifecycle_gates_keep_alias_operands_and_typed_source_subjects() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    // naming.source.authored-command-transition-advice
    // docs/design/analysis/name-resolution-proofs/authored-command-transition-advice.md
    // These assertions cover source selection and diagnostic coordinates;
    // they do not establish runtime alias installation or widget creation.
    for source in [
        "package require Tk 8.6\ninterp alias {} createEntry {} entry .e\ncreateEntry -placeholder text\n",
        "package require Tk 8.6\nrename entry createEntry\ncreateEntry .e -placeholder text\n",
    ] {
        let result = Analyser::new().analyse(source, "tcl8.6");
        let found = result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::W136)
            .collect::<Vec<_>>();
        assert_eq!(found.len(), 1, "{source}: {found:?}");
        let subject = found[0]
            .registry_source()
            .expect("typed Registry lifecycle ownership");
        assert_eq!(
            subject.kind(),
            crate::analyser::RegistrySourceDiagnosticKind::Lifecycle
        );
        assert_eq!(subject.argument(), Some(1));
        assert_eq!(&source[subject.span().as_range()], "-placeholder");
    }
    for source in [
        "package require Tk 8.6\nproc entry args {}; entry .e -placeholder text\n",
        "package require Tk 8.6\ninterp alias {} createEntry {} entry .e -placeholder text\ncreateEntry\n",
        "package require Tk 8.6\nentry .e $flag text -placeholder text\n",
    ] {
        assert!(
            !Analyser::new()
                .analyse(source, "tcl8.6")
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::W136),
            "{source}"
        );
    }
}

#[test]
fn option_value_that_looks_like_a_flag_is_not_retested() {
    // `-placeholder`'s value is itself `-placeholder`. The
    // value word must be skipped, so exactly ONE W136 fires (the option),
    // not two (the option plus its value re-tested as an option).
    let src = "package require Tk 8.6\nentry .e -placeholder -placeholder\n";
    assert_eq!(count(src, "W136"), 1, "{:?}", version_diags(src));
}

#[test]
fn value_word_of_ungated_option_draws_no_spurious_diagnostic() {
    // `-textvariable` consumes the next word `-placeholder` as its VALUE, so
    // `-placeholder` must not be tested as a (gated) option — no W136.
    let src = "package require Tk 8.6\nentry .e -textvariable -placeholder\n";
    assert_eq!(count(src, "W136"), 0, "{:?}", version_diags(src));
}

#[test]
fn option_below_floor_fires_w136() {
    // `-placeholder` needs Tk 8.7; the require guarantees only 8.6.
    let src = "package require Tk 8.6\nentry .e -placeholder hi\n";
    assert!(fires(src, "W136"), "{:?}", version_diags(src));
}

#[test]
fn option_met_by_floor_is_silent() {
    let src = "package require Tk 8.7\nentry .e -placeholder hi\n";
    assert!(!fires(src, "W136"), "{:?}", version_diags(src));
}

#[test]
fn ungated_option_is_silent() {
    // `-width` carries no `min_version`, so no version diagnostic.
    let src = "package require Tk 8.6\nentry .e -width 10\n";
    assert!(!fires(src, "W136"), "{:?}", version_diags(src));
}

#[test]
fn require_without_version_is_permissive_for_unpinned_packages() {
    // No version floor to compare against ⇒ nothing flagged. This is
    // the contract for a package the profile does NOT pin — under the
    // permissive fallback profile (`"tcl"`, no library pins) an
    // unversioned require yields no floor. On a pinned host the
    // shipped version floors it instead (§7.1 — see
    // `tracksbase_tk_pin_floors_an_unversioned_require`).
    let src = "package require Tk\nentry .e -placeholder hi\n";
    assert_eq!(
        version_diags_for(src, "tcl", None).len(),
        0,
        "{:?}",
        version_diags_for(src, "tcl", None)
    );
}

#[test]
fn no_require_draws_no_version_diagnostic() {
    // Missing `package require` is W120's job, not a version diagnostic.
    let src = "entry .e -placeholder hi\n";
    assert!(!fires(src, "W136"), "{:?}", version_diags(src));
}

#[test]
fn command_below_floor_fires_w135() {
    // `ttk::button` needs Tk 8.5. On a tcl8.4 host the shipped Tk is
    // 8.4 (TracksBase, §7.1) and a `require Tk 8.4` cannot raise it — so
    // the floor is 8.4 either way and W135 fires.
    //
    // The floors are equal, so which one the message *names* is the
    // reporting tie-break, and it names the require: of the ways a floor
    // can arise, the line in this file is the one whose author is reading
    // the diagnostic.  The tie-break is explicit — see `FloorSource` — and
    // the file's own require leads it.
    let src = "package require Tk 8.4\nttk::button .b\n";
    let diags = version_diags_for(src, "tcl8.4", None);
    assert!(
        diags
            .iter()
            .any(|(c, m)| c == "W135" && m.contains("`package require` guarantees only 8.4")),
        "{diags:?}"
    );
    // On a tcl8.6 host the same source is FINE: `package require Tk
    // 8.4` states a minimum, it does not downgrade the shipped Tk 8.6 —
    // the old require-only floor drew a false positive here.
    assert_eq!(
        version_diags_for(src, "tcl8.6", None).len(),
        0,
        "the shipped Tk 8.6 satisfies an 8.5 introduction"
    );
}

#[test]
fn command_met_by_floor_is_silent() {
    let src = "package require Tk 8.6\nttk::button .b\n";
    assert!(!fires(src, "W135"), "{:?}", version_diags(src));
}

#[test]
fn conditional_probe_does_not_raise_floor() {
    // An optional `catch {package require Tk 8.7}` does not guarantee 8.7;
    // the guaranteed floor is the unconditional 8.6, so W136 still fires.
    let src = "package require Tk 8.6\n\
                   catch {package require Tk 8.7}\n\
                   entry .e -placeholder hi\n";
    assert!(fires(src, "W136"), "{:?}", version_diags(src));
}

/// Every version-gate code for `source` under `dialect`, including the
/// retirement (W139) and deprecation (W144) rungs the narrower helpers
/// above filter out.
fn lifecycle_diags(source: &str, dialect: &str) -> Vec<(String, String)> {
    Analyser::new()
        .analyse(source, dialect)
        .diagnostics
        .iter()
        .filter(|d| matches!(d.code.as_str(), "W135" | "W136" | "W139" | "W144"))
        .map(|d| (d.code.to_string(), d.message.clone()))
        .collect()
}

#[test]
fn command_level_argument_value_follows_the_core_floor() {
    // `close chan ?read|write?` — the half-close direction word is TIP
    // 332, Tcl 8.6.  It sits at a command-level `arg_values` position
    // that is deliberately *not* a closed value set, so W137 never sees
    // it; the command-level version gate is what reports it.
    let src = "close $s read\n";
    for dialect in ["tcl8.4", "tcl8.5"] {
        let diags = lifecycle_diags(src, dialect);
        assert!(
            diags.iter().any(|(code, message)| code == "W135"
                && message.contains("Argument value 'read' on 'close'")
                && message.contains("requires Tcl 8.6")),
            "{dialect}: half-close needs 8.6, got {diags:?}"
        );
    }
    for dialect in ["tcl8.6", "tcl9.0"] {
        assert_eq!(
            lifecycle_diags(src, dialect).len(),
            0,
            "{dialect}: half-close is real from 8.6"
        );
    }
    // FP-guards: the plain one-argument form has no direction word at
    // all, and a dynamic direction is not a literal value.
    assert_eq!(
        lifecycle_diags("close $s\n", "tcl8.5"),
        [] as [(std::string::String, std::string::String); 0]
    );
    assert_eq!(
        lifecycle_diags("close $s $dir\n", "tcl8.5"),
        [] as [(std::string::String, std::string::String); 0]
    );
}

#[test]
fn sub_subcommand_follows_the_core_floor() {
    // `info object properties` / `info class definitionnamespace` are the
    // 9.0 TclOO introspection additions (TIP 558 / TIP 524).
    for src in [
        // The operands are bound: unset, `$o` raises before the subcommand
        // is looked up (tclsh 8.6.18 / 9.0.4).
        "set o [oo::object new]\ninfo object properties $o\n",
        "set c [oo::class create K]\ninfo class definitionnamespace $c\n",
    ] {
        let diags = lifecycle_diags(src, "tcl8.6");
        assert!(
            diags
                .iter()
                .any(|(code, message)| code == "W135" && message.contains("requires Tcl 9.0")),
            "8.6 must reject {src:?}: {diags:?}"
        );
        assert_eq!(
            lifecycle_diags(src, "tcl9.0").len(),
            0,
            "9.0 must admit {src:?}"
        );
    }
    // The 8.6 operations of the same ensembles stay silent, and a unique
    // prefix of a gated operation is gated exactly as the full word is.
    assert_eq!(
        lifecycle_diags("set o [oo::object new]\ninfo object class $o\n", "tcl8.6"),
        [] as [(std::string::String, std::string::String); 0]
    );
    assert!(
        lifecycle_diags(
            "set c [oo::class create K]\ninfo class definitionn $c\n",
            "tcl8.6"
        )
        .iter()
        .any(|(code, _)| code == "W135"),
        "an abbreviation resolves to the same gated operation"
    );
}

#[test]
fn deprecated_subcommand_reports_once_on_the_axis_that_still_has_it() {
    // `trace variable` is deprecated from 8.4 and removed in 9.0.
    let src = "trace variable v w handler\n";
    let diags = lifecycle_diags(src, "tcl8.6");
    assert!(
        diags.iter().any(|(code, message)| code == "W144"
            && message.contains("Subcommand 'variable' on 'trace'")
            && message.contains("deprecated as of Tcl 8.4")),
        "8.6 still has the legacy form, deprecated: {diags:?}"
    );
    // On 9.0 the form is gone from the profile entirely — that is W002's
    // word, and the version gate must not flag it a second time.
    assert_eq!(
        lifecycle_diags(src, "tcl9.0").len(),
        0,
        "a dialect-absent subcommand stays W002's alone"
    );
}

#[test]
fn a_require_range_that_reaches_a_retirement_is_hedged() {
    // The floor (8.6) still has `trace variable`, but the range admits
    // every version up to 9.1 — including the 9.0 that removed it.
    let src = "package require Tcl 8.5-9.1\ntrace variable v w handler\n";
    let diags = lifecycle_diags(src, "tcl8.6");
    assert!(
        diags.iter().any(|(code, message)| code == "W139"
            && message.contains("not available in every version satisfying requirement `8.5-9.1`")
            && message.contains("removed in Tcl 9.0")),
        "a straddling range is hedged, not asserted: {diags:?}"
    );
    // A range that stops below the retirement keeps the ordinary
    // floor verdict …
    let inside = "package require Tcl 8.5-8.7\ntrace variable v w handler\n";
    assert!(
        lifecycle_diags(inside, "tcl8.6")
            .iter()
            .all(|(code, _)| code == "W144"),
        "a range inside the window reports only the deprecation"
    );
    // … and so does an open-ended or bare requirement, which states no
    // ceiling at all.
    for req in ["8.5-", "8.5"] {
        let src = format!("package require Tcl {req}\ntrace variable v w handler\n");
        assert!(
            lifecycle_diags(&src, "tcl8.6")
                .iter()
                .all(|(code, _)| code == "W144"),
            "`{req}` states no ceiling"
        );
    }
    // A conditional probe guarantees nothing, so it cannot widen the
    // window either.
    let probed = "package require Tcl 8.5\n\
                      catch {package require Tcl 8.5-9.1}\n\
                      trace variable v w handler\n";
    assert!(
        lifecycle_diags(probed, "tcl8.6")
            .iter()
            .all(|(code, _)| code == "W144"),
        "a conditional require is not a guarantee"
    );
}
