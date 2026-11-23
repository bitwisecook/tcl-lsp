# BIG-IP language checks and release scope

Reviewed against `work` at `c0980e638a`. The appliance evidence covers
21.1.0.1 build 0.0.26 on one VE. These decisions define what the project can
claim from that evidence and what needs wider coverage.

## Q1. Oracle and release coverage

### Why this was asked

No BIG-IP runtime was available in the analysis container. The command-definition study ([registry definition errors](../registry-definition-errors.md), plan items 116 and 123–131) checked the Tcl, Tk and package definitions against real tclsh 8.4–9.1 and Tk, and found confirmed false errors and wrong release windows; the iRules and BIG-IP definitions were deliberately left out because nothing could play tclsh's role for them. The 999 iRules specs cite clouddocs pages as their only source, and the repository's `fakeCMP` and `tcl-irule-test` are local simulators. Every later question (Q5–Q8) needed to know what would count as evidence.

### The question in full

> **Q1. Oracle.** What can stand in for tclsh when checking iRules and BIG-IP definitions: a BIG-IP VE or lab system (`tmsh`, iRule compile and `test`/`fakeCMP`), F5 clouddocs scrapes (the 999 iRules specs cite clouddocs URLs), captured validation output, or customer configurations? Which BIG-IP versions must be covered, and how are per-version differences recorded?

### Coverage of the answer

Answered in full: the oracle per question kind (TMSH verify for grammar and attachment, the iRule compiler for syntax and diagnostics, TMM traffic for values, effects and delivery), the role of `fakeCMP`, clouddocs and customer configurations, the releases to pin (17.1.x, 17.5.x, 21.1.x live; 15.1.x and 16.1.x historical) and the fields recorded per row. Only 21.1.0.1 build 0.0.26 has been measured so far.

### Answer


Use a real BIG-IP VE or a representative physical lab appliance as the
execution oracle. Use TMSH verify for configuration grammar and attachment,
the iRule compiler for syntax and diagnostic text, and a real TMM traffic
test for returned values, effects and event delivery. Test tmsh-hosted Tcl,
iApp implementation and APL callbacks in their own contexts. Stock Tcl is
a control for shared language behaviour. The repository's
[`tcl-irule-test` and fakeCMP](../../../../rust/tcl-irule-test/src/lib.rs) are
local regression tools.

