# Hosted original-source evidence review

The accepted rows below retain byte-identical original corpus programs in the actual CLI, iApp implementation, iCall, APL and TMM host wrappers. They answer the recorded final operation only. Dynamic ASCII-hex decoding/evaluation is the retained input channel; this does not supply a literal loader answer. The planned original namespace/procedure names were guarded absent before execution. Configuration objects were removed and planned runtime-state checks were captured. These checks do not locate possible namespace-definition fallback owners or prove total interpreter cleanup; [cleanup bounds](original-hosted-cleanup-review.md) retain that limitation. No fresh-interpreter premise is asserted for hosted contexts.

The non-TMM review accepted nine cases per context: four successful complete original programs and five source-reviewed final-operation errors. APL observations came from actual browser-selected callback option values; iApp implementation observations are separate. The TMM review accepted four cases in `RULE_INIT` and nine each in native procedure and traffic contexts. A primary pointer identifies one actual unit/invocation; other units and invocations remain independent records in the run reports.

For each promoted pointer this review checked the original question/canonical identity, source SHA-256, complete operation bytes in case metadata, the hex-encoded operation in actual input, metadata/wrapper/configuration hashes, provider/load/runtime/cleanup hashes, complete result fields, context and reached final operation. Source-reviewed code1 receipts additionally link the immutable reachability review: the authored final expression was entered, including substitutions; this does not assert outer-command dispatch after a substitution failure. Setup completion means the authored prefix completed, rather than proving every expected binding was created. Over 7,900 retained file descriptors were hash-checked across these reviews.

| Run | Accepted case/context rows | Complete measurement records | Source-derived final-error review |
| --- | --- | --- | --- |
| `nh20261012exact1` | 27 | [records](nh20261012exact1/records.json), [report](nh20261012exact1/ANSWERS.md) | [review](nh20261012exact1/reviewed-reachability.json) |
| `nh20261012aplexact1` | 9 | [records](nh20261012aplexact1/records.json), [report](nh20261012aplexact1/ANSWERS.md) | [review](nh20261012aplexact1/reviewed-reachability.json) |
| `nh20261012exact2` | 22 | [records](nh20261012exact2/records.json), [report](nh20261012exact2/ANSWERS.md) | [review](nh20261012exact2/reviewed-reachability.json) |

