# KCS: How do I create a command spec without knowing Rust?

> **Audience:** User
> **Type:** How-To

## Applies to

all-editors, tcl-lsp CLI, MCP, Claude skill

## Question

My package defines commands tcl-lsp does not know. I write Tcl, not Rust.
How do I get them treated like built-ins?

## Before you start

For a five-minute fix, [stubs](kcs-howto-annotate-commands-with-stubs.md) are
the quick fallback: no subcommands, no arity checking, but your commands
stop being flagged unknown. For the full treatment — hover, completion,
subcommands, options, version gates, security checks — write a **SpecTcl**
pack: a `.tclspec` file that describes your commands the same way a
built-in's own spec does.

## Answer

Two ways to write a pack today:

1. **By hand.** A `.tclspec` file is its own dialect, so it gets full
   editor support out of the box — highlighting, completion, and
   diagnostics for a misspelled trait or role — the same machinery it
   configures. See [how to write a SpecTcl
   pack](kcs-howto-write-a-tclspec-pack.md) for the minimal shape, or copy
   one of the worked examples in
   [`spec-dsl-examples/`](../design/spec-dsl-examples/README.md).
2. **With the spec-author Claude Code skill.** Point it at your library's
   source; it scans every `proc` with the real compiler, infers arity and
   argument roles from how each parameter is used, and writes the pack for
   you, citing the evidence behind every field.

Validate what you write with `mcp__tcl-lsp__spectcl_check` (the skill runs
this for you). It loads the pack through the real parser and reports, per
command, which fields your declaration set, every dropped or misspelled
word, and any name clash with a shipped command.

The [Spec Studio](features/kcs-feature-spec-studio.md) renders a drop-in
`.rs` module or a stub from its form, which is what a **shipped**
contribution needs; its Pack DSL tab also reads and writes a `.tclspec`
pack directly, with syntax highlighting, hover, and loader-notice markers
on the source text.

**Contributing** a pack is the same file you wrote for yourself: attach it
to a GitHub issue instead of dropping it in your config directory.

Loading a pack automatically — so it lights your commands up in the editor
the moment you save it — already works: see [how to write a SpecTcl
pack](kcs-howto-write-a-tclspec-pack.md) for how discovery and reload
work.

## How to tell it worked

`mcp__tcl-lsp__spectcl_check` reports each command with the fields you
expect set and no unexpected notices. In the editor, the command stops
being flagged unknown and hover shows your documentation.

## Related

- [How to write a SpecTcl pack](kcs-howto-write-a-tclspec-pack.md)
- [The Command Spec Studio](features/kcs-feature-spec-studio.md)
- [How to annotate commands with stubs](kcs-howto-annotate-commands-with-stubs.md)
- [The command registry design doc](../design/compiler/command-registry.md)
- [SpecTcl pack design](../design/spec-packs.md)
- [KCS index](README.md)
