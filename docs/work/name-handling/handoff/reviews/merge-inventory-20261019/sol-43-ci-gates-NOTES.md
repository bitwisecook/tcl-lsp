# Sol-43 — CI gate fixes (hosted run 39304840863)

Branch `work` @ c8449735a7, working tree only (nothing committed). The full diff of
every file listed here is `sol-43-ci-gates.patch` in this directory (141 files,
+1076/−874; generated with `git diff -- <my files>`).

## Changes made in other agents' areas (please review)

| Area (owner) | File | Change |
|---|---|---|
| runtime/rust (Sol-41) | `runtime/rust/include/tcl.h` | `Tcl_GetReturnOptions` moved from the shared declarations into the `TCL_HOST_NATIVE`-only leg (item 4). |
| runtime/rust (Sol-41) | 15 files under `runtime/rust/src/` | no-libtommath warning fixes only: `#[cfg(have_tommath)]` on imports, `#[cfg_attr(not(have_tommath), allow(dead_code))]` on items reached only by the bignum backend, one `let _ = (current, amount);` (item 7). |
| rust/tcl-lsp-* (Sol-42) | `rust/tcl-lsp-server-wasm/src/lib.rs`, `rust/tcl-lsp-server-wasi/src/main.rs` | `#![recursion_limit = "256"]` with the same comment `tcl-lsp-server` uses (item 6). |
| none (shared) | `rust/tcl-bytecode/src/lib.rs` | `#[allow(clippy::struct_excessive_bools)]` on `Instruction` (item 5). |

I did not touch rust/tcl-compiler, rust/tcl-vm, rust/tcl-cmd-core, rust/tcl-engine-tclvm, or rust/tcl-lsp-{core,db,server}.

## 1. cargo-deny: fixed

* The root `Cargo.lock` had `inout 0.2.3` (yanked). origin/rust also pins 0.2.3, so upstream will hit the same error. Ran `cargo update -p inout`, which gave 0.2.5 (a 2-line lock diff).
* With the root fixed, the script went on to the next lockfiles and they failed as well:
  * `rust/bigip-report-gen/{python,wasm}/Cargo.lock` also had `inout 0.2.3`. I ran `cargo update -p inout` in each, giving 0.2.5.
  * `rust/bigip-query-wasm`, `rust/bigip-report-gen/{python,wasm}`, `rust/tcl-explorer-wasm` and `rust/tcl-spec-studio-wasm` lockfiles were stale under `--locked`. The merge added the path deps `bytecount`, `fluent-uri`, `borrow-or-share`, `ref-cast(-impl)` and `tcl-host-c-abi`. I regenerated them with `cargo metadata`. Every diff only adds entries, and the versions match the root lock.
  * `scripts/dev/cargo-deny-all.sh` selected lockfiles with `git ls-files '*Cargo.lock'`. That pattern also matched the archived evidence files `docs/design/analysis/name-resolution-proofs/rust-validation/frozen235/lockfile-source235/{original,cargo-updated}-Cargo.lock`, which have no manifest beside them, so the script stopped with an error. It now uses `':(glob)**/Cargo.lock'`, which matches only files named exactly `Cargo.lock`.
* Verified with the pinned cargo-deny 0.20.2 (the release binary, kept in the scratchpad): `bash scripts/dev/cargo-deny-all.sh` returns rc=0. All 12 workspaces report `advisories ok, bans ok, licenses ok, sources ok`. Duplicate-crate warnings remain, which is acceptable.

## 2. python: lockfile fixed; the job still fails at `lint-py` (not fixed)

* `rust/bigip-report-gen/python/Cargo.lock` was regenerated as described in item 1. The "cannot derive Python engine non-compilation target roots" message has the same cause: `make python-engine-source-hash` runs `cargo metadata --locked`, which failed on the stale lock, so the script got empty JSON. `make --no-print-directory python-engine-source-hash` now prints a hash.
* **Next failure, not fixed:** `make lint-py` (ruff 0.17.1). Tracked `*.py` files grew from 30 on origin/rust to 813 on `work`. 650 files would be reformatted and `ruff check` reports 1937 errors. Almost all of these are under `experiments/name-handling/**`, `docs/design/analysis/name-resolution-proofs/**` and `scripts/dev/bigip-probes/**` (name-handling evidence). Some of those files are digest-pinned, so a mass reformat would invalidate the proofs. Someone needs to decide on the exclusions: add them to `PY_FILES` in the Makefile, the same way `.../evidence/**` is already excluded. Outside experiments/docs, about 50 files fail, mostly in `scripts/dev/bigip-probes/{name-patterns,resolution-2286}`, plus `rust/bigip-report-gen/python/tests/test_certs.py` and `rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/prepare.py`.
* Not run: `make typecheck-py` / `test-py-engine`. Both need a maturin release build of the engine, and there was not enough disk.

