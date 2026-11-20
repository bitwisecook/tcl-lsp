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

use super::{UpgradeOptions, UpgradeStatus, environment_effect_snapshot, upgrade_source};
use crate::loader::evaluate_pack;

/// A pack with a `dialects` row at each of the three written shapes —
/// property, flag, multi-token — comes out spelling `available`, with
/// every other byte where the author left it.
#[test]
fn dialects_rows_translate_and_nothing_else_moves() {
    let source = "# a comment the rewriter must not touch\n\
                      speclib probe 1.2 {\n\
                      \x20   command demo {\n\
                      \x20       arity 1..\n\
                      \x20       dialects {tcl8.6+ f5-irules}\n\
                      \x20       option -x -dialects tcl8.x   ;# trailing comment\n\
                      \x20       subcommand sub {\n\
                      \x20           arity 0\n\
                      \x20           dialects tk\n\
                      \x20       }\n\
                      \x20   }\n\
                      }\n";
    let outcome = upgrade_source(source, &UpgradeOptions::default());
    assert_eq!(outcome.status, UpgradeStatus::Upgraded, "{outcome:#?}");
    assert_eq!(outcome.translated.len(), 3, "{:#?}", outcome.translated);
    assert!(outcome.deferred.is_empty(), "{:#?}", outcome.deferred);
    assert!(outcome.refusals.is_empty(), "{:#?}", outcome.refusals);
    assert_eq!(
        outcome.source,
        "# a comment the rewriter must not touch\n\
             speclib probe 2.3 {\n\
             \x20   command demo {\n\
             \x20       arity 1..\n\
             \x20       available {tcl 8.6-} {f5-irules}\n\
             \x20       option -x -available {tcl 8.4-9.0}   ;# trailing comment\n\
             \x20       subcommand sub {\n\
             \x20           arity 0\n\
             \x20           available {package Tk}\n\
             \x20       }\n\
             \x20   }\n\
             }\n"
    );
    // U7: nothing in the rewritten file needs a vocabulary above its
    // own declaration.
    assert!(
        outcome.above_target.is_empty(),
        "{:#?}",
        outcome.above_target
    );
}

/// `--restyle` re-emits the upgraded pack in canonical form —
/// comments and author layout gone, rows at the house margins — and the
/// restyled pack loads to the same snapshot as the plain rewrite.
#[test]
fn restyle_emits_canonical_form_that_loads_to_the_same_snapshot() {
    let source = "# a comment the restyle drops\n\
                      speclib probe 1.1 {\n\
                      display_name {Probe}\n\
                      command demo {\n\
                            arity 1..  ;# odd indent\n\
                        dialects {tcl8.6+ tk}\n\
                        subcommand sub { arity 0 }\n\
                      }\n\
                      }\n";
    let plain = upgrade_source(source, &UpgradeOptions::default());
    assert_eq!(plain.status, UpgradeStatus::Upgraded, "{plain:#?}");
    let restyled = upgrade_source(
        source,
        &UpgradeOptions {
            restyle: true,
            ..UpgradeOptions::default()
        },
    );
    assert_eq!(restyled.status, UpgradeStatus::Upgraded, "{restyled:#?}");
    assert!(restyled.restyled);
    assert!(
        restyled.restyle_losses.is_empty(),
        "{:?}",
        restyled.restyle_losses
    );
    assert_eq!(
        restyled.source,
        "speclib probe 1.1 {\n\
             display_name {Probe}\n\
             \n\
             command demo {\n\
             \x20   arity 1..\n\
             \x20   available {tcl 8.6-} {package Tk}\n\
             \n\
             \x20   subcommand sub {\n\
             \x20       arity 0\n\
             \x20   }\n\
             }\n\
             \n\
             }\n"
        .replace("probe 1.1", "probe 2.3")
    );
    assert!(
        restyled.above_target.is_empty(),
        "{:?}",
        restyled.above_target
    );
    let plain_pack = evaluate_pack(&plain.source);
    let restyled_pack = evaluate_pack(&restyled.source);
    assert!(
        restyled_pack.notices.is_empty(),
        "{:?}",
        restyled_pack.notices
    );
    assert_eq!(restyled_pack.display_name.as_deref(), Some("Probe"));
    assert_eq!(
        format!("{:?}", plain_pack.command("demo").expect("demo").spec),
        format!("{:?}", restyled_pack.command("demo").expect("demo").spec),
    );
    assert_eq!(
        environment_effect_snapshot(&plain_pack),
        environment_effect_snapshot(&restyled_pack)
    );
    // A restyled pack is a fixed point of the restyle.
    let again = upgrade_source(
        &restyled.source,
        &UpgradeOptions {
            restyle: true,
            ..UpgradeOptions::default()
        },
    );
    assert_eq!(again.status, UpgradeStatus::AlreadyCurrent, "{again:#?}");
    assert!(!again.restyled);
}

