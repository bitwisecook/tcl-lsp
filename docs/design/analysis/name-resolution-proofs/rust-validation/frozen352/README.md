# Frozen352 software evidence

The source inventory identifies commit `cd8efdf8f5eac670980eb19534c5910ea7d08124` and its complete tracked file hashes. The VM and Runtime library targets have independently emitted, owned executable images and recorded normal inventories.

| Exact isolated library assertions | Passed | Failed assertions | Timed out | Aborted |
| --- | ---: | ---: | ---: | ---: |
| VM: 431 selectors | 377 | 53 | 1 | 0 |
| Runtime: 504 selectors | 481 | 23 | 0 | 0 |

The unchanged dictionary test covering 345 native comparisons also has a separate successful execution against the same owned VM image: one passed assertion, no failed or ignored assertions, and 124.047 seconds elapsed under an 1800-second budget. Its 300-second timeout in the isolated group remains a distinct observation.

[The experiment index](experiment-index.json) contains exact commands, process statuses, elapsed times, image identities and retention hashes. The child records preserve each exact selector and its original assertion output. Complete gzip images and whole Rust source files round-trip to their original recorded bytes; retained images are replay material, not substitutes for the corresponding source and provider dependencies.

These observations do not establish outcomes for another source, the whole test suite, compiler/editor targets, a build without tommath, or BIG-IP. Three controls absent from the normal feature inventory require their own actual build. The recorded environment configuration is not a measurement of the complete inherited environment. The Rust source archive does not establish external SDK or included-resource closure. Per-question bindings require the actual original test body and the separately documented purpose of each Native observation.