## 3. web-frontends: fixed

* Ran the repo's pinned prettier 3.9.10 (`npm ci` with npm 12.2.0 through a corepack shim, as the job does) with `--write` on `src/nativeEditorHost.ts` and `test/units.test.ts`. It applied the repo's `.prettierrc.json` (printWidth 100). The diff is formatting only.
* The next failure was `spec-studio-test`, which CI never reached: `ERR_PACKAGE_PATH_NOT_EXPORTED: './browser'` from `vscode-jsonrpc`. `test/units.test.ts` now imports `src/lspClient.ts`, which imports `vscode-jsonrpc/browser`. The test bundle keeps packages external, and that export only has a `browser` condition. Fix: the `package.json` `test` script now runs `node --conditions=browser --test …`, with a comment in `build.mjs`.
* Verified: `make typecheck-spec-studio-ts lint-spec-studio-ts spec-studio-assets spec-studio-test` returns rc=0 (127/127 unit tests). This ran on local node 22; CI uses node 26.

## 4. wasm-real-link: fixed (header leg corrected)

* origin/rust's `tcl.h` does not declare `Tcl_GetReturnOptions`. `runtime/rust/src/capi.rs` does not export it, but `rust/tcl-cshim/src/ffi.rs` (the native shim) does. So I moved the declaration into the `#if defined(TCL_HOST_NATIVE)` leg instead of exporting it from the runtime. `docs/design/runtime/c-extension-abi.md` now lists it among the native-leg additions.
* Verified: `TCL_REQUIRE_WASM_LINK=1 make check-c-extension-wasm` reports "WASM leg declares 33 function(s) … native leg 37 … wasm32 compiles passed" (wasi-sdk at /opt/wasi-sdk). The later steps of the job, `cargo test -p tcl-compiler --test wasm_real_link …` and `tcl-engine-wasm`, were not run because of disk.

## 5. rust-check: my part fixed; Sol-41/42 crates listed for later

* `rust/tcl-bytecode/src/lib.rs`: `#[allow(clippy::struct_excessive_bools)] // independent per-instruction flags, not a state machine` on `Instruction`. Its four bools are `no_fold`, `foreach_collect`, `push_verbatim` and `script_command`. This follows the existing pattern in tcl-cmd-core and tcl-registry.
* After that, the gate (`cargo clippy --workspace --all-targets -- -D warnings`) reported **3035 findings**. 260 of them were in non-owned crates, and I fixed all 260:
  * Machine-applicable fixes via `cargo clippy --fix -p <those crates>`, followed by `cargo fmt -p <those crates>`.
  * 44 × `#[allow(clippy::too_many_lines)]`, which is the repo convention (117 existing uses).
  * The rest by hand:
    * Moved items-after-statements to the top of their scope.
    * Used let-else.
    * Replaced `Default::default()` with the named type.
    * Replaced truncating casts with `try_from().unwrap()` in tests.
    * Passed a reference instead of a value: `tcl-engine-wasm` `decline_engine`, `tcl-irule-test` `refuse_value_access`, `tcl-spec-hooks` `verbs_for_with_metadata_reader` (now `Option<&HostMetadataListReader>`), and xtask `bind_enum_pattern` / `field`.
    * Derived `Copy` on `tcl-mcp` `PathSource`.
    * Boxed the `Notice` error of `tcl-spectcl` `include_name`.
    * Iterated large test arrays by reference, and used a `vec!` for a `CommandSpec` table.
    * `clone_from` / `clone_into`.
    * Wildcard arm replaced with the named variant.
    * `const { assert!(..) }`.
  * Targeted allows with a reason: `large_enum_variant` (`IrulesWhenSourceInventory`), `struct_excessive_bools` (spectcl eval `State`), `option_option` (two private fns), `needless_pass_by_value` (two spectcl fns whose public caller is in tcl-lsp-server), `type_complexity`, `format_collect` (test hex helpers), `single_range_in_vec_init` and `needless_range_loop` (tests), `mutable_key_type` (borrowed analysis keys), and `enum_variant_names` (`Failure::ExecutionFailure` mirrors `EngineError`).
