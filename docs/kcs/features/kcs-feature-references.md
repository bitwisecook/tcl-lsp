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

TclOO dispatch is followed as well. A method is found through every `$obj method` call on a tracked instance, every intra-class `my method` dispatch, and `next` / `nextto` super-dispatch, whether the call sits at the top level of a body, inside a `[…]` command substitution, embedded in a quoted or compound word such as `"value: [my get]"`, or nested inside `if` / `while` / `foreach` / `switch` (either the inline or single-braced-clause-list form) / `try` / `catch` / `eval` / `dict for` — any combination of these, arbitrarily deep. A property is found the same way through every `my <property>` read (a property has no `$obj` dispatch or inheritance model). A `classmethod` is dispatched differently — on the class's own command, never on an instance — so its references are every bare `ClassName method` call, including from a subclass's own command when the subclass inherits (does not override) the classmethod, in the defining file, a subclass-only file, or a pure-consumer file that never mentions the class body at all (the workspace index's per-method `kind` carries the "this is a classmethod" fact to whichever file needs it). Because a class command is an ordinary command rather than a frame-bound receiver, a bare classmethod dispatch is also found inside a `namespace eval` body or an `apply` lambda body — the two places an instance's `$obj method` dispatch is deliberately *not* followed, since `$obj` names something different there. The same applies to snit's `typemethod` — snit and TclOO members are both registry-declared class methods, so this is not a TclOO-specific check. A stock-TclOO `self method`, however, is not inherited: a subclass's own class command never reaches it, so `Subclass method` is not a reference to a parent's `self method` (only `ooutil`'s `classmethod` keyword propagates that way). An expr math function resolves to its backing proc, so a `proc ::tcl::mathfunc::foo` is found from every `foo(...)` written inside an `expr`.

