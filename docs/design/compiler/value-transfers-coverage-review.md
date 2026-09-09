# Review supplement: every diagnostic and optimisation code

This ledger turns the [architectural review](value-transfers-review.md) into
a code-by-code migration contract. Baseline:
`b0f0b432b1d31ba2e53c7e85dfda389d8d5d836a`. It covers all **228** entries in
[`DiagCode`](../../../rust/tcl-core-types/src/diag_code.rs): 169 public
diagnostics, 23 internal diagnostics, five reserved diagnostics and 31
optimisation codes. It additionally covers the **13 XC translation codes**
and **18 BPF frontend codes** outside that enum, plus **16 numbered TLS
estimate/report codes**: **275 numbered entries**. Sixteen certificate-chain
finding kinds, open-ended policy/scanner code families and five uncoded BPF
verifier failure variants are accounted for separately.
The proposed O131 is not allocated at this baseline and is not counted.

This is a source-backed design coverage audit, **not** a claim to have
executed every diagnostic or proved every existing optimisation sound.
Source owners and representative construction paths were inspected; rows
identify required evidence, ownership and adversarial acceptance obligations.
The generated [diagnostic catalogue](../../generated/diagnostic_codes.md)
and [optimisation catalogue](../../generated/optimisation_codes.md) supply
the public meanings. The existing
[`diag-emission-check`](../../../rust/xtask/src/diag_emission.rs) is a useful
producer-presence heuristic, not a substitute for these semantic contracts.

## How to read the ledger

The second column identifies current producer families using the linked
source key below. It does not claim that every cited module directly
constructs the final diagnostic; some produce facts that a compiler check
or analyser projection turns into a finding. The third column states the
fact needed, not a prescription to move rule policy into the registry.
The last column is a **required design/migration check**, not necessarily
an observed current defect. Specific design findings and executed
counterexamples are distinguished in the main review as R1–R16.

All rows inherit these requirements:

- Source revision, resolved binding, dialect/version/semantic profile and
  relevant pack/world epochs accompany evidence and cached answers.
- Loaded workspace-authored semantic facts are authoritative, including
  for narrowing, reachability and code elimination, as the owner confirmed.
  No extra trust opt-in, advisory-only tier or widen-only cap applies.
  Authors own false semantics; structural API validation, binding validity
  and evaluator execution limits remain independent requirements.
- Unknown, unavailable, not-yet-computed, analysis-budget decline and a
  proved language error are different states. Absence of proof is not a
  finding that the program is necessarily wrong.
- Disabling, suppressing or changing the severity of a diagnostic does not
  alter semantic facts, legal-program acceptance or optimiser proofs.
- Rules choose codes/messages/severity and consume facts; presentation
  rebases/maps spans and filters findings. The registry owns command
  meaning, not a copy of LSP publication policy.
- An optimisation candidate needs source-edit validity and behavioural
  equivalence, including errors, effects, binding, evaluation order,
  implicit results and target semantics. A diagnostic is not that proof.

The [fact-interface and authoring supplement](value-transfers-authoring-review.md)
defines the reusable contracts behind these rows, including how existing
dynamic return-type hooks join SSA, SCCP and other fact domains.

### Source key

| Key | Current owner / representative source |
|---|---|
| VALID | [analyser/diagnostics/validity.rs](../../../rust/tcl-compiler/src/analyser/diagnostics/validity.rs) |
| USAGE | [analyser/diagnostics/usage.rs](../../../rust/tcl-compiler/src/analyser/diagnostics/usage.rs) |
| SECURITY | [analyser/diagnostics/security.rs](../../../rust/tcl-compiler/src/analyser/diagnostics/security.rs) |
| VERSION | [analyser/diagnostics/version_gate.rs](../../../rust/tcl-compiler/src/analyser/diagnostics/version_gate.rs) |
| FLOW | [analyser/diagnostics/dataflow.rs](../../../rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs) |
| RESOLVE | [analyser/diagnostics/unresolved.rs](../../../rust/tcl-compiler/src/analyser/diagnostics/unresolved.rs) |
| VARCMD | [analyser/diagnostics/var_command.rs](../../../rust/tcl-compiler/src/analyser/diagnostics/var_command.rs) |
| SYNTAX | [syntax_checks.rs](../../../rust/tcl-compiler/src/analyser/syntax_checks.rs), [recovery.rs](../../../rust/tcl-compiler/src/analyser/recovery.rs), [state.rs](../../../rust/tcl-compiler/src/analyser/state.rs) |
| WALK | [commands.rs](../../../rust/tcl-compiler/src/analyser/commands.rs), [handlers.rs](../../../rust/tcl-compiler/src/analyser/handlers.rs), [scope.rs](../../../rust/tcl-compiler/src/analyser/scope.rs) |
| BOUNDS | [analyser/bounds_checks.rs](../../../rust/tcl-compiler/src/analyser/bounds_checks.rs), [interval_bounds.rs](../../../rust/tcl-compiler/src/interval_bounds.rs) |
| TEXT | [source_decode.rs](../../../rust/tcl-lsp-core/src/source_decode.rs), [source_style.rs](../../../rust/tcl-lsp-core/src/source_style.rs), [source_integrity.rs](../../../rust/tcl-compiler/src/analyser/source_integrity.rs) |
| CHECK | [compiler_checks.rs](../../../rust/tcl-compiler/src/compiler_checks.rs) |
| TAINT | [taint.rs](../../../rust/tcl-compiler/src/taint.rs), [registry taint](../../../rust/tcl-registry/src/taint.rs) |
| SHIMMER | [shimmer](../../../rust/tcl-compiler/src/shimmer) and CHECK |
| EVENTS | [irules_event_checks.rs](../../../rust/tcl-compiler/src/analyser/irules_event_checks.rs) |
| IRULE | [irules_checks.rs](../../../rust/tcl-compiler/src/irules_checks.rs), CHECK |
| BIGIP | [validator.rs](../../../rust/tcl-bigip/src/validator.rs) |
| IAPP | [iapp_diagnostics.rs](../../../rust/tcl-bigip/src/apl/iapp_diagnostics.rs) |
| SSLIC | [dsl.rs](../../../rust/tcl-sslictcl/src/dsl.rs) |
| OPT | [optimiser](../../../rust/tcl-compiler/src/optimiser); each row names its pass family |
| SERVER | [server implementation](../../../rust/tcl-lsp-server/src/lib.rs) |
| XC | [translator.rs](../../../rust/f5-xc/src/translator.rs), [diagnostics.rs](../../../rust/f5-xc/src/diagnostics.rs) |
| BPF | [bpf-tcl-ir/src](../../../rust/bpf-tcl-ir/src), [BpfDiag](../../../rust/bpf-tcl-ir/src/diag.rs) |

## Syntax, decoding and invocation validity

These checks demonstrate why “everything is a value transfer” is the wrong
abstraction. Many need only syntax or a resolved invocation. Preserve their
cheap path; do not initialise a VM or solve SSA to detect an unmatched brace.

