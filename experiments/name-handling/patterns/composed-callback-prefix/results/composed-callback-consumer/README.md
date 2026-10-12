# Original-source consumer regression inputs

[Proposed test source](proposed-tests.rs) contains four bounded consumer tests
for the exact retained positive/adversarial source and the separate private
source fixture. The four concrete tests are now applied inside the existing
`references.rs` test module. The [first frozen build](frozen-build-1/result.json)
failed in existing test-source code before any of the four assertions ran;
no successful test binary or execution receipt exists. The separate logical
type-hint control was not attempted after that compilation failure.
[Source-edit receipt](source-edit.json) and [exact source diff](source.diff)
retain the frozen owner hash and selectors. Only the owned Rust file was
formatted, with `skip_children=true`. No production code changed.

[Proposal receipt](proposed.json) pins source bytes, owner-file pre-edit hash,
precise test selectors and the dependency boundary. `tcl-lsp-core` normally
consumes compiler/registry source owners, but both upstream crates' test closures
reach core through `tcl-spectcl`; those reverse dev-dependency paths prevent
assuming an independent build snapshot.

The intended tests require actual original builder/schema/argument receipts,
actual positive public/private selector ranges, explicit unknown/opaque/nested
writer withdrawal and full source/input/configuration currency. An owner refusal
must remain a failing intended-consumer assertion and be inspected at its first
actual barrier. It must not be converted to a passing conditional assertion.

The selector filter is `composed_callback_prefix_tests`; native provider records
remain separate and do not establish that these consumer assertions pass.

The [corrected frozen build](frozen-build-2/result.json) compiled and executed
exactly four composed tests and one separate Logical type-hint control. All four
composed tests failed at the common source-extraction fatal-tail guard before
producer/reference assertions. The Logical control failed its first positive
int-hint assertion with empty output. Raw streams, the actual test binary hash
and source-currency checks are retained; no assertion was weakened.

The [body-helper corrected run](frozen-build-3/result.json) executed exactly
four controls with restored compiler diagnostic source: one adverse absence
control passed and three positive/owner-currentness controls failed. Original
builder/schema checks passed, but the positive authored-prefix receipt was
absent and references retained only the declaration. Passing adverse absence
cannot independently prove writer-caused withdrawal without the failed positive
baseline. All assertions remain concrete and unchanged. The Logical control was
not rerun; its separate prior empty-hints failure remains open.
