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

use super::{EntryProcedureFrame, ResolvedAnalysisInput};
use std::sync::Arc;

/// A document a loader evaluates inside a procedure (`auto_load_index`'s
/// `eval [read $f]`, `tclPkgUnknown`'s `source`) starts with the loader's
/// local `dir` and its `global` links; a bare `set` stays local and `::`
/// reaches a global. With `lappend auto_path DIR` and a `tclIndex` /
/// `pkgIndex.tcl` in DIR holding `set plain 1`, `set ::rooted [info level]`
/// and `set ::seen_dir $dir`, tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0
/// all print `0 3 p` for the `tclIndex` and `0 1 p` for the `pkgIndex.tcl`
/// (`[info exists ::plain]`, `$::rooted`, `[file tail $::seen_dir]`). A
/// document without the frame keeps the fresh-interpreter rule: `$dir` is an
/// uncreated global and ends the script.
#[test]
fn an_entry_procedure_frame_holds_the_loader_locals_and_global_links() {
    let reached = |source: &str, release: &str, frame: bool| {
        let environment = tcl_registry::model::ingress::resolve_environment(release);
        let profile = environment.analyser_profile();
        let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::context_for_profile(profile),
            config,
        );
        let input = if frame {
            input.with_entry_procedure_frame(Arc::new(
                EntryProcedureFrame::new()
                    .with_global("auto_index")
                    .with_global("auto_path")
                    .with_local("dir", Some("/library/p".to_owned())),
            ))
        } else {
            input
        };
        crate::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name)
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::W123)
    };
    for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for (source, frame, expected) in [
            ("set seen $dir\nnosuch\n", true, true),
            ("set seen $dir\nnosuch\n", false, false),
            (
                "set auto_index(a) [list source [file join $dir a.tcl]]\nset auto_index(b) [list source [file join $dir b.tcl]]\nnosuch\n",
                true,
                true,
            ),
            ("set seen $auto_path\nnosuch\n", true, true),
            ("global g\nset g 1\nset seen $::g\nnosuch\n", true, true),
            ("set ::rooted 1\nset seen $::rooted\nnosuch\n", true, true),
            ("set plain 1\nset seen $::plain\nnosuch\n", true, false),
        ] {
            assert_eq!(
                reached(source, release, frame),
                expected,
                "{release}: frame {frame}: {source}"
            );
        }
    }
}