| Code | Producer | Required evidence | Ownership / adversarial acceptance obligation |
|---|---|---|---|
| E001 | VALID, VARCMD, VERSION | Selected dispatch form and missing dispatch word | Share ensemble/object invocation resolution; dynamic expansion must not invent a missing method. |
| E002 | VALID | Proven argument-count lower bound and selected signature | Unknown expansion length is not a definite too-few error. |
| E003 | VALID | Proven count exceeding selected signature | Use the same version/form as lowering and hover; do not count source words as expanded argv. |
| E004 | VALID | Clause-grammar failure and offending operand | Move command grammar to registry structural plans; keep error code/range in the rule. |
| E005 | VALID | Signature count-shape relation, not just interval arity | Express pair/stride/alternative shapes declaratively; exercise new loop and ensemble forms. |
| E006 | VALID | Shared formal-list parser verdict | Definition grammars own formals; procedure/method/lambda consumers agree on invalid lists. |
| E100 | SYNTAX | CST/token unmatched close bracket | No command evaluator; preserve recovery spans and token-kind distinctions. |
| E101 | SYNTAX | Missing-opening-brace recovery evidence | Recovery guess is not a valid structural plan or optimisation proof. |
| E102 | SYNTAX | CST/token unmatched close brace | Distinguish escaped literal brace from structural token. |
| E103 | SYNTAX | Nested-body stolen-closer recovery | Keep evidence/source origin through nested body mapping; do not rebuild delimiters from text. |
| E200 | SYNTAX | Lexer/segmenter incomplete-command boundary | Incomplete editor text stays analysable conservatively, without speculative execution. |
| E201 | SYNTAX | Unterminated bracket substitution | One syntax owner and one span policy across direct/incremental paths. |
| E202 | SYNTAX | Unterminated quoted word | Syntax error, not evaluator decline. |
| E203 | SYNTAX | Unterminated braced word | Preserve brace nesting and recovery evidence independently of command identity. |
| E204 | SYNTAX | Extra characters after variable-reference brace | Reuse lexer warning mapping; avoid another variable parser in rules. |
| E205 | SYNTAX | Extra characters after quoted variable name | Use lexer/source fact even when no executable IR is available. |
| E206 | SYNTAX | Missing variable-reference close brace | Preserve partial-source availability, not fabricated variable existence. |
| E207 | WALK | Analysis recursion/depth budget exhausted | Publish partial-analysis availability; never infer absence of downstream errors from skipped bodies. |
| W001 | VALID, widget diagnostics | Resolved subcommand/widget option catalogue | Use the selected registry/receiver form; unknown dispatch is different from proven unknown member. |
| W002 | VALID, OO analysis | Dialect availability of command/function/class construct | Binding and dialect facts shared by analysis and lowering; disabled code does not become builtin semantics. |
| W003 | VALID | Expression operator availability | Expression semantic profile owns operator set; BPF is not merely a Tcl version. |
| W004 | VALID | Option's dialect availability | Resolve abbreviation and form once; do not validate another option spelling than execution. |
| W107 | TEXT | Decoder replacement offsets and original-byte validity | Maintain original/decoded source distinction; no value-engine dependency. |
| W109 | TEXT | Decoder's non-text/unsupported-encoding abstention | Stop unsupported analysis explicitly, not with an empty “clean” fact snapshot. |
| W125 | WALK | Structural position and orphan keyword role | Registry grammar describes keywords; literal arguments must not be diagnosed as statements. |
| W142 | VALID | Lexical/dispatch-context constraint | Separate parse context from runtime reachability; custom body plans supply correct context. |
| W145 | VALID | Shared prefix-resolution ambiguity set | Same prefix owner for analyser, registry, runtime and suggested replacements. |
| W146 | VALID | `LiteralArgumentValidator` typed invalid/member verdict | Preserve Valid/Invalid/Abstain and operand identity; no command-specific revalidation in rules. |
| W147 | VALID | Resolved option relation conflict | Use shared relation verdict, not a second option scan; preserve authoritative dynamic-role resolver. |
| W149 | VERSION | Version-window signature alternatives | Show another-release fit without silently selecting that release for execution. |
| W152 | VALID | Missing companion/required-set relation | Preserve conditional uncertainty for expansions and dynamic forms. |

## Source style, expression usage and structural advice

| Code | Producer | Required evidence | Ownership / adversarial acceptance obligation |
|---|---|---|---|
| W100 | USAGE | Expression argument role, quoting and substitution stages | Share an expression-usage fact with O111; display filtering cannot be its producer. |
| W104 | USAGE | String-building operation in list-building context | Registry operation category plus use context; spelling alone must not justify a list rewrite. |
| W105 | USAGE | Body role and early substitution | Custom EDA bodies inherit the check through roles; opaque data does not become script. |
| W106 | USAGE | Switch body grammar and substitution staging | Reuse structural plan; dynamic arm layout may require abstention. |
| W108 | USAGE | Non-ASCII token content and source span | Policy-only source check; preserve Unicode character/byte distinction. |
| W110 | USAGE | Comparison operands and semantic string/numeric evidence | Share comparison reasoning with O120; advice is not an unconditional equivalence proof. |
| W111 | TEXT | Source line lengths and configured limit | Style configuration invalidates findings, not analysis domains. |
| W112 | TEXT | Trailing whitespace spans | Do not trim exact Tcl values while detecting source whitespace. |
| W114 | USAGE | Nested expression structure and staging | Shared nested-expression candidate with O115; preserve errors and substitutions. |
| W115 | TEXT | Comment continuation segmentation | Shared comment-line owner; a textual edit must preserve script boundaries. |
| W118 | TEXT | Line-ending distribution | Pure source policy, independent of registry/evaluator. |
| W121 | USAGE | Shared IP/network parsing and mask contiguity | Use numeric/network owner; literal malformed subnet is not an SCCP concern. |
| W124 | FLOW | Known argument value and shared IP parser verdict | Reuse W121's semantic owner for propagated literals; no duplicate IP algorithm. |
| W200 | USAGE | Binary format grammar and release-gated modifiers | One format parser, selected Tcl release; malformed format and unsupported feature distinct. |
| W201 | CHECK, path_concat | Path-construction provenance and operation shape | Domain path fact, not string equality alone; platform/volume semantics constrain any edit. |
| W212 | USAGE | Variable-name role and source substitution provenance | Resolved roles drive check, including custom commands; computed variable names can be intentional. |
| W215 | WALK | Shared variable-name grammar and declaration span | Name validity is not value type; preserve valid indirect access despite no dollar form. |
| W216 | USAGE | Variable-reference parse and literal suffix/index | Reuse syntax owner; do not infer substitution from decoded contents. |
| W217 | VALID | Actual operands remaining after option resolution | No independently hardcoded `unset` option scanner in a generic consumer. |
| W218 | WALK | Formal-list position of `args` | Definition grammar owns variadic meaning; list membership alone is insufficient. |
| W314 | WALK | Namespace/name resolution and absolute-name reachability | Shared names owner, not string prefix heuristics in diagnostics. |

## Binding, packages, dialects and definitions

