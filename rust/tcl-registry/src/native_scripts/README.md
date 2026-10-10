# Native Jim procedure bodies

These files contain the exact bodies of the pinned Jim 0.84 standard-library
procedures, including the leading/trailing newline and indentation returned
by `info body`. The adjacent `LICENSE.jim` retains Jim's original notices.
Do not add source comments inside these bodies: procedure introspection
observes their complete spelling.

`dictionary_scope::stock_scripted_wrappers` supplies the native formals and
selects this roster for the audited Jim engine. Bootstrap creates real
procedures in the retained root namespace. The separate dispatch recipe
retains the original selector spelling and argument schedule; invocation
resolves the current procedure binding rather than treating this source as
proof that an unchanged stock implementation is still installed.

Native source, body/formal captures and execution controls are retained in
`rust/tcl-vm/tests/data/native_jim_dictionary`.

`native_scripted_distribution::procedures` exposes independently selected
Dictionary, NamespaceInfo and NamespaceEnsemble rosters. NamespaceInfo
contains the original literal multiword `namespace info` helper.
NamespaceEnsemble contains the original `ensemble` constructor and
`namespace ensemble` helper together with their NamespaceInfo dependency. Both
backends use their ordinary procedure-definition owner and retain the
original formal and body objects. Native core constructors do not install
these source rosters. A full distribution specimen selects the required
library explicitly before exercising its original source.

A scripted library is bootstrap metadata, never authority that its command
still names that procedure. Jim's namespace extension constructs its helper
name from the original selector and resolves the current command binding in
the invoking namespace. Host replacements, namespace-local replacements and
deletions are therefore observed by the same command-resolution owner as
other invocations. Jim's info core retains the exact -nons scope separately
from its invocation head. Namespace-aware inventory invokes the current
literal `namespace info` binding with the unchanged original selector and
operands; direct core inventory independently selects exact -all grammar and
retained table-key spelling. Rooting the info head does not bypass namespace
forwarding. Unknown native dialects do not receive this roster or C
ensemble-configuration authority.

Add exact formal/body assets and source pins when extending a roster. Keep
introspection bytes unchanged and compare actual provider output. A caller
must select the source library independently of core inventory, compiler
admission and live command binding; neither a version label nor the stored
source establishes those other capabilities.

The Jim namespace-ensemble schema exposes exact `create` and `-automap`
words. The option concatenates its literal command-name prefix with the
requested subcommand. The shared nested keyword resolver applies the same
authored exact policy as option resolution. Source-library cards provide no
C ensemble configuration, compiler preparation, publication transition or
closed effect guarantee: those require independently retained current
helper/factory bindings.

## Consumer entry points

Use `NativeScriptedLibrary::ALL` for a distribution bootstrap. Runtime's
`cmd_proc::install_stock_scripted_wrappers` and VM's
`refresh_scripted_distribution_libraries` consume that single roster. For an
internal fixture that needs one library, use Runtime's
`install_stock_scripted_library` or VM's `install_scripted_library` with that
specific enum value. `NamespaceEnsemble` includes `NamespaceInfo` as a
required dependency. The installers register through their ordinary
procedure owner and preserve existing host/user bindings; VM additionally
retains the installed generations. Calling a helper always resolves its
current binding. Loading the roster does not confer core registration,
compiler admission or C ensemble configuration.

For Jim `info commands` and `info procs`, keep the selected
`NativeJimInfoScope` from original dispatch. `NamespaceAware` permits
forwarding; `DirectCore` is selected by the original exact `-nons` option.
Pass the original operand count, original first operand bytes and actual
retained frame namespace to `NativeJimInfoScope::command_inventory`. Its
`Flat` pattern index is relative to the member operands, excluding the
selector. `NamespaceHelper` requires an invocation of the current command
whose single name is `namespace info`, carrying the unchanged original
selector and operands. A rooted `::info` head does not select `DirectCore`.
Do not parse or consume `-all` before this decision: forwarding preserves
that option for the helper's independent argument grammar.

Direct enumeration uses `tcl_cmd_core::info::jim_core_command_list` and the
shared `NativeNameProtocol::jim_core_command_names` matcher. The physical
table owner supplies live entries and their retained
`NativeJimCommandTableKey`. `comparison_bytes()` identifies the selected
root-stripped comparison units; `report_bytes()` supplies the original
publication spelling for introspection. Replacing an occupied comparison
slot retains its original report key through `retain_for_replacement`;
rename and fresh publication select a new key through the appropriate name
purpose. These bytes do not certify an original object header or command
generation. Storage lookup, import and deletion use actual slots rather
than reparsing the reported name.

The proof records for [direct inventory](../../../../docs/design/analysis/name-resolution-proofs/info-original-root-and-explicit-nons-command-inventory.md),
[current helper forwarding](../../../../docs/design/analysis/name-resolution-proofs/info-original-namespace-helper-declaration-and-current-forwarding.md)
and [original table keys](../../../../docs/design/analysis/name-resolution-proofs/jim-original-command-table-key-publication.md)
retain their separate questions, exact source channels and provider bounds.
The Registry roster assertion compares exact captured formals/body bytes;
Runtime and VM inventory assertions compare the original public completion
and result windows. Pure recipe assertions provide neither a live helper
binding nor a table-object identity.
