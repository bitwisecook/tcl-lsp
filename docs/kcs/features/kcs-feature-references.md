# KCS: feature — Find References

> **Audience:** User
> **Type:** Functionality

## Summary

Find all references to a proc, variable, class, or TclOO method across the
file. Method dispatch (`$obj method`, `my method`) and `expr` function calls
that resolve to a `::tcl::mathfunc` proc are counted as references.

## Applies to

all-editors, MCP, analyser

## How to use

- **Editor**: Shift+F12 or right-click > Find All References.
- **MCP**: `find_references` tool — pass source, line, and character position.
- **Settings**: Toggle with `tclLsp.features.references`.

## Operational context

Locates all usages of the symbol under the cursor, including definitions, calls, and variable reads/writes. Uses shared proc-reference matching. For a TclOO method, a `$obj method` dispatch is resolved through the same [SSA](../../GLOSSARY.md#ssa) OBJECT type lattice the unknown-method check uses: the receiver's class comes from the lattice, and the class hierarchy maps the method to the ancestor that defines it, so an inherited call is credited to that ancestor. A `my method` self-dispatch uses the enclosing method's class. Because the lattice only ever types genuine objects, a channel or callback call such as `$chan gets` is never mistaken for a method reference. An `expr` function call `Foo(...)` is matched to the proc `::tcl::mathfunc::Foo` when one is defined.

## File-path anchors

- `server/features/references.py`
- `analyser/proc_lookup.py`
- `analyser/_analyser/_commands.py` — method-dispatch and `expr` call-site capture
- `analyser/_analyser/_diag_var_command.py` — `_record_method_invocations` resolves dispatch sites via the OBJECT type lattice

## Failure modes

- Incomplete references after scope or namespace changes.
- A method call through an object the type lattice cannot type (e.g. an object returned by a factory proc) is not resolved — the same limitation as the W308 unknown-method check, which shares the lattice.

## Test anchors

- `tests/test_references.py`
- `tests/test_tickets_954_958.py` — method and `expr` mathfunc references

## Screenshots

- `16-references` — find all references panel

![find all references panel](../screenshots/16-references.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
