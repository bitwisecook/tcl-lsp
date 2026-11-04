# naming.array-source.combined-set-receiver

Kind: `native-observation`

## Problem statement

array set a(b) can be mistaken for a whole array literally named a(b), an ordinary element of a, or a dictionary-valued element. The combined receiver grammar and selected container model must be chosen before claiming a successful store.

## Question

Does array set a(b), including the rooted ::N::a(b) form, create a whole array, reject an element receiver, or install an indexed dictionary value?

## Conclusion

All five C captures reject both array-set forms with variable is not array. Jim completes both and reports1 1 1 for array exists root, array exists indexed receiver and info exists indexed receiver. The successful Jim dictionary-valued indexed receiver is distinct from a C whole-array receiver. This source result does not issue an original Normal-write receipt, native cell identity, observer closure or arbitrary dictionary membership.

## Scope

Selected cases array-set-parens,array-set-qualified-parens from25 actual source programs per provider,150 total captured completions. Every case uses a new interpreter. Public Tcl_Eval or Jim_Eval consumes NUL-terminated ASCII source; binary-format/source-escape values are produced at runtime and are not raw zero source or independently held original native String objects. Result bytes are sampled after the guest completion through the actual public string getter. Source/header/library/executable and separate stdout/stderr hashes, compile/process0 and actual original command arrays are retained. C release associations and Jim git-describe tag are recorded, but this probe does not query a linked runtime patchlevel or object primary/refcount. BIG-IP is not tested; no Rust execution is inferred.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (recorded build association; linked runtime version not queried by this probe). Build: headerSHA=824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf; librarySHA=532be0a794ca8277c5ad979a227c38600f27c3c91d436e5d3731d661c271fe47; executableSHA=5a206bc4a8be86ebc4f16fe7ab2d1ede603903ead3ebf4ad584789895d75f414. Full original compile argument vector retained in the provider receipt; compiler-version query absent.. Channel: Tcl_Eval/Jim_Eval NUL-terminated ASCII source; public result string getter. Dialect: C Tcl.

Exact selected rows in case/guest-code/result-byte-count/result-hex order:

```text
array-set-parens	1	38	63616e277420736574202261286229223a207661726961626c652069736e2774206172726179
array-set-qualified-parens	1	43	63616e27742073657420223a3a4e3a3a61286229223a207661726961626c652069736e2774206172726179
```

Compile status0 and whole process0 are separate from the printed guest completions.

### tcl8.5

Status: `observed`. Version: 8.5.19 (recorded build association; linked runtime version not queried by this probe). Build: headerSHA=c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5; librarySHA=94313727ea507b1b2b00018062a0f57587458a3509c37f2e4b351dd58d0592dc; executableSHA=65045fd8185fb249adc3328e05b77e59053228c5eb16dcaf4390d0658a85fbf4. Full original compile argument vector retained in the provider receipt; compiler-version query absent.. Channel: Tcl_Eval/Jim_Eval NUL-terminated ASCII source; public result string getter. Dialect: C Tcl.

Exact selected rows in case/guest-code/result-byte-count/result-hex order:

```text
array-set-parens	1	38	63616e277420736574202261286229223a207661726961626c652069736e2774206172726179
array-set-qualified-parens	1	43	63616e27742073657420223a3a4e3a3a61286229223a207661726961626c652069736e2774206172726179
```

Compile status0 and whole process0 are separate from the printed guest completions.

### tcl8.6

Status: `observed`. Version: 8.6.18 (recorded build association; linked runtime version not queried by this probe). Build: headerSHA=aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245; librarySHA=980630e3a7d37f82322fda1a7e45a85b86d8d905cb395a2aa0c083b3c7eaa8cb; executableSHA=c6e5158450735a2b8a8b350d436df1a55239ee9daf1ba38ee6f2640adc07ceaa. Full original compile argument vector retained in the provider receipt; compiler-version query absent.. Channel: Tcl_Eval/Jim_Eval NUL-terminated ASCII source; public result string getter. Dialect: C Tcl.

Exact selected rows in case/guest-code/result-byte-count/result-hex order:

```text
array-set-parens	1	38	63616e277420736574202261286229223a207661726961626c652069736e2774206172726179
array-set-qualified-parens	1	43	63616e27742073657420223a3a4e3a3a61286229223a207661726961626c652069736e2774206172726179
```

Compile status0 and whole process0 are separate from the printed guest completions.

### tcl9.0

Status: `observed`. Version: 9.0.4 (recorded build association; linked runtime version not queried by this probe). Build: headerSHA=eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a; librarySHA=dace08db925224497714ba86969fabb51c05a78f2bda22db6c05f795c3a70ae4; executableSHA=21baee6e61de27574b6a60baa1da95b8ab635efa49f7c985c0a8ba61e2816f01. Full original compile argument vector retained in the provider receipt; compiler-version query absent.. Channel: Tcl_Eval/Jim_Eval NUL-terminated ASCII source; public result string getter. Dialect: C Tcl.

Exact selected rows in case/guest-code/result-byte-count/result-hex order:

```text
array-set-parens	1	38	63616e277420736574202261286229223a207661726961626c652069736e2774206172726179
array-set-qualified-parens	1	43	63616e27742073657420223a3a4e3a3a61286229223a207661726961626c652069736e2774206172726179
```

Compile status0 and whole process0 are separate from the printed guest completions.

### tcl9.1

Status: `observed`. Version: 9.1.0 (recorded build association; linked runtime version not queried by this probe). Build: headerSHA=30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950; librarySHA=513028850edf096856a29e4e16869aac91cba2ef75c3924aaaaeeb6e45e167db; executableSHA=ba7203882ca9d0d9af06fc1cbb8aed2a458405680046ca64720fd3b08018bda4. Full original compile argument vector retained in the provider receipt; compiler-version query absent.. Channel: Tcl_Eval/Jim_Eval NUL-terminated ASCII source; public result string getter. Dialect: C Tcl.

