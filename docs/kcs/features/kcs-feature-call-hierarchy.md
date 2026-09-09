# KCS: feature — Call Hierarchy

> **Audience:** User
> **Type:** Functionality

## Summary

View incoming and outgoing calls for a proc or `TclOO` method.

## Applies to

all-editors, MCP, analyser

## How to use

- **Editor**: Right-click a proc or method > Show Call Hierarchy, or Shift+Alt+H.
- **MCP**: `call_graph` tool — pass source for the full call graph.
- **Settings**: Toggle with `tclLsp.features.callHierarchy`.

## Operational context

The call hierarchy provider traces call relationships between procs, showing which procs call a given proc (incoming) and which procs it calls (outgoing).

For a `TclOO` **method**, edges are intra-class: one fires on a `my <method>`
dispatch site in a sibling method's body, at any nesting depth. A bare
`<method>` head is never a dispatch — real Tcl answers "invalid command name"
— so it is never an edge. A method also gets outgoing edges to plain procs it
calls bare. External `$obj method` dispatch from another class or document is
a Find References concern, not a call-hierarchy edge.

A **classmethod** dispatches on the class's own command, `ClassName <method>`,
which is an ordinary global command, so the edge fires wherever the call is
written — in another classmethod, an instance method, a plain proc, or at the
top level (shown as `<top-level>`) — including frame-shifting bodies such as
`namespace eval` or an `apply` lambda, where `$obj method` would not count.
`Factory make` where `Factory` is an ordinary proc is a call to *that proc*,
and creates no method edge.

An instance `method` and a `classmethod` sharing a name never cross-link.
`my <word>` resolves against the table the caller's own body belongs to, and
`ClassName <method>` reaches only the class object's table. A caller that
writes both — `my make` and `C make` — gets two callee entries, each pointing
at its own declaration, because edges are grouped by declaration rather than
by display name.

Callers are listed by qualified name — `::util::helper` for a proc,
`::Factory::build` for a method — so both kinds read consistently.

## Failure modes

- Missing edges when procs are called via variable indirection
  (`set cmd greet; $cmd`).
- A `next` / `nextto` super-dispatch is not a call-hierarchy edge (Find
  References does surface it — see
  [Find References](kcs-feature-references.md)).
- An external `$obj method` call from a different class or document is not
  an incoming edge on the method (intra-class only).
- [incr Tcl]'s class-scoped `proc` gets edges for its real
  `Factory::make` dispatch shape, but only within one document — a call in
  a sibling file is not an incoming edge. itcl's two-word `Factory make`
  is object creation (`ClassName instanceName`), not a dispatch, and is
  correctly never an edge.

## Example

Given this Tcl source:

```tcl
proc greet {name} {
    puts "Hello, [format_name $name]"
}

proc format_name {name} {
    return [string totitle $name]
}

greet "world"
```

Placing the cursor on `format_name` and running **Show Call Hierarchy**
opens a tree view whose **Incoming calls** pane lists `greet`; on `greet`,
**Outgoing calls** lists `format_name`.  Click either to jump to its
definition.  Built-in commands such as `string` are not edges — only procs
and `TclOO` members are.

The same works for a class method:

```tcl
oo::class create Greeter {
    method greet {name} { my format_name $name }
    method format_name {name} { return [string totitle $name] }
}
```

Placing the cursor on `format_name` shows `greet` as an incoming call — the
`my format_name` dispatch site inside it.

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
