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

A `.tclspec` file opens with `speclib`, names your library and the DSL
vocabulary it is written against, and holds one `command` block per
command:

```tcl
speclib mylib 2.0 {
    command mylib::with_var {
        available {tcl 8.6-}
        arity 2..3
        arg 0 -role VarWrite
        arg 1 -role Body
        hover {
            summary {Run a script with a caller variable bound.}
            returns {The script's result.}
        }
    }
}
```

The version word is the *vocabulary's*, not your library's: `2.0` is the
current one, and it is what `tcl spec upgrade` rewrites an older pack to
(`--check` only reports; `--restyle` re-emits the result in canonical form,
dropping comments and your own layout — a pack that runs code rather than
listing declarations is refused rather than rewritten). Older packs keep
loading unchanged — the words a 1.x pack spells still mean what they
meant.

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
shipped one, unless you write `command NAME -override { … }`. An override
replaces the shipped command's arguments, options and hover text. It cannot
remove the shipped command's taint facts, or change how the compiler and the
analyser handle the command: those stay as shipped, and the override does
not warn you that it kept them.

A pack a package ships, beside that package's own `tclpkg.tcl`, also loses
its `alias_of` and `runtime_backing` rows unless the package is your own or
one your manifest requires directly, and a Tcl body (`tcl-body`) unless the
package is your own
([why](kcs-qa-why-was-a-declaration-dropped-from-my-dependencys-pack.md)).

### Saying a Tcl proc defines the command

`runtime_backing tcl-body {-pack-text {proc NAME args body}}` says the command
is that `proc`, and the compiler inlines the body into any procedure that
calls it, while the runtime still holds the call to exactly that text: a
library that differs from the pack runs as it always did, only slower. The
text must be one `proc` that defines the command and nothing else. The compiler
splices a body only where the call it replaces would have answered the same: a
body that reads a variable it never sets, hands a command a variable's name
(`[set y]`) or runs a script in a substitution is called as it always was, and so
is one defined in a namespace of its own, in a caller outside that namespace,
when it names a command without a leading `::` or substitutes one.
`tcl-body {-package-source PATH}` names a file of your package instead,
relative to the directory of its `tclpkg.tcl`; it is read when the pack
loads, so a library edit reaches the compiler at the next load without any
change to the pack, and a path that cannot be read draws a warning on the
command's line. A call at the top level of a script is not inlined.

When the body is plain value-position Tcl — commands a hook body may call (`set`,
`expr`, `if`, `string`, `dict` and the like), no `upvar`, `uplevel`, `global`,
`variable`, channel or `exec`, no namespace-qualified variable, `return` only as
the last statement, and parameters with no defaults — you can also have the
analyser evaluate a call whose arguments it knows by running the body. Say so
with `-evaluate` beside the source: `tcl-body {-pack-text {proc NAME args body}
-evaluate}` or `tcl-body {-package-source PATH -evaluate}`. Give the command an
`arity` that is exactly its parameters and leave `semantics` and `evaluate` off
it. Nothing is run unless you say so, because the engine that runs the body
emulates an older release imperfectly (`string is integer`'s width before 9.0,
`tcl_precision`, `format %c`, the index and bound forms of `lindex` and
`lreplace`, floating-point division by zero), and only you know whether your
body meets one of those. `-evaluate` is your word that the body answers what a
real shell does under every release your package is used with, so compare what
`tcl opt --dialect tclX.Y` folds a call to with what that release's own `tclsh`
prints before you add it. A body the analyser cannot run draws a warning on the
command's line that names the reason (a command off the list, a text that is not
one `proc`), and so does `-evaluate` beside a command that states its own
`semantics` or `evaluate`; the call is compiled as it was.

### Validating a pack

Run it through `mcp__tcl-lsp__spectcl_check` — the spec-author Claude Code
skill does this for you automatically. It parses the pack for real and
reports, per command, which fields your declaration actually set; every
dropped or misspelled word, with the line it was on; every hook you
declared and whether it is cheap to call repeatedly; any name collision
with a shipped command; and any codegen row the pack's tier may not keep
([why a codegen hook is refused](kcs-qa-why-was-my-pack-codegen-hook-refused.md)).
Fix every notice — a dropped word is otherwise silent. The MCP tool is the only validator; `tcl spec` itself has
`import`, `upgrade`, `export`, and `test`, which runs the package in a real shell
and reports each declared fact — arity, examples, return type, purity, a Tcl
reference body — that the package does not bear out.

### The server loads your pack automatically

Dropping a `.tclspec` file in one of the three tiers above lights its
commands up without a restart. The server discovers and installs your
packs when a workspace opens, and again whenever a `.tclspec` file
changes on disk or `tclLsp.specPacks` moves — your editor's watched-files
mechanism tells it, so saving the file is enough. Discovery, the
nearest-wins merge, and installation into the running command registry
all happen on the live server, not only in the library's own tests.

### One bad pack cannot take the server down

A pack is a Tcl program, and it is run as one — in a sandboxed
interpreter with its own time and memory budget, isolated per pack, with
no file, network, or process access. A pack that only declares (no
`foreach` over a table, no `proc` building rows) skips the interpreter
altogether and is read straight off its parse tree; the two paths are
gated against each other, so the shortcut is an optimisation, never a
second reading of your file.

Hook bodies — a resolver, a const-folder, a predicate gate — run later,
at query time, in the same sandbox. Containment is live on every load and
reload, not aspirational: a crash, a budget blowout, or a runaway loop is
converted to abstention, and only that pack's hook switches off for the
session — never the server.

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
- [How to declare an evaluator for a pack command](spectcl/kcs-howto-declare-an-evaluator-for-a-pack-command.md)
- [SpecTcl pack design](../design/registry/spec-packs.md)
- [The frozen SpecTcl syntax](../design/spec-dsl-examples/README.md)
- [KCS index](README.md)
