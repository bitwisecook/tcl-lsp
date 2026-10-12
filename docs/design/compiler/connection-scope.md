# Connection scope — cross-event variable flow (iRules)

The connection-scope analysis resolves variable cells before comparing event
handlers. A spelling does not establish shared storage: bare `config` in
`RULE_INIT` names a global namespace cell, while bare `config` in a traffic
handler names a connection-frame cell. Absolute `::static::config` and the
relative root spelling `static::config` name the same worker-local cell when
namespace resolution proves that target.

## Owners

- `tcl-registry::events` declares variable frames and typed lifecycle relations.
- `tcl-registry::f5::storage` declares the TMM namespace availability and storage
  overlay. Initialisation has no virtual-server/connection context. The static
namespace is shared across rules within each worker; runtime writes do not
propagate to other workers.
- `var_resolve`, `variable_bindings` and `place_bridge` resolve point-specific
  cells, aliases and observable reads. Source-text searches never manufacture
  `info exists` reads.
- `connection_scope` projects those facts into per-handler summaries. Individual
  handlers remain distinct; an event-name union cannot prove execution order.

## Certainty

`EventVarSummary::cell_defs` contains possible definitions. `must_defs` contains
cells established by proven non-observed constant assignments on every normal
CFG exit, after destructive writes. Generic calls and callbacks invalidate this
proof; conditional or partial writers remain possible definitions. Exception
edges conservatively retain the state before a block could have partially
executed. A successful earlier event still is not proof that the event ran:
optional lifecycle phases, disabled handlers and repeated requests matter.

The compatibility `cross_event_defs` and `cross_event_imports` sets describe
possible observable flow for liveness and conditional diagnostic suppression.
They are not definite initialisation or constant-value facts. Optimisers must
not seed values or fold existence from those sets.

The registry's lifecycle relation distinguishes initialisation before traffic,
same-event handlers, possible order, a reader earlier than its writer, and
unknown order. Missing event metadata is unknown rather than valid flow.
Priority-sensitive same-event transfer requires explicit handler ordering.

## Runtime counterpart

The test framework retains a real coroutine activation for a connection.
Traffic scripts execute in that activation, preserving arrays, aliases and
traces without copying values. `RULE_INIT` executes in the interpreter's root
frame. The host provides private coroutine control commands while event code
retains the measured F5 language surface. A host lacking coroutine support
reports that persistent frames are unsupported.

Each simulated CMP worker owns a real child interpreter. `RULE_INIT` executes
in each worker's root namespace; ordinary globals and runtime-defined procedures
remain per worker. `LiveSession` explicitly installs the authored
`RuleInitPublication` policy: reached initialisation writes to resolved static
cells publish to enrolled actual namespace incarnations. Aliases are resolved
before classification, recipient traces use normal mutation ordering, and late
enrollment replays the journal's original Values. Event and traffic updates
outside that policy remain per worker. This authored simulator contract does
not establish identical appliance initialisation values or a shared physical
Tcl cell across TMMs. Arrays, aliases, traces and user command tables survive
worker switches. Shared commands such as `table` and data groups use the
orchestrator's mock state; switching itself reads, deletes or recreates no
user cells.

The simulator validates the implementation against these contracts; it is not
independent evidence of F5 semantics. Public [static namespace documentation](https://clouddocs.f5.com/api/irules/static.html),
[RULE_INIT documentation](https://clouddocs.f5.com/api/irules/RULE_INIT.html),
[CMP compatibility guidance](https://clouddocs.f5.com/api/irules/CMPCompatibility.html)
and recorded appliance transcripts own the expected runtime behaviour. The
`AES::key` example documents independent per-TMM initialisation, so an analyser
must not assume identical initial values across workers.

## Validation

Connection-scope tests cover root initialisation versus connection locals,
absolute/relative static identities, conditional assignments and inert
`info exists` text. The live-session frame test covers helper `upvar`, persistent
same-frame aliases and arrays, exact read-trace counts and connection reset.

The worker-interpreter test verifies zero read-trace callbacks during worker
switches, isolated runtime procedure definitions, retained connection aliases and
per-worker initialisation globals. Stock C Tcl can validate these Tcl mechanics;
TMM-specific build facts remain separately attributed F5 evidence.
