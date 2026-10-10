# naming.procedure-static.original-member-object-boundaries

Kind: `native-observation`

## Problem statement

Converting a static declaration list to UTF-8 or rebuilding literal members can merge counted names and lose the initializer object. Value copy and raw-cell reference capture have distinct definition-time meanings.

## Question

What happens for the finite original-list literal, opaque copy/reference and atomic rejection cases under the selected Jim and C Tcl procedure grammars?

## Conclusion

Jim accepts the five counted names and retains each original nested-list literal value. Opaque copy versus reference observes FIRST versus SECOND after the actual source cell replacement. Missing and duplicate declarations fail without installing p. C8.4–9.1 reject the extra static-list operand. These observations do not grant source compilation, callback/release closure or a missing linked target.

## Scope

Five literal names x, xFF, xD800, xD801 and x/raw-zero/tail; actual original nested-list value, original pointer identity before sole result string getter, opaque xFF copy/reference replacement, missing and duplicate rejection. Independent original object-vector and ASCII query channels are retained. No arbitrary wrapper linkage, arrays, release callbacks, object cache layout, source minifier or BIG-IP behaviour is inferred.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual compiled executable SHA 7c284aab4190ea194c5c67d09b6220d47989017c51d08b1832c5bd24235d6957; compiler argv/path/SHA/version and 5 source/header/library/build pins are retained in the exact provider receipt.. Channel: Counted original native String/list object vectors and retained result pointer before getter; independent ASCII version and command-presence queries.. Dialect: Tcl.

All five literal forms plus copied/reference/missing/duplicate four-operand procedure definitions return code 1 with the recorded wrong-argument diagnostic. No procedure p is installed by the failed definitions. These are static-list grammar availability negatives; body execution is not attempted.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual compiled executable SHA 8184ae5a2c4981a026b6abaad251de161691ec9faa507bc23be726d4d66e2de3; compiler argv/path/SHA/version and 5 source/header/library/build pins are retained in the exact provider receipt.. Channel: Counted original native String/list object vectors and retained result pointer before getter; independent ASCII version and command-presence queries.. Dialect: Tcl.

All five literal forms plus copied/reference/missing/duplicate four-operand procedure definitions return code 1 with the recorded wrong-argument diagnostic. No procedure p is installed by the failed definitions. These are static-list grammar availability negatives; body execution is not attempted.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual compiled executable SHA 4f7fc4011d2d4418866b457cdb4a4e64913df877bd14c6e3f57b47d37f9efaf1; compiler argv/path/SHA/version and 6 source/header/library/build pins are retained in the exact provider receipt.. Channel: Counted original native String/list object vectors and retained result pointer before getter; independent ASCII version and command-presence queries.. Dialect: Tcl.

All five literal forms plus copied/reference/missing/duplicate four-operand procedure definitions return code 1 with the recorded wrong-argument diagnostic. No procedure p is installed by the failed definitions. These are static-list grammar availability negatives; body execution is not attempted.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual compiled executable SHA 3699676451691d6e702bfd087c1d19a6ca13f5d20f6eaf9eed1419a4768ab30c; compiler argv/path/SHA/version and 6 source/header/library/build pins are retained in the exact provider receipt.. Channel: Counted original native String/list object vectors and retained result pointer before getter; independent ASCII version and command-presence queries.. Dialect: Tcl.

All five literal forms plus copied/reference/missing/duplicate four-operand procedure definitions return code 1 with the recorded wrong-argument diagnostic. No procedure p is installed by the failed definitions. These are static-list grammar availability negatives; body execution is not attempted.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual compiled executable SHA 823c4727dd7e81d0557ffafb701aeb2ebbc024a59a8e14990a2fb28cd48cf1ba; compiler argv/path/SHA/version and 6 source/header/library/build pins are retained in the exact provider receipt.. Channel: Counted original native String/list object vectors and retained result pointer before getter; independent ASCII version and command-presence queries.. Dialect: Tcl.

All five literal forms plus copied/reference/missing/duplicate four-operand procedure definitions return code 1 with the recorded wrong-argument diagnostic. No procedure p is installed by the failed definitions. These are static-list grammar availability negatives; body execution is not attempted.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual compiled executable SHA ec65c3a7591ba9e52e9deb789be11b15d2361488a763c7cb6b6c8b4ee3f334dc; compiler argv/path/SHA/version and 5 source/header/library/build pins are retained in the exact provider receipt.. Channel: Counted original native String/list object vectors and retained result pointer before getter; independent ASCII version and command-presence queries.. Dialect: Jim Tcl.

All five listed counted literal names define successfully, and calls return the identical original nested-list initializer object. The opaque copied initializer returns FIRST after source replacement while raw reference returns SECOND, both by original result identity. Missing and duplicate initializers fail and leave p absent.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance execution is attached.

## Exact evidence

