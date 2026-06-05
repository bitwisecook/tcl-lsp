# EXP-BIGNUM — the numeric tower's bignum representation (evidence)

Throwaway probes for the bignum-rep decision recorded in
`docs/design/runtime/rust-runtime-port.md` (EXP-BIGNUM). Run against the
reference libtommath bundled in `tmp/tcl9.0.3/libtommath` with its Tcl wrapper
header `tmp/tcl9.0.3/generic/tclTomMath.h`.

## `lt_layout.c` — the `mp_int` ABI layout across targets

```sh
cd tmp/tcl9.0.3
# native (defaults to MP_64BIT on a 64-bit host)
clang -Igeneric -Ilibtommath runtime/.../lt_layout.c -o /tmp/lt_n && /tmp/lt_n
# wasm32 default (libtommath picks MP_32BIT off the 32-bit pointer)
zig cc --target=wasm32-wasi -Igeneric -Ilibtommath lt_layout.c -o a.wasm && wasmtime a.wasm
# wasm32 + forced MP_64BIT (the chosen, wasm-matched config)
zig cc --target=wasm32-wasi -DMP_64BIT -Igeneric -Ilibtommath lt_layout.c -o a.wasm && wasmtime a.wasm
```

Result:

| target / config            | `mp_digit` | `mp_int` | offsets (used/alloc/sign/dp) |
|----------------------------|-----------:|---------:|------------------------------|
| native (MP_64BIT)          | 8          | 24       | 0 / 4 / 8 / 16               |
| wasm32 default (MP_32BIT)  | 4          | 16       | 0 / 4 / 8 / 12               |
| **wasm32 -DMP_64BIT**      | **8**      | **16**   | 0 / 4 / 8 / 12               |

Conclusion: forcing `MP_64BIT` on wasm32 keeps the struct at 16 bytes (only the
heap digit array widens to 8-byte/60-bit limbs) → native-i64 arithmetic with the
`mp_int` ABI unchanged for extensions (they compile against the same
`tclTomMath.h`). libtommath also compiles + runs on wasm32 (this probe did).

## `lt_arith.c` — arithmetic + floor-division sanity

Computes `2**100`, inspects `used`/`fits_i64`, and shows raw `mp_div(-7,2)`
(C-truncation → q=-3 r=-1; Tcl floor-adjusts to q=-4 r=1). The full multi-file
link needs libtommath's per-file `#ifdef BN_*_C` build toggle (a Track-3
whole-program-link recipe detail), so this is kept as the arithmetic-shape
reference, not a CI gate.
