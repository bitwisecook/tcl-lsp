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
places a pack can live, and what happens once the server picks it up.

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

### The server loads your pack automatically

Dropping a `.tclspec` file in one of the three tiers above lights its
commands up without a restart. The server discovers and installs your
packs when a workspace opens, and again whenever a `.tclspec` file
changes on disk or `tclLsp.specPacks` moves — your editor's watched-files
mechanism tells it, so saving the file is enough. Discovery, the
nearest-wins merge, and installation into the running command registry
all happen on the live server, not only in the library's own tests.

### One bad pack cannot take the server down

A pack's declarations — arity, roles, hover text, and the rest — are read
as plain data; nothing executes. The one part of a pack that is code is a
hook body — a resolver, a const-folder, a predicate gate — and it runs in
a sandboxed interpreter with its own time and memory budget, isolated per
pack. This is live on every load and reload, not aspirational: a crash or
a runaway hook is contained, and only that pack's hook switches off for
the session — never the server.

## How to tell it worked

`spectcl_check` reports your commands with the fields you expect set and
no notices — check this first, since a dropped word is otherwise silent.
Then look at the editor itself: a command your pack declares stops being
flagged unknown, and hover on it shows the summary and return text you
wrote. If it does not, check the server's log channel for a `SpecTcl:`
load line — it names how many packs and commands were found, and any
notice or shipped-command collision.

## Related

- [How to create a command spec without knowing Rust](kcs-howto-create-command-specs-without-rust.md)
- [How to annotate commands with stubs](kcs-howto-annotate-commands-with-stubs.md)
- [SpecTcl pack design](../design/spec-packs.md)
- [The frozen SpecTcl syntax](../design/spec-dsl-examples/README.md)
- [KCS index](README.md)
