# Current F5 answer acceptance review

Reviewed Git object: `755f78207b993cdfa4b0fd230abcafb1f3f606bc` (fetched work); no checkout, source/proof mutation, build, oracle execution, registration or question-status edit.

Scope: exactly the two `nh20261012coverage2/answers.md` projections, their coverage JSON/input digests, original question/context identities and linked capture/seal receipts:

- `experiments/name-handling/patterns/f5/answers/nh20261012coverage2/answers.md`
- `experiments/name-handling/f5/answers/nh20261012coverage2/answers.md`

Admission: the retained original matrix is internally consistent and checkable at this object. It has 50 questions × 8 contexts = 400 distinct rows: 93 observed, 244 unreached, 63 unavailable. Both coverage JSON blobs are byte-identical; every original_matrix equals its question/context entry in `patterns/f5/questions.json` (400/400). All 93 admitted answer pointers match record question, original-source identity and context, with stock-8.4/8.5 kept distinct beneath the stock bundle context. The other 307 rows remain measured attempts without original answers.

Integrity verification: 30 actual MANIFEST.sha256 files; all 26,097 listed blob hashes verified from Git, zero missing/mismatching entries. This includes both three-entry coverage seals. All 26 coverage input digests verified. All 1,665 unique path/hash references reached from the 93 admitted records plus the 16 authored original-source entries verified. Coverage read_problems is empty. The compact companion JSON retains every seal path and entry count, rather than a claim that 30 campaigns completed.

Report inventory: 23 input reports, 5,317 measurement records, and 16 broad F5-N questions. Counts project retained units/invocations; broad receipt totals overlap original-pattern receipts and are not additional canonical answers. The 30 seals also include capture/cleanup/repair/validation artefacts outside those 23 report inputs. A capture-complete timestamp is a capture receipt, not proof that every lookup or cleanup completed; interrupted attempts remain retained.

For every admitted record (93/93), original/actual input and wrapper, BIG-IP version/build, invocation evidence/raw output channel, explicit intended+captured lookup reachability and complete result capture survive. Retained release is BIG-IP 21.1.0.1 build 0.0.26; APL patchlevel 8.4.6 is retained where recorded. Provider patchlevel/namespace omissions remain null, not inferred. Reached events remain RULE_INIT for TMM init/procedure and HTTP_REQUEST for these traffic pointers. Hosted/stock/APL contexts retain their invocation scripts and output channels rather than acquiring a TMM event. Full independent inventory remains in coverage.json and records.json.

Five bounded original observed patterns, all in `patterns/f5/answers/nh20261012exact2/records.json`, TMM procedure, RULE_INIT, unit 0:0, invocation once:
| Original case | Pointer | Completion | Retained result |
| --- | --- | --- | --- |
| alias-prefix-lifetime | /180 | 0 | `{::nh::emit {a b}} {{a b} {c d}}` |
| computed-binding-mutation | /184 | 0 | `replacement` |
| computed-qualified | /188 | 0 | `first` |
| factory-body-near-miss | /196 | 1 | `can't read "name": no such variable` |
| finite-factory | /200 | 0 | `red blue` |

These admit only the measured final operation/context. The hosted original alias/qualified/factory receipts also retain independent code-1 results (for example invalid command `::nh::emit`/`::nh::red`), not the TMM result. A source-reviewed final operation includes substitutions; it does not prove outer dispatch after a substitution error or promise that successful prefix evaluation created every desired binding. Adapted stage-split, ownership-guard, whole-program, constraint-skip and loader-only controls remain separate.

Critical limits:

- Entered suspended frames, compiled descendants/identity after deletion, and restart/reload after deletion remain unmeasured. F5-N10/N13 are bounded variants, not full closures.
- The traffic pointers preserve reached HTTP_REQUEST units/invocations; other traffic events/profiles, restart state, all flow/TMM combinations and unmeasured native route variants need their own receipts. Four initial TMM units do not establish four actual traffic units.
- F5-N13 retains 101 observed storage receipts; persist1 has 287 records (186 F5-N11, 101 F5-N13), a capture-complete receipt with configuration_saved=false, owned-cell and object-absence checks. This does not prove universal cleanup of earlier campaign state. Ownership collisions/setup guards leave original lookup unreached. Exact owned cleanup, same/fresh connections and cross-rule controls must keep their independent provenance.
- Unavailable command/context/options and unreached lookup are distinct. Error options omitted by older capture channels and child harnesses stay unavailable; observed errors are still observed final-operation completions. Generic record limitations retain a V1 warning even on records with identified invocation capture fields; use the actual per-field metadata.
- The answer Markdown embeds author-machine `/Users/jimd/src/tcl-lsp/...` links. The committed paths and hashes are checkable via Git, but those absolute links are not portable in this workspace. No missing sealed blobs were found.

No integrity or context-identity blocker to accepting the retained scoped receipts. No new broad closure, cleanup guarantee, canonical registration or original-question promotion is justified by this projection alone. Root owns integration and issue/tracking updates; this is temporary current review material.
