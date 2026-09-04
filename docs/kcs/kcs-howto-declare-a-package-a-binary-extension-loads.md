# KCS: How do I tell tcl-lsp that a binary extension also loads Tk?

> **Audience:** User
> **Type:** How-To

## Applies to

VS Code, all-editors

## Question

My script says `package require myExtension` and then calls `ttk::frame`
and `pack`. `myExtension` is a compiled extension — a `.dll` / `.so` —
whose C `Init` brings Tk up for me, so I never write `package require Tk`.
The editor still treats the file as plain Tcl: no Tk completions, no Tk
checks, and a warning that `ttk::frame` "requires `package require Tk`".
How do I tell it Tk is there?

## Before you start

- You know which package your extension loads behind the scenes. Tk is the
  usual one, but the same steps work for any package name.
- You can either edit the file, or add a setting to your workspace or a
  `.tcl-lsp.ini` file beside your project.

## Answer

Declare the dependency. tcl-lsp reads a `pkgIndex.tcl` and follows the
`package require`s inside the Tcl files it loads, so a *Tcl* wrapper
package is found on its own. A compiled extension loads Tk from C, where
there is no Tcl text to read, so the link has to be stated.

There are two ways to state it. Use the comment when the fact belongs to
the file you are looking at, and the config file when it belongs to the
project.

### A comment in the file

Add a `# tcl-lsp: package … provides …` line naming the extension and what
it brings up:

```tcl
# tcl-lsp: package myExtension provides Tk
package require myExtension

ttk::frame .f
pack .f
```

Name several packages on one line — `# tcl-lsp: package myExtension
provides Tk Img` — and put the comment wherever you like; because it names
the extension, it does not have to sit next to the `package require`. It
only takes effect in a file that actually requires that extension. This
form travels with the file, needs no configuration at all, and is the only
one the `tcl` CLI reads.

### `.tcl-lsp.ini` (shared with the project)

Add a `[packages.provides]` section, one line per package:

```ini
[packages.provides]
myExtension = Tk
```

List several with commas — `myExtension = Tk, Img` — and add a line per
extension. State it once here and every file in the project is covered.
The declarations chain: if `myExtension` provides `myWidgets` and
`myWidgets` provides `Tk`, requiring `myExtension` gets you Tk.

### VS Code settings (just for you)

Set `tclLsp.packages.provides` in your workspace or user settings:

```json
"tclLsp.packages.provides": {
    "myExtension": ["Tk"]
}
```

## How to tell it worked

Open a file that requires your extension and type `tt` at the start of a
line: `ttk::frame`, `ttk::button` and the rest of the Tk widgets are now
offered, and hovering one shows its documentation. The "requires
`package require Tk`" warning is gone, and the Tk-specific checks are
running — a widget path whose parent was never created is reported, and so
is a `pack` / `grid` conflict on the same container.

Nothing else changes: the declaration says the package is *there*, never
which release it is, so no version-gated check starts firing against a
version you did not claim.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [W120 — missing package require](codes/kcs-diagnostic-w120-missing-package-require.md)
- [What sections and keys are valid in tcl-lsp config files?](kcs-qa-what-config-sections-are-valid.md)
- [How do I say a package is ambient under one of my pack's dialects and not another?](kcs-howto-scope-an-ambient-package-to-one-dialect.md)
