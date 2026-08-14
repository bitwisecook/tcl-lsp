# KCS: feature — Command Registry Spec Studio

> **Audience:** User
> **Type:** Functionality

## Summary

A browser page for exploring the command registry: browse every command tcl-lsp knows, edit any field of its command specification, and render the result back out as a drop-in registry `.rs` file or a Tcl dialect stub.

## Applies to

tcl-lsp CLI

## Availability

| Context | How |
|---------|-----|
| Hosted | <https://bitwisecook.github.io/tcl-lsp/spec-studio/> |
| Local | `make spec-studio-wasm`, then open `rust/tcl-spec-studio-wasm/dist/index.html` |

## How to use

The studio is one self-contained HTML file. Open it and everything works —
including offline, and including from a file you saved to disk.

1. **Pick a dialect** at the top left. The command list underneath is that
   dialect's real registry, so Tcl 8.4 shows what Tcl 8.4 has.
2. **Choose a command** to load its live specification into the form, or press
   **New command** to start from scratch. If you already know the name, type it
   into the box and press **Load** (or Enter) — the box also offers the
   matching names as you type. An ambiguous or unknown name is reported rather
   than guessed at, since loading the wrong command silently is worse than
   saying nothing matched.
3. **Edit any field.** The form is grouped — Identity, Availability, Arity and
   arguments, Types, and so on. A field that differs from the default is
   marked **set**, and each group heading counts how many of its fields are
   set, so what a command actually declares is visible at a glance. Every
   group and every field carries a **?** button that opens a plain-language
   explanation, written for Tcl developers, with Tcl examples rather than
   Rust ones.
4. **Read the output** on the **Rendered .rs** and **Tcl stub** tabs. Both
   update as you type.
5. **Copy** it, **Download** it, or **Add to files** to collect several
   artefacts together.
6. **Files & issue** downloads the collected files and opens a pre-filled
   GitHub issue so you can propose the spec.

### On a phone

The studio is usable on a phone, not merely reachable from one. Below 34rem
every toolbar control takes the full width, the tab strip scrolls sideways
instead of stacking, and touch targets meet the 44px minimum. The command list
sits below the editor rather than beside it — which is why typing a name and
pressing **Load** matters there: the list is off-screen, so filtering alone
would look like nothing had happened.

![The spec studio on a phone, with a command loaded by name](../../screenshots/spec-studio-mobile.png)

### Looking things up: the Reference tab

The **Reference** tab holds the registry's whole vocabulary behind one
search box: every specification field, every behavioural trait, every
argument role, every taint colour, and the rest of the picker catalogues,
each with what it means and what it drives. Searching "taint" finds the
taint fields and every taint colour; searching "upvar" finds the traits
and fields about scope aliasing. The same text sits behind the form's
**?** buttons, so nothing has to be learned in two places.

### Nothing you type is uploaded

The command registry, the Tcl compiler's analyser, and both renderers are
compiled to WebAssembly and embedded in the page. It carries a content
security policy of `connect-src 'none'`, so it *cannot* make a network
request — an unreleased command or a proprietary package you import stays on
your machine. Opening a GitHub issue is the one action that leaves the page,
and it is a link you click: the issue form opens pre-filled in a new tab so
you can read it before posting.

### Importing a package

The **Import a package** tab takes a package's own `.tcl` files. Each is
compiled with the real analyser — the same one the language server runs — and
every `proc` it finds becomes a draft specification:

- **Arity** comes from the parameter list. Parameters without a default are
  required, ones with a default are optional, and a trailing `args` makes the
  command variadic.
- **Argument roles** come from how each parameter is *used* in the body:
  evaluated as a script, `upvar`'d and written, iterated as a list, invoked as
  a command.
- **Traits** come from the same evidence — a parameter evaluated as a script
  makes the command a dynamic barrier, an `upvar`'d write makes it a
  scope-alias creator.
- **Hover text** comes from the `proc`'s doc comment, and **package gating**
  from `package provide`.

Every guess is listed with the evidence behind it, so you can accept it or
overrule it. They are a starting point, not an assertion.

### The Pack DSL tab

Beside the form, `.rs` module, and stub, the **Pack DSL** tab holds the
[SpecTcl pack](../../design/spec-packs.md)'s `.tclspec` source directly —
the studio's one authoritative document for a pack you are building.
Edit a field in the form and the DSL text updates; edit the text and the
form, the command list, and the collision report all follow. Open an
existing `.tclspec` with **Open a .tclspec…**, or start one from scratch
and **Download** or **Add to files** when you are done. **Re-render
canonically** rebuilds the whole document from its commands — useful
after a lot of form editing, at the cost of your own comments and layout.

The tab paints real syntax highlighting over the source, shows hover text
for a property word or catalogue spelling (the same schema and help text
the form's **?** buttons use), and marks the lines a load notice applies
to in a gutter strip — entirely client-side, so nothing you type leaves
the page. This widget is deliberately the minimum viable editor, not a
full code-editing surface: it is slated for replacement by an embedded
Monaco editor with the language server itself, compiled to WebAssembly,
attached as its language server, so pack authoring in the browser gets
the identical SpecTcl experience an editor gets.

### What a stub cannot carry

The stub language is deliberately narrower than a command specification: it
has no subcommands, options, types, or hooks. Anything a stub cannot express
is written as a comment beside it rather than dropped silently, so what the
stub does *not* say stays visible.

### Fields the studio cannot read back

A few specification fields hold a function pointer or a reference to a static
descriptor. Rust can tell such a field is set but not recover the *expression*
that set it. Those are listed in a warning above the form and emitted as a
`TODO` in the rendered file, so behaviour the original command had is never
dropped without saying so. Fill the expression in under **Advanced** to emit
it.

## Example

Loading `lsearch` from the Tcl 9.0 registry shows its declared dialects,
19 options, and the rest of its specification in the generated form:

![The spec studio with lsearch loaded](../../screenshots/spec-studio-editor.png)

The **Rendered .rs** tab produces a complete registry module, copyright banner
included:

![The rendered .rs output](../../screenshots/spec-studio-rendered-rs.png)

Importing a two-`proc` package infers a signature for each, with the evidence
behind every guess:

![Importing a package and inferring signatures](../../screenshots/spec-studio-import.png)

For a small package like:

```tcl
package provide mypkg 2.1
# Evaluate a script with a caller variable bound to a fresh value.
proc mypkg::with_var {varName script {mode fast}} {
    upvar 1 $varName v
    set v 1
    uplevel 1 $script
}
```

the studio infers arity `2..3`, an argument role of `VarWrite` on `varName`
and `Body` on `script`, the `EVALUATES_CODE` and `CREATES_SCOPE_ALIAS` traits,
and a `package require mypkg` gate — reporting, for each, the line of
reasoning that produced it.

## See also

- [How to create a command spec without knowing Rust](../kcs-howto-create-command-specs-without-rust.md)
  — the step-by-step path from "my command is unknown" to a proposed spec.
- [The command registry contract](../../design/compiler/command-registry.md) —
  the full field reference.
- [The spec studio design doc](../../design/contracts/command-spec-studio.md) —
  how the schema, draft model, and renderers fit together.
- [Dialect command stubs](../../design/contracts/dialect-stubs.md) — the stub
  language the studio emits.
- [SpecTcl pack design](../../design/spec-packs.md) — the `.tclspec`
  authoring format the studio's Pack DSL tab reads and writes.
- [How to write a SpecTcl pack](../kcs-howto-write-a-tclspec-pack.md) —
  write one by hand today.
