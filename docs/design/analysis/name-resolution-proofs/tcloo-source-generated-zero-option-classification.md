# naming.tcloo.source-generated-zero-option-classification

Kind: `native-observation`

## Problem statement

A runtime-generated zero operand can be classified as an object name or a leading-dash method selector. Reusing a raw native String-zero truncation rule would wrongly accept a flag produced by binary format or a source escape.

## Question

How do superclass/mixin classify binary-format zero, zeroSet, -zero-set and written -s\u0000et operands?

## Conclusion

C8.6/9.0/9.1 reject zero and zeroSet as object names that do not refer to an object, and reject the leading-dash zero forms as unknown methods. The returned diagnostic units contain C080 for zero. C8.6 lists -append/-clear/-set; C9 additionally lists -appendifnew/-prepend/-remove. C8.4/8.5/Jim reject oo::class before those operands are reached. All inputs are ASCII NUL-terminated source; binary format creates runtime values that can materialize modified zero. Neither original rawString00 identity nor CString matching is measured.

## Scope

Selected cases superclass-zero,superclass-zero-suffix,superclass-dash-zero,superclass-escaped-zero,mixin-zero,mixin-zero-suffix,mixin-dash-zero,mixin-escaped-zero from25 actual source programs per provider,150 total captured completions. Every case uses a new interpreter. Public Tcl_Eval or Jim_Eval consumes NUL-terminated ASCII source; binary-format/source-escape values are produced at runtime and are not raw zero source or independently held original native String objects. Result bytes are sampled after the guest completion through the actual public string getter. Source/header/library/executable and separate stdout/stderr hashes, compile/process0 and actual original command arrays are retained. C release associations and Jim git-describe tag are recorded, but this probe does not query a linked runtime patchlevel or object primary/refcount. BIG-IP is not tested; no Rust execution is inferred.

## Provider answers

### tcl8.4

Status: `unsupported`. Version: 8.4.20 (recorded build association; linked runtime version not queried by this probe). Build: headerSHA=824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf; librarySHA=532be0a794ca8277c5ad979a227c38600f27c3c91d436e5d3731d661c271fe47; executableSHA=5a206bc4a8be86ebc4f16fe7ab2d1ede603903ead3ebf4ad584789895d75f414. Full original compile argument vector retained in the provider receipt; compiler-version query absent.. Channel: Tcl_Eval/Jim_Eval NUL-terminated ASCII source; public result string getter. Dialect: C Tcl.

Exact selected rows in case/guest-code/result-byte-count/result-hex order:

```text
superclass-zero	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
superclass-zero-suffix	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
superclass-dash-zero	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
superclass-escaped-zero	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
mixin-zero	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
mixin-zero-suffix	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
mixin-dash-zero	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
mixin-escaped-zero	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
```

Compile status0 and whole process0 are separate from the printed guest completions. oo::class is unavailable, so the targeted slot worker and zero operand are unreached.

### tcl8.5

Status: `unsupported`. Version: 8.5.19 (recorded build association; linked runtime version not queried by this probe). Build: headerSHA=c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5; librarySHA=94313727ea507b1b2b00018062a0f57587458a3509c37f2e4b351dd58d0592dc; executableSHA=65045fd8185fb249adc3328e05b77e59053228c5eb16dcaf4390d0658a85fbf4. Full original compile argument vector retained in the provider receipt; compiler-version query absent.. Channel: Tcl_Eval/Jim_Eval NUL-terminated ASCII source; public result string getter. Dialect: C Tcl.

Exact selected rows in case/guest-code/result-byte-count/result-hex order:

```text
superclass-zero	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
superclass-zero-suffix	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
superclass-dash-zero	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
superclass-escaped-zero	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
mixin-zero	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
mixin-zero-suffix	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
mixin-dash-zero	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
mixin-escaped-zero	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
```

Compile status0 and whole process0 are separate from the printed guest completions. oo::class is unavailable, so the targeted slot worker and zero operand are unreached.

### tcl8.6

Status: `observed`. Version: 8.6.18 (recorded build association; linked runtime version not queried by this probe). Build: headerSHA=aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245; librarySHA=980630e3a7d37f82322fda1a7e45a85b86d8d905cb395a2aa0c083b3c7eaa8cb; executableSHA=c6e5158450735a2b8a8b350d436df1a55239ee9daf1ba38ee6f2640adc07ceaa. Full original compile argument vector retained in the provider receipt; compiler-version query absent.. Channel: Tcl_Eval/Jim_Eval NUL-terminated ASCII source; public result string getter. Dialect: C Tcl.

Exact selected rows in case/guest-code/result-byte-count/result-hex order:

```text
superclass-zero	1	30	c08020646f6573206e6f7420726566657220746f20616e206f626a656374
superclass-zero-suffix	1	33	c08053657420646f6573206e6f7420726566657220746f20616e206f626a656374
superclass-dash-zero	1	56	756e6b6e6f776e206d6574686f6420222dc080736574223a206d757374206265202d617070656e642c202d636c656172206f72202d736574
superclass-escaped-zero	1	56	756e6b6e6f776e206d6574686f6420222d73c0806574223a206d757374206265202d617070656e642c202d636c656172206f72202d736574
mixin-zero	1	30	c08020646f6573206e6f7420726566657220746f20616e206f626a656374
mixin-zero-suffix	1	33	c08053657420646f6573206e6f7420726566657220746f20616e206f626a656374
mixin-dash-zero	1	56	756e6b6e6f776e206d6574686f6420222dc080736574223a206d757374206265202d617070656e642c202d636c656172206f72202d736574
mixin-escaped-zero	1	56	756e6b6e6f776e206d6574686f6420222d73c0806574223a206d757374206265202d617070656e642c202d636c656172206f72202d736574
```

Compile status0 and whole process0 are separate from the printed guest completions.

### tcl9.0

Status: `observed`. Version: 9.0.4 (recorded build association; linked runtime version not queried by this probe). Build: headerSHA=eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a; librarySHA=dace08db925224497714ba86969fabb51c05a78f2bda22db6c05f795c3a70ae4; executableSHA=21baee6e61de27574b6a60baa1da95b8ab635efa49f7c985c0a8ba61e2816f01. Full original compile argument vector retained in the provider receipt; compiler-version query absent.. Channel: Tcl_Eval/Jim_Eval NUL-terminated ASCII source; public result string getter. Dialect: C Tcl.

Exact selected rows in case/guest-code/result-byte-count/result-hex order:

```text
superclass-zero	1	30	c08020646f6573206e6f7420726566657220746f20616e206f626a656374
superclass-zero-suffix	1	33	c08053657420646f6573206e6f7420726566657220746f20616e206f626a656374
superclass-dash-zero	1	89	756e6b6e6f776e206d6574686f6420222dc080736574223a206d757374206265202d617070656e642c202d617070656e6469666e65772c202d636c6561722c202d70726570656e642c202d72656d6f7665206f72202d736574
superclass-escaped-zero	1	89	756e6b6e6f776e206d6574686f6420222d73c0806574223a206d757374206265202d617070656e642c202d617070656e6469666e65772c202d636c6561722c202d70726570656e642c202d72656d6f7665206f72202d736574
mixin-zero	1	30	c08020646f6573206e6f7420726566657220746f20616e206f626a656374
mixin-zero-suffix	1	33	c08053657420646f6573206e6f7420726566657220746f20616e206f626a656374
mixin-dash-zero	1	89	756e6b6e6f776e206d6574686f6420222dc080736574223a206d757374206265202d617070656e642c202d617070656e6469666e65772c202d636c6561722c202d70726570656e642c202d72656d6f7665206f72202d736574
mixin-escaped-zero	1	89	756e6b6e6f776e206d6574686f6420222d73c0806574223a206d757374206265202d617070656e642c202d617070656e6469666e65772c202d636c6561722c202d70726570656e642c202d72656d6f7665206f72202d736574
```

Compile status0 and whole process0 are separate from the printed guest completions.

### tcl9.1

Status: `observed`. Version: 9.1.0 (recorded build association; linked runtime version not queried by this probe). Build: headerSHA=30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950; librarySHA=513028850edf096856a29e4e16869aac91cba2ef75c3924aaaaeeb6e45e167db; executableSHA=ba7203882ca9d0d9af06fc1cbb8aed2a458405680046ca64720fd3b08018bda4. Full original compile argument vector retained in the provider receipt; compiler-version query absent.. Channel: Tcl_Eval/Jim_Eval NUL-terminated ASCII source; public result string getter. Dialect: C Tcl.

Exact selected rows in case/guest-code/result-byte-count/result-hex order:

