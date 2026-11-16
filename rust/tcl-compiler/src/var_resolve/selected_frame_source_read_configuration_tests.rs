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
fn direct_selected_frame_keeps_current_script_configuration_not_caller_grammar() {
    let registry = CommandRegistry::build_default();
    let parent_config = tcl_lexer::LexerConfig::default();
    let mut child_config = parent_config;
    child_config.strict_quoting = !parent_config.strict_quoting;
    let mut parent = ResolveContext::for_function("::parent");
    parent.source_read_configuration = SourceReadConfiguration::Selected(parent_config);
    let mut child = ResolveContext::for_function("::child");
    child.source_read_configuration = SourceReadConfiguration::Selected(child_config);
    child.caller = Some(std::sync::Arc::new(parent));
    let selected = child
        .selected_frame_context(FrameLevel::Relative(1))
        .unwrap();
    assert_eq!(selected.read_lexer_config(&registry), Some(child_config));
    assert_eq!(
        child.caller.as_ref().unwrap().read_lexer_config(&registry),
        Some(parent_config)
    );
}
