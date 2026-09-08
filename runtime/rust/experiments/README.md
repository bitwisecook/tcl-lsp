# `runtime/rust/experiments/` — throwaway data-structure experiments

Per the porting method — when a value type has a real crossover question,
**measure candidates compiled to WASM under `wasmtime`** (the real target —
constant factors differ from the host) before committing to a representation.

These programs are **throwaway**. The decision each one settled is recorded in
the module that implements it (`dict.rs`, `cmd_string.rs` / `obj.rs`,
`bignum.rs`); the code here is the reproducible evidence behind it.

Each file answers one question, stated in its header.

| Experiment | Question | Decision |
|---|---|---|
| `dict_rep.rs` | Which structure for the insertion-ordered dict? | ordered `Vec` + FNV-hash index (EXP-DICT) |
| `string_rep.rs` | Char-access + append without an O(n²) cliff? | ASCII fast path + lazy char-offset index; capacity-backed append (EXP-STRING) |
| `bignum/` | Which bignum representation, and does libtommath build for wasm32? | libtommath `mp_int` with `-DMP_64BIT` on wasm32 (EXP-BIGNUM); see its own README |

Run an experiment:

```
rustc -O --edition 2021 --target wasm32-wasip1 dict_rep.rs -o /tmp/x.wasm && wasmtime /tmp/x.wasm
rustc -O --edition 2021                        dict_rep.rs -o /tmp/x       && /tmp/x   # native contrast
```
