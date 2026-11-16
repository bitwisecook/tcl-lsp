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

use super::super::super::state::Analyser;

mod command_and_option_gates;
mod dsl_gates;
mod floor_sources;
mod versioned_arity;

/// `(code, message)` pairs for the version-gate codes only.
fn version_diags(source: &str) -> Vec<(String, String)> {
    Analyser::new()
        .analyse(source, "tcl8.6")
        .diagnostics
        .iter()
        .filter(|d| matches!(d.code.as_str(), "W135" | "W136"))
        .map(|d| (d.code.to_string(), d.message.clone()))
        .collect()
}

fn fires(source: &str, code: &str) -> bool {
    version_diags(source).iter().any(|(c, _)| c == code)
}

fn count(source: &str, code: &str) -> usize {
    version_diags(source)
        .iter()
        .filter(|(c, _)| c == code)
        .count()
}

/// `(code, message)` version-gate pairs for an arbitrary dialect with
/// optional keyed library-version pins (§7.1 axis C).
fn version_diags_for(
    source: &str,
    dialect: &str,
    bigip_version: Option<&str>,
) -> Vec<(String, String)> {
    let mut a = Analyser::new();
    a.library_versions.bigip_version = bigip_version.map(str::to_owned);
    a.analyse(source, dialect)
        .diagnostics
        .iter()
        .filter(|d| matches!(d.code.as_str(), "W135" | "W136" | "W139"))
        .map(|d| (d.code.to_string(), d.message.clone()))
        .collect()
}

/// A synthetic package whose two options conflict **only from 2.0**.
///
/// Nothing shipped declares a lifecycle on an `option_conflict` yet — a
/// `.tclspec` pack is the first thing that will (`option_conflict {-a -b}
/// -introduced 2.0` is loader vocabulary 1.1) — so the gate is exercised
/// through an overlay registry, which is exactly the route a workspace's
/// packs reach the analyser by.
mod gated_option_conflict {
    use super::super::super::super::state::Analyser;

    /// Overlay key for the synthetic pack below. Any non-zero value works;
    /// the cache keys `(profile, overlay)` on it.
    const OVERLAY: u64 = 0x7e57_c0f1;

    const OPTIONS: &[tcl_registry::hover::OptionSpec] = &[
        tcl_registry::hover::OptionSpec {
            name: "-alpha",
            ..tcl_registry::hover::OptionSpec::DEFAULT
        },
        tcl_registry::hover::OptionSpec {
            name: "-beta",
            ..tcl_registry::hover::OptionSpec::DEFAULT
        },
    ];

    const CONSTRAINTS: &[tcl_registry::OptionRelation] = &[tcl_registry::OptionRelation {
        terms: &[
            tcl_registry::OptionTerm::Option("-alpha"),
            tcl_registry::OptionTerm::Option("-beta"),
        ],
        lifecycle: tcl_registry::lifecycle::Lifecycle::introduced_in("2.0"),
        ..tcl_registry::OptionRelation::DEFAULT
    }];

    /// The same relationship with no lifecycle at all — the zero-change
    /// control for the inline path.
    const UNGATED_CONSTRAINTS: &[tcl_registry::OptionRelation] = &[tcl_registry::OptionRelation {
        terms: &[
            tcl_registry::OptionTerm::Option("-alpha"),
            tcl_registry::OptionTerm::Option("-beta"),
        ],
        ..tcl_registry::OptionRelation::DEFAULT
    }];

    fn spec(
        name: &'static str,
        constraints: &'static [tcl_registry::OptionRelation],
    ) -> tcl_registry::CommandSpec {
        tcl_registry::CommandSpec {
            name,
            required_package: Some("Fauxpkg"),
            arity: tcl_registry::Arity::any(),
            options: OPTIONS,
            option_relations: constraints,
            ..tcl_registry::CommandSpec::DEFAULT
        }
    }

    /// Every W147 message `source` draws under `tcl8.6`, with the
    /// synthetic pack installed.
    fn conflicts(source: &str) -> Vec<String> {
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
        let _registry =
            tcl_registry::registry_for_profile_with_overlay(profile, OVERLAY, |registry| {
                registry.insert(spec("fauxgated", CONSTRAINTS));
                registry.insert(spec("fauxplain", UNGATED_CONSTRAINTS));
            });
        Analyser::new()
            .with_pack_overlay(OVERLAY)
            .analyse(source, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| d.code.as_str() == "W147")
            .map(|d| d.message.clone())
            .collect()
    }

