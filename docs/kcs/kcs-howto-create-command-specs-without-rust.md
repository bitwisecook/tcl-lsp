# KCS: How do I create a command spec without knowing Rust?

> **Audience:** User
> **Type:** How-To

## Applies to

all-editors, tcl-lsp CLI

## Question

My package defines commands tcl-lsp does not know. I write Tcl, not Rust.
How do I describe my commands — arguments, script bodies, switches,
highlighting, checks — so the tools treat them like built-ins?

## Before you start

There are two paths, and they are not rivals:

- **Stubs** are per-project annotations you keep next to your own code.
  They cover the essentials (name, argument count, which arguments are
  scripts or variables) in a few minutes. See
  [how to annotate commands with stubs](kcs-howto-annotate-commands-with-stubs.md).
- **A command spec** is a full registry entry — everything `foreach` or
  `string` gets: hover documentation, completion, subcommands, options,
  version gates, and the security checks. You author it in the
  [Command Spec Studio](features/kcs-feature-spec-studio.md), a web page,
  and no Rust knowledge is needed.

This note is about the second path.

## Answer

Open the studio at <https://bitwisecook.github.io/tcl-lsp/spec-studio/>.
It runs entirely in your browser; nothing you type is uploaded.

### 1. Start from something like your command

Pick your dialect at the top left, then load the built-in command that
behaves most like yours — its filled-in form is the best documentation:

- a loop that runs a body: load `foreach`
- an ensemble with subcommands, like `mylib cmd args`: load `dict` or `string`
- a command that writes a variable: load `set` or `lappend`
- a command with `-flag` switches: load `lsort`

Or press **New command** to start empty. If your package's source is at
hand, the **Import a package** tab is faster still: drop your `.tcl` files
on it and every `proc` becomes a draft spec, with the argument count and
roles inferred from how each parameter is actually used — each guess
listed with its evidence.

### 2. Fill in the form, most valuable first

The form is grouped, and every group and field has a **?** button with a
plain-language explanation and Tcl examples. In rough order of value:

1. **Identity** — the command name, exactly as scripts type it.
2. **Arity and arguments** — how many arguments (the rule behind Tcl's
   own `wrong # args` message), and what each position is: a script
   body, an [expr] expression, a variable name being written or read, a
   pattern, a channel. This is what makes a body argument highlight as
   code and makes rename and "unused variable" see through your command.
3. **Availability** — which Tcl versions and dialects have the command,
   and the package a script must `package require` first.
4. **Documentation** — the hover text and synopsis. Pure documentation,
   zero risk, and the part users see most.
5. **Subcommands** and **Options and values** — for `mylib cmd` ensembles
   and `-flag` switches. Declared ones get completion and spelling
   checks; abbreviations are handled for you.
6. **Behaviour** — the traits: does it evaluate code, alter control
   flow, create an `upvar`-style alias? Diagnostics key off these.
7. **Side effects** and **Taint and security** — what state it touches,
   and whether it is a source, sink, or sanitiser of untrusted data.

Everything is optional beyond the name. An unset field means "unknown",
never "wrong" — start small and deepen later.

### 3. Look things up on the Reference tab

The **Reference** tab is the searchable version of all of this: every
spec field, every behavioural trait, every argument role, every
[taint](../GLOSSARY.md#taint-analysis) colour, with what each means and what it
drives. Try searching "taint", "body", or "upvar". The **?** buttons in
the form show the same text in place.

### 4. Send it in

You do not write or read Rust at any point — the **Rendered .rs** tab
generates the registry file as you type. When you are happy:

1. Press **Add to files** on the rendered output.
2. On the **Files & issue** tab, write a line or two about what the
   command does and where its behaviour is documented.
3. Press **Open a GitHub issue ↗**. The issue opens pre-filled in a new
   tab for you to review and post.

A maintainer reviews the spec and merges it into the shipped registry.

### What the form cannot express

A few behaviours need a hook — code that inspects each call, such as an
argument layout that changes shape the way `if`'s `elseif` chain does.
Those fields sit under **Advanced**, and the studio carries them as
opaque expressions. Do not let that stop you: describe the rule in plain
words in the issue notes ("the last argument is always the body;
everything before it comes in name–value pairs") and a maintainer writes
the few lines.

For commands that must stay private to your project, stubs remain the
answer today; richer private registries are being discussed in
[issue #1363](https://github.com/bitwisecook/tcl-lsp/issues/1363).

## How to tell it worked

In the studio, load your draft's dialect and check the **Rendered .rs**
tab shows your command with no `TODO` markers you did not expect. Once
your spec ships in a release, the command stops being flagged as
unknown, hover shows your documentation, and its script-body arguments
highlight like a built-in's.

## Related

- [The Command Spec Studio](features/kcs-feature-spec-studio.md) — the
  full tour of the page.
- [How to annotate commands with stubs](kcs-howto-annotate-commands-with-stubs.md)
  — the lighter per-project path.
- [The command registry design doc](../design/compiler/command-registry.md)
  — the complete field reference, for the curious.
- [KCS index](README.md)