* Verified: `cargo clippy -p <the 24 crates> --all-targets` reports no findings in those crates. `cargo clippy -p tcl-bigip-query --no-default-features --features x509 --all-targets` is clean. `cargo fmt --all --check` shows diffs only in other agents' in-progress files (`tcl-compiler/src/analyser/{diagnostics/fp/mod.rs,types.rs}` and the untracked `tcl-vm/tests/zz_s41_probe.rs`).
* Other rust-check (`xtask-check`) sub-gates I fixed:
  * `check-runtime-rust-paths`: the runtime closure gained `tcl-bytecode` and `tcl-host-c-abi`. Both were added to `scripts/dev/runtime-rust-path.sh`. `bash scripts/dev/test-runtime-rust-paths.sh` passes.
  * `check-rust-tests-paths` / `check-lsp-e2e-paths`: added `rust/tcl-host-c-abi` to `scripts/dev/{rust-tests,lsp-e2e}-package-paths.txt`. Both scripts pass.
  * `check-rust-tests-runner`: assigned the 14 new test binaries in `scripts/dev/rust-test-binary-shards.tsv` (rows appended per shard; no existing row moved). The only binary still missing is Sol-41's untracked temporary `tcl-vm::zz_s41_probe`, which goes away when that file is deleted.
* Other sub-gates:
  * Passing: `check-lsp-e2e-partitions`, `already-green`, `monitoring-triggers`, `persistent-cargo-target`, `wasm-cc-env`, `homebrew-ci`, `sign-and-upload`, `retired-asset-names`, `release-dependency-graph`, `vsix-web-assets-contract`, `spectcl-compat-paths`, `pr-gate-path`, `python-ci-paths`, `lsp-wasi-lto`, `check-c-extension-wasm`.
  * **`check-name-resolution-proofs` fails, not fixed (Sol-42 / analysis area):**
    * 1331 evidence/census digests are stale, 952 of them under `scripts/dev/bigip-probes/**`.
    * 3 proof documents are not indexed.
    * 4 test proof annotations have no catalog binding.
    * About 30 of the changed-digest files are files I edited for clippy or warnings, so refresh the digests after all agents finish.
  * **Not run:** every `xtask-*` sub-gate that needs `cargo xtask`. The xtask build failed because `tcl-vm` (lib) does not currently compile in the shared tree (E0596, Sol-41 in-progress edits), and disk was low.

### Clippy findings left for Sol-41/Sol-42 (from the post-fix workspace run, `--all-targets`, deduplicated by file:line:col)

Totals: tcl-compiler 1177, tcl-lsp-server 587, tcl-vm 517, tcl-lsp-core 414, tcl-lsp-db 58, tcl-cmd-core 19, tcl-engine-tclvm 2, plus 4 in `runtime/rust/tests/data/*/software_controls.rs`, which are compiled by tcl-vm tests. The counts below are grouped by clippy message (backticked identifiers are elided).