    #[test]
    fn a_conflict_introduced_later_is_silent_below_its_floor() {
        // The relationship does not exist in Fauxpkg 1.x, so enforcing it
        // would report a rule that release has not got.
        let src = "package require Fauxpkg 1.0\nfauxgated -alpha -beta\n";
        assert_eq!(conflicts(src).len(), 0, "{:?}", conflicts(src));
    }

    #[test]
    fn a_conflict_introduced_later_fires_once_the_floor_reaches_it() {
        let src = "package require Fauxpkg 2.0\nfauxgated -alpha -beta\n";
        let diags = conflicts(src);
        assert!(
            diags
                .iter()
                .any(|m| m.contains("-alpha, -beta") && m.contains("fauxgated")),
            "{diags:?}"
        );
    }

    #[test]
    fn no_resolvable_floor_stays_permissive() {
        // An unversioned require on an unpinned package resolves no floor
        // at all, and "no version known ⇒ do not gate" is the standing
        // registry rule — so the conflict is reported.
        let src = "package require Fauxpkg\nfauxgated -alpha -beta\n";
        assert_eq!(conflicts(src).len(), 1, "{:?}", conflicts(src));
    }

    #[test]
    fn an_unversioned_constraint_is_untouched_by_the_gate() {
        // The zero-behaviour-change control: a constraint with no
        // lifecycle keeps the inline path, floor or no floor.
        for src in [
            "package require Fauxpkg 1.0\nfauxplain -alpha -beta\n",
            "package require Fauxpkg 2.0\nfauxplain -alpha -beta\n",
            "package require Fauxpkg\nfauxplain -alpha -beta\n",
        ] {
            assert_eq!(conflicts(src).len(), 1, "{src:?}: {:?}", conflicts(src));
        }
    }

    #[test]
    fn a_retired_conflict_stops_being_enforced_but_a_deprecated_one_does_not() {
        // Retirement is exclusive: the relationship is gone at 3.0…
        const RETIRED: &[tcl_registry::OptionRelation] = &[tcl_registry::OptionRelation {
            terms: &[
                tcl_registry::OptionTerm::Option("-alpha"),
                tcl_registry::OptionTerm::Option("-beta"),
            ],
            lifecycle: tcl_registry::lifecycle::Lifecycle::UNSPECIFIED
                .retired_from("3.0")
                .deprecated_from("2.0"),
            ..tcl_registry::OptionRelation::DEFAULT
        }];
        const KEY: u64 = 0x7e57_c0f2;
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
        let _registry = tcl_registry::registry_for_profile_with_overlay(profile, KEY, |registry| {
            registry.insert(spec("fauxretired", RETIRED));
        });
        let count = |source: &str| {
            Analyser::new()
                .with_pack_overlay(KEY)
                .analyse(source, "tcl8.6")
                .diagnostics
                .iter()
                .filter(|d| d.code.as_str() == "W147")
                .count()
        };
        assert_eq!(
            count("package require Fauxpkg 3.0\nfauxretired -alpha -beta\n"),
            0,
            "a retired relationship is not enforced"
        );
        // … but a *deprecated* one still exists, and deprecation is not
        // absence.
        assert_eq!(
            count("package require Fauxpkg 2.0\nfauxretired -alpha -beta\n"),
            1,
            "a deprecated relationship still holds"
        );
    }

    #[test]
    fn a_shadowing_user_proc_still_suppresses_a_gated_conflict() {
        // The gate decides whether the relationship exists; it must not
        // bypass the `pending_arity` shadowing suppression an ungated
        // conflict goes through.
        let src = "package require Fauxpkg 2.0\n\
                       proc fauxgated args {}\n\
                       fauxgated -alpha -beta\n";
        assert_eq!(conflicts(src).len(), 0, "{:?}", conflicts(src));
    }

    #[test]
    fn a_require_inside_the_file_gates_a_conflict_in_a_proc_body() {
        // The floor is a whole-file fact and the `package require` may
        // follow the call: buffering is what makes this work.
        let below = "proc use {} { fauxgated -alpha -beta }\n\
                         package require Fauxpkg 1.0\n";
        assert_eq!(conflicts(below).len(), 0, "{:?}", conflicts(below));
        let met = "proc use {} { fauxgated -alpha -beta }\n\
                       package require Fauxpkg 2.0\n";
        assert_eq!(conflicts(met).len(), 1, "{:?}", conflicts(met));
    }
}

