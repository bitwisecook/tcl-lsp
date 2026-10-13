# What security material was removed, and how can the measurements be reproduced?

The scrub covers the current `work` files, including older evidence, source
snapshots, compressed archives, encoded captures, and retained binary inputs.
The initial complete scan covered 84,194 files; the post-redaction scan covered
84,198 files and 5,180,953,199 source bytes. Both reported zero expansion or
scan errors. A separate DER/base64 inspection covered all 84,188 tracked files.
Candidate locations and categories are in [scan-review.json](scan-review.json);
no credential values, snippets, or credential-value hashes are included.

## Redactions and retained boundaries

| Location | Material removed | What remains and how to reproduce |
| --- | --- | --- |
| `rust/tcl-sslictcl/data/raw/pem/*.pem` | All seven vendor certificate bundles: 1,039 certificate occurrences | Explicit placeholders; pinned membership and issuer metadata; [vendor retrieval and OpenSSL verification](../../../../rust/tcl-sslictcl/data/README.md) |
| `rust/tcl-sslictcl/data/raw/chrome_root_store/root_store.certs` | 104 further certificate occurrences | Placeholder and pinned Chrome policy metadata; same vendor recipe |
| `rust/tcl-sslictcl/data/raw/trust_stores_as_pem.tar.gz` | Original certificate archive | Deterministic archive containing only an omission notice |
| `rust/tcl-sslictcl/data/generated/trust-seed.json` | 398 stored DER certificates and associated certificate material | 629 membership records, each with absent material; 629 explicit material exceptions |
| `rust/tcl-bigip-query/tests/fixtures/x509_example.pem` and `probes.json` | Certificate and public-key bodies | Placeholder and stable metadata; [fresh certificate creation, key comparison and verification](../../../../rust/tcl-bigip-query/tests/fixtures/certificate-generation.md) |
| `rust/bigip-report-gen/python/tests/data/certs-chain.ucs` | Three certificates and one private key in filestore members | Configuration and named placeholders; [exact OpenSSL generator and verification boundaries](ucs-certificate-generation.md) |
| `rust/f5-cli/tests/fixtures/ucs-cert-sample.ucs` | One filestore certificate | Configuration and placeholder; same UCS recipe |
| `rust/tcl-registry/tests/data/native_wasi_libc_environment_original/selected-inputs/wasmtime-49.0.2/wasmtime.gz` | Executable containing 160 public trust-anchor key objects | Compressed redaction notice; [local tool retrieval and identity verification](../../../../rust/tcl-registry/tests/data/native_wasi_libc_environment_original/selected-inputs/wasmtime-49.0.2/REDACTION.md) |

These counts include duplicate vendor roots across stores; they are not a
count of distinct root authorities. The updater fetches membership and policy
metadata only, the generator omits material, and the runtime cannot silently
hydrate roots from committed certificate bundles. Callers must supply local
certificate material for chain completion.

The Wasmtime file is no longer an executable input. Canonical retained-byte
descriptors, the storage mapping, and the two affected proof documents identify
that boundary. Historical executed-input hashes remain historical; they were
not relabelled as hashes of newly generated or redacted bytes. Unchanged Tcl
inputs and observed oracle result statuses remain independent evidence.

## Credential review

No known current lab credential, provider-token prefix, JWT, or F5 encrypted
secret was found by the comparison and format checks. Credential-like matches
were reviewed as source identifiers, command/field documentation, variable
references, explicit projection placeholders, synthetic parser/security
fixtures, or public algorithm conformance vectors. Those syntax and algorithm
fixtures do not configure a live account. Invalid PEM examples used to test
parsing are format fixtures, not certificates or private-key bodies.

The six remaining DER public-key candidates are in two retained copies of
the OpenSSL library. Their three starts are 32 bytes apart and their claimed
DER ranges overlap. This suggests serialization prefixes followed by adjacent
library data, rather than independently stored keys; exact upstream constant
provenance was not established. This interpretation is an inference, with the
measured ranges in [binary candidate review](binary-candidate-review.json).
They are not certificate or private-key objects. The 160 public
trust-anchor candidates in Wasmtime were removed. The final DER scan reported
zero certificate/private-key candidates and zero scan errors.

