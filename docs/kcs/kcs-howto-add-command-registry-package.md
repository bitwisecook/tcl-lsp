# KCS: How do I add a third-party Tcl library to the command registry?

> **Audience:** Contributor
> **Type:** How-To

## Applies to

VS Code, Zed, JetBrains, Neovim, Helix, Emacs, Sublime Text, tcl-lsp CLI

## Question

How do I add first-class registry support for a third-party Tcl package
(hover docs, completion, arity checks, side-effect classification, taint
hints, call-graph integration) — the same way `tcllib` is wired in?

## Before you start

- The target package is widely used enough to justify being in the
  shipped registry rather than declared per-project via
  [stubs](kcs-howto-annotate-commands-with-stubs.md).
- You can enumerate the package's commands (names, arities, argument
  shapes) from upstream documentation.
- You have a checkout of the repo and can run `make rust-check` and
  `make test-rust` locally.

## Answer

Each command is one [`CommandSpec`](../GLOSSARY.md#commandspec) value in
the `tcl-registry` crate. Specs are grouped into *packs* — one directory
per library or dialect, one small Rust module per command inside it.
Adding a library means adding modules to a pack (or adding a new pack)
and listing them in that pack's spec builder. There is no decorator and
no registration side effect at import time — the builder is the single
registration point.

### 1. Choose the pack

| Distribution shape | Pack |
|---|---|
| Bundled with the Tcl core | `tcl` |
| Ships with Tcl (`http`, `msgcat`, `platform`, `tcltest`, …), or a standalone C extension | `stdlib` |
| A tcllib module | `tcllib` |
| Tk, [incr Tcl], Expect, argparse, ticklecharts | the pack of that name |
| F5 iRules, or iApps and tmsh | `irules`, `iapps` |
| An EDA vendor shell | **not this guide** — see below |

The core, stdlib, tcllib, Tk, [incr Tcl], argparse, and ticklecharts
packs are loaded into every registry. The iRules, iApps, Expect, and
BPF-Tcl packs load when their [dialect](../GLOSSARY.md#dialect) is
active.

**The EDA vendor libraries are not Rust modules.** `sdc_base` and the
five vendor packs are bundled `SpecTcl` loadables — `specs/*.tclspec`,
shipped beside the server executable and read by the pack loader
(`docs/design/spec-packs.md`). Adding or editing an EDA command means
editing the `.tclspec` file: the syntax is
`docs/design/spec-dsl-examples/README.md`, `tcl spec check` validates a
pack, and `rust/tcl-spectcl/tests/eda_loadables.rs` is the gate. None of
the steps below — no module, no `mod` line, no collector entry, no
rebuild — applies to them.

### 2. Write one module per command

Copy a neighbouring spec rather than starting from scratch — `puts` in
the core pack is a good, fully annotated model. Name the module after
the command, adding a trailing underscore where the bare name would
clash in Rust, and writing `::` as a double underscore (`http::geturl`
becomes `http__geturl`). Each module exposes a single `spec()` function
that returns the command's `CommandSpec`; you set only the fields the
command actually needs, and the rest come from the spec defaults.

Reach first for the summary and synopsis that drive hover and
completion, the arity that drives the E002 and E003 diagnostics, the
[subcommand](../GLOSSARY.md#subcommand) table if the command is an
ensemble, the argument roles that tell the analyser which words are
script bodies, variable names, expressions, or channels, and the
side-effect and taint declarations. Every available field is described
in the [command registry design
doc](../design/compiler/command-registry.md).

If you would rather fill in a form than an editor buffer,
`make spec-studio-wasm` builds the [spec
studio](../design/contracts/command-spec-studio.md), which edits a spec
in the browser and renders the finished registry module for you.

### 3. Register the modules

Declare each new module in its pack's `mod.rs` and add its `spec()` call
to the list that the pack's spec-builder function returns. For an
existing pack that is the whole job — the builder is the only thing the
registry calls. A brand-new pack additionally needs its builder wired
into the registry's own load path, next to the packs already there.

### 4. Gate it on `package require`

A command that exists only after a `package require` names the package
it needs on its own spec. Completion then offers it only once the
document requires that package, and the analyser reports W120 when it is
used without one. Declare the requirement rather than leaving the
command out of the pack: the spec still has to exist so that hover,
arity, and the call graph work inside a file that *does* require it.

A command that some dialects have and others do not declares its dialect
set instead, and a command whose availability depends on the version of
its owning package declares a lifecycle.

### 5. Model instance commands

A factory such as `sqlite3 db :memory:` creates a command whose name the
user chooses, so the registry — which is keyed on fixed names — cannot
hold a spec for it. Where the factory names its instance positionally,
the spec says which argument that is and which class the new command
belongs to, and later `db method …` calls resolve through that class.
The Tk widget commands (`button .b`, then `.b configure …`) work exactly
this way. Where a library does not fit that shape, users can still
declare the instance command with
[stubs](kcs-howto-annotate-commands-with-stubs.md).

### 6. Add tests

The registry's own suites already sweep every command in every dialect
and check each spec's own invariants, so a well-formed spec is covered
the moment it is registered. Add a focused case when the command has
behaviour worth pinning — an unusual arity, a subcommand set, a taint
sink, or event scoping. The suites generate real Tcl and iRules from the
registry and assert the resulting diagnostics; see the [registry
contract tests](../design/contracts/registry-contract-tests.md) for how
that works.

### 7. Refresh the generated files

Run `make codegen` to rebuild the generated catalogues, then
`make rust-check` to run the drift gates — including the one that
cross-checks the registry's core Tcl commands against the runtime that
backs them. The gates only report; they do not fix. Commit whatever
`make codegen` changed alongside the specs, or the gate fails in CI.

## How to tell it worked

- `tcl command-info <command>` prints the registry entry — arity,
  arguments, and side effects — instead of reporting an unknown command.
- A file that requires the package and then calls the command no longer
  raises the W123 "unresolved command" hint, and one that calls it
  *without* the `package require` raises W120.
- Hovering the command in your editor shows the summary and synopsis
  from the spec.
- `tcl callgraph` on a file using the library shows edges into its
  callbacks without a `# tcl-lsp: stub` block in the source.

## Related

- [How to annotate an external Tcl command with a stub](kcs-howto-annotate-commands-with-stubs.md)
  — the lighter-weight, per-project alternative when registry inclusion
  isn't warranted.
- [Command registry design doc](../design/compiler/command-registry.md)
- [Command spec studio](../design/contracts/command-spec-studio.md)
- [KCS index](README.md)
