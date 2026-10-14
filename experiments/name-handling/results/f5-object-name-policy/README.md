# Authored F5 object-name policy controls

These are software-policy controls, not BIG-IP oracle observations. The
original-source harness retains the replaced helper implementations; the
migrated harness retains their shared-owner adapters. Both exercise the same
fixed references, paths and authored partitions: 330 comparisons each passed.
Eight standalone owner controls and 16 extracted actual topology helper comparisons passed, including duplicate exact rows and ambiguous short names. Full workspace typechecks and consumer
integration suites are separate checks; these captures establish neither.

The 330-comparison captures cover six configuration helpers, TLS generic references and GTM helpers. They do not cover topology. The separate topology harness extracts both actual rule-source and virtual-server helper bodies with fixture-only model adapters; it supplies no live identity grant.

The source snapshot and extracted helper bodies are retained for reproduction.
Run these commands with the workspace Rust toolchain from this directory:

```sh
rustc --edition 2024 --test object-owner-source.txt -o /tmp/f5-object-owner-controls
/tmp/f5-object-owner-controls
rustc --edition 2024 -A dead_code original-policy-controls.rs -o /tmp/f5-object-original-controls
/tmp/f5-object-original-controls
rustc --edition 2024 -A dead_code migrated-consumer-controls.rs -o /tmp/f5-object-migrated-controls
/tmp/f5-object-migrated-controls
rustc --edition 2024 -A dead_code topology-policy-controls.rs -o /tmp/f5-object-topology-controls
/tmp/f5-object-topology-controls
```

Unused extracted fixture helpers are suppressed only in these standalone
harnesses; production has no new lint allowance. Selection assertions are
exact. Fixed controls cover ambiguity, different first/unique policies,
absolute misses, same-leaf collisions, colons, repeated slashes and literal
`..`. Appliance measurements remain unanswered in the
[F5 question packet](../../f5/questions/object-path-resolution.md).
