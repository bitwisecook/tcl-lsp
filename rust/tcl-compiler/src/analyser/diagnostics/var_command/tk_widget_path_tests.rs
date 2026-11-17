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

use crate::provider_fixtures::{Provider, analyse};

/// A Tk widget constructor binds its window path as the widget's object
/// command (`Tcl_CreateObjCommand` in `tkEntry.c`, Tk 8.4.20–9.0.4), so a
/// head whose value is that path names a present command and W307 stays
/// quiet, at statement level and inside a substitution (#2324). Tcl 8.6
/// and 9.0 under the Tk provider.
#[test]
fn a_head_holding_a_created_widget_path_is_a_present_command() {
    for release in ["tcl8.6", "tcl9.0"] {
        for constructor in ["entry", "ttk::entry"] {
            for use_site in ["puts [$e configure]", "$e configure -width 4"] {
                let source =
                    format!("package require Tk\nset e .e\n{constructor} $e\n{use_site}\n");
                let analysis = analyse(&source, release, &[Provider::Tk]);
                let realm = analysis.retained_command_realm().unwrap();
                let at = u32::try_from(source.find("$e configure").unwrap()).unwrap();
                assert_eq!(
                    realm
                        .invocation_at_source("", at)
                        .lookup_command_word(".e")
                        .selected_slot_presence(),
                    crate::command_binding::SourceCommandSlotPresence::Present,
                    "{release}: {source}"
                );
                assert!(
                    analysis
                        .diagnostics
                        .iter()
                        .all(|d| d.code != tcl_core_types::DiagCode::W307),
                    "{release}: {source}: {:?}",
                    analysis.diagnostics
                );
            }
        }
        // Before the constructor runs, the path names nothing.
        let source = "package require Tk\nset e .e\n$e configure\nentry $e\n";
        let analysis = analyse(source, release, &[Provider::Tk]);
        let at = u32::try_from(source.find("$e configure").unwrap()).unwrap();
        assert_ne!(
            analysis
                .retained_command_realm()
                .unwrap()
                .invocation_at_source("", at)
                .lookup_command_word(".e")
                .selected_slot_presence(),
            crate::command_binding::SourceCommandSlotPresence::Present,
            "{release}"
        );
    }
}

/// A Tk constructor returns its path as a fresh window object
/// (`TkNewWindowObj`, Tk 8.4.20–9.0.4), so a head holding that result names
/// the widget and W307 stays quiet; the call is never folded away, since
/// constructing the widget is the point (#2324). Tcl 8.6 and 9.0.
#[test]
fn a_head_holding_a_constructor_result_names_the_widget() {
    for release in ["tcl8.6", "tcl9.0"] {
        for source in [
            "package require Tk\nset e [entry .e]\n$e configure -width 4\n",
            "package require Tk\nset e [ttk::entry .e]\nputs [$e configure]\n",
            "package require Tk\nputs [[ttk::entry .e] configure]\n",
        ] {
            let analysis = analyse(source, release, &[Provider::Tk]);
            assert!(
                analysis
                    .diagnostics
                    .iter()
                    .all(|d| d.code != tcl_core_types::DiagCode::W307),
                "{release}: {source}: {:?}",
                analysis.diagnostics
            );
        }
        let profile = tcl_registry::model::ingress::resolve_environment(release).analyser_profile();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let source = "package require Tk\nset e [entry .e]\nputs $e\n";
        let (optimised, _) =
            crate::optimiser::optimise_source_multipass(source, &registry, Some(profile), 4);
        assert!(optimised.contains("[entry .e]"), "{release}: {optimised}");
    }
}

/// A call through a created widget takes the selected instance method's
/// declared effects, so the command world after `puts [$e configure]` is
/// still known; an unknown method stays an unknown call and leaves it
/// unknown (#2324). Tcl 8.6 and 9.0 under the Tk provider.
#[test]
fn a_widget_method_call_keeps_the_command_world_it_declares() {
    for release in ["tcl8.6", "tcl9.0"] {
        for (call, known) in [
            ("puts [$e configure]", true),
            ("$e configure -width 4", true),
            ("$e bogus", false),
            ("$e $method", false),
        ] {
            let source = format!("package require Tk\nset e .e\nentry $e\n{call}\nputs done\n");
            let analysis = analyse(&source, release, &[Provider::Tk]);
            let at = u32::try_from(source.find("puts done").unwrap()).unwrap();
            let presence = analysis
                .retained_command_realm()
                .unwrap()
                .invocation_at_source("", at)
                .lookup_command_word(".e")
                .selected_slot_presence();
            assert_eq!(
                presence == crate::command_binding::SourceCommandSlotPresence::Present,
                known,
                "{release}: {source}: {presence:?}"
            );
        }
    }
}

/// Deleting a widget command destroys the window, whose `<Destroy>`
/// bindings run arbitrary scripts, so `rename .e {}`, `rename .e .f` and
/// `destroy .e` leave no proof that `.e` is still bound (#2324). Tcl 8.6
/// and 9.0 under the Tk provider.
#[test]
fn deleting_a_widget_withdraws_its_binding() {
    for release in ["tcl8.6", "tcl9.0"] {
        for deletion in ["rename .e {}", "rename .e .f", "destroy .e"] {
            let source = format!("package require Tk\nentry .e\n{deletion}\nputs done\n");
            let analysis = analyse(&source, release, &[Provider::Tk]);
            let at = u32::try_from(source.find("puts done").unwrap()).unwrap();
            let presence = analysis
                .retained_command_realm()
                .unwrap()
                .invocation_at_source("", at)
                .lookup_command_word(".e")
                .selected_slot_presence();
            assert_ne!(
                presence,
                crate::command_binding::SourceCommandSlotPresence::Present,
                "{release}: {source}"
            );
        }
    }
}
