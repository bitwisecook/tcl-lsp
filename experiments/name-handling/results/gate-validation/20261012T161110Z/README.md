# Does version-selected command backing agree with the declarations?

Yes: all 392 core declarations agree with the runtime constructed for the
release each row selects. Tcl 9.1-only `tcl::InitAutoPath` is absent from both
C9.0 reports and a builtin in both C9.1 reports. The generated report labels
each row's selected release; it does not combine the reports into one command
surface or publish later commands to older profiles.

The owner scanner verifies both original mode-lookup calls and the shared
`names_mode` helper. The literal-pool contract retains its interpreter/world/
epoch requirements in the existing five-column manifest. The Jim-only option
fixture declares no option effect. Its focused test passes. The owner removal
test removes the actual Unicode-glob owner row and still requires the missing
owner to be reported.

All commands source `/tmp/name-env.sh`, use
`CARGO_TARGET_DIR=/workspace/tcl-lsp/tmp/name-target-issues`, and run at `-j2`
with incremental compilation and development/test debug information disabled.

| Final check | Result | Exact evidence |
| --- | --- | --- |
| `cargo run -j2 --package xtask -- command-backing` | Generated report; zero disagreements. | [log](command-backing-final-generate.log) |
| `cargo run -j2 --package xtask -- command-backing --check` | Passes: 392 commands. | [log](command-backing-final-check.log) |
| `cargo test -j2 --package xtask command_backing:: -- --nocapture` | Passes: 12 tests, including independent C9.0 absence/C9.1 presence. | [log](command-backing-final-tests.log) |
| `cargo test -j2 --package xtask owner_resolution:: -- --nocapture` | Passes: 21 tests. | [log](owner-resolution-final-tests.log) |
| `cargo run -j2 --package xtask -- owner-resolution` | Fails: 165 manifest obligations remain. | [log](owner-resolution-final.log), [problems](owner-manifest-problems.json) |
| `cargo run -j2 --package xtask -- gen-ai-diagnostics --check` | Passes. | [log](gen-ai-final-check.log) |
| Changed xtask files: `rustfmt --edition 2024 --check` | Passes. | [log](final-rustfmt-check.log) |

The 165 owner obligations consist of 159 entries without public declarations,
two uncovered owner bullets, and four non-xtask proof-check gate references.
The public visibility default remains strict. The scope distinction and concrete
examples are in [owner-entry-scope.md](owner-entry-scope.md).

The final [source receipt](frozen-source-inventory.json) inventories 4,942 Rust,
Cargo, and owner-contract inputs. Its SHA-256 is
`7a2b961bcbd21f21cfbe52ec3e1a9d19818d5be938652c9e70006b9bd2beffb6`.
The [before/after check](source-inventory-check.json) confirms identical input
bytes and no changed paths throughout the final validation. The owner-problem
record is bound to that same receipt. These are source receipts, not refreshed
semantic proof digests.

Earlier logs retain the test-compilation and stale-test failures. The first
selected-backing build records the compiler's new C namespace/body obligations
missing from diagnostic presentation; their owner supplied explicit mappings
before the successful final build. The
[intermediate receipt](intermediate-source-inventory-check.json) identifies the
single runtime `native_each.rs` lazy-selection correction between an earlier
validation and the final fixed-source run. These earlier attempts do not supply
the final source-stability claim.

This result does not establish the workspace lint, semantic-proof, or PR gates.
