# Retired namespace Runtime validation

One fresh standalone Runtime14 library-test build passed. Each of the three exact independent controls then ran once from the same binary and failed its original assertion (exit 101). No source correction, expected-output change, adaptation or retry occurred. The Runtime cache and build lease were released after the third control.

`validation.json` records exact commands, artifact freshness, binary SHA, actual source hashes, conservative source inventory and decoded assertion bytes. `sealed-files.json` seals 26 immutable capture files, including all three original raw logs and their six local byte-vector decodes. This README is a separate derived annotation and is outside that seal. The original 46-file C/Jim question and 20-file passive route capture remain unchanged.

| Control | Actual reached outcome | Retained C86 expectation |
| --- | --- | --- |
| Source entries (five rows) | Literal scalar, literal element and original sequence output match; dynamic scalar and dynamic element retain seeded `NH UNSET` despite the expected guest error message and completion | Dynamic scalar publishes `TCL WRITE VARNAME`; dynamic element publishes three-word `TCL LOOKUP VARNAME` |
| Variable heads (two rows) | Scalar matches; element publishes `TCL WRITE VARNAME` | Element publishes four-word `TCL LOOKUP VARNAME foo` |
| Genuine original-object vector | Guest code and message match; actual parsed-root local-index assertion passes; errorCode comparison fails with `TCL WRITE VARNAME` | Four-word `TCL LOOKUP VARNAME foo` |

The first two controls pass guest completion and no-host-refusal assertions, then fail whole-output assertions after evaluating their unchanged original scripts. Their later allocation-retirement checks are unreached. The third reaches and passes its original-object parsing and real local-index assertions, then compares completion and message successfully before failing errorCode; its final no-refusal assertion is unreached. These failures are guest diagnostic metadata differences, not evidence of Runtime conformance. No full original orchestration was repeated.

Raw logs: `01-retired_namespace_c86_source_entries_preserve_scalar_and_element_sites.log`, `02-retired_namespace_c86_variable_heads_preserve_original_root_tuple.log`, and `03-retired_namespace_original_object_vector_keeps_root_despite_compiled_cell.log`. The corresponding `*-actual-assertion-bytes.txt` and `*-expected-assertion-bytes.txt` decode only the existing Rust numeric assertion vectors; they are not new executions.

The independently run Syntax policy controls pass (parent-reported). That narrow policy result does not resolve the Runtime caller-site selection or metadata publication gaps. Follow-up instrumentation remains scratch-only until separately authorised.
