# naming.alias.original-child-interpreter-publication

Kind: `native-observation`

## Problem statement

Parent-to-child alias creation uses provider-specific APIs and can retain different returned values while accepting the same original call spellings. Cross-interpreter ports must preserve each API result instead of inferring shared storage or a single returned-name protocol.

## Question

How do rooted, relative-qualified, unqualified and extra-root alias names publish and resolve for parent-to-child aliases under each provider's actual supported child-interpreter API?

## Conclusion

Each recorded C release uses interp create, interp alias $child $name {} list VALUE and interp eval $child. For the four exact names ::A::a, A::relative, plain and ::::extra, creation is caught with code zero, returns the original alias name, and the child call returns VALUE with code zero. Jim 0.84-9-g5bac7c9 uses interp to create its child handle, $child alias $name list VALUE and $child eval. Its four creation codes are also zero, but each creation result is the empty list field {}; each child call returns VALUE with code zero. Both branches delete their child through the corresponding API and all six whole processes exit zero with empty stderr. The original per-provider rows retain the return-value distinction. Matching public child invocation results establish no private table-key, allocation, cross-interpreter storage, alias header, frame, namespace-token or complete API equivalence. BIG-IP is not tested, and no source-carrier or edit authority follows.

## Scope

One original fixed ASCII LF file per fresh provider process selects an explicit C versus Jim branch by its reported patchlevel, creates one child, tests four original alias names and deletes the child. The aliases target the parent list VALUE prefix; child evaluation is exactly [list $name]. No nonroot child current namespace, arbitrary target, target rename/delete/replacement, alias loop, NUL, Unicode, continuation/control escape, entered receiver frame or interpreter lifetime after deletion is measured. API return values are retained independently and not equated.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Exact CLI executable SHA-256 f0e742ccc08a150afd9a0a236ce24c8e12098dae38c0efd46e59f9b1a28bf58a; original required library/header/Makefile/source associations and recorded environment are retained and independently rechecked. No fresh build command or compiler binary version is recorded.. Channel: ASCII LF source file; explicit provider API branches, C interp alias and Jim child-handle alias; public returned names and child calls, no private key or shared storage claim. Dialect: C Tcl stock interp alias CLI.

Actual C interp alias API 8.4.20 records four exact rows: child_alias ::A::a 0 ::A::a 0 VALUE; child_alias A::relative 0 A::relative 0 VALUE; child_alias plain 0 plain 0 VALUE; child_alias ::::extra 0 ::::extra 0 VALUE. Creation results remain the original alias spellings while each child call returns VALUE with code zero. This is the measured provider-specific API branch; no return-value or private storage/key/frame equivalence is inferred. Whole process exits zero with empty stderr.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Exact CLI executable SHA-256 e7eb17821aeb7b1f84619f431e89ecff667bc5461fa8e00d67498b6b7d5dd61a; original required library/header/Makefile/source associations and recorded environment are retained and independently rechecked. No fresh build command or compiler binary version is recorded.. Channel: ASCII LF source file; explicit provider API branches, C interp alias and Jim child-handle alias; public returned names and child calls, no private key or shared storage claim. Dialect: C Tcl stock interp alias CLI.

Actual C interp alias API 8.5.19 records four exact rows: child_alias ::A::a 0 ::A::a 0 VALUE; child_alias A::relative 0 A::relative 0 VALUE; child_alias plain 0 plain 0 VALUE; child_alias ::::extra 0 ::::extra 0 VALUE. Creation results remain the original alias spellings while each child call returns VALUE with code zero. This is the measured provider-specific API branch; no return-value or private storage/key/frame equivalence is inferred. Whole process exits zero with empty stderr.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Exact CLI executable SHA-256 9b156375d068a58fcfb82a2a0afd170afd2b03912cbb6e588676c7688118c8ff; original required library/header/Makefile/source associations and recorded environment are retained and independently rechecked. No fresh build command or compiler binary version is recorded.. Channel: ASCII LF source file; explicit provider API branches, C interp alias and Jim child-handle alias; public returned names and child calls, no private key or shared storage claim. Dialect: C Tcl stock interp alias CLI.

Actual C interp alias API 8.6.18 records four exact rows: child_alias ::A::a 0 ::A::a 0 VALUE; child_alias A::relative 0 A::relative 0 VALUE; child_alias plain 0 plain 0 VALUE; child_alias ::::extra 0 ::::extra 0 VALUE. Creation results remain the original alias spellings while each child call returns VALUE with code zero. This is the measured provider-specific API branch; no return-value or private storage/key/frame equivalence is inferred. Whole process exits zero with empty stderr.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Exact CLI executable SHA-256 f4748357cdbf2762f1c5c05916fa53e54718d087120ae9de9c403bbdfaf764b1; original required library/header/Makefile/source associations and recorded environment are retained and independently rechecked. No fresh build command or compiler binary version is recorded.. Channel: ASCII LF source file; explicit provider API branches, C interp alias and Jim child-handle alias; public returned names and child calls, no private key or shared storage claim. Dialect: C Tcl stock interp alias CLI.

