# iRulesLX remote methods — the Tcl ↔ JavaScript symbol model

An iRulesLX plugin has two halves in two languages. The iRule opens a handle
onto a running Node.js extension and calls a method on it by name; the
extension registers that name on an `ILXServer`. Nothing in either file names
the other. The model retains readonly source cards and separate workspace
associations. Readonly navigation can suggest a JavaScript registration from
authenticated original constructor/setter/read source cards. It does not
establish an evaluated handle or a successfully dispatched RPC.

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
| Which original iRule words supply readonly ILX method/constructor candidates | `tcl_irules::ilx`, through the shared guarded source context |
| Whether a variable handle read has an original constructor/setter association | `tcl_compiler::registry_invocation::source_remote_first_operand_associations` |
| Which JavaScript registrations an extension source declares | `tcl_irules::ilx` |
| Which extension source an `ILX::init PLUGIN EXTENSION` refers to | `tcl_lsp_core::ilx_navigation` |
| Wire-level definition / hover / references | `tcl_lsp_server` (three thin tiers) |

The providers never name a command: they ask the registry which word carries
the method. That is also the **dialect gate** — `ILX::init`, `ILX::call` and
`ILX::notify` are `SpecSurface::IRULES` specs, so a registry built for stock
Tcl holds no command of the name, finds no descriptor, and the whole relation
is inert. A plain Tcl file with its own `proc ILX::call` is untouched.

## Evidence

The command and workspace schemas come from F5's documentation. These sources do not establish an executed call or current handle:

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

A plugin deliberately named differently from its workspace is not navigable by
that rule alone, and approximating it by matching on the extension name would
resolve to the wrong file in a workspace that has two plugins — exactly the
guess criterion 4 forbids. So the user says it instead.

### The declared mapping

```ini
# .tcl-lsp.ini, in the workspace folder
[iruleslx.plugins]
prod_plugin = workspaces/ws_alpha

[iruleslx.rules]
prod_plugin =
    irules/http
    irules/tcp
```

or, identically, the `tclLsp.iruleslx` editor setting:

```json
{ "tclLsp.iruleslx": {
    "plugins": { "prod_plugin": "workspaces/ws_alpha" },
    "rules":   { "prod_plugin": ["irules/http", "irules/tcp"] } } }
```

Every key is a plugin name — the `PLUGIN` word of `ILX::init`. Paths are
relative to the workspace folder the configuration belongs to (absolute paths
are taken as given) and are lexically normalised, so a declaration written
`../shared/ws` compares equal to the same directory reached from a document's
own ancestors. That equality is what lets the *JavaScript* end recognise a
declared workspace as well.

A declaration is **authoritative**: once `prod_plugin` is declared, the
directory-name convention is not consulted for it at all. Treating the
declaration as one more candidate would turn a workspace that happens to
share the plugin's name into an `ExtensionAmbiguous` — an answer strictly
worse than the one the user asked for.

