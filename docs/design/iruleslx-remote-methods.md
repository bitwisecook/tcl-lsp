# iRulesLX remote methods — the Tcl ↔ JavaScript symbol model

An iRulesLX plugin has two halves in two languages. The iRule opens a handle
onto a running Node.js extension and calls a method on it by name; the
extension registers that name on an `ILXServer`. Nothing in either file names
the other, so go-to-definition, hover and find-references could not cross the
boundary (issue #1707).

```tcl
when HTTP_REQUEST {
    set handle [ILX::init my_plugin my_extension]
    set reply [ILX::call $handle my_js_function [HTTP::uri]]
    ILX::notify $handle my_js_function logged
}
```

```javascript
var f5 = require('f5-nodejs');
var ilx = new f5.ILXServer();
ilx.addMethod('my_js_function', function (req, res) { res.reply('ok'); });
ilx.listen();
```

A method name is meaningful **only within one extension**. Nothing in this
model is keyed by method name alone: two extensions may both register
`process`, and they are two different symbols.

## Where each fact lives

| Fact | Owner |
|---|---|
| Which word is the handle / the method, and whether the call awaits a reply | `tcl_registry::remote_method`, hung off `CommandSpec::remote_method` |
| Which iRule words are ILX sites, and what handle they resolve to | `tcl_irules::ilx` |
| Which JavaScript registrations an extension source declares | `tcl_irules::ilx` |
| Which extension source an `ILX::init PLUGIN EXTENSION` refers to | `tcl_lsp_core::ilx_navigation` |
| Wire-level definition / hover / references | `tcl_lsp_server` (three thin tiers) |

The providers never name a command: they ask the registry which word carries
the method. That is also the **dialect gate** — `ILX::init`, `ILX::call` and
`ILX::notify` are `SpecSurface::IRULES` specs, so a registry built for stock
Tcl holds no command of the name, finds no descriptor, and the whole relation
is inert. A plain Tcl file with its own `proc ILX::call` is untouched.

## Evidence

All behaviour below is taken from F5's documentation, fetched 2026-08-30:

- <https://clouddocs.f5.com/api/irules/ILX__init.html> — "ILX::init [plugin
  name] [extension name]", "Creates a handle for future use by ILX::call and
  ILX::notify".
- <https://clouddocs.f5.com/api/irules/ILX__call.html> — "ILX::call \<ILX
  handle\> [-timeout n] \<method\> [optional arg]+"; it blocks until a reply
  arrives; the default timeout is 3000 ms.
- <https://clouddocs.f5.com/api/irules/ILX__notify.html> — `ILX::notify HANDLE
  METHOD (ARGS)*`; delivery is "best effort and is not guaranteed".
- <https://clouddocs.f5.com/api/irules-lx/ILXServer.html> — `addMethod(name,
  callback)` "Add a method handler"; also `removeMethod(name)`,
  `setDefaultMethod(callback)`, `listen()`.
- <https://clouddocs.f5.com/cli/tmsh-reference/v16/modules/ilx/ilx_workspace.html>
  — the workspace layout `/var/ilx/workspaces/<partition>/<workspace>/`
  with `extensions/` and `rules/`, and the entry-point rule: "node will look
  in package.json for a main field that identifies the main entry point of the
  plugin. If the main field is not present node will look for the file
  index.js."
- <https://clouddocs.f5.com/cli/tmsh-reference/v16/modules/ilx/ilx_plugin.html>
  — a plugin is created *from a workspace*
  (`create ilx plugin P from-workspace W`).

## Workspace mapping

The source layout establishes an **extension** name: it is the directory under
`extensions/`. It does not establish a **plugin** name — a plugin is created
from a workspace and the two names need not match. The rule applied, and the
only one:

> the `PLUGIN` word of `ILX::init` must equal the name of the workspace
> directory that holds `extensions/EXTENSION`.

Candidate workspace directories are looked for along the **document's own
ancestors** only — the enclosing workspace of a rule in `…/W/rules/x.tcl`, and
a `…/<ancestor>/PLUGIN/extensions/…` sibling. The workspace is never scanned
wholesale. If no ancestor matches, or two distinct directories do, nothing
resolves.

A plugin deliberately named differently from its workspace is therefore **not
navigable**. An explicit configured mapping (the "documented workspace/config
mapping" half of issue #1707 criterion 2) is not implemented; approximating it
by matching on the extension name alone would resolve to the wrong file in a
workspace that has two plugins, which is exactly the guess criterion 4
forbids.

## Supported JavaScript

Recognised:

- an `ILXServer` construction assigned to a variable — `var ilx = new
  f5.ILXServer();`, `const ilx = new ILXServer();`, `let ilx = new
  require('f5-nodejs').ILXServer();` — i.e. any `new` expression whose
  constructor path ends in `ILXServer`;
- `ilx.addMethod('name', handler)` / `ilx.addMethod("name", handler)` on such
  a receiver, with a literal, escape-free, single- or double-quoted name and a
  second argument present.

The scanner is a comment- and string-aware token scan, not a JavaScript
parser: a `//` inside a string cannot swallow a line, a `/["']/` regular
expression cannot open a string, and a registration inside a comment or a
string literal is not a registration. It deliberately understands nothing
else — this is not a JavaScript language server (an explicit exclusion of the
issue).