| Original case | Context | Primary JSON pointer | Completion | Result bytes (hex) |
| --- | --- | --- | --- | --- |
| `alias-prefix-lifetime` | `iapp-implementation` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/0` | 1 | `696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a656d697422` |
| `computed-qualified` | `iapp-implementation` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/2` | 1 | `696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a656d697422` |
| `factory-body-near-miss` | `iapp-implementation` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/4` | 1 | `696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a72656422` |
| `finite-factory` | `iapp-implementation` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/5` | 1 | `696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a72656422` |
| `frame-wrapper` | `iapp-implementation` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/6` | 0 | `6368616e676564` |
| `frame-wrapper-near-miss` | `iapp-implementation` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/7` | 0 | `6f726967696e616c` |
| `list-prefix` | `iapp-implementation` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/9` | 0 | `7b6120627d207b6320647d` |
| `namespace-captured-prefix` | `iapp-implementation` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/10` | 0 | `676c6f62616c` |
| `string-prefix-near-miss` | `iapp-implementation` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/13` | 1 | `696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f22` |
| `alias-prefix-lifetime` | `icall` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/45` | 1 | `696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a656d697422` |
| `computed-qualified` | `icall` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/47` | 1 | `696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a656d697422` |
| `factory-body-near-miss` | `icall` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/49` | 1 | `696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a72656422` |
| `finite-factory` | `icall` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/50` | 1 | `696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a72656422` |
| `frame-wrapper` | `icall` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/51` | 0 | `6368616e676564` |
| `frame-wrapper-near-miss` | `icall` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/52` | 0 | `6f726967696e616c` |
| `list-prefix` | `icall` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/54` | 0 | `7b6120627d207b6320647d` |
| `namespace-captured-prefix` | `icall` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/55` | 0 | `676c6f62616c` |
| `string-prefix-near-miss` | `icall` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/58` | 1 | `696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f22` |
| `alias-prefix-lifetime` | `tmsh-cli-script` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/90` | 1 | `696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a656d697422` |
| `computed-qualified` | `tmsh-cli-script` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/92` | 1 | `696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a656d697422` |
| `factory-body-near-miss` | `tmsh-cli-script` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/94` | 1 | `696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a72656422` |
| `finite-factory` | `tmsh-cli-script` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/95` | 1 | `696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a72656422` |
| `frame-wrapper` | `tmsh-cli-script` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/96` | 0 | `6368616e676564` |
| `frame-wrapper-near-miss` | `tmsh-cli-script` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/97` | 0 | `6f726967696e616c` |
| `list-prefix` | `tmsh-cli-script` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/99` | 0 | `7b6120627d207b6320647d` |
| `namespace-captured-prefix` | `tmsh-cli-script` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/100` | 0 | `676c6f62616c` |
| `string-prefix-near-miss` | `tmsh-cli-script` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/103` | 1 | `696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f22` |
| `alias-prefix-lifetime` | `iapp-apl` | [nh20261012aplexact1/records.json](nh20261012aplexact1/records.json) `/0` | 1 | `696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a656d697422` |
| `computed-qualified` | `iapp-apl` | [nh20261012aplexact1/records.json](nh20261012aplexact1/records.json) `/2` | 1 | `696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a656d697422` |
| `factory-body-near-miss` | `iapp-apl` | [nh20261012aplexact1/records.json](nh20261012aplexact1/records.json) `/4` | 1 | `696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a72656422` |
| `finite-factory` | `iapp-apl` | [nh20261012aplexact1/records.json](nh20261012aplexact1/records.json) `/5` | 1 | `696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a72656422` |
| `frame-wrapper` | `iapp-apl` | [nh20261012aplexact1/records.json](nh20261012aplexact1/records.json) `/6` | 0 | `6368616e676564` |
| `frame-wrapper-near-miss` | `iapp-apl` | [nh20261012aplexact1/records.json](nh20261012aplexact1/records.json) `/7` | 0 | `6f726967696e616c` |
| `list-prefix` | `iapp-apl` | [nh20261012aplexact1/records.json](nh20261012aplexact1/records.json) `/9` | 0 | `7b6120627d207b6320647d` |
| `namespace-captured-prefix` | `iapp-apl` | [nh20261012aplexact1/records.json](nh20261012aplexact1/records.json) `/10` | 0 | `676c6f62616c` |
| `string-prefix-near-miss` | `iapp-apl` | [nh20261012aplexact1/records.json](nh20261012aplexact1/records.json) `/13` | 1 | `696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f22` |
| `factory-body-near-miss` | `tmm-RULE_INIT` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/16` | 1 | `63616e2774207265616420226e616d65223a206e6f2073756368207661726961626c65` |
| `finite-factory` | `tmm-RULE_INIT` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/20` | 0 | `72656420626c7565` |
| `keyed-variable-link` | `tmm-RULE_INIT` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/32` | 0 | `6368616e676564` |
| `namespace-captured-prefix` | `tmm-RULE_INIT` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/40` | 1 | `63616e2774207265616420223a3a6e683a3a707265666978223a206e6f2073756368207661726961626c65` |
| `alias-prefix-lifetime` | `tmm-procedure` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/180` | 0 | `7b3a3a6e683a3a656d6974207b6120627d7d207b7b6120627d207b6320647d7d` |
| `computed-binding-mutation` | `tmm-procedure` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/184` | 0 | `7265706c6163656d656e74` |
| `computed-qualified` | `tmm-procedure` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/188` | 0 | `6669727374` |
| `factory-body-near-miss` | `tmm-procedure` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/196` | 1 | `63616e2774207265616420226e616d65223a206e6f2073756368207661726961626c65` |
| `finite-factory` | `tmm-procedure` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/200` | 0 | `72656420626c7565` |
| `keyed-variable-link` | `tmm-procedure` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/212` | 0 | `6368616e676564` |
| `list-prefix` | `tmm-procedure` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/216` | 0 | `7b6120627d207b6320647d` |
| `namespace-captured-prefix` | `tmm-procedure` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/220` | 1 | `63616e2774207265616420223a3a6e683a3a707265666978223a206e6f2073756368207661726961626c65` |
| `string-prefix-near-miss` | `tmm-procedure` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/232` | 1 | `696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f22` |
| `alias-prefix-lifetime` | `tmm-traffic-event` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/360` | 0 | `7b3a3a6e683a3a656d6974207b6120627d7d207b7b6120627d207b6320647d7d` |
| `computed-binding-mutation` | `tmm-traffic-event` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/376` | 0 | `7265706c6163656d656e74` |
| `computed-qualified` | `tmm-traffic-event` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/392` | 0 | `6669727374` |
| `factory-body-near-miss` | `tmm-traffic-event` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/424` | 1 | `63616e2774207265616420226e616d65223a206e6f2073756368207661726961626c65` |
| `finite-factory` | `tmm-traffic-event` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/440` | 0 | `72656420626c7565` |
| `keyed-variable-link` | `tmm-traffic-event` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/488` | 0 | `6368616e676564` |
| `list-prefix` | `tmm-traffic-event` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/504` | 0 | `7b6120627d207b6320647d` |
| `namespace-captured-prefix` | `tmm-traffic-event` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/520` | 1 | `63616e2774207265616420223a3a6e683a3a707265666978223a206e6f2073756368207661726961626c65` |
| `string-prefix-near-miss` | `tmm-traffic-event` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/568` | 1 | `696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f22` |

The [current matrix](../questions.json) carries SHA-256 anchored pointers. Accepted proof documents remain indexed in the canonical [proof collection](../../../../../docs/design/analysis/name-resolution-proofs/README.md); this answer document neither replaces that authority nor promotes adapted controls.

## Measured attempts

A matrix `attempt_record` uses the same path/SHA-256/JSON-pointer descriptor as `answer_record`, but `answer_record` remains null. This additive field is a navigation pointer to measured guard, setup, or whole-program evidence. It grants no canonical final-lookup answer. A guard/package failure or known earlier authored failure is `unreached`; a complete original-program error whose final-operation reach is unproved is `unavailable`. Load status remains independent.

Thirty original Wiki selections in each hosted context failed before the intended body. Their complete real suite and exact guards are retained; no extracted Tcltest substitute or harness PASS is presented as body bytes. Other whole-program errors keep their narrower scope. Do not infer semantic unsupported status from a generic code1 result.

The separate [stock original-source review](stock-original-source-review.md) and [stock Wiki review](stock-wiki-review.md) retain stock interpreter outcomes. Wiki execution-leave observation is explicitly instrumented, with unchanged original bodies and real harness; unavailable inner child error options remain unavailable.

Provider receipts identify BIG-IP 21.1.0.1 build 0.0.26, Edition Point Release 1; no explicit Hotfix value was observed. Edition is preserved separately. Each run records unsaved configuration and config-object cleanup. Runtime name cleanup and activated native-catalog retirement are distinct claims.
