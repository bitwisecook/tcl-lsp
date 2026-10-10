# naming.substitution.namespace-and-local-context

Kind: `native-observation`

## Problem statement

A namespace eval may read an existing global variable when an unqualified set does not create the expected namespace cell. That setup can make a correctly current template appear to reuse the wrong namespace. Fresh templates, explicit declarations, independent qualified reads and actual varFrame/cache booleans distinguish setup fallback from incorrect cache currency; procedure contexts separately test borrowed local names.

## Question

After explicit A/B namespace declarations, do fresh and reused direct templates resolve A, B and ROOT correctly, and what localCache/procPtr fields accompany P/Q procedure contexts?

## Conclusion

v6 independently reads A=41, B=42 and ROOT=524f4f54, and all C5 fresh/reused direct A/B substitutions return 41/42. P, repeated P, Q and returned P values match their actual formal values. On C8.6/C9.0/C9.1 the original substcode changes cached namespace with the actual varFrame and retains a localCache in procedure callbacks while procPtr remains absent. Earlier unqualified-set variants return ROOT and do not prove a wrong cache namespace.

## Scope

v6 explicit variable x A/B and independent qualified reads, fresh/reused direct original templates, actual varFrame/cache boolean observations, P/Q layouts. v2-v5 are separately retained setup controls. No numeric address or identity is inferred from equal epoch fields.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel). Build: Actual executed v6 probe 01ca0f3ddb0b292c94e267c9341f7e57bf29bb041bfd45a795bd051050241892; matching header 824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf, static library 532be0a794ca8277c5ad979a227c38600f27c3c91d436e5d3731d661c271fe47, Makefile 0fb0c580d2402093bb212ad340ea87f88822aa5844a8114e112bec4c990411b1 and private-header/configured-flag receipts retained. Earlier variants retain their own build hashes.. Channel: Counted Tcl_NewStringObj template bytes supplied to stock subst through Tcl_EvalObjv, and independently direct Tcl_SubstObj in v3-v6. Setup uses ASCII NUL-terminated Tcl_Eval source. Header windows precede the reached result string getter.. Dialect: Tcl.

v6 declared A/B/root hex41/42/524f4f54; fresh/reused direct A/B resolve41/42; P/Q match actual formal values. Direct simple-variable result string primary.

### tcl8.5

Status: `observed`. Version: 8.5.19 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel). Build: Actual executed v6 probe 3089d7fb65500eed6e072d99213cf2d9a2effb815b3f927f34338999c613ae86; matching header c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5, static library 94313727ea507b1b2b00018062a0f57587458a3509c37f2e4b351dd58d0592dc, Makefile 26ae775d2e4657ecfcb45421cc77c4c26170cb2a21985fbb002eb4791bc766a5 and private-header/configured-flag receipts retained. Earlier variants retain their own build hashes.. Channel: Counted Tcl_NewStringObj template bytes supplied to stock subst through Tcl_EvalObjv, and independently direct Tcl_SubstObj in v3-v6. Setup uses ASCII NUL-terminated Tcl_Eval source. Header windows precede the reached result string getter.. Dialect: Tcl.

v6 declared A/B/root hex41/42/524f4f54; fresh/reused direct A/B resolve41/42; P/Q match actual formal values. Direct simple-variable result none primary before getter.

### tcl8.6

Status: `observed`. Version: 8.6.18 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel). Build: Actual executed v6 probe a41fc11b43cf4611115f3234ffe29bd4732433aaffe41525d108bca172576a5d; matching header aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245, static library 980630e3a7d37f82322fda1a7e45a85b86d8d905cb395a2aa0c083b3c7eaa8cb, Makefile 8afb8697cb70b90518876861086bdb43f6e31b5e96e6d8091ae7b1de33d90d7e and private-header/configured-flag receipts retained. Earlier variants retain their own build hashes.. Channel: Counted Tcl_NewStringObj template bytes supplied to stock subst through Tcl_EvalObjv, and independently direct Tcl_SubstObj in v3-v6. Setup uses ASCII NUL-terminated Tcl_Eval source. Header windows precede the reached result string getter.. Dialect: Tcl.