`[iruleslx.rules]` is the caller-side admission of the same gap: the deployed
layout keeps every rule in `rules/`, but a repository routinely keeps its
iRules elsewhere and builds the workspace at release time. Those directories
*are* walked (the workspace's own `rules/` is not — see below), bounded to 8
levels and 512 directories per request, because the user named them
deliberately. A `rules` entry for a plugin with no `plugins` entry is dropped:
extra caller directories are only meaningful once the plugin's workspace is
known, and keeping a half-declaration would widen the search for a plugin the
user never associated.

The resolved associations are reported by `tcl-lsp.getEffectiveConfig` as
`iruleslx_plugins`, so a client can see where a declaration actually pointed.

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
| `setDefaultMethod(cb)` | registers no name, so default-method dispatch has no target |
| `addMethod` on a receiver this file does not bind to an `ILXServer` | the receiver may be any object |
| a name containing a backslash escape | a half-decoded name would match the wrong Tcl word |

### `removeMethod` subtracts

`removeMethod` is **modelled**, not ignored: after
`ilx.addMethod('m', cb); ilx.removeMethod('m');` the running extension has no
`m`, so offering the earlier registration would be a *wrong* answer rather
than a missing one.

The subtraction is deliberately order-free, because source order is not
execution order — a `removeMethod` can sit in a branch, a callback, or a later
module:

- a literal `removeMethod('m')` anywhere in the entry point suppresses `m`,
  wherever it is written relative to the registration;
- a `removeMethod` whose name is **not** literal (`ilx.removeMethod(name)`)
  suppresses the whole table, because it could take out any of it;
- a `removeMethod` on a receiver this file does not bind to an `ILXServer` is
  not this API and changes nothing.

## Tcl source cards and independent handle identity

The Tcl side consumes complete original source vectors, actual structural
ContextRegistry and selected Registry arity/option grammar. It retains genuine
written method extents and full source/configuration/Registry currency. Stored
body syntax supplies source cards without proving event entry or a call.

| Written source | Current result |
| --- | --- |
| `ILX::call $h m` | Literal method card; evaluated handle/cell unavailable |
| `ILX::call [ILX::init p e] m` | Method card plus separate possible constructor labels; resolved handle remains unavailable |
| `ILX::call $h -timeout 500 -- m` | Method ordinal selected by the actual shared option grammar |
| `set h [ILX::init p e]; ILX::call $h m` under the document analysis | Separate source candidate when the compiler authenticates the exact constructor, setter and first handle operand |
| A later read after an opaque operation, reassignment, observer change or foreign frame | No reused constructor association |
| Computed method/control words, unknown expansion or missing written geometry | No method card |
| Stock Tcl or stale complete source/context/Registry | No hosted ILX source selection |

Original constructor labels remain `source_target`, separate from `target`.
The successful constructor result, current handle cell/read/observers,
handler applicability, conditional body activation and constructor effects
remain independently unresolved. The retained compiler association must match
the same analysis owner at query time; identical text analysed afresh cannot
reuse it. Standalone source capture does not issue a variable-handle association.

The compiler withdraws the association at opaque earlier operations. In the
example above, the first `ILX::call` can suggest the registration, while the
later `ILX::notify` cannot reuse the earlier handle association. Supported
literal true condition bodies share the enclosing source frame; a sibling
event or procedure does not inherit its local association. Inline constructors
have their own original source labels without borrowing an earlier handle read.

Equal method labels alone do not identify one symbol. Public query data is
reselected against complete current source before navigation or reference
projection; changed labels, forged targets, stale configuration and a foreign
analysis owner cannot replace that source owner. Hover labels a source candidate
and keeps the evaluated handle unresolved. See the
[ILX source contract](../analysis/name-resolution-proofs/original-ilx-method-source-candidates.md).

## Find-references

Core keeps identity references separate from readonly `source_candidates` and
`source_candidates_from_registration`. The server projects the independently
current source candidates into its readonly LSP location list, alongside any
identity references, removing duplicates. This projection never fills a
runtime target or authorises rename or another edit. Each sibling rule is
analysed under its own input; the initiating rule's facts cannot describe it.

Workspace discovery covers the open document, immediate children of its
`rules/` directory and explicitly configured caller directories.

`rules/` is deliberately not walked recursively and the workspace is never
scanned wholesale: a deeper walk of a directory the user did not point at
would turn one find-references into a tree scan. A declared directory is
different — it exists precisely because the callers are not in `rules/` — so
it is walked, under the bounds above. A rule in neither is not found, which is a
configuration the user can make rather than a limit of the model.

Every other file — a sibling rule, the extension's entry point, its
`package.json` — is read the way the server's own cross-document providers read
one: the editor's buffer when the document is open, the file on disk otherwise.
An unsaved `addMethod`, or one deleted in the editor, is therefore what
navigation sees.

The JavaScript end is gated on the document being the extension's resolved
entry point, so an ordinary `.js` file elsewhere in a project is never
scanned.

It is reachable two ways. The server reads closed files through its source
store, so a `textDocument/references` naming a closed `index.js` is answered
directly. For an **open** JavaScript buffer the VS Code extension registers a
*second* `vscode.ReferenceProvider` for `javascript`, which calls the
`tcl-lsp.ilxReferences` command with the buffer's own text and contributes the
Tcl call sites alongside whatever the JavaScript language service found.

A command rather than a document-selector entry, deliberately: adding
`javascript` to the language client's selector would hand every JavaScript
file in the project to the Tcl server — analyser, workspace index and
diagnostics pipeline included — which has nothing true to say about a language
it understands two API calls of. As a second provider the `.js` document never
enters the Tcl document model at all, and neither provider displaces the
other. The extension-entry gate is applied on the server as well as in the
client's cheap `extensions/<name>/` pre-filter, so a request naming an
ordinary `.js` file answers nothing rather than scanning it.

Registering the provider is not itself an activation event: the extension
activates on a Tcl language id or on a workspace containing Tcl sources, which
an ILX workspace always does. A JavaScript-only project never activates it.

### Desktop only, and why

The whole relation — definition, hover and both directions of
find-references — is **filesystem-path** shaped: `extensions/<name>/` is
matched on path components, the entry point is resolved through
`package.json`'s `main`, and the caller search lists directories. Every server
tier therefore starts by turning the request's URI into a path, and a URI that
is not `file:` has none.

So on the web hosts (vscode.dev / github.dev, where documents are `vscode-vfs:`
and friends) every ILX tier abstains. The browser entry point deliberately does
**not** register the JavaScript reference provider: it would send a request the server cannot answer. Making this work on
the web means giving the relation a URI-shaped discovery path rather than a
path-shaped one — extension lookup, entry-point resolution and directory
listing all over the virtual filesystem — which is its own change, not a
registration.

## Anchors

- `rust/tcl-registry/src/remote_method.rs` — the descriptors.
- `rust/tcl-registry/src/commands/irules/ilx__{init,call,notify}.rs` — the
  three specs that carry them.
- `rust/tcl-irules/src/ilx.rs` — the Tcl walk and the JavaScript scanner.
- `rust/tcl-lsp-core/src/ilx_navigation.rs` — workspace association,
  definition / hover / references.
- `rust/tcl-lsp-core/src/config_ini.rs` — the `[iruleslx.*]` INI sections.
- `editors/vscode/src/ilxReferences.ts` — the client-side JavaScript
  reference provider.
- `rust/tcl-lsp-server/tests/e2e/issue1707_ilx_methods.rs` — the end-to-end
  suite, over a real on-disk ILX workspace.