| Code | Producer | Required evidence | Ownership / adversarial acceptance obligation |
|---|---|---|---|
| W113 | WALK | User definition and resolved builtin binding | Same binding evidence as evaluator eligibility; spelling equality is not identity. |
| W116 | VALID | Stub definition colliding with builtin | Overlay revision and provenance shared with analysis; disabling warning must not restore builtin trust. |
| W117 | VALID | Stub math definition colliding with builtin | Invalidate expression evaluation transitively on math binding changes. |
| W120 | RESOLVE, SERVER | Package provider and document/workspace require evidence | Workspace refinement is a fact query, not ad hoc message removal; preserve execution-time uncertainty. |
| W123 | RESOLVE, VARCMD, SERVER | Command/receiver/unknown-handler resolution | Unknown binding cannot use builtin constant evaluator; dynamic name facts can refine resolution. |
| W128 | FLOW | Ordered rename/delete binding transitions | Program-point identity, not whole-file name sets; preserve `unknown` behaviour. |
| W129 | WALK | Safe-interpreter visibility and exposure state | Capability/namespace facts, not purity; hidden commands cannot inherit normal builtin execution. |
| W130 | Reserved, no producer | Manifest requirement versus lockfile | Future package graph owner; explicitly reserved, not covered by a value-transfer implementation. |
| W131 | Reserved, no producer | Manifest/lock consistency | Future revisioned package facts; reserved status must remain visible in coverage. |
| W132 | Reserved, no producer | CAS integrity evidence | Package/content integrity owner, not command VM or a claimed constant hash. |
| W133 | Reserved, no producer | Package directive safe-mode policy | Capability verdict separate from diagnostic display and evaluator sandbox. |
| W134 | Reserved, no producer | Resolved package index availability | Filesystem/package snapshot dependency; lack of producer is intentional at this baseline. |
| W135 | VERSION | Command introduction and resolved package floor | One availability query feeds all consumers and summaries. |
| W136 | VERSION | Option introduction and selected package floor | Per-form/option availability; do not borrow base-command version. |
| W137 | SECURITY | Value-domain release gate | Registry-declared value constraint with target context, not a rule-local command list. |
| W138 | VERSION | Format/scan conversion introduction | Shared format semantics; coexist with W200 without competing parsers. |
| W139 | VERSION | Exclusive retirement boundary | Central version comparisons; diagnostics and command execution eligibility agree. |
| W140 | WALK | Interpreter creation/deletion and target path resolution | Dynamic interpreter lifecycle is a state domain; unknown creation is not proven absence. |
| W143 | VALID | Registry/public API boundary and resolved namespace | Keep private-API policy separate from whether a call can actually execute. |
| W144 | VERSION | Deprecation metadata at selected target | Deprecation is policy, not unavailable semantics or an automatic rewrite. |
| W148 | WALK | Numeral acceptance under selected release | Shared numeric owner used by direct folds and VM adapters. |
| W150 | VERSION | Availability over the whole declared target range | A fold valid at one target must not silently justify cross-target output. |
| W151 | WALK | Numeral meanings/validity across target range | Preserve exact spelling; `010` must not lose its release-dependent meaning. |
| W250 | VARCMD | Receiver/class abstraction and instantiation contract | Object/type graph and registry definition grammar; no value hook inventing a class. |
| W308 | VARCMD | Receiver type, inheritance and method visibility | Unknown receiver is not a definite missing method; invalidate on dynamic class mutation. |
| W315 | WALK, OO refinement | Ordered definition-body member-table operations | Partial/aborting definitions require state/completion modelling, not one final member set. |
| H301 | RESOLVE | Source order of provider requirement and invocation | Keep editorial ordering hint distinct from runtime command-resolution proof. |

## SSA, existence, ranges, control flow and liveness

These are the primary integration tests for the proposed value architecture.
They require analysis facts at program points, not a bag of name-to-value
results. Const-to-dynamic transitions must preserve other known domains and
bounded explanations; see R15 and the main review's partial-reduction section.

| Code | Producer | Required evidence | Ownership / adversarial acceptance obligation |
|---|---|---|---|
| W126 | FLOW | Argument role plus semantic channel/handle type | Existing dynamic return hooks and rich types feed one query; string-shaped handles remain distinct. |
| W210 | FLOW | Reaching definition, existence, executable predecessors and conditional writes | Remove private regexp/scan computation (R14); no-match/partial writes/aliasing/custom-loop zero path must agree with SCCP. |
| W211 | FLOW | Def-use/liveness and escape/reflection evidence | Registry output binders create real definitions; external/body uses prevent false unused findings. |
| W213 | FLOW | Existence before unset and resolved no-complain mode | Reuse store transitions and option resolution; Unknown is not definitely absent. |
| W214 | FLOW | Formal binding and body/callback uses | Custom definition/body grammars participate in scope and liveness without rule edits. |
| W220 | FLOW | A definition overwritten on relevant paths before any observable use | Shared dead-store evidence with O109; a warning does not authorise removing a failing/traced write. |
| W230 | BOUNDS, FLOW | List length/shape, index interval and operation's clamp/empty semantics | Exact and interval paths use shared list/index owners; opaque EDA collection is not a list. |
| W231 | BOUNDS, FLOW | List shape, index path and error-producing update semantics | Distinguish `lset` failure from read/clamp operations; partial mutation/error must survive optimisation. |
| W232 | BOUNDS, FLOW | Target-aware string length/index facts and no-op/clamp contract | Character units/version and end-relative indices shared with direct evaluation. |
| W233 | FLOW, interval analysis | Divisor exactly/range-proved zero and reached evaluation | Lazy branches must not report an unreachable error; Tcl and BPF arithmetic profiles differ. |
| W240 | BOUNDS | Loop entry predicate proved false | Generic loop protocol supplies entry edge; custom iteration cannot be inferred from body role alone. |
| W241 | BOUNDS | No terminating path plus induction/step and completion evidence | Unknown callbacks/break/error invalidate proof; wrong-direction heuristics must not become optimiser facts. |
| W242 | BOUNDS | Failure to establish loop termination, with reason | Optional hint represents proof absence, not proven nontermination; EDA opaque iteration can legitimately abstain. |
| H300 | FLOW | Consecutive/repeated assignment values and source context | Reuse reaching-value facts; equal values do not imply effect-free repeated assignment. |
| I230 | FLOW | Typed branch fact and supporting executable-edge/value/existence evidence | Consolidate existing existence post-pass/recomputation; condition observation is not automatically an SCCP edge (R14). |
| I231 | FLOW | Ordered switch-arm selection, modes, fall-through and evidence | Opaque arm observation and CFG reachability remain distinct until structural integration (R9). |

## Security, taint, representation and resource state

These facts cannot be reconstructed from exact values alone. In particular,
unknown exact value does not mean unknown taint, and exact text does not
prove safe provenance, internal representation or ownership.