v6 declared A/B/root hex41/42/524f4f54; fresh/reused direct A/B resolve41/42; P/Q match actual formal values. Direct simple-variable result none primary before getter.

### tcl9.0

Status: `observed`. Version: 9.0.4 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel). Build: Actual executed v6 probe 77a13410411609c8d0b087639070801ec8b09b7d0afa40c734bb819737eb62c3; matching header eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a, static library dace08db925224497714ba86969fabb51c05a78f2bda22db6c05f795c3a70ae4, Makefile 69f1915c208d66c7e38e6871c7f51c8f7be7c2138b45a5361641f9e91854fea5 and private-header/configured-flag receipts retained. Earlier variants retain their own build hashes.. Channel: Counted Tcl_NewStringObj template bytes supplied to stock subst through Tcl_EvalObjv, and independently direct Tcl_SubstObj in v3-v6. Setup uses ASCII NUL-terminated Tcl_Eval source. Header windows precede the reached result string getter.. Dialect: Tcl.

v6 declared A/B/root hex41/42/524f4f54; fresh/reused direct A/B resolve41/42; P/Q match actual formal values. Direct simple-variable result none primary before getter.

### tcl9.1

Status: `observed`. Version: 9.1.0 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel). Build: Actual executed v6 probe 74d4c16f0747ad8d33b827cacaca26eeaf1e5f90ce6eff70892d83871e593755; matching header 30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950, static library 513028850edf096856a29e4e16869aac91cba2ef75c3924aaaaeeb6e45e167db, Makefile c1ecfb5a77697f0dc6f1057f63ff7aabd75b444f62b5954480aff01ff0565ab1 and private-header/configured-flag receipts retained. Earlier variants retain their own build hashes.. Channel: Counted Tcl_NewStringObj template bytes supplied to stock subst through Tcl_EvalObjv, and independently direct Tcl_SubstObj in v3-v6. Setup uses ASCII NUL-terminated Tcl_Eval source. Header windows precede the reached result string getter.. Dialect: Tcl.

v6 declared A/B/root hex41/42/524f4f54; fresh/reused direct A/B resolve41/42; P/Q match actual formal values. Direct simple-variable result none primary before getter.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

This exact C substitution/private-header question was not tested on this provider.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

This exact C substitution/private-header question was not tested on this provider.

## Exact evidence