The earlier F5 transport projection remains documented in
[F5 sanitisation evidence](../f5-evidence-sanitisation/README.md). Readable
passwords, provider-issued API keys, SSH keys, hardware credentials, and browser
passkeys have [creation and fresh verification procedures](material-generation.md).
Actual values, session cookies, credential identifiers, recovery data, and key
bodies must stay outside evidence.

## Repeating the review

`scan.py` scans tracked files and nonignored untracked files, recursively
expanding gzip, bzip2, xz, tar, and zip inputs, and checking textual hex/base64
projections. Its depth limit is three, archive member limit is 256 MiB, and
encoded token limit is 4 MiB. Binary inputs receive the strong credential and
PEM-marker checks. `scan-der.py` uses the local Python `cryptography` package
to validate candidate DER/base64 certificates and keys. Both write only
locations/categories to private `/tmp` review files:

```sh
python3 experiments/name-handling/results/security-material-scrub/scan.py
python3 experiments/name-handling/results/security-material-scrub/scan-der.py
```

For exact comparisons, set `TCL_SCRUB_COMPARISON_FILE` to a private local file
containing one credential per line. The file must stay outside the checkout.
The scan also checks those values as hex and base64. Do not copy that file or
its contents into the report. A regex candidate requires contextual review;
a zero count does not prove absence of unknown encodings or formats.

The review covers the current retained tree, rather than rewriting Git
history. It does not perform OCR of images, decode arbitrary proprietary
formats, or assert that synthetic test vectors authenticate anywhere.

## Validation

See [verification.json](verification.json) for exact checks, results, and
remaining gates. OpenSSL generation, key checks, key/certificate correspondence,
valid single-certificate verification, chain signature verification with time
ignored, and expected expired-leaf rejection all passed. The private generated
material was removed after each run.

The first test build and later full source-data CLI link hit ENOSPC. Generated
build outputs were cleaned before retrying; subsequent focused builds use
optimisation and no debug information to limit space. Full Rust/PR gates and
existing naming-proof implementation-digest repairs remain outstanding under
the explicitly authorised development-push exception on `work`. The Python
report test collection also needs its compiled PyO3 `_engine` module; its
absence is recorded separately from the verified standalone UCS generator.

The report certificate test file finishes with 15 passes and two failures in
unchanged orphan-reachability assertions (`constrained_pattern_filters_orphans_by_name`
and `orphan_reachability_is_partition_aware`). Both tests using the generated
certificate UCS pass. A pre-scrub baseline for those two failing assertions was
not measured; their failures remain explicit, without a global green claim.

The final Clippy attempt remains blocked by `assert_is_empty` errors on
unchanged assertions in the selected test targets. The new assertion and
iteration warnings introduced during the scrub were corrected. No lint
allowances were added. [Descriptor verification](descriptor-verification.json)
confirms that only two of 2,606 canonical records change retention descriptions
and scope; observed results remain unchanged, and three retained-byte hashes
match their current artifacts. The [focused final scan](focused-scan.json)
checks all changed and nonignored untracked files after the whole-tree scan.

## Integration with incoming work

The scrub was rebased onto `05499cc75`, preserving 72 incoming commits and
their current coordination state. The [post-rebase delta scan](post-rebase-scan.json)
checks all 4,191 retained files changed since the initial base, including those
incoming commits: no expansion errors or DER objects were found. Its six
binary hash/authorisation matches are the same synthetic fixture/format values
already reviewed; no values or credential hashes were persisted. Fresh query
checks pass 4/4 and 5/5, and trust controls pass 3/3 after the rebase. The
canonical comparison is against the incoming base: only the two retention
records change, with no observation-result changes. Earlier broad test and
Clippy results remain scoped to the pre-rebase tree.

The remaining post-rebase UCS checks pass: CLI reader 4/4 and report
certificate/filestore tests 2/2. This gives 18 focused passes after integration.
The wider 15/17 report-file result, missing Python module, full gate limits and
Clippy failures remain the earlier scoped results; no broad post-rebase green
claim is made.
