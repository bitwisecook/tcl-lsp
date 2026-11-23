# BIG-IP language checks and release scope

Reviewed against `work` at `c0980e638a`. The appliance evidence covers
21.1.0.1 build 0.0.26 on one VE. These decisions define what the project can
claim from that evidence and what needs wider coverage.

## Q1. Oracle and release coverage

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
