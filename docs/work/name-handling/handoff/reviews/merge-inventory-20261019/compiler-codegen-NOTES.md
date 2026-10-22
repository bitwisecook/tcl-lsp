# compiler-codegen: entry-less whole-module compiles, pack compiler rows, `string cat`, dict release units

Implementer: compiler-codegen agent. Branch `work`. I committed, pushed, stashed and reset nothing;
the coordinator's WIP snapshots (latest seen: 56893780b9) carry the tree as it stood.

User decision applied: `user-decisions-20261019T1420Z.json` — a document or module with no explicit
entry runs as a FRESH INTERPRETER OF ITS PROFILE (`tclsh file.tcl`).

Files changed (all compile; `cargo check -p tcl-registry -p tcl-compiler -p tcl-spectcl --lib
--tests` and `runtime/rust cargo check --locked --features engine --all-targets`: 0 warnings):

| File | Owner | Change |
|---|---|---|
| `rust/tcl-compiler/src/compile_service.rs` | mine | fresh whole-module path (§1, §2), profile-less engine, manifest restamp, Direct fallback, 5 regression tests |
| `rust/tcl-compiler/src/codegen/control_flow.rs` | mine | `release_compiled` for `dict update` / `dict with` (§4) + test |
| `rust/tcl-compiler/src/codegen/emitter/mod.rs` | mine | `module_manifest` is `pub(crate)` |
| `rust/tcl-compiler/src/codegen/statements/native_scalar.rs` | mine | `string cat` emission (§5) |
| `rust/tcl-compiler/src/command_binding/compiled_preflight.rs` | granted by coordinator | entry-less provider rule (§1); `command` split into plan + descent frames (§7) |
| `rust/tcl-compiler/src/command_binding/compiled_preflight/original_preparation.rs` | granted | fresh-world expression emission (§1) |
| `rust/tcl-compiler/src/optimiser/manager.rs` | was git-clean; see §6 | `optimise_raw` over a profile's registry analyses for that profile |
| `rust/tcl-spectcl/src/install.rs` | granted by coordinator | derived native compilers for pack rows (§3) + 2 tests |
| `rust/tcl-registry/src/native_string_operand_compilation.rs`, `native_compilation.rs`, `commands/tcl/string_.rs` | granted (string cat) | `string cat` compiler grammar (§5) + test |
| `runtime/rust/src/interp/native_body_artifact/native_string_operand.rs` | granted (string cat) | `Cat` arm |

The fix already in my files from the compiler agent (`codegen/cmd_subst.rs::emit_native_argument_word`,
`string length "{}$z"`, test `compile_service::tests::native_operand_words_in_a_substitution_keep_their_variables_live`)
is intact and untouched; it passes before and after.

(Sections and measurement tables follow.)
