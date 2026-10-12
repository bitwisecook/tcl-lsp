# Reached original operand numeric caches

`c-operand-cache-probe.c` retains the actual object returned by a native command,
then uses it as a variable operand in numeric `<`, runtime `lrange`, scalar
`lindex`, and `lindex` after explicitly priming the original index as a List.
It records 210 actual original-object cache outcomes across C Tcl 8.4.20,
8.5.19, 8.6.18, 9.0.4 and 9.1.0. `manifest.json` retains source, native library,
executable and output hashes. Use the same static-library command as the
adjacent `native_scalar_getters` probes to rebuild it.

These stages are not primitive Wide extraction. C8.4's numeric comparison of
the huge integer spelling succeeds through a fallback while leaving its
original String cache; later C publishes Bignum. A current grouped List index
keeps that original List while converting a separately extracted child.
An `end-1` index has its own offset cache, not a numeric cache. Runtime getters
are also distinct from bytecode immediate literal-index instructions.

The registry recipes are conditional on an independently validated integer-only
input and an actual successful numeric getter continuation. They do not turn
ordinary handler success into proof of that branch, close object callback
effects, normalize original string bytes or supply a concrete integer value.
C8.4 expression preparation and Jim are excluded from the initial GetNumber
recipe; its selected C8.5+ category conservatively includes Integer, Bignum
and Double caches. The extra native extension cases supply a genuine Double
cache while retaining the original integer spelling through public object
fields. `GetNumber` preserves that Double, while lindex can take the grouped
child fallback and leave the original object as List. Its initial cache recipe
therefore requires a current original integer cache, not just integer bytes.

`c-original-class-probe.c` adds 30 same-object observations: original String,
native Integer, List, ByteArray, Dict (C8.5+) and Double before numeric `<`.
Each original object stays retained while its variable operand reaches native
comparison. String, one-element List and ASCII ByteArray containing `2` become
Int; native Integer stays integer and Double stays Double. Dict `2 3` returns
normally through string comparison without changing Dict, so normal relational
completion does not prove a numeric getter. C8.4 has no Dict constructor here;
that control retains the two-element List instead. The separate
`original-class-manifest.json` records all native source/library/output hashes.
Rebuild with `cc -std=c11 -I<tcl>/generic -I<tcl>/unix
c-original-class-probe.c <tcl>/unix/libtcl<major.minor>.a -lpthread -ldl -lm -lz`.

`NativeOperandNumericCacheProduction::with_original_cache_class` refines only
the retained C8.5+ expression GetNumber stage after an independently proved
normal integer-only numeric conversion. Its input describes the current cache
of the exact original object, separately from semantic types or known spelling.
Proved String/List/ByteArray/Integer yields Int; proved Double yields Double;
Dict, unknown numeric subtype and unproved cache class retain Numeric. String
normalization, original identity and conversion effects remain separate proofs.
