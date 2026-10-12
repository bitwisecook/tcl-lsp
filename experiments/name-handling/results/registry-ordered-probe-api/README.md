# Does the ordered comparison fixture implement the current API?

The retained clock-worker package invocation failed to compile this integration
fixture: `ExprOps::compare_string` now returns `Result<Ordering, Error>`.
`OrderedProbe` preserves its decimal-string lexical comparison and wraps the
unchanged result in `Ok`; its values, refusal paths and expected outcomes stay
unchanged. This changes only the test adapter.

The [exact capture](20261012T210251Z/result.json) records the authored
`the_seven_ordered_state_witnesses` control: one passed, zero failed,26 filtered.
[Command and input hashes](20261012T210251Z/source-before.json) and the
[full log](20261012T210251Z/test.log) identify the sources and compiled binary.
The selected source hashes match before and after execution. This result does
not claim acceptance for filtered tests or native object/frame conformance.