// §5.4 range targeting, W150 / W151.

mod range_targeting {
    use super::super::super::super::state::Analyser;

    /// Every range / version-gate / numeral code for `source` under
    /// `dialect`, on a fresh analyser.
    fn diags(source: &str, dialect: &str) -> Vec<(String, String)> {
        diags_with(Analyser::new(), source, dialect)
    }

    fn diags_with(mut analyser: Analyser, source: &str, dialect: &str) -> Vec<(String, String)> {
        analyser
            .analyse(source, dialect)
            .diagnostics
            .iter()
            .filter(|d| {
                matches!(
                    d.code.as_str(),
                    "W002"
                        | "W135"
                        | "W136"
                        | "W137"
                        | "W138"
                        | "W139"
                        | "W144"
                        | "W148"
                        | "W150"
                        | "W151"
                )
            })
            .map(|d| (d.code.to_string(), d.message.clone()))
            .collect()
    }

    fn fires(diags: &[(String, String)], code: &str) -> bool {
        diags.iter().any(|(c, _)| c == code)
    }

    /// The owner's canonical case, both ways: a leading-zero literal
    /// under a range spanning 8.x and 9.x draws the meaning-change
    /// warning; a single-target 9.0 project — and an undeclared one —
    /// stays clean.
    #[test]
    fn the_octal_case_both_ways() {
        let spanning = "# tcl-lsp: supports tcl 8.5-9.0\nexpr {010}\n";
        let diags = diags(spanning, "tcl8.6");
        assert!(
            diags.iter().any(|(c, m)| c == "W151"
                && m.contains("means 8")
                && m.contains("10")
                && m.contains("silent meaning change")),
            "8.5-9.0 spans the octal/decimal boundary: {diags:?}"
        );

        let single = "# tcl-lsp: supports tcl 9.0\nexpr {010}\n";
        assert!(
            !fires(&super::range_targeting::diags(single, "tcl9.0"), "W151"),
            "a single 9.0 target has one numeral grammar"
        );
        let undeclared = "expr {010}\n";
        for dialect in ["tcl8.6", "tcl9.0", "tcl"] {
            let diags = super::range_targeting::diags(undeclared, dialect);
            assert!(
                !fires(&diags, "W151") && !fires(&diags, "W150"),
                "{dialect}: no declaration, no range mode: {diags:?}"
            );
        }
    }

    /// Prefix and separator numerals draw the validity half of W151
    /// when a declared target predates them — and only then.
    #[test]
    fn prefix_numerals_follow_the_declared_range() {
        // `0o17` predates nothing in 8.5-9.0…
        let fine = "# tcl-lsp: supports tcl 8.5-9.0\nexpr {0o17}\n";
        assert!(
            !fires(&diags(fine, "tcl8.6"), "W151"),
            "0o is 8.5+ and the range starts at 8.5"
        );
        // …but an 8.4 target rejects it.
        let with_84 = "# tcl-lsp: supports tcl 8.4-9.0\nexpr {0o17}\n";
        let d = diags(with_84, "tcl8.6");
        assert!(
            d.iter()
                .any(|(c, m)| c == "W151" && m.contains("not accepted") && m.contains("8.4")),
            "0o17 is a bareword under 8.4: {d:?}"
        );
        // 9.x-only spellings under a range reaching back to 8.x — the
        // primary (9.0) accepts them, the 8.x targets do not.
        for word in ["0d5", "1_000"] {
            let source = format!("# tcl-lsp: supports tcl 8.5-9.0\nset x {word}\n");
            let d = diags(&source, "tcl9.0");
            assert!(
                d.iter()
                    .any(|(c, m)| c == "W151" && m.contains("not accepted")),
                "{word} is 9.0-only: {d:?}"
            );
            // Under an 8.6 primary the same word is W148's
            // single-target fact, never double-reported as W151.
            let d = diags(&source, "tcl8.6");
            assert!(
                fires(&d, "W148") && !fires(&d, "W151"),
                "{word}: primary rejection stays W148's: {d:?}"
            );
        }
    }