- `probe.c` (input): [rust/tcl-registry/tests/data/native_procedure_static_original/probe.c](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/probe.c). SHA-256 `88b0b366ceed66311cc46a667c4a5851d9a34569c2969a82f4e20329b5d46405`. Exact public counted object/list invocation and result identity before the only getter.
- `queue.json` (input): [rust/tcl-registry/tests/data/native_procedure_static_original/queue.json](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/queue.json). SHA-256 `a6fd58fc898fc9ca98b1001f6407b1022da1a4045ff41cf35ba07ec39b16dcf6`. Original finite observation queue; criteria only.
- `capture.py` (input): [rust/tcl-registry/tests/data/native_procedure_static_original/capture.py](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/capture.py). SHA-256 `30c1baccd4cf1d2c24fea4d8e1dac1eb48a6d7b51ca577547facf93f312aed82`. Retained actual compiler/launcher protocol and provider pins.
- `receipt.json` (provider): [rust/tcl-registry/tests/data/native_procedure_static_original/receipt.json](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/receipt.json). SHA-256 `c802437dedddccb703cd1f8edb6209b9c65b7d3190fe46cb4726499034c39283`. Six complete provider captures with source/library/header/compiler/version and stream associations.
- `tcl8.4-receipt.json` (provider): [rust/tcl-registry/tests/data/native_procedure_static_original/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/8.4.20/receipt.json). SHA-256 `565aa10968e93adbcb4befb542d422853e51da21e700fdd397dba2ee7305901b`. Exact original capture bytes; associated complete exit status and versions are retained in the provider receipt.
- `tcl8.4-stdout.tsv` (observation): [rust/tcl-registry/tests/data/native_procedure_static_original/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/8.4.20/stdout.tsv). SHA-256 `97b08704be4c07f9eae6331ccc80efad062c39291c323d7eb02b777aaf96e870`. Exact original capture bytes; associated complete exit status and versions are retained in the provider receipt.
- `tcl8.4-stderr` (observation): [rust/tcl-registry/tests/data/native_procedure_static_original/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original capture bytes; associated complete exit status and versions are retained in the provider receipt.
- `tcl8.4-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_procedure_static_original/8.4.20/compile.stdout](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/8.4.20/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original capture bytes; associated complete exit status and versions are retained in the provider receipt.
- `tcl8.4-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_procedure_static_original/8.4.20/compile.stderr](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/8.4.20/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original capture bytes; associated complete exit status and versions are retained in the provider receipt.
- `tcl8.5-receipt.json` (provider): [rust/tcl-registry/tests/data/native_procedure_static_original/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/8.5.19/receipt.json). SHA-256 `11eb7578bcc52c6c57cc7e26f37d6dde9db63adb4d79c0e3a636ded1a3b17aec`. Exact original capture bytes; associated complete exit status and versions are retained in the provider receipt.
- `tcl8.5-stdout.tsv` (observation): [rust/tcl-registry/tests/data/native_procedure_static_original/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/8.5.19/stdout.tsv). SHA-256 `087732da625c06822d00c33545e240cbe1a13fc51822c5140fae098bb1d19727`. Exact original capture bytes; associated complete exit status and versions are retained in the provider receipt.
- `tcl8.5-stderr` (observation): [rust/tcl-registry/tests/data/native_procedure_static_original/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original capture bytes; associated complete exit status and versions are retained in the provider receipt.
- `tcl8.5-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_procedure_static_original/8.5.19/compile.stdout](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/8.5.19/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original capture bytes; associated complete exit status and versions are retained in the provider receipt.
- `tcl8.5-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_procedure_static_original/8.5.19/compile.stderr](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/8.5.19/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original capture bytes; associated complete exit status and versions are retained in the provider receipt.
- `tcl8.6-receipt.json` (provider): [rust/tcl-registry/tests/data/native_procedure_static_original/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/8.6.18/receipt.json). SHA-256 `8a223a7cce8479993e8b485ebeac8292ab3220cce97881a2637e3a90f2c3a80c`. Exact original capture bytes; associated complete exit status and versions are retained in the provider receipt.
- `tcl8.6-stdout.tsv` (observation): [rust/tcl-registry/tests/data/native_procedure_static_original/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/8.6.18/stdout.tsv). SHA-256 `f6cd022f2ae7c20a07000c04ba672f96f2c690f1e1bb027f4ce0f208d47f9ab8`. Exact original capture bytes; associated complete exit status and versions are retained in the provider receipt.
- `tcl8.6-stderr` (observation): [rust/tcl-registry/tests/data/native_procedure_static_original/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original capture bytes; associated complete exit status and versions are retained in the provider receipt.
- `tcl8.6-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_procedure_static_original/8.6.18/compile.stdout](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/8.6.18/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original capture bytes; associated complete exit status and versions are retained in the provider receipt.
- `tcl8.6-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_procedure_static_original/8.6.18/compile.stderr](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/8.6.18/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original capture bytes; associated complete exit status and versions are retained in the provider receipt.
- `tcl9.0-receipt.json` (provider): [rust/tcl-registry/tests/data/native_procedure_static_original/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/9.0.4/receipt.json). SHA-256 `fa124e23b7c733935d03efbceb7537cce16fdee95f5b868c2ccc9a166b2f7fdb`. Exact original capture bytes; associated complete exit status and versions are retained in the provider receipt.
- `tcl9.0-stdout.tsv` (observation): [rust/tcl-registry/tests/data/native_procedure_static_original/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/9.0.4/stdout.tsv). SHA-256 `7bbf32fb0f9bf0dbe97fe5677d96158683f0a8db8a116ad05d8e363d86d101c2`. Exact original capture bytes; associated complete exit status and versions are retained in the provider receipt.
- `tcl9.0-stderr` (observation): [rust/tcl-registry/tests/data/native_procedure_static_original/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original capture bytes; associated complete exit status and versions are retained in the provider receipt.
- `tcl9.0-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_procedure_static_original/9.0.4/compile.stdout](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/9.0.4/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original capture bytes; associated complete exit status and versions are retained in the provider receipt.
- `tcl9.0-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_procedure_static_original/9.0.4/compile.stderr](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/9.0.4/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original capture bytes; associated complete exit status and versions are retained in the provider receipt.
- `tcl9.1-receipt.json` (provider): [rust/tcl-registry/tests/data/native_procedure_static_original/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/9.1.0/receipt.json). SHA-256 `3bdbb478ca2e57a5959820ede50a5448f22b3e935a50b108241788c5de9163ea`. Exact original capture bytes; associated complete exit status and versions are retained in the provider receipt.
- `tcl9.1-stdout.tsv` (observation): [rust/tcl-registry/tests/data/native_procedure_static_original/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/9.1.0/stdout.tsv). SHA-256 `69b8cac9fd4895203dbebf4e58bd4f5c90ce84c18669827287ff098e62bc9dcc`. Exact original capture bytes; associated complete exit status and versions are retained in the provider receipt.
- `tcl9.1-stderr` (observation): [rust/tcl-registry/tests/data/native_procedure_static_original/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original capture bytes; associated complete exit status and versions are retained in the provider receipt.
- `tcl9.1-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_procedure_static_original/9.1.0/compile.stdout](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/9.1.0/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original capture bytes; associated complete exit status and versions are retained in the provider receipt.
- `tcl9.1-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_procedure_static_original/9.1.0/compile.stderr](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/9.1.0/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original capture bytes; associated complete exit status and versions are retained in the provider receipt.
- `jim-receipt.json` (provider): [rust/tcl-registry/tests/data/native_procedure_static_original/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/jim/receipt.json). SHA-256 `0c875767a2dcdca20998950823b1462e79593522280e6ef0a8a99b462c3a0ba7`. Exact original capture bytes; associated complete exit status and versions are retained in the provider receipt.
- `jim-stdout.tsv` (observation): [rust/tcl-registry/tests/data/native_procedure_static_original/jim/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/jim/stdout.tsv). SHA-256 `d3c9af58ac85e901d8225614926afe4fc2b7a19c3dc37b5356b78dcb093a572a`. Exact original capture bytes; associated complete exit status and versions are retained in the provider receipt.
- `jim-stderr` (observation): [rust/tcl-registry/tests/data/native_procedure_static_original/jim/stderr](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original capture bytes; associated complete exit status and versions are retained in the provider receipt.
- `jim-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_procedure_static_original/jim/compile.stdout](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/jim/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original capture bytes; associated complete exit status and versions are retained in the provider receipt.
- `jim-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_procedure_static_original/jim/compile.stderr](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/jim/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original capture bytes; associated complete exit status and versions are retained in the provider receipt.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/command/native_static_original_tests.rs](../../../../rust/tcl-vm/src/command/native_static_original_tests.rs), `command::native_static_original_tests::original_static_member_objects_match_native_counted_names_and_value_capture` (linked): Recreates original native list members and counted names; compares all recorded declaration codes/messages, literal/copy/reference result identity and bytes, plus atomic missing/duplicate command absence, in six independently selected VM contexts.
- [runtime/rust/src/cmd_proc/native_static_original_tests.rs](../../../../runtime/rust/src/cmd_proc/native_static_original_tests.rs), `cmd_proc::native_static_original_tests::original_static_member_objects_match_native_counted_names_and_value_capture` (linked): Separately compares original Runtime object pointers, counted declaration/value bytes and atomic failed publication against every listed provider row. No Rust execution result is inferred.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-registry/tests/data/native_procedure_static_original/capture.py"
]
```

The retained runner describes the exact captured absolute paths and immutable header/library/compiler associations. Replays require the matching pinned providers and a fresh output directory; verify complete process exits and raw streams. No Rust pass is inferred from native observations.
