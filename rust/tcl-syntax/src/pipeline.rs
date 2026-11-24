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

/// The command text supplied as a pipeline filename to `open`.
#[must_use]
pub fn open_pipeline_script(filename: &[u8]) -> Option<&[u8]> {
    filename.strip_prefix(b"|")
}

/// Spell a pipeline filename from a command in the `open` pipeline grammar.
#[must_use]
pub fn open_pipeline_filename(command: &str) -> String {
    format!("|{command}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_first_byte_selects_the_open_pipeline_grammar() {
        assert_eq!(open_pipeline_script(b"file|name"), None);
        assert_eq!(open_pipeline_script(b" | command"), None);
        assert_eq!(open_pipeline_script(b"|"), Some(b"".as_slice()));
        let filename = open_pipeline_filename(" {command name} | next");
        assert_eq!(
            open_pipeline_script(filename.as_bytes()),
            Some(b" {command name} | next".as_slice())
        );
    }
}
