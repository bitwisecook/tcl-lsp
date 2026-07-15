# v2.1.9

**2.x alpha — pre-release channel.**

Another pre-release on the **2.x** line, where the ongoing Python → Rust
rewrite of tcl-lsp ships its alphas. It is opt-in: install it from the VS Code
Marketplace **pre-release** channel or the JetBrains Marketplace **eap**
channel, or download the pre-release VSIX / plugin / native binaries from this
GitHub release. The stable **1.x** line stays the default for everyone who has
not opted into pre-releases, and a `2.1.x` build never becomes the "latest"
GitHub release or the default Marketplace download.

This release is two deep-review passes — one across the compiler, one across
diagnostics — plus a cluster of fixes to the BIG-IP report generator's
printing and, more importantly, a privacy fix: the in-browser generator could
silently upload a device configuration instead of keeping everything local.

## New Features

- **Tk/ttk widget instance commands are understood.** `.t instate {…} {…}`,
  `$listbox curselection`, `$w tag configure …` and similar widget-instance
  calls were never resolved back to the widget class that created them, so
  none of the LSP's tooling — highlighting, hover, completion, or
  unknown-subcommand/arity diagnostics — could see into them. Every Tk widget
  constructor now records the class it creates, so calls on `.t`/`$w` resolve
  to the real widget's subcommand table; a mistyped widget method is now
  flagged like any other unknown subcommand.
- **Completion falls back to fuzzy matching.** When a typed fragment matches
  nothing by prefix, completion now offers the closest commands, switches,
  subcommands, variables and methods by edit distance — `lsaerch` offers
  `lsearch`, `lsort -ncoase` offers `-nocase`, `$bnaana` offers `$banana`.
  Ordinary prefix completions are unaffected.
- **New diagnostic W218** flags `args` used in a non-final parameter
  position (`proc p {args extra} {…}`), which C Tcl silently treats as an
  ordinary parameter rather than the variadic catch-all an author would
  expect.
- **More quick fixes.** Did-you-mean rename fixes for undefined-variable
  warnings and deprecated iRules commands (IRULE2002); deterministic
  rewrites for `append` → `lappend` (W104), unwrapping a redundant nested
  `expr` (W114), and `file join`-ifying a manually concatenated path (W201);
  arity errors (E002/E003/E005) now show the expected usage alongside the
  argument count.
- **The `tcl` CLI auto-detects each file's dialect** (`diag`/`lint`/`validate`),
  matching the language server instead of requiring an explicit `--dialect`.
- **Printing a BIG-IP report shows progress.** Large reports could take
  10-20 seconds to prepare for printing with no visible feedback; a staged
  toast now names what's happening (formatting iRules, rendering diagrams,
  opening the print dialog).

## Improvements

- **Diagnostic ranges are noticeably tighter.** Expression-shimmer warnings
  (S100/S101) now anchor the specific operand rather than the whole
  statement; W313 anchors the dynamic path argument; W110 anchors the `==`/
  `!=` operator itself; I230 quotes a bare `$n` condition the way the source
  actually spells it.
- **Numeric comparisons match tclsh exactly.** `==`, `<`, and the rest of the
  comparison operators now compare int/double operands using C Tcl's own
  rules instead of a lossy promotion to `f64`, fixing wide-integer and
  bignum boundary cases.
- **Go-to-definition and hover are namespace-aware.** Proc and class lookups
  now walk the enclosing namespace chain (current → ancestors → global)
  instead of matching by exact string, and command-option matching
  (`lsort`, `lsearch`, `trace`, `tcl::prefix match`, `string is`, …) now
  reproduces tclsh's abbreviation rules and error wording exactly.
- **Fewer false positives:** a `rename`d command's new name is no longer
  flagged as unknown; iRules reading a variable across events no longer
  draws a read-before-set warning; a dialect-disabled class/type definer no
  longer cascades a wall of follow-on "unknown command" warnings; comments
  containing ordinary punctuation like em-dashes or smart quotes no longer
  trip the Trojan-source scanner (invisible/direction-altering characters
  still do); `TK1003` no longer flags legal option-value or abbreviated-flag
  forms; a comment merely containing a word like "interactive" no longer
  flips a file's detected dialect.
- **W128 recognises `ClassName destroy`**, so a TclOO/snit instance obtained
  after its class was destroyed is now flagged the same way a destroyed
  instance already was.
- **Printed BIG-IP reports are more complete and more legible.** Diagnostics
  and optimiser suggestions found by the analyser now actually print under
  each iRule (previously only the underlines printed, with no messages);
  iRule flow diagrams print for every iRule, not just the ones expanded on
  screen before printing; a diagram smaller than the page prints at its
  natural size instead of being stretched up to full width with
  oversized labels; the tcl-lsp and f5-query marks render at a consistent
  size wherever they appear together.

## Bug Fixes

- **The in-browser BIG-IP report generator could silently upload a device
  configuration.** Its "is the local engine available" check tested
  `window.wasm_bindgen`, but the inlined engine only ever defines a bare
  `wasm_bindgen` global — so the check always failed and the generator
  silently fell back to a network backend. On static hosting (GitHub Pages)
  that surfaced as an outright `405` error; everywhere else it quietly broke
  the "nothing leaves your browser" guarantee. Both the standalone generator
  and every generated report now also enforce a strict
  network-blocking content-security-policy in the browser itself, so the
  no-upload guarantee no longer depends solely on application code.
- **A generated report's Print button, Ctrl/Cmd-P, and embedded query
  console could all be dead on arrival.** The inlined WebAssembly loader
  throws when the page's own URL is a `blob:` URL — exactly what the
  in-browser generator uses to open a finished report — which silently
  poisoned a shared binding and took the Print button, its keyboard
  shortcut, and the report's query console down with it. Reports opened
  from disk or a normal URL were unaffected, which is why this shipped
  unnoticed.
- **Reports and CLI/MCP version output showed a placeholder `0.1.0`**
  instead of the released version, and could claim to be a "dirty"
  developer build even when built by CI from a clean release tag.
- **`console eval` and `consoleinterp eval`/`record` script bodies are now
  highlighted** instead of rendering as one opaque unhighlighted string.
- **References, rename, and call hierarchy now find fully-qualified and
  relative namespace-qualified calls**, including a proc invoked by its
  fully-qualified name from inside a callback script — previously only
  exact-form matches within the same namespace were found.
