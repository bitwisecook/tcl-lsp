# Draft: request for original #2286 result provenance

**Do not send until reviewed.** This is a separate BIG-IP evidence-provenance question, not a request to rerun tests or accept current proof records.

The retained `r2286q` and `r2286s` results contain six result fields whose current hex decodes match their embedded SHA-256 and byte length. The current sanitized containers and report have different whole-file bytes from the historical capture identities. The transport note says Tcl operation/status/result fields were preserved while transport/device values were projected, but it does not provide an independent prior digest of these six fields. A hash of the current field or report cannot authenticate the historical field.

Could you provide either:

1. the exact pre-projection `decoded-results.json` (SHA-256 `ee533a62089293ff9c10075994ee3fc81fb01402ca978836a95b2a3dff368426`, 1,711,344 bytes) and `CONTEXT_NAMING_RESULTS.md` (SHA-256 `c19780153d8f494bc30bda0f4db8036acc9f3e94f7b210be688f0875c24ec1f7`, 31,721 bytes), transferred as exact bytes; or
2. an independently retained, pre-projection record that binds each selector below to its exact original operation-result bytes (or SHA-256 and byte count), and identifies the capture/run and source of that record.

| Run / selector | Original input-payload SHA-256 | Reported result SHA-256 | Result bytes |
| --- | --- | --- | ---: |
| r2286q `nul_counted_last` | `c6d687f237c239600f5bd1cf76e16d371f584eea315f1338fceecb14149a64e9` | `85e71e85003bea2934299cf3e46e90a2b4e57e20c9549c8855cc37bc8ffb7791` | 299 |
| r2286q `nul_array_root` | `1a942ef715ac1b9655a3c34098864fa1f87a9feeb129d9afd95dba545de75fd0` | `9fd64024074015b88b07c7f74f57a5fbeec34a68a10fc2e4bfb9cf688539d010` | 490 |
| r2286q `nul_array_index` | `8e0c4f79d9213ef60f1b8725af64b25ef1cad809a06aa99643290dd04a1d7cf6` | `d211a7cd9096ddee008b44997a4fe46c54a2255ec0f0e1171e513524e6ca548a` | 503 |
| r2286q `lexical_clean` | `7a55589049588f9bb5ddbe36b944c41b2294374942c348a969dfa56e65db543f` | `2508dc8a3f13c4d8401bb3d7564de443dd64cf4233354a96e37f0fbd007d7128` | 1,843 |
| r2286s `nul_array_root_portable` | `8fa234686f7534a4961585b9138d994f1d2016a8a81f6c456bb1f0abd8bc1654` | `bdc8a515be08639361b95f5ca92bdfbb8647121baecab7e6739a85a269d29610` | 1,357 |
| r2286s `nul_array_index_portable` | `27444ad7e4c3ffa8e72c8655f4b562a53fc362982b2ae3b41bf35297fc6b014d` | `8f17dec9408761ab1945644e8a2e0cb050360eb34a28b810141bba8fce5f225f` | 1,612 |

For an original-artifact response, please include the bytes without editorial conversion and a transfer-side SHA-256/length check. For a digest-only response, please identify when and where the digest was computed, whether it was recorded before sanitization, and how the record binds the digest to the exact run, selector, and result bytes. Please preserve the original `r2286q` and `r2286s` fixture/source payload identities and the execution context (BIG-IP version/build, TMM versus any other context, and phase) alongside the record. The existing documentation distinguishes TMM iRules, `tmsh` scripts, iApp implementation/presentation and iCall; these must not be conflated.

A fresh replay may provide new evidence for the same selectors, but it cannot restore the identity of a historical capture. Please label any fresh replay with a new run ID and retain it separately. If neither the original bytes nor an independently retained pre-projection field digest exists, please confirm that limitation; we will keep the historical whole-file IDs and mark the old field-level identity unverified.

## Existing local references checked

- `scripts/dev/bigip-probes/resolution-2286/CONTEXT_NAMING_RESULTS.md` — published `r2286q` and `r2286s` result tables and original whole-report hash/size.
- `scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006naming/decoded-results.json` — current sanitized decoded fields; current whole-file hash/size differs from the historical identity.
- `scripts/dev/bigip-probes/resolution-2286/evidence/TRANSPORT_PROJECTION.md` and `CURRENT_TRANSPORT_SHA256SUMS` — projection scope and hashes for current projected bytes.
- `scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006naming/r2286q-appliance-evidence.tgz`, `r2286q-dev-evidence.tgz`, and corresponding `r2286s-*` archives — captured artifact families; not independently verified as containing the two exact pre-projection files, and not inspected/extracted for this question.
- `experiments/name-handling/results/owner-adoption-20261016/reviewed-source-successor-889/projection-identity/naming-proof-native-result-field-bridge.json` — prior bounded review; records the six field mappings and explicitly lists original raw script/HTTP-envelope bytes as unavailable.
- `experiments/name-handling/results/owner-adoption-20261016/reviewed-source-successor-889/projection-identity/naming-proof-native-result-field-bridge-schema-candidate.json` — earlier schema proposal; superseded for historical-identity purposes. It must not be used to admit the current projection as historical field proof absent the requested independent original grounding.