A method, a classmethod, and a property can share one name within the same
class (rare, but each lives in its own independent table, so it's legal);
Find References resolves to whichever declaration the cursor actually sits
on rather than a fixed priority, so the reference count for one member is
never silently attributed to a different, same-named one.

Triggering from the classmethod's own bare dispatch site (not just its declaration or a code lens) resolves too — the cursor's receiver word is resolved directly to the class whose `classmethod` it names, the reverse of the forward "which names dispatch this class" lookup used to *count* references.

A class is found through every use of its name, not only `<Class> new` instantiations: a `superclass`, `mixin`, or `[incr Tcl]` `inherit` argument that names the class is a reference to it, and a `forward` member's delegated command is a reference to that command. These references are resolved by the class's namespace exactly as a call would be, so a fully-qualified `superclass ::ns::Base` in one file is found from `::ns::Base`'s declaration in another, and a same-named class in an unrelated namespace is never cross-linked. Because the same references drive rename, renaming a class rewrites every `superclass` / `mixin` / `inherit` site that names it, keeping the inheritance graph intact.

A **namespace variable** is found across files, in both directions: from the
`variable v` token inside `namespace eval ns { … }` to every `$::ns::v` /
`$ns::v` occurrence in sibling documents, and from any of those occurrences
back to the declaration and to each other. Only *qualified* occurrences
count — a bare `$v` names whatever the local scope chain supplies, so it is
not attributable to a namespace cell from outside its own file, and a
`$name`-shaped run inside a comment or data brace is not an occurrence at
all. Proc locals are unaffected: they keep their existing within-file
reference set.

A command reached through a **mutated command table** is found too. An
`interp alias {} sayHi {} greet` makes every `sayHi` call a real call site of
`greet`, and a `rename greet hello` makes every later `hello` one, so both
appear in `greet`'s reference set — and a query started from the alias or the
renamed-to name answers the same unified set. Both directions are order-gated:
a call written before the `interp alias` or `rename` is not attributed to the
target, matching Tcl, where it raises `invalid command name`. Rename
deliberately does **not** rewrite those call sites — they spell a *different*
command's name, which keeps its own spelling when the target is renamed. When
a name carries both a `rename` and an `interp alias`, the one written later is
the one whose call sites are attributed.

A callback wrapped for its namespace — `trace add variable v w [namespace
code [list Handler]]`, the idiom Tk's own `fontchooser.tcl` uses — is a
reference to `Handler`, in the `[list …]`, braced, and bareword forms alike.
Each such callback counts once, so the reference lens above the declaration
reads the same number Find All References opens.

A proc declared twice in one file is one command with two headers: a query
from either header returns the same set. A `rename` between the two
declarations breaks that, because it takes the command **object** away under
its own name: `proc p …`, `rename p oldp`, `proc p …` leaves two unrelated
commands, and the surviving `p`'s reference set excludes the `oldp` call
sites — they belong to the definition the rename carried off.

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

### Caller-frame variables

A call-by-name helper writes its caller's variable through `upvar`, so the
name appears twice in two different shapes — once bare, as the argument that
names it, and once with a `$`, as the read:

```tcl
proc setdef {varName} { upvar 1 $varName target; set target "default" }
proc build {} {
    setdef options       ;# the bare word names the variable
    return $options      ;# and this reads it
}
```

Find All References treats both as one variable, so asking from either the
bare `options` on the `setdef` line or the `$options` read returns the same
two locations. Linking them is the point of the idiom — without it the bare
word looks unrelated to every read it feeds. Occurrences Tcl never
substitutes (inside a comment, or inside a brace-quoted data word) are left
out, exactly as they are for an ordinary variable.

Turning declarations off (`includeDeclaration: false`) drops only the call
sites that **create** the variable. A helper that upvar-*reads* its caller's
variable without writing it (`peek options`) creates nothing, so that call
site is an ordinary reference and stays in the list.

Calls written inside an `if`, `while`, `foreach`, `catch` or `switch` body run
in the same frame and are found; calls inside a nested `proc`, an `apply`
lambda, a `namespace eval` or an `uplevel` body run in a different frame and
are not. Only `upvar 1` (or an omitted level) reaches the caller's frame at
all — `upvar 0`, `upvar #0` and `upvar 2` name other frames, so a call to a
helper using one of those binds nothing here.

When nothing binds the name, Find All References returns nothing rather than
falling back to a command or method of the same name — a `$`-led token can
only ever be the variable.

### Namespace names

A namespace is a symbol in its own right. Asking for references from the
`::tomato` of `namespace children ::tomato` — or from the name word of a
`namespace eval ::tomato { … }` block — returns every other place that names
that namespace: `namespace exists`, `namespace delete`, `namespace parent`,
`namespace upvar`, `namespace inscope`, and every declaring `namespace eval`
block, in this file and in every other file in the workspace. Turning
declarations off drops the `namespace eval` blocks and keeps the rest.
Relative names count: inside `namespace eval ::outer`, a bare `inner` names
`::outer::inner` and is listed with the qualified spellings of the same
namespace. Words that merely look like namespace names are not listed —
`namespace tail` and `namespace qualifiers` take arbitrary strings, and
`namespace import` / `export` / `forget` take glob patterns.

### Method-name references in a class body

A method's references include the definition-body words that *name* it, not
just the calls that dispatch it: `export m`, `unexport m`, `filter m`,
`deletemethod m`, `renamemethod m n`. Those are the registry's own
method-reference members, so the same list holds for `TclOO`, snit, and
[incr Tcl] without the provider naming a keyword. They matter beyond
completeness: a `TclOO` method whose name starts with an upper-case letter is
unexported by default, so an `export` list that disagrees with its
declaration is a broken class, and Rename rewrites those words for exactly
that reason.

## File-path anchors

- `rust/tcl-lsp-core/src/references.rs` (`find_obj_method_call_sites` —
  instance dispatch plus a classmethod's own-class-command dispatch;
  `member_reference_spans` — `export` / `unexport` / `filter` method-name
  words, from the definer grammar)
- `rust/tcl-lsp-core/src/caller_frame.rs` (caller-frame variables — the
  call-site word and the reads it feeds, shared with Hover and Go to
  Definition)
- `rust/tcl-lsp-core/src/definition.rs` (shared namespace-aware resolvers)
- `rust/tcl-lsp-core/src/namespace_symbol.rs` (the one namespace resolver Go
  to Definition, Hover, and Find References all answer through)
- `rust/tcl-compiler/src/analyser/oo.rs` (`record_member_command_references` —
  `superclass` / `mixin` / `inherit` / `forward` as command references)

## Failure modes

- A class referenced only through a dynamic (`$var`) superclass / mixin name is
  not linked — the name is not statically decidable.
- A `my` / `next` / `$obj method` dispatch written inside `uplevel`, a
  `namespace eval` body, or an `apply` lambda is not found: `uplevel`'s body
  may run in a different call frame (only level `0` is truly the same frame,
  and that can't be told apart from `uplevel 1`+ statically), a `namespace
  eval` body resolves `$obj` against that namespace's own variables, and an
  `apply` lambda's body runs in its own frame that has no route back to the
  enclosing object's `my` unless the lambda is explicitly constructed with
  the object's namespace. A bare `ClassName classmethod` dispatch (and a
  `CLASS create NAME` object command's `NAME method`) **is** found in all of
  those bodies — a class command is an ordinary command and resolves from any
  frame.
- Inside an `apply` lambda, only a `{braced}` body element is searched. A
  body written as a bare or double-quoted list element is backslash-decoded
  before `apply` evaluates it, so its source text is not the script that runs
  and a call site found in it would point at the wrong bytes.
- Expect's `expect { -re pattern body … }` clause list is not descended (only
  a plain `{pattern body …}` clause list, `switch`'s shape, is).
- A method whose name is a bare operator — Tcl lets you write `method + {o}
  {…}` — is not resolved from its call site. Every other method name is:
  hyphens, dots, and TIP 558's generated property accessors
  (`<ReadProp-NAME>` / `<WriteProp-NAME>`, produced by `oo::configurable`'s
  `property`) are all part of the name. Operator-only names stay out because
  `expr {$a < $b}` would otherwise read as `$a` dispatching a method called
  `<`, and `expr` bodies are far more common than operator-named methods.
- [incr Tcl]'s class-scoped `proc` dispatch is followed **within one
  document only**. itcl gives every class a real namespace and installs its
  class-scoped `proc`s as commands inside it, so `Factory::make` is a genuine
  command name; Find References, Go to Definition, Rename, and Call Hierarchy
  all resolve it against the call's own namespace. A call written in a
  *sibling file* is not found — the cross-file layer still looks only for the
  two-word `Class method` shape. The two-word form in itcl source is
  guarded in the other direction too: `Factory make` there is itcl's
  instance-creation syntax (`ClassName instanceName`), never a class-proc
  dispatch, and is never counted as one.
- A `CLASS create NAME` object command (`rex bark`) is resolved against the
  call's own namespace, exactly like a classmethod's class-command dispatch
  (`Factory make`). `CLASS create NAME` binds `NAME` in the *creation site's*
  namespace, so `::a::Factory create rex` and `::b::Widget create rex` are
  two different commands that coexist (tclsh 9.0 and 8.6 both dispatch `rex
  make` inside `::a` to `::a::rex` and inside `::b` to `::b::rex`), and the
  two are never cross-linked. This was issue #981's object-command half,
  closed by PR C3: previously the two were one flat name, so references on
  one class's method counted the other's call site and a rename rewrote it.
  Object commands created by a *registry* object factory (a Tk widget path,
  a tcllib naming factory) still match by bare name — those carry no
  creating user class to attribute a dispatch to in the first place.
- When two different classes bind the **same qualified** object command in
  one namespace, which class a later `NAME method` reaches is a runtime
  fact. References still report both call sites; **rename refuses**, with a
  reason (see [Rename](kcs-feature-rename.md)).

## Test anchors

- `rust/tcl-lsp-core/tests/references_residual.rs`
- `rust/tcl-lsp-core/tests/name_resolution.rs` (`my_method_dispatch`,
  `obj_method_dispatch`, `next_dispatch`, `classmethod_dispatch` —
  control-flow-nested dispatch TP/FP/TN matrix (issue #957) and the
  classmethod/typemethod/itcl-proc TP/FP/TN/FN matrix, including issue #956's
  exact repro and call-site-cursor resolution;
  `non_identifier_method_names` — hyphenated / dotted / TIP 558
  angle-bracketed method names; `itcl_class_proc_dispatch` — the
  colon-qualified [incr Tcl] class-proc TP/FP/TN matrix;
  `namespace_scoped_class_dispatch` — two same-named classes in different
  namespaces are not cross-linked, and (PR C3) two same-named `CLASS create
  NAME` object commands in different namespaces are not either)
- `rust/tcl-lsp-core/src/references.rs` (`mod tests`, including
  `references_for_property_includes_decl_and_my_dispatch_call_sites`,
  `references_disambiguates_property_and_method_sharing_a_name_by_cursor`,
  `dispatch_scan_depth_guard_stops_runaway_nesting`,
  `tn_expect_clause_flags_not_decomposed`)
- `rust/tcl-lsp-server/tests/e2e/issue923_class_refs.rs` (cross-file)
- `rust/tcl-lsp-server/tests/e2e/issue1088_namespace_symbols.rs` (namespace
  names as references, in one file and across files)
- `rust/tcl-lsp-server/tests/e2e/issue923_crossdoc.rs` (cross-file namespace
  variables, TP + TN)
- `rust/tcl-lsp-server/tests/e2e/tcloo_navigation.rs` (rename / references /
  code-lens agreement on the same `TclOO` member — issues #991, #993)
- `rust/tcl-lsp-server/tests/e2e/name_resolution.rs`
  (`my_method_dispatch::tp_my_dispatch_nested_in_control_flow_reference_and_lens`)
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