| Code | Producer | Required evidence | Ownership / adversarial acceptance obligation |
|---|---|---|---|
| W101 | SECURITY | Script-concatenation role and substitution stages | Registry script contract shared with lowering/taint; do not duplicate `eval` semantics in the rule. |
| W102 | SECURITY | Substitution language and variable-input provenance | Selected flags affect executed substitution kinds; bounded VM is not permission to execute untrusted scripts. |
| W103 | SECURITY | Channel-open mode and known/possible pipeline prefix | Immediate/deferred literal paths share prefix/value facts; unknown suffix must retain taint. |
| W127 | SECURITY | Closed value-set constraint verdict | Registry/domain validator owns allowed set; conditional/dynamic inputs abstain appropriately. |
| W141 | SECURITY | Option value's structural/content validator verdict | Share shape parsing and resolved option identity; do not conflate malformed with not-in-set. |
| W300 | SECURITY | Source-file script role and dynamic path provenance | Source execution effect belongs in registry; no analysis-time filesystem execution fallback. |
| W301 | SECURITY | Uplevel script staging, frame/alias effects | Generic completion/escape facts retain dynamic caller scope semantics. |
| W302 | SECURITY | Catch form and absence of result handling | Structured completion/result bindings, not inferred from return type alone. |
| W303 | SECURITY | Pattern syntax/cost-risk analysis | Reuse our regexp semantic owner where applicable; fuel exhaustion alone is neither no-match nor a ReDoS proof. |
| W304 | SECURITY | Option-bearing form, operand provenance and terminator position | One resolved invocation for literal and deferred checks; avoid duplicate option scanners. |
| W305 | TEXT | Bidi control positions in source | Pure source-integrity owner, unrelated to constant execution. |
| W306 | SECURITY | Literal-only argument contract and source substitutions | Preserve source provenance even if substitution can be folded to a constant. |
| W307 | VARCMD | Nonliteral command head and resolved/dynamic binding | Warning policy does not erase possible callees or legitimise builtin evaluation. |
| W309 | SECURITY | Nested substitution/evaluation staging | Shared script-expression structure; preserve evaluation count and errors in any suggested edit. |
| W310 | SECURITY | Registry credential role and source-literal provenance | Do not expose secret values in explanation/cache logs; do not mistake computed constant for authored literal. |
| W311 | USAGE | Channel encoding/translation state and interaction | Multi-call resource state, not independent value folds; unknown reconfiguration invalidates the relation. |
| W312 | SECURITY | Interpreter-eval form and script concatenation | Share interpreter state and script roles with W140/T105; do not rescan by command name. |
| W313 | TAINT | Destructive-path sink and path taint/provenance | Registry sink/context contract; constant prefix is not traversal sanitisation. |
| T100 | TAINT | Source-to-sink flow, expression staging and numeric/type coercion | Existing return-type sanitiser hook integrates with richer taint; numeric knowledge is context-specific, not universal cleansing. |
| T101 | TAINT, registry | Output sink role and incoming flow | New output command uses registry sink descriptor, no additional taint command-name case. |
| T102 | TAINT | Tainted operand reaching option region without terminator | Consume canonical expansion/option facts; exact folding must not hide source taint. |
| T103 | TAINT | Pattern role and pattern taint | Separate injection risk from exact matching and regex complexity; use our regexp engine, not a second evaluator. |
| T104 | TAINT | Network-address sink and address provenance | Destination/resource policy remains separate from string type and successful parsing. |
| T105 | TAINT | Cross-interpreter execution sink and flow | Capability and interpreter isolation facts matter even for a calculable script string. |
| T106 | TAINT | Encoding-state provenance and repeated transform kind | Track encoding identity, not only string result or generic sanitiser bit. |
| S100 | SHIMMER | Guaranteed intrep transitions at uses outside loops | Integrate existing dynamic return hooks; semantic list membership is not list intrep. |
| S101 | SHIMMER | Same transitions plus loop execution context | Custom EDA loop plan supplies loop context without requiring exact enumeration. |
| S102 | SHIMMER | Loop-carried representation oscillation | Phi/back-edge joins retain representation history/provenance; do not infer from two source spellings. |
| S103 | SHIMMER | Potential sharing/aliasing and mutating operation | Track copy-on-write risk independently of constant value; registry mutation descriptor centralises recognition. |
| S110 | SHIMMER | Byte-array provenance and string coercion operation | Exact transport must preserve bytes and not manufacture string/intrep facts. |
| TK1001 | Tk analyser | Parent widget's geometry-manager claims | Domain ownership/state machine, not command return values; conditional creation/management needs path joins. |
| TK1002 | Tk analyser | Widget hierarchy and parent existence | Dynamic widget construction produces unknown existence, not proven missing parent. |
| TK1003 | Widget diagnostics | Resolved widget kind and option schema | Receiver/type resolution and registry option schema shared; mutable widget command identity invalidates facts. |

## iRules protocol, event, deployment and security domains

Per-command domain events belong in registry metadata/specialisation;
state-machine algorithms belong in their domain analysis owner. Diagnostic
generation consumes a violation/observation with provenance. Centralisation
does not require collapsing HTTP state, taint and scalar SCCP into one lattice.

