# v2.1.5

**2.x alpha — pre-release channel.**

Another pre-release on the **2.x** line, where the ongoing Python → Rust
rewrite of tcl-lsp ships its alphas. It is opt-in: install it from the VS Code
Marketplace **pre-release** channel or the JetBrains Marketplace **eap**
channel, or download the pre-release VSIX / plugin / native binaries from this
GitHub release. The stable **1.x** line stays the default for everyone who has
not opted into pre-releases, and a `2.1.x` build never becomes the "latest"
GitHub release or the default Marketplace download.

## New Features

- **Arity checking for every kind of call target.** Call-arity diagnostics no
  longer apply only to plain `proc` definitions. They now understand
  `interp alias`, commands renamed with `rename`, and TclOO methods, so a
  wrong-argument-count call is reported wherever the target was actually
  defined.
- **Command-prefix callbacks are first-class.** Callbacks passed as a command
  prefix — `trace add variable`, `after`, `fileevent`, and friends — now
  participate in go-to-definition, find-references, arity checking, and the call
  graph, including across files.
- **Commands from `auto_path` library files are recognised.** A command defined
  in a library file on the `auto_path` no longer draws the unresolved-command
  hint `W123`.
- **Dialect-aware interpreter globals.** A special-variable registry now models
  the globals each dialect actually provides (`::errorInfo`, `::tcl_platform`,
  and the rest), so they hover, complete, and stop being flagged as undefined.
- **tcltest definitions in outlines.** `test` definitions surface in the
  document and workspace symbol outlines.

## Improvements

- **Snappier semantic highlighting.** Semantic-token requests are now
  prioritised ahead of the coarse whole-workspace analysis, so colouring appears
  promptly on a large file instead of waiting behind a full scan.
- **`dict for` and `dict map` bodies are analysed.** Their bodies are lowered
  into the analysis control-flow graph, so diagnostics, data flow, and the call
  graph see inside them.
- **Sharper variable-name scanning.** A long tail of edge cases in how variable
  names are recognised — braced names, namespace-qualified names, array element
  targets, and similar — now resolve correctly.
- **Zed editor assets are generated from the registry.** The Zed extension's
  tree-sitter highlight queries are produced from the command registry and
  guarded by a drift gate, so Zed highlighting tracks the registry the same way
  the VS Code and JetBrains assets already did.

## Bug Fixes

- Twenty-seven tracked issues across the compiler, the language server, and the
  supporting tooling.
- Compiler correctness: miscompiles in sparse conditional constant propagation,
  inlining, and constant folding; `switch` fall-through into a `default` arm;
  code sinking past a redefinition of the right-hand side's read set.
- Panics fixed: out-of-bounds reads on a trailing backslash and on a ghost `]`
  in the lexer; multibyte slicing in `subst -nocommands`, in certificate common
  names, and in `string last` on an empty haystack; a config-fold panic and a
  semantic-tokens panic in the server.
- Language-server correctness across navigation, rename, formatting, and minify.
- Taint tracking now propagates sources through `expr` command substitutions and
  matches subcommands by prefix abbreviation the way Tcl does.
- The eBPF backend uses signed division and modulo for Tcl integers, and rejects
  stray top-level statements when a `when` block is present.
- F5 Distributed Cloud translation preserves match criteria through `if`/`else`,
  `switch` mode, and fall-through.
- The runtime parses list and string indices and counts with the correct radix.

## Breaking Changes

- **`samples/for_f5_query/sysadmin/monitor_timeline.py` is removed.** It wrapped
  a renderer that no longer exists. Use `f5 query --render gantt`, which
  reproduces its output; tune the resolution with
  `--render-opt unit-minutes=N`.
- **Building from source now requires Rust 1.97.** The workspace tracks the
  current stable toolchain. This does not affect users of the published
  binaries, extensions, or plugins.