    /// A command introduced at 8.6 under an 8.5–9.0 range: fine at the
    /// primary, missing at the declared 8.5 target — the availability
    /// half of the range family (mask-gated, no lifecycle involved).
    #[test]
    fn a_command_introduced_at_86_fails_the_85_target() {
        let source = "# tcl-lsp: supports tcl 8.5-9.0\nlmap x {1 2} {set x}\n";
        let d = diags(source, "tcl8.6");
        assert!(
            d.iter().any(|(c, m)| c == "W150"
                && m.contains("'lmap' requires Tcl 8.6")
                && m.contains("include 8.5")),
            "lmap is 8.6+: {d:?}"
        );
        // A declared single target equal to the primary changes nothing.
        let pinned = "# tcl-lsp: supports tcl 8.6\nlmap x {1 2} {set x}\n";
        assert!(!fires(&diags(pinned, "tcl8.6"), "W150"));
    }

    /// The removal direction: `case` is gone from 9.0, so a range that
    /// includes 9.0 flags it even though the 8.6 primary still has it.
    #[test]
    fn a_command_removed_at_9_fails_the_90_target() {
        let source = "# tcl-lsp: supports tcl 8.5-9.0\ncase $x in {a {set y 1}}\n";
        let d = diags(source, "tcl8.6");
        assert!(
            d.iter()
                .any(|(c, m)| c == "W150" && m.contains("'case'") && m.contains("missing at 9.0")),
            "case does not exist at the 9.0 target: {d:?}"
        );
    }

    /// The assistance/semantic split: an item that fails at the
    /// **primary** keeps today's semantic-floor diagnostic (W002 here)
    /// and never draws the assistance range warning on top.
    #[test]
    fn a_primary_failure_stays_the_floor_diagnostic() {
        let source = "# tcl-lsp: supports tcl 8.5-9.0\nlmap x {1 2} {set x}\n";
        let d = diags(source, "tcl8.5");
        assert!(
            !fires(&d, "W150"),
            "absent at the primary is W002's, not a range fact: {d:?}"
        );
        assert!(
            fires(&d, "W002"),
            "the semantic floor verdict stands: {d:?}"
        );
    }

    /// A library range (`supports Tk 8.5-8.6`) gates package items
    /// through the same placement/floor machinery: the require
    /// satisfies the floor, the declared 8.5–8.6 window does not reach
    /// the option's 8.7 introduction.
    #[test]
    fn a_package_item_under_a_library_range() {
        let source = "# tcl-lsp: supports Tk 8.5-8.6\n\
                          package require Tk 8.7\n\
                          entry .e -placeholder hi\n";
        let d = diags(source, "tcl8.6");
        assert!(
            !fires(&d, "W136"),
            "the 8.7 require satisfies the floor: {d:?}"
        );
        assert!(
            d.iter().any(|(c, m)| c == "W150"
                && m.contains("'-placeholder'")
                && m.contains("requires Tk 8.7")
                && m.contains("8.5")),
            "the declared Tk range misses the 8.7 introduction: {d:?}"
        );
        // Without the declaration the same source is silent — the
        // library range is what changes the verdict.
        let undeclared = "package require Tk 8.7\nentry .e -placeholder hi\n";
        assert!(!fires(&diags(undeclared, "tcl8.6"), "W150"));
    }

    /// Settings-declared targets (`tclLsp.targets`) resolve exactly as
    /// the directive does, and the directive wins per provider.
    #[test]
    fn settings_and_directive_ingress() {
        let configured = || {
            Analyser::new().with_declared_targets(vec![("tcl".to_owned(), "8.5-9.0".to_owned())])
        };
        let source = "lmap x {1 2} {set x}\n";
        let d = diags_with(configured(), source, "tcl8.6");
        assert!(
            d.iter().any(|(c, m)| c == "W150" && m.contains("'lmap'")),
            "settings alone switch range mode on: {d:?}"
        );
        // A source directive narrows the same provider back to the
        // primary's own line — the more specific source wins.
        let overridden = "# tcl-lsp: supports tcl 8.6\nlmap x {1 2} {set x}\n";
        assert!(
            !fires(&diags_with(configured(), overridden, "tcl8.6"), "W150"),
            "the directive overrides the configured pair"
        );
        // A malformed declaration is dropped, never guessed at.
        let malformed = "# tcl-lsp: supports tcl not-a-version\nlmap x {1 2} {set x}\n";
        assert!(!fires(&diags(malformed, "tcl8.6"), "W150"));
    }

