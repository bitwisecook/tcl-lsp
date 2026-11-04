# Math-function result-class ownership: #2131

This receipt checks the published ownership change: `MathFuncSpec` carries a
`MathResultClass`, and the compiler's math-function classifier reads
`tcl_syntax::expr::mathfunc::result_class`. It does not claim that nested
expression results are inferred through the public `CompilationUnit` API.

## Source pins

The tracked source hashes below match at `ad4d43160916c343e8376763cfbefba9fc184ad5`,
`02a9c35c6`, and `34b4f160e0f326612c5e01600eb1da4050e4464f`. Full pins are in
[source-snapshot.json](source-snapshot.json) and
[source-files.sha256](source-files.sha256).

| Source | SHA-256 |
| --- | --- |
| `rust/tcl-syntax/src/expr/mathfunc.rs` | `c7b8d65e2a0f5c9f26ef402e7c5c86a5b7204b5f562b4b0b4032d51e6875cf61` |
| `rust/tcl-compiler/src/type_infer.rs` | `2572d7a04f17c84846421b634a7030c6da4cb35d8cc4a2aa15c7dba92950ad1a` |
| `rust/tcl-compiler/src/compilation_unit.rs` | `5734cdcb48b4ad50b16fd53802c8249e975fe42166e834fade01a0cabb1afc4e` |
| `rust/tcl-registry/src/mathfunc.rs` | `ef97836bd243cfc6b6aca0337210234111a6cada9a04ed0e7366464c5aa7204c` |
| `rust/tcl-compiler/tests/type_propagation.rs` | `f58a2d0a037910330dc9acb0dcd6d057084b9e4148101b9ac9a22f21e2e4bdc0` |

At `type_infer.rs:448–452`, `expr_call_type` imports the shared `MathResultClass`
and `result_class` and maps classes to compiler types. The helper and its AST
inference caller are `#[cfg(test)]`; the public `CompilationUnit` probes below
therefore do not establish production nested-expression inference.

## Shared-owner and compiler controls

The standalone fixed control in [shared-class-test.rs](shared-class-test.rs)
was compiled directly against the frozen `tcl_syntax` rlib and passed 1/1. It
checks `MathFuncSpec.result_class` and `result_class(name)` for Float (`ceil`,
`floor`, `trunc`), Int (`int`, `round`), Bool (`isnan`, `signbit`), Numeric
(`abs`, `max`), and Any for an unknown name. Its test binary is SHA-256
`3300bfa5fb1194b6b2dc8e9bc8b0a628ec0703f4765ec26f74f7a7d20e69a918`.

The exact direct-link command was:

```sh
rustc --edition=2024 --test /tmp/lsp-2131-shared-class.rs \
  -L dependency=/workspace/tcl-lsp/tmp/lsp-bindings/target/debug/deps \
  --extern tcl_syntax=/workspace/tcl-lsp/tmp/lsp-bindings/target/debug/deps/libtcl_syntax-d55a159e25981def.rlib \
  -o /workspace/tcl-lsp/tmp/lsp-2131-proof/shared-class-test
/workspace/tcl-lsp/tmp/lsp-2131-proof/shared-class-test --nocapture
```

The syntax rlib SHA-256 was
`818e667c32831729b55dba1786fe885f72a00daf3b4d33b7c99baa271b2e1eb9`.
The compiler rlib (`d4012741853e0713f64960e6b46711abf7668a50abe74092235b4bb51251de66`)
and registry rlib (`7486b2fd6da0c9a47c958da75f464672f3e56232dac15773aef6b8fdd8f3b856`)
were also held unchanged. Their 145-record dependency manifest is
[compiler-dependencies.json](compiler-dependencies.json).

The focused compiler unit test passed 1/1:

```sh
/workspace/tcl-lsp/tmp/lsp-values-clean-target/debug/deps/tcl_compiler-1169709807dfb4a1 \
  type_infer::tests::math_function_calls_infer_their_return_type --exact --nocapture
```

That binary's SHA-256 is
`4a20809c03c1daeaaf39796ee84c33792f652df41628c256ace63ab1a3fff17b`; its
compiler rlib SHA-256 is
`ebaec4453a8732237767f2bf75831c8cb232b8522e8a9cb6249ead4b078cd51b`.
Both before/after records are preserved in
[unit-artifacts-before.sha256](unit-artifacts-before.sha256) and
[unit-artifacts-after.sha256](unit-artifacts-after.sha256); they are identical.
The binary was built from `a12227e33` with the sealed list3 WIP manifest
`ec4d3cb1997c89bcf0aa6ba259c82a90c3adf35871841c028114cd91d3cc07eb`; the
math-function source hashes above are unchanged in that snapshot.

## Native Tcl and Jim controls

The exact fixed script is [oracle.tcl](oracle.tcl), and all captured output is
in [oracle.log](oracle.log). It records displayed values, caught evaluation
errors, and `string is` predicate results. Those predicates overlap and may
change an object's representation; this run does not inspect physical
representations. `ERROR` records only the caught outcome and does not establish
why evaluation failed or whether a metadata row exists. Tcl 8.4, 8.5, and 8.6
display `ceil(3.14)` as `4.0` with `double=1`, and `int(3.14)` as `3` with
`integer=1`. Tcl 9.0 additionally displays `isnan(1.0)` as `0` with
`boolean=1`; Tcl 9.1 displays `trunc(2.5)` as `2.0` with `double=1` and
`signbit(-1.0)` as `1` with `boolean=1`. Jim `0.84-10-g82fb4d0` records the
same displayed values and predicate results for `ceil` and `int`.

## Public-API limit

The attempted public `CompilationUnit` top-level probe is preserved in
[public-api-probe.rs](public-api-probe.rs) with its output in
[public-api-probe.log](public-api-probe.log). It produced no Tcl type for `z`.
An earlier procedure-shaped probe of `expr {ceil($x)}` returned `UNKNOWN` for
the function result. Neither probe is a passing public-inference control, and
no public API coverage is claimed.

## Validation limits

The evidence file hashes, including the exact scripts and captured logs, are
in [receipt-files.sha256](receipt-files.sha256). Relevant compiler and syntax
source hashes, normal rlib hashes, and unit-binary hashes are recorded above
and in the included dependency and before/after manifests.

This receipt verifies shared ownership and the existing compiler consumer's
fixed-class expectations. It does not claim a full Cargo test, Clippy, or
workspace gate; none was run for this check. No product source was changed by
the verification.
