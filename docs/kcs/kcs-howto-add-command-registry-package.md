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
  shapes) from upstream docs. Read the real manual page for each
  command — a generic `cmd subcommand ?arg ...?` synopsis hides
  subcommands with materially different argument shapes.
- You have a checkout and can run `make prep-pr`.

## Answer

The registry lives in the `tcl-registry` crate. The pattern is one Rust
file per command, each exposing a `spec()` function that returns a
`CommandSpec`, and a `mod.rs` per package that lists the modules and
collects their specs.

This walkthrough uses `sqlite3` — the same package used as the running
stub example elsewhere — as a concrete target.

### 1. Decide the home

Pick the directory under `rust/tcl-registry/src/commands/` that matches
the package's distribution shape:

| Distribution shape | Folder |
|---|---|
| Bundled with the Tcl core | `tcl/` |
| Standard library package (Tk, http, msgcat, …) | `stdlib/` |
| tcllib package | `tcllib/` |
| Dialect-specific (iRules, iApps, Expect, itcl) | `irules/`, `iapps/`, `expect/`, `itcl/` |
| An EDA vendor shell | **not this guide** — see below |
| Standalone C extension (sqlite3, tdom, …) | `stdlib/` |

`sqlite3` is a standalone C extension that needs `package require
sqlite3`, so it belongs under `stdlib/`.

**The EDA vendor libraries are not Rust modules.** `sdc_base` and the five
vendor packs are bundled `SpecTcl` loadables — `specs/*.tclspec`, shipped
beside the server executable and read by the pack loader
([spec-packs.md](../design/spec-packs.md)). Adding or editing an EDA command
means editing the `.tclspec` file: the syntax is
[kcs-howto-write-a-tclspec-pack.md](kcs-howto-write-a-tclspec-pack.md),
`tcl spec check` validates a pack, and
`rust/tcl-spectcl/tests/eda_loadables.rs` is the gate. None of the steps
below — no module, no `mod` line, no collector entry, no codegen refresh —
applies to them.

### 2. Add one module per command

Create `rust/tcl-registry/src/commands/stdlib/sqlite3.rs`. Follow the
shape of an existing neighbour such as `msgcat__mc.rs` — a package
namespace is spelled with a double underscore in the filename. Each
module exposes a single `spec()`:

```rust
//! `sqlite3` command.
use crate::prelude::*;

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "sqlite3",
        required_package: Some("sqlite3"),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet { /* summary, synopsis, … */ }),
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
```

Spread `..CommandSpec::DEFAULT` rather than filling every field — the
struct is wide and the defaults are the neutral answer.

### 3. Wire it into the package

Add a `mod sqlite3;` line and a `sqlite3::spec(),` entry to the package's
`mod.rs` collector. Nothing else registers it; the collector is the only
list.

### 4. Model the instance command

`sqlite3 db :memory:` creates a command named `db`. The instance name is
user-chosen, so a fixed spec cannot name it. Declare the binding as
registry data instead — a `HandleBindingSpec` hung off the creating
command's `binds_handle` field says which argument names the variable
that receives the handle and which says its class. Never add a walker
arm that matches the command name; the whole point of the binding spec
is that every spelling C Tcl resolves to the same command resolves the
same way.

### 5. Add the metadata that drives other analyses

`CommandSpec` carries optional fields that other passes read:

| Field | What it drives |
|---|---|
| `arg_roles` / `arg_role_resolver` | Where script bodies, variables, channels, and patterns live. Feeds the call-graph scanner, the variable-usage analyser, and semantic highlighting. |
| `arity` | Arity diagnostics. |
| `side_effects` | Purity propagation, dead-store elimination, iRule taint flow. |
| `binds_handle` | Object-handle binding (see step 4). |
| `required_package` | Gates the command on a matching `package require`. |

A subcommand that takes a script needs an explicit `ArgRole::Body` on
that `SubCommand` entry — it is never inferred from arity or synopsis
text. If the script evaluates in a *different* interpreter from the
caller's, classify it as a cross-interpreter sink rather than a
same-interpreter one.

The full field reference is in the [command registry design
doc](../design/compiler/command-registry.md).

### 6. Add tests

Unit tests live beside the code, in the package's `mod.rs` or the command
module itself. Assert the spec is reachable by name, that its arity and
argument roles are what the manual page says, and — for a command with a
script argument — that a body actually recurses.

### 7. Refresh the generated editor assets

The registry is the source of truth for several generated files. Run
`make generate` and `make gen-editor-settings`, then commit the results
alongside the registry change — the drift gates in `make xtask-check`
fail if they are stale.

## How to tell it worked

- A Tcl file containing `package require sqlite3` followed by
  `sqlite3 db :memory:` no longer draws an unresolved-command diagnostic
  on `sqlite3`.
- Hovering `sqlite3` shows the synopsis from your `HoverSnippet`.
- `tcl callgraph` on a sqlite-using file shows edges into row callbacks
  *without* a `# tcl-lsp: stub` block in the source — the registry now
  knows the command shape directly.

## Related

- [How to annotate an external Tcl command with a stub](kcs-howto-annotate-commands-with-stubs.md)
  — the lighter-weight, per-project alternative when registry inclusion
  isn't warranted.
- [Command registry design doc](../design/compiler/command-registry.md)
- [Command spec studio](../design/contracts/command-spec-studio.md)
  — fill in a form instead of an editor buffer; renders the finished
  registry module or `.tclspec` for you.
- [How to write a `.tclspec` pack](kcs-howto-write-a-tclspec-pack.md)
  — the loadable route, and the only one for the EDA vendor libraries.
- [KCS index](README.md)