    /// The argument-DSL rung follows the range too: a conversion fine
    /// at the primary but newer than a declared target warns.
    #[test]
    fn a_dsl_feature_fails_an_older_declared_target() {
        let source = "# tcl-lsp: supports tcl 8.5-9.0\nformat %b 5\n";
        let d = diags(source, "tcl8.6");
        assert!(
            d.iter().any(|(c, m)| c == "W150"
                && m.contains("%b")
                && m.contains("requires Tcl 8.6")
                && m.contains("8.5")),
            "%b is 8.6+ and 8.5 is declared: {d:?}"
        );
        assert!(!fires(&d, "W138"), "the primary itself is satisfied");
    }

    /// The no-range pin: a declared single target equal to the
    /// primary's own line leaves the diagnostic list byte-identical to
    /// an undeclared document.
    #[test]
    fn a_single_target_matching_the_primary_changes_nothing() {
        let body = "lmap x {1 2} {set x}\n\
                        expr {010}\n\
                        format %b 5\n\
                        entry .e -placeholder hi\n\
                        trace variable v w handler\n";
        let declared = format!("# tcl-lsp: supports tcl 8.6\n{body}");
        let base: Vec<String> = Analyser::new()
            .analyse(body, "tcl8.6")
            .diagnostics
            .iter()
            .map(|d| format!("{} {}", d.code.as_str(), d.message))
            .collect();
        let pinned: Vec<String> = Analyser::new()
            .analyse(&declared, "tcl8.6")
            .diagnostics
            .iter()
            .map(|d| format!("{} {}", d.code.as_str(), d.message))
            .collect();
        assert_eq!(base, pinned, "single target ⇒ today's behaviour");
    }

    // The Tk pilot rides the same range machinery.
    //
    // §5.4's "packages take range targets exactly like cores" (§3.2,
    // last bullet), proved on the acceptance case. `Tk` is a package
    // axis, so `supports Tk …` is the library half of ruling R6's
    // directive and nothing about it is Tcl-core-specific.

    /// A project declaring `Tk 8.5-8.6` gets warned about an item Tk
    /// only grew at 8.6 — a representative case. `tk busy`
    /// is `Lifecycle::introduced_in("8.6")` on the `tk` ensemble.
    #[test]
    fn a_tk_range_warns_on_an_item_the_older_tk_lacks() {
        let src = "# tcl-lsp: supports Tk 8.5-8.6\n\
                       package require Tk\n\
                       tk busy hold .\n";
        let d = diags(src, "tcl8.6");
        assert!(
            d.iter().any(|(c, m)| c == "W150"
                && m.contains("Tk 8.6")
                && m.contains("declared targets include")
                && m.contains("8.5")),
            "an 8.6-only Tk subcommand under Tk 8.5-8.6: {d:?}"
        );
    }

    /// The three controls that keep the Tk range honest: a range that
    /// starts at 8.6 is clean, a `tcl` declaration does not gate the
    /// `Tk` axis (invariant I2 — values from different axes are never
    /// compared), and an undeclared document is untouched.
    #[test]
    fn a_tk_range_is_scoped_to_the_tk_axis() {
        let body = "package require Tk\ntk busy hold .\n";
        let covered = format!("# tcl-lsp: supports Tk 8.6-9.0\n{body}");
        assert!(
            !fires(&diags(&covered, "tcl8.6"), "W150"),
            "Tk 8.6-9.0 covers an 8.6 introduction"
        );
        // A *core* range naming the same numerals says nothing about
        // the Tk axis.
        let core_only = format!("# tcl-lsp: supports tcl 8.5-8.6\n{body}");
        let d = diags(&core_only, "tcl8.6");
        assert!(
            !d.iter().any(|(c, m)| c == "W150" && m.contains("Tk 8.6")),
            "the core axis must not gate the Tk axis: {d:?}"
        );
        assert!(!fires(&diags(body, "tcl8.6"), "W150"), "undeclared");
    }

    // The tcllib adversarial modules.
    //
    // A tcllib module is an independently versioned package with its
    // own axis, so `supports struct::tree …` gates on the *module's*
    // train, not on tcllib-as-a-blob and not on the Tcl core. The
    // acceptance item is `::struct::tree::prune`: `tree1.tcl` (the
    // 1.2.3 train) has no such proc, `tree_tcl.tcl` (2.1.3) defines
    // it as `return -code 5`, so it carries
    // `introduced: "2.0"` on the `struct::tree` axis.

