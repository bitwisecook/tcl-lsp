# Native variable access

Variable names and array keys are `NameBytes`. A binding contains an exact key
and its physical frame or namespace owner. The `VarArena` owns the cell identified
by `VarId`; deleting a binding never makes that identifier refer to another cell.

## Operand materialization

Use `Vm::native_name_operand_bytes` on the original `Value` before selecting a
name purpose. The VM authenticates `NamePolicyProtocol` independently of numeric,
expression, source, and command-availability policies. A logical name provider
requires explicit installation and retains its authored-simulation authority.

An existing resident string is authoritative. A pure C Tcl byte array uses the
shared native string materializer and caches the selected representation on the
same object while retaining its binary storage. Jim's unsupported pure byte-array
materialization produces a host refusal. Do not decode, repair, or escape bytes
to make an operand acceptable to a name consumer.

## Input forms and ownership

Keep these forms separate:

| Input | VM access | Meaning |
| --- | --- | --- |
| One complete name operand | `get_var_bytes`, `set_var_bytes`, `unset_one_bytes` | The selected native combined-name parser determines scalar or array access. |
| Separate root and element operands | `get_elem_bytes`, `set_elem_bytes`, `read_elem_traced_bytes`, `store_elem_result_bytes` | Each part keeps its native protocol; the root is not reparsed as a combined name. |
| A selected caller frame | `get_var_from_bytes` | Resolution starts from that actual activation. |
| A selected procedure formal | `set_local_bytes` | The already-selected key is bound directly in the current activation. |
| A captured array cell | `ArrayTarget`, `array_key_bytes_checked_at`, `array_read_elem_bytes_at` | Enumeration retains the original cell across callbacks. |

The `VarStore` byte methods provide checked defaults for adapters whose storage
requires Unicode. The VM overrides them with exact byte resolution. Mutation
methods preserve the trait's storage-oriented contract; callers needing a Tcl
completion use the VM's completion-bearing accessors.

`NativeNameProjection::qualification` is authoritative. C Tcl can retain a full
unqualified scalar name containing raw NUL while deciding qualification from the
prefix before that NUL. Scanning the selected bytes again can incorrectly turn a
later `::` into namespace syntax.

Jim's global variable table uses its flat byte-key protocol. Use
`jim_global_variable_key_bytes` with the actual namespace object spelling. Do not
run that key through C Tcl namespace segmentation.

## Links, callbacks, and reporting

C Tcl links retain cells. Jim's selected-frame name links retain their binding
owner and reselect its value at access. Preserve that distinction when changing
lookup code. Namespace deletion and recreation must preserve namespace
incarnations, and a detached C array-element alias must retain its old cell.

A callback can unset a variable, recreate its spelling, or retarget an alias.
Reads and write read-back retain the reached `VarId` across callbacks. Array
operations retain the array and element cells and release their operation
references on every completion path. Keep trace pins, link references, constants,
and undefined-shell cleanup in the owning storage routines.

Trace registration/query, ordinary variable access, and diagnostic reporting use
separate name projections. Registration can select a CString receiver even when
ordinary scalar access retains a longer name. Trace prefixes remain original
`Value` objects. Callback names retain the original operation's part boundaries
and the actual followed element key; they are not reconstructed from a variable's
canonical name.

`ValueOps::native_string_bytes` performs checked native string materialisation
before a consumer selects its name purpose. Its default is suitable only for
adapters with authoritative resident strings; adapters with pure binary storage
supply their own selected native recipe. `Traces::fire_bytes` returns operational
host refusal in the outer result and guest callback failure in the inner result.

`read_variable_result_bytes` produces missing diagnostics while retaining the
original receiver. Pass the actual `NativeVariableFailureSite` to the shared
diagnostic owner: binding lookup, reached read, reached write, or unset. The
owner selects diagnostic bytes, effective reason and the complete native error
code independently of the selected storage key.

Captured updates return `VariableUpdateResult`, carrying both the stored value
and read metadata retained on success. Command handlers and bytecode opcodes
must apply both fields. A failed native read can initialise a successful update
while leaving error options visible to `catch`.

Guest diagnostics use the shared native diagnostic owner. Unsupported reporting
and byte-source compiler ingress produce an explicit host refusal. The selected
storage key, original operand, diagnostic name, and callback name are distinct
data and must remain distinct.

## Enumeration and changes

Use byte enumeration for frames, namespaces, constants, and array elements.
Unicode convenience views check the complete result; they never omit opaque
rows or substitute replacement characters. `ArrayTarget::name_bytes` exposes an
exact operand. `ArrayTarget::name` requires a valid UTF-8 name.

For changes to name handling, select the actual native protocol first, record
the original input form, resolve its owner once, and pass constructed keys and
retained cells to consumers. Add differential controls for raw resident strings,
modified UTF encodings, pure byte arrays, qualification before and after NUL,
combined and separate element input, and callback-driven deletion or retargeting.
Compare exact result bytes, completion options, error metadata, enumerated keys,
and the physical variable affected. An unsupported native operation or host
refusal is part of the observation and must not be counted as an agreement.

## Original objects and native consumers

`ValueOps::native_object_snapshot` inspects primary storage without conversion.
Resident bytes do not establish a native String descriptor, a Unicode cache,
object identity, or the canonical empty allocation. `same_object` supplies the
actual identity comparison; an adapter without that capability abstains.

`full_native_equality` owns full case-sensitive object equality. C8.6+ retains
its identity shortcut, binary backing eligibility, String count and Unicode
preparation, and empty-object enquiry. Jim reaches the original string and
count getters in order even for the same object. Trap matching passes original
list members to this owner; it does not compare decoded host strings.

Prefix tables retain original members and return the selected member. Their
native owner keeps table lookup, prefix character comparison and diagnostic
extents separate. C8.6 retires the key's internal representation after a
nonidentity temporary-table lookup; C9 preserves it. Error options are validated
at the selected native frontier and retain their original key/value handles.

Deferred scripts use `prepare_script_commands_value`: materialize the original
object once, retain a native `SourceImage`, capture the actual compiler entry and
namespace token, and compile its complete byte-command prefix. A script's
`SourceImage` retains its input channel and original offsets. Unicode document
views do not replace native value source.

Try handlers retain original selectors, binding names and script objects. C
validates clauses before body evaluation. Jim validates them after the body and
retains the body's captured error-code object separately from live private
return state. Jim options observe that private return code, level and stack
storage at their actual binding or completion boundary. A normalized completion
from an ordinary command is not a substitute for those private fields.

## Namespace-variable queries

`Namespaces::namespace_variable_name_bytes_checked` owns the namespace query
boundary. C Tcl's `Tcl_FindNamespaceVar` uses its own CString projection and
probes namespace tables, including undefined cells and aliases. Reporting uses
the selected raw cell's name; following an alias would report the wrong name.
C Tcl 8.x can search the global namespace after the current namespace; C Tcl 9
uses the current namespace alone for a relative query. Procedure locals remain
outside this query.

Jim's `namespace which -variable` helper performs canonicalisation and returns
a qualified spelling even when storage is absent. Consumers must preserve that
textual operation instead of donating C Tcl's cell-existence test. Jim namespace
upvar also qualifies its target without requiring the namespace to exist.

Dictionary-valued variable roots use the authentic native object-list and key
materialisation doors. Lexical grammar is not a native storage issuer. Checked
operations propagate missing storage capabilities outside guest completions.
