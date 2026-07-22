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

TclOO dispatch is followed as well. A method is found through every `$obj method` call on a tracked instance, every intra-class `my method` dispatch, and `next` / `nextto` super-dispatch, whether the call sits at the top level of a body, inside a `[…]` command substitution, or embedded in a quoted or compound word such as `"value: [my get]"`. A `classmethod` is dispatched differently — on the class's own command, never on an instance — so its references are every bare `ClassName method` call, including from a subclass's own command when the subclass inherits (does not override) the classmethod, in the defining file, a subclass-only file, or a pure-consumer file that never mentions the class body at all (the workspace index's per-method `kind` carries the "this is a classmethod" fact to whichever file needs it). The same applies to snit's `typemethod` — snit and TclOO members are both registry-declared class methods, so this is not a TclOO-specific check. An expr math function resolves to its backing proc, so a `proc ::tcl::mathfunc::foo` is found from every `foo(...)` written inside an `expr`.

Triggering from the classmethod's own bare dispatch site (not just its declaration or a code lens) resolves too — the cursor's receiver word is resolved directly to the class whose `classmethod` it names, the reverse of the forward "which names dispatch this class" lookup used to *count* references. A class that declares both a `method` and a `classmethod` of the same name (TclOO keeps them in separate dispatch tables, so this is legal) is disambiguated explicitly: unambiguous when only one exists, otherwise by which declaration the cursor is actually on, never inferred from bare name-map membership — so each one's lens, reference count, and rename affect only its own dispatch shape.

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

- `rust/tcl-lsp-core/src/references.rs` (`find_obj_method_call_sites` —
  instance dispatch plus a classmethod's own-class-command dispatch)
- `rust/tcl-lsp-core/src/definition.rs` (shared namespace-aware resolvers)
- `rust/tcl-compiler/src/analyser/oo.rs` (`record_member_command_references` —
  `superclass` / `mixin` / `inherit` / `forward` as command references)

## Failure modes

- A class referenced only through a dynamic (`$var`) superclass / mixin name is
  not linked — the name is not statically decidable.
- [incr Tcl] dispatches a class-scoped `proc` (itcl's own class-method form)
  as a single `::`-qualified identifier — `Factory::make`, not `Factory make`
  — which is a different call shape entirely from `classmethod` /
  `typemethod`'s two-word dispatch. Find References does not yet follow this
  form (it would need to be resolved through the ordinary proc-reference
  path, not the object-method scanner). The reverse direction is guarded,
  though: itcl's own two-word instance-creation syntax (`ClassName
  instanceName`), which can coincidentally share text shape with a
  same-named class-proc, is never mistaken for a dispatch of it.
- A classmethod's / typemethod's own-class-command dispatch (unlike an
  instance's `$obj method`) is matched by exact name-set membership (the
  class's as-written simple name plus its fully `::`-qualified name), so a
  call spelled with a *partial* namespace qualifier from a sibling namespace
  is not matched — the same imprecision `CLASS create NAME` object-command
  dispatch already has. The same imprecision can also over-match: two
  unrelated classes sharing a simple (tail) name in different namespaces
  aren't distinguished by lexical scope, so a bare dispatch inside one
  namespace can be wrongly attributed to the other's same-named class —
  renaming from the wrong one's declaration could then rewrite the
  unrelated class's call site. Tracked as a follow-up; not yet fixed.

## Test anchors

- `rust/tcl-lsp-core/tests/references_residual.rs`
- `rust/tcl-lsp-core/tests/name_resolution.rs` (`classmethod_dispatch`,
  `obj_method_dispatch` — TP/FP/TN/FN matrix, including issue #956's exact
  repro, snit `typemethod`, the itcl `proc` boundary in both directions,
  and call-site-cursor resolution)
- `rust/tcl-lsp-server/tests/e2e/issue923_class_refs.rs` (cross-file)
- `rust/tcl-lsp-server/src/lib.rs` unit tests: `cross_file_consumer_finds_classmethod_bare_dispatch`,
  `cross_file_consumer_finds_inheriting_subclass_classmethod_dispatch`,
  `cross_file_method_references_reach_inheritor_document_for_classmethod`,
  `classmethod_rename_reaches_subclass_only_document`,
  `cross_file_consumer_does_not_bare_dispatch_an_itcl_class_proc`,
  `code_lens_resolve_disambiguates_method_and_classmethod_of_the_same_name`,
  `rename_disambiguates_method_and_classmethod_of_the_same_name`

## Screenshots

- `16-references` — find all references panel

![find all references panel](../screenshots/16-references.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