    /// A project declaring `struct::tree 1.2-2.2` — both trains — is
    /// told that `prune` does not exist across the whole range.
    #[test]
    fn a_tcllib_module_range_warns_on_an_item_the_older_release_lacks() {
        let src = "# tcl-lsp: supports struct::tree 1.2-2.2\n\
                       package require struct::tree\n\
                       ::struct::tree::prune\n";
        let d = diags(src, "tcl8.6");
        assert!(
            d.iter().any(|(c, m)| c == "W150"
                && m.contains("struct::tree 2.0")
                && m.contains("declared targets include")
                && m.contains("1.2")),
            "a 2.0-only command under struct::tree 1.2-2.2: {d:?}"
        );
        // The declaration alone is enough — no `package require`
        // needed, because a target set is a project fact, not a
        // document one.
        let bare = "# tcl-lsp: supports struct::tree 1.2-2.2\n::struct::tree::prune\n";
        assert!(fires(&diags(bare, "tcl8.6"), "W150"));
    }

    /// The module range lives on **that module's** axis (invariant
    /// I2), which a tcllib module can prove in a way `Tk` could not:
    /// a *different tcllib module's* range with overlapping numerals
    /// must say nothing about this one.
    #[test]
    fn a_tcllib_module_range_is_scoped_to_that_modules_axis() {
        let body = "package require struct::tree\n::struct::tree::prune\n";
        // Covered: a range that starts inside the 2.x train is clean.
        let covered = format!("# tcl-lsp: supports struct::tree 2.1-\n{body}");
        assert!(
            !fires(&diags(&covered, "tcl8.6"), "W150"),
            "2.1- covers 2.0"
        );
        // Another *module* with the same numerals says nothing here.
        let sibling = format!("# tcl-lsp: supports struct::list 1.2-2.2\n{body}");
        assert!(
            !fires(&diags(&sibling, "tcl8.6"), "W150"),
            "struct::list's axis must not gate struct::tree's items",
        );
        // Neither does the core axis.
        let core = format!("# tcl-lsp: supports tcl 8.5-9.0\n{body}");
        assert!(
            !fires(&diags(&core, "tcl8.6"), "W150"),
            "the core axis must not gate a tcllib module axis (I2)",
        );
        assert!(!fires(&diags(body, "tcl8.6"), "W150"), "undeclared");
    }

    /// The single-floor seed of the same family still works, and a
    /// module is **hosted**: the floor comes from the document's own
    /// `package require`, never from a placement pin.
    #[test]
    fn a_tcllib_module_floor_comes_from_the_package_require() {
        let old = "package require struct::tree 1.2\n::struct::tree::prune\n";
        let d = diags(old, "tcl8.6");
        assert!(
            d.iter().any(|(c, m)| c == "W135"
                && m.contains("struct::tree 2.0")
                && m.contains("guarantees only 1.2")),
            "the 1.2 require floors below the 2.0 introduction: {d:?}"
        );
        let new = "package require struct::tree 2.1\n::struct::tree::prune\n";
        assert!(!fires(&diags(new, "tcl8.6"), "W135"), "2.1 satisfies it");
        // No require at all is permissive on the version axis — W120
        // owns the missing-require nag for a hosted package.
        let none = "::struct::tree::prune\n";
        assert!(!fires(&diags(none, "tcl8.6"), "W135"));
    }

    // The jim ladder.
    //
    // Jim's releases are targets on the `jim` **core** axis, not nine
    // catalogue profiles, so a jim range is declared exactly as a Tcl
    // one is and the §5.4 machinery answers on the family's own
    // ladder. Invariant I2 is the load-bearing half: a jim
    // declaration must never reach the Tcl axis, and a Tcl one must
    // never reach jim's.

