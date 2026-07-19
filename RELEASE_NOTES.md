# v2.1.11

**2.x alpha — pre-release channel.**

A small follow-up pre-release on the **2.x** line, where the ongoing Python →
Rust rewrite of tcl-lsp ships its alphas. It is opt-in: install it from the
VS Code Marketplace **pre-release** channel or the JetBrains Marketplace
**eap** channel, or download the pre-release VSIX / plugin / native binaries
from this GitHub release. The stable **1.x** line stays the default for
everyone who has not opted into pre-releases, and a `2.1.x` build never
becomes the "latest" GitHub release or the default Marketplace download.

This release addresses five Tcl name-resolution reports filed against
v2.1.10's TclOO / `apply` / `expr` handling in the Rust LSP. Four needed a
fix, covered below; the fifth (`$obj method` references) was already correct
and only gained regression coverage.

## Bug Fixes

- **`apply` lambda argument list is highlighted correctly.** A bare
  (unbraced) argument list in `apply {dir {…}}` was painted as a plain
  string instead of being recognised as a parameter declaration; braced
  argument lists and `proc` were already correct and are unaffected.
- **`pkgIndex.tcl`'s implicit `$dir` no longer flags as read-before-set.**
  The package loader sets `dir` before a `pkgIndex.tcl` script runs, so
  reading it there is always safe. The suppression is scoped to files
  literally named `pkgIndex.tcl`; other undefined reads there, and `$dir`
  reads elsewhere, still flag as before.
- **`my method` dispatch nested in a command substitution is found again.**
  A call such as `return [my getOptions $key]` was previously missed by
  find references and the member reference-count lens; it now resolves at
  parity with a top-level `my` call.
- **`::tcl::mathfunc::<fn>` functions used inside a nested `expr` are
  tracked.** `[expr {Foo()}]` written inside a command substitution
  previously recorded no invocation of the backing proc, so references,
  rename, and arity checking missed it.
- **`my`/`$obj` method dispatch is found inside compound and quoted words.**
  A call embedded in a bareword or double-quoted compound word — for
  example `return "opts: [my get]"` or `[$b get]-tail` — was missed because
  the segmenter merges such a word into a single token; references and
  rename now recover the nested `[…]` call from both quoted and compound
  words. A literal `[…]` inside a braced `{…}` word is correctly left
  untouched.
