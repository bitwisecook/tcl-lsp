# Selected C86 object-worker proposal

Five prospective owners: registry commands/tcl/clock_.rs + tcl/mod.rs; cmd-core clock.rs; runtime cmd_clock.rs + value_ops.rs.
Registry owns C86-only worker admission, actual NULL constructor hook and shipped ObjCommand backing independently of purity; other profiles remain absent from this backing.
Core clock.rs owns one generic original-object parser and measured C86 Gregorian/Julian/ISO calendar/transition algorithm; no compiler/analyser command-name branch or formatter substitution.
Use existing ValueOps original List element/index access and checked Dict constructor, preserving reached row traversal, timezone-name identity and literal key order.
A small core NativeClockDateOps: ValueOps boundary reaches the existing central NativeScalarGetterKind::Int for changeover/offset; ValueOps::as_int already reaches Wide.
Its runtime implementation in value_ops.rs reuses ValueCoercion::enter/scalar(Int), actual native integer ABI and current operation/receiver currency, returning the original typed getter/error-code cause.
After both top-level numeric getters, inspect existing native_object_snapshot Numeric(Number::Big) to reject the actual seconds primary; do not infer bignum from returned wrapped Wide value/string.
Preserve the pool literal overflow message with NONE error code at that stage; general Wide/Int overflow retains each original central scalar diagnostic.
Shared parser creates the genuine checked Dict from runtime-owned original field-key/CE/BCE literals plus fresh numeric fields and borrowed original timezone-name child.
cmd_clock.rs adds exactly one Rc-owned registration/delete callback to current two workers, retaining needed actual literals; three implemented runtime workers do not simulate seven C worker holds.
Runtime tests must verify selected C86 spec+actualBuiltin ObjCommand backing, absent other profiles, NULL hook, raw name identity, Dict primary, key-sharing, result retirement and three-owner deletion order.
Fixed scalar/list/error/transition/calendar rows are the acceptance inputs; arity and conversion order must agree with raw C86 controls, not C85/C9 convenience behaviour.
Checked arithmetic must honestly refuse unmeasured native overflow/storage extents rather than inventing C signed-overflow results; add measured boundaries before wider acceptance.

Empty tzdata requires genuine host C-localtime support, unavailable from current Clock::local_offset_secs (default0 and no native name/year mapping/currentness contract).
Initial five-owner scope may operationally refuse that reached branch via explicit unavailable native localtime capability; it cannot return UTC/+0000 based on a profile or synthetic fixture.
Full localtime backing would require separately enumerated platform/native-host capability owners and real year/range/TZ/currentness evidence before source authorisation.
Remaining Oldscan, ConvertLocalToUTC and both GetJulianDay workers remain unsupported; native environment/guest env-array synchronisation remains a separate three-row difference.
No old seven-worker pool lifetime or all.tcl suite-pass claim follows; original orchestration stays before suites until actual next worker is implemented/reviewed.
