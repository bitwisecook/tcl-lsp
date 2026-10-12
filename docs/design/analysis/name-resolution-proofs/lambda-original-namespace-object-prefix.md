# naming.lambda.original-namespace-object-prefix

Kind: `native-observation`

## Problem statement

Lambda namespace selection can be incorrectly inferred from source presentation or assumed identical to Procedure publication. Binary-produced values and raw counted String objects also require separate input channels before zero-byte behavior can be related.

## Question

What namespace current string or caught error does the exact source return for ten Apply namespace controls spanning plain/colon/root forms, binary-produced00 suffixes and literal UTF8 Unicode?

## Conclusion

C8.5–C9.1 return::ns for the plain/colon variants and:: for omitted/empty namespaces, while all binary-produced00 suffix cases fail full namespace lookup. Current Jim returns literal root-prefixed spellings and preserves complete00 suffixes. C8.4 cannot execute Apply. These source results do not establish raw counted String00 or C ByteArray getter behavior.

## Scope

Six fresh source-file CLI processes retain six queried versions and78 sequential caught controls:18 namespace setup catches and60 Apply attempts. Each process shares its namespace setup; the controls are separately caught rather than independently initialized. The exact probe contains literal UTF8 U+00E9 even though immutable request/receipt metadata calls it ASCII/escaped. Source-produced binary values and stdout encoding remain separate from sampled native object bytes. No namespace pointer/physical allocation, object/cache/header, command token, source/compiler entry, arbitrary Normal, frame correspondence or BIG-IP behavior is measured. The independent constructor/getter question supplies actual original SDK input objects and its own narrower representation samples. Independent linked implementation coverage: The shared C85–91 recipe retains original getter bytes and prefixes exact :: only when the counted object lacks that prefix. Namespace lookup applies its separate C-string extent afterwards; no actual namespace existence, object/header storage or entered frame follows from the pure byte projection. C84 and Jim refuse this C-specific purpose. Compiler explicit lambda namespace and child-source lambda advice, VM actual original getter and Runtime original cached/fallback lambda object producers consume the same helper. Jim retains its own counted original namespace object route.

## Provider answers

### tcl8.4

Status: `unsupported`. Version: 8.4.20. Build: Actual CLI executable SHA f0e742ccc08a150afd9a0a236ce24c8e12098dae38c0efd46e59f9b1a28bf58a with retained SDK/header/library/build/source pins, environment and runner associations. No compile command or compiler binary version is retained for this source-file runner.. Channel: Exact UTF8 LF source file with literal U+00E9 in two source lines and no literal NUL; binary format produces the three00-containing values. One fresh CLI process shares namespace setup across thirteen sequential caught controls.. Dialect: Tcl.

All three namespace setups succeed, but each of the ten Apply controls fails because apply is unavailable, with errorCode NONE. No lambda namespace selection or binary-produced namespace value acceptance is measured for this release.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual CLI executable SHA e7eb17821aeb7b1f84619f431e89ecff667bc5461fa8e00d67498b6b7d5dd61a with retained SDK/header/library/build/source pins, environment and runner associations. No compile command or compiler binary version is retained for this source-file runner.. Channel: Exact UTF8 LF source file with literal U+00E9 in two source lines and no literal NUL; binary format produces the three00-containing values. One fresh CLI process shares namespace setup across thirteen sequential caught controls.. Dialect: Tcl.

Plain/relative/absolute colon spellings return::ns; omitted/empty namespaces return root::. All three binary-format-produced00 suffix variants fail namespace lookup with the complete suffix preserved in the diagnostic/errorCode. Literal UTF8 :é returns::é. These source-produced values do not establish raw counted original String00 behavior; the independent SDK constructor/getter question supplies that channel.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual CLI executable SHA 9b156375d068a58fcfb82a2a0afd170afd2b03912cbb6e588676c7688118c8ff with retained SDK/header/library/build/source pins, environment and runner associations. No compile command or compiler binary version is retained for this source-file runner.. Channel: Exact UTF8 LF source file with literal U+00E9 in two source lines and no literal NUL; binary format produces the three00-containing values. One fresh CLI process shares namespace setup across thirteen sequential caught controls.. Dialect: Tcl.