| Code | Producer | Required evidence | Ownership / adversarial acceptance obligation |
|---|---|---|---|
| IRULE1001 | EVENTS | Command's allowed/effective event context | Registry event constraints and resolved body context; custom wrappers require summaries. |
| IRULE1002 | EVENTS | Event catalogue resolution | Registry event owner; unknown event is independent of body value analysis. |
| IRULE1003 | EVENTS | Event deprecation metadata | Policy only; deprecated is not unreachable. |
| IRULE1004 | EVENTS | Event priority declaration and registry requirement | Explicit versus default priority provenance preserved. |
| IRULE1005 | IRULE | Data-event entry state and required collection protocol | Generic registered protocol relation, with event/connection side identity. |
| IRULE1006 | IRULE | Payload read requiring prior collection | Path-sensitive resource state; equal payload text does not prove collection occurred. |
| IRULE1007 | IRULE | Outstanding collection without required release | Model completion paths and connection side; do not treat any later release spelling as sufficient. |
| IRULE1008 | IRULE | Release without corresponding collection state | Unknown protocol state differs from proved absent registration. |
| IRULE1201 | IRULE | HTTP response-commit state before later operation | Registry domain events feed one flow analysis; code motion must respect commit effects. |
| IRULE1202 | IRULE | Multiple/conflicting response commits across paths | Preserve path compatibility and completion; do not count mutually exclusive syntax blindly. |
| IRULE2001 | VALID | Deprecated operation and replacement metadata | Advice separate from equivalence of replacement; new aliases should inherit registry policy. |
| IRULE2002 | VALID | Command deprecation in iRules target | Central availability/deprecation query, not duplicated Tcl/iRules tables. |
| IRULE2003 | EVENTS | Unsafe-command classification in selected dialect | Diagnostic severity does not grant executable capability or evaluator trust. |
| IRULE2004 | VALID | Rule-load refusal versus runtime interpreter availability | Preserve distinct load/runtime phases; runtime reachability via eval is not load-time acceptance. |
| IRULE2101 | EVENTS | Regex cost category and high-frequency event context | Cost advisory uses registry/engine facts, not a requirement to run every pattern during analysis. |
| IRULE3001 | TAINT, registry | Response-body sink and HTML-context flow | Exact string/type cannot erase HTML taint; context-specific encoding provenance required. |
| IRULE3002 | TAINT, registry | Header/cookie sink and CRLF-sensitive flow | Sanitiser identity/context matters; generic string normalisation is not sufficient. |
| IRULE3003 | TAINT, registry | Log sink and line/control-character provenance | Preserve unknown suffix/segment flow through partial string reduction. |
| IRULE3004 | TAINT, registry | Redirect sink and destination provenance | URL encoding alone does not establish a trusted destination. |
| IRULE3101 | TAINT, registry | Setter value's proved leading slash | Prefix fact available with dynamic suffix; avoid requiring whole-string constness. |
| IRULE3102 | EVENTS, IRULE | URI getter form and normalisation state | Consolidate duplicate form recognition; use canonical invocation and shared URI provenance. |
| IRULE3103 | CHECK, uri_split | Manual parsing of unnormalised URI provenance | Domain parser/provenance owner, not equal-string comparison; preserve origin across aliases and summaries. |
| IRULE4001 | EVENTS | Static storage write and event identity | Registry writes and resolved storage place, including indirect aliases. |
| IRULE4002 | IRULE | Static declaration name and collision policy | Name-style policy should not affect storage identity or value analysis. |
| IRULE4003 | EVENTS | Variable scope and event lifetime | Event boundaries and shared/local places explicit; no reuse of proc-local SSA across events. |
| IRULE4004 | IRULE | Constant initialisation plus event lifetime/frequency | Hoisting also needs effect, binding and per-connection equivalence; a constant value alone is insufficient. |
| IRULE4005 | FLOW | Cross-event shared writes/reads and possible concurrency | World/event summary, not intraprocedural SCCP; retain may-alias and unknown event effects. |
| IRULE5001 | EVENTS | Log operation, event frequency and guard context | Cost/policy fact independent of log's value computability. |
| IRULE5002 | IRULE | Drop/reject/discard effect and subsequent disable/return paths | Shared completion-aware CFG/domain flow, including catch and nested bodies. |
| IRULE5003 | EVENTS | Induction step, comparison shape and possible zero-skipping | Numeric/range proof shared with loop analysis; unknown custom loop semantics must abstain. |
| IRULE5004 | IRULE | DNS response operation and Tcl completion | Domain return is not Tcl `return`; registry completion descriptor prevents conflation. |
| IRULE5005 | WALK | Dialect-specific procedure dispatch convention | Call resolution/lowering and diagnostic agree on direct versus `call` invocation. |
| IRULE5006 | EVENTS | Top-level-only command's actual lexical placement | Structured body/scope plan supplies nesting; folded body text does not change placement. |
| IRULE5007 | EVENTS | Declaration-only top level and executable statement | Registry structural classification; no independent name catalogue in the walker. |
| IRULE6001 | EVENTS | Global storage use and CMP compatibility effect | Storage/alias effects survive constant propagation; equal local/global values are not interchangeable. |

## BIG-IP configuration and iApp graph facts

These require a revisioned configuration/resource graph, not a Tcl VM. The
registry's object-reference and profile schemas centralise command meaning;
the configuration validator owns cross-object relations and diagnostic policy.

| Code | Producer | Required evidence | Ownership / adversarial acceptance obligation |
|---|---|---|---|
| BIGIP6001 | BIGIP | Data-group reference and configuration lookup | Resolve partition/path and registry argument role; unknown computed reference is not missing. |
| BIGIP6002 | BIGIP | Pool reference and resource lookup | Share object-reference interface with generic BIGIP6013. |
| BIGIP6003 | BIGIP | Virtual-server attachment to missing iRule | Configuration graph owner; no command evaluator needed. |
| BIGIP6004 | BIGIP | Attached rule's command requirements versus profiles | Command summaries plus deployment graph; update on either rule or profile edit. |
| BIGIP6005 | BIGIP | Virtual server's pool reference | Resource namespace resolution shared with BIGIP6002. |
| BIGIP6006 | BIGIP | Data-group reverse-reference set | Dynamic/unknown references weaken unused proof; do not claim absence from an incomplete graph. |
| BIGIP6007 | BIGIP | SNAT-pool command reference | Registry object kind/argument position rather than command-name matching. |
| BIGIP6008 | BIGIP | Pool declaration and member cardinality | Configuration shape fact; incomplete parse needs availability handling. |
| BIGIP6009 | BIGIP | Duplicate attachment identities | Resolve canonical resource identity before comparing spelling. |
| BIGIP6010 | BIGIP | Persistence operation requirements and attached profile | Shared command capability summary plus configuration graph. |
| BIGIP6011 | BIGIP | IP data-group record parser verdict | Reuse shared IP/network owner used by Tcl diagnostics. |
| BIGIP6012 | BIGIP | Event handlers and effective priority per attachment | Preserve explicit/default priorities and composition scope. |
| BIGIP6013 | BIGIP | Generic registry-declared object reference | New resource-bearing command works without validator name cases. |
| BIGIP6014 | BIGIP | Duplicate object kind and canonical path | Same namespace resolver as all reference diagnostics. |
| BIGIP6038 | BIGIP | Event-to-profile requirement over deployment graph | Event registry metadata and profile compatibility owner, not rule-local guessing. |
| BIGIP6039 | BIGIP | Incompatible attached profile types | Configuration type relation independent of Tcl value lattice. |
| IAPP7001 | IAPP | Implementation reference to presentation field | Cross-language symbol graph and source mapping, not Tcl constant lookup alone. |
| IAPP7002 | IAPP | Presentation field without implementation uses | Dynamic references/incomplete implementation weaken unused proof. |
| IAPP7003 | IAPP | Include resolution and filesystem snapshot | Explicit dependency/availability; safe analysis must not execute included text. |

## SslicTcl declarative vocabulary

The DSL owner already supplies a separate semantic boundary. Its deliberate
non-execution is an important negative test for the proposed runtime reuse.

| Code | Producer | Required evidence | Ownership / adversarial acceptance obligation |
|---|---|---|---|
| SSLIC1001 | SSLIC | Declaration syntax/segmentation failure | Shared Tcl syntax owner, DSL-specific interpretation. |
| SSLIC1002 | SSLIC | Forbidden substitution/expansion in declarative input | Do not fold substitutions to bypass the literal-only vocabulary. |
| SSLIC1003 | SSLIC | Missing required vocabulary header | Document schema fact, no executable query. |
| SSLIC1004 | SSLIC | Duplicate header declarations | Preserve source locations and declaration order. |
| SSLIC1005 | SSLIC | Declaration word-count schema | DSL grammar, not generic Tcl command arity selected by spelling. |
| SSLIC1006 | SSLIC | Non-braced declaration body | Source literal provenance remains essential even if text is constant. |
| SSLIC1007 | SSLIC | Unknown member in closed block | Closed versus extensible vocabulary context explicit. |
| SSLIC1008 | SSLIC | Duplicate declaration kind/name | Shared DSL symbol table and canonical identity. |
| SSLIC1009 | SSLIC | Value outside schema domain | Domain validator separate from arbitrary Tcl execution. |
| SSLIC1010 | SSLIC | Missing required member | Whole-block structural fact with partial-parse availability. |
| SSLIC1011 | SSLIC | Unresolved declarative reference | Revisioned DSL symbol graph, not runtime command binding. |
| SSLIC1012 | SSLIC | Mutually exclusive members | Reusable constraint verdict shape, DSL-owned schema. |
| SSLIC1101 | SSLIC | Unknown declaration preserved as extension | Preserve opaque data; absence of semantics must not become an error or execution request. |
| SSLIC1102 | SSLIC | Newer vocabulary version and extension policy | Availability/provenance distinct from unsupported Tcl command. |
| SSLIC1103 | SSLIC | Retained predicate body, deliberately unevaluated | Never route it through VM evaluation merely because its inputs are constant. |

