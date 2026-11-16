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
use tcl_compiler::analyser::Analyser;

#[test]
fn itcl_proc_sites_keep_actual_quoted_and_braced_tail_geometry() {
    for call in ["Factory::make", "{Factory::make}", "\"Factory::make\""] {
        let source = format!("itcl::class Factory {{proc make {{}} {{return 1}}}}\n{call}");
        let analysis = Analyser::new().analyse(&source, "tcl8.6");
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
        let spans = itcl_class_proc_call_sites(&source, &analysis, profile, "::Factory", "make");
        let start = u32::try_from(source.rfind("make").unwrap()).unwrap();
        assert_eq!(
            spans,
            vec![tcl_lexer::Span::new(start, start + 4)],
            "{call}"
        );
    }
}

#[test]
fn itcl_proc_sites_refuse_dynamic_expanded_foreign_and_missing_inputs() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    for call in [
        "Factory::$member",
        "Factory::ma[set suffix]",
        "{*}{Factory::make}",
        "Factory::m\\ake",
    ] {
        let source = format!("itcl::class Factory {{proc make {{}} {{return 1}}}}\n{call}");
        let analysis = Analyser::new().analyse(&source, "tcl8.6");
        assert!(
            itcl_class_proc_call_sites(&source, &analysis, profile, "::Factory", "make").is_empty(),
            "{call}"
        );
    }
    let source = "itcl::class Factory {proc make {} {return 1}}\nFactory::make";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    assert_eq!(
        itcl_class_proc_call_sites(
            &source.replace("return 1", "return 2"),
            &analysis,
            profile,
            "::Factory",
            "make"
        ),
        [] as [tcl_lexer::Span; 0]
    );
    analysis.resolved_input = None;
    assert_eq!(
        itcl_class_proc_call_sites(source, &analysis, profile, "::Factory", "make"),
        [] as [tcl_lexer::Span; 0]
    );
}