/// D13 / E-R12: a programmed pack is never rewritten. Its rows are not
/// translated either — the whole file is refused, byte-identical.
#[test]
fn restyle_refuses_a_programmed_pack() {
    let source = "speclib probe 2.3 {\n\
                      set version 1.0\n\
                      foreach name {alpha beta} {\n\
                      \x20   command $name { arity 0 }\n\
                      }\n\
                      }\n";
    let outcome = upgrade_source(
        source,
        &UpgradeOptions {
            restyle: true,
            ..UpgradeOptions::default()
        },
    );
    assert_eq!(outcome.status, UpgradeStatus::Refused, "{outcome:#?}");
    assert_eq!(outcome.source, source, "nothing is rewritten");
    assert!(!outcome.restyled);
    assert!(
        outcome
            .refusals
            .iter()
            .any(|refusal| refusal.message.contains("E-R12")),
        "{:?}",
        outcome.refusals
    );
    // Without `--restyle` the same file is simply current.
    let plain = upgrade_source(source, &UpgradeOptions::default());
    assert_eq!(plain.status, UpgradeStatus::AlreadyCurrent, "{plain:#?}");
}

/// E-R12, per word: a computed word anywhere in the pack — a `[list …]`
/// body, a `$var` in a nested option row, a substituting quoted word —
/// makes it a program, and a program is never rewritten, whatever its
/// head words look like.
#[test]
fn restyle_refuses_a_computed_word_at_any_depth() {
    let programmed = [
        "speclib probe 2.3 {\n command demo [list arity 1]\n}\n",
        "speclib probe 2.3 {\n\
             set flag -nocase\n\
             command demo {\n\
             \x20   arity 1\n\
             \x20   option $flag -takes none\n\
             }\n\
             }\n",
        "speclib probe 2.3 {\n\
             command demo {\n\
             \x20   arity 1\n\
             \x20   subcommand sub {\n\
             \x20       option [list -nocase] -takes none\n\
             \x20   }\n\
             }\n\
             }\n",
        "speclib probe 2.3 {\n\
             set x 1\n\
             command demo {\n\
             \x20   arity \"$x..\"\n\
             }\n\
             }\n",
    ];
    for source in programmed {
        let outcome = upgrade_source(
            source,
            &UpgradeOptions {
                restyle: true,
                ..UpgradeOptions::default()
            },
        );
        assert_eq!(
            outcome.status,
            UpgradeStatus::Refused,
            "{source}\n{outcome:#?}"
        );
        assert_eq!(outcome.source, source, "nothing is rewritten");
        assert!(
            outcome
                .refusals
                .iter()
                .any(|refusal| refusal.message.contains("E-R12")),
            "{source}\n{:?}",
            outcome.refusals
        );
    }
    // Braced bodies at every level are verbatim data, so a canonical pack
    // with nested blocks and braced prose still restyles.
    let canonical = "speclib probe 2.3 {\n\
                         command demo {\n\
                         \x20   arity 1\n\
                         \x20   hover { summary {Costs $5 [really].} }\n\
                         \x20   subcommand sub {\n\
                         \x20       option -nocase -takes none\n\
                         \x20   }\n\
                         }\n\
                         }\n";
    let outcome = upgrade_source(
        canonical,
        &UpgradeOptions {
            restyle: true,
            ..UpgradeOptions::default()
        },
    );
    assert_eq!(outcome.status, UpgradeStatus::Restyled, "{outcome:#?}");
    assert!(outcome.restyled);
    assert!(
        outcome.source.contains("summary {Costs $5 [really].}"),
        "{}",
        outcome.source
    );
}