## Every optimisation code

These rows are proof obligations for the shared architecture. Several
existing passes recognise source idioms, which is legitimate; they must
query central semantic operations and facts rather than reimplementing the
meaning of a command. The optimiser owns choosing and applying a rewrite.

| Code | Current pass family | Required evidence | Additional proof / integration obligation |
|---|---|---|---|
| O100 | OPT propagation, branch cascade, CHECK | SCCP exact value at each use | Per-SSA version, alias/binding validity and correct Tcl quoting; preserve effectful producer. |
| O101 | OPT expression/branch folding | Shared expression result and completion | Target-aware arithmetic, lazy operands, errors and exact value transport; not numeric-only for all `expr`. |
| O102 | OPT load forwarding | Single reaching literal definition | Dominance, no intervening alias/trace mutation, source representation and substitution stage. |
| O103 | OPT static procedure summaries | Interprocedural result/effect summary and call inputs | Captures/globals, recursion, implementation/binding epochs, completion and argument effects. |
| O104 | OPT chain_fold | Ordered string-build chain and segment values | Preserve coercion, traces, errors and implicit result; retain dynamic segments for partial reduction. |
| O105 | OPT GVN/CSE | Value numbering and equivalent operation/effect dependencies | Same value is not same observable computation; memory/binding versions and exceptional behaviour matter. |
| O106 | OPT loop invariants | Loop structure, invariant operands and effects | Hoisting cannot introduce execution on zero-trip path, move errors or violate packet guards. |
| O107 | OPT elimination | Proven non-executable CFG region | Opaque switch hint is insufficient; executable-edge proof includes completion and dynamic control. |
| O108 | OPT aggressive dead-code elimination | Transitive liveness and effect/completion freedom | Preserve implicit return, errors, callbacks, traces and external-world effects. |
| O109 | OPT dead-store elimination | Reaching stores, no observable read before overwrite | W220 evidence plus removal proof; partial failing writes and shared/traced storage remain observable. |
| O110 | OPT expression simplifier | Typed residual expression and algebraic prerequisites | R15 floating-point witness blocks unconditional regrouping; monotone bounded residual facts are not constant strings. |
| O111 | SERVER brace-expression hints | Shared expression-usage finding/candidate | Existing filtered-W100 dependency leaks presentation into production (R14); make facts independent or define explicit group policy. |
| O112 | OPT structure elimination | Constant condition plus structured completion/CFG | Removing condition/loop scaffolding must preserve evaluation effects, bindings, fall-through and returned result. |
| O113 | OPT strength reduction | Numeric type/range, operator and target semantics | Integer versus float, sign/modulo/shift/overflow rules; do not transfer Tcl identities blindly to BPF. |
| O114 | OPT pattern recognition | Assignment/addition idiom and destination identity | `set`/`expr`/`incr` error, initial-existence, parsing, trace and release behaviour must agree. |
| O115 | OPT expression/propagation/branch paths | Nested expression structure | Shared candidate/proof; removal must preserve coercion, parsing and lazy evaluation stages. |
| O116 | OPT literal-list folding/propagation | Shared list construction exact value | Correct serialisation and command identity; no optimiser-local `list` execution algorithm. |
| O117 | OPT expression string-length rewrite | Empty-test semantics and string operation facts | Same value/error behaviour, target character semantics and substitution count; operation category from registry. |
| O118 | OPT lindex folding | Shared list/index evaluator and exact inputs | Nested indices, end offsets, malformed list/error, exact element spelling and target compatibility. |
| O119 | OPT pattern packing | Consecutive assignments and target binding plan | Packed `lassign`/`foreach` change results, traces, partial failure and scope; compare all, not just final values. |
| O120 | OPT string comparison | Proven intended string comparison domain | Numeric-looking strings and nonnumeric values differ under `==`/`eq`; W110 is not an edit certificate. |
| O121 | OPT tail-call sites | Self-call identity, tail position and completion | Rebinding, stack inspection, traces and target `tailcall` support; preserve argument evaluation. |
| O122 | OPT tail-call loop conversion | Recursive summary, argument/state mapping and CFG | Simultaneous argument updates, defaults/variadics, scope, errors and frame observability. |
| O123 | OPT accumulator hint | Recognised recursive structure and candidate algebra | Hint-only status explicit; associativity/overflow/float/evaluation-order proof required before automatic conversion. |
| O124 | OPT unused procedures | Whole-event call graph and possible dynamic calls | Reflection/unknown callbacks/entry roots weaken unused proof; commenting out is still a semantic edit. |
| O125 | OPT code sinking | Dominance, use regions, effects and availability | Moving a write changes execution paths, error timing and implicit results; backend guards/lifetimes matter. |
| O126 | OPT unused assignments | No reads/escapes and removable assignment | Separate W211-style unused fact from producer effects and store errors. |
| O127 | OPT assignment inlining | Single use and evaluation-order dependencies | Do not duplicate/move side effects, cross alias writes or alter source substitution stage. |
| O128 | OPT end-offset rewrite | Index argument role, same sequence identity and length arithmetic | List versus string units, index semantics, evaluation count and lifetime; new index-taking command uses registry role. |
| O129 | OPT registry folding/propagation | Explicit evaluator capability, exact result and effects | Direct core/expression/VM routes share context; purity alone is insufficient and known output does not authorise deletion. |
| O130 | OPT chain_fold list append | Ordered list-building transitions and exact/partial elements | Shared list owner, initial-cell existence, malformed list errors, aliases/traces and partial results. |

For the proposed **O131 switch specialisation**, reserve no code until the
ordered matching, CFG/opaque distinction, completion, source-edit mapping
and proof contracts in R9 are implemented. Reuse our regexp engine when
the switch mode requires it; budget decline must not select a default arm.

## XC translation catalogue outside DiagCode

The translator already projects `TranslationItem` records into diagnostics,
which is a useful separation. The underlying translation recognisers still
contain command-specific knowledge. Move expressible command mappings into
registry backend/domain descriptors while keeping XC target construction
and translation policy in its owner. “Constant result” is not “translatable
behaviour”; partial/untranslatable status must survive projection.