#### rust/tcl-compiler: 1177 findings
- 240 x this function has too many lines
- 95 x consider adding a `_` to the last statement for consistent formatting
- 90 x redundant closure
- 57 x this expression creates a reference which is immediately dereferenced by the compiler
- 54 x using `_` on type `_` which implements the `_` trait
- 42 x adding items after statements is confusing, since items exist from the start of the scope
- 39 x implicit borrow as raw pointer
- 36 x use of `_`
- 35 x this argument is passed by value, but not consumed in the function body
- 33 x item in documentation is missing backticks
- 32 x casting `_` to `_` may truncate the value on targets with 64-bit wide pointers
- 32 x this boolean expression can be simplified
- 29 x used underscore-prefixed binding
- 28 x used `_` with an equality comparison
- 24 x calling `_` is more clear than this expression
- 23 x these match arms have identical bodies
- 19 x this `_` statement can be collapsed
- 16 x binding's name is too similar to existing binding
- 16 x mutable key type
- 12 x used `_` where `_` could be used instead
- 12 x empty String is being created manually
- 11 x assigning the result of `_` may be inefficient
- 11 x this function has too many arguments (8/7)
- 11 x allocating a local array larger than N bytes
- 10 x large size difference between variants: the entire enum is at least N bytes
- 10 x you seem to be trying to use `_` for destructuring a single pattern. Consider using `_`
- 9 x this could be rewritten as `_`
- 9 x very complex type used. Consider factoring parts into `_` definitions
- 8 x called `_` on an `_` value
- 8 x the following explicit lifetimes could be elided: 'a
- 7 x items after a test module
- 7 x deref which would be done by auto-deref
- 6 x unused `_` argument
- 5 x more than 3 bools in a struct
- 5 x this `_` may be rewritten with the `_` operator
- 5 x implicitly cloning a `_` by calling `_` on its dereferenced type
- 4 x usage of `_` in `_`
- 4 x this function has a `_` attribute with no message, but returns a type already considered as `_`
- 4 x this function's return value is unnecessarily wrapped by `_`
- 4 x this function has too many arguments (9/7)
- 3 x single-character string constant used as pattern
- 3 x it is more concise to loop over references to containers instead of using explicit iteration methods
- 3 x called `_` on a `_` value
- 3 x wildcard matches only a single variant and will also match any future added variants
- 3 x `_` appended to existing `_`
- 3 x field assignment outside of initializer for an instance created with Default::default()
- 3 x usage of wildcard import
- 3 x unnecessary boolean `_` operation
- 3 x manual saturating arithmetic
- 2 x boolean to int conversion using if
- 2 x cloning an `_` using `_`
- 2 x unnecessary semicolon
- 2 x using `_` with a constant chunk size
- 2 x these match arms have identical bodies: the wildcard arm
- 2 x unnecessary hashes around raw string literal
- 2 x empty line after doc comment
- 2 x used `_` on primitive type `_`
- 1 x unnecessary closure used to substitute value for `_`
- 1 x variables can be used directly in the `_` string
- 1 x this method could have a `_` attribute
- 1 x calling `_` using a single-character string literal
- 1 x only a `_` in `_`-then statement
- 1 x this `_` can be written more succinctly using another method
- 1 x this argument (400 byte) is passed by value, but might be more efficient if passed by reference (limit: 256 byte)
- 1 x needlessly taken reference of both operands
- 1 x an inclusive range would be more readable
- 1 x iterating on a map's keys
- 1 x accessing first element with `words
- 1 x getter function appears to return the wrong field
- 1 x using `_` instead of `_` is more efficient
- 1 x unneeded unit expression
- 1 x unnecessary `_` operation
- 1 x useless conversion to the same type: `_`
- 1 x called `_` on a `_`; this will needlessly iterate the entire iterator
- 1 x parameter is only used in recursion
- 1 x this function has too many arguments (10/7)
- 1 x redundant pattern matching
- 1 x this method chain can be written more clearly with `_`
- 1 x manual implementation of `_`
- 1 x all variants have the same postfix: `_`
- 1 x you appear to be counting bytes the naive way
- 1 x for loop over a single element
- 1 x this argument (264 byte) is passed by value, but might be more efficient if passed by reference (limit: 256 byte)
- 1 x this block may be rewritten with the `_` operator
- 1 x unnecessary closure used with `_`
- 1 x returning the result of a `_` binding from a block
- 1 x unnecessary use of `_`
- 1 x this `_` can be simplified
- 1 x unnested or-patterns

#### rust/tcl-lsp-server: 587 findings
- 197 x the `_`-variant returned from this function is very large
- 124 x large future with a size of N bytes
- 54 x this function has too many lines
- 23 x calling `_` is more clear than this expression
- 22 x this could be rewritten as `_`
- 21 x enclosing `_` and `_` operator are unneeded
- 11 x unnecessary semicolon
- 10 x the `_`-variant returned from this closure is very large
- 9 x redundant field names in struct initialization
- 8 x usage of wildcard import
- 8 x passing a unit value to a function
- 7 x item in documentation is missing backticks
- 7 x redundant closure
- 6 x assigning the result of `_` may be inefficient
- 6 x this `_` statement can be collapsed
- 5 x adding items after statements is confusing, since items exist from the start of the scope
- 5 x very complex type used. Consider factoring parts into `_` definitions
- 5 x binding's name is too similar to existing binding
- 5 x you seem to be trying to use `_` for destructuring a single pattern. Consider using `_`
- 4 x allocating a local array larger than N bytes
- 4 x cloning an `_` using `_`
- 3 x deref which would be done by auto-deref
- 3 x large size difference between variants: the entire enum is at least N bytes
- 3 x this function has too many arguments (8/7)
- 3 x unneeded `_` statement
- 3 x this expression creates a reference which is immediately dereferenced by the compiler
- 2 x unnecessary `_` operation
- 2 x used underscore-prefixed binding
- 2 x returning the result of a `_` binding from a block
- 2 x this `_` expression is redundant
- 2 x long literal lacking separators
- 2 x unnecessary use of `_` to create a slice from a reference
- 2 x this argument is passed by value, but not consumed in the function body
- 2 x this boolean expression can be simplified
- 1 x it is more idiomatic to use `_` instead of `_`
- 1 x struct constructor field order is inconsistent with struct definition field order
- 1 x this function has too many arguments (11/7)
- 1 x called `_` on an `_` value
- 1 x empty line after doc comment
- 1 x unnecessary boolean `_` operation
- 1 x this function has too many arguments (9/7)
- 1 x these match arms have identical bodies
- 1 x consider adding a `_` to the last statement for consistent formatting
- 1 x this function can be simplified using the `_` syntax
- 1 x casting `_` to `_` may truncate the value on targets with 64-bit wide pointers
- 1 x using `_` with a constant chunk size
- 1 x the borrowed expression implements the required traits
- 1 x this `_` may be rewritten with the `_` operator
- 1 x matching over `_` is more explicit

#### rust/tcl-vm: 517 findings
- 88 x this function has too many lines
- 53 x using `_` with a constant chunk size
- 46 x this `_` statement can be collapsed
- 40 x useless conversion to the same type: `_`
- 39 x very complex type used. Consider factoring parts into `_` definitions
- 27 x consider adding a `_` to the last statement for consistent formatting
- 26 x item in documentation is missing backticks
- 24 x redundant closure
- 19 x adding items after statements is confusing, since items exist from the start of the scope
- 17 x this argument is passed by value, but not consumed in the function body
- 11 x matching over `_` is more explicit
- 11 x this could be rewritten as `_`
- 8 x use of `_` to build up a string from an iterator
- 8 x unnecessary hashes around raw string literal
- 7 x binding's name is too similar to existing binding
- 6 x called `_` on a `_` value
- 6 x used underscore-prefixed binding
- 5 x you seem to be trying to use `_` for destructuring a single pattern. Consider using `_`
- 5 x this function has too many arguments (8/7)
- 5 x the borrowed expression implements the required traits
- 5 x this argument (272 byte) is passed by value, but might be more efficient if passed by reference (limit: 256 byte)
- 4 x usage of wildcard import
- 4 x this boolean expression can be simplified
- 3 x used `_` with an equality comparison
- 3 x returning the result of a `_` binding from a block
- 3 x unnecessary use of `_` to create a slice from a reference
- 3 x unneeded `_` statement
- 3 x unused `_` argument
- 3 x using `_` over `_`
- 3 x assigning the result of `_` may be inefficient
- 2 x this function has too many arguments (9/7)
- 2 x large size difference between variants: the entire enum is at least N bytes
- 2 x more than 3 bools in a struct
- 2 x items after a test module
- 2 x these match arms have identical bodies
- 2 x replacing an `_` with `_`
- 2 x calling `_` is more clear than this expression
- 1 x this match could be written as a `_` statement
- 1 x borrow of a mutable reference
- 1 x this expression creates a reference which is immediately dereferenced by the compiler
- 1 x this binding can be a slice pattern to avoid indexing
- 1 x long literal lacking separators
- 1 x call to `_` with a value that does not implement `_`. Dropping such a type only extends its contained lifetimes
- 1 x empty line after doc comment
- 1 x this `_` can be collapsed into the outer `_`
- 1 x enclosing `_` and `_` operator are unneeded
- 1 x unnecessary use of `_`
- 1 x deref which would be done by auto-deref
- 1 x used `_` on primitive type `_`
- 1 x this call to `_` does nothing
- 1 x called `_` on an `_` value
- 1 x it is more idiomatic to use `_` instead of `_`
- 1 x casting `_` to `_` may cause a loss of precision (`_` is 64 bits wide, but `_`'s mantissa is only 52 bits wide)
- 1 x wildcard matches only a single variant and will also match any future added variants
- 1 x this block may be rewritten with the `_` operator

#### rust/tcl-lsp-core: 414 findings
- 86 x this function has too many lines
- 61 x redundant closure
- 30 x casting `_` to `_` may truncate the value on targets with 64-bit wide pointers
- 24 x item in documentation is missing backticks
- 21 x this `_` statement can be collapsed
- 15 x unnecessary hashes around raw string literal
- 14 x adding items after statements is confusing, since items exist from the start of the scope
- 12 x consider adding a `_` to the last statement for consistent formatting
- 11 x this expression creates a reference which is immediately dereferenced by the compiler
- 11 x binding's name is too similar to existing binding
- 10 x this function has a `_` attribute with no message, but returns a type already considered as `_`
- 9 x field assignment outside of initializer for an instance created with Default::default()
- 8 x this argument is passed by value, but not consumed in the function body
- 7 x this `_` may be rewritten with the `_` operator
- 6 x this boolean expression can be simplified
- 6 x unnecessary use of `_` to create a slice from a reference
- 5 x large size difference between variants: the entire enum is at least N bytes
- 4 x this function has too many arguments (8/7)
- 4 x implicit borrow as raw pointer
- 4 x this function's return value is unnecessarily wrapped by `_`
- 4 x called `_` on a `_` value
- 4 x assigning the result of `_` may be inefficient
- 3 x mutable key type
- 3 x you seem to be trying to use `_` for destructuring a single pattern. Consider using `_`
- 3 x these match arms have identical bodies
- 3 x redundant field names in struct initialization
- 3 x used `_` with a literal bool
- 2 x empty line after doc comment
- 2 x allocating a local array larger than N bytes
- 2 x casting `_` to `_` may truncate the value on targets with 32-bit wide pointers
- 2 x use of `_`
- 2 x single-character string constant used as pattern
- 2 x the following explicit lifetimes could be elided: 'a
- 2 x using `_` over `_`
- 2 x more than 3 bools in a struct
- 2 x reference as raw pointer
- 2 x used `_` on `_` value
- 1 x you appear to be counting bytes the naive way
- 1 x deref which would be done by auto-deref
- 1 x backticks are unbalanced
- 1 x unnested or-patterns
- 1 x manual saturating arithmetic
- 1 x implicitly cloning a `_` by calling `_` on its dereferenced type
- 1 x redundant else block
- 1 x field name starts with the struct's name
- 1 x long literal lacking separators
- 1 x called `_` on an `_` value
- 1 x needlessly taken reference of both operands
- 1 x unnecessary map of the identity function
- 1 x this `_` block can be collapsed
- 1 x this could be rewritten as `_`
- 1 x wildcard matches only a single variant and will also match any future added variants
- 1 x field name ends with the struct's name
- 1 x using `_` with a constant chunk size
- 1 x this block may be rewritten with the `_` operator
- 1 x unused "map closure" when calling `_` value
- 1 x it is more concise to loop over references to containers instead of using explicit iteration methods
- 1 x unnecessary boolean `_` operation
- 1 x unnecessary `_` operation
- 1 x called `_` on `_`

#### rust/tcl-lsp-db: 58 findings
- 12 x this function has too many lines
- 11 x used `_` with an equality comparison
- 11 x adding items after statements is confusing, since items exist from the start of the scope
- 4 x using `_` with a constant chunk size
- 3 x consider using `_`
- 3 x redundant closure
- 3 x usage of wildcard import
- 2 x unnecessary map of the identity function
- 2 x this `_` may be rewritten with the `_` operator
- 1 x this argument is passed by value, but not consumed in the function body
- 1 x you seem to be trying to use `_` for destructuring a single pattern. Consider using `_`
- 1 x very complex type used. Consider factoring parts into `_` definitions
- 1 x this expression creates a reference which is immediately dereferenced by the compiler
- 1 x this `_` statement can be collapsed
- 1 x assigning the result of `_` may be inefficient
- 1 x allocating a local array larger than N bytes

#### rust/tcl-cmd-core: 19 findings
- 10 x matching over `_` is more explicit
- 3 x long literal lacking separators
- 2 x using `_` with a constant chunk size
- 1 x redundant closure
- 1 x variables can be used directly in the `_` string
- 1 x items after a test module
- 1 x this function has too many lines

#### runtime/rust (tests/data via tcl-vm): 4 findings
- 3 x more than 3 bools in a struct
- 1 x using `_` with a constant chunk size

#### rust/tcl-engine-tclvm: 2 findings
- 1 x returning the result of a `_` binding from a block
- 1 x this function has too many lines

### runtime/rust `make runtime-rust-lint` (Sol-41)

`cargo clippy --locked --all-targets --features engine -- -D warnings`, with libtommath present. It reports 246 errors on the lib and 289 on the lib test, all of them existing ones; my changes added none. The counts by message below combine both targets:

```
     73 error: the `_`-variant returned from this function is very large
     60 error: useless conversion to the same type: `_`
     27 error: very complex type used. Consider factoring parts into `_` definitions
     27 error: the `_`-variant returned from this closure is very large
     25 error: the borrowed expression implements the required traits
     16 error: called `_` on a `_` value
     10 error: this function has too many arguments (8/7)
      9 error: using `_` on type `_` which implements the `_` trait
      7 error: using `_` over `_`
      4 error: field assignment outside of initializer for an instance created with Default::default()
      3 error: try not to call a closure in the expression where it is declared
      3 error: this function has too many arguments (9/7)
      3 error: the loop variable `_` is used to index `_`
      2 error: unneeded `_` statement
      2 error: this `_` statement can be collapsed
      2 error: redundant closure
      2 error: empty lines after doc comment
      2 error: empty line after doc comment
      1 error: usage of `_` in `_`
      1 error: unnecessary use of `_` to create a slice from a reference
      1 error: unnecessary parentheses
      1 error: unnecessary closure used to substitute value for `_`
      1 error: this function has too many arguments (10/7)
      1 error: this expression creates a reference which is immediately dereferenced by the compiler
      1 error: this `_` has identical blocks
      1 error: this `_` expression can be replaced with `_`
      1 error: this `_` can be collapsed into the outer `_`
      1 error: this OR pattern can be rewritten using a range
      1 error: large size difference between variants
      1 error: digits of hex, binary or octal literal not in groups of equal size
      1 error: derefed type is same as origin
      1 error: current MSRV (Minimum Supported Rust Version) is `_` but this item is stable since `_`
      1 error: called `_` on `_` after checking its variant with `_`
      1 error: `_` is already on the heap, the boxing is unnecessary