/// A partial upgrade keeps its markers and is not restyled, while
/// the row rewrite still goes ahead and says why the restyle waited.
#[test]
fn restyle_is_skipped_on_a_partial_upgrade() {
    let source = "speclib probe 1.2 {\n\
                      command demo {\n\
                      \x20   arity 1\n\
                      \x20   dialects {tcl8.6 bpf}\n\
                      }\n\
                      }\n";
    let outcome = upgrade_source(
        source,
        &UpgradeOptions {
            restyle: true,
            ..UpgradeOptions::default()
        },
    );
    assert_eq!(outcome.status, UpgradeStatus::Partial, "{outcome:#?}");
    assert!(!outcome.restyled);
    assert!(outcome.restyle_skipped.is_some());
    assert!(
        outcome.source.contains("TODO(spectcl 2.0)"),
        "{}",
        outcome.source
    );
}

/// An already-current pack with nothing to translate is still
/// restyled when asked, and reports as such.
#[test]
fn restyle_of_a_current_pack_reports_restyled() {
    let source = "speclib probe 2.3 {\n  command demo {\n      arity 1\n  }\n}\n";
    let outcome = upgrade_source(
        source,
        &UpgradeOptions {
            restyle: true,
            ..UpgradeOptions::default()
        },
    );
    assert_eq!(outcome.status, UpgradeStatus::Restyled, "{outcome:#?}");
    assert!(outcome.restyled);
    assert_eq!(
        outcome.source,
        "speclib probe 2.3 {\n\ncommand demo {\n    arity 1\n}\n\n}\n"
    );
}

/// U9 in miniature: the rewritten pack loads to the same specs.
#[test]
fn the_rewritten_pack_loads_to_the_same_specs() {
    let source = "speclib probe 1.2 {\n\
                      \x20 command demo {\n\
                      \x20   arity 1\n\
                      \x20   dialects {tcl8.6+ tk}\n\
                      \x20 }\n\
                      }\n";
    let outcome = upgrade_source(source, &UpgradeOptions::default());
    assert_eq!(outcome.status, UpgradeStatus::Upgraded, "{outcome:#?}");
    let before = evaluate_pack(source);
    let after = evaluate_pack(&outcome.source);
    assert!(after.notices.is_empty(), "{:?}", after.notices);
    assert_eq!(
        format!("{:?}", before.command("demo").expect("demo").spec),
        format!("{:?}", after.command("demo").expect("demo").spec),
    );
}

/// U3: a membership token whose environment declares one ambient
/// package translates to that package's `available` row, and the
/// rewritten pack loads to the same specs (the environment-derived
/// package↔bit table in `loader::available`).
#[test]
fn a_membership_token_translates_to_its_environments_ambient_package() {
    let source = "speclib probe 1.2 {\n\
                      \x20 command demo {\n\
                      \x20   arity 1\n\
                      \x20   dialects {tcl8.6 f5-iapps}\n\
                      \x20 }\n\
                      \x20 command duo {\n\
                      \x20   arity 1\n\
                      \x20   dialects f5-tmsh\n\
                      \x20 }\n\
                      \x20 command trio {\n\
                      \x20   arity 1\n\
                      \x20   dialects expect\n\
                      \x20 }\n\
                      }\n";
    let outcome = upgrade_source(source, &UpgradeOptions::default());
    assert_eq!(outcome.status, UpgradeStatus::Upgraded, "{outcome:#?}");
    assert!(
        outcome
            .source
            .contains("available {tcl 8.6} {package f5-iapps-cmds}"),
        "{}",
        outcome.source
    );
    assert!(
        outcome.source.contains("available {package f5-tmsh-cmds}"),
        "{}",
        outcome.source
    );
    assert!(
        outcome.source.contains("available {package Expect}"),
        "{}",
        outcome.source
    );
    let before = evaluate_pack(source);
    let after = evaluate_pack(&outcome.source);
    for name in ["demo", "duo", "trio"] {
        assert_eq!(
            format!("{:?}", before.command(name).expect(name).spec),
            format!("{:?}", after.command(name).expect(name).spec),
            "{name}"
        );
    }
    assert!(
        outcome.above_target.is_empty(),
        "{:#?}",
        outcome.above_target
    );
}

