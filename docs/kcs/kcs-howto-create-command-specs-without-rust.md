# KCS: How do I create a command spec without knowing Rust?

> **Audience:** User
> **Type:** How-To

## Applies to

all-editors, tcl-lsp CLI

## Question

My package defines commands tcl-lsp does not know. I write Tcl, not Rust.
How do I get them treated like built-ins?

## Before you start

For a quick, per-project fix, [stubs](kcs-howto-annotate-commands-with-stubs.md)
declare the essentials in minutes. For the full treatment — hover,
completion, subcommands, options, version gates, security checks — write a
command spec in the [Spec Studio](features/kcs-feature-spec-studio.md).
No Rust needed.

## Answer

Open <https://bitwisecook.github.io/tcl-lsp/spec-studio/>. Everything runs
in your browser; nothing is uploaded.

1. **Start from something like your command.** Pick your dialect, then
   load the built-in that behaves most like yours — `foreach` for a loop,
   `dict` for subcommands, `set` for a variable writer, `lsort` for
   `-flag` switches. Or drop your package's `.tcl` files on **Import a
   package** and every `proc` becomes a draft spec, inferred from how the
   body uses each parameter.
2. **Fill in what you know.** Name and arity first, then argument roles
   (which words are scripts, variables, patterns — this is what makes
   highlighting and rename work through your command), then availability,
   hover text, subcommands, and options. Every field and group has a
   **?** button explaining it in Tcl terms. Everything beyond the name is
   optional: unset means "unknown", never "wrong".
3. **Look things up on the Reference tab** — every field, trait, argument
   role, and taint colour, behind one search box.
4. **Send it in.** Press **Add to files** on the **Rendered .rs** tab,
   then **Open a GitHub issue ↗** on **Files & issue**. The generated
   Rust goes with it — you never write or read it.

A few behaviours need code (under **Advanced**, shown as opaque
expressions). Describe the rule in plain words in the issue notes and a
maintainer writes the few lines.

For commands that must stay private to your project, use stubs; richer
private registries are being discussed in
[issue #1363](https://github.com/bitwisecook/tcl-lsp/issues/1363).

## How to tell it worked

The **Rendered .rs** tab shows your command with no unexpected `TODO`
markers. Once the spec ships, the command stops being flagged as unknown,
hover shows your documentation, and its body arguments highlight like a
built-in's.

## Related

- [The Command Spec Studio](features/kcs-feature-spec-studio.md)
- [How to annotate commands with stubs](kcs-howto-annotate-commands-with-stubs.md)
- [The command registry design doc](../design/compiler/command-registry.md)
- [KCS index](README.md)