    /// The declared targets of a `jim` document land on the jim core
    /// axis, spelled off the jim ladder — and nowhere else.
    #[test]
    fn a_declared_jim_range_lands_on_the_jim_core_axis() {
        use tcl_dialect::model::{Family, Version, VersionAxisId};
        let mut analyser = Analyser::new();
        let _ = analyser.analyse("# tcl-lsp: supports jim 0.76-0.79\nset x 1\n", "jim");
        let context = analyser
            .range_context
            .as_ref()
            .expect("a jim declaration switches range mode on");
        let declared = context
            .declared_targets(&VersionAxisId::core(Family::Jim))
            .expect("targets on the jim axis");
        assert!(declared.contains(&Version::parse("0.78").expect("version")));
        assert!(!declared.contains(&Version::parse("0.84").expect("version")));
        // I2, both spellings of the leak: not the Tcl core axis, and
        // not a fictitious `package:jim` axis either, which is what an
        // ingress recognising only the name `tcl` as a family would mint.
        assert!(
            context
                .declared_targets(&VersionAxisId::core(Family::Tcl))
                .is_none(),
            "a jim range is not a Tcl range"
        );
        assert!(
            context
                .declared_targets(&VersionAxisId::package("jim"))
                .is_none(),
            "jim is a family, not a package"
        );
    }

    /// A family range declared against the wrong core is **dropped**,
    /// not coerced — in both directions. A `supports jim` line in a
    /// Tcl document and a `supports tcl` line in a jim document are
    /// each a statement about a ladder the document is not on.
    #[test]
    fn a_family_range_off_its_own_core_is_dropped() {
        use tcl_dialect::model::{Family, VersionAxisId};
        let mut in_tcl = Analyser::new();
        let _ = in_tcl.analyse("# tcl-lsp: supports jim 0.81-0.84\nset x 1\n", "tcl8.6");
        assert!(
            in_tcl.range_context.is_none(),
            "a jim range says nothing about a Tcl document"
        );

        let mut in_jim = Analyser::new();
        let _ = in_jim.analyse("# tcl-lsp: supports tcl 8.5-9.0\nset x 1\n", "jim");
        assert!(
            in_jim.range_context.is_none(),
            "a Tcl range says nothing about a jim document"
        );

        // …while the *package* axes still cross freely, because a
        // package is not a ladder: `supports Tk 8.5-8.6` is a real
        // declaration under either core.
        let mut package = Analyser::new();
        let _ = package.analyse("# tcl-lsp: supports Tk 8.5-8.6\nset x 1\n", "jim");
        assert!(
            package
                .range_context
                .as_ref()
                .expect("range mode")
                .declared_targets(&VersionAxisId::package("Tk"))
                .is_some()
        );
        assert!(
            package
                .range_context
                .as_ref()
                .expect("range mode")
                .declared_targets(&VersionAxisId::core(Family::Jim))
                .is_none()
        );
    }

    /// The W151 cross-target numeral check is a **Tcl-ladder** fact,
    /// so a jim range never switches it on — the octal case under
    /// `supports jim 0.76-0.84` stays silent where the same span of
    /// the Tcl ladder would warn.
    #[test]
    fn a_jim_range_does_not_drive_the_tcl_numeral_check() {
        let jim = "# tcl-lsp: supports jim 0.76-0.84\nexpr {010}\n";
        assert!(!fires(&diags(jim, "jim"), "W151"));
        // The control: the same shape on the Tcl axis does warn.
        let tcl = "# tcl-lsp: supports tcl 8.5-9.0\nexpr {010}\n";
        assert!(fires(&diags(tcl, "tcl8.6"), "W151"));
    }

    /// Under `jim` the shared core surface resolves through the
    /// ancestry edge: no W002 for `lassign`, `lmap` or `dict`, which
    /// on the jim branch were three of 76 hand-re-authored specs.
    #[test]
    fn the_jim_environment_resolves_the_inherited_core_surface() {
        let source = "set x 1\nlassign {a b} p q\nlmap i {1 2} {set i}\ndict get {a 1} a\n";
        let d = diags(source, "jim");
        assert!(
            !d.iter().any(|(code, _)| code == "W002"),
            "inherited core commands must resolve: {d:?}"
        );
    }

    /// Under `wish` the same declaration works with **no** `package
    /// require`: the `tk` environment ships Tk ambient, so the range
    /// is declared against a package the environment already placed.
    #[test]
    fn a_tk_range_applies_under_the_ambient_wish_placement() {
        let src = "# tcl-lsp: supports Tk 8.5-8.6\ntk busy hold .\n";
        let d = diags(src, "tk");
        assert!(
            d.iter().any(|(c, m)| c == "W150" && m.contains("Tk 8.6")),
            "wish + Tk 8.5-8.6: {d:?}"
        );
        assert!(
            !d.iter().any(|(c, _)| c == "W120"),
            "and no missing-require nag under an ambient placement: {d:?}"
        );
    }
}
