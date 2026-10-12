# BIG-IP naming verification

Verify the appliance-specific variable and lexical naming contracts needed by
Tcl compiler and language-server consumers. Execute the two requests below on
an actual BIG-IP. Portable C Tcl and Jim results do not establish appliance
semantics. Keep measurements separated by execution context and actual TMM.

Use branch **`work`**. Record the exact source commit and actual BIG-IP product,
version, build, point release and hotfix at the beginning of each report. The
reference reports describe BIG-IP 21.1.0.1 build 0.0.26, Point Release 1; verify
this run independently. Read [BIGIP_RESULTS.md](BIGIP_RESULTS.md),
[FOLLOWUP_APPLIANCE_RESULTS.md](FOLLOWUP_APPLIANCE_RESULTS.md) and
[CONTEXT_NAMING_RESULTS.md](CONTEXT_NAMING_RESULTS.md) for measured coverage.

## Requests requiring response files

| Complete source and requirements | Required peer report |
| --- | --- |
| [VARIABLE_LIFECYCLE_CHECKS.md](VARIABLE_LIFECYCLE_CHECKS.md) | `VARIABLE_LIFECYCLE_RESULTS.md` |
| [LEXICAL_BYTE_CLASS_CHECKS.md](LEXICAL_BYTE_CLASS_CHECKS.md) | `LEXICAL_BYTE_CLASS_RESULTS.md` |

The lexical results template is not an executed response. Verify whether either
peer report exists on the fetched branch before running its request. Preserve
complete existing reports and distinguish additional measurements explicitly.

The variable request includes its complete HTTP iRule and additional controls:
read callbacks, unset and recreation, original output-assignment operands,
arrays, aliases, real `upvar 0` versus `upvar #0` frames, and `static::` storage.
Measure RULE_INIT initialization on each reached TMM and event-local mutations
on the actual executing TMM. Report global versus connection/event storage and
unsupported contexts separately; successful creation alone proves no broadcast.

The lexical request contains the complete payload, complete three-event iRule
and non-TMM requirements. Each execution comprises **1,044 rows**: 261 inputs,
two source producers, and braced/unbraced substitution. Preserve all 256 binary
singletons and the five explicitly labelled non-ASCII producer controls.
`format %c 233` and `format %c 769` are distinct from binary `e9`, `c3a9` and
`cc81`. Preserve literal backslashes, braces, dollar signs, physical line breaks,
continuations and escapes exactly as specified. The source wrappers are ASCII
with LF line endings; generated binary operands include NUL and non-ASCII
bytes. Never replace them with text-normalized approximations. Extract the full
code blocks byte-for-byte, retain source dumps and SHA-256 hashes before and
after transfer, and hash any separately adapted wrapper. Follow the original
getter/inspection order so diagnostics do not prime the tested source object.

## Traffic and collection

Create clients and servers on **dev.bragi0.com**, including loopback testing,
using any suitable protocol. Every test BIG-IP virtual server must use
**SNAT automap**. The supplied HTTP wrappers require TCP and HTTP profiles;
another protocol needs a separately hashed event-valid wrapper that retains the
original tested operands. Prove the real backend path with complete client
request/response bytes and the server's received peer address and run marker.
An iRule-generated response alone does not prove backend delivery.

Record owned VIP/pool/member/profiles, CMP configuration, the actual TMM roster
and every reached event identity. Capture `/var/log/ltm` before load through
traffic and cleanup. Select lightweight connection tuples before the heavy
lexical rule, then follow its finite sequential connection budget. Do not
substitute a configured CPU count or repeated requests for measured TMM
coverage. Preserve complete log chunk sets and raw HTTP bodies; missing,
conflicting or truncated chunks leave the corresponding observation unknown.
Keep load rejections, setup failures, unsupported commands and runtime failures
as separate outcomes instead of silently changing the probe.

Use fresh owned names, verify their absence before creating them and remove
only objects and clients/servers created for this run. Preserve cleanup and
object-absence evidence. Do not restart TMM, alter global CMP or save global
configuration. Collect only compiler-relevant run evidence; omit credentials,
secrets and unrelated configuration or security-sensitive log content.

## Deliverables

Write both peer reports beside their request files on **the same `work` branch**.
Store complete executed source, raw per-context status/stdout/stderr or HTTP/LTM
capture, decoded rows and a checksummed evidence manifest in a run-specific
subdirectory of `evidence/`. Record source commit, exact appliance version,
source/response hashes, context, TMM, row/chunk completeness, deviations and
unsupported or unreached cases. Keep raw bytes and interpretation distinct.
Report only measured behavior; script output supplies no native object-header,
ABI, cache or compiler-hook proof.

Commit and push the reports and evidence to `work`. Return the report commit,
links to both peer reports and evidence, archive SHA-256 if used, exact appliance
version, actual TMM/context coverage and any remaining unknowns. Coordinate
concurrent branch updates normally without overwriting other commits.
