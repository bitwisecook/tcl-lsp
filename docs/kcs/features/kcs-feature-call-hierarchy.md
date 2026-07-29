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

For a `TclOO` method, incoming/outgoing edges are intra-class: an edge fires
on a `my <method>` dispatch site in a sibling method's body, at any nesting
depth (`[…]` substitutions, `if` / `while` / `foreach` / `switch` / `try` /
`catch` / `eval` bodies, or any combination). A bare `<method>` call is
never a `TclOO` dispatch — a method is not a bare-callable command, so a
bare head errors "invalid command name" at runtime — so it is never treated
as a call edge, matching real Tcl semantics rather than a look-alike shape.
A method also gets outgoing edges to plain procs it calls bare, same as a
proc-to-proc edge. External `$obj method` dispatch (a different class or
document calling in) is not part of the method call graph — it is a Find
References concern instead.

A `classmethod` is different: it is dispatched on the class's own command,
`ClassName <method>`, so that shape is a call edge too. A class command is
an ordinary global command, so the edge fires wherever the call is written
— inside another classmethod, inside an instance method, inside a plain
proc, or at the top level (the caller is then shown as `<top-level>`).
Because it is a real command call, `Factory make` where `Factory` happens
to be an ordinary proc is a call to *that proc*, not to any class's
same-named classmethod, and no method edge is created for it.

An instance `method` and a `classmethod` sharing a name (rare, but `TclOO`
keeps them in independent tables, so it's legal) never cross-link: `my
<word>` dispatch scope depends on which table the *caller's own body*
belongs to (`self` is the class object inside a `classmethod`'s body, the
instance everywhere else), so a `my` edge only ever connects two members of
the same kind. The `ClassName <method>` shape reaches only the class
object's own table, so it never edges to a same-named instance method.

## File-path anchors

- `rust/tcl-lsp-core/src/call_hierarchy.rs`
- `rust/tcl-lsp-core/src/references.rs` (`scan_my_method_sites` — the
  shared `my`-dispatch matcher call hierarchy, Find References, rename,
  and the code lens all resolve through — and
  `find_obj_method_call_sites`, the shared bare `ClassName <method>`
  classmethod-dispatch scanner)

## Failure modes

- Missing edges when procs are called via variable indirection
  (`set cmd greet; $cmd`).
- A `next` / `nextto` super-dispatch is not a call-hierarchy edge (Find
  References does surface it — see
  [Find References](kcs-feature-references.md)).
- An external `$obj method` call from a different class or document is not
  an incoming edge on the method (intra-class only).

## Test anchors

- `rust/tcl-lsp-core/src/call_hierarchy.rs` (`mod tests`, including
  `outgoing_calls_does_not_conflate_method_and_classmethod_sharing_a_name`
  and `prepare_resolves_classmethod_over_same_named_method_by_cursor`)
- `rust/tcl-lsp-server/tests/e2e/navigation_extras.rs`
  (`method_incoming_and_outgoing_calls_match_my_dispatch`,
  `method_outgoing_calls_nested_in_control_flow`,
  `classmethod_incoming_and_outgoing_calls_match_bare_class_dispatch`,
  `bare_dispatch_on_a_same_named_proc_is_not_a_classmethod_edge`)

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

Placing the cursor on `format_name` and running **Show Call
Hierarchy** opens a tree view where the **Incoming calls** pane
lists `greet` and the **Outgoing calls** pane lists `string` —
click either one to jump to its definition.

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
