# v2.1.10

**2.x alpha — pre-release channel.**

Another pre-release on the **2.x** line, where the ongoing Python → Rust
rewrite of tcl-lsp ships its alphas. It is opt-in: install it from the VS Code
Marketplace **pre-release** channel or the JetBrains Marketplace **eap**
channel, or download the pre-release VSIX / plugin / native binaries from this
GitHub release. The stable **1.x** line stays the default for everyone who has
not opted into pre-releases, and a `2.1.x` build never becomes the "latest"
GitHub release or the default Marketplace download.

This release is dominated by two large passes: a comprehensive rework of how
the compiler tracks value types (union types, per-array-element tracking,
exact big-integer arithmetic) and a soundness sweep across name resolution
(TclOO, autoload, `source` context, interpreters, command names held in
variables). Alongside those: a much more accurate model of Tcl-version and
BIG-IP-version availability, several VM correctness fixes, and a handful of
smaller diagnostic and editor fixes.

## New Features

- **BIG-IP-version-aware analysis.** A new `--bigip-version` CLI flag and
  `tclLsp.bigipVersion` setting narrow event, command, and profile validity
  (and the new W135/W136/W139 version diagnostics) to a specific BIG-IP
  release instead of the widest possible range.
- **First-class support for the `f5-tmsh` and `bpf` dialects, plus Tcl 9.1.**
  The tmsh shell (Tcl 8.5 host + `tmsh::` surface) and BIG-IP's eBPF Tcl
  runtime (Tcl 9.0-based) are now modelled as their own dialects rather than
  approximated by a neighbour.
- **New diagnostics:** W129 (command hidden in the current safe interpreter),
  W140 (use of an interpreter that was never created), W314 (a definition
  whose name has no absolute written form — e.g. `proc :`), and W137/W138
  (a `string is` class or `format`/`scan` specifier used below the Tcl
  version that introduced it).
- **Per-diagnostic severity override.** `tclLsp.diagnosticSeverity.<CODE>`
  lets any diagnostic's default severity (e.g. W211's faint hint) be raised
  or lowered without changing the underlying analysis.
- **VS Code: suppress diagnostics in diff editors.** The opt-in
  `tclLsp.suppressDiagnosticsInDiffEditors` setting hides a file's Tcl
  diagnostics while it is shown only in a diff/compare view (Source Control,
  "Compare With…"), so reviewing a change doesn't carry the analyser's
  squiggles; a file also open normally is unaffected, and closing the diff
  brings the squiggles straight back.
- **Go-to-definition, references, and rename reach further:** into
  autoloaded library files, colon-heavy proc/namespace names (`proc :`,
  trailing `::`), a TclOO method inherited from a base class defined in
  another file, and command names used in introspection calls (`info body`,
  `namespace which -command`, `trace add execution`).
- **Hover and inlay types show full unions.** A value with three or more
  merging types (e.g. from separate branches) now renders every member
  instead of collapsing to "unknown".

## Improvements

- **Array elements are tracked individually.** `arr(a)` and `arr(b)` are no
  longer conflated: hover and type inference report the right element's
  type, most false positives from treating a whole array as one value are
  gone, and a genuine type oscillation on the *same* element is still
  caught.
- **Exact big-integer arithmetic.** `expr`, constant folding, and the VM's
  math functions now match `tclsh` precisely at extreme values — huge
  exponents and shifts, `isqrt`, comparisons between a bignum and a double,
  and the exact domain errors C Tcl raises (`0.0 ** -1`, NaN operands, …).
- **A large sweep of dialect/version false positives is gone.** 8.5/8.6-era
  commands (`dict`, `lassign`, `apply`, `lmap`, `coroutine`, …) no longer
  false-positive under `f5-iapps`/`expect`/EDA dialects; completion and
  hover now correctly hide subcommands and options that don't exist yet at
  a buffer's effective Tcl version (e.g. `dict getwithdefault` under 8.6);
  Tk commands are never offered inside a vendor shell.
- **Variable rename and references follow every alias together** —
  `global v`, `variable v`, and `namespace upvar` declarations for the same
  cell are now treated as one unit instead of being edited separately (or
  missed).
- **Fewer unused-variable / read-before-set false positives.** `upvar 1
  $name local` (the dynamic-target accessor idiom) no longer flags W210; a
  global written at the top level and read only inside a proc no longer
  flags W211/W220.
- **VM re-entrancy and step traces are more faithful.** A parent-interpreter
  alias invoked from inside a coroutine resume, `lsort -command`, a trace
  callback, or an `after`/`vwait` callback no longer errors with "C stack
  busy" or mis-re-enters; execution step traces now observe opcode-inlined
  commands (`set`, `incr`, `return`, control flow), not just dispatched
  ones.

## Bug Fixes

- **Renaming a TclOO instance variable no longer corrupts the method.** The
  edit previously could span and destroy the whole method body; it now
  anchors precisely on the `variable` declaration.
- **VS Code's rename could refuse to start.** `prepareRename` only checked
  the local file, so renaming a symbol with no declaration in the buffer
  being edited (e.g. an autoloaded or sibling-defined proc) failed with
  "The element can't be renamed" even though the rename itself would have
  worked. It now resolves through the workspace like the rename request
  does.
- **Renaming a command held in a variable rewrites the right thing.** `set
  cmd target; $cmd` now rewrites the assignment when renaming `target`,
  never the `$cmd` use site.
- **TclOO resolution soundness fixes:** an external call to an unexported
  method is no longer resolved as if it were exported; per-object
  (`oo::objdefine`) methods on same-named objects in different scopes no
  longer collide; go-to-definition on a method call now follows the actual
  dispatch chain instead of landing on an arbitrary same-named method
  elsewhere.
- **`f5 fetch --transport ssh` now verifies the remote host key.** It
  previously accepted any presented key; it now checks against a persisted
  `known_hosts` store, trusting a host on first use and rejecting a
  different key on a later connection (failing closed if the store can't
  be written). Concurrent fetches against the same device no longer
  collide and overwrite each other's saved file.
- A renamed cross-interpreter alias no longer leaves a dangling command
  behind after the alias's target interpreter is deleted.
- A `source`d file with multiple physical declaration sites is now treated
  as the multiple distinct runtime identities it really has, instead of
  merging them under rename.
- Bare `break`/`continue` reaching a proc boundary now raises the same
  "invoked ... outside of a loop" error C Tcl does, instead of leaking an
  internal completion code.
