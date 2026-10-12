# Reviewed original stock Tcl final failures

This is a derived review of the sealed `nh20261012stock2` capture, not another appliance execution. All four derived records retain that actual run ID and original input, wrapper, invocation and raw-output paths. The nested review directory uses the original capture name for strict identity validation. The sealed capture is unchanged.

Both stock interpreters reached the authored final operation in each original program:

| Case | Stock Tcl 8.4.13 | Stock Tcl 8.5.13 | Source/stack boundary |
| --- | --- | --- | --- |
| factory-body-near-miss | code 1: `can't read "name": no such variable` | same | Original line 4 invokes `::nh::red`; the procedure body then executes `return $name` and fails. The foreach/procedure definition prefix completed. |
| string-prefix-near-miss | code 1: `invalid command name "::nh::two"` | same | Original line 4 enters `eval "$target {a b} {c d}"`; the inner script attempts `::nh::two words {a b} {c d}`. The spaced procedure name and target assignment completed first. |

The retained invocation starts a fresh `/usr/bin/tclsh8.4` or `/usr/bin/tclsh8.5` process for each case. Wrapper setup is empty and returns code 0; it is not an ownership guard and does not alone establish original prefix completion. The unchanged original operation bytes, final source range, complete error stack and sequential execution establish the final-operation boundary.

`records.json` contains four derived measurements; `reviewed-proofs.json` contains four narrow candidate canonical records. `nh20261012stock2/reviewed-reachability.json` hashes the original source, materialised wrapper, case operation pointer, original measurement and complete raw stack chunks. The current strict reachability validator accepted all four. Original exact error options retain their captured availability.

- `factory-body-near-miss` / `stock-8.4`: original `experiments/name-handling/patterns/probes/factory-body-near-miss.tcl` SHA-256 `a6cab17e3a2ba03a60ba081432b5c149230a68eb35fbd2539c1669eecedb993b`; original records `/4`  SHA-256 `2e413c93a1a358d84a0b35f9bfd4ee7f2d0ba76a3f2f0ea4a7e385159313fcf1`; raw `experiments/name-handling/patterns/f5/answers/nh20261012stock2/stock-8.4-4/runtime.stdout` SHA-256 `64a29a31d2736c5c1b1a56fa77ec21169c35c390abd3353debf4c40f92a84386`.
- `string-prefix-near-miss` / `stock-8.4`: original `experiments/name-handling/patterns/probes/string-prefix-near-miss.tcl` SHA-256 `0d068108e0d3c22b24b647f7a17d15ab1d1f1420f2653ad3b1609011e4665620`; original records `/13`  SHA-256 `2e413c93a1a358d84a0b35f9bfd4ee7f2d0ba76a3f2f0ea4a7e385159313fcf1`; raw `experiments/name-handling/patterns/f5/answers/nh20261012stock2/stock-8.4-13/runtime.stdout` SHA-256 `c254f9f8dd226949b9c3635f2cdfb254aed742cd02d55d3a3061287ae36bfd4f`.
- `factory-body-near-miss` / `stock-8.5`: original `experiments/name-handling/patterns/probes/factory-body-near-miss.tcl` SHA-256 `a6cab17e3a2ba03a60ba081432b5c149230a68eb35fbd2539c1669eecedb993b`; original records `/49`  SHA-256 `2e413c93a1a358d84a0b35f9bfd4ee7f2d0ba76a3f2f0ea4a7e385159313fcf1`; raw `experiments/name-handling/patterns/f5/answers/nh20261012stock2/stock-8.5-4/runtime.stdout` SHA-256 `2a7151cdf2a69463b2b2627c70310c1929ba652e3859a4309da9d4a565dd7703`.
- `string-prefix-near-miss` / `stock-8.5`: original `experiments/name-handling/patterns/probes/string-prefix-near-miss.tcl` SHA-256 `0d068108e0d3c22b24b647f7a17d15ab1d1f1420f2653ad3b1609011e4665620`; original records `/58`  SHA-256 `2e413c93a1a358d84a0b35f9bfd4ee7f2d0ba76a3f2f0ea4a7e385159313fcf1`; raw `experiments/name-handling/patterns/f5/answers/nh20261012stock2/stock-8.5-13/runtime.stdout` SHA-256 `e16734b2acb42e250e528f29dce46671f99beebdbe07f1d0c9da408fd7cb13f7`.
