// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

// Adapt the VS Code extension's authoritative Tcl TextMate grammar to Monaco.
// Semantic tokens still supply the precise registry-backed paint; this is the
// shared lexical base visible before those tokens arrive and while a hidden
// editor is being revealed.

import type * as monaco from "monaco-editor/editor/editor.api.js";
import * as oniguruma from "vscode-oniguruma";
import * as textmate from "vscode-textmate";

class GrammarState implements monaco.languages.IState {
  constructor(readonly stack: textmate.StateStack) {}

  clone(): GrammarState {
    return new GrammarState(this.stack);
  }

  equals(other: monaco.languages.IState): boolean {
    return other instanceof GrammarState && this.stack.equals(other.stack);
  }
}

/** Register `languageIds` against the one shared Tcl grammar. */
export async function registerTclGrammar(
  monacoApi: typeof monaco,
  languageIds: string[],
  grammarUrl: string,
  onigurumaUrl: string,
): Promise<void> {
  const onigBytes = await fetch(onigurumaUrl).then((response) => {
    if (!response.ok) throw new Error(`could not load Oniguruma (${response.status})`);
    return response.arrayBuffer();
  });
  await oniguruma.loadWASM(onigBytes);

  const registry = new textmate.Registry({
    onigLib: Promise.resolve({
      createOnigScanner: (patterns) => new oniguruma.OnigScanner(patterns),
      createOnigString: (text) => new oniguruma.OnigString(text),
    }),
    loadGrammar: async (scopeName) => {
      if (scopeName !== "source.tcl") return null;
      const response = await fetch(grammarUrl);
      if (!response.ok) throw new Error(`could not load the Tcl grammar (${response.status})`);
      // `grammarUrl` is content-keyed with a query string; passing it as the
      // filename makes vscode-textmate miss the `.json` suffix and attempt to
      // parse the JSON as a plist.
      return textmate.parseRawGrammar(await response.text(), "tcl.tmLanguage.json");
    },
  });
  const grammar = await registry.loadGrammar("source.tcl");
  if (!grammar) throw new Error("the Tcl grammar did not declare source.tcl");

  for (const languageId of languageIds) {
    monacoApi.languages.setTokensProvider(languageId, {
      getInitialState: () => new GrammarState(textmate.INITIAL),
      tokenize: (line, state) => {
        const current = state instanceof GrammarState ? state.stack : textmate.INITIAL;
        const result = grammar.tokenizeLine(line, current);
        return {
          endState: new GrammarState(result.ruleStack),
          tokens: result.tokens.map((token) => ({
            startIndex: token.startIndex,
            // Monaco's themes match dotted token prefixes in the same manner
            // as TextMate scope selectors, so preserve the most-specific
            // scope emitted by the extension grammar.
            scopes: token.scopes[token.scopes.length - 1] ?? "source.tcl",
          })),
        };
      },
    });
  }
}
