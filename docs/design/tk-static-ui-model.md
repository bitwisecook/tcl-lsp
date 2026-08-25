# Static Tk UI model

Status: implementation contract for the Tk preview surface. The model is
static and conservative; it is not a Tk interpreter and it is not a screenshot
of a running application.

## Purpose

The LSP server builds one `TkUiModel` from the document snapshot already known to
the server. It uses the Tcl CST and the `tcl-registry` command and option
descriptors. The same versioned model is the source for the LSP command, the
MCP `tk_layout` tool, and editor previews. Clients must not maintain a second
Tk command or option catalogue.

The request identifies an open document rather than carrying a second source
string:

```json
{
  "uri": "file:///workspace/app.tcl",
  "version": 42
}
```

The response is stamped with `schema_version`, `document_uri`, and
`document_version`. A client accepts a response only when those values still
match the request and the active document. If a platform facade does not expose
its LSP document version, it may send `document_sha256`, an opaque SHA-256 of
the already-open text. The server verifies it without accepting a second source
copy and echoes it in the model; that client compares the echoed digest and its
current editor text before presenting the response. If an edit wins the race,
the client discards the response and waits for the model for the newer snapshot.

The design was also checked against the 2026
[CGI Coffee Tcl/Tk tutorial](https://cgicoffee.com/blog/2026/04/tcl-tk-develop-cross-platform-cli-gui-tools-tutorial-guide):
its modernized application uses grid layout, input validation, relative/DPI-aware
sizing, rate-limited canvas resize, namespaces, and event-driven callbacks, while
its code-as-data discussion calls out the security cost of dynamic evaluation.
The shipped response is deliberately structural rather than pixel-rendered;
geometry evidence and uncertainty serve the layout cases, registry timing and
taint facts serve validation/callbacks, and dynamic code remains an explicit
abstention instead of being executed for a prettier preview.

## Model contract

The stable parts of the model are:

- whether Tk is statically active for the selected dialect/document;
- a root toplevel and a tree of widget instances;
- each instance's literal pathname and registry-resolved widget type;
- source spans for the creation and geometry evidence that produced the node;
- literal option values which can be displayed safely;
- geometry-manager evidence and the parent/container to which it applies;
- orphan widgets whose literal parent is not constructed in the model;
- static facts from which a client can derive certainty, plus explicit
  uncertainty records;
- geometry conflicts and source-linked uncertainty records that the client can
  render without re-parsing Tcl.

Source spans are evidence, not an assertion that the program will execute that
line. A model may contain an incomplete tree when the source is conditional,
dynamic, or malformed. The uncertainty must be visible to the user rather than
silently filled with invented widgets or default values.

The schema is versioned independently of Tcl/Tk release detection. A client
must reject an unknown schema version and show a useful “preview unavailable”
message. Adding an optional field is compatible; changing the meaning of an
existing field requires a schema version change.

## Forms currently suitable for static preview

The foundation is deliberately limited to forms whose command and relevant
words are visible in the CST and resolve through the registry. In particular,
the model can represent:

- literal classic or `ttk::` widget constructors with a literal pathname;
- nested literal pathnames such as `.main.toolbar.save` when their literal
  parents can be established;
- balanced Tcl words, including quoted and braced option values;
- direct and `configure` literal-target `grid`, `pack`, and `place`
  placements, their registry-declared options, and registry-declared
  `forget`/`remove` releases;
- nested executable bodies that the shared Tcl walker can visit;
- registry-recognized widget options whose values are literal Tcl words.

Window-manager commands, row/column configuration, resource creation, and
other commands may be useful context to a future model, but are not currently
represented as verified `TkUiModel` facts. Unrecognized options are not
interpreted by the model; non-literal values are recorded as uncertainty.

The preview is an approximation of structure and declared layout. It does not
claim pixel accuracy, platform-native painting, font metrics, theme behavior,
or event-loop behavior.

## Abstention and uncertainty

The analyser should abstain locally when it cannot prove a fact. Examples
include:

- a widget command or pathname comes from a variable, command substitution,
  `eval`, `uplevel`, or an unknown alias;
- a source or procedure boundary prevents the relevant creation/configuration
  from being associated with the current snapshot;
- option names or values are computed, or a resource is created conditionally;
- control flow may execute different constructors or geometry managers;
- a widget is destroyed, renamed, or recreated in a way that makes the final
  state ambiguous;
- an extension provides a widget type not present in the selected registry
  profile.

An uncertain node may still be useful in the tree, but it must carry an
uncertainty reason and must not be presented as a verified runtime fact. A
blank or partial result is preferable to executing code or inventing a layout.
Individual uncertainty records are capped at 200 in document order. The model
reports the number omitted in `uncertainties_truncated`, and clients must show
that total rather than implying the retained prefix is the complete set. This
keeps wrapper-heavy or generated UIs bounded without hiding that the static
model abstained more often.

The serialized tree is independently capped at 1,000 distinct constructed
widget paths plus the implicit root. `widget_count` reports the distinct paths
represented or omitted, including the root, and `widgets_truncated` reports
how many constructor facts for new paths were omitted. Duplicate constructors
for one path are represented by an uncertainty rather than counted as separate
widgets. A generated document therefore cannot force unbounded JSON or DOM
recursion, and the client does not imply that the bounded tree is complete.

## Geometry rule

Tk's geometry-container ownership rule is narrower than “one manager per
parent.” `pack` and `grid` claim an effective container through
`TkSetGeometryContainer`; with propagation enabled, both cannot claim the same
container. `place` manages a widget but does not claim or resize its container,
so it can coexist with either manager. All three accept `-in`, which can make
the effective geometry container differ from the widget pathname's parent.
These facts live in each manager's registry descriptor. Dynamic `-in`,
propagation changes, control flow, or interpreter-crossing placement must be
reported as uncertain rather than promoted to a definite conflict.

The model is temporal, not a bag of commands. Re-managing one content widget
releases its previous manager first, and `forget`/`remove` release it
explicitly. Thus `pack .a; grid .a` is legal when `.a` is the only packed
child, while `pack .a .b; grid .a` conflicts because `.b` keeps `pack`'s
container claim active. Query and container-configuration subcommands never
create placement facts. A conflict is an execution event: a later `destroy` or
reconstruction may change the final tree, but cannot erase the earlier Tk
error that rejected the placement.

The official [grid manual](https://www.tcl-lang.org/man/tcl8.6/TkCmd/grid.htm)
and [pack manual](https://www.tcl-lang.org/man/tcl8.6/TkCmd/pack.htm) describe
the manager/container relationship. `rowconfigure` and `columnconfigure`
remain useful future model inputs, but the current schema does not record them.

## What is intentionally not in the current model

The following are useful future graph inputs, but are not currently promised
as shipped preview facts:

- a complete callback/event graph;
- resource lifetime and reachability graphs for images, fonts, menus,
  variables, and channels;
- event-loop scheduling, timer execution, or callback return values;
- computed geometry, theme/style resolution, platform window-manager effects,
  or accessibility behavior;
- execution of `source`, packages, network/file operations, or arbitrary Tcl.

The registry already contains callback/body and command-prefix facts. Those
facts can support future static edges, but a client must not claim that a
callback graph exists merely because a callback-shaped option was recognized.

## Security boundary

Static preview never invokes `tclsh`, `wish`, packages, callbacks, or user
code. It must not read arbitrary files or use a workspace-provided interpreter
as part of rendering.

A future runtime preview would be a separate, explicit feature with a distinct
request and result type. It would require opt-in, process isolation, strict
timeouts and cancellation, constrained filesystem/network access, a clean
environment, and a clear warning that the program is being executed. Runtime
preview must not be silently substituted for the static model when static
analysis abstains.

Tk's user-editable state is also an input boundary for ordinary taint
analysis. Registry instance-method metadata marks value getters for `entry`,
`spinbox`, `text`, `combobox`, scales, toggle switches, and related editable
widgets as taint sources. Text `dump`, clipboard and selection reads, and the
file/directory/colour chooser results are sources too. Argument-sensitive
getters use a zero-argument source trait: a bare scale/control `get` reads user
state, while a coordinate/limit form derived from explicit arguments does not.
The
compiler resolves those facts through the registry-declared object class, so
real calls such as `.password get` and `$widget get` use the same declaration
as hover and instance-method diagnostics. Top-level widget commands remain
available to callback procedure analysis through compilation-unit object
facts; no consumer owns a Tk command-name list.

Two-way `-textvariable` and `-variable` options carry `VarRead`/`VarWrite`
roles. Only the individual option that accepts user input carries the
input-link bit: a checkbutton's state `-variable` is a source, while its
display-only `-textvariable` is not. The constructor's `TAINTS_VAR_WRITES`
trait advertises that it has such an option, but the compiler taints only the
specific SSA variable named by that option. An actual linked variable
therefore becomes tainted even when code reads it rather than calling the
widget's getter. Registry-typed instance reconfiguration uses the same class
option table: the method carries `CONFIGURES_INSTANCE_OPTIONS`, and a call such
as `.entry configure -textvariable draft` introduces a fresh, phase-correct
SSA definition of `draft`. The SpecTcl/Spec Studio option row exposes this as
`-taints-var-write` / `taints_var_write`, so authored packs and generated Rust
round-trip the security fact instead of dropping it. `entry -show` changes only presentation: a masked password
remains untrusted input and is never treated as sanitized.

## Consumers

- The LSP `tcl-lsp.tkPreview` command analyses the server's open snapshot and
  returns the stamped model.
- MCP `tk_layout` exposes the same structured result for tools and agents.
- VS Code renders the model and rejects stale or unsupported schema versions.
  JetBrains requests the same model and currently presents its JSON rather than
  maintaining a separate renderer.
- LSP diagnostics, hover, navigation, and future editor tools should reuse
  the CST/registry evidence rather than reimplementing Tk command lists.

The implementation must preserve this ownership boundary: adding a Tk command,
option, callback shape, or resource descriptor belongs in the registry and the
shared analyser, not in a client renderer.
