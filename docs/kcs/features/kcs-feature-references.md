# KCS: feature — Find References

> **Audience:** User
> **Type:** Functionality

## Summary

Find all references to a proc, variable, class, TclOO method, or expr math
function across the file and the whole workspace.

## Applies to

all-editors, MCP, analyser

## How to use

- **Editor**: Shift+F12 or right-click > Find All References.
- **MCP**: `find_references` tool — pass source, line, and character position.
- **Settings**: Toggle with `tclLsp.features.references`.

## Operational context

Locates all usages of the symbol under the cursor, including definitions, calls, and variable reads/writes. Uses shared proc-reference matching.

TclOO dispatch is followed as well. A method is found through every `$obj method` call on a tracked instance, every intra-class `my method` dispatch, and `next` / `nextto` super-dispatch, whether the call sits at the top level of a body, inside a `[…]` command substitution, or embedded in a quoted or compound word such as `"value: [my get]"`. An expr math function resolves to its backing proc, so a `proc ::tcl::mathfunc::foo` is found from every `foo(...)` written inside an `expr`.

A class is found through every use of its name, not only `<Class> new` instantiations: a `superclass`, `mixin`, or `[incr Tcl]` `inherit` argument that names the class is a reference to it, and a `forward` member's delegated command is a reference to that command. These references are resolved by the class's namespace exactly as a call would be, so a fully-qualified `superclass ::ns::Base` in one file is found from `::ns::Base`'s declaration in another, and a same-named class in an unrelated namespace is never cross-linked. Because the same references drive rename, renaming a class rewrites every `superclass` / `mixin` / `inherit` site that names it, keeping the inheritance graph intact.

## Example

```tcl
oo::class create Store {
    method get {key} { return [my lookup $key] }
    method lookup {key} { return $key }
}
set s [Store new]
puts "value: [$s get k]"
```

Find All References on the `lookup` method name returns its declaration plus
the `my lookup` call inside `get`. On the `get` method name it returns the
declaration plus the `[$s get k]` dispatch — even though that call is inside a
double-quoted string.

## File-path anchors

- `rust/tcl-lsp-core/src/references.rs`
- `rust/tcl-lsp-core/src/definition.rs` (shared namespace-aware resolvers)
- `rust/tcl-compiler/src/analyser/oo.rs` (`record_member_command_references` —
  `superclass` / `mixin` / `inherit` / `forward` as command references)

## Failure modes

- A class referenced only through a dynamic (`$var`) superclass / mixin name is
  not linked — the name is not statically decidable.

## Test anchors

- `rust/tcl-lsp-core/tests/references_residual.rs`
- `rust/tcl-lsp-server/tests/e2e/issue923_class_refs.rs` (cross-file)

## Screenshots

- `16-references` — find all references panel

![find all references panel](../screenshots/16-references.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