/// U3: a membership token whose environment declares **no** ambient
/// provider (its surface is compiled) is left byte-identical, marked
/// in place, and reported — and the version word does **not** move,
/// because the file is only partially upgraded (U1).
#[test]
fn an_environment_membership_token_is_deferred_not_guessed() {
    let source = "speclib probe 1.2 {\n\
                      \x20 command demo {\n\
                      \x20   arity 1\n\
                      \x20   dialects {tcl8.6 spectcl}\n\
                      \x20 }\n\
                      \x20 command duo {\n\
                      \x20   arity 1\n\
                      \x20   dialects bpf\n\
                      \x20 }\n\
                      }\n";
    let outcome = upgrade_source(source, &UpgradeOptions::default());
    assert_eq!(outcome.status, UpgradeStatus::Partial, "{outcome:#?}");
    assert_eq!(outcome.deferred.len(), 2, "{:#?}", outcome.deferred);
    assert_eq!(outcome.deferred[0].token, "spectcl");
    assert_eq!(outcome.deferred[1].token, "bpf");
    assert!(
        outcome.source.contains("# TODO(spectcl 2.0):"),
        "{}",
        outcome.source
    );
    assert!(
        outcome.source.contains("dialects {tcl8.6 spectcl}"),
        "the row itself is untouched: {}",
        outcome.source
    );
    assert!(
        outcome.source.contains("speclib probe 1.2"),
        "a partial upgrade keeps its 1.x header: {}",
        outcome.source
    );
}

/// U4: an `ambient_package` row rehomes into the owning environment's
/// `-extend` block — here the owner is the pack's sole membership
/// token — and the environment-effect snapshot is unchanged across
/// the rewrite.
#[test]
fn an_ambient_package_row_rehomes_into_the_owning_environment() {
    let source = "speclib probe 1.2 {\n\
                      ambient_package iapp_utils 1.4\n\
                      \x20 command demo {\n\
                      \x20   arity 1\n\
                      \x20   dialects f5-iapps\n\
                      \x20 }\n\
                      }\n";
    let outcome = upgrade_source(source, &UpgradeOptions::default());
    assert_eq!(outcome.status, UpgradeStatus::Upgraded, "{outcome:#?}");
    assert!(
        outcome
            .source
            .contains("environment f5-iapps -extend {\n    ambient iapp_utils 1.4\n}"),
        "{}",
        outcome.source
    );
    let with_bom = upgrade_source(&format!("\u{feff}{source}"), &UpgradeOptions::default());
    assert_eq!(with_bom.status, outcome.status);
    assert_eq!(with_bom.source, format!("\u{feff}{}", outcome.source));
    let before = evaluate_pack(source);
    let after = evaluate_pack(&outcome.source);
    assert!(
        after.ambient_packages.is_empty(),
        "{:#?}",
        after.ambient_packages
    );
    assert_eq!(after.environments.len(), 1, "{:#?}", after.notices);
    assert!(after.environments[0].extends);
    assert_eq!(
        super::environment_effect_snapshot(&before),
        super::environment_effect_snapshot(&after),
        "the registry effect must not move"
    );
    assert!(
        outcome.above_target.is_empty(),
        "{:#?}",
        outcome.above_target
    );
}