```

## 6. lsp-server-wasm / lsp-server-wasi / test-ext-web: fixed

* Reproduced with the exact CI build for `rust/tcl-lsp-server-wasm`: `CARGO_TARGET_DIR=$here/target RUSTFLAGS="-C link-arg=-zstack-size=67108864" cargo build --target wasm32-unknown-unknown --release`, with fat LTO. It failed with `error[E0275]: overflow evaluating the requirement PhantomData<tcl_compiler::ir::WordPart>: Sync` in `tcl_lsp_server_wasm`. The WASI crate failed the same way with `CARGO_PROFILE_RELEASE_LTO=thin`, as the CI functional step uses.
* Fix: `#![recursion_limit = "256"]` with the comment "Retained analysis receipts require deeper auto-trait evaluation." It goes in both crate roots, the same way `tcl-lsp-server` (lib, main, tests), `tcl-lsp-db` and `tcl-compiler` already do it. Nothing needed changing in tcl-compiler.
* Verified:
  * The WASI release build with the fix finishes (thin LTO, 0 warnings).
  * The WASM build with the fix and fat LTO was OOM-killed by the memory cgroup while other agents were building. With `CARGO_PROFILE_RELEASE_LTO=false CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16` it finishes with no E0275. The error shows up the same way with and without LTO: thin LTO reproduced it on WASI.
  * test-ext-web runs `make lsp-server-wasm`, so the same fix covers it.
  * Not run: wasm-bindgen packaging, `make lsp-server-wasm-test`, `make lsp-server-wasi-test`.
