# Primitive native scalar getter stages

These deterministic direct-object probes cover Tcl 8.4.20, 8.5.19, 8.6.18,
9.0.4, 9.1.0 and math-enabled Jim 0.84 at
`5bac7c99ad65864c87da513e22e2f01703fa4e03`. They call primitive getters;
expression parsing, implicit `sqrt`, source numeric syntax and mathematical
truth are separate protocols.

The committed output files record 312 fresh Wide/Double values and cache
transitions, 77 Boolean/storage transitions, 105 existing C caches and
60 C NUL/storage getter calls. `jim-range-state.txt` adds eight explicit
clear/ERANGE and cached-integer boundary contrasts. Getter return bits are
independent of the retained cache: C8.5/8.6 can return a wrapped Wide while
retaining a larger bignum; explicit C8.4 Double converts WideInt, while its
implicit scalar math preserves it. Jim Double parses decimal integers first
and retains the exact integer under a coerced-double cache.

Raw String and C ByteArray are distinct original storage kinds. ByteArray
string materialization encodes zero as `c0 80` and high bytes as their UTF-8
characters. The equivalent raw String retains embedded NUL. C8.5+ primitive
numeric conversion can consume a NUL prefix of that raw String; the ByteArray
materialization changes the actual parser input and fails. Boolean word
parsing has a separate byte extent and cache protocol.

After building each pinned Tcl Unix static library, run from the repository:

```sh
native_tcl_root="$PWD/tmp/tcl8.6.18"
native_tcl_version=8.6
native_getter_dir="$PWD/rust/tcl-syntax/tests/data/native_scalar_getters"
native_getter_build=$(mktemp -d)
for native_probe in double wide boolean cached-getter nul-getter; do
    cc -std=c11 -I"$native_tcl_root/generic" -I"$native_tcl_root/unix" \
        "$native_getter_dir/c-$native_probe-probe.c" \
        "$native_tcl_root/unix/libtcl$native_tcl_version.a" \
        -lpthread -ldl -lm -lz -o "$native_getter_build/$native_probe"
    "$native_getter_build/$native_probe" > "$native_getter_build/$native_probe.txt"
done
```

Compare `double`, `wide`, `boolean`, `cached-getter` and `nul-getter` output
with the respective `double-VERSION`, `wide-VERSION`, `boolean-VERSION`,
`cached-VERSION` and `nul-VERSION` files. Repeat for the other pinned releases.
For the pinned Jim source/build directory, set `native_jim_root` and run:

```sh
for native_probe in double wide boolean range-state; do
    cc -std=c11 -I"$native_jim_root" \
        "$native_getter_dir/jim-$native_probe-probe.c" \
        "$native_jim_root/libjim.a" -ldl -lm \
        -o "$native_getter_build/jim-$native_probe"
    "$native_getter_build/jim-$native_probe" > "$native_getter_build/jim-$native_probe.txt"
done
rm -rf "$native_getter_build"
```

Jim expected files are `jim-double-probe.txt`, `jim-wide-native.txt`,
`jim-boolean-native.txt` and `jim-range-state.txt`. The sequential Wide probe
intentionally retains native `errno` across cases: an unsigned overflow
precedes its signed-MIN discriminator. The paired range-state probe separately
supplies clear versus ERANGE native state, and proves that existing Int cache
bypasses it. A portable adapter must not use its Rust process errno as evidence
of native guest history. Without independently retained state, fresh signed
boundaries abstain with a typed capability result; this is not a guest parse
error. Existing cached integers remain usable.

String materialization and custom object effects precede the pure getter
recipe. A required original string must be prepared before changing the cache;
cache changes can precede a guest error. None of these records supplies object
identity, callback-effect closure, compiler permission or expression error
presentation. Original native proof manifests with source/library/binary
hashes are retained in `.proofs/2286-packages-dialects/double-getter`.
