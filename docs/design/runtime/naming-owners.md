# Runtime and VM naming owners

Both execution engines, the runtime (`runtime/rust`) and the bytecode VM
(`rust/tcl-vm`), take their naming rules from the shared owners in
`tcl_syntax::naming` (`rust/tcl-syntax/src/naming`). An engine keeps only what
is physically its own: namespace tokens and their lifetimes, command and
variable cells, frames, and the original argument objects a native operation
reads. This document lists which owner each kind of engine consumer asks, and
the release rules those owners carry.

The C oracle behind the rules below is
[`naming-runtime-audit-open-questions.md`](../../../evidence/tcl/naming-runtime-audit-open-questions.md).

## Issuers

Each owner takes the selected issuer: a C release (8.4, 8.5, 8.6, 9.0, 9.1) or
Jim. Jim keeps flat command and variable tables keyed by counted names; C keeps
a namespace tree. A compatibility profile never grants another issuer's rules,
and BIG-IP behaviour needs appliance evidence of its own.

## Namespaces

- A namespace a script names resolves through the namespace address owner:
  `Interp::namespace_address_components` in the runtime
  (`interp/namespace_address.rs`, reached through `find_named_namespace`), and
  `namespace_address_from_native_bytes` in the VM. This covers `namespace eval`
  creation, `apply` lambda namespaces and the `find_namespace` state traits.
- Jim's flat namespaces, and a profile with no issuer, keep the namespace
  table's own walk (`Namespaces::find_namespace`). Fixed support names
  (`::tcl::mathfunc`, `::tcl::mathop`, `::oo::*`, stock ensembles) also use it
  directly; they are bootstrap slots, not script lookups.
- A deleted namespace's token is never reused for one recreated under the same
  name. Imports, `namespace origin` targets and ensembles bound to the old
  token do not reach the new one.

## Commands

- Lookup, publication, C API publication, alias publication and rename each
  have their own projection on `NativeNameProtocol`
  (`command_lookup_projection`, `command_publication_projection`, and so on).
  A creation never borrows lookup's namespace-path or global fallback.
- A member's qualified report joins through
  `native_qualified_inventory_report_bytes`; the runtime's
  `Namespaces::qualified_member_report` supplies the namespace's report parts
  (its Jim flat holder, or its constructed C path). Other qualified keys use
  `rooted_member_key_bytes` and, for Jim globals, `jim_global_variable_key_bytes`.
  Engines do not join `::` by hand.

## Variables

Variable failures are reported through `report_native_variable_diagnostic_at`
with the operation, the reason the engine observed, and the stage it reached
(`NativeVariableFailureSite`). The owner applies the C rules:

- Through an `upvar` link into a deleted namespace or a deleted array, only a
  store reports `upvar refers to variable in deleted namespace` or
  `upvar refers to element in deleted array` (`TCL WRITE VARNAME` from 8.6). A
  read or unset finds the dead cell undefined and reports `no such variable`
  with `TCL READ VARNAME` or `TCL UNSET VARNAME`. 8.4 `incr` reads first; from
  8.5 it reaches the store.
- An unset that reaches a variable entry with no value (declared, linked, or a
  compiled local already unset) fails at the unset itself: engines pass
  `ValueUnset`, giving `TCL UNSET VARNAME` from 8.6. Only a name with no entry
  fails at lookup, giving `TCL LOOKUP VARNAME <name>`.
- A read that asks for no diagnostic (`Tcl_ObjGetVar2` without
  `TCL_LEAVE_ERR_MSG`, as a no-value `lappend` reads) still leaves the
  `ValueRead` code on a successful completion once it reaches an entry with no
  value (`TCL READ VARNAME` from 8.6), but a name with no entry leaves nothing.
  9.1 compiles `lappend name` as a reached read, so it leaves the code there too.
- 8.5 tags exactly one variable failure: an unset of a missing array element,
  `TCL LOOKUP ELEMENT <key>`. 8.4 tags none.
- `namespace which -variable` answers declared variables, including ones
  without a value, and never elements; `info exists` and `namespace upvar` reach
  elements.

## Jim's scripted distribution

Jim implements part of `namespace` (`import`, `upvar`, `origin`, `info`,
`ensemble` and others), and several `dict` subcommands, as Tcl procedures that
jimsh loads as static extensions. The registry carries them as
`NativeScriptedLibrary` rosters. A native-core interpreter
(`Interp::with_native_core` in the runtime, `native_fixture::interpreter` in
the VM) installs none of them; an embedding or test that compares against
jimsh output installs them explicitly
(`cmd_proc::install_stock_scripted_wrappers`,
`native_fixture::install_distribution`). The VM's Jim `exists` implements the
`-command`, `-proc` and `-alias` selectors the helpers use.

## NUL in names and values

In every C release a Tcl NUL character is part of a namespace, command or
variable name, never its end. C stores it as the modified-UTF-8 pair `C0 80`;
a raw `00` byte only reaches a value from a C caller or as a raw byte written
in the source, and C keeps that byte as it is. The `CString` extents the naming
owners apply (`c_string_extent`) therefore cut only at a raw `00`.

Both engines follow the same rule for C releases; Jim keeps its counted bytes:

- `NativeStringProtocol::host_text_bytes` and `host_decoded_bytes`
  (`tcl_syntax::native_string`) spell a host NUL as `C0 80`, and
  `RawString::unicode` reads `C0 80` back as a NUL. Every other invalid
  sequence is still refused.
- Source escapes (`\x00`, `\u0000`, `\0`) decode to `C0 80`: the runtime takes
  the native escaped-text path in every C release, the VM's code generator
  decodes through `native_source_string_bytes_in`, and `subst` follows the same
  rule. A folded `format` result is encoded where the registry folds it.
- Results built from host text are encoded: in the runtime every
  `ValueOps::new_str`/`new_string`; in the VM `ValueOps::new_text`, which the
  shared character-level string and `split` results use, plus `format`,
  compiled `string map` and `encoding convertfrom`. Other VM values built from
  host text keep a raw `00` raw, as C does for bytes it never decodes.
- Channel output decodes `C0 80` to a NUL byte (`encode_output_bytes` and the
  VM's `puts`), and the runtime's `env` writes decode it and stop at the first
  NUL, as C's external encoding and `environ` do.

Known differences: Tcl 9 orders a NUL before every other character in
`lsort -ascii` and `string compare`, while both engines compare the stored
bytes, which is 8.x order; and the VM stores a compiled variable name decoded
from an escaped NUL (`set "v\x00w" 1`) as a raw `00`, so `info exists` with the
same literal misses it; and the VM's `dict get` with a NUL key aborts.