/// U4's cannot-infer rule: with no declared environment and no sole
/// membership token, the row keeps its marker and the file reports
/// partial.
#[test]
fn an_ambient_package_row_with_no_owner_is_deferred() {
    let source = "speclib probe 1.2 {\n\
                      ambient_package iapp_utils 1.4\n\
                      \x20 command demo {\n\
                      \x20   arity 1\n\
                      \x20   dialects tcl8.6\n\
                      \x20 }\n\
                      }\n";
    let outcome = upgrade_source(source, &UpgradeOptions::default());
    assert_eq!(outcome.status, UpgradeStatus::Partial, "{outcome:#?}");
    assert!(
        outcome.source.contains("ambient_package iapp_utils 1.4"),
        "{}",
        outcome.source
    );
    assert!(
        outcome
            .source
            .contains("# TODO(spectcl 2.0): this `ambient_package` row"),
        "{}",
        outcome.source
    );
}

/// U5: a `file_extension … -dialect D` row's detection moves into
/// `environment D -extend { … }`, the flag itself dropped, everything
/// else verbatim — and the effect snapshot is unchanged.
#[test]
fn a_file_extension_dialect_row_moves_into_the_environment_block() {
    let source = "speclib probe 1.1 {\n\
                      file_extension upf -name {Unified Power Format} -dialect synopsys-eda-tcl\n\
                      \x20 command demo {\n\
                      \x20   arity 1\n\
                      \x20 }\n\
                      }\n";
    let outcome = upgrade_source(source, &UpgradeOptions::default());
    assert_eq!(outcome.status, UpgradeStatus::Upgraded, "{outcome:#?}");
    assert!(
        outcome.source.contains(
            "environment synopsys-eda-tcl -extend {\n\
                 \x20   file_extension upf -name {Unified Power Format}\n\
                 }"
        ),
        "{}",
        outcome.source
    );
    let before = evaluate_pack(source);
    let after = evaluate_pack(&outcome.source);
    assert!(after.file_extensions.is_empty());
    assert_eq!(
        super::environment_effect_snapshot(&before),
        super::environment_effect_snapshot(&after),
    );
    assert!(
        outcome.above_target.is_empty(),
        "{:#?}",
        outcome.above_target
    );
}

/// U6: `--infer-provides` hoists the pack-level default, and the
/// rewritten pack loads to the same specs through the `provides`
/// fallback.
#[test]
fn infer_provides_hoists_the_pack_level_default() {
    let source = "speclib probe 1.2 {\n\
                      default required_package upf\n\
                      \x20 command demo {\n\
                      \x20   arity 1\n\
                      \x20 }\n\
                      }\n";
    let options = UpgradeOptions {
        infer_provides: true,
        ..UpgradeOptions::default()
    };
    let outcome = upgrade_source(source, &options);
    assert_eq!(outcome.status, UpgradeStatus::Upgraded, "{outcome:#?}");
    assert!(
        outcome.source.contains("provides upf"),
        "{}",
        outcome.source
    );
    assert!(
        !outcome.source.contains("default required_package"),
        "{}",
        outcome.source
    );
    let before = evaluate_pack(source);
    let after = evaluate_pack(&outcome.source);
    assert_eq!(after.provides.len(), 1);
    assert_eq!(
        before.command("demo").expect("demo").spec.required_package,
        after.command("demo").expect("demo").spec.required_package,
    );
}

