# Issue action proposals

Inspected basis: merged `c49ef6fd5`, 12 October 2026. GitHub readback
returns #2286 and #2287 OPEN; subsequent readback confirms #2450 CLOSED. The published `work` ref is
`bcaf88c12400c1a0a529a54ba8a73dacb216da41`. Only the coordinator mutates
GitHub; proposals below distinguish current source, retained execution and
fresh validation.

## #2450: confirmed closed documentation defect

GitHub readback confirms closure at 2026-10-12T11:56:05Z based on current
text plus the already published removal.

Exact comment:

> Fixed in [47c00bbad](https://github.com/bitwisecook/tcl-lsp/commit/47c00bbad14c1a52bb689d9087ec1990d42f007f), published through PR #2457. `docs/kcs/codes/kcs-diagnostic-w210-variable-read-before-set.md` now describes current registry-driven behaviour; the “Before this was registry-driven” paragraph and #2117 development-history reference are absent in the current `work` tree. Closing this documentation defect.

This closure says nothing about repository-wide documentation cleanup.

## #2286: update now, close after current validation

Published [b049f3caa](https://github.com/bitwisecook/tcl-lsp/commit/b049f3caa90a49d806bc809ab7fbc115435ca682)
introduces the four reported shared-frame source fixtures and regression
selector. Subsequent Original owner work changes source/Registry/loader
selection and adds direct/captured alias controls.

[The current GitHub update](https://github.com/bitwisecook/tcl-lsp/issues/2286#issuecomment-6124201808)
records the actual fresh positive/negative result and withholds closure:

> A fresh compiled regression run passes all four reported source fixtures across Tcl 8.4, 8.5, 8.6, 9.0 and 9.1 profiles. The paired genuine-undefined-read control still fails under 8.4: it expects one W210 for `$absent` but receives no warning. Passing the original examples alone therefore does not establish correct shared-frame analysis; keeping this issue open while that control and the user-facing paths are checked.

The completed shared-frame command has one pass/one failure; the completed
alias command has one pass/one failure. Each genuine-read negative stops at
its first Tcl 8.4 case, so other negative release/capture variants are not
covered by that failing execution. The alias fixture-positive runs its own
five-profile/captured-prefix matrix. Source changed during the builds;
the post-freeze negative-only run confirms both controls still fail at their
first Tcl 8.4 assertion (zero W210 instead of one). Its selected source
inventory has identical before/after hashes; no later negative profile or
captured-alias variant executed past those failed assertions. The raw logs, exact commands/counts and mixed/frozen source inventories are
retained in [issue reproduction records](../../../experiments/name-handling/results/issue-reproductions-2026-10-12/records.json).
The earlier mixed builds do not certify every later source; the final
negative-only capture separately records its unchanged selected inventory.

Current selectors, each from `rust/tcl-compiler/src/analyser/diagnostics/tests.rs`:

- `tcltest_shared_frame_preserves_the_reported_setup_body_cleanup_reads`
- `tcltest_shared_frame_keeps_genuine_undefined_reads_and_data_opaque`
- `tcltest_aliases_preserve_the_reported_setup_body_cleanup_reads`
- `tcltest_aliases_keep_genuine_undefined_reads_and_result_data_opaque`
- `tcltest_import_is_namespace_scoped`

The shared-frame and alias controls loop all five C profiles. Their genuine
Tcltest provider fixture/attestation is part of the test; a package name alone
does not authorise body execution.

```sh
cargo test --manifest-path Cargo.toml -p tcl-compiler --lib tcltest_shared_frame_ -- --nocapture
cargo test --manifest-path Cargo.toml -p tcl-compiler --lib tcltest_aliases_ -- --nocapture
cargo test --manifest-path Cargo.toml -p tcl-compiler --lib tcltest_import_is_namespace_scoped -- --nocapture
```

The retained frozen334 log has all five selectors `... ok` at lines
117–121. Its complete receipt inventory is independent of current source;
a fresh result must retain the current source and actual command. After current controls pass, a final
closure comment should state the freshly observed diagnostic result and the
published fixing commit, without claiming complete centralisation.

## #2287: execute the actual namespaced semantic-token regression

The pinned benchmark checkout lacks `test/native.test`. A supplemental
GitHub contents query found it at actual file-touch revision
`8f97791d186f5a4e4c63799778e36b42fe5b8c59`, blob
`25b9bd134fcb617ff98a593e4662f611ac63694f`. Full/range hashes, upstream
MIT attribution and adaptation are in
[issue2287-source.json](../../../experiments/name-handling/patterns/issue2287-source.json).
This query changes no benchmark pin.

The new server fixture retains the first two actual tests, package
requirements and `namespace path ::tclopt`; it moves
`namespace import ::tcltest::*` into `::nativeTest` as reported.
It asserts both `-body`/`-result` option pairs and nested
`mp_enorm`, `mp_covar`, `set`, `list`, `expr` and `dict`
command tokens through the packaged-server helper.

```sh
cargo test --manifest-path Cargo.toml -p tcl-lsp-server --test e2e semantic_tokens::test_namespaced_imported_tcltest_test_structure_recognised -- --exact --nocapture
cargo test --manifest-path Cargo.toml -p tcl-lsp-server --test e2e semantic_tokens::test_imported_tcltest_test_structure_recognised -- --exact --nocapture
```

The fixture does not execute the C kernels. Both actual packaged-server
runs fail: the new namespace-scoped fixture's two `-body` tokens are strings,
and the existing top-level companion also reports string instead of decorator.
The namespace fixture has zero passes/one failure in 7.40 seconds; its
companion has zero passes/one failure in 1.88 seconds. This is a shared source
advice gap, not evidence of a namespace-only failure. Exact raw results are in
[the retained records](../../../experiments/name-handling/results/issue-reproductions-2026-10-12/records.json).

[Current GitHub update](https://github.com/bitwisecook/tcl-lsp/issues/2287#issuecomment-6124559399):

> The shared naming work now includes an exact namespace-scoped semantic-token regression based on the first two upstream `test/native.test` tests, with the `tcltest` import moved inside `namespace eval ::nativeTest` as reported. It checks Tcltest option highlighting and commands inside both test bodies through the server. The new namespaced regression and existing top-level companion both run and fail: `-body` is classified as string instead of an option. This issue remains open while the shared conditional source-advice owner is completed.

## Question-level proof gaps

The refreshed 1,050-issue search found no separate new issue titled for
Original receipt/proof gaps. Do not invent an issue number or claim a failure
merely from a pending provider answer.

The canonical native-attempt
`naming.variable.original-frame-array-trace-retirement-horizon` fails during
setup before the intended question. A new valid retirement probe would be a
different input and question, retaining the original failed attempt.
New corpus Jim/F5 inputs remain pending independently of canonical
observations for other inputs. Current Rust runs now establish the specific
consumer failures above; they supply no new native-provider answer.

If the complete proof checker or a fresh consumer run fails, retain its exact
input, source, reached stage and output, then create/update the corresponding
narrow issue. Do not turn general `not-tested` provider statuses into runtime
bug reports. Canonical proof registration belongs to the manifest/index owner.
