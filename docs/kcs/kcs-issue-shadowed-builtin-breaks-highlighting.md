# KCS: `else`/`elseif` look wrong and a bareword builtin breaks bracket colour

> **Audience:** Contributor
> **Type:** Issue

## Applies to

vs-code, jetbrains, sublime-text

## Symptom

Two related syntax-highlighting glitches, reported in issue #637:

1. The `else` and `elseif` words in an `if` chain (and `on`, `trap`,
   `finally` in a `try` chain) are coloured as plain strings instead of
   keywords, so they do not match the `if`/`try` keyword colour.
2. When a built-in command name is used as a plain argument word — for
   example `proc` in `dict set frame proc "asasdas asd"` — the closing
   braces at the end of the surrounding block lose their bracket colour.
   Renaming the word to anything that is not a built-in (e.g. `proc1`)
   fixes it.

## Operational context

The two glitches come from two different layers.

**Structural keywords (`else`/`elseif`/`on`/`trap`/`finally`).** Semantic
tokens classify a word as a keyword only at the command-name position
(argument 0). These words sit at *argument* positions inside `if`/`try`,
so they fell through to the default classifier and were emitted as
strings. The command's `arg_role_resolver` already walks the chain to map
body and expression arguments, but it simply skipped the keyword words.

**Bareword builtin breaking brackets.** This is a TextMate-grammar
fault, not the semantic-token layer (which correctly tags the bareword as
a string). The `proc`/`method` definition rule matched
`(proc|method)\s+(\S+)` anywhere the word is preceded by whitespace,
including argument position. The `\S+` name capture then swallowed the
opening `"` of the following quoted word, so the string's begin/end
scoping ran away to the end of the file and the trailing `}` braces were
scoped as string content instead of brackets.

## Decision rules / contracts

1. Structural keyword words carry the registry role
   [`ArgRole.KEYWORD`](../../compiler/registry/signatures.py). The
   `if`/`try` `arg_role_resolver`s mark `then`/`elseif`/`else` and
   `on`/`trap`/`finally` with it, and the semantic-token collector emits
   those argument positions as `keyword`. Adding `KEYWORD` to a position
   that previously had no role is inert for every other role consumer —
   they filter by the roles they care about.
2. The TextMate proc/method name capture excludes string, brace, and
   bracket delimiters (`[^\s"{}\[\];$]+`). A bareword `proc` followed by
   `"` therefore fails to match, leaving the quoted word's scoping intact.
   The grammar cannot know command position, so a bareword `proc value`
   may still mis-tag `value` cosmetically — semantic tokens override the
   colour, and the structural string/brace scoping is what matters.
3. The VS Code grammar is canonical. JetBrains copies it at build time
   (and the committed copy is kept in sync); Sublime's hand-written
   `Tcl.sublime-syntax` carries the same fix and the versioned / iRule
   grammars `extends` it. Zed, Helix, and Neovim use tree-sitter
   (structural, unaffected); Emacs uses font-lock.

## File-path anchors

- `compiler/registry/signatures.py` — `ArgRole.KEYWORD`
- `dialects/tcl/if_.py`, `dialects/tcl/try_.py` — resolvers mark keyword positions
- `server/features/_semantic_tokens/_collect.py` — emits keyword-role args
- `editors/vscode/syntaxes/tcl.tmLanguage.json` — canonical proc/method rule
- `editors/sublime-text/Tcl.sublime-syntax` — `proc-definition`

## Failure modes

- A new structural keyword (a future `if`/`try` sub-word) is added to a
  resolver's flow but not marked `KEYWORD`, so it renders as a string.
- The proc/method name class is widened back to `\S+`, re-introducing the
  quote-swallowing bracket bug.
- The committed JetBrains grammar copy drifts from the canonical VS Code
  grammar (it is overwritten by the build, but a stale commit misleads
  reviewers).

## Test anchors

- `tests/test_semantic_tokens.py` — `test_if_else_elseif_are_keywords`,
  `test_try_on_finally_are_keywords`,
  `test_builtin_name_as_bareword_arg_is_string`

## Related

- [KCS index](README.md)
- [semantic tokens feature](features/kcs-feature-semantic-tokens.md)
- [command registry](../design/compiler/command-registry.md)
