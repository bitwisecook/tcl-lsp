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
fn actual_head_and_body_words_keep_complete_variable_geometry() {
    let source = "set cmd puts\n${cmd} value\n${cmd}Suffix value\neval ${cmd}\neval ${arr(key)}\n";
    let mut analyser = Analyser::new();
    analyser.analyse(source, "tcl9.0");
    let config = analyser.file_lexer_config();
    let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    assert!(analyser.head_is_whole_word_variable(commands[1].argv[0]));
    assert!(!analyser.head_is_whole_word_variable(commands[2].argv[0]));
    let first = commands[3].argv[1];
    analyser.record_var_body_const_dispatch(first, &[]);
    assert_eq!(
        analyser.pending_const_dispatches.last().unwrap().var_name,
        "cmd"
    );
    analyser.record_var_body_const_dispatch(commands[4].argv[1], &[]);
    assert_eq!(
        analyser.pending_const_dispatches.last().unwrap().var_name,
        "arr(key)"
    );
    analyser.source = format!("{}{}", analyser.source, "# foreign source").into();
    assert!(!analyser.head_is_whole_word_variable(commands[1].argv[0]));
    assert!(
        analyser
            .original_whole_variable_source_word(first, false)
            .is_none()
    );
}
