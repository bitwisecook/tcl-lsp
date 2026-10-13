# BIG-IP name-handling request

Give this document to the chat with BIG-IP access. Return answers and raw
measurements to the same `work` branch. This request covers name semantics;
it does not claim that stock Tcl behaviour describes the F5 interpreter.

## Handoff prompt

You have BIG-IP access. On `bitwisecook/tcl-lsp` branch `work`, prepare and run
isolated name-handling oracle probes for the unanswered variants of questions
F5-N01 through F5-N16 below. Read `answers/README.md` and
`answers/outstanding-operation-audit.md` first, then map existing answers to
exact contexts, operands and entered frames. Request only missing variants;
do not rerun answered operations as though their evidence were absent.
Read `AGENTS.md`, `experiments/name-handling/README.md`, this request,
`scripts/dev/bigip-probes/README.md` and the four-context generator/cleanup
contracts first. Preserve any incoming answers and branch changes.

The portable `experiments/name-handling/cases.tcl` and its catalogue provide
executable question inputs for applicable stock/CLI/iApp contexts. They are
not an iRule load file. Materialise individual TMM cases with the existing
probe infrastructure; retain generated source alongside each result. Choose
unique run-tagged probe names, check absence before creating, run in the
appropriate lab context, trap cleanup and prove absence after deletion. Do not
save configuration. Preserve raw acceptance and execution outcomes. Avoid the
legacy driver that deletes fixed names without collision checks.

Do not alter name-handling implementation to force agreement, infer missing
answers from stock tclsh, or treat rejected syntax as a runtime result.
Record unsupported/unavailable/infrastructure failures explicitly. A blocked
case needs a precise reason and, when possible, a reduced applicable variant.
The repository's implementation chat will consume your versioned answer
records and turn real observations into regressions.

Commit answer documents and raw transcripts under
`experiments/name-handling/f5/answers/<run-id>/`, and reusable materialised
scripts/probes under that run or a shared experiment source directory. Use
`ANSWER-TEMPLATE.md`; include the input hash and map every result to question
and case IDs. Commit and push answer batches regularly on `work`; the user authorises
development pushes while checks are red. Record actual checks and failures.
All required repository gates must pass before the final PR. Do not
rewrite existing answers or erase contradictory measurements. Return a concise
list of answered, unsupported and still-pending questions and the branch
commit. No implementation/development diary belongs in the answer document.

## Context matrix

For each relevant question record separately:

- TMM `RULE_INIT`, iRule proc, and traffic event body where legal.
- tmsh CLI script.
- iApp implementation (and whether APL is a separately reachable interpreter).
- Appliance stock tclsh controls, with reported version.

Record BIG-IP version, hotfix/build, platform, TMM count and relevant profiles/
events. A control does not substitute for an unavailable TMM/iApp measurement.
Distinguish load/compile ACCEPT/WARN/REJECT from runtime completion and value.

## Questions and executable seeds

| ID | Required question and variants | Starting input |
| --- | --- | --- |
| F5-N01 | Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval. | Existing command-availability/context-parity probes; materialise one feature per probe. |
| F5-N02 | Bare and relative-qualified command lookup: current, path, global order; qualifier namespace exists without target; no ancestor walk; absolute bypass. Does path exist? | NH01–NH04; dynamic command head variants. |
| F5-N03 | Colon runs, empty command/tail, lone colon names, namespace segment colons and duplicate display FQNs: which slots are actually reached? | NH09–NH10, plus returned `namespace which`/origin text. |
| F5-N04 | Qualified/relative-qualified variables and namespace-scope missing-read fallback: read/write/create/exists differences. | NH05–NH07, then declarations/unset variants. |
| F5-N05 | Literal `{}`, `{$n}`, `{{a}}`, Unicode and byte-valued names; array indices with colons/parentheses. Are byte values representable in each context? | NH08; add binary/Unicode/array inputs without lossy conversion. |
| F5-N06 | `global`, `variable`, `upvar 0/1/#0`, `namespace upvar`, eval/uplevel composition: exact target frame and cell, including namespace-eval callers. | NH14–NH15 plus target-frame namespace and alias-chain variants. |
| F5-N07 | Import/export/force/forget/origin support and lifecycle; global-origin imports, source rename/redefinition/deletion, chains and conflicts. | NH11–NH12, NH16 and supported reduced variants. |
| F5-N08 | `interp alias` target namespace anchoring, late binding, qualified destination parent creation, describe/delete, child interpreters and duplicate prefixes. | NH13 plus #2193/#2269/#2275 reduced witnesses. |
| F5-N09 | Rename, command traces, unknown/namespace-unknown: when are lookups repeated, which target namespace is used, which errors/options are returned? | Existing unknown/rename fixtures; materialise callback mutation cases. |
| F5-N10 | Namespace deletion/recreation while active: command/path/OO/trace tokens and caches; do old compiled descendants retain old identity? | Reduced #1755/#1778 lifecycle witnesses. |
| F5-N11 | iRule proc namespace: unqualified invocation versus `call`, rule/partition qualification, rename, `info commands` and visibility across rules. | Existing proc-semantics suite; compare identical proc tails in isolated rules. |
| F5-N12 | Protocol prefix commands versus true namespaces: same-tail user shadows and profile/event availability, absolute/relative/computed invocation. | Registry namespace model and command-availability controls. |
| F5-N13 | Persistence and isolation: definitions/variables from RULE_INIT into proc/events and repeated requests, per TMM/per connection/cross-rule scope. | Existing dynamic-code suite; unique run/case records and controlled traffic. |
| F5-N14 | Expr mathfunc names and object/method/forward context where supported: resolution namespace, shadows and mutation. | Exact feature-discovery probes first; unsupported is a measured capability. |
| F5-N15 | Error conformance: missing command/variable/namespace, forbidden link and rename/import conflict: result, completion, `-errorcode`, `-errorinfo`, options. | Error variants of NH02/NH04/NH05/NH08/NH11; inspect options only where supported. |
| F5-N16 | Do lexical accepted names and runtime evaluated names differ? Compare braced/quoted/substituted/list command prefixes, brace-newline rules and unaddressable-name output round trips. | Portable source variants plus existing parser measurement inputs. |

Keep exact questions even if a context lacks the feature: the answer should
state how capability was measured. Split a question into stable variant IDs
such as `F5-N07-import-rename-RULE_INIT`, and preserve a mapping to portable
NH IDs where the script is identical. This table is the question backlog;
not all variants are already materialised runnable appliance probes.

## Observation constraints

Literal missing commands may reject an iRule before its `catch` runs. Build
unknown command names dynamically for runtime lookup probes, and keep a
literal-control probe to demonstrate the acceptance distinction. `proc` in a
when body may be rejected lexically even inside a braced eval body; where
measuring runtime construction, hold script text in a variable and retain
that exact variant.

`RULE_INIT` runs per TMM. Record all duplicate records with TMM/run/case identity;
deduplicate only as an explicit derived projection. Syslog may collapse
identical messages: emit unique joined per-case records. iApp merge does not
execute the implementation; record execution via `execute-action definition`.
Do not use `tcl_patchLevel` unguarded in contexts that do not expose it.

No current answer is implied by a stock C control, the protocol names, an
older BIG-IP transcript or a passing load check. Retain source and raw logs so
future agents can independently interpret the result.
