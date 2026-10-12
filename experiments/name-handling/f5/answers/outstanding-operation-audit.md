# Measured operation scope and remaining variants

The [307-row answer file](../../patterns/f5/answers/nh20261012finish1/ANSWERS.md)
records a reviewed conclusion for every previously outstanding matrix row.
The 400-row matrix now has 108 observed answers, 25 unsupported native-command
boundaries and 267 unreached original operations. Its 93 earlier accepted
answers are preserved. Completing an investigation does not mean an original
operation ran after a setup failure, ownership collision or constraint skip.

## Native questions: 40 context rows

The [non-TMM capability run](../../patterns/f5/answers/nh20261012nativecap1/README.md)
and [actual Chrome APL callbacks](../../patterns/f5/answers/nh20261012aplrepair1/README.md)
record absent native `call` and actual failing dispatch attempts. These supply
25 scoped unsupported rows: five questions in CLI, iApp implementation, iCall,
APL and aggregate appliance stock Tcl. Stock 8.4 and 8.5 have separate receipts.
They supply no hypothetical provider routing or storage answer.

The other 15 rows have newly applied native configurations and measured
operation families. [Native1](../../patterns/f5/answers/nh20261012native1/ANSWERS.md)
retains the full route, argument, lifetime, cell and paired-connection campaign;
[native2](../../patterns/f5/answers/nh20261012native2/ANSWERS.md) retains small
RULE_INIT completion controls after native1 log bursts lost some fields.
[Exact composite selectors](../../patterns/f5/answers/nh20261012native1/native-composites.json)
bind every conclusion to source, configuration, context, unit and invocation.

| Question | Measured result | Limits |
| --- | --- | --- |
| `corpus-f5-rule-path` | Caller-local, Common relative/absolute, absolute folder and second-partition routes select distinct owner sentinels. Folder-leaf relative and missing routes return captured errors. | Caller is in the retained Common folder; no other caller partition or unspecified folder search is inferred. |
| `corpus-f5-computed-target` | Literal, variable-built and list-built matching target bytes reach the same provider with identical argv bytes. | Only the retained targets, argument bytes and invoking contexts. |
| `corpus-f5-prefix-arguments` | Direct literal and eval/linsert list-prefix forms pass two words, `a b` and `{x} ::tail`. Space-joining reaches a wrong-argument-count error. | Supported list-prefix form, with no claim about arbitrary expansion syntax. |
| `corpus-f5-rule-lifetime` | Continuing HTTP and HTTP-called procedure frames return OLD, NEW after accepted replacement, NEW after failed replacement/detachment/configuration deletion, then RECREATED. Fresh RULE_INIT observers have independently recorded phase receipts. | Native2's all-four-unit OLD/NEW/rejected sequence is a different owned provider from native1's detachment/deletion/recreation sequence. Missing native1 init fields remain missing. No suspended-frame or restart claim. |
| `corpus-f5-storage-context` | Provider writes change an `upvar 1` caller cell and explicitly qualified namespace/global/static cells; caller readback matches. Paired connections share worker cells but keep their connection cells distinct. | RULE_INIT completion controls cover four actual units; traffic and HTTP-called procedure receipts cover actual unit `0:0`. No another-worker traffic inference. |

Native1 preserves incomplete captures, diagnostic omission and source/job
aliases. Its rejected load has an exit-code receipt and unchanged live
provider source, but its compiler stderr was withheld; no specific reason is
asserted from that receipt. Native2 retains an actual rejection diagnostic.
Fresh observer activation and continuing traffic calls remain separate.

## Broader numbered requirements

The [current coverage snapshot](nh20261012coverage2/answers.md) inventories
the 63 broader cases, individual capabilities, literal controls and native
controls. They are bounded witnesses, not an exhaustive variant claim.
The following identifies missing operations against the authored
[requirements](../REQUEST.md), rather than merely counting returned rows.

