# Procedure body activation

Procedure definitions retain formal and body objects, the actual
namespace token, persistent static storage, and native header registration.
Defining a procedure validates its declaration without parsing or compiling
its body. `info body` reads the retained declaration body. C Tcl captures a
shared body as a new plain string, preserving the complete original bytes and
source location. An unshared C body and every Jim body retain their original
object. The definition owner inspects sharing before acquiring another handle.
Only the native bare-`args` header check reaches an otherwise unshared body
string updater; other definitions preserve pure List and ByteArray backing.

`ProcDef.body: Option<CompiledUnit>` distinguishes original source from an
admitted C body cache. An unprepared definition has `None`; it does not carry
empty assembly or compiler provenance. Host definitions, `apply`, ordinary
`proc`, and scripted library procedures use this same representation.

`PreparedProcedureActivation` carries the selected declaration and its actual
execution unit. Callers enter that unit through the shared activation owner;
they do not prepare a body while publishing a definition. A namespace spelling
cannot replace the retained namespace token.

| Engine | Activation ordering | Body ownership |
| --- | --- | --- |
| C Tcl 8.4 | Compile the body; a parse failure precedes formal binding. | Successful compilation is cached; failed body compilation is not published. |
| C Tcl 8.5–9.1 | Compile the body, bind formals, then execute. A retained syntax failure is reached by body execution. | The cache retains actual compiler, namespace, interpreter, profile, and invalidation receipts. |
| Jim 0.84 | Validate arity, handle an empty body, create the frame and bind formals, then execute the original Script. | The original body owns its Script representation. Each call has a separate execution lease; the declaration does not cache that lease. |

The registry's `NativeProcedureActivationProtocol` selects these rules from
the actual execution engine. Authored grammar and assistance profiles do not
grant body compilation or Script authority. Unknown engine capabilities remain
host refusals.

C cache reuse checks the current compile service, interpreter identity,
retained namespace, native cache stamp, profile, and command dependencies.
Active step tracing selects the appropriate body compilation policy. The
refreshed declaration is published only in its still-live visible or hidden
command domain. A deleted command handle does not republish its name.

Implementers adding a procedure adapter must retain original source and use
the activation owner. A Jim activation retains the existing `Rc<ProcDef>`;
copying the declaration would add an extra owner of its body object. Compiled
local layout installation requires a real layout receipt. A native Script
activation supplies no compiled-local layout.

Native procedure references are owned by
`NativeProcedureRoleLedger`, `NativeProcedureReference`, and
`NativeProcedureBinding` in `tcl-runtime-api`. A command binding owns one
native declaration reference. Cloning a binding transports that same role;
it does not acquire another. `NativeProcedureBinding::replace_from` publishes
new client data into the same binding before releasing the old declaration.
Command queries borrow a declaration without acquiring a native role.

C procedure frames acquire a `NativeProcedureReference` only after successful
body preparation and formal binding. They release it after raw body completion,
before variable and command teardown. A C8.4 local-variable-name primary owns
its own actual procedure reference; its duplicate hook acquires another.
Jim frames retain the original parameter, body, and namespace objects through
Jim's separate activation rules.

Both backends separate genuine native resource owners from memory-safety
transports. The Runtime's `ProcedureObject` retains an allocation without
incrementing its native object reference count; the VM uses native lifetime
leases. The final native procedure release drains body/default references,
static storage, local-name tables, and namespace objects. Resource cells are
taken before native free hooks run. A surviving query cannot acquire a new
role on a retired declaration. An independently retained object can remain
live after its procedure owner retires.

Checked Runtime native getters reject retired allocations before reading or
updating their headers. Procedure entry and body/default queries validate both
the declaration and its retained object views. C interpreter shutdown withdraws
the actual visible and hidden procedure binding roles even when query handles
keep their metadata and allocation views alive.

Renaming a procedure moves the same declaration. Future calls use its current
location; an already entered frame keeps its selected namespace. Jim's
counted-byte relocation recipe keeps the body namespace for an unqualified
destination and selects the original qualifier when one is present. Namespace
command copies create distinct declarations with their own native references.

The fixed body-capture fixture in
`rust/tcl-vm/tests/data/native_procedure_body_capture/` records original and
stored body identity, reference counts, type, and string presence for shared
and unshared String, List, ByteArray, malformed, and counted-NUL bodies.
Jim has no ByteArray constructor in this fixture.

The fixed activation conformance fixture in
`rust/tcl-vm/tests/data/native_procedure_activation/` records definition,
wrong-arity, malformed-body, and body-side-effect observations from all five
C releases and Jim. The tests compare code and result bytes and verify that
definitions remain unprepared. Full completion metadata requires its own
captured native observer controls.

`rust/tcl-vm/tests/data/native_procedure_parse_context/` supplies original-object
C8.4 parse-failure controls with space-bearing, opaque, NUL, and clipped native
UTF command names. The activation adds compiler contexts before formal binding;
the ordinary object-vector entry adds the invocation frame. The shared naming
owner selects the name extent and the native UTF owner selects byte excerpts.
