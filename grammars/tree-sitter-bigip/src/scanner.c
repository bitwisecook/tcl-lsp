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

// External scanner for the body of an `ltm rule` — an iRule, i.e. Tcl.
//
// The body is consumed as a single raw token so that `injections.scm` can hand
// its exact byte range to the Tcl parser. Doing this inside the grammar as
// balanced-brace token soup looks fine until a brace appears inside a Tcl
// string or behind a backslash, at which point the body node ends in the wrong
// place and the injected Tcl is truncated.
//
// The rule this implements is Tcl's own: inside a braced word, braces must
// balance, and a backslash escapes the next character (so `\{` is a literal
// brace and does not open a group). That is the whole rule — Tcl does *not*
// exempt braces inside quotes from the balance requirement, so neither do we.

#include "tree_sitter/parser.h"

enum TokenType {
  RULE_BODY,
};

void *tree_sitter_bigip_external_scanner_create(void) { return NULL; }

void tree_sitter_bigip_external_scanner_destroy(void *payload) { (void)payload; }

unsigned tree_sitter_bigip_external_scanner_serialize(void *payload, char *buffer) {
  (void)payload;
  (void)buffer;
  return 0;
}

void tree_sitter_bigip_external_scanner_deserialize(void *payload, const char *buffer,
                                                    unsigned length) {
  (void)payload;
  (void)buffer;
  (void)length;
}

bool tree_sitter_bigip_external_scanner_scan(void *payload, TSLexer *lexer,
                                             const bool *valid_symbols) {
  (void)payload;

  if (!valid_symbols[RULE_BODY]) {
    return false;
  }

  // The rule's opening `{` has already been consumed by the grammar, so we
  // start one level in and stop *before* the `}` that closes it — leaving that
  // brace for the grammar, which expects it.
  int depth = 0;
  bool saw_content = false;

  for (;;) {
    if (lexer->eof(lexer)) {
      // Unterminated body. Emit what we have rather than failing the whole
      // parse: a config being edited is unbalanced most of the time.
      break;
    }

    int32_t c = lexer->lookahead;

    if (c == '\\') {
      // A backslash escapes the next character, whatever it is — including a
      // brace, which then does not count towards the balance.
      lexer->advance(lexer, false);
      if (!lexer->eof(lexer)) {
        lexer->advance(lexer, false);
      }
      saw_content = true;
      lexer->mark_end(lexer);
      continue;
    }

    if (c == '}' && depth == 0) {
      break;
    }

    if (c == '{') {
      depth++;
    } else if (c == '}') {
      depth--;
    }

    lexer->advance(lexer, false);
    saw_content = true;
    lexer->mark_end(lexer);
  }

  lexer->result_symbol = RULE_BODY;
  return saw_content;
}