Plain/relative/absolute colon spellings return::ns; omitted/empty namespaces return root::. All three binary-format-produced00 suffix variants fail namespace lookup with the complete suffix preserved in the diagnostic/errorCode. Literal UTF8 :é returns::é. These source-produced values do not establish raw counted original String00 behavior; the independent SDK constructor/getter question supplies that channel.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual CLI executable SHA f4748357cdbf2762f1c5c05916fa53e54718d087120ae9de9c403bbdfaf764b1 with retained SDK/header/library/build/source pins, environment and runner associations. No compile command or compiler binary version is retained for this source-file runner.. Channel: Exact UTF8 LF source file with literal U+00E9 in two source lines and no literal NUL; binary format produces the three00-containing values. One fresh CLI process shares namespace setup across thirteen sequential caught controls.. Dialect: Tcl.

Plain/relative/absolute colon spellings return::ns; omitted/empty namespaces return root::. All three binary-format-produced00 suffix variants fail namespace lookup with the complete suffix preserved in the diagnostic/errorCode. Literal UTF8 :é returns::é. These source-produced values do not establish raw counted original String00 behavior; the independent SDK constructor/getter question supplies that channel.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual CLI executable SHA 2a82f721b68f2ed2347ca4e3daa9a56c49c9e3efce77d00f22fc3b2012d8814d with retained SDK/header/library/build/source pins, environment and runner associations. No compile command or compiler binary version is retained for this source-file runner.. Channel: Exact UTF8 LF source file with literal U+00E9 in two source lines and no literal NUL; binary format produces the three00-containing values. One fresh CLI process shares namespace setup across thirteen sequential caught controls.. Dialect: Tcl.

Plain/relative/absolute colon spellings return::ns; omitted/empty namespaces return root::. All three binary-format-produced00 suffix variants fail namespace lookup with the complete suffix preserved in the diagnostic/errorCode. Literal UTF8 :é returns::é. These source-produced values do not establish raw counted original String00 behavior; the independent SDK constructor/getter question supplies that channel.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual CLI executable SHA e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806 with retained SDK/header/library/build/source pins, environment and runner associations. No compile command or compiler binary version is retained for this source-file runner.. Channel: Exact UTF8 LF source file with literal U+00E9 in two source lines and no literal NUL; binary format produces the three00-containing values. One fresh CLI process shares namespace setup across thirteen sequential caught controls.. Dialect: Jim Tcl.

The exact returned namespace current strings retain a literal root prefix before the complete supplied namespace spelling: :ns→:::ns, :::ns→:::::ns, ns→::ns and::ns→::::ns. Omitted/empty namespaces return::. All three binary-produced00 suffix cases complete0 and preserve the full00/suffix in the reported namespace string. Literal UTF8 :é returns:::é. These Jim results are independent of C namespace allocation and constructor/getter behavior.

### bigip

Status: `not-tested`. Version: not tested. Build: not recorded. Channel: not tested. Dialect: F5 iRules.

No appliance observation of these exact lambda namespace source controls.

## Exact evidence