Actual C interp alias API 9.0.4 records four exact rows: child_alias ::A::a 0 ::A::a 0 VALUE; child_alias A::relative 0 A::relative 0 VALUE; child_alias plain 0 plain 0 VALUE; child_alias ::::extra 0 ::::extra 0 VALUE. Creation results remain the original alias spellings while each child call returns VALUE with code zero. This is the measured provider-specific API branch; no return-value or private storage/key/frame equivalence is inferred. Whole process exits zero with empty stderr.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Exact CLI executable SHA-256 2a82f721b68f2ed2347ca4e3daa9a56c49c9e3efce77d00f22fc3b2012d8814d; original required library/header/Makefile/source associations and recorded environment are retained and independently rechecked. No fresh build command or compiler binary version is recorded.. Channel: ASCII LF source file; explicit provider API branches, C interp alias and Jim child-handle alias; public returned names and child calls, no private key or shared storage claim. Dialect: C Tcl stock interp alias CLI.

Actual C interp alias API 9.1.0 records four exact rows: child_alias ::A::a 0 ::A::a 0 VALUE; child_alias A::relative 0 A::relative 0 VALUE; child_alias plain 0 plain 0 VALUE; child_alias ::::extra 0 ::::extra 0 VALUE. Creation results remain the original alias spellings while each child call returns VALUE with code zero. This is the measured provider-specific API branch; no return-value or private storage/key/frame equivalence is inferred. Whole process exits zero with empty stderr.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Exact CLI executable SHA-256 e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806; original required library/header/Makefile/source associations and recorded environment are retained and independently rechecked. No fresh build command or compiler binary version is recorded.. Channel: ASCII LF source file; explicit provider API branches, C interp alias and Jim child-handle alias; public returned names and child calls, no private key or shared storage claim. Dialect: Jim actual interp interface.

Actual Jim child-handle alias API 0.84-9-g5bac7c9 records four exact rows: child_alias ::A::a 0 {} 0 VALUE; child_alias A::relative 0 {} 0 VALUE; child_alias plain 0 {} 0 VALUE; child_alias ::::extra 0 {} 0 VALUE. Creation results remain empty while each child call returns VALUE with code zero. This is the measured provider-specific API branch; no return-value or private storage/key/frame equivalence is inferred. Whole process exits zero with empty stderr.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: BIG-IP.

No appliance execution answers this exact API/input question; stock C/Jim public outcomes supply no hosted result.

## Exact evidence