/// U6's per-command shape: a uniform `required_package` row in every
/// command hoists to one pack-level `provides`, and specs are
/// unchanged.
#[test]
fn infer_provides_hoists_uniform_per_command_rows() {
    let source = "speclib probe 1.2 {\n\
                      \x20 command demo {\n\
                      \x20   arity 1\n\
                      \x20   required_package upf\n\
                      \x20 }\n\
                      \x20 command duo {\n\
                      \x20   arity 1\n\
                      \x20   required_package upf\n\
                      \x20 }\n\
                      }\n";
    let options = UpgradeOptions {
        infer_provides: true,
        ..UpgradeOptions::default()
    };
    let outcome = upgrade_source(source, &options);
    assert_eq!(outcome.status, UpgradeStatus::Upgraded, "{outcome:#?}");
    assert!(
        outcome.source.contains("provides upf"),
        "{}",
        outcome.source
    );
    assert!(
        !outcome.source.contains("required_package upf"),
        "{}",
        outcome.source
    );
    let before = evaluate_pack(source);
    let after = evaluate_pack(&outcome.source);
    for name in ["demo", "duo"] {
        assert_eq!(
            before.command(name).expect(name).spec.required_package,
            after.command(name).expect(name).spec.required_package,
            "{name}"
        );
    }
}

/// A 1.x dialect word the upgrade cannot carry refuses the whole file,
/// which is left as written. Q3: `f5-bigip` is not translated into a
/// claim about the Tcl axis. U0: the 1.x vocabulary is closed, so a word
/// outside it is not carried through unread.
#[test]
fn a_dialect_word_the_upgrade_cannot_carry_refuses_the_file() {
    for (word, reason) in [
        ("f5-bigip", "leaves the Tcl axis"),
        ("klingon", "is not a SpecTcl 1.x dialect word"),
    ] {
        let source = format!(
            "speclib probe 1.2 {{\n command demo {{\n arity 1\n dialects {word}\n }}\n}}\n"
        );
        let outcome = upgrade_source(&source, &UpgradeOptions::default());
        assert_eq!(outcome.status, UpgradeStatus::Refused, "{word}");
        assert_eq!(
            outcome.source, source,
            "{word}: a refused file is not rewritten"
        );
        assert!(
            outcome.refusals[0].message.contains(reason),
            "{word}: {:#?}",
            outcome.refusals
        );
    }
}

/// U10: downgrades are refused.
#[test]
fn a_downgrade_is_refused() {
    let outcome = upgrade_source(
        "speclib probe 2.0 {\n command demo { arity 1 }\n}\n",
        &UpgradeOptions {
            from: "2.0".to_owned(),
            to: "1.2".to_owned(),
            ..UpgradeOptions::default()
        },
    );
    assert_eq!(outcome.status, UpgradeStatus::Refused);
    assert!(
        outcome.refusals[0]
            .message
            .contains("refusing to downgrade 2.0 → 1.2"),
        "{:#?}",
        outcome.refusals
    );
}

/// A file with no `speclib` line is skipped, not rewritten.
#[test]
fn a_file_with_no_speclib_line_is_skipped() {
    let outcome = upgrade_source("# just a comment\n", &UpgradeOptions::default());
    assert_eq!(outcome.status, UpgradeStatus::NotAPack);
}

/// A hook body is arbitrary Tcl, not pack vocabulary: its words are
/// never rewritten, even when one of them reads like a declaration.
#[test]
fn a_hook_body_is_not_descended_into() {
    let source = "speclib probe 1.2 {\n\
                      \x20 hook probe {ctx} {\n\
                      \x20   set dialects tcl8.6\n\
                      \x20 }\n\
                      \x20 command demo { arity 1 }\n\
                      }\n";
    let outcome = upgrade_source(source, &UpgradeOptions::default());
    assert!(outcome.translated.is_empty(), "{:#?}", outcome.translated);
    assert!(
        outcome.source.contains("set dialects tcl8.6"),
        "{}",
        outcome.source
    );
}

/// A pack already at the target vocabulary with no 1.x spellings left
/// is finished, and its bytes are untouched.
#[test]
fn an_already_current_pack_is_left_alone() {
    let source = "speclib probe 2.3 {\n command demo {\n arity 1\n \
                      available {tcl 8.6-}\n }\n}\n";
    let outcome = upgrade_source(source, &UpgradeOptions::default());
    assert_eq!(outcome.status, UpgradeStatus::AlreadyCurrent);
    assert_eq!(outcome.source, source);
}