* I deleted both `rust/tcl-lsp-server-wasm/target` and `rust/tcl-lsp-server-wasi/target` (1–2 GB each, both created by me) afterwards.

## 7. runtime no-libtommath warnings (rust-tests-doctest, runtime-rust-tests): fixed

Reproduction: `runtime/rust`, with `tmp/tcl9.0.4/libtommath` hidden for my process only (`unshare -m` plus a bind mount of an empty dir) and a scratch `CARGO_TARGET_DIR`. That way the other agents' runtime builds were not affected and the cached build-script output was not poisoned. Running `cargo check --locked --lib --tests` gave 23 lib warnings and 22 lib-test warnings, the same as CI.

Fixes, behaviour unchanged:

| File | Warning | Fix |
|---|---|---|
| builtins.rs | unused import `NativeScalarGetterValue` | `#[cfg(have_tommath)]` on a separate `use` |
| codegen_native.rs | unused `current_interp` import | cfg now matches its only user module: `all(test, have_tommath, not(wasm32))` |
| codegen_abi/original_ingress_tests.rs | `later_command` unused | `#[cfg(have_tommath)]` (its only test is gated) |
| return_options/native_string_access_tests.rs | unused `Code` | `#[cfg(have_tommath)] use crate::interp::Code;` |
| value_ops.rs | unused `number::self` import; unused closure args `current`, `amount` | `number` import gated with `Radix`; `let _ = (current, amount);` in the `not(have_tommath)` arm |
| value_ops/native_jim_index.rs | unused `NativeRegexSource` (tests) | `#[cfg(have_tommath)]` |
| typed_value.rs | 6 unused re-exports | split into a `#[cfg(have_tommath)] pub(crate) use …` |
| typed_value/native_boolean_truth.rs | 6 fns plus `OriginalOps::neutral` dead | `#[cfg_attr(not(have_tommath), allow(dead_code))]` (still used by module tests) |
| expr_error.rs | `ExprError` constructors and `Interp::report_expr_error` dead | the same `cfg_attr` on both impl blocks (some are used by `cfg(test)` code without tommath, so they cannot be cfg-removed) |
| interp.rs | `retain_expression_error_stage` | `cfg_attr` allow |
| interp/native_body_artifact/native_control.rs | fields `program`, `folded` never read | `cfg_attr` allow on the fields |
| interp/native_compilation.rs | field `identity`, `active_native_builtin_identity` | `cfg_attr` allow |
| interp/native_jim_import.rs | `OriginalJimImportWhile` and its methods, `original_import_while` | `cfg_attr` allow |
| native_arithseries.rs `new_series`, obj.rs `jim_character_count` | dead without bignum | `cfg_attr` allow |