- `alias199-probe.tcl` (input): [rust/tcl-registry/tests/data/native_child_alias_publication199/probe.tcl](../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/probe.tcl). SHA-256 `d679427bf3e3a4da804be1915547b472eafd3b4df79f0198f8ed465087f5152a`. Exact original ASCII LF source executed by all six providers.
- `alias199-request.json` (input): [rust/tcl-registry/tests/data/native_child_alias_publication199/request.json](../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/request.json). SHA-256 `fdbadae34e74e91e505a48d1bd3daa8175499d8e1caffc9ad95b7bbf98e4d141`. Unchanged authored human question, required providers and original source/input channel; canonical catalogue ID is assigned independently.
- `alias199-capture.py` (input): [rust/tcl-registry/tests/data/native_child_alias_publication199/capture.py](../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/capture.py). SHA-256 `7463df321145dab386e700a211247a4abc3b78a6e10b0725de4c31d50053768f`. Exact Root-executed launcher bytes; original external provider paths, explicit source/executable/artifact checks and recorded process commands. No new launch is claimed.
- `alias199-8.4.20-receipt.json` (provider): [rust/tcl-registry/tests/data/native_child_alias_publication199/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/8.4.20/receipt.json). SHA-256 `44c1b7c358ab96e3e9dc95430dca7a9819dcfb805315416b807c1e6fef76ec64`. Whole actual process receipt, original command/environment/version and required executable/library/header/build/source pin associations.
- `alias199-8.4.20-stdout` (observation): [rust/tcl-registry/tests/data/native_child_alias_publication199/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/8.4.20/stdout). SHA-256 `604b5bb9c8ebf4bcccf822a9f5d02a605ddf26bc1d5460cd5c449da755961f8d`. Whole exact stdout; original public codes, name spellings, calls and provider-specific results are retained without normalisation.
- `alias199-8.4.20-stderr` (observation): [rust/tcl-registry/tests/data/native_child_alias_publication199/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Whole exact empty process stderr.
- `alias199-8.5.19-receipt.json` (provider): [rust/tcl-registry/tests/data/native_child_alias_publication199/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/8.5.19/receipt.json). SHA-256 `1b5f4d9b1268830f822cc250acbc8fb18dab118079dc330b23a2e961eba4f451`. Whole actual process receipt, original command/environment/version and required executable/library/header/build/source pin associations.
- `alias199-8.5.19-stdout` (observation): [rust/tcl-registry/tests/data/native_child_alias_publication199/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/8.5.19/stdout). SHA-256 `20669f079fd54f2fd89bee4492481accd1e81d3fa8a9950e1de954277e9eb764`. Whole exact stdout; original public codes, name spellings, calls and provider-specific results are retained without normalisation.
- `alias199-8.5.19-stderr` (observation): [rust/tcl-registry/tests/data/native_child_alias_publication199/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Whole exact empty process stderr.
- `alias199-8.6.18-receipt.json` (provider): [rust/tcl-registry/tests/data/native_child_alias_publication199/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/8.6.18/receipt.json). SHA-256 `a7d8ee8e6bb97d019df36fcb395f4bd6c673599843a484fab15e3c2b51656b65`. Whole actual process receipt, original command/environment/version and required executable/library/header/build/source pin associations.
- `alias199-8.6.18-stdout` (observation): [rust/tcl-registry/tests/data/native_child_alias_publication199/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/8.6.18/stdout). SHA-256 `8429bc831b3cd97eb808fbc049b4a02b939dbf933bb54053a383807d3ddb1c4b`. Whole exact stdout; original public codes, name spellings, calls and provider-specific results are retained without normalisation.
- `alias199-8.6.18-stderr` (observation): [rust/tcl-registry/tests/data/native_child_alias_publication199/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Whole exact empty process stderr.
- `alias199-9.0.4-receipt.json` (provider): [rust/tcl-registry/tests/data/native_child_alias_publication199/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/9.0.4/receipt.json). SHA-256 `68a64fe1a10cbe3ad97309d6ec16cf18e5baee60ab67d9da6c461c9bc1d23429`. Whole actual process receipt, original command/environment/version and required executable/library/header/build/source pin associations.
- `alias199-9.0.4-stdout` (observation): [rust/tcl-registry/tests/data/native_child_alias_publication199/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/9.0.4/stdout). SHA-256 `de5bdd38cf013b12d54755720ef5b8395bd4c9cb2e6cb13af9c0eefe48f0d97a`. Whole exact stdout; original public codes, name spellings, calls and provider-specific results are retained without normalisation.
- `alias199-9.0.4-stderr` (observation): [rust/tcl-registry/tests/data/native_child_alias_publication199/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Whole exact empty process stderr.
- `alias199-9.1.0-receipt.json` (provider): [rust/tcl-registry/tests/data/native_child_alias_publication199/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/9.1.0/receipt.json). SHA-256 `ed638434778da70677e798c499df202099375e6d84644867780c5be9cce62c0e`. Whole actual process receipt, original command/environment/version and required executable/library/header/build/source pin associations.
- `alias199-9.1.0-stdout` (observation): [rust/tcl-registry/tests/data/native_child_alias_publication199/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/9.1.0/stdout). SHA-256 `e86a395585e11852bf0b6eb190352cd8a76b8ee9363e4272976e6c6c30ef9144`. Whole exact stdout; original public codes, name spellings, calls and provider-specific results are retained without normalisation.
- `alias199-9.1.0-stderr` (observation): [rust/tcl-registry/tests/data/native_child_alias_publication199/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Whole exact empty process stderr.
- `alias199-jim-receipt.json` (provider): [rust/tcl-registry/tests/data/native_child_alias_publication199/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/jim/receipt.json). SHA-256 `1e849f3509d6cad616cd8103ad31e4289d1ef2d9aec8f2461c52ab760a8d04e9`. Whole actual process receipt, original command/environment/version and required executable/library/header/build/source pin associations.
- `alias199-jim-stdout` (observation): [rust/tcl-registry/tests/data/native_child_alias_publication199/jim/stdout](../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/jim/stdout). SHA-256 `a68b956c2154ca5c9053befa9a2baa43057a3f4284e7e7bd25ffc1aa44b68eb2`. Whole exact stdout; original public codes, name spellings, calls and provider-specific results are retained without normalisation.
- `alias199-jim-stderr` (observation): [rust/tcl-registry/tests/data/native_child_alias_publication199/jim/stderr](../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Whole exact empty process stderr.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/cmd_alias.rs](../../../../runtime/rust/src/cmd_alias.rs), `cmd_alias::tests::child_alias_publication_matches_original_supported_provider_apis` (linked): Independent Runtime alias conformance compares the complete original measured public rows after provider-version metadata through the selected supported API; no extra native process, private key/cell or frame identity follows.
- [rust/tcl-vm/tests/native_compilation_conformance.rs](../../../../rust/tcl-vm/tests/native_compilation_conformance.rs), `native_compilation_conformance::child_alias_publication_matches_original_supported_provider_apis` (linked): Full original source and retained provider stdout bind VM public conformance after version metadata; actual assertions require separate validation receipts and do not establish private interpreter storage or API equivalence.

A named test is a coverage binding, not a claim that it executed.

## Replay

Whole exact commands, streams, environment overrides and required executable/library/header/Makefile/source associations remain in unchanged receipts. Other inherited environment variables are unrecorded. The exact executed launcher depends on original external provider paths and prior receipts, and is not a portable replay promise. Re-running produces new observations. A current source hash association does not prove the executable was built from that source. Publication and proof checking execute no native or Rust process.