There is a common hosted-fixture issue: an unqualified `proc` inside
`namespace eval` creates a global command in CLI, iApp, iCall and APL on this
appliance. The independent [owner controls](nh20261012nsowner1/README.md)
and [actual APL owner control](nh20261012aplnsowner1/README.md) prove that
bounded difference. A failed later qualified call does not answer lookup
inside a procedure that was never installed there. New reduced fixtures
should define an explicit absolute owned procedure name, prove its actual
catalogue/body and invocation before the intended operation, and record the
procedure's actual namespace/frame. This is a new input, not promotion of
an unchanged original answer. Existing original cleanup limits remain in
[the cleanup review](../../patterns/f5/answers/original-hosted-cleanup-review.md).

| ID | Existing bounded evidence | Missing or still separate operation |
| --- | --- | --- |
| F5-N01 | 17 dynamic capabilities; 11 literal source forms across selected contexts; broader operations demonstrate further namespace/interp forms. TMM literal controls rejected before lookup. | Enumerate the namespace subcommands and interp forms intended by the requirement, then give each independently measured form its own result. The present capability list is not the complete API inventory. Add literal/computed paired controls for uncovered forms and actual APL literal forms, with absence guards for any definitions. |
| F5-N02 | Current/global, relative qualifier, no ancestor walk, absolute bypass and path capability; independent namespace-definition owner controls. | Hosted current/relative/global fixtures must prove the intended command slots, then measure current-before-global and an existing qualifier lacking its target. For unsupported path, the exact path capability error answers that boundary. |
| F5-N03 | Colon runs and empty tail/head; duplicate-display and lone-colon construction attempts. | Capture construction and slot invocation separately where creation fails. For constructible slots, identify each slot by distinct sentinel before comparing displayed FQN and invoking its round trip. Hosted failures after unexpected global definition do not establish qualified-slot selection. |
| F5-N04 | Relative variable read/write, missing-read global fallback, exists and missing-parent attempts. | Split namespace write, global readback, qualified readback, declaration and unset into individual operations; the existing write/readback program can fail only on its second read. Include create/exists before and after unset with distinct cells. |
| F5-N05 | Four bounded literal-name, constructed Unicode, NUL and array-index witnesses. | Add exact constructed byte-name values beyond NUL and a retained direct Unicode source-channel control, recording conversion/representability independently. Compare literal `{}`, `{$n}`, `{{a}}` names individually and both array-index colon/parenthesis spellings; no universal Unicode or binary range inference. |
| F5-N06 | Upvar 0/1/#0, global, variable and namespace-upvar capability; namespace-eval caller and variable alternating arguments. | Correct hosted procedure fixtures; add explicit eval/uplevel forwarding and a two-link alias chain with target-cell readbacks, actual levels and namespace at each frame. Keep namespace-upvar unsupported result separate. |
| F5-N07 | Source rename/redefinition/deletion, force conflict, import chain/origin, isolated global-origin forget. | Prove actual qualified source and imported command slots before mutation on hosted surfaces; then capture call, origin and forget independently. Add export-list change followed by existing import versus new import if lifecycle coverage is intended. The isolated global-origin result is a child-interpreter scope. |
| F5-N08 | Target rebinding/global anchor, describe, destination parent creation, child owner and delete. | Repair hosted rebinding fixture; add duplicate fixed-prefix words with exact argv capture and alias describe/delete before and after target mutation. Child-owner evidence must remain explicitly child-interpreter evidence. |
| F5-N09 | Procedure rename, command rename trace, isolated unknown and namespace-unknown attempts. Native ordinary-head/catalogue/rename controls are separate. | Correct hosted qualified procedure/trace fixtures; retain repeated call before/after callback mutation, rename and delete trace operations with callback argv. Namespace-unknown capability failures cannot substitute for a successful handler. |
| F5-N10 | Namespace-code deleted/recreated and active delete/recreate witnesses. | Active hosted fixture is blocked by original definition ownership. Add a warmed/compiled descendant invoked from an entered frame across deletion/recreation with distinct old/new cells. Path, OO and trace token lifecycle need independent capability and lifecycle controls; unsupported boundaries are legitimate answers, not inferred execution. |
| F5-N11 | app3 owner routes; canonical folders/partitions; persist1 computed ordinary head versus native call, six empty catalogue entries, computed rename failure with old native route still callable. | The new native runs measure routes, matching arguments and accepted/rejected/detached/deleted/recreated phases in their actual contexts. Missing native1 init captures remain explicit; native2 supplies a separate OLD/NEW/rejected provider identity. Literal ordinary-head rejection and computed ordinary-head runtime error need separate exact loader/runtime receipts. This does not require substituting dynamic Tcl procs for native definitions. |
| F5-N12 | Same-tail HTTP shadow, absolute and computed HTTP::header under the reached HTTP profile/event; errors on other surfaces. | Retain the same native operation in an event without HTTP availability/profile, and compare namespace existence/catalogue against protocol invocation. The profile/event boundary cannot be established from a hosted interpreter error alone. |
| F5-N13 | Four RULE_INIT seeds; dynamic definition persists; namespace/regular-global/static reads in procedure and event; 27 actual HTTP responses including paired connection isolation/write/unset/recreate. Canonical static mutation isolates unit `0:2`. | The new native runs measure provider-side shared-cell writes and caller-local native links in RULE_INIT and actual HTTP scope. Ordinary-global/namespace writes on another actual traffic worker remain unmeasured; either route controlled traffic to another worker or state that limit explicitly. |
| F5-N14 | Local/replaced expr mathfunc attempts and OO-forward capability attempts. | First prove explicit qualified mathfunc owner and actual expr extensibility, including global/local shadow and replacement if supported. The hosted unqualified-definition fixture cannot distinguish unsupported mathfunc lookup from missing intended function. Independently measured absence of actual TclOO can answer object-method boundaries. |
| F5-N15 | Missing command/variable/namespace, forbidden link, rename/import conflict errors with complete result/errorcode/errorinfo; catch-options capability. | Repair forbidden-link and import-conflict fixtures so the intended owned cells/commands demonstrably exist; catch each operation individually. Full return options stay explicitly unavailable where the third catch argument is unsupported. |
| F5-N16 | Braced/quoted/substituted/list heads, space-joined near miss, brace-newline and display roundtrip attempts. | Separate display-slot construction from roundtrip; retain exact displayed bytes and invoke them as one list word. Original corpus and adapted lexical witnesses remain separate inputs. |

