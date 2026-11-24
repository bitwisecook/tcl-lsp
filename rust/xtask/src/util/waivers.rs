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

use std::collections::BTreeMap;

use rustc_lexer::TokenKind;

struct Comment<'a> {
    body: &'a str,
    standalone: bool,
}

/// Exact tags at the start of Rust line comments. A site's comment block ends
/// at the first blank line, code line or trailing comment on earlier code.
pub(crate) struct Waivers<'a> {
    comments: BTreeMap<usize, Comment<'a>>,
    first_code_line: Option<usize>,
}

impl<'a> Waivers<'a> {
    pub(crate) fn new(source: &'a str) -> Self {
        let mut comments = BTreeMap::new();
        let mut first_code_line = None;
        let mut offset = 0;
        let mut line = 1;
        let mut line_start = 0;
        for token in rustc_lexer::tokenize(source) {
            let end = offset + token.len;
            let lexeme = &source[offset..end];
            if token.kind == TokenKind::LineComment {
                comments.insert(
                    line,
                    Comment {
                        body: lexeme[2..].trim_start_matches(['/', '!']).trim(),
                        standalone: source[line_start..offset].trim().is_empty(),
                    },
                );
            } else if !matches!(
                token.kind,
                TokenKind::Whitespace | TokenKind::BlockComment { .. }
            ) {
                first_code_line.get_or_insert(line);
            }
            line += lexeme.bytes().filter(|byte| *byte == b'\n').count();
            if let Some(newline) = lexeme.rfind('\n') {
                line_start = offset + newline + 1;
            }
            offset = end;
        }
        Self {
            comments,
            first_code_line,
        }
    }

    fn tagged(&self, line: usize, tag: &str) -> Option<&'a str> {
        let payload = self.comments.get(&line)?.body.strip_prefix(tag)?;
        Some(payload.trim())
    }

    pub(crate) fn attached(&self, line: usize, tag: &str) -> Option<&'a str> {
        self.site(line, tag)
            .map(|(_, payload)| payload)
            .filter(|payload| !payload.is_empty())
    }

    pub(crate) fn site(&self, line: usize, tag: &str) -> Option<(usize, &'a str)> {
        if let Some(payload) = self.tagged(line, tag) {
            return Some((line, payload));
        }
        let mut last = line;
        while let Some(previous) = last.checked_sub(1) {
            if !self
                .comments
                .get(&previous)
                .is_some_and(|comment| comment.standalone)
            {
                break;
            }
            if let Some(payload) = self.tagged(previous, tag) {
                return Some((previous, payload));
            }
            last = previous;
        }
        None
    }

    pub(crate) fn leading(&self, tag: &str) -> Option<(usize, &'a str)> {
        self.comments
            .iter()
            .take_while(|(line, comment)| {
                self.first_code_line.is_none_or(|first| **line < first) && comment.standalone
            })
            .find_map(|(&line, _)| self.tagged(line, tag).map(|payload| (line, payload)))
    }

    pub(crate) fn continuation(&self, line: usize) -> impl Iterator<Item = &'a str> + '_ {
        let standalone = self
            .comments
            .get(&line)
            .is_some_and(|comment| comment.standalone);
        (line + 1..).map_while(move |next| {
            let comment = self.comments.get(&next)?;
            (standalone && comment.standalone && !comment.body.is_empty()).then_some(comment.body)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_tag_belongs_in_the_leading_comments() {
        let tag = "value-transfer-ok(file):";
        let source = "// value-transfer-ok(file): reason\nfn f() {}";
        assert_eq!(Waivers::new(source).leading(tag), Some((1, "reason")));
        let source = "fn f() {}\n// value-transfer-ok(file): reason\nfn g() {}";
        assert_eq!(Waivers::new(source).leading(tag), None);
    }

    #[test]
    fn site_tags_are_exact_and_comment_scoped() {
        let tags = [
            "drift-ok:",
            "dialect-drift-ok:",
            "number-drift-ok:",
            "segmentation-drift-ok:",
            "presentation-ok:",
            "retired-api-ok:",
            "one-oracle-ok:",
            "registry-axis-ok:",
            "registry-axis-ok(file):",
            "value-transfer-ok:",
            "value-transfer-ok(file):",
            "grammar-bypass-ok:",
        ];
        for tag in tags {
            let source = format!(
                "// {tag} the reason\n// more detail\ncall();\nother();\n\
                 after(); // {tag} trailing\nnext();\n\
                 let text = r#\"// {tag} literal\"#;\n\
                 /* // {tag} block */ block();\n\
                 // prose mentions {tag} here\nprose();\n\
                 // {tag} separated\n\nblank();\n"
            );
            let waivers = Waivers::new(&source);
            for candidate in tags {
                assert_eq!(
                    waivers.attached(3, candidate),
                    (candidate == tag).then_some("the reason")
                );
                assert_eq!(
                    waivers.attached(5, candidate),
                    (candidate == tag).then_some("trailing")
                );
            }
            for line in [4, 6, 7, 8, 10, 13] {
                assert_eq!(waivers.attached(line, tag), None);
            }
        }
    }
}