- `lambda188-probe.tcl` (input): [rust/tcl-registry/tests/data/native_lambda_namespace_source/probe.tcl](../../../../rust/tcl-registry/tests/data/native_lambda_namespace_source/probe.tcl). SHA-256 `f2cf19f936f7bb9c252c50a79f40899011b3644d5ac88bc62cfe368f97bfc4cb`. Exact retained original source/request/runner, SDK association or raw native stream. Request/receipt ASCII and explicit-escape wording remains immutable; the actual source has literal UTF8 U+00E9 and canonical channel metadata describes those bytes.
- `lambda188-request.json` (input): [rust/tcl-registry/tests/data/native_lambda_namespace_source/request.json](../../../../rust/tcl-registry/tests/data/native_lambda_namespace_source/request.json). SHA-256 `ded1155e1f3c062f656bcf58e8d38c25860b9e25bd198e943aee253eafae775a`. Exact retained original source/request/runner, SDK association or raw native stream. Request/receipt ASCII and explicit-escape wording remains immutable; the actual source has literal UTF8 U+00E9 and canonical channel metadata describes those bytes.
- `lambda188-capture.py` (input): [rust/tcl-registry/tests/data/native_lambda_namespace_source/capture.py](../../../../rust/tcl-registry/tests/data/native_lambda_namespace_source/capture.py). SHA-256 `65ee1b73a04ce12a6e7f088b55fa3864782572b16a780c11daa026554d9aeb47`. Exact retained original source/request/runner, SDK association or raw native stream. Request/receipt ASCII and explicit-escape wording remains immutable; the actual source has literal UTF8 U+00E9 and canonical channel metadata describes those bytes.
- `lambda188-8.6.18-receipt.json` (provider): [rust/tcl-registry/tests/data/native_lambda_namespace_source/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_lambda_namespace_source/8.6.18/receipt.json). SHA-256 `751ed9895c5e8123f8ebe2e7ca7dbe2182657ab2a5f322cd869d3cbc43b9c387`. Exact retained original source/request/runner, SDK association or raw native stream. Request/receipt ASCII and explicit-escape wording remains immutable; the actual source has literal UTF8 U+00E9 and canonical channel metadata describes those bytes.
- `lambda188-8.6.18-stderr` (observation): [rust/tcl-registry/tests/data/native_lambda_namespace_source/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_lambda_namespace_source/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact retained original source/request/runner, SDK association or raw native stream. Request/receipt ASCII and explicit-escape wording remains immutable; the actual source has literal UTF8 U+00E9 and canonical channel metadata describes those bytes.
- `lambda188-8.6.18-stdout` (observation): [rust/tcl-registry/tests/data/native_lambda_namespace_source/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_lambda_namespace_source/8.6.18/stdout). SHA-256 `9fb91cde86ec9e1faa47f97d81319cc5305ebcedb9fd9c79827c7833196bd3fd`. Exact retained original source/request/runner, SDK association or raw native stream. Request/receipt ASCII and explicit-escape wording remains immutable; the actual source has literal UTF8 U+00E9 and canonical channel metadata describes those bytes.
- `lambda188-9.1.0-receipt.json` (provider): [rust/tcl-registry/tests/data/native_lambda_namespace_source/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_lambda_namespace_source/9.1.0/receipt.json). SHA-256 `0ea76f1253288e5cf08c95e2ab557f25683c80830b9bff1cbeae2dc5d8d2dd0e`. Exact retained original source/request/runner, SDK association or raw native stream. Request/receipt ASCII and explicit-escape wording remains immutable; the actual source has literal UTF8 U+00E9 and canonical channel metadata describes those bytes.
- `lambda188-9.1.0-stderr` (observation): [rust/tcl-registry/tests/data/native_lambda_namespace_source/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_lambda_namespace_source/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact retained original source/request/runner, SDK association or raw native stream. Request/receipt ASCII and explicit-escape wording remains immutable; the actual source has literal UTF8 U+00E9 and canonical channel metadata describes those bytes.
- `lambda188-9.1.0-stdout` (observation): [rust/tcl-registry/tests/data/native_lambda_namespace_source/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_lambda_namespace_source/9.1.0/stdout). SHA-256 `9e29e77e250e937780e1879ba148d1b27a7e79addf62babffcff74ab04e0ea33`. Exact retained original source/request/runner, SDK association or raw native stream. Request/receipt ASCII and explicit-escape wording remains immutable; the actual source has literal UTF8 U+00E9 and canonical channel metadata describes those bytes.
- `lambda188-8.4.20-receipt.json` (provider): [rust/tcl-registry/tests/data/native_lambda_namespace_source/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_lambda_namespace_source/8.4.20/receipt.json). SHA-256 `c6740b1a8305f8d1cde60b509c86861e7dea5a20ee677eebcb2518484cd17cb8`. Exact retained original source/request/runner, SDK association or raw native stream. Request/receipt ASCII and explicit-escape wording remains immutable; the actual source has literal UTF8 U+00E9 and canonical channel metadata describes those bytes.
- `lambda188-8.4.20-stderr` (observation): [rust/tcl-registry/tests/data/native_lambda_namespace_source/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_lambda_namespace_source/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact retained original source/request/runner, SDK association or raw native stream. Request/receipt ASCII and explicit-escape wording remains immutable; the actual source has literal UTF8 U+00E9 and canonical channel metadata describes those bytes.
- `lambda188-8.4.20-stdout` (observation): [rust/tcl-registry/tests/data/native_lambda_namespace_source/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_lambda_namespace_source/8.4.20/stdout). SHA-256 `e81e213f9613bde119d207d206cf172531256eb41637beede003b5c580210a6e`. Exact retained original source/request/runner, SDK association or raw native stream. Request/receipt ASCII and explicit-escape wording remains immutable; the actual source has literal UTF8 U+00E9 and canonical channel metadata describes those bytes.
- `lambda188-9.0.4-receipt.json` (provider): [rust/tcl-registry/tests/data/native_lambda_namespace_source/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_lambda_namespace_source/9.0.4/receipt.json). SHA-256 `faed014f022767d984ec68218c6a68d29cc152c01b05b9dfb93c071b587fefd3`. Exact retained original source/request/runner, SDK association or raw native stream. Request/receipt ASCII and explicit-escape wording remains immutable; the actual source has literal UTF8 U+00E9 and canonical channel metadata describes those bytes.
- `lambda188-9.0.4-stderr` (observation): [rust/tcl-registry/tests/data/native_lambda_namespace_source/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_lambda_namespace_source/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact retained original source/request/runner, SDK association or raw native stream. Request/receipt ASCII and explicit-escape wording remains immutable; the actual source has literal UTF8 U+00E9 and canonical channel metadata describes those bytes.
- `lambda188-9.0.4-stdout` (observation): [rust/tcl-registry/tests/data/native_lambda_namespace_source/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_lambda_namespace_source/9.0.4/stdout). SHA-256 `fa0abe15bdb5d25e16729011186f9c8968fb3efba53115736681b38e844e82e9`. Exact retained original source/request/runner, SDK association or raw native stream. Request/receipt ASCII and explicit-escape wording remains immutable; the actual source has literal UTF8 U+00E9 and canonical channel metadata describes those bytes.
- `lambda188-8.5.19-receipt.json` (provider): [rust/tcl-registry/tests/data/native_lambda_namespace_source/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_lambda_namespace_source/8.5.19/receipt.json). SHA-256 `8b1046de6daa081048602a5053a9d8a64420b390eb96d6c4565154039d288be1`. Exact retained original source/request/runner, SDK association or raw native stream. Request/receipt ASCII and explicit-escape wording remains immutable; the actual source has literal UTF8 U+00E9 and canonical channel metadata describes those bytes.
- `lambda188-8.5.19-stderr` (observation): [rust/tcl-registry/tests/data/native_lambda_namespace_source/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_lambda_namespace_source/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact retained original source/request/runner, SDK association or raw native stream. Request/receipt ASCII and explicit-escape wording remains immutable; the actual source has literal UTF8 U+00E9 and canonical channel metadata describes those bytes.
- `lambda188-8.5.19-stdout` (observation): [rust/tcl-registry/tests/data/native_lambda_namespace_source/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_lambda_namespace_source/8.5.19/stdout). SHA-256 `05d3b7eac35305c38ef5a516e37c237a25d077f2a27edb2d646e32b575e79a0b`. Exact retained original source/request/runner, SDK association or raw native stream. Request/receipt ASCII and explicit-escape wording remains immutable; the actual source has literal UTF8 U+00E9 and canonical channel metadata describes those bytes.
- `lambda188-jim-receipt.json` (provider): [rust/tcl-registry/tests/data/native_lambda_namespace_source/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_lambda_namespace_source/jim/receipt.json). SHA-256 `bc6845c8dd4fffd5a26c8995f3859fc0424ca017dbb5ac39da6b59feaead7ae6`. Exact retained original source/request/runner, SDK association or raw native stream. Request/receipt ASCII and explicit-escape wording remains immutable; the actual source has literal UTF8 U+00E9 and canonical channel metadata describes those bytes.
- `lambda188-jim-stderr` (observation): [rust/tcl-registry/tests/data/native_lambda_namespace_source/jim/stderr](../../../../rust/tcl-registry/tests/data/native_lambda_namespace_source/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact retained original source/request/runner, SDK association or raw native stream. Request/receipt ASCII and explicit-escape wording remains immutable; the actual source has literal UTF8 U+00E9 and canonical channel metadata describes those bytes.
- `lambda188-jim-stdout` (observation): [rust/tcl-registry/tests/data/native_lambda_namespace_source/jim/stdout](../../../../rust/tcl-registry/tests/data/native_lambda_namespace_source/jim/stdout). SHA-256 `13e8490db6f31c037f88f7e70e465f38a013f69422cb78a1d2571658a5a8bd23`. Exact retained original source/request/runner, SDK association or raw native stream. Request/receipt ASCII and explicit-escape wording remains immutable; the actual source has literal UTF8 U+00E9 and canonical channel metadata describes those bytes.
- `lambda188-sdk-queue.json` (provider): [rust/tcl-registry/tests/data/native_lambda_namespace_source/sdk-queue.json](../../../../rust/tcl-registry/tests/data/native_lambda_namespace_source/sdk-queue.json). SHA-256 `3eb6871f7e4def2770d65bde01d20ccbe7e7840b3128da03ba072c473292db7e`. Exact retained original source/request/runner, SDK association or raw native stream. Request/receipt ASCII and explicit-escape wording remains immutable; the actual source has literal UTF8 U+00E9 and canonical channel metadata describes those bytes.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NativeNamePurpose`: Retain the independently selected original lambda namespace input/getter route and its separate source or runtime purpose; linked Rust coverage does not promote native field samples or entered frame correspondence.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NativeNameProtocol::lambda_namespace_input`: Retain the independently selected original lambda namespace input/getter route and its separate source or runtime purpose; linked Rust coverage does not promote native field samples or entered frame correspondence.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NativeNameProtocol::lambda_namespace_path`: Retain the independently selected original lambda namespace input/getter route and its separate source or runtime purpose; linked Rust coverage does not promote native field samples or entered frame correspondence.
- [rust/tcl-compiler/src/command_binding/namespace_slots.rs](../../../../rust/tcl-compiler/src/command_binding/namespace_slots.rs), `namespace_for_lambda_operand`: Retain the independently selected original lambda namespace input/getter route and its separate source or runtime purpose; linked Rust coverage does not promote native field samples or entered frame correspondence.
- [rust/tcl-compiler/src/analyser/interp_visibility.rs](../../../../rust/tcl-compiler/src/analyser/interp_visibility.rs), `literal_lambda_body`: Retain the independently selected original lambda namespace input/getter route and its separate source or runtime purpose; linked Rust coverage does not promote native field samples or entered frame correspondence.
- [rust/tcl-vm/src/command.rs](../../../../rust/tcl-vm/src/command.rs), `lambda_namespace`: Retain the independently selected original lambda namespace input/getter route and its separate source or runtime purpose; linked Rust coverage does not promote native field samples or entered frame correspondence.
- [runtime/rust/src/cmd_proc.rs](../../../../runtime/rust/src/cmd_proc.rs), `apply_native_c`: Retain the independently selected original lambda namespace input/getter route and its separate source or runtime purpose; linked Rust coverage does not promote native field samples or entered frame correspondence.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-registry/tests/data/native_lambda_namespace_source/capture.py"
]
```

Retained runner uses original absolute SDK associations and writes provider folders; exact executable/environment pins and a fresh output directory are required. Keep literal UTF8 source and binary-produced inputs unchanged. CLI rendering does not attest native getter bytes, original object identity or raw String00 extent.
