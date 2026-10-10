# Bignum representation and libtommath build probes

Layout and arithmetic probes for the numeric tower. Run against the
reference libtommath bundled in `tmp/tcl9.0.4/libtommath` with its Tcl wrapper
header `tmp/tcl9.0.4/generic/tclTomMath.h`.

## `lt_layout.c` — the `mp_int` ABI layout across targets

```sh
# Run from the repository root. Native 64-bit hosts default to MP_64BIT.
cc -Itmp/tcl9.0.4/generic -Itmp/tcl9.0.4/libtommath \
    runtime/rust/experiments/bignum/lt_layout.c -o /tmp/lt-layout-native
/tmp/lt-layout-native

# Use the provisioned wasi-sdk clang for the two wasm32 layouts.
"$WASI_SDK_PATH/bin/clang" -Itmp/tcl9.0.4/generic -Itmp/tcl9.0.4/libtommath \
    runtime/rust/experiments/bignum/lt_layout.c -o /tmp/lt-layout.wasm
wasmtime /tmp/lt-layout.wasm
"$WASI_SDK_PATH/bin/clang" -DMP_64BIT -Itmp/tcl9.0.4/generic \
    -Itmp/tcl9.0.4/libtommath runtime/rust/experiments/bignum/lt_layout.c \
    -o /tmp/lt-layout-64.wasm
wasmtime /tmp/lt-layout-64.wasm
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

## `lt_arith.c` — arithmetic + floor-division (builds + runs, native **and** wasm32)

Computes `2**100`, inspects `used`/`fits_i64`, and shows raw `mp_div(-7,2)`
(C-truncation → q=-3 r=-1; Tcl floor-adjusts to q=-4 r=1).

### Build recipe

Tcl's bundled libtommath is wired into Tcl's stubs (`tclTomMath.h` renames every
`mp_*` to `TclBN_*` and tangles the `MP_INIT_INT` code-gen templates when its own
`.c` files are compiled). Build **pristine** instead, with
`-DTCL_WITH_EXTERNAL_TOMMATH` (switches `tommath_private.h` to the real
`tommath.h` and skips the `TclBN_*` renaming) and `-DLTM_ALL` (enables every
file's `BN_*_C` guard):

```bash
# Run from the repository root.
mapfile -t bignum_sources < <(
    rg --files tmp/tcl9.0.4/libtommath -g '*.c' | rg -v 'bn_deprecated|rand|prime'
)
cc -DTCL_WITH_EXTERNAL_TOMMATH -DLTM_ALL -Itmp/tcl9.0.4/libtommath \
    runtime/rust/experiments/bignum/lt_arith.c "${bignum_sources[@]}" \
    -o /tmp/lt-arith-native
/tmp/lt-arith-native
"$WASI_SDK_PATH/bin/clang" -DTCL_WITH_EXTERNAL_TOMMATH -DLTM_ALL -DMP_64BIT \
    -Itmp/tcl9.0.4/libtommath runtime/rust/experiments/bignum/lt_arith.c \
    "${bignum_sources[@]}" -o /tmp/lt-arith.wasm
wasmtime /tmp/lt-arith.wasm
```

Both print `2**100 = 1267650600228229401496703205376` with **2 limbs**
(confirming MP_64BIT's 60-bit limbs). `rand`/`prime` are excluded — the integer
tower needs no RNG/primality (they pull `s_read_arc4random`/`mp_rand` externals);
they're added back, with the right RNG config, only if an extension needs them in
the whole-program-link build. This recipe is what the runtime's `build.rs` drives
to link `mp_*` for the `TCL_BIGNUM_TYPE` FFI.
