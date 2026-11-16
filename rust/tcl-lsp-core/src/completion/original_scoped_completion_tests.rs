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
fn original_scoped_completion_uses_source_body_receipts_and_current_owners() {
    // Implementation contract: naming.core.original-scoped-body-completion
    // docs/design/analysis/name-resolution-proofs/original-scoped-body-completion.md
    let source = "::report::defstyle st {} {\n    to\n}\n";
    let mut analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
    let image = tcl_lexer::SourceImage::document(source);
    let cursor = u32::try_from(source.find("    to").unwrap() + 6).unwrap();
    let body = analysis
        .original_scoped_body_in_source(&image, analysis.body_lexer_config.unwrap(), cursor)
        .expect("original scoped source body");
    assert!(body.environment().command("top").is_some());
    assert_eq!(body.original_body().image(), &image);
    analysis.scoped_command_regions.clear();
    let profile = analysis.resolved_profile().unwrap();
    let labels = completions(source, 1, 6, &analysis, None, None, profile)
        .into_iter()
        .map(|item| item.label)
        .collect::<Vec<_>>();
    assert!(labels.iter().any(|label| label == "top"), "{labels:?}");
    assert_eq!(
        completions(
            &source.replace("st", "zz"),
            1,
            6,
            &analysis,
            None,
            None,
            profile
        ),
        [] as [crate::completion::CompletionItem; 0]
    );
    for prefix in [
        "namespace eval ::report {}\nproc ::report::defstyle {args} {}\n",
        "namespace eval ::report {}\nproc ::report::defstyle {args} {}\nrename ::report::defstyle {}\n",
    ] {
        let custom = format!("{prefix}{source}");
        let mut blocked = tcl_compiler::analyser::Analyser::new().analyse(&custom, "tcl8.6");
        blocked
            .scoped_command_regions
            .push(tcl_compiler::analyser::types::ScopedBodyRegion {
                span: tcl_lexer::Span::new(0, u32::try_from(custom.len()).unwrap()),
                env: body_environment(),
            });
        let index = tcl_lexer::LineIndex::new(&custom);
        let cursor = u32::try_from(custom.find("    to").unwrap() + 6).unwrap();
        let position = index.position_at_utf16(cursor, &custom);
        assert!(
            scoped_env_at(
                &blocked,
                &custom,
                position.line,
                position.character.get(),
                &index
            )
            .is_none()
        );
    }
}
fn body_environment() -> &'static tcl_registry::scoped::ScopedCommandEnv {
    &tcl_registry::scoped::REPORT_DEFSTYLE_ENV
}