```text
superclass-zero	1	30	c08020646f6573206e6f7420726566657220746f20616e206f626a656374
superclass-zero-suffix	1	33	c08053657420646f6573206e6f7420726566657220746f20616e206f626a656374
superclass-dash-zero	1	89	756e6b6e6f776e206d6574686f6420222dc080736574223a206d757374206265202d617070656e642c202d617070656e6469666e65772c202d636c6561722c202d70726570656e642c202d72656d6f7665206f72202d736574
superclass-escaped-zero	1	89	756e6b6e6f776e206d6574686f6420222d73c0806574223a206d757374206265202d617070656e642c202d617070656e6469666e65772c202d636c6561722c202d70726570656e642c202d72656d6f7665206f72202d736574
mixin-zero	1	30	c08020646f6573206e6f7420726566657220746f20616e206f626a656374
mixin-zero-suffix	1	33	c08053657420646f6573206e6f7420726566657220746f20616e206f626a656374
mixin-dash-zero	1	89	756e6b6e6f776e206d6574686f6420222dc080736574223a206d757374206265202d617070656e642c202d617070656e6469666e65772c202d636c6561722c202d70726570656e642c202d72656d6f7665206f72202d736574
mixin-escaped-zero	1	89	756e6b6e6f776e206d6574686f6420222d73c0806574223a206d757374206265202d617070656e642c202d617070656e6469666e65772c202d636c6561722c202d70726570656e642c202d72656d6f7665206f72202d736574
```

Compile status0 and whole process0 are separate from the printed guest completions.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9 (recorded build association; linked runtime version not queried by this probe). Build: headerSHA=d9b020a910ba79cd90d2194849ad56b691fecb5dafd2b3f01c3d4d94e04c369d; librarySHA=a692b833f5b8f3351c1dc5911810aadf4c5f0ff4ca795ee6b802e6550e57b3da; executableSHA=616daed2a6071a8adc76ac1dc77b505cff843f2e4fed9a5dd0daf8e40a8959b8. Full original compile argument vector retained in the provider receipt; compiler-version query absent.. Channel: Tcl_Eval/Jim_Eval NUL-terminated ASCII source; public result string getter. Dialect: Jim Tcl.

Exact selected rows in case/guest-code/result-byte-count/result-hex order:

```text
superclass-zero	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
superclass-zero-suffix	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
superclass-dash-zero	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
superclass-escaped-zero	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
mixin-zero	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
mixin-zero-suffix	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
mixin-dash-zero	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
mixin-escaped-zero	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
```

Compile status0 and whole process0 are separate from the printed guest completions. oo::class is unavailable, so the targeted slot worker and zero operand are unreached.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No BIG-IP load/event or appliance-Tcl result for these exact programs is attached.

## Exact evidence

