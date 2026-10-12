# Do the owner and backing contracts match the current sources?

The generated AI outputs match their templates. Owner and runtime-backing
validation still fail on the specific discrepancies below.

The source snapshot is branch `work`, HEAD
`8f6e25e5d1200d6a90e2ba92e07ac3bef4fe8683`, including the uncommitted semantic
changes and completed workspace formatting. Commands source `/tmp/name-env.sh`,
set `CARGO_TARGET_DIR=/workspace/tcl-lsp/tmp/name-target-issues`, and use `-j2`
with incremental compilation and development/test debug information disabled.

| Check | Result | Evidence |
| --- | --- | --- |
| `cargo run -j2 --package xtask -- owner-resolution` | Fails: array-names scanner omits the `names_mode` helper; original literal-pool manifest row has six columns instead of five. | [log](owner-resolution.log) |
| `cargo run -j2 --package xtask -- command-backing` | Regenerated the 392-command report; reports one disagreement. | [log](command-backing-generate.log) |
| `cargo run -j2 --package xtask -- command-backing --check` | Fails: `tcl::InitAutoPath` declares shipped-builtin but runtime reports absent. | [log](command-backing-check.log) |
| `cargo run -j2 --package xtask -- gen-ai-diagnostics` | Regenerated Tcl and iRules prompts in both canonical and Claude destinations. | [log](gen-ai-diagnostics-generate.log) |
| `cargo run -j2 --package xtask -- gen-ai-diagnostics --check` | Passes. | [log](gen-ai-diagnostics-check.log) |
| `cargo test -j2 --package xtask owner_resolution:: -- --nocapture` | Test compilation fails: `audit_option_dialects.rs:1318` fixture omits `OptionSpec.effect`. | [log](owner-resolution-tests.log) |
| `cargo test -j2 --package xtask command_backing:: -- --nocapture` | Same test-compilation blocker. | [log](command-backing-tests.log) |
| `cargo test -j2 --package xtask gen_ai:: -- --nocapture` | Same test-compilation blocker. | [log](gen-ai-tests.log) |
| `cargo test -j2 --package tcl-registry --test registry_sweep a_shipped_builtin_names_its_registered_implementation -- --exact --nocapture` | Passes: one test. | [log](registry-shipped-identity-test.log) |

The manifest row must retain the actual interpreter/world/epoch ownership
contract within its existing five columns. The array scanner must verify the
shared helper and the production call to it. The missing fixture field and
`tcl::InitAutoPath` registration/declaration discrepancy need separate bounded
corrections before these checks can pass. No source edits, waiver additions,
proof digest refreshes, or captured native input changes were made by this run.

This result does not establish the workspace lint, proof, or PR gates.