- `e0` (input): [rust/tcl-registry/tests/data/native_substitution_owner/inputs.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/inputs.json). SHA-256 `364bf581c1b5d6b4e6b2b321ae4005d553f7c84fe120ca17c0c61e443022c307`. Complete ten native template byte arrays as hexadecimal. v1 shortened counts are separately encoded in its original probe.
- `e1` (input): [rust/tcl-registry/tests/data/native_substitution_owner/v1/probe.c](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v1/probe.c). SHA-256 `b633702ddc869247bc9b00acb1f6068a72bf2b36066c21595d5e879a72f6bfc4`. Exact immutable v1 translation unit and original counted template/setup operations.
- `e2` (input): [rust/tcl-registry/tests/data/native_substitution_owner/v2/probe.c](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v2/probe.c). SHA-256 `eb0eb2cc580340b982745d616d9d9db9c6fc36bb4c0171c2d275f0fe9c1bdd9c`. Exact immutable v2 translation unit and original counted template/setup operations.
- `e3` (input): [rust/tcl-registry/tests/data/native_substitution_owner/v3/probe.c](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v3/probe.c). SHA-256 `7d7cedcd46b03c3634d8c974e9557a21333a2424c75a25aaec3603b5feac3538`. Exact immutable v3 translation unit and original counted template/setup operations.
- `e4` (input): [rust/tcl-registry/tests/data/native_substitution_owner/v4/probe.c](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v4/probe.c). SHA-256 `9a0dd266956f6bbd7a283d95b8942681642135dd5e8b1edbdc5a3828ac562b8b`. Exact immutable v4 translation unit and original counted template/setup operations.
- `e5` (input): [rust/tcl-registry/tests/data/native_substitution_owner/v5/probe.c](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v5/probe.c). SHA-256 `f42aa9712f61038336f07e84b6b4182a8e0e4bde03621457f0c1e742d605139a`. Exact immutable v5 translation unit and original counted template/setup operations.
- `e6` (input): [rust/tcl-registry/tests/data/native_substitution_owner/v6/probe.c](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v6/probe.c). SHA-256 `1d19fec43fcd1f3de7809dcce3f9fead2dbac32de27c3711a01dcd9aefb08f56`. Exact immutable v6 translation unit and original counted template/setup operations.
- `e7` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/manifest.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/manifest.json). SHA-256 `762b4287fd6746d901efbfb000d1bc4e50012af00d0260ca9364e395f88377f4`. Finite corpus provider mapping, variant input/setup/process limits and source excerpts.
- `e8` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v1/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v1/8.4.20/receipt.json). SHA-256 `2e6107045ad05496c4ddde80919d37d225243a5a08493b21f443dbfea53c0338`. Original v1 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e9` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v1/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v1/8.4.20/stdout.tsv). SHA-256 `946299440d249822ac41186d960ade337565ed387198ddd79e615876225ca914`. Exact v1 reached raw stream, 170 rows; exit 0.
- `e10` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v2/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v2/8.4.20/receipt.json). SHA-256 `a0b8dd5601f2af48d8b0594a46ab92c7f2b007f89d75d4d6d38a1e1d6ef78772`. Original v2 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e11` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v2/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v2/8.4.20/stdout.tsv). SHA-256 `4415d731247a9ae6b3489aede0770a5a37f9d0fa37c0d49d820ed791ddf35e22`. Exact v2 reached raw stream, 343 rows; exit 0.
- `e12` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v3/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v3/8.4.20/receipt.json). SHA-256 `c08ef41b9994b9b9ad40c856d9479b05d7042277f2c10122b0be4674cca9f3aa`. Original v3 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e13` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v3/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v3/8.4.20/stdout.tsv). SHA-256 `0380393bc927b48a9215fd5c169e8bfa566ad962406e6dbe69eca6c8bee13582`. Exact v3 reached raw stream, 355 rows; exit 0.
- `e14` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v5/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v5/8.4.20/receipt.json). SHA-256 `da56a9b5f7123fa08e0a2cbb9bb76e906e41addc057cce76bdb825a81680511f`. Original v5 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e15` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v5/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v5/8.4.20/stdout.tsv). SHA-256 `380f91656ef8bf439cbd91e1dff0cb42dcb11c732d421faf732a40757827e515`. Exact v5 reached raw stream, 538 rows; exit 0.
- `e16` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v6/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v6/8.4.20/receipt.json). SHA-256 `5436df2cea8adcfc91031f1fa630d2dc1075fe3b616b61346d916d2c98e03f5f`. Original v6 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e17` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v6/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v6/8.4.20/stdout.tsv). SHA-256 `f1fda94901ca4e83efb558cf3ae3607993be0494da315e7cef0b14edc5753770`. Exact v6 reached raw stream, 539 rows; exit 0.
- `e18` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v1/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v1/8.5.19/receipt.json). SHA-256 `6bc565f5e980db54b9af199a86fed570cef16f9252546088da8bb9798f0e0fe1`. Original v1 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e19` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v1/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v1/8.5.19/stdout.tsv). SHA-256 `405d9d40a60bbd47e55d894b0c6cf3dda97e39e9a5ce2dadfb48f81922de89d1`. Exact v1 reached raw stream, 170 rows; exit 0.
- `e20` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v2/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v2/8.5.19/receipt.json). SHA-256 `e8fbd9242c8a60db870668b1bf436de3106a3390470869bba2c50eec2dd2311f`. Original v2 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e21` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v2/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v2/8.5.19/stdout.tsv). SHA-256 `0e369ca4932ec0fde7fb9cebc58b065d30bfb3c59e37edf6494495dff8ef00b5`. Exact v2 reached raw stream, 343 rows; exit 0.
- `e22` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v3/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v3/8.5.19/receipt.json). SHA-256 `b4654083aa62a4c34989179d959527a11b337c90d0cfeaf51f0a8e746b18d8ad`. Original v3 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e23` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v3/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v3/8.5.19/stdout.tsv). SHA-256 `da37ca2b7271df9e81b45357b424f4437a4c210ce62126192bea0d3788cf9060`. Exact v3 reached raw stream, 355 rows; exit 0.
- `e24` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v4/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v4/8.5.19/receipt.json). SHA-256 `d72ec93a6e53b3e51c273362848fcd29ee34e18213745490f12a53e571873e6d`. Original v4 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e25` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v4/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v4/8.5.19/stdout.tsv). SHA-256 `b286bd994adaac3cf7e500d00cf9b7ef4d2874fc74e01f469261446a952e3149`. Exact v4 reached raw stream, 538 rows; exit 0.
- `e26` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v5/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v5/8.5.19/receipt.json). SHA-256 `92798c4cbee68e6379088292b819fc927bdc451a6f02926f5988ed6126e39d5e`. Original v5 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e27` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v5/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v5/8.5.19/stdout.tsv). SHA-256 `c6d48b47f0d7ade421711661fcc7eadfbc04a571ff1b00116e850c22277fac26`. Exact v5 reached raw stream, 538 rows; exit 0.
- `e28` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v6/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v6/8.5.19/receipt.json). SHA-256 `83091afccce1641bb647a41d4ad0b87b2b55ae9d4115ee0d60defb01f955d624`. Original v6 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e29` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v6/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v6/8.5.19/stdout.tsv). SHA-256 `8705e95e1c39fefaf8b9c248ce092630926001d65a6987685be180a5274f5440`. Exact v6 reached raw stream, 539 rows; exit 0.
- `e30` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v1/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v1/8.6.18/receipt.json). SHA-256 `18c2b5684e496a25236828fb78710b0aa2355f4ce0b39a69c06305e649d985de`. Original v1 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e31` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v1/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v1/8.6.18/stdout.tsv). SHA-256 `d93292126239ec04ad2a4b8c37691addba3081b9f701c299fb3e4d350261c362`. Exact v1 reached raw stream, 961 rows; exit 0.
- `e32` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v2/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v2/8.6.18/receipt.json). SHA-256 `52e374f5646a615d95979639dc1d2a8890652498a46836e9e8986f1e0b50274c`. Original v2 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e33` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v2/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v2/8.6.18/stdout.tsv). SHA-256 `2ff89962b23ae314038ea57c012af6f49cf06b28064f95330f502e2ddc12ae48`. Exact v2 reached raw stream, 1156 rows; exit 0.
- `e34` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v3/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v3/8.6.18/receipt.json). SHA-256 `a53bb63648d9be0204b081bea201037db1ca1ebbef76d22d914ab49197778d3e`. Original v3 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e35` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v3/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v3/8.6.18/stdout.tsv). SHA-256 `c15b3afcdb8b91342406a38f3836bc826b237687f705f275d518b07b4e6159ff`. Exact v3 reached raw stream, 1182 rows; exit 0.
- `e36` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v4/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v4/8.6.18/receipt.json). SHA-256 `8a9f796771877c110f352b546031d84826482886a4afd0117bdc52d15be26aa8`. Original v4 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e37` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v4/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v4/8.6.18/stdout.tsv). SHA-256 `618d04c08e5c33268b3c25ba9f154cdb6c9560680a30726a58b7cbc7fad269ce`. Exact v4 reached raw stream, 1371 rows; exit 0.
- `e38` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v5/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v5/8.6.18/receipt.json). SHA-256 `21c251ffef4cbb02381bcafae6000562e566301651a4be900e6b4d5b0d2b722e`. Original v5 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e39` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v5/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v5/8.6.18/stdout.tsv). SHA-256 `456edda1c926b4ba62daa530c63a19888572c1627e71a0138403fd5406ea3bbe`. Exact v5 reached raw stream, 1371 rows; exit 0.
- `e40` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v6/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v6/8.6.18/receipt.json). SHA-256 `969ff08a9150a13962a7c5512a24d4cb277dd4919ead068b51621d2ef1795f8d`. Original v6 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e41` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v6/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v6/8.6.18/stdout.tsv). SHA-256 `edb185e48d5424b60d07374a90f5bdbb36b625c39b8f99c89b0da44e55d8659f`. Exact v6 reached raw stream, 1372 rows; exit 0.
- `e42` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v1/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v1/9.0.4/receipt.json). SHA-256 `f8112753b7c92e35d75fbc77be1602880589cf2897677a14bed9c70676fa090c`. Original v1 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e43` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v1/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v1/9.0.4/stdout.tsv). SHA-256 `4a6f2f8680b14ccf910783098cd41e79464d76031822bf9d129de8875d27e5a8`. Exact v1 reached raw stream, 961 rows; exit 0.
- `e44` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v2/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v2/9.0.4/receipt.json). SHA-256 `170201e5d9c6edce9d89480e499a5d22f671394ce82145f2fe614fa80a8ea5d3`. Original v2 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e45` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v2/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v2/9.0.4/stdout.tsv). SHA-256 `002e41c6a36b746d31809338793474f64726d75a8b3fe2ace48b181d23ca915f`. Exact v2 reached raw stream, 1156 rows; exit 0.
- `e46` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v3/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v3/9.0.4/receipt.json). SHA-256 `5cc4f192969fb126100cbf0d18528d0ca8dd2b3034db701fd7b3f8f11415016c`. Original v3 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e47` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v3/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v3/9.0.4/stdout.tsv). SHA-256 `e90b9e2073de2e327e2ad2d6ccf06b0863bb9863aef2b3b32989f2ff33824a32`. Exact v3 reached raw stream, 1182 rows; exit 0.
- `e48` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v4/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v4/9.0.4/receipt.json). SHA-256 `4b8ffa42d27aacdc449cc4b11b739af2e893d40fde5ff8066ead3190e2d5d5ea`. Original v4 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e49` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v4/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v4/9.0.4/stdout.tsv). SHA-256 `6f57b5787e8b6ea33207f294c1fd986bf88d794fdd10215ee3485737db854153`. Exact v4 reached raw stream, 1371 rows; exit 0.
- `e50` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v5/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v5/9.0.4/receipt.json). SHA-256 `3ac7fe19a6e1965e49b9aa350e3970656746ce47e9f9f8d112995d1bc6ed1083`. Original v5 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e51` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v5/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v5/9.0.4/stdout.tsv). SHA-256 `e7f7fac3103987ed2aa9a6a3c2c20c4147a192f924efb122d8d5b19a0fa123c7`. Exact v5 reached raw stream, 1371 rows; exit 0.
- `e52` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v6/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v6/9.0.4/receipt.json). SHA-256 `4b49fdc218993431db83f2ee0c0c1e64ce82468195d6867ce83433dbf0be5fd5`. Original v6 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e53` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v6/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v6/9.0.4/stdout.tsv). SHA-256 `428d1d3813bb9dbb9b34f1fb68d39dadad4df3c6e6c9c9ec2418c3e174b47736`. Exact v6 reached raw stream, 1372 rows; exit 0.
- `e54` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v1/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v1/9.1.0/receipt.json). SHA-256 `5de06c76c4d4e8f40ebf883f8ec2a9747ce06aa7515daff287bd35176d932c36`. Original v1 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e55` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v1/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v1/9.1.0/stdout.tsv). SHA-256 `fdce7014155e34cce4c539b141db9c9326b9312a0ad9d54ff2dcc08863ae9b8f`. Exact v1 reached raw stream, 961 rows; exit 0.
- `e56` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v2/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v2/9.1.0/receipt.json). SHA-256 `b8e4eee64900fbe9b843f7f99d309deebbbe68da3e3034dd9e2abe8ccffdda12`. Original v2 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e57` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v2/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v2/9.1.0/stdout.tsv). SHA-256 `945ebe0565cc6ba7725a125ac539ed95acb6424e76a3636242052571a2ad7339`. Exact v2 reached raw stream, 1156 rows; exit 0.
- `e58` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v3/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v3/9.1.0/receipt.json). SHA-256 `aaadb8c81ef294aa5e7ace96b0947796706823f8fd4885716395112f140f4b1b`. Original v3 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e59` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v3/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v3/9.1.0/stdout.tsv). SHA-256 `e1f1857a1b8438794a24f75dccd19710a67c15a4e7d278b0b4b116795da9c56f`. Exact v3 reached raw stream, 1182 rows; exit 0.
- `e60` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v4/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v4/9.1.0/receipt.json). SHA-256 `746094e198f1b4415357e647e619de8e472017fe17a2d14cab064ed4fee22374`. Original v4 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e61` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v4/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v4/9.1.0/stdout.tsv). SHA-256 `116dfc3bd3b73bb827f0959d814961eebd1523de5dd10f7a154c998b8112c57c`. Exact v4 reached raw stream, 1371 rows; exit 0.
- `e62` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v5/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v5/9.1.0/receipt.json). SHA-256 `912fe931b455c1231de5b123958c6bad57664f8b8f6b47ab7788e2eee76c5d4e`. Original v5 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e63` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v5/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v5/9.1.0/stdout.tsv). SHA-256 `2f6b78d103be099a0b45a97760e7f0b0f8ece9ed8c78d45f4a2cb1c9271fe611`. Exact v5 reached raw stream, 1371 rows; exit 0.
- `e64` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/v6/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v6/9.1.0/receipt.json). SHA-256 `e86284c732def0a3db51ddf783e8a6c6f00311cabe412ac211114aedad4d7f4a`. Original v6 build/process receipt: configured AC_FLAGS and exact header/library/executable/input/stream hashes.
- `e65` (observation): [rust/tcl-registry/tests/data/native_substitution_owner/v6/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v6/9.1.0/stdout.tsv). SHA-256 `0d4bad1c744b58abbc6f1d0149ab237c07041e7bac04319d7132941e1ec7ff5e`. Exact v6 reached raw stream, 1372 rows; exit 0.
- `row-input-audit` (limitation): [docs/design/analysis/name-resolution-proofs/substitution-row-input-audit.json](../../../../docs/design/analysis/name-resolution-proofs/substitution-row-input-audit.json). SHA-256 `687d6f868c2df6b21a5d7fb965973d1a3e77b0ef472af52cc214feb86a535021`. Offline named-field extraction and per-question one-based original row selections; retains all template ordinals, masks, repeated labels and empty results. No fresh execution.
- `offline-provider-checks` (limitation): [docs/design/analysis/name-resolution-proofs/substitution-offline-checks.json](../../../../docs/design/analysis/name-resolution-proofs/substitution-offline-checks.json). SHA-256 `38a5fdb1e81ba1c2e153ba4a54dc378ca2cfb356927e9bd50d3f82baf11180e5`. Actual six-variant verify-only checks of30 configured source/header/library/flags/stream associations; no compiler, native executable or Rust assertion was launched.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/command/native_substitution_tests.rs](../../../../rust/tcl-vm/src/command/native_substitution_tests.rs), `command::native_substitution_tests::original_substitution_changes_namespace_and_borrowed_local_cache_without_proc_header` (linked): Actual native template A/B/root/P/Q contexts, retained local-name table, no proc-header ownership, repeated/current cache and duplicate string-only separation.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-registry/tests/data/native_substitution_owner/replay.py",
  "--source-root",
  "tmp",
  "--output",
  "/tmp/native-substitution-reconfirmation",
  "--variant",
  "v6"
]
```

Requires exact configured C5 trees, matching public/private headers, static libraries, Makefiles and AC_FLAGS. It compares whole stdout/stderr and process exit against retained receipts. Run v1-v5 separately with --variant to preserve their input/setup differences; v4 intentionally retains the C8.4 harness crash. No Jim/BIG-IP launch or Rust execution is implied.