- `probe` (input): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/probe.c](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/probe.c). SHA-256 `67c72e6f705c81c3758f6ff0e455bc6afa1996a7ea704ca47f8aa96b30840c87`. Exact public API/source producers and independently reset interpreter/guest result observer.
- `cases` (input): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/cases.json](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/cases.json). SHA-256 `b51abb476ac87f9a1957ab81d092896f6a64b460b2aee7d178b2cff669a760d0`. All25 named ASCII source strings match the exact C literal vector.
- `receipt` (provider): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/receipt.json](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/receipt.json). SHA-256 `dc1651ef2421ed9870620095c74e5232942948426b0b1d797a2c821f492bf8b9`. Six complete original source/header/library/executable/compile/process/raw stream associations.
- `replay` (implementation): [scripts/dev/replay-array-oo-source-controls.py](../../../../scripts/dev/replay-array-oo-source-controls.py). SHA-256 `d099fa238a83daa1159f0655af4f4c893d3a377204d378aa817f286049eb1888`. Maintained exact-input replayer requires captured build correspondence and separate new output; no execution of this script is inferred.
- `rows-tcl8.4` (observation): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.4.20/stdout.tsv). SHA-256 `7cbd7699c14f48e75e1127da37c651556be9fc0aaacea14228834c5271b865fa`. Exact25 completion/result-unit rows; selected labels superclass-zero,superclass-zero-suffix,superclass-dash-zero,superclass-escaped-zero,mixin-zero,mixin-zero-suffix,mixin-dash-zero,mixin-escaped-zero.
- `build-tcl8.4` (provider): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.4.20/receipt.json). SHA-256 `d7a6e52a209c1b943b4a84fc9e91bc9f9c7a8384d1e9fb12532fd805e9f3a812`. Independent exact original provider association, compile/process status and whole streams.
- `stderr-tcl8.4` (observation): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty original process stderr.
- `rows-tcl8.5` (observation): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.5.19/stdout.tsv). SHA-256 `7cbd7699c14f48e75e1127da37c651556be9fc0aaacea14228834c5271b865fa`. Exact25 completion/result-unit rows; selected labels superclass-zero,superclass-zero-suffix,superclass-dash-zero,superclass-escaped-zero,mixin-zero,mixin-zero-suffix,mixin-dash-zero,mixin-escaped-zero.
- `build-tcl8.5` (provider): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.5.19/receipt.json). SHA-256 `e377a2831dd7b6fdf2ac5383df71ddc7c4dfd075e20bf3da8bd358eaa7ce188d`. Independent exact original provider association, compile/process status and whole streams.
- `stderr-tcl8.5` (observation): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty original process stderr.
- `rows-tcl8.6` (observation): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.6.18/stdout.tsv). SHA-256 `6be20356479597fd026d09b276bbbadb58023c86b4e2440c2d61843998539a0f`. Exact25 completion/result-unit rows; selected labels superclass-zero,superclass-zero-suffix,superclass-dash-zero,superclass-escaped-zero,mixin-zero,mixin-zero-suffix,mixin-dash-zero,mixin-escaped-zero.
- `build-tcl8.6` (provider): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.6.18/receipt.json). SHA-256 `306279e54d4ba01cbf6fd790977a70707dcc82eabb75b5cbd36ff8311f369e65`. Independent exact original provider association, compile/process status and whole streams.
- `stderr-tcl8.6` (observation): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty original process stderr.
- `rows-tcl9.0` (observation): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/9.0.4/stdout.tsv). SHA-256 `59c80ed5e7b0831bc38604def7e871af4473669524794044a3b39f95f1d17d1c`. Exact25 completion/result-unit rows; selected labels superclass-zero,superclass-zero-suffix,superclass-dash-zero,superclass-escaped-zero,mixin-zero,mixin-zero-suffix,mixin-dash-zero,mixin-escaped-zero.
- `build-tcl9.0` (provider): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/9.0.4/receipt.json). SHA-256 `8a8cbca290a90c9df76b8241c7e9e87c14dfd3900fc06857ff8b485514c64eab`. Independent exact original provider association, compile/process status and whole streams.
- `stderr-tcl9.0` (observation): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty original process stderr.
- `rows-tcl9.1` (observation): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/9.1.0/stdout.tsv). SHA-256 `59c80ed5e7b0831bc38604def7e871af4473669524794044a3b39f95f1d17d1c`. Exact25 completion/result-unit rows; selected labels superclass-zero,superclass-zero-suffix,superclass-dash-zero,superclass-escaped-zero,mixin-zero,mixin-zero-suffix,mixin-dash-zero,mixin-escaped-zero.
- `build-tcl9.1` (provider): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/9.1.0/receipt.json). SHA-256 `29114d00f323c483dc653643f87d5f6315b046baa9d09e2c58d35f27c2b5d258`. Independent exact original provider association, compile/process status and whole streams.
- `stderr-tcl9.1` (observation): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty original process stderr.
- `rows-jim` (observation): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/jim/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/jim/stdout.tsv). SHA-256 `d944673515bc35d65cc92f3b55dfb6d992f22dcbd707557654b104a89c138d53`. Exact25 completion/result-unit rows; selected labels superclass-zero,superclass-zero-suffix,superclass-dash-zero,superclass-escaped-zero,mixin-zero,mixin-zero-suffix,mixin-dash-zero,mixin-escaped-zero.
- `build-jim` (provider): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/jim/receipt.json). SHA-256 `52868889195e2450bbcf139dd0e187a54ad084d478174d88ee6b9a5a03280455`. Independent exact original provider association, compile/process status and whole streams.
- `stderr-jim` (observation): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/jim/stderr](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty original process stderr.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/definer.rs](../../../../rust/tcl-registry/src/definer.rs), `SlotSpec::split_original_call`: Selects original query versus update grammar and exact slot operation without issuing target/effect or successful native dispatch.
- [rust/tcl-registry/src/definer.rs](../../../../rust/tcl-registry/src/definer.rs), `SlotOp::resolve_runtime`: Matches the release-selected original operation bytes, preserving materialized-zero rejection separately from a raw String-zero byte purpose.
- [rust/tcl-registry/src/definer.rs](../../../../rust/tcl-registry/src/definer.rs), `definer::tests::original_tcloo_slot_calls_distinguish_queries_empty_updates_and_materialised_zero` (linked): Pure selected slot grammar distinguishes no-operand query from bare/explicit empty updates and refuses materialized C080 method flags on C86/90/91, with unavailable C84/85/Jim source admission. It does not invoke these native providers.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "scripts/dev/replay-array-oo-source-controls.py",
  "--tcl-root",
  "${TCL_SOURCE_ROOT}",
  "--jim-root",
  "${JIM_SOURCE_ROOT}",
  "--output",
  "${NEW_OUTPUT}"
]
```

The replayer first validates the retained whole input/stream bytes, then requires explicitly selected header/library hashes equal to this exact captured build. It compiles the unchanged probe in a new output directory, retaining compiler/process status and separate streams, and requires exact original output/status agreement. A different provider build needs an independently scoped new record. No maintained native launch or Rust passing result is claimed by attaching the runner. The original captures preserve observed unsupported/error doors; outer process0 is not guest success.
