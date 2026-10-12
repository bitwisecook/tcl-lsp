# Formatter engine contracts

The formatter retains one complete source input and selected Registry context
for each document. Layout options remain separate from source grammar and
command advice. LSP formatting, format on save, the CLI, MCP and hosted callers
use the same core engine.

`format_tcl_with_input` and `range_formatting_with_input` accept the caller's
`ResolvedAnalysisInput`. It supplies the complete `LexerConfig`, source policy,
profile and structural `ContextRegistry`, including overlays and library axes.
The compatibility entry retains its documented lenient modern-Tcl context and
the supplied Registry generation; a heterogeneous Registry cannot select the
last inserted dialect as the document's provider. CLI inputs keep separate
source analyses even when their formatted output is joined.

## Selected source layout

`FormattingSourceLayout` retains the whole document, resolved input and command
realm. Each command's shared `OriginalRegistryWords` selects authored argument
roles, traits and presentation from its actual ContextRegistry. Effective
arguments map back only to their own written operands. Captured alias prefixes
and expanded values cannot borrow another word's source position.

Body and case-list layout consume the shared original script-body descriptors.
Lambda and parameter-list formatting use their original whole word and native
list value owners. Opaque native units, unavailable source geometry and malformed
list values preserve the original spelling. A reporting name or a nominal
command head cannot supply missing source roles.

These are source presentation capabilities. They do not establish an entered
frame, handler execution, native compiler admission, cell contents or Normal.
Expression bracing requires its separate bounded original literal equivalence
receipt. Native keyword changes require their own operand equivalence receipt;
source roles alone provide none. Explicit Logical compatibility formatting
retains its independent whole context and established rewrite policy.

## Whitespace and ranges

Whitespace geometry comes from complete original words under the full selected
lexer configuration. Trailing whitespace is removed only outside those words;
quoted and braced data, nested command words and bare Unicode units remain
protected. Backslash wrapping uses actual gaps between original command words,
including separately owned bracket scripts. Continuations inside nested data
words keep their original spelling. Expression wrapping uses the shared
expression lexer and preserves string and command terms; unavailable grammar,
malformed terms and comments decline wrapping.

Range indentation follows actual lexical bracket regions and selected source
body, case and lambda regions. A selection must contain complete words in its
selected script. A range that cuts through a data word, cooked body or
unavailable source region produces no edit. The raw document remains the owner
of replacement coordinates and line endings.

## Bounds and stability

The existing `MAX_FORMAT_DEPTH` of 128 bounds recursive layout. The source lexer
also reports its structural budget before semantic analysis begins, so a deeply
nested input can remain unchanged without expanding the stack or analysing an
unbounded body graph. Malformed input must stabilise, rather than gain a closer
on each formatting pass. Idempotence and fixed-input tests cover list shapes,
source data preservation, range boundaries and the unchanged depth-2000 case.

Formatter settings are declared once on `FormatterConfig` and generated into
editor configuration by `cargo xtask gen-editor-settings`. Existing docstrings
are retained; generation is an explicit action described in
[docstring-handling.md](docstring-handling.md).

## File-path anchors

- `rust/tcl-lsp-core/src/formatting/engine.rs`: selected source layout and reconstruction.
- `rust/tcl-lsp-core/src/formatting/source_layout.rs`: full-word whitespace and range boundaries.
- `rust/tcl-lsp-core/src/formatting/config.rs`: style settings and complete input propagation.
- `rust/tcl-lsp-core/src/formatting/mod.rs`: document and range edits with original line coordinates.
- `rust/tcl-compiler/src/registry_invocation/source_structure.rs`: sealed source schema.
- `rust/tcl-compiler/src/registry_invocation/source_scoped_body.rs`: shared original script regions.
- `rust/tcl-lsp-core/src/formatting/keywords.rs`: separately gated keyword rewrites.
- `rust/tcl-lsp-core/src/formatting/docstring.rs`: docstring parse and rendering.

## Failure modes

- Nominal or stale command metadata supplies roles to another source owner.
- Cooked text acquires invented original body offsets.
- Whitespace in a literal value is treated as command trivia.
- A partial range rewrites a containing data word.
- A source-role capability supplies runtime or general reflection equivalence.
- Unbounded semantic analysis starts before the structural depth guard.

## Discoverability

- [Design doc index](../README.md)
- [LSP feature providers](lsp-feature-providers.md)
- [Parsing contracts](parsing.md)
- [Original source formatting](../analysis/name-resolution-proofs/editor-original-source-formatting.md)
- [Original whitespace geometry](../analysis/name-resolution-proofs/original-source-whitespace-geometry.md)
