# v2.1.13

**2.x alpha — pre-release channel.**

Another pre-release on the **2.x** line, where the ongoing Python → Rust
rewrite of tcl-lsp ships its alphas. It is opt-in: install it from the VS Code
Marketplace **pre-release** channel or the JetBrains Marketplace **eap**
channel, or download the pre-release VSIX / plugin / native binaries from this
GitHub release. The stable **1.x** line stays the default for everyone who has
not opted into pre-releases, and a `2.1.x` build never becomes the "latest"
GitHub release or the default Marketplace download.

v2.1.12's JetBrains build failed its own packaging check, which correctly
blocked *both* the VS Code and JetBrains Marketplace publishes for that tag —
so nothing from that cycle reached either Marketplace. This release fixes
the JetBrains packaging bug and otherwise carries the same content forward:
six more confirmed fixes from the issue #923 differential-audit campaign — a
tclsh-vs-analyser sweep across tricky TclOO, namespace, and `interp` idioms —
to reference resolution, go-to-definition, and rename, plus platform-targeted
packaging for VS Code and JetBrains.

## New Features

- **VS Code: platform-targeted VSIX packages.** The Marketplace now serves
  six small, single-binary VSIX packages (Windows, Linux, and macOS, each on
  x64/arm64) alongside the existing universal package, so most installs pull
  down one platform's binary instead of the full multi-platform bundle. The
  universal VSIX remains available for riscv64 Linux and manual
  "Install from VSIX" side-loading.
- **JetBrains: the plugin bundles a native server for every supported
  platform.** Previously it shipped only a Linux x64 binary regardless of
  the host, so macOS, Windows, and ARM users had no working install. The
  plugin now bundles one binary per platform (matching the VS Code universal
  build) and picks the right one at runtime.

## Bug Fixes

- **JetBrains plugin packaging includes the bundled native server binaries
  again.** v2.1.12's plugin build silently produced a package missing all
  six platform binaries: `prepareSandbox`'s `into("$pluginName/server")`
  stringified an unresolved Gradle provider instead of the plugin name,
  so the staged binaries landed under the wrong directory and never made
  it into the distributable zip. The plugin's own packaging check caught
  this and failed the build rather than shipping a broken package, which
  is why v2.1.12 never reached the JetBrains or VS Code Marketplace.
- **`superclass`, `mixin`, and (incr Tcl) `inherit` targets are tracked as
  class references.** Find All References on a base class previously
  returned only its own declaration, missing every subclass that named it,
  and renaming a class left those subclasses dangling. Covers TclOO,
  `oo::configurable`, `[incr Tcl]`, `self mixin`, `oo::objdefine mixin`, and
  `forward` targets, cross-file.
- **Fully- and relatively-qualified namespace variable references now
  resolve.** Hover, go-to-definition, references, and rename never resolved
  a `::`-qualified variable read like `$::ns::var` or `$ns::var` at all;
  they now resolve via the same namespace-path lookup already used for
  qualified command resolution.
- **Definition spans for the second and later parameters in a parameter
  list are correct again.** Every parameter after the first in a
  proc/`apply`/TclOO-method parameter list previously lost its own
  definition span and fell back to pointing at the proc or method name.
- **A definition nested inside another proc or class body no longer
  outranks a real builtin of the same name.** The "rename the builtin away,
  install a same-named shadow, restore it" idiom was permanently shadowing
  the builtin for every call site in the workspace, including ones that run
  after the shadow is renamed back off.
- **`interp create -safe NAME` and `NAME eval {...}` are tracked
  correctly.** The created interpreter's name was never registered as a
  known command (causing false-positive "unknown command" diagnostics), and
  the handle-call spelling of `interp eval` (`NAME eval {...}`, the more
  common form in real code) was not isolated from same-named procs in
  unrelated interpreters. Renaming or deleting an interpreter's handle
  command now correctly retires its tracked state.
- **Dynamic (`$var`) `namespace eval` and `oo::define` targets no longer
  merge unrelated call sites that happen to reuse a variable name.** Two
  lexically unrelated `namespace eval $name {...}` or `oo::define $class
  ...` sites sharing a variable name previously collapsed into the same
  scope, so go-to-definition/references from one could jump into a
  completely unrelated proc, and `oo::define` could merge methods from
  unrelated call sites into one class's document-symbol range.