**Classified as abstentions until modelled** (each yields no target, never a
wrong one):

| Form | Why |
|---|---|
| `addMethod(name, …)`, `` addMethod(`t`, …) ``, `addMethod('a' + 'b', …)` | the name is not a literal |
| a method map passed to a constructor | not a documented registration shape |
| `removeMethod(name)` | retracts a registration; modelling it needs an order-sensitive method table |
| `setDefaultMethod(cb)` | registers no name, so default-method dispatch has no target |
| `addMethod` on a receiver this file does not bind to an `ILXServer` | the receiver may be any object |
| a name containing a backslash escape | a half-decoded name would match the wrong Tcl word |

## Tcl-side resolution and its abstentions

| Written | Result |
|---|---|
| `set h [ILX::init p e]` … `ILX::call $h m` | resolved |
| `ILX::call [ILX::init p e] m` | resolved |
| `ILX::call $h -timeout 500 -- m` | resolved; options come from the spec's own table |
| `ILX::init $p e`, `ILX::init p $e` | handle unknown → the call abstains |
| `set h [ILX::init p e]; set h $other` | binding widens → the call abstains |
| `ILX::init e` (one word) | undocumented form → abstains |
| a handle bound in a *sibling* `when` body | not in scope → abstains |
| `ILX::call $h $method`, `ILX::call $h m$suffix` | no literal method → not a site at all |
| the extension registers the name twice | reported as an ambiguity; no target |
| no JavaScript in the workspace | abstains |

A call whose method word is literal but whose handle is unknown is still a
*site*: hover can honestly say "method `m`, extension unknown", while
navigation offers nothing.

**No diagnostic is emitted from any of this.** An unresolved method is not an
error: the extension's method table is only known when its JavaScript is in
the workspace, and a plugin's sources are routinely absent from the repository
holding its iRules.

## Find-references

From either end, the set is the registration(s) plus every literal
`ILX::call` / `ILX::notify` of the same method **on the same extension**, in
the open document and in the immediate children of the associated workspace's
`rules/` directory (the documented layout puts every rule there). A rule
outside that directory, or in a different workspace, is not searched.

The JavaScript end is gated on the document being the extension's resolved
entry point, so an ordinary `.js` file elsewhere in a project is never
scanned. It is reachable for a *closed* file (the server reads closed files
through its source store); the VS Code extension does not associate `.js` with
the Tcl server, so an editor-side "find references" started from inside
`index.js` is not wired up yet.

## Anchors

- `rust/tcl-registry/src/remote_method.rs` — the descriptors.
- `rust/tcl-registry/src/commands/irules/ilx__{init,call,notify}.rs` — the
  three specs that carry them.
- `rust/tcl-irules/src/ilx.rs` — the Tcl walk and the JavaScript scanner.
- `rust/tcl-lsp-core/src/ilx_navigation.rs` — workspace association,
  definition / hover / references.
- `rust/tcl-lsp-server/tests/e2e/issue1707_ilx_methods.rs` — the end-to-end
  suite, over a real on-disk ILX workspace.