## Real harness boundaries

The 120 hosted Wiki rows fail in real package initialisation while reading
`debug`; 90 TMM Wiki rows fail because `tcltest` is absent. The seven hosted
`tcltest-shared-lifecycle` reductions have the corresponding earlier package
boundary. The [diagnostic controls](../../patterns/f5/answers/nh20261012repair2/README.md)
and [Chrome APL controls](../../patterns/f5/answers/nh20261012aplrepair1/README.md)
retain those native-parent failures. A standard fresh hosted child can load
the real package; that child result does not answer a native-parent row.

The explicitly sourced unchanged package in a separate TMM child fails at
`pwd`, as retained in [harness2](../../patterns/f5/answers/nh20261012harness2/README.md).
The earlier oversized capture attempt remains in
[harness1](../../patterns/f5/answers/nh20261012harness1/README.md).
No modified or replacement harness is used to promote an original answer.
The complete source, input hashes, failures and capture limits are retained.

Eight aggregate stock Wiki rows have genuine constraint skips: code-7's
`noAunt` condition and seven OO cases' `tcloo` condition. Both actual stock
versions and constraints are reviewed in the
[constraint-boundary review](../../patterns/f5/answers/portable-constraint-boundary-review.md).
The separate stock-8.4 prefix-2 skip does not erase its reached stock-8.5
answer. Constraints are not forced to obtain a different original result.

The remaining original failures and collisions have independent
[source reachability review](../../patterns/f5/answers/portable-unavailable-review.md)
and [collision-avoiding controls](../../patterns/f5/answers/nh20261012collision1/README.md).
Eleven adapted controls reach their final operation; the adapted OO case
still stops at class creation. Their changed spellings do not fill original
matrix answer slots.