| Code | Producer | Required evidence | Ownership / adversarial acceptance obligation |
|---|---|---|---|
| XC100 | XC | Translated route/command action and enclosing matches | Registry action mapping plus translation IR; preserve outer conditions and partial status. |
| XC101 | XC | Route/redirect/response action mapping | Target capability and source action semantics, not result value. |
| XC102 | XC | Representable conditional/service-policy match structure | Shared expression/branch facts may help, but source match extraction is not SCCP reachability. |
| XC103 | XC | Header modification mapping | Registry operation/form and target header-action semantics. |
| XC105 | XC | Data-group/class-based policy mapping | Resource graph and predicate compatibility; opaque dynamic groups must remain partial. |
| XC106 | XC | ASM-disable to WAF-exclusion mapping | Security-policy translation effect, not a value transfer or permission to weaken protection silently. |
| XC107 | XC | ASM-enable already represented by target baseline | Record why no additional action is needed; target baseline is a dependency. |
| XC200 | XC | Dynamic switch subject not representable as target criteria | Preserve enclosing criteria; inability to map is not proof of an unconditional route. |
| XC201 | XC | Unsupported/unknown event translation | Event registry/capability mapping distinct from Tcl syntax validity. |
| XC203 | XC | Complex condition only partially mapped | Keep partial status and evidence; do not present a guessed equivalent policy. |
| XC250 | XC | Event better served by another XC facility | Advisory target policy, not analyser semantic rejection. |
| XC300 | XC | Known unsupported command/barrier | Registry backend capability or conservative unknown effect; don't simulate away an unsupported action. |
| XC301 | XC | Unmapped command-family operation | Unknown capability distinct from known untranslatable semantics; extensible registry descriptor avoids prefix catalogues. |

## BPF frontend catalogue outside DiagCode

See the [EDA/eBPF supplement](value-transfers-authoring-review.md) and
[backend contract](ebpf-backend.md). BPF-Tcl has different arithmetic and
acceptance rules, a distinct IR, and target safety obligations. These codes
must not disappear from “every diagnostic” coverage just because their
owner uses another enum.

| Code | Producer | Required evidence | Ownership / adversarial acceptance obligation |
|---|---|---|---|
| BPF001 | BPF frontend/lower | Construct outside accepted typed language subset | Calculable Tcl expression/call is not automatically legal BPF source. |
| BPF002 | BPF lower | Missing registry BPF operation for resolved command | Use existing `BpfOpSpec`; no automatic Tcl runtime fallback. |
| BPF003 | BPF parsing/lower | BPF form/arity mismatch | Same selected operation descriptor as lowering, including macro forms. |
| BPF004 | BPF expression/lower | Scalar/pointer/width type mismatch | Preserve BPF types and provenance; do not reuse Tcl coercion rules. |
| BPF005 | BPF expression/lower | Undefined local at program point | Definite assignment/control flow, including unrolled induction bindings. |
| BPF006 | BPF parsing/expression | Malformed or unacceptable integer literal | BPF numeric owner/profile, not Tcl bignum acceptance. |
| BPF007 | BPF unroll/lower | Invalid/unbounded loop count/protocol | It has active producers despite the enum's historical reserved wording; literal-bound contract is not arbitrary SCCP constness. |
| BPF008 | BPF frontend | Unknown/invalid BPF event | Registry event/program type compatibility. |
| BPF009 | BPF alloc/lower | Stack allocation/resource bound failure | Liveness/allocation and target ABI proof; exact values alone do not prove stack safety. |
| BPF010 | BPF profile | Invalid profile/field schema or reference | Revisioned profile type/layout facts and registry operation requirements. |
| BPF011 | BPF template | Invalid template/use declaration | Structural expansion owner and source-origin map; no unrestricted Tcl evaluation. |
| BPF012 | BPF capability | Profile allow/deny or command capability violation | Mandatory legality verdict independent of warning settings and constant folding. |
| BPF013 | BPF deploy | Invalid attach or program-type mismatch | Attach/program/ABI facts, not generic command availability. |
| BPF014 | BPF frontend | Stray executable top-level statement | Prevent silently dropped source; structural acceptance before code generation. |
| BPF015 | BPF lower | Handler path without explicit verdict | CFG completion over every path, not one constant branch's return value. |
| BPF016 | BPF lower | Map declaration/access kind, sizes and capacity mismatch | Map schema, helper capabilities, pointer/nullability and lifetime facts. |
| BPF017 | BPF frontend | Equal-priority handlers with ambiguous composition | Event identity/order provenance and source mapping through composition. |
| BPF999 | BPF internal checks | Compiler invariant failure | Internal failure is not a user semantic error or conservative evaluator decline. |

The separate rootless
[`VerifyError`](../../../rust/bpf-tcl-codegen/src/ebpf/verifier.rs) variants
are additional proof failures, not allocated diagnostic codes:

| Failure | Required backend fact / obligation |
|---|---|
| `NoExit` | Emitted control-flow/exit structure; frontend completion facts alone are insufficient. |
| `BadContextPrologue` | Correct context ABI and prologue, preserved by emission and optimisation. |
| `UnprovenPacketLoad` | Dominating packet bounds/provenance proof at the emitted load. |
| `MissingRelocation` | Required map/symbol relocation exists for emitted instruction. |
| `StackOutOfRange` | Emitted stack offset/width within target bounds. |

Passing these checks does not claim acceptance by every kernel verifier.
Backend proof diagnostics and CLI/LSP rendering should share source-origin
mapping, but backend legality must never depend on whether a diagnostic is
displayed.

## TLS estimate/report codes and extensible finding identities

A wider source scan finds another catalogue outside the central enum:
[estimate.rs](../../../rust/tcl-sslictcl/src/estimate.rs) emits 14 numbered
TLS findings, and the report's
[tls.rs](../../../rust/bigip-report-gen/rust/src/tls.rs) adds two. These are
not the `SSLIC` declarative-syntax diagnostics above. Their facts come from
TLS configuration, certificates, trust data and imported observations, not
Tcl value evaluation. This review describes the repository's current rules;
it is not an independent assessment of those rules' security adequacy.