Record the exact TMOS version and build, platform, provisioned module,
profiles and side, execution context, fixture, warnings or errors and observed
results. Keep compiled, attached and executed verdicts distinct. Pin 17.1.x,
17.5.x and 21.1.x in the live matrix, starting with the measured 21.1.0.1.
Use archived validation transcripts, release documentation and offline
captures to cover legacy 15.1.x and 16.1.x; start or extend live coverage
there if a compatibility claim needs it. Record build observations separately
from documented introduction, deprecation and removal ranges. Unmeasured
builds remain unknown. Sanitised customer configurations are example and
regression inputs. They can supply observed facts when accompanied by
build-specific compile or execution evidence.
[F5 software support policy](https://my.f5.com/manage/s/article/K5903) ·
[TMSH execution evidence](../../../../evidence/bigip/tmsh-21.1-name-handling.md) ·
[TMM execution evidence](../../../../evidence/tcl/bigip-name-handling-tls-http.md)

## Q5. Events and profiles

### Why this was asked

The [remaining-sprawl survey](../remaining-sprawl.md) found, and this session confirmed in the source, two wrong answers: `f5-cli`'s `explain_flow` lists `CLIENTSSL_HANDSHAKE` before `CLIENTSSL_CLIENTHELLO` (`rust/f5-cli/src/commands/explain_flow.rs:56–57`) while the registry has the opposite order (`rust/tcl-registry/src/events/event_order.rs:51,63`), and MCP's `irule_gen` adds a CLIENTSSL profile for SERVERSSL events (`rust/tcl-mcp/src/irule_gen.rs:66–67,292`). Hand-written event tables also exist in the report front end (`topology.ts`), the VS Code iRule skeleton (four HTTP events against MCP's seven) and `f5-xc`'s mapping. The fix (plan items 107 and 111) needed an authoritative source.

### The question in full

> **Q5. Event and profile facts.** Which source is authoritative for event order, event-to-profile requirements and per-event command availability? Known disagreements to resolve: `f5-cli explain_flow` orders `CLIENTSSL_HANDSHAKE` before `CLIENTSSL_CLIENTHELLO`; MCP `irule_gen` gives SERVERSSL events a CLIENTSSL profile; the report generator, VS Code skeleton and `f5-xc` keep their own tables.

### Coverage of the answer

Answered for TLS event order on both sides (client: `CLIENTSSL_CLIENTHELLO` then `CLIENTSSL_HANDSHAKE`; server: `SERVERSSL_CLIENTHELLO_SEND`, `SERVERSSL_SERVERHELLO`, `SERVERSSL_HANDSHAKE`), event spelling (`SERVERSSL_CLIENTHELLO` is not an event), profile eligibility including the PERSIST alternative, and command eligibility severity (a warning for `HTTP::header` in `RULE_INIT`). The registry already holds this order and these profile alternatives; the faults are in the consumers. Only one sample of per-event command availability was measured; the full per-event table belongs to the Q6 audit.

### Answer


Keep one registry event relation keyed by flow, profile alternatives and
side. Use live, ordered, per-flow traces for delivery and relative order,
and separate attachment checks for profile eligibility. Use versioned F5
manuals for documented event meaning and valid command events. Command
restrictions may apply to an invocation form or subcommand within an event.

| Verified result | Consumer consequence |
| --- | --- |
| `CLIENTSSL_CLIENTHELLO` preceded `CLIENTSSL_HANDSHAKE` on both TLS 1.2 requests. | Fix the reversed order in [CLI explain flow](../../../../rust/f5-cli/src/commands/explain_flow.rs). Use the registry's order for the matching flow. |
| The proxied flow fired `SERVERSSL_CLIENTHELLO_SEND`, then `SERVERSSL_SERVERHELLO`, then `SERVERSSL_HANDSHAKE` on its TLS 1.3 backend leg. TMSH rejects the spelling `SERVERSSL_CLIENTHELLO`. | Use the measured spelling and server-side order. Leave optional events conditional. |
| TMSH rejected attaching ClientHello without CLIENTSSL/PERSIST, and rejected server handshake with CLIENTSSL alone. The corresponding SSL profile, or either tested persistence profile, allowed attachment. | Have [MCP scaffolding](../../../../rust/tcl-mcp/src/irule_gen.rs) add Client SSL or Server SSL on the proper side to cause the requested TLS flow. Keep PERSIST as an attachment alternative. |
| TMSH warned on a literal `HTTP::header` in `RULE_INIT` but returned success. The same command ran under a configured HTTP event. | Store severity as well as eligibility. Keep compiler acceptance and runtime delivery separate. |

Publish the shared event and profile facts to MCP, the report front end and
the VS Code skeleton. Give `f5-xc` the same TMOS vocabulary and keep its
XC mapping local. A single flow trace supports partial order, including the
measured intra-leg constraints. It cannot order conditional branches and both
legs of every flow. Priority controls callbacks within the same event.
[TMSH attachment and compile results](../../../../evidence/bigip/tmsh-21.1-name-handling.md#ssl-rule-attachment) ·
[TMM trace](../../../../evidence/tcl/bigip-name-handling-tls-http.md) ·
[F5 event catalogue](https://clouddocs.f5.com/api/irules/Events.html)

## Q6. Audit of the 1,018 iRules specs

### Why this was asked

The command-definition harness found Tcl and Tk definition errors only because real interpreters could be probed with every argument count ([registry definition errors](../registry-definition-errors.md)). The 1,018 iRules spec files (about 63,000 lines) were out of scope there by the user's choice, and the analyser's iRules checks (arity, subcommands, per-event availability, `disabled_in_irules`) rest on those specs. Before applying the same audit (the iRules counterpart of plan items 123–131), the method and the admissible evidence had to be fixed.

### The question in full

> **Q6. iRules command definitions.** How should the 1,018 iRules spec files be checked: arity, subcommands, per-event availability, per-version availability, `disabled_in_irules` Tcl commands, deprecated commands, and return types? What counts as evidence when no runtime is available here?

### Coverage of the answer

Answered as a method (versioned rows per command and form; presence, compile and TMM checks; distinct disabled, deprecated, removed and unmeasured states) with a first sample: `exec` and `open` disabled, `clock seconds` accepted, deprecated `matchclass` still working, and two `HTTP::header` forms that the analyser's subcommand model would misread (an unknown first word and an extra argument both write a header). The audit of the 1,018 specs has not been run.

### Answer


Use [the registry oracle audit](../../../../rust/xtask/src/registry_oracle.rs)
and the [appliance probe tools](../../../../scripts/dev/bigip-probes/open-questions)
to inventory the surfaces first. Add a versioned row per command and form,
with the argument-count bounds, supported subcommands and implicit forms,
event, profile/module, output shape and effect. Organise the checks as follows.

1. Compare presence, spelling, lifecycle and documented signature against
   versioned TMSH command help, manuals and appliance command inventories. Use
   explicit disabled Tcl controls as well as missing-command controls. Record
   deprecated-but-present separately from removed.
2. Compile minimal accepted calls and negative controls for each form, plus
   boundary argument counts and near-miss subcommands. Preserve warnings and
   verify the rule can attach under the expected profiles.
3. Exercise representative argument forms and all disputed or effect-bearing
   forms in TMM with real events. Check exact results and errors, return shape
   and relevant state reads or writes. Keep module-gated rows observationally
   unknown until the module and event can be exercised.

The sample above checked only a few forms. The compiler rejected `exec` and
`open` as disabled and accepted `clock seconds`. The deprecated `matchclass`
still resolved and returned the matching index at runtime. TMM treated
`HTTP::header nhq_no_such_subcommand Host` as a write, and
`HTTP::header value Host Extra` also wrote to the named header. Both returned
empty results. A checker that assumes every first word is a closed
subcommand or derives runtime arity from compile exit status would miss those
forms. For return shapes distinguish a Tcl value's documented meaning and
the observed output from any unmeasured internal representation.

Without an appliance use versioned explicit F5 contracts and existing
build-specific transcripts, with source links and confidence at the field
level. Keep third-party examples, scrapes and simulator outcomes as leads or
local parity checks. Treat a docs disagreement as unresolved until adjudicated.
[TMSH compiler results](../../../../evidence/bigip/tmsh-21.1-name-handling.md#rule-compiler) ·
[TMM runtime results](../../../../evidence/tcl/bigip-name-handling-tls-http.md) ·
[F5 HTTP::header reference](https://clouddocs.f5.com/api/irules/HTTP__header.html) ·
[F5 matchclass reference](https://clouddocs.f5.com/api/irules/matchclass.html)

## Q7. Configuration value grammars

### Why this was asked

The [remaining-sprawl survey](../remaining-sprawl.md) found the typed parser `value::monitor_expression::try_parse` used only by `validator.rs:863`, while `lint.rs:291,298` (the `orphan-monitor` rule), `convert.rs:282` and `irule_context.rs:604` take the first word of a pool's monitor (`monitor.split(' ').next()`) and `bigip-report-gen/rust/src/graph.rs:417` uses its own regex splitter. Observed consequence: for `monitor /Common/a and /Common/b` only `a` counts as in use, so `b` is falsely reported orphaned, and `min 1 of {…}` looks up a monitor named `min`. Other configuration values (persistence, iRule priority, pool members, addresses, route domains) were also split as words.

### The question in full

> **Q7. Monitor and other value grammars.** Is `value::monitor_expression` complete for every monitor rule form (`and`, `min N of {…}`, quoted names, partitions), and which other configuration value grammars (persistence, iRule priority, pool member and address forms, route domains) need typed parsers instead of word splitting?

### Coverage of the answer

Answered for LTM pool monitor rules: the accepted grammar (`none`, `default`, single, `and` chain, `min N of {…}` with 1 ≤ N ≤ list length, quoted tokens and a quoted whole expression) and five parser faults in `MonitorExpression::try_parse` (quotes kept, `and` inside a `min` list accepted, thresholds 0 and over-length accepted, a trailing `and` accepted). Priority is bounded 0–1000; persistence with `fallback-persistence` was verified. Not measured: GTM and other monitor families, canonical rendering of quoted expressions, and the other value grammars beyond those named.

### Answer


The LTM pool grammar includes `none`, `default`, a single reference,
an `and` chain and `min N of { … }` with a list of monitor references.
The loader accepted individually quoted names and a quoted full expression.
It rejected a monitor definition whose actual name contained a space.
`and` in the measured minimum list was looked up as a monitor name. The
threshold must be positive and at most the number of references in the list.

A small Rust driver invoked the checked-out
[`MonitorExpression::try_parse`](../../../../rust/tcl-bigip/src/value/monitor_expression.rs)
directly with the same strings. The valid unquoted conjunction and minimum
list returned both references. The quoted inputs returned names retaining
quote characters, and the bad minimum cases returned ASTs:

| Rust input | Returned minimum and monitor names |
| --- | --- |
| `"/Common/http" and "/Common/https"` | No minimum, both names retain quotes |
| `"/Common/http and /Common/https"` | No minimum, first name has a leading quote and second has a trailing quote |
| `min 1 of { /Common/http and /Common/https }` | `1`, `/Common/http`, `and`, `/Common/https` |
| `min 0 of { /Common/http /Common/https }` | `0`, both monitor names |
| `min 3 of { /Common/http /Common/https }` | `3`, both monitor names |
| `/Common/http and` | No minimum, single monitor. TMSH reported an incomplete command. |

Use the shared parser as the starting point and verify spans, quote
decoding and complete consumption. Check threshold and lookup validity
separately from lexical parsing, preserve original text for edits, and scope
the result to LTM until other monitor families are measured. Use typed
persistence/attachment values including the default and fallback, typed pool
member and destination forms, address and route-domain types, and the
shared iRule handler reader for priority. Reuse the existing types and
resolve names in their owning device and partition.
[TMSH grammar checks](../../../../evidence/bigip/tmsh-21.1-name-handling.md#monitor-rules)

## Q8. Partition paths, addresses and transport encoding

### Why this was asked

The [remaining-sprawl survey](../remaining-sprawl.md) found partition and path rules implemented several times with different outputs: `tcl-bigip/src/object_names.rs:27` and `stats.rs:88` render `"/P/"` or `"(no partition)"`, while `bigip-report-gen/rust/src/model.rs:209` and `apm.rs:159` return `"P"` or `""`, and `profile_types_for_virtual` is duplicated. The registry review counted 219 BIG-IP vocabulary comparisons in `tcl-bigip`. Addresses raise the same question harder: IPv4 and IPv6 literals, route-domain suffixes (`%N`), the tmsh dot service separator for IPv6 (`2001:db8::1.80`) against URL forms (`[2001:db8::1]:80`), wildcard and `any6` forms, masks, FQDN nodes, and the REST `~Common~name` encoding all appear in configuration, iRules strings and tool output. The user also recalled an early BIG-IP context that took a colon port on an IPv6 address (`fe80::1:80`); the reading is determined by whether the property takes a port. A study of every parser against every form ([BIG-IP names and addresses](../bigip-addresses-design.md)) was started alongside these answers.

### The question in full

> **Q8. Names and partitions.** One rule for `/Partition/folder/name`, `~Common~name`, route-domain suffixes `%N`, IPv6 forms and the `(no partition)` rendering: which crate owns it, and which consumers must move to it?

### Coverage of the answer

Answered as ownership (`tcl-bigip::object_names` for paths, `tcl-bigip::value` for typed addresses and services, iControl REST for `~` encoding, an optional partition rendered as `(no partition)`) with these measured forms: qualified node names with a colon service for IPv4- and IPv6-backed nodes; an IPv6 literal with a route domain and a dot service; the inverse forms rejected; route-domain existence validated; `~Common~name` rejected by TMSH. Not yet measured on an appliance: IPv4 literals with a route domain and port as members, masks and prefixes, wildcard forms (`0.0.0.0:0`, `any`, `any6.any`, `*:*`), service names, FQDN nodes and members, virtual destinations, self-IPs, routes, monitor destinations, GTM objects, iApp `.app` folders, names with special characters, and the user-reported legacy colon-port IPv6 form (release unknown). Our parsers' treatment of all of these is in the [BIG-IP names and addresses](../bigip-addresses-design.md) study.

### Answer


`tcl-bigip::object_names` owns textual object identity and scoped lookup.
Carry a partition, folder components and leaf for qualified identifiers, and
preserve relative and genuinely unpartitioned input when resolving.
Use `tcl-bigip::value` for typed addresses, service suffixes and route-domain
IDs. Preserve original spelling alongside typed values. The lab accepted
colon service for a named IPv6-backed node and dot service for the literal
IPv6 address. It rejected the inverse forms measured here, and validated
route-domain existence separately. Keep the REST tilde encoder at the
iControl boundary consuming the shared path components. Represent a missing
partition as an optional value and render `(no partition)` in summaries.

Migrate the independent path helpers in tcl-bigip stats and conversion,
f5-cli split and remote object I/O, report model/APM/graph projections,
query path and filter code and the MCP and XC adapters that need those
facts. Existing query object projection already uses
`object_path_partition`. Paths from archives, redaction and edits should
preserve the original parts needed by their callers.
[TMSH name and address checks](../../../../evidence/bigip/tmsh-21.1-name-handling.md#names-route-domains-and-services) ·
[Shared name resolver](../../../../rust/tcl-bigip/src/object_names.rs) ·
[F5 REST identifier format](https://clouddocs.f5.com/training/community/firewall/html/archive/archive1/lab6/step5.html)

## Q9. Taint and credentials

### Why this was asked

The [registry review](../registry-review.md) found taint and credential facts stated more than once in one spec: `HTTP::header` declares spec-level `taint_output_sink_subcommands: ["insert", "replace"]` restating the subcommand-level `credential_arg`, duplicates its `sensitive_headers` list, repeats `STATE_DECIDED` on every subcommand, and marks mutators only with `mutator: true` and no side effect. Plan item 120 (one statement per fact) needed to know which level owns these facts.

### The question in full

> **Q9. Taint and security facts.** Where should iRules taint sources and sinks, sensitive headers and credential arguments live (spec subcommand fields, spec-level lists or a separate table), given they are duplicated today?

### Coverage of the answer

Answered: taint, sanitiser, sink and credential roles live on the resolved invocation form in the registry, with a shared case-insensitive sensitive-header classification and `security_floor` enforcement. The measured `HTTP::header` writes show that the form must be resolved before taint is applied; the rest of the `HTTP::*` family is still to be checked before extending the fact.

### Answer


Keep source, sanitiser, sink and credential argument roles in the typed
registry descriptor for the form that performs the operation: subcommand,
option or implicit command form. Use a command-level default for genuinely
uniform behaviour. Make the existing invocation resolver the shared way to
obtain the argument positions and action effects for the analyser, CLI/MCP,
and SpecTcl/editor projections. Share a small registry-owned classification
for case-insensitive sensitive HTTP header names. A form can reference that
classification and add a scoped policy override, with the hardening rules
enforced by `security_floor`. The same literal list appears in
`HTTP::header` and `HTTP::cookie`. Check the latter forms against their
actual cookie operands before assigning them the HTTP-header policy.

The live control read a synthetic Authorization value, counted it and
observed two different implicit mutating call forms. It establishes why
the form resolution has to precede taint and credential checks. Classifying
the value as a secret and deciding diagnostic severity are policy decisions.
The checked-out `HTTP::header` spec lists only `insert` and `replace` as
header output sink subcommands. The measured mutation needs a descriptor
that makes the selected header name and value available to the same sink
logic; verify the rest of the family before extending the fact globally.
[TMM header evidence](../../../../evidence/tcl/bigip-name-handling-tls-http.md) ·
[Current header spec](../../../../rust/tcl-registry/src/commands/irules/http__header.rs) ·
[Resolved credential arguments](../../../../rust/tcl-registry/src/resolved_invocation/source_options.rs)

## Q10. Release scope

### Why this was asked

The [architecture review](../architecture-review.md) recommended releasing the LSP from its current scope and leaving the restructuring (engine convergence, crate split, caching, licence) until after (plan item 96). The BIG-IP findings include both small wrong answers (the orphan-monitor false positive, the SSL event order and profile mapping, plan item 107) and large data work (the schema crate, generators and audit, items 111, 118, 119), so the release boundary had to be drawn for this domain too.

### The question in full

> **Q10. Scope against the release.** Which BIG-IP and iRules fixes belong in the current LSP release (wrong answers such as the orphan-monitor false positive and the SSL event order), and which wait for the data crate and generators?

### Coverage of the answer

Answered: item 107's bounded fixes ship in the current release with focused regressions (compound monitors, SSL order, side-specific profiles, confirmed name outputs), individual descriptor fixes only with a focused test (the `HTTP::header` forms are a candidate), the 21.1 SSL defaults only with a versioned row; the data crate, generators, full migration and catalogue audit stay in items 111, 118, 119 and the definition audit.

### Answer


- Do item 107's bounded fixes in the current LSP release after focused
  regressions. Resolve every member of accepted monitor expressions in
  `orphan-monitor`, iRule context and conversion. In a target that cannot
  express a compound monitor rule, report the loss instead of silently
  selecting the first monitor. Fix the measured TLS order and MCP's server-side
  profile inference. Use the existing typed name/partition owners for confirmed
  bad outputs where a focused regression covers the change.
- Include corrections to an individual iRules descriptor only where a
  focused test establishes the affected form and downstream effect. The
  `HTTP::header` observation is a candidate for such a fix. Give the 21.1 SSL
  defaults a versioned evidence row before extending the existing fallback.
- Keep the data-crate extraction, full graph and consumer migration,
  historical snapshot/generator recovery and catalogue-wide audit in
  items 111, 118 and 119 and the subsequent definition audit. The release
  fixes can use the existing owners. Carry unmeasured releases and modules
  into the matrix instead of treating the current VE as blanket coverage.
- Require target-level regressions for the bounded fixes and the normal PR
  gates before release. This evidence pass changes the plan and evidence
  only. It does not mark the implementation or release gate complete.
[Measured configuration cases](../../../../evidence/bigip/tmsh-21.1-name-handling.md) ·
[Measured TLS and header cases](../../../../evidence/tcl/bigip-name-handling-tls-http.md) ·
[Source review](../remaining-sprawl.md)