Verified:
* Without tommath and with `CARGO_BUILD_WARNINGS=deny`: `cargo check --locked --all-targets` and `--all-targets --features engine` are both clean. The only output is the build-script "libtommath source not found" note.
* With tommath: `cargo check --locked --all-targets --features engine` with warnings denied is clean. The runtime clippy run shows no new unused or dead-code findings.
* `cargo fmt --check` passes in runtime/rust.
* I did not run the runtime test suites themselves.

## 8. spectcl-compat (eval_loader): root cause in tcl-vm (Sol-41), not fixed

`cargo test -p tcl-spectcl --test eval_loader` fails 24 of 28 tests. Every failing test fails the same way: the pack evaluation fails with `execution refused: native command name protocol is unavailable`, so nothing loads.

With a temporary probe test (since deleted; a copy is in the scratchpad as `zz_s43_probe.rs`) I narrowed it to tcl-vm:

* `TclVmEngine::new().compile("set x 1")` refuses. `compile` defines the unit procedure through `Vm::try_define_procedure`.
* A plain `tcl_vm::Vm::new()` with `try_define_procedure("p", &[], "")` refuses with `HostCommandRefusal { reason: "native command name protocol is unavailable", … }` for `p`, `::p` and `::spectcl::unit::1`, with or without `set_native_engine_profile(tcl9.0)`.
* A plain `Vm::new().eval_interactive("proc p {} {return ok}; p")` returns `code=Error`.
* `try_register_native_command("p", …)`, which goes through the same `native_procedure_publication` lookup, returns Ok.
* The tcl9.0 profile resolves to `DialectPoint{Tcl ordinal 3, Canonical}` with `native_name_protocol() == Some(C(V9_0))`, so the profile itself is fine.