Exact selected rows in case/guest-code/result-byte-count/result-hex order:

```text
array-set-parens	1	38	63616e277420736574202261286229223a207661726961626c652069736e2774206172726179
array-set-qualified-parens	1	43	63616e27742073657420223a3a4e3a3a61286229223a207661726961626c652069736e2774206172726179
```

Compile status0 and whole process0 are separate from the printed guest completions.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9 (recorded build association; linked runtime version not queried by this probe). Build: headerSHA=d9b020a910ba79cd90d2194849ad56b691fecb5dafd2b3f01c3d4d94e04c369d; librarySHA=a692b833f5b8f3351c1dc5911810aadf4c5f0ff4ca795ee6b802e6550e57b3da; executableSHA=616daed2a6071a8adc76ac1dc77b505cff843f2e4fed9a5dd0daf8e40a8959b8. Full original compile argument vector retained in the provider receipt; compiler-version query absent.. Channel: Tcl_Eval/Jim_Eval NUL-terminated ASCII source; public result string getter. Dialect: Jim Tcl.

Exact selected rows in case/guest-code/result-byte-count/result-hex order:

```text
array-set-parens	0	5	3120312031
array-set-qualified-parens	0	5	3120312031
```

Compile status0 and whole process0 are separate from the printed guest completions.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No BIG-IP load/event or appliance-Tcl result for these exact programs is attached.

## Historical references

- `probe` (input): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/probe.c](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/probe.c). Exact public API/source producers and independently reset interpreter/guest result observer.
- `cases` (input): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/cases.json](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/cases.json). All25 named ASCII source strings match the exact C literal vector.
- `receipt` (provider): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/receipt.json](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/receipt.json). Six complete original source/header/library/executable/compile/process/raw stream associations.
- `replay` (implementation): [scripts/dev/replay-array-oo-source-controls.py](../../../../scripts/dev/replay-array-oo-source-controls.py). Maintained exact-input replayer requires captured build correspondence and separate new output; no execution of this script is inferred.
- `rows-tcl8.4` (observation): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.4.20/stdout.tsv). Exact25 completion/result-unit rows; selected labels array-set-parens,array-set-qualified-parens.
- `build-tcl8.4` (provider): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.4.20/receipt.json). Independent exact original provider association, compile/process status and whole streams.
- `stderr-tcl8.4` (observation): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.4.20/stderr). Exact empty original process stderr.
- `rows-tcl8.5` (observation): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.5.19/stdout.tsv). Exact25 completion/result-unit rows; selected labels array-set-parens,array-set-qualified-parens.
- `build-tcl8.5` (provider): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.5.19/receipt.json). Independent exact original provider association, compile/process status and whole streams.
- `stderr-tcl8.5` (observation): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.5.19/stderr). Exact empty original process stderr.
- `rows-tcl8.6` (observation): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.6.18/stdout.tsv). Exact25 completion/result-unit rows; selected labels array-set-parens,array-set-qualified-parens.
- `build-tcl8.6` (provider): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.6.18/receipt.json). Independent exact original provider association, compile/process status and whole streams.
- `stderr-tcl8.6` (observation): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.6.18/stderr). Exact empty original process stderr.
- `rows-tcl9.0` (observation): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/9.0.4/stdout.tsv). Exact25 completion/result-unit rows; selected labels array-set-parens,array-set-qualified-parens.
- `build-tcl9.0` (provider): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/9.0.4/receipt.json). Independent exact original provider association, compile/process status and whole streams.
- `stderr-tcl9.0` (observation): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/9.0.4/stderr). Exact empty original process stderr.
- `rows-tcl9.1` (observation): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/9.1.0/stdout.tsv). Exact25 completion/result-unit rows; selected labels array-set-parens,array-set-qualified-parens.
- `build-tcl9.1` (provider): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/9.1.0/receipt.json). Independent exact original provider association, compile/process status and whole streams.
- `stderr-tcl9.1` (observation): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/9.1.0/stderr). Exact empty original process stderr.
- `rows-jim` (observation): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/jim/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/jim/stdout.tsv). Exact25 completion/result-unit rows; selected labels array-set-parens,array-set-qualified-parens.
- `build-jim` (provider): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/jim/receipt.json). Independent exact original provider association, compile/process status and whole streams.
- `stderr-jim` (observation): [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/jim/stderr](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/jim/stderr). Exact empty original process stderr.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_variable_table.rs](../../../../rust/tcl-registry/src/native_variable_table.rs), `NativeVariableTableProtocol::dictionary_arrays`: Selects array versus dictionary container model; actual indexed receiver/read/write/observer currency remains an independent compiler/runtime obligation.
- [rust/tcl-registry/src/resolved_invocation.rs](../../../../rust/tcl-registry/src/resolved_invocation.rs), `resolved_invocation::selected_variable_receiver_form_keeps_array_argv_combined_and_aliases_separate` (linked): Selected C8.6 receiver metadata preserves the Combined original argv for scalar set and array set/get/names/unset a(b), while role-only and alias arguments cannot supply receiver form. It asserts no guest success, Normal transfer, physical root-only API or runtime cell identity.
- [rust/tcl-compiler/src/analyser/scope.rs](../../../../rust/tcl-compiler/src/analyser/scope.rs), `analyser::scope::tests::original_array_receiver_symbols_keep_combined_names_and_guest_error_independent` (linked): Five independently analysed absolute ::N::a(b) source programs preserve selected Combined receiver naming and source declaration roles without inferring guest success or later reachability from a prior failing array operation.

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
