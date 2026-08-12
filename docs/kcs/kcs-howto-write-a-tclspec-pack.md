# KCS: How do I write a SpecTcl pack?

> **Audience:** User
> **Type:** How-To

## Applies to

all-editors, MCP

## Question

I want to describe my own package's commands as a `.tclspec` file. What do
I write, where does it go, and what happens once I save it?

## Before you start

If you have not chosen an authoring route yet, start with [creating a
command spec without knowing Rust](kcs-howto-create-command-specs-without-rust.md).
This note is the quickstart once you have: the minimal shape, the three
places a pack can live, and — because the loader is still landing — what
works today versus what is coming.

## Answer

### The minimal pack

A `.tclspec` file opens with `speclib`, names your library and a version,
and holds one `command` block per command:

```tcl
speclib mylib 1 {
    command mylib::with_var {
        arity 2 3
        arg 0 -role VarWrite
        arg 1 -role Body
        hover -summary {Run a script with a caller variable bound.} \
              -returns {The script's result.}
    }
}
```

Save it as `mylib.tclspec`. Because `.tclspec` is its own dialect, opening
it in any supported editor gives you highlighting, completion, and
diagnostics for a misspelled trait or role — the same experience a
built-in command gets, from the same machinery.

### Where it goes

Three tiers, nearest wins:

- **Workspace** — beside a `tclpkg.tcl` package manifest, or under a
  `.tcl-lsp/` directory in your project.
- **User** — your platform config directory
  (`~/.config/tcl-lsp/specs/` on Linux; the macOS and Windows equivalents),
  loaded for every workspace.
- **Bundled** — shipped with tcl-lsp itself; you never write to this one.

A command name your pack shares with a shipped command loses to the
shipped one, unless you write `command NAME -override { … }`.

### Validating a pack

Run it through `mcp__tcl-lsp__spectcl_check` — the spec-author Claude Code
skill does this for you automatically. It parses the pack for real and
reports, per command, which fields your declaration actually set; every
dropped or misspelled word, with the line it was on; every hook you
declared and whether it is cheap to call repeatedly; and any name
collision with a shipped command. Fix every notice — a dropped word is
otherwise silent. A `tcl spec check` command-line equivalent is planned;
the MCP tool is what exists today.

### What works today versus what is landing

- **Today.** Writing and validating a pack works exactly as described
  above. The three-tier discovery and the "nearest wins" merge exist and
  are tested as a library, but nothing in the running language server, and
  no editor setting, reads from your workspace or config directory yet.
  Dropping a pack in `.tcl-lsp/` does not, today, change what your editor
  shows.
- **Landing.** Wiring that discovery into the running server, so a saved
  pack lights your commands up without a restart, is in progress — see
  [issue #1363](https://github.com/bitwisecook/tcl-lsp/issues/1363). Until
  it ships, use [stubs](kcs-howto-annotate-commands-with-stubs.md) for
  live effect in your editor, and keep the pack you validated now: nothing
  about it changes when the loader lands.

### One bad pack cannot take the server down

This is a design requirement, stated plainly rather than assumed. A pack's
declarations — arity, roles, hover text, and the rest — are read as plain
data; nothing executes. The one part of a pack that is code is a hook
body, and a hook body is designed to run in a sandboxed interpreter with
its own time and memory budget, isolated per pack: a crash or a runaway
hook is contained, and only that pack's hook switches off for the
session — never the server. Hook execution ships alongside the runtime
loader above, so today no pack code runs at all during editing; the
guarantee describes what ships together, not a claim about today's code
path.

## How to tell it worked

`spectcl_check` reports your commands with the fields you expect set and
no notices. That is the whole test today. Once runtime loading ships, the
signal moves to the editor: the command stops being flagged unknown.

## Related

- [How to create a command spec without knowing Rust](kcs-howto-create-command-specs-without-rust.md)
- [How to annotate commands with stubs](kcs-howto-annotate-commands-with-stubs.md)
- [SpecTcl pack design](../design/spec-packs.md)
- [The frozen SpecTcl syntax](../design/spec-dsl-examples/README.md)
- [KCS index](README.md)