So procedure definition is broken in tcl-vm itself after the merge: `embed.rs::define_procedure_internal` reaches a `NativeCommandLookupUnavailable::ProtocolUnavailable` refusal somewhere in the proc path, likely `define_proc_binding_with_jim_key` or body compilation, but not the initial publication. tcl-spectcl and tcl-registry are only the victims, so I changed nothing there and did not weaken any test. For **Sol-41**: reproduce with `Vm::new()` followed by `try_define_procedure("p", &[], "")` or `eval_interactive("proc p {} {}; p")`. The 4 passing eval_loader tests are the ones that never evaluate a pack.

## Extra (coordinator request): VS Code extension jobs: fixed

* The npm pin check passes in CI ("Active npm: 12.2.0"). The `packageManager`, `.nvmrc` and `engines` pins match origin/rust exactly. The real failure was the next step, `make check-vscode-test-partitions`.
* Cause: commit a372766c36 on `work` added `currentDiagnostics.test.js`, taking the manifest to 108 files, without bumping `scripts/dev/test-vscode-test-partitions.sh`, which still said 107. Later, 3d92614629 raised the identity counts to 999/998 in the manifest and in `test-vscode-test-partitions.sh` / `verify-vscode-test-partitions.mjs`, but not in `scripts/dev/verify-vscode-test-results.mjs`, which still said 973/972. The merge did not cause this; it was already broken on `work` before the merge.
* Fix: the script now expects 108 files, and `verify-vscode-test-results.mjs` expects 999/998.
* Verified: `make check-vscode-test-partitions` returns rc=0, with all five contracts passing. If the real run's identity count differs from 999, the result verifier in test-ext will report it.

## Housekeeping

* Deleted my scratch target dirs, the wasm/wasi target dirs, my `eval_loader` and probe test executables in `target/debug/deps`, and `rust/tcl-spec-studio/web/node_modules`.
* My aborted `cargo build -p xtask` (stopped by the disk guard; tcl-vm also fails to compile) left shared dev-profile rlibs in `target/debug/deps`. I did not delete them because other cargo processes were running.
* Observation, not changed: the `lsp-server-wasm` path filter in `.github/workflows/ci.yml` (around line 297) does not list `rust/tcl-host-c-abi/*`, which is now in that crate's lock closure. No gate checks it.