| Code | Producer | Required evidence | Ownership / adversarial acceptance obligation |
|---|---|---|---|
| SSLICTL1001 | TLS estimate | No known effective protocol set | Unknown configuration is not an empty, secure protocol set. |
| SSLICTL1002 | TLS estimate | No known effective cipher set | Preserve coverage/confidence independently of finding visibility. |
| SSLICTL1010 | TLS estimate | HSTS configuration and estimate policy | Versioned scoring policy consumes configuration facts; rendering must not infer grade from displayed warning. |
| SSLICTL1020 | TLS report | Certificate/private-key SPKI mismatch | Move shared grade-cap/finding policy out of JSON report assembly; private-key material must not enter diagnostic evidence. |
| SSLICTL1021 | TLS report | Certificate/private-key match unavailable | Unknown is not mismatch; shared confidence/grade policy should serve all report/API consumers. |
| SSLICTL1101 | TLS estimate | SSL 2.0 enabled in effective configuration | Domain estimate policy, not Tcl command semantics or current-network observation. |
| SSLICTL1102 | TLS estimate | SSL 3.0 enabled | Same revisioned TLS fact snapshot as grading and policy evaluation. |
| SSLICTL1103 | TLS estimate | TLS 1.0/1.1 enabled | Separate configuration observation from policy severity/cap. |
| SSLICTL1104 | TLS estimate | Protocol prohibited by declared TLS facts | Explicit catalogue override and its provenance must agree between estimate and policy. |
| SSLICTL1201 | TLS estimate | Null/export/anonymous cipher classification | Shared cipher semantic owner, not independently repeated suite-name heuristics. |
| SSLICTL1202 | TLS estimate | RC4 cipher classification | Configuration evidence and scoring policy separate; no live scan implied. |
| SSLICTL1203 | TLS estimate | 3DES cipher classification | Same declared/derived cipher facts across estimates and policy checks. |
| SSLICTL1204 | TLS estimate | Cipher prohibited by declared facts | Author-supplied domain facts are authoritative; retain revision dependencies and explanatory provenance. |
| SSLICTL1301 | TLS estimate | Certificate signature algorithm identified as MD5 | Certificate parser/crypto owner supplies fact; estimate owns severity/grade policy. |
| SSLICTL1302 | TLS estimate | Certificate signature algorithm identified as SHA-1 | Algorithm identity, not text matching in presentation. |
| SSLICTL1303 | TLS estimate | RSA public-key size below repository policy threshold | Key type/size facts and policy version separate; unknown size is not a proved violation. |

`apply_chain_result` maps the 16
[`ChainFindingKind`](../../../rust/tcl-sslictcl/src/chain.rs) variants to
`SSLICTL-CHAIN-` plus the uppercased Rust debug name. These are finite
identities even though they have no numeric suffix:

| Kind / resulting code suffix | Required evidence and boundary |
|---|---|
| `NotYetValid` / `NOTYETVALID` | Certificate validity interval and explicit evaluation time; time belongs in dependencies. |
| `Expired` / `EXPIRED` | Same time/validity owner; a cached result cannot outlive its declared temporal assumptions. |
| `HostnameMismatch` / `HOSTNAMEMISMATCH` | SAN/hostname matching from the certificate owner, not a generic Tcl glob. |
| `LeafNotServerAuth` / `LEAFNOTSERVERAUTH` | Leaf extended-key-usage interpretation and endpoint purpose. |
| `UnknownCriticalExtension` / `UNKNOWNCRITICALEXTENSION` | Unsupported critical extension is not successful validation. |
| `IssuerNotFound` / `ISSUERNOTFOUND` | Incomplete supplied issuer graph, distinct from invalid signature. |
| `AmbiguousIssuer` / `AMBIGUOUSISSUER` | Alternative issuer paths and bounded selection evidence. |
| `IssuerNotCa` / `ISSUERNOTCA` | Basic Constraints CA semantics. |
| `IssuerCannotSign` / `ISSUERCANNOTSIGN` | Issuer Key Usage certificate-signing permission. |
| `InvalidSignature` / `INVALIDSIGNATURE` | Cryptographic verification result, not calculable certificate text. |
| `PathLengthExceeded` / `PATHLENGTHEXCEEDED` | Basic Constraints path-length accounting. |
| `NameConstraintViolation` / `NAMECONSTRAINTVIOLATION` | Issuer name constraints and subject names. |
| `UnsupportedNameConstraint` / `UNSUPPORTEDNAMECONSTRAINT` | Unsupported critical constraint remains unknown/unsupported, never permitted. |
| `PathLoop` / `PATHLOOP` | Issuer graph cycle/depth guard; distinguish structural cycle from budget exhaustion in evidence. |
| `ExplicitlyDistrusted` / `EXPLICITLYDISTRUSTED` | Represented client trust-program decision and trust-store revision. |
| `TrustUnknown` / `TRUSTUNKNOWN` | Missing represented trust evidence, not explicit distrust. |

Do not stabilise wire identities accidentally through `Debug` formatting:
either retain an explicit mapping or gate changes to these emitted names.
The source-backed architecture lesson is the same as evaluator declines:
incomplete, unsupported and invalid are materially different answers.

Two code families are intentionally open-ended, so a finite numeric list
cannot enumerate every possible runtime code:

- [`SSLICTL-POLICY-<check_id>`](../../../rust/tcl-sslictcl/src/policy.rs),
  including the built-in `SSLICTL-POLICY-grade`: declarative policy checks
  consume endpoint/certificate/estimate facts and return identity, severity,
  message and evidence. Preserve `(check_id, endpoint)` identity, declared
  policy provenance and stable evidence. Predicate bodies remain ignored,
  as the current policy owner explicitly requires; loading a pack must not
  start evaluation. User-authored diagnostic policy is not a value hook.
- `TESTSSL-<imported id>`: imported scanner findings carry external source
  identity and severity. They are observations, not locally proved facts.
  The current estimate code deliberately does not change a grade from
  scanner severity alone. Keep explicit mappings and observation revisions
  separate from report formatting and do not run a network scan implicitly.

The report currently changes serialised `grade`, `caps` and `confidence`
while constructing SSLICTL1020/1021. That is a concrete domain-policy leak
into presentation and should become a shared typed estimate refinement.
It is independent of whether the Tcl value-transfer project implements
that migration in its first slice, but must not be described as already
centralised.

The broader code scan also found `T3001`–`T3004` in the explorer's
[`taint_severity`](../../../rust/tcl-explorer/src/serialise.rs) display
helper, although the current catalogue uses `IRULE3001`–`IRULE3004`.
These old spellings are not extra current producers. They illustrate why
catalogue tests must cover consumers and severity policy, not only emission
sites. Removed W122 and dummy/test codes likewise are not active entries.

## What completeness now means

This ledger establishes **catalogue coverage**, not a declaration that the
proposal already represents all these facts. It does not. The missing
contract is the product of structural plans, independently available
analysis domains, explicit effects/completion, revisioned external worlds,
and separate target/diagnostic/rewrite consumers.

Implementation acceptance should attach a fixed-input positive, negative
and unknown/decline case to each applicable row, plus its incremental
invalidation case. Where two codes intentionally share a fact, test their
independent enable/suppression policy. Where multiple paths produce the
same code, test that literal, propagated, direct and incremental paths
agree without recomputing command semantics privately.

Extend the existing inventory gates to cover the XC/BPF/TLS catalogues,
extensible finding identities and uncoded backend failures, or declare
their separate gate owners explicitly.
Keep W130–W134 reserved until real producers land. Gate the semantic-owner
mapping as well as producer presence: a new command fitting an existing
interface must exercise relevant rows without editing generic consumers.

Finally, do not solve this by moving all diagnostic policy into registry
hooks. A registry-owned literal validator may already return a typed issue
and authored constraint explanation; that is compatible with separation.
A value evaluator returning arbitrary W/O codes, severity, LSP spans or
text edits is not. Analysis owns facts and proofs, domain validators own
their semantics, rules own findings, and presentation owns delivery.
