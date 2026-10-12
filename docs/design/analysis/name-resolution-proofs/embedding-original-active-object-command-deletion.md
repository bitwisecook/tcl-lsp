# naming.embedding.original-active-object-command-deletion

Kind: `native-observation`

## Problem statement

Deletion of an idle public command does not establish when its deletion callback runs during an active object-command invocation. Deferring the deletion callback until the active command returns changes its observable event sequence.

## Question

When an original publicly registered object command deletes itself inside its actual active callback, where does its deletion callback occur relative to deletion-API return, callback INVOKE_RETURN and first public evaluation return, and does a once-only same-name publication by that callback receive the second invocation?

## Conclusion

The five measured C providers reach DELETE_ORIGINAL before DELETE_API_RETURN and INVOKE_RETURN. Measured Jim0.84-9-g5bac7c9 reaches DELETE_ORIGINAL after INVOKE_RETURN and before the first public evaluation returns. All six return exact ORIGINAL_RETURN from that first evaluation. Without recreation the second evaluation fails with the same exact invalid-command result. With once-only recreation the same held caller word reaches the replacement once and returns exact REPLACEMENT on the second evaluation; original and replacement deletion each occur once through full interpreter teardown. These are138 ordered event/result rows across12 fresh active cases in6 closed probe processes. Each independent bare-list CLI query also exits0 but emits no version text; actual linked VERSION/BUILD/ABI rows provide provider attribution. Idle replacement measurements remain independent. No client-data free, reference-count, procedure, imported-command, recursive deletion, rename/hide, Rust or BIG-IP parity follows.

## Scope

Exact original C99 probe SHA 3572cff663d103e59a8e814a7b15002acd5c8320cbf8c415c8fabf04f48a9a05 on measured C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and Jim0.84-9-g5bac7c9. Mode0 actively deletes without recreation; mode1 recreates the same public command name once from the original deletion callback. Independent stack-backed client records stay alive through full teardown and are deliberately not freed by the callback. One caller-name object is held through both actual vector evaluations and then released; no extra command-record or protective reference is introduced. C uses Tcl_CreateObjCommand legacy int-count registration, including Tcl9 compatibility, not the size-count registration family. A third independent interpreter per probe supplies VERSION metadata; its result does not donate callback evidence. Full source/header/config/static-library/CLI/compiler/link input hashes and original before/after inventories are associated with their actual receipts. Whole source snippets use physical LF and preserve terminal LF and whitespace. SDK configure/build replay and source-to-library equivalence are unrecorded; external toolchain/system binaries lacking archived payloads remain hash-only. Application issuers, namespace/source/frame receipts, unmeasured command families, object-address/incarnation and general active lifetime claims remain unavailable.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual linked-probe VERSION/BUILD/ABI: VERSION	0	382e342e323020382e34; BUILD	HEADER=8.4.20	GetVersion=8.4.20/2; ABI	int=4	long=8	ptr=8	CHAR_BIT=8; whole ELF SHA 43acfd89fa1a5a9cfe5533b237347d80269fe4a33094e694218ea599c8530c56. Exact header/compiler/link/static-library and input hashes retained. Independent original CLI query exits0 with empty stdout/stderr; it supplies no separate version text. SDK configure/build replay and source-to-library equivalence are unrecorded.. Channel: Original C Tcl_CreateObjCommand legacy int-count registration/Tcl_DeleteCommandFromToken/Tcl_EvalObjv or Jim_CreateCommand/Jim_DeleteCommand/Jim_EvalObjVector, two fresh initialized case interpreters, one held caller word through both evaluations, complete raw event/result and teardown streams.. Dialect: Selected actual initialized C Tcl provider; C9 compatibility int-count callback remains distinct from size-count registration..

DELETE_ORIGINAL occurs before DELETE_API_RETURN and INVOKE_RETURN in both active cases. Both first evaluations return code0 and exact ORIGINAL_RETURN. Case0 second evaluation returns code1 and exact invalid command name "active_retiring". Case1 recreates once in the original deletion callback; the held caller word second evaluation returns code0 and exact REPLACEMENT, reaches replacement once, and full teardown reaches original and replacement deletion exactly once. All23 original event/result rows, including caller-word release and interpreter destruction, are retained.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual linked-probe VERSION/BUILD/ABI: VERSION	0	382e352e313920382e35; BUILD	HEADER=8.5.19	GetVersion=8.5.19/2; ABI	int=4	long=8	ptr=8	CHAR_BIT=8; whole ELF SHA 6d3b1a303f6b7af7d71fb188b7410a43f4f2a50ee58139f74555a4e31dcb2fce. Exact header/compiler/link/static-library and input hashes retained. Independent original CLI query exits0 with empty stdout/stderr; it supplies no separate version text. SDK configure/build replay and source-to-library equivalence are unrecorded.. Channel: Original C Tcl_CreateObjCommand legacy int-count registration/Tcl_DeleteCommandFromToken/Tcl_EvalObjv or Jim_CreateCommand/Jim_DeleteCommand/Jim_EvalObjVector, two fresh initialized case interpreters, one held caller word through both evaluations, complete raw event/result and teardown streams.. Dialect: Selected actual initialized C Tcl provider; C9 compatibility int-count callback remains distinct from size-count registration..

DELETE_ORIGINAL occurs before DELETE_API_RETURN and INVOKE_RETURN in both active cases. Both first evaluations return code0 and exact ORIGINAL_RETURN. Case0 second evaluation returns code1 and exact invalid command name "active_retiring". Case1 recreates once in the original deletion callback; the held caller word second evaluation returns code0 and exact REPLACEMENT, reaches replacement once, and full teardown reaches original and replacement deletion exactly once. All23 original event/result rows, including caller-word release and interpreter destruction, are retained.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual linked-probe VERSION/BUILD/ABI: VERSION	0	382e362e313820382e36; BUILD	HEADER=8.6.18	GetVersion=8.6.18/2; ABI	int=4	long=8	ptr=8	CHAR_BIT=8; whole ELF SHA 2be2675e3c60f77dc16c648f3bc14134ac69eeaa9dd3c901ce63b07153a99316. Exact header/compiler/link/static-library and input hashes retained. Independent original CLI query exits0 with empty stdout/stderr; it supplies no separate version text. SDK configure/build replay and source-to-library equivalence are unrecorded.. Channel: Original C Tcl_CreateObjCommand legacy int-count registration/Tcl_DeleteCommandFromToken/Tcl_EvalObjv or Jim_CreateCommand/Jim_DeleteCommand/Jim_EvalObjVector, two fresh initialized case interpreters, one held caller word through both evaluations, complete raw event/result and teardown streams.. Dialect: Selected actual initialized C Tcl provider; C9 compatibility int-count callback remains distinct from size-count registration..

DELETE_ORIGINAL occurs before DELETE_API_RETURN and INVOKE_RETURN in both active cases. Both first evaluations return code0 and exact ORIGINAL_RETURN. Case0 second evaluation returns code1 and exact invalid command name "active_retiring". Case1 recreates once in the original deletion callback; the held caller word second evaluation returns code0 and exact REPLACEMENT, reaches replacement once, and full teardown reaches original and replacement deletion exactly once. All23 original event/result rows, including caller-word release and interpreter destruction, are retained.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual linked-probe VERSION/BUILD/ABI: VERSION	0	392e302e3420392e30; BUILD	HEADER=9.0.4	GetVersion=9.0.4/2; ABI	int=4	long=8	ptr=8	CHAR_BIT=8; whole ELF SHA c9de9302a293fe72275df01c01bcf42ece429c8d56d9382ddff4c9c5ce904574. Exact header/compiler/link/static-library and input hashes retained. Independent original CLI query exits0 with empty stdout/stderr; it supplies no separate version text. SDK configure/build replay and source-to-library equivalence are unrecorded.. Channel: Original C Tcl_CreateObjCommand legacy int-count registration/Tcl_DeleteCommandFromToken/Tcl_EvalObjv or Jim_CreateCommand/Jim_DeleteCommand/Jim_EvalObjVector, two fresh initialized case interpreters, one held caller word through both evaluations, complete raw event/result and teardown streams.. Dialect: Selected actual initialized C Tcl provider; C9 compatibility int-count callback remains distinct from size-count registration..

DELETE_ORIGINAL occurs before DELETE_API_RETURN and INVOKE_RETURN in both active cases. Both first evaluations return code0 and exact ORIGINAL_RETURN. Case0 second evaluation returns code1 and exact invalid command name "active_retiring". Case1 recreates once in the original deletion callback; the held caller word second evaluation returns code0 and exact REPLACEMENT, reaches replacement once, and full teardown reaches original and replacement deletion exactly once. All23 original event/result rows, including caller-word release and interpreter destruction, are retained.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual linked-probe VERSION/BUILD/ABI: VERSION	0	392e312e3020392e31; BUILD	HEADER=9.1.0	GetVersion=9.1.0/2; ABI	int=4	long=8	ptr=8	CHAR_BIT=8; whole ELF SHA ab4a5f20aa19ecdafabb90b3af0b5a3397642b9fb583419abd06d2d225545f05. Exact header/compiler/link/static-library and input hashes retained. Independent original CLI query exits0 with empty stdout/stderr; it supplies no separate version text. SDK configure/build replay and source-to-library equivalence are unrecorded.. Channel: Original C Tcl_CreateObjCommand legacy int-count registration/Tcl_DeleteCommandFromToken/Tcl_EvalObjv or Jim_CreateCommand/Jim_DeleteCommand/Jim_EvalObjVector, two fresh initialized case interpreters, one held caller word through both evaluations, complete raw event/result and teardown streams.. Dialect: Selected actual initialized C Tcl provider; C9 compatibility int-count callback remains distinct from size-count registration..

DELETE_ORIGINAL occurs before DELETE_API_RETURN and INVOKE_RETURN in both active cases. Both first evaluations return code0 and exact ORIGINAL_RETURN. Case0 second evaluation returns code1 and exact invalid command name "active_retiring". Case1 recreates once in the original deletion callback; the held caller word second evaluation returns code0 and exact REPLACEMENT, reaches replacement once, and full teardown reaches original and replacement deletion exactly once. All23 original event/result rows, including caller-word release and interpreter destruction, are retained.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual linked-probe VERSION/BUILD/ABI: VERSION	0	302e38342d392d673562616337633920302e3834; BUILD	JIM_GITVERSION=0.84-9-g5bac7c9	JIM_VERSION=84	JIM_ABI_VERSION=102	JIM_UTF8=1; ABI	int=4	long=8	ptr=8	CHAR_BIT=8; whole ELF SHA 32feb36adbbd51460b7f1c4193555b81e93f0ce13d577d6dc7909069685e95f1. Exact header/compiler/link/static-library and input hashes retained. Independent original CLI query exits0 with empty stdout/stderr; it supplies no separate version text. SDK configure/build replay and source-to-library equivalence are unrecorded.. Channel: Original C Tcl_CreateObjCommand legacy int-count registration/Tcl_DeleteCommandFromToken/Tcl_EvalObjv or Jim_CreateCommand/Jim_DeleteCommand/Jim_EvalObjVector, two fresh initialized case interpreters, one held caller word through both evaluations, complete raw event/result and teardown streams.. Dialect: Selected actual initialized Jim provider; C9 compatibility int-count callback remains distinct from size-count registration..

DELETE_API_RETURN and INVOKE_RETURN precede DELETE_ORIGINAL in both active cases; deletion and optional recreation finish before first-public-invocation-returned. Both first evaluations return code0 and exact ORIGINAL_RETURN. Case0 second evaluation returns code1 and exact invalid command name "active_retiring". Case1 recreates once in the original deletion callback; the held caller word second evaluation returns code0 and exact REPLACEMENT, reaches replacement once, and full teardown reaches original and replacement deletion exactly once. All23 original event/result rows, including caller-word release and interpreter destruction, are retained.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: BIG-IP; active public object-command experiment not executed.

No BIG-IP execution or active command deletion result is attached. C Tcl/Jim callback ordering supplies no appliance parity.

## Exact evidence

- `active671-summary-json` (observation): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/summary.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/summary.json). SHA-256 `c63b474c5c165847870924f04c98ddba62486af8ac9a6b663fdc493bc2c0a794`. Complete original experiment input, input inventory or compiler-identity receipt and stream. Original bytes and captured status remain authoritative.
- `active671-request-json` (input): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/request.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/request.json). SHA-256 `0bbcc3c527ee19eadb73f9b00556f3fe6440d5a2769233cc496eb81228602aff`. Complete original experiment input, input inventory or compiler-identity receipt and stream. Original bytes and captured status remain authoritative.
- `active671-probe-c` (input): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/probe.c](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/probe.c). SHA-256 `3572cff663d103e59a8e814a7b15002acd5c8320cbf8c415c8fabf04f48a9a05`. Complete original experiment input, input inventory or compiler-identity receipt and stream. Original bytes and captured status remain authoritative.
- `active671-design-probe-c` (input): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/design-probe.c](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/design-probe.c). SHA-256 `ea5cd968ccea5352cabd92407a0239e29ff6392a23fcaddbec9ebe76159f05bc`. Complete original experiment input, input inventory or compiler-identity receipt and stream. Original bytes and captured status remain authoritative.
- `active671-design-REQUEST-json` (input): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/design-REQUEST.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/design-REQUEST.json). SHA-256 `ecc32a827ba486268852577355e821e16b4030d5751ea70734bf537e5c010a21`. Complete original experiment input, input inventory or compiler-identity receipt and stream. Original bytes and captured status remain authoritative.
- `active671-design-SOURCE-WINDOWS-json` (input): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/design-SOURCE-WINDOWS.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/design-SOURCE-WINDOWS.json). SHA-256 `9ddeb2afe46e8e06c3d76d9490277813f6fe9f02fe96caf86ca3c7691ccc361d`. Complete original experiment input, input inventory or compiler-identity receipt and stream. Original bytes and captured status remain authoritative.
- `active671-design-BASE-HARNESS-REFERENCE-json` (input): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/design-BASE-HARNESS-REFERENCE.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/design-BASE-HARNESS-REFERENCE.json). SHA-256 `6d037d59fa1c97fc04b777eca2dba1024f864e048e8852cb458137175f7f17f4`. Complete original experiment input, input inventory or compiler-identity receipt and stream. Original bytes and captured status remain authoritative.
- `active671-c-version-tcl` (input): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/c-version.tcl](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/c-version.tcl). SHA-256 `48ea443887443eb394745e58dc0921e1488421a1f781fb468f80cbc4da147cdd`. Complete original experiment input, input inventory or compiler-identity receipt and stream. Original bytes and captured status remain authoritative.
- `active671-jim-version-tcl` (input): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim-version.tcl](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim-version.tcl). SHA-256 `46f8390c69828657c3232ee571f00988b5dad2f520f441d40028de69cfa38770`. Complete original experiment input, input inventory or compiler-identity receipt and stream. Original bytes and captured status remain authoritative.
- `active671-compiler-version-json` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/compiler-version.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/compiler-version.json). SHA-256 `bbf0539d98f9c797dba37631004de266fea7cf82b94279d129e0e29fe2d89273`. Complete original experiment input, input inventory or compiler-identity receipt and stream. Original bytes and captured status remain authoritative.
- `active671-compiler-version-stdout` (observation): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/compiler-version.stdout](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/compiler-version.stdout). SHA-256 `2ae7618a3dfad6644f70dbf191f205882a96d3aac6350978839c4c19389f95a5`. Complete original experiment input, input inventory or compiler-identity receipt and stream. Original bytes and captured status remain authoritative.
- `active671-compiler-version-stderr` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/compiler-version.stderr](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/compiler-version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete original experiment input, input inventory or compiler-identity receipt and stream. Original bytes and captured status remain authoritative.
- `active671-inputs-before-json` (input): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/inputs-before.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/inputs-before.json). SHA-256 `a92c6578dad968dbdfdc332f6ea81a73dd3cc6983d679e2e686b3b2378176aea`. Complete original experiment input, input inventory or compiler-identity receipt and stream. Original bytes and captured status remain authoritative.
- `active671-inputs-after-json` (input): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/inputs-after.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/inputs-after.json). SHA-256 `a92c6578dad968dbdfdc332f6ea81a73dd3cc6983d679e2e686b3b2378176aea`. Complete original experiment input, input inventory or compiler-identity receipt and stream. Original bytes and captured status remain authoritative.
- `active671-runner-py` (input): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/runner.py](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/runner.py). SHA-256 `67952b55113c6e3b80300c6f73935b08ea7076334bc22eb09032e497394a6e89`. Original Root experiment runner; no replay invocation is claimed by this record.
- `active671-8-4-20-receipt-json` (observation): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.4.20/receipt.json). SHA-256 `fd5fd85ad69b21f31fecfb23884a112538aa54be5a8a59e3a443068b3f64a8c8`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-4-20-headers-json` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.4.20/headers.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.4.20/headers.json). SHA-256 `16761c5a3490d1f90ed5a23154f0a52b4defa0baef4134e208c7a4b2be66b53a`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-4-20-headers-stdout` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.4.20/headers.stdout](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.4.20/headers.stdout). SHA-256 `61b244959d89ec5d8677f2212dacdefd564927d94bfa2093514e6940870dbd0f`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-4-20-headers-stderr` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.4.20/headers.stderr](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.4.20/headers.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-4-20-compile-json` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.4.20/compile.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.4.20/compile.json). SHA-256 `4a49ac22b479903f205bf0cde35015a55306ac5b3cf521f0763b360fbc53a61e`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-4-20-compile-stdout` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.4.20/compile.stdout](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.4.20/compile.stdout). SHA-256 `fccf1c327ba9d5468dfc72b67cf49a0c0dbb4779b76763383a7575df156d0066`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-4-20-compile-stderr` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.4.20/compile.stderr](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.4.20/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-4-20-cli-version-json` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.4.20/cli-version.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.4.20/cli-version.json). SHA-256 `01004ba214241afd0f9d993c061829b09ef4832db7c74b0e0fedf8423c20f5a4`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-4-20-cli-version-stdout` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.4.20/cli-version.stdout](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.4.20/cli-version.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-4-20-cli-version-stderr` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.4.20/cli-version.stderr](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.4.20/cli-version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-4-20-probe-json` (observation): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.4.20/probe.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.4.20/probe.json). SHA-256 `727603b8809485975a077bee1cf796070d5ff79c9a8f72bec3f0bf683ca4ecd7`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-4-20-probe-stdout` (observation): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.4.20/probe.stdout](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.4.20/probe.stdout). SHA-256 `279a0ab59956abeca88f48fe7fb6a10682ebfde379e71f6044434a144b876221`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-4-20-probe-stderr` (observation): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.4.20/probe.stderr](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.4.20/probe.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-4-20-inputs-before-json` (input): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.4.20/inputs-before.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.4.20/inputs-before.json). SHA-256 `71aa36714635130f73b71f9c153f9c8680ccd89c160994f84f8c6fcc344b7cd0`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-4-20-inputs-after-json` (input): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.4.20/inputs-after.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.4.20/inputs-after.json). SHA-256 `71aa36714635130f73b71f9c153f9c8680ccd89c160994f84f8c6fcc344b7cd0`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-5-19-receipt-json` (observation): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.5.19/receipt.json). SHA-256 `040e2af26ae63c7ee37d2771bb6f4853c35a0f227c51f1d9808aa80f276e4257`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-5-19-headers-json` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.5.19/headers.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.5.19/headers.json). SHA-256 `28fbb36f59ffd5f502833adf3d9964018d2202f667e6d8575ddc203919ce51f1`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-5-19-headers-stdout` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.5.19/headers.stdout](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.5.19/headers.stdout). SHA-256 `8d14d6eb868f37ce8faa9d46ce7dd2f3d92fc3146626a26bbddf3234836fbe9e`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-5-19-headers-stderr` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.5.19/headers.stderr](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.5.19/headers.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-5-19-compile-json` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.5.19/compile.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.5.19/compile.json). SHA-256 `94adee9eb66449f7acf051c71c547c99b8e94bf1837dc1a9e3aedcc200426176`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-5-19-compile-stdout` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.5.19/compile.stdout](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.5.19/compile.stdout). SHA-256 `83ece261abdc97dbf74d39031e0d3e8358ef805aaa371be01958187840107bd6`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-5-19-compile-stderr` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.5.19/compile.stderr](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.5.19/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-5-19-cli-version-json` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.5.19/cli-version.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.5.19/cli-version.json). SHA-256 `0f0e541c87b9859054a58d4ed546cf41f64796ac29a4f9a5e921433f5a3c81fa`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-5-19-cli-version-stdout` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.5.19/cli-version.stdout](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.5.19/cli-version.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-5-19-cli-version-stderr` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.5.19/cli-version.stderr](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.5.19/cli-version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-5-19-probe-json` (observation): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.5.19/probe.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.5.19/probe.json). SHA-256 `f1f16e0c34ea3cf20dadba169ee95eb83ce5590ac33c61c249eab97027dcceae`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-5-19-probe-stdout` (observation): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.5.19/probe.stdout](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.5.19/probe.stdout). SHA-256 `ac4b5b9effdc27e1cd3b30348a6df7f82a79ce8e4d5327c74f0d4178a6c50006`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-5-19-probe-stderr` (observation): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.5.19/probe.stderr](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.5.19/probe.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-5-19-inputs-before-json` (input): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.5.19/inputs-before.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.5.19/inputs-before.json). SHA-256 `71aa36714635130f73b71f9c153f9c8680ccd89c160994f84f8c6fcc344b7cd0`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-5-19-inputs-after-json` (input): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.5.19/inputs-after.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.5.19/inputs-after.json). SHA-256 `71aa36714635130f73b71f9c153f9c8680ccd89c160994f84f8c6fcc344b7cd0`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-6-18-receipt-json` (observation): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.6.18/receipt.json). SHA-256 `e993f1a4d7a6465fbd3729cab7f891039cd4937bcc3200656a83d677e2236e28`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-6-18-headers-json` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.6.18/headers.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.6.18/headers.json). SHA-256 `d16c20d1bc4cdb579ca2bb6a294fa21a21fc75ffb7d8d737a89c8e0913c24eb0`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-6-18-headers-stdout` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.6.18/headers.stdout](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.6.18/headers.stdout). SHA-256 `7e1809670cdeacff502f73a33edffad41e316885fdae57448cd7295d73715663`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-6-18-headers-stderr` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.6.18/headers.stderr](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.6.18/headers.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-6-18-compile-json` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.6.18/compile.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.6.18/compile.json). SHA-256 `aa02d884df366f719b222c2ffea5966e47378da85e9c563896ca502e55d4de99`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-6-18-compile-stdout` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.6.18/compile.stdout](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.6.18/compile.stdout). SHA-256 `103f4a76e736260fb795ea12bee85d5db1c3dbf839b9d9014c3292a6c847ee0d`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-6-18-compile-stderr` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.6.18/compile.stderr](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.6.18/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-6-18-cli-version-json` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.6.18/cli-version.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.6.18/cli-version.json). SHA-256 `d7732ed6f126086ac9ecc97d2984beae479928a720094ec24c83381f65579769`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-6-18-cli-version-stdout` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.6.18/cli-version.stdout](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.6.18/cli-version.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-6-18-cli-version-stderr` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.6.18/cli-version.stderr](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.6.18/cli-version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-6-18-probe-json` (observation): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.6.18/probe.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.6.18/probe.json). SHA-256 `531a9ef5196bb576338dde5ac9439745a40ea89f4438865f865af2e469618947`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-6-18-probe-stdout` (observation): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.6.18/probe.stdout](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.6.18/probe.stdout). SHA-256 `3134ba085979de36998aeae8e59a99cbe4d3f3aec2d8b5dcf36fd698e5978c32`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-6-18-probe-stderr` (observation): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.6.18/probe.stderr](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.6.18/probe.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-6-18-inputs-before-json` (input): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.6.18/inputs-before.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.6.18/inputs-before.json). SHA-256 `71aa36714635130f73b71f9c153f9c8680ccd89c160994f84f8c6fcc344b7cd0`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-8-6-18-inputs-after-json` (input): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.6.18/inputs-after.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.6.18/inputs-after.json). SHA-256 `71aa36714635130f73b71f9c153f9c8680ccd89c160994f84f8c6fcc344b7cd0`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-9-0-4-receipt-json` (observation): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.0.4/receipt.json). SHA-256 `8b0203d847f47dad887835a7453248a0b22d88b61d1d6a8b06a863f423e9195d`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-9-0-4-headers-json` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.0.4/headers.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.0.4/headers.json). SHA-256 `3cf07357f506e099a3ee77bf258b2d9ee8ce7997b31fd8661c6b3d3826502c59`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-9-0-4-headers-stdout` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.0.4/headers.stdout](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.0.4/headers.stdout). SHA-256 `81110220f7a52810d47cb5f1c4cb272c22403b437f38a6a81da3e50bddc20c66`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-9-0-4-headers-stderr` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.0.4/headers.stderr](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.0.4/headers.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-9-0-4-compile-json` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.0.4/compile.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.0.4/compile.json). SHA-256 `f4fd66356769389bea5332a9720ac85e3c3ed8924691934fab3b6c0c886a1023`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-9-0-4-compile-stdout` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.0.4/compile.stdout](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.0.4/compile.stdout). SHA-256 `b31c8cbdbcb06965e3e72d66fbb4ae7d8df1641bb626b7d87a2da8da113f17e8`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-9-0-4-compile-stderr` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.0.4/compile.stderr](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.0.4/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-9-0-4-cli-version-json` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.0.4/cli-version.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.0.4/cli-version.json). SHA-256 `0c07bae4f21b3e7efe17ba4bead7c046dd8a3c3cc78ae6a2fef9576d82aaab68`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-9-0-4-cli-version-stdout` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.0.4/cli-version.stdout](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.0.4/cli-version.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-9-0-4-cli-version-stderr` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.0.4/cli-version.stderr](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.0.4/cli-version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-9-0-4-probe-json` (observation): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.0.4/probe.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.0.4/probe.json). SHA-256 `5884cd05a1326b87993fdea805e91f7d9d4a5190c3c076aa501411c07f5eed1c`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-9-0-4-probe-stdout` (observation): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.0.4/probe.stdout](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.0.4/probe.stdout). SHA-256 `4b977632f4c7591470acca074907b896f800124b1672b353a9eee5fae73e949b`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-9-0-4-probe-stderr` (observation): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.0.4/probe.stderr](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.0.4/probe.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-9-0-4-inputs-before-json` (input): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.0.4/inputs-before.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.0.4/inputs-before.json). SHA-256 `71aa36714635130f73b71f9c153f9c8680ccd89c160994f84f8c6fcc344b7cd0`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-9-0-4-inputs-after-json` (input): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.0.4/inputs-after.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.0.4/inputs-after.json). SHA-256 `71aa36714635130f73b71f9c153f9c8680ccd89c160994f84f8c6fcc344b7cd0`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-9-1-0-receipt-json` (observation): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.1.0/receipt.json). SHA-256 `2a5c049dc35ebcc2dbcfc82d86b5dbcaa0e8ec7976c804ef9a2e965204d09ea0`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-9-1-0-headers-json` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.1.0/headers.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.1.0/headers.json). SHA-256 `1591a9940cdb7be4a871a716ded934cc6304c09ba1803bdea981c4e44b1458c8`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-9-1-0-headers-stdout` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.1.0/headers.stdout](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.1.0/headers.stdout). SHA-256 `45ace7134343c6d349a1134f730e2acad83c98bcc05bfc3ad38e549699791db0`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-9-1-0-headers-stderr` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.1.0/headers.stderr](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.1.0/headers.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-9-1-0-compile-json` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.1.0/compile.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.1.0/compile.json). SHA-256 `94023ddb5cd5055cd826c4c4fcd892eac0a9f9c4dc3a6cc3781a9f86db740318`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-9-1-0-compile-stdout` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.1.0/compile.stdout](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.1.0/compile.stdout). SHA-256 `abe9fbbbaef0279c8c477062e80ce94de0dc55011356ac5b217466cf094af227`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-9-1-0-compile-stderr` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.1.0/compile.stderr](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.1.0/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-9-1-0-cli-version-json` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.1.0/cli-version.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.1.0/cli-version.json). SHA-256 `d2769cca2790447a626b3af8665e1cfe68344ce33229cc53b9784885064a6c05`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-9-1-0-cli-version-stdout` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.1.0/cli-version.stdout](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.1.0/cli-version.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-9-1-0-cli-version-stderr` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.1.0/cli-version.stderr](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.1.0/cli-version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-9-1-0-probe-json` (observation): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.1.0/probe.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.1.0/probe.json). SHA-256 `de07f2e56c4061c17bf3bcf61fb159020ae0dab76a5e07b6c4ec4aa94e9445a4`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-9-1-0-probe-stdout` (observation): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.1.0/probe.stdout](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.1.0/probe.stdout). SHA-256 `c17d7430e15eb362d3d10778d91778665711d1fe678f248f28af5be7eeb8ad5c`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-9-1-0-probe-stderr` (observation): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.1.0/probe.stderr](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.1.0/probe.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-9-1-0-inputs-before-json` (input): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.1.0/inputs-before.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.1.0/inputs-before.json). SHA-256 `71aa36714635130f73b71f9c153f9c8680ccd89c160994f84f8c6fcc344b7cd0`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-9-1-0-inputs-after-json` (input): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.1.0/inputs-after.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.1.0/inputs-after.json). SHA-256 `71aa36714635130f73b71f9c153f9c8680ccd89c160994f84f8c6fcc344b7cd0`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-jim-receipt-json` (observation): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim/receipt.json). SHA-256 `0055e99f2be407638c5597fddb828fd8e3cb7bd71d7d8d3bc27aaa92b0d6cafd`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-jim-headers-json` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim/headers.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim/headers.json). SHA-256 `97dc59d81d55a8cf32cf669bdbf1d873940edf5da84530a757acac99e8e036c1`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-jim-headers-stdout` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim/headers.stdout](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim/headers.stdout). SHA-256 `f31a2303a53c3611d6b7eb4443ec54d83978ba68bdd825ddcac1e038fb16efb6`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-jim-headers-stderr` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim/headers.stderr](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim/headers.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-jim-compile-json` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim/compile.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim/compile.json). SHA-256 `82873a7d95278ac2f5e846a76095dcef5942e2d4769999f11fed076f1e584535`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-jim-compile-stdout` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim/compile.stdout](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim/compile.stdout). SHA-256 `ad25e25faef8a0437d52aad15907e62ca422e85f830267c21384b4c93be2dfef`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-jim-compile-stderr` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim/compile.stderr](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-jim-cli-version-json` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim/cli-version.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim/cli-version.json). SHA-256 `b3ec73aa6453ec97b3ff8bb3a19d9efab9f9b8c33c2c60d789006076dae6499c`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-jim-cli-version-stdout` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim/cli-version.stdout](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim/cli-version.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-jim-cli-version-stderr` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim/cli-version.stderr](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim/cli-version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-jim-probe-json` (observation): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim/probe.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim/probe.json). SHA-256 `5a921b29d9aaa7aa649d09257d32ef33bf87adcf317375d1b5d6221c81c3119a`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-jim-probe-stdout` (observation): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim/probe.stdout](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim/probe.stdout). SHA-256 `c1608cccab519410e8c307547744ecfd31b4686625e81c7c956f3fd94c8ec2dc`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-jim-probe-stderr` (observation): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim/probe.stderr](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim/probe.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-jim-inputs-before-json` (input): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim/inputs-before.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim/inputs-before.json). SHA-256 `b21ef98881dabc82fd7b681ec60c5684d4a391aec9c08350fee1f839fdf41ade`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-jim-inputs-after-json` (input): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim/inputs-after.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim/inputs-after.json). SHA-256 `b21ef98881dabc82fd7b681ec60c5684d4a391aec9c08350fee1f839fdf41ade`. Complete original provider process receipt or raw stream; includes argv, explicit environment, status and actual whole hashes. CLI query output is genuinely empty.
- `active671-EXACT-OBSERVATIONS-json` (observation): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/EXACT-OBSERVATIONS.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/EXACT-OBSERVATIONS.json). SHA-256 `9ebbd1e5465058a7fbb41ea934fb3bdd301ca160fdbffe6be742f1e31a59cba0`. Lossless reconstruction of all138 original event/result rows, per-provider raw-version rows and exact result hex, independently joined to original streams and receipts.
- `active671-INPUT-RETENTION-JOINS-json` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/INPUT-RETENTION-JOINS.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/INPUT-RETENTION-JOINS.json). SHA-256 `d49da3cf0d08dc9030b8c39add28eda61bbca4b7df05ec60a7fcee93e170c3bc`. Exact original input hashes and authenticated existing SDK/header/library/CLI/source restoration mappings; separate explicit external-system hash-only limitations.
- `active671-source-windows-tcl8-4-Tcl_DeleteCommandFromToken-2550-2696-txt` (source-anchor): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/source-windows/tcl8.4-Tcl_DeleteCommandFromToken-2550-2696.txt](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/source-windows/tcl8.4-Tcl_DeleteCommandFromToken-2550-2696.txt). SHA-256 `476e1f08a2b8f5dce752e771dd84bf25463be35b7c797f5c5fc48c15628ca9b0`. Complete physical-LF implementation window with terminal LF and original whitespace preserved; source does not enlarge the measured callback purposes.
- `active671-whole-source-tcl8.4` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclBasic.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclBasic.c). SHA-256 `cab6d4decd1a3e365fe3cef49a2d5430dfef01cd775dd7d34f98064032f3a81a`. Whole original measured-provider source containing the physically counted implementation window; independently unchanged original bytes.
- `active671-source-windows-tcl8-5-Tcl_DeleteCommandFromToken-2774-2939-txt` (source-anchor): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/source-windows/tcl8.5-Tcl_DeleteCommandFromToken-2774-2939.txt](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/source-windows/tcl8.5-Tcl_DeleteCommandFromToken-2774-2939.txt). SHA-256 `90cae98331d5db4913ab7b71bdd42bbcba8cef11c576bdd305f405955de3a368`. Complete physical-LF implementation window with terminal LF and original whitespace preserved; source does not enlarge the measured callback purposes.
- `active671-whole-source-tcl8.5` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/generic/tclBasic.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/generic/tclBasic.c). SHA-256 `d09c16386ea0376dc3590a74ded2c6e2bcef5079f2b1a3785545bad37c7ead63`. Whole original measured-provider source containing the physically counted implementation window; independently unchanged original bytes.
- `active671-source-windows-tcl8-6-Tcl_DeleteCommandFromToken-3147-3320-txt` (source-anchor): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/source-windows/tcl8.6-Tcl_DeleteCommandFromToken-3147-3320.txt](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/source-windows/tcl8.6-Tcl_DeleteCommandFromToken-3147-3320.txt). SHA-256 `b639cfb45d73ae19e8369a59b15e52062b016b531bbbb98a93afe34258a32e06`. Complete physical-LF implementation window with terminal LF and original whitespace preserved; source does not enlarge the measured callback purposes.
- `active671-whole-source-tcl8.6` (source-anchor): [rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/8.6.18/tclBasic.c](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/8.6.18/tclBasic.c). SHA-256 `19e28c2c9fbcd27e7e538a06960f0d806d04f8c25d94499cf9eb9031f8440198`. Whole original measured-provider source containing the physically counted implementation window; independently unchanged original bytes.
- `active671-source-windows-tcl9-0-Tcl_DeleteCommandFromToken-3727-3902-txt` (source-anchor): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/source-windows/tcl9.0-Tcl_DeleteCommandFromToken-3727-3902.txt](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/source-windows/tcl9.0-Tcl_DeleteCommandFromToken-3727-3902.txt). SHA-256 `41a0ff277f6b4a974f3799fd974eea75fe9184520fbca9378f82ca289541b382`. Complete physical-LF implementation window with terminal LF and original whitespace preserved; source does not enlarge the measured callback purposes.
- `active671-whole-source-tcl9.0` (source-anchor): [rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/9.0.4/tclBasic.c](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/9.0.4/tclBasic.c). SHA-256 `6d995c1af94fea92717183882ca41775fe8c0e088def163f6ac8fe699cbffc67`. Whole original measured-provider source containing the physically counted implementation window; independently unchanged original bytes.
- `active671-source-windows-tcl9-1-Tcl_DeleteCommandFromToken-3697-3872-txt` (source-anchor): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/source-windows/tcl9.1-Tcl_DeleteCommandFromToken-3697-3872.txt](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/source-windows/tcl9.1-Tcl_DeleteCommandFromToken-3697-3872.txt). SHA-256 `e1c5f7b00f1d8399aca5eca6bd84fa489448d10320f4827e1c58bd330b97da1a`. Complete physical-LF implementation window with terminal LF and original whitespace preserved; source does not enlarge the measured callback purposes.
- `active671-whole-source-tcl9.1` (source-anchor): [rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/9.1.0/tclBasic.c](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/9.1.0/tclBasic.c). SHA-256 `5f52206bed251629baedd8ef8cc0f7ae1237bf9a7edc7d4bca76d3932f8870d2`. Whole original measured-provider source containing the physically counted implementation window; independently unchanged original bytes.
- `active671-source-windows-jim-JimIncrCmdRefCount-3881-3884-txt` (source-anchor): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/source-windows/jim-JimIncrCmdRefCount-3881-3884.txt](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/source-windows/jim-JimIncrCmdRefCount-3881-3884.txt). SHA-256 `84004aa829ae81eb24e6c27f33f7122d423135f7cf9df520840daf904a28f331`. Complete physical-LF implementation window with terminal LF and original whitespace preserved; source does not enlarge the measured callback purposes.
- `active671-whole-source-jim` (source-anchor): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/request/jim.c](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/request/jim.c). SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`. Whole original measured-provider source containing the physically counted implementation window; independently unchanged original bytes.
- `active671-source-windows-jim-JimDecrCmdRefCount-3887-3922-txt` (source-anchor): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/source-windows/jim-JimDecrCmdRefCount-3887-3922.txt](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/source-windows/jim-JimDecrCmdRefCount-3887-3922.txt). SHA-256 `b4469d6dbcfcb67df151f4cd74b18204e183f9d58f8c7de0dcc708fd1320bbd0`. Complete physical-LF implementation window with terminal LF and original whitespace preserved; source does not enlarge the measured callback purposes.
- `active671-source-windows-jim-Jim_DeleteCommand-4447-4460-txt` (source-anchor): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/source-windows/jim-Jim_DeleteCommand-4447-4460.txt](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/source-windows/jim-Jim_DeleteCommand-4447-4460.txt). SHA-256 `2e1cb096fe5693cc82d34496f1626c2211372fe716777bf88c71c3127ce973e8`. Complete physical-LF implementation window with terminal LF and original whitespace preserved; source does not enlarge the measured callback purposes.
- `active671-source-windows-jim-JimInvokeCommand-11101-11215-txt` (source-anchor): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/source-windows/jim-JimInvokeCommand-11101-11215.txt](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/source-windows/jim-JimInvokeCommand-11101-11215.txt). SHA-256 `4700e8a5500db11ee1288c0000dca1240c5ecd8da248a1f1a186b7dad28702fe`. Complete physical-LF implementation window with terminal LF and original whitespace preserved; source does not enlarge the measured callback purposes.
- `active671-SOURCE-JOINS-json` (source-anchor): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/SOURCE-JOINS.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/SOURCE-JOINS.json). SHA-256 `555d7069955ae0bbb235488b56c0339a59c31629a13586f6628d48190ffdea52`. Nine complete physical-LF source joins: five C deletion implementations and Jim acquisition, release, deletion and invocation. Exact source bytes do not certify unmeasured purposes.
- `active671-elf-8-4-20` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.4.20/probe.elf.gz](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.4.20/probe.elf.gz). SHA-256 `0030435de1cedbd1381bac871cc4a68df4664a55c482e4d22740dfa03ebd6cf0`. Lossless retained linked probe ELF; full decompression equals the original executable SHA 43acfd89fa1a5a9cfe5533b237347d80269fe4a33094e694218ea599c8530c56. No new execution is implied.
- `active671-elf-8-5-19` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.5.19/probe.elf.gz](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.5.19/probe.elf.gz). SHA-256 `6b4630b8faa6d8b91f19222a55cefbe1a8c4669b727a3cbfc7fc2e0cd7937ca2`. Lossless retained linked probe ELF; full decompression equals the original executable SHA 6d3b1a303f6b7af7d71fb188b7410a43f4f2a50ee58139f74555a4e31dcb2fce. No new execution is implied.
- `active671-elf-8-6-18` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.6.18/probe.elf.gz](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/8.6.18/probe.elf.gz). SHA-256 `ea84bb346141f42729c6a36d210ea9869f83b212df6aa489b68c70df77bc51cc`. Lossless retained linked probe ELF; full decompression equals the original executable SHA 2be2675e3c60f77dc16c648f3bc14134ac69eeaa9dd3c901ce63b07153a99316. No new execution is implied.
- `active671-elf-9-0-4` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.0.4/probe.elf.gz](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.0.4/probe.elf.gz). SHA-256 `9b4acf551f83018e238b6196d642004753770ed1bfd1f16f2e4012b3908eb435`. Lossless retained linked probe ELF; full decompression equals the original executable SHA c9de9302a293fe72275df01c01bcf42ece429c8d56d9382ddff4c9c5ce904574. No new execution is implied.
- `active671-elf-9-1-0` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.1.0/probe.elf.gz](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/9.1.0/probe.elf.gz). SHA-256 `0400423a1df1e0d7e84aaabd3893ed908c8ef3d20bcff64de1456ac5daf60467`. Lossless retained linked probe ELF; full decompression equals the original executable SHA ab4a5f20aa19ecdafabb90b3af0b5a3397642b9fb583419abd06d2d225545f05. No new execution is implied.
- `active671-elf-jim` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim/probe.elf.gz](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/jim/probe.elf.gz). SHA-256 `5863dff022e24dda10223fa648fc797187370056e24048aa92e567e1f2e0555d`. Lossless retained linked probe ELF; full decompression equals the original executable SHA 32feb36adbbd51460b7f1c4193555b81e93f0ce13d577d6dc7909069685e95f1. No new execution is implied.
- `active671-elf-retention` (provider): [rust/tcl-registry/tests/data/native_active_object_command_deletion/671/ELF-RETENTION.json](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671/ELF-RETENTION.json). SHA-256 `9c8e467b52fc52eeaf7b19d5574770d0f8b6560c70b4a86679a8b5ac3445843f`. Six exact compressed/uncompressed linked-probe byte identities and actual complete decompression checks.

## Source inspection

tcl8.4 8.4.20, revision `Exact release source and whole hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclBasic.c`, function `Tcl_DeleteCommandFromToken`, lines 2550–2696. Line unit: physical LF only. Original byte range [78942, 84048). Full-source SHA-256 `cab6d4decd1a3e365fe3cef49a2d5430dfef01cd775dd7d34f98064032f3a81a`; snippet SHA-256 `476e1f08a2b8f5dce752e771dd84bf25463be35b7c797f5c5fc48c15628ca9b0`; retained evidence `active671-whole-source-tcl8.4`.

```text
Tcl_DeleteCommandFromToken(interp, cmd)
    Tcl_Interp *interp;		/* Token for command interpreter returned by
				 * a previous call to Tcl_CreateInterp. */
    Tcl_Command cmd;            /* Token for command to delete. */
{
    Interp *iPtr = (Interp *) interp;
    Command *cmdPtr = (Command *) cmd;
    ImportRef *refPtr, *nextRefPtr;
    Tcl_Command importCmd;

    /*
     * The code here is tricky.  We can't delete the hash table entry
     * before invoking the deletion callback because there are cases
     * where the deletion callback needs to invoke the command (e.g.
     * object systems such as OTcl). However, this means that the
     * callback could try to delete or rename the command. The deleted
     * flag allows us to detect these cases and skip nested deletes.
     */

    if (cmdPtr->flags & CMD_IS_DELETED) {
	/*
	 * Another deletion is already in progress.  Remove the hash
	 * table entry now, but don't invoke a callback or free the
	 * command structure.
	 */

        Tcl_DeleteHashEntry(cmdPtr->hPtr);
	cmdPtr->hPtr = NULL;
	return 0;
    }

    /* 
     * We must delete this command, even though both traces and
     * delete procs may try to avoid this (renaming the command etc).
     * Also traces and delete procs may try to delete the command
     * themsevles.  This flag declares that a delete is in progress
     * and that recursive deletes should be ignored.
     */
    cmdPtr->flags |= CMD_IS_DELETED;

    /*
     * Bump the command epoch counter. This will invalidate all cached
     * references that point to this command.
     */
    
    cmdPtr->cmdEpoch++;

    /*
     * Call trace procedures for the command being deleted. Then delete
     * its traces. 
     */

    if (cmdPtr->tracePtr != NULL) {
	CommandTrace *tracePtr;
	CallCommandTraces(iPtr,cmdPtr,NULL,NULL,TCL_TRACE_DELETE);
	/* Now delete these traces */
	tracePtr = cmdPtr->tracePtr;
	while (tracePtr != NULL) {
	    CommandTrace *nextPtr = tracePtr->nextPtr;
	    if ((--tracePtr->refCount) <= 0) {
		ckfree((char*)tracePtr);
	    }
	    tracePtr = nextPtr;
	}
	cmdPtr->tracePtr = NULL;
    }
    
    /*
     * If the command being deleted has a compile procedure, increment the
     * interpreter's compileEpoch to invalidate its compiled code. This
     * makes sure that we don't later try to execute old code compiled with
     * command-specific (i.e., inline) bytecodes for the now-deleted
     * command. This field is checked in Tcl_EvalObj and ObjInterpProc, and
     * code whose compilation epoch doesn't match is recompiled.
     */

    if (cmdPtr->compileProc != NULL) {
        iPtr->compileEpoch++;
    }

    if (cmdPtr->deleteProc != NULL) {
	/*
	 * Delete the command's client data. If this was an imported command
	 * created when a command was imported into a namespace, this client
	 * data will be a pointer to a ImportedCmdData structure describing
	 * the "real" command that this imported command refers to.
	 */
	
	/*
	 * If you are getting a crash during the call to deleteProc and
	 * cmdPtr->deleteProc is a pointer to the function free(), the
	 * most likely cause is that your extension allocated memory
	 * for the clientData argument to Tcl_CreateObjCommand() with
	 * the ckalloc() macro and you are now trying to deallocate
	 * this memory with free() instead of ckfree(). You should
	 * pass a pointer to your own method that calls ckfree().
	 */

	(*cmdPtr->deleteProc)(cmdPtr->deleteData);
    }

    /*
     * If this command was imported into other namespaces, then imported
     * commands were created that refer back to this command. Delete these
     * imported commands now.
     */

    for (refPtr = cmdPtr->importRefPtr;  refPtr != NULL;
            refPtr = nextRefPtr) {
	nextRefPtr = refPtr->nextPtr;
	importCmd = (Tcl_Command) refPtr->importedCmdPtr;
        Tcl_DeleteCommandFromToken(interp, importCmd);
    }

    /*
     * Don't use hPtr to delete the hash entry here, because it's
     * possible that the deletion callback renamed the command.
     * Instead, use cmdPtr->hptr, and make sure that no-one else
     * has already deleted the hash entry.
     */

    if (cmdPtr->hPtr != NULL) {
	Tcl_DeleteHashEntry(cmdPtr->hPtr);
    }

    /*
     * Mark the Command structure as no longer valid. This allows
     * TclExecuteByteCode to recognize when a Command has logically been
     * deleted and a pointer to this Command structure cached in a CmdName
     * object is invalid. TclExecuteByteCode will look up the command again
     * in the interpreter's command hashtable.
     */

    cmdPtr->objProc = NULL;

    /*
     * Now free the Command structure, unless there is another reference to
     * it from a CmdName Tcl object in some ByteCode code sequence. In that
     * case, delay the cleanup until all references are either discarded
     * (when a ByteCode is freed) or replaced by a new reference (when a
     * cached CmdName Command reference is found to be invalid and
     * TclExecuteByteCode looks up the command in the command hashtable).
     */
    
    TclCleanupCommand(cmdPtr);
    return 0;
}

```

tcl8.5 8.5.19, revision `Exact release source and whole hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclBasic.c`, function `Tcl_DeleteCommandFromToken`, lines 2774–2939. Line unit: physical LF only. Original byte range [84225, 89835). Full-source SHA-256 `d09c16386ea0376dc3590a74ded2c6e2bcef5079f2b1a3785545bad37c7ead63`; snippet SHA-256 `90cae98331d5db4913ab7b71bdd42bbcba8cef11c576bdd305f405955de3a368`; retained evidence `active671-whole-source-tcl8.5`.

```text
Tcl_DeleteCommandFromToken(
    Tcl_Interp *interp,		/* Token for command interpreter returned by a
				 * previous call to Tcl_CreateInterp. */
    Tcl_Command cmd)		/* Token for command to delete. */
{
    Interp *iPtr = (Interp *) interp;
    Command *cmdPtr = (Command *) cmd;
    ImportRef *refPtr, *nextRefPtr;
    Tcl_Command importCmd;

    /*
     * Bump the command epoch counter. This will invalidate all cached
     * references that point to this command.
     */

    cmdPtr->cmdEpoch++;

    /*
     * The code here is tricky. We can't delete the hash table entry before
     * invoking the deletion callback because there are cases where the
     * deletion callback needs to invoke the command (e.g. object systems such
     * as OTcl). However, this means that the callback could try to delete or
     * rename the command. The deleted flag allows us to detect these cases
     * and skip nested deletes.
     */

    if (cmdPtr->flags & CMD_IS_DELETED) {
	/*
	 * Another deletion is already in progress. Remove the hash table
	 * entry now, but don't invoke a callback or free the command
	 * structure. Take care to only remove the hash entry if it has not
	 * already been removed; otherwise if we manage to hit this function
	 * three times, everything goes up in smoke. [Bug 1220058]
	 */

	if (cmdPtr->hPtr != NULL) {
	    Tcl_DeleteHashEntry(cmdPtr->hPtr);
	    cmdPtr->hPtr = NULL;
	}
	return 0;
    }

    /*
     * We must delete this command, even though both traces and delete procs
     * may try to avoid this (renaming the command etc). Also traces and
     * delete procs may try to delete the command themsevles. This flag
     * declares that a delete is in progress and that recursive deletes should
     * be ignored.
     */

    cmdPtr->flags |= CMD_IS_DELETED;

    /*
     * Call trace functions for the command being deleted. Then delete its
     * traces.
     */

    if (cmdPtr->tracePtr != NULL) {
	CommandTrace *tracePtr;
	CallCommandTraces(iPtr,cmdPtr,NULL,NULL,TCL_TRACE_DELETE);

	/*
	 * Now delete these traces.
	 */

	tracePtr = cmdPtr->tracePtr;
	while (tracePtr != NULL) {
	    CommandTrace *nextPtr = tracePtr->nextPtr;
	    if ((--tracePtr->refCount) <= 0) {
		ckfree((char *) tracePtr);
	    }
	    tracePtr = nextPtr;
	}
	cmdPtr->tracePtr = NULL;
    }

    /*
     * The list of command exported from the namespace might have changed.
     * However, we do not need to recompute this just yet; next time we need
     * the info will be soon enough.
     */

    TclInvalidateNsCmdLookup(cmdPtr->nsPtr);

    /*
     * If the command being deleted has a compile function, increment the
     * interpreter's compileEpoch to invalidate its compiled code. This makes
     * sure that we don't later try to execute old code compiled with
     * command-specific (i.e., inline) bytecodes for the now-deleted command.
     * This field is checked in Tcl_EvalObj and ObjInterpProc, and code whose
     * compilation epoch doesn't match is recompiled.
     */

    if (cmdPtr->compileProc != NULL) {
	iPtr->compileEpoch++;
    }

    if (cmdPtr->deleteProc != NULL) {
	/*
	 * Delete the command's client data. If this was an imported command
	 * created when a command was imported into a namespace, this client
	 * data will be a pointer to a ImportedCmdData structure describing
	 * the "real" command that this imported command refers to.
	 */

	/*
	 * If you are getting a crash during the call to deleteProc and
	 * cmdPtr->deleteProc is a pointer to the function free(), the most
	 * likely cause is that your extension allocated memory for the
	 * clientData argument to Tcl_CreateObjCommand() with the ckalloc()
	 * macro and you are now trying to deallocate this memory with free()
	 * instead of ckfree(). You should pass a pointer to your own method
	 * that calls ckfree().
	 */

	(*cmdPtr->deleteProc)(cmdPtr->deleteData);
    }

    /*
     * If this command was imported into other namespaces, then imported
     * commands were created that refer back to this command. Delete these
     * imported commands now.
     */
    if (!(cmdPtr->flags & CMD_REDEF_IN_PROGRESS)) {
	for (refPtr = cmdPtr->importRefPtr;  refPtr != NULL;
		refPtr = nextRefPtr) {
	    nextRefPtr = refPtr->nextPtr;
	    importCmd = (Tcl_Command) refPtr->importedCmdPtr;
	    Tcl_DeleteCommandFromToken(interp, importCmd);
	}
    }

    /*
     * Don't use hPtr to delete the hash entry here, because it's possible
     * that the deletion callback renamed the command. Instead, use
     * cmdPtr->hptr, and make sure that no-one else has already deleted the
     * hash entry.
     */

    if (cmdPtr->hPtr != NULL) {
	Tcl_DeleteHashEntry(cmdPtr->hPtr);
	cmdPtr->hPtr = NULL;
    }

    /*
     * Mark the Command structure as no longer valid. This allows
     * TclExecuteByteCode to recognize when a Command has logically been
     * deleted and a pointer to this Command structure cached in a CmdName
     * object is invalid. TclExecuteByteCode will look up the command again in
     * the interpreter's command hashtable.
     */

    cmdPtr->objProc = NULL;

    /*
     * Now free the Command structure, unless there is another reference to it
     * from a CmdName Tcl object in some ByteCode code sequence. In that case,
     * delay the cleanup until all references are either discarded (when a
     * ByteCode is freed) or replaced by a new reference (when a cached
     * CmdName Command reference is found to be invalid and TclExecuteByteCode
     * looks up the command in the command hashtable).
     */

    TclCleanupCommandMacro(cmdPtr);
    return 0;
}

```

tcl8.6 8.6.18, revision `Exact release source and whole hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclBasic.c`, function `Tcl_DeleteCommandFromToken`, lines 3147–3320. Line unit: physical LF only. Original byte range [98088, 103755). Full-source SHA-256 `19e28c2c9fbcd27e7e538a06960f0d806d04f8c25d94499cf9eb9031f8440198`; snippet SHA-256 `b639cfb45d73ae19e8369a59b15e52062b016b531bbbb98a93afe34258a32e06`; retained evidence `active671-whole-source-tcl8.6`.

```text
Tcl_DeleteCommandFromToken(
    Tcl_Interp *interp,		/* Token for command interpreter returned by a
				 * previous call to Tcl_CreateInterp. */
    Tcl_Command cmd)		/* Token for command to delete. */
{
    Interp *iPtr = (Interp *) interp;
    Command *cmdPtr = (Command *) cmd;
    ImportRef *refPtr, *nextRefPtr;
    Tcl_Command importCmd;

    /*
     * Bump the command epoch counter. This will invalidate all cached
     * references that point to this command.
     */

    cmdPtr->cmdEpoch++;

    /*
     * The code here is tricky. We can't delete the hash table entry before
     * invoking the deletion callback because there are cases where the
     * deletion callback needs to invoke the command (e.g. object systems such
     * as OTcl). However, this means that the callback could try to delete or
     * rename the command. The deleted flag allows us to detect these cases
     * and skip nested deletes.
     */

    if (cmdPtr->flags & CMD_IS_DELETED) {
	/*
	 * Another deletion is already in progress. Remove the hash table
	 * entry now, but don't invoke a callback or free the command
	 * structure. Take care to only remove the hash entry if it has not
	 * already been removed; otherwise if we manage to hit this function
	 * three times, everything goes up in smoke. [Bug 1220058]
	 */

	if (cmdPtr->hPtr != NULL) {
	    Tcl_DeleteHashEntry(cmdPtr->hPtr);
	    cmdPtr->hPtr = NULL;
	}
	return 0;
    }

    /*
     * We must delete this command, even though both traces and delete procs
     * may try to avoid this (renaming the command etc). Also traces and
     * delete procs may try to delete the command themselves. This flag
     * declares that a delete is in progress and that recursive deletes should
     * be ignored.
     */

    cmdPtr->flags |= CMD_IS_DELETED;

    /*
     * Call trace functions for the command being deleted. Then delete its
     * traces.
     */

    cmdPtr->nsPtr->refCount++;

    if (cmdPtr->tracePtr != NULL) {
	CommandTrace *tracePtr;
	CallCommandTraces(iPtr,cmdPtr,NULL,NULL,TCL_TRACE_DELETE);

	/*
	 * Now delete these traces.
	 */

	tracePtr = cmdPtr->tracePtr;
	while (tracePtr != NULL) {
	    CommandTrace *nextPtr = tracePtr->nextPtr;

	    if (tracePtr->refCount-- <= 1) {
		ckfree(tracePtr);
	    }
	    tracePtr = nextPtr;
	}
	cmdPtr->tracePtr = NULL;
    }

    /*
     * The list of commands exported from the namespace might have changed.
     * However, we do not need to recompute this just yet; next time we need
     * the info will be soon enough.
     */

    TclInvalidateNsCmdLookup(cmdPtr->nsPtr);
    TclNsDecrRefCount(cmdPtr->nsPtr);

    /*
     * If the command being deleted has a compile function, increment the
     * interpreter's compileEpoch to invalidate its compiled code. This makes
     * sure that we don't later try to execute old code compiled with
     * command-specific (i.e., inline) bytecodes for the now-deleted command.
     * This field is checked in Tcl_EvalObj and ObjInterpProc, and code whose
     * compilation epoch doesn't match is recompiled.
     */

    if (cmdPtr->compileProc != NULL) {
	iPtr->compileEpoch++;
    }

    if (!(cmdPtr->flags & CMD_REDEF_IN_PROGRESS)) {
	/*
	 * Delete any imports of this routine before deleting this routine itself.
	 * See issue 688fcc7082fa.
	 */
	for (refPtr = cmdPtr->importRefPtr; refPtr != NULL;
		refPtr = nextRefPtr) {
	    nextRefPtr = refPtr->nextPtr;
	    importCmd = (Tcl_Command) refPtr->importedCmdPtr;
	    Tcl_DeleteCommandFromToken(interp, importCmd);
	}
    }

    if (cmdPtr->deleteProc != NULL) {
	/*
	 * Delete the command's client data. If this was an imported command
	 * created when a command was imported into a namespace, this client
	 * data will be a pointer to a ImportedCmdData structure describing
	 * the "real" command that this imported command refers to.
	 *
	 * If you are getting a crash during the call to deleteProc and
	 * cmdPtr->deleteProc is a pointer to the function free(), the most
	 * likely cause is that your extension allocated memory for the
	 * clientData argument to Tcl_CreateObjCommand with the ckalloc()
	 * macro and you are now trying to deallocate this memory with free()
	 * instead of ckfree(). You should pass a pointer to your own method
	 * that calls ckfree().
	 */

	cmdPtr->deleteProc(cmdPtr->deleteData);
    }

    /*
     * Don't use hPtr to delete the hash entry here, because it's possible
     * that the deletion callback renamed the command. Instead, use
     * cmdPtr->hptr, and make sure that no-one else has already deleted the
     * hash entry.
     */

    if (cmdPtr->hPtr != NULL) {
	Tcl_DeleteHashEntry(cmdPtr->hPtr);
	cmdPtr->hPtr = NULL;

	/*
	 * Bump the command epoch counter. This will invalidate all cached
	 * references that point to this command.
	 */

	cmdPtr->cmdEpoch++;
    }

    /*
     * A number of tests for particular kinds of commands are done by checking
     * whether the objProc field holds a known value. Set the field to NULL so
     * that such tests won't have false positives when applied to deleted
     * commands.
     */

    cmdPtr->objProc = NULL;

    /*
     * Now free the Command structure, unless there is another reference to it
     * from a CmdName Tcl object in some ByteCode code sequence. In that case,
     * delay the cleanup until all references are either discarded (when a
     * ByteCode is freed) or replaced by a new reference (when a cached
     * CmdName Command reference is found to be invalid and
     * TclNRExecuteByteCode looks up the command in the command hashtable).
     */

    cmdPtr->flags |= CMD_DEAD;
    TclCleanupCommandMacro(cmdPtr);
    return 0;
}

```

tcl9.0 9.0.4, revision `Exact release source and whole hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclBasic.c`, function `Tcl_DeleteCommandFromToken`, lines 3727–3902. Line unit: physical LF only. Original byte range [115770, 121494). Full-source SHA-256 `6d995c1af94fea92717183882ca41775fe8c0e088def163f6ac8fe699cbffc67`; snippet SHA-256 `41a0ff277f6b4a974f3799fd974eea75fe9184520fbca9378f82ca289541b382`; retained evidence `active671-whole-source-tcl9.0`.

```text
Tcl_DeleteCommandFromToken(
    Tcl_Interp *interp,		/* Token for command interpreter returned by a
				 * previous call to Tcl_CreateInterp. */
    Tcl_Command cmd)		/* Token for command to delete. */
{
    Interp *iPtr = (Interp *) interp;
    Command *cmdPtr = (Command *) cmd;
    ImportRef *refPtr, *nextRefPtr;
    Tcl_Command importCmd;

    /*
     * The code here is tricky. We can't delete the hash table entry before
     * invoking the deletion callback because there are cases where the
     * deletion callback needs to invoke the command (e.g. object systems such
     * as OTcl). However, this means that the callback could try to delete or
     * rename the command. The deleted flag allows us to detect these cases
     * and skip nested deletes.
     */

    if (cmdPtr->flags & CMD_DYING) {
	/*
	 * Another deletion is already in progress. Remove the hash table
	 * entry now, but don't invoke a callback or free the command
	 * structure. Take care to only remove the hash entry if it has not
	 * already been removed; otherwise if we manage to hit this function
	 * three times, everything goes up in smoke. [Bug 1220058]
	 */

	if (cmdPtr->hPtr != NULL) {
	    Tcl_DeleteHashEntry(cmdPtr->hPtr);
	    cmdPtr->hPtr = NULL;
	}

	/*
	 * Bump the command epoch counter. This will invalidate all cached
	 * references that point to this command.
	 */

	cmdPtr->cmdEpoch++;

	return 0;
    }

    /*
     * We must delete this command, even though both traces and delete procs
     * may try to avoid this (renaming the command etc). Also traces and
     * delete procs may try to delete the command themselves. This flag
     * declares that a delete is in progress and that recursive deletes should
     * be ignored.
     */

    cmdPtr->flags |= CMD_DYING;

    /*
     * Call each functions and then delete the trace.
     */

    cmdPtr->nsPtr->refCount++;

    if (cmdPtr->tracePtr != NULL) {
	CommandTrace *tracePtr;
	/* CallCommandTraces() does not cmdPtr, that's
	 * done just before Tcl_DeleteCommandFromToken() returns */
	CallCommandTraces(iPtr,cmdPtr,NULL,NULL,TCL_TRACE_DELETE);

	/*
	 * Now delete these traces.
	 */

	tracePtr = cmdPtr->tracePtr;
	while (tracePtr != NULL) {
	    CommandTrace *nextPtr = tracePtr->nextPtr;

	    if (tracePtr->refCount-- <= 1) {
		Tcl_Free(tracePtr);
	    }
	    tracePtr = nextPtr;
	}
	cmdPtr->tracePtr = NULL;
    }

    /*
     * The list of commands exported from the namespace might have changed.
     * However, we do not need to recompute this just yet; next time we need
     * the info will be soon enough.
     */

    TclInvalidateNsCmdLookup(cmdPtr->nsPtr);
    TclNsDecrRefCount(cmdPtr->nsPtr);

    /*
     * If the command being deleted has a compile function, increment the
     * interpreter's compileEpoch to invalidate its compiled code. This makes
     * sure that we don't later try to execute old code compiled with
     * command-specific (i.e., inline) bytecodes for the now-deleted command.
     * This field is checked in Tcl_EvalObj and ObjInterpProc, and code whose
     * compilation epoch doesn't match is recompiled.
     */

    if (cmdPtr->compileProc != NULL) {
	iPtr->compileEpoch++;
    }

    if (!(cmdPtr->flags & CMD_REDEF_IN_PROGRESS)) {
	/*
	 * Delete any imports of this routine before deleting this routine itself.
	 * See issue 688fcc7082fa.
	 */
	for (refPtr = cmdPtr->importRefPtr; refPtr != NULL;
		refPtr = nextRefPtr) {
	    nextRefPtr = refPtr->nextPtr;
	    importCmd = (Tcl_Command) refPtr->importedCmdPtr;
	    Tcl_DeleteCommandFromToken(interp, importCmd);
	}
    }

    if (cmdPtr->deleteProc != NULL) {
	/*
	 * Delete the command's client data. If this was an imported command
	 * created when a command was imported into a namespace, this client
	 * data will be a pointer to a ImportedCmdData structure describing
	 * the "real" command that this imported command refers to.
	 *
	 * If you are getting a crash during the call to deleteProc and
	 * cmdPtr->deleteProc is a pointer to the function free(), the most
	 * likely cause is that your extension allocated memory for the
	 * clientData argument to Tcl_CreateObjCommand with the Tcl_Alloc()
	 * macro and you are now trying to deallocate this memory with free()
	 * instead of Tcl_Free(). You should pass a pointer to your own method
	 * that calls Tcl_Free().
	 */

	cmdPtr->deleteProc(cmdPtr->deleteData);
    }

    /*
     * Don't use hPtr to delete the hash entry here, because it's possible
     * that the deletion callback renamed the command. Instead, use
     * cmdPtr->hptr, and make sure that no-one else has already deleted the
     * hash entry.
     */

    if (cmdPtr->hPtr != NULL) {
	Tcl_DeleteHashEntry(cmdPtr->hPtr);
	cmdPtr->hPtr = NULL;

	/*
	 * Bump the command epoch counter. This will invalidate all cached
	 * references that point to this command.
	 */

	cmdPtr->cmdEpoch++;
    }

    /*
     * A number of tests for particular kinds of commands are done by checking
     * whether the objProc field holds a known value. Set the field to NULL so
     * that such tests won't have false positives when applied to deleted
     * commands.
     */

    cmdPtr->objProc = NULL;

    /*
     * Now free the Command structure, unless there is another reference to it
     * from a CmdName Tcl object in some ByteCode code sequence. In that case,
     * delay the cleanup until all references are either discarded (when a
     * ByteCode is freed) or replaced by a new reference (when a cached
     * CmdName Command reference is found to be invalid and
     * TclNRExecuteByteCode looks up the command in the command hashtable).
     */

    cmdPtr->flags |= CMD_DEAD;
    TclCleanupCommandMacro(cmdPtr);
    return 0;
}

```

tcl9.1 9.1.0, revision `Exact release source and whole hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBasic.c`, function `Tcl_DeleteCommandFromToken`, lines 3697–3872. Line unit: physical LF only. Original byte range [116033, 121758). Full-source SHA-256 `5f52206bed251629baedd8ef8cc0f7ae1237bf9a7edc7d4bca76d3932f8870d2`; snippet SHA-256 `e1c5f7b00f1d8399aca5eca6bd84fa489448d10320f4827e1c58bd330b97da1a`; retained evidence `active671-whole-source-tcl9.1`.

```text
Tcl_DeleteCommandFromToken(
    Tcl_Interp *interp,		/* Token for command interpreter returned by a
				 * previous call to Tcl_CreateInterp. */
    Tcl_Command cmd)		/* Token for command to delete. */
{
    Interp *iPtr = (Interp *) interp;
    Command *cmdPtr = (Command *) cmd;
    ImportRef *refPtr, *nextRefPtr;
    Tcl_Command importCmd;

    /*
     * The code here is tricky. We can't delete the hash table entry before
     * invoking the deletion callback because there are cases where the
     * deletion callback needs to invoke the command (e.g. object systems such
     * as OTcl). However, this means that the callback could try to delete or
     * rename the command. The deleted flag allows us to detect these cases
     * and skip nested deletes.
     */

    if (cmdPtr->flags & CMD_DYING) {
	/*
	 * Another deletion is already in progress. Remove the hash table
	 * entry now, but don't invoke a callback or free the command
	 * structure. Take care to only remove the hash entry if it has not
	 * already been removed; otherwise if we manage to hit this function
	 * three times, everything goes up in smoke. [Bug 1220058]
	 */

	if (cmdPtr->hPtr != NULL) {
	    Tcl_DeleteHashEntry(cmdPtr->hPtr);
	    cmdPtr->hPtr = NULL;
	}

	/*
	 * Bump the command epoch counter. This will invalidate all cached
	 * references that point to this command.
	 */

	cmdPtr->cmdEpoch++;

	return 0;
    }

    /*
     * We must delete this command, even though both traces and delete procs
     * may try to avoid this (renaming the command etc). Also traces and
     * delete procs may try to delete the command themselves. This flag
     * declares that a delete is in progress and that recursive deletes should
     * be ignored.
     */

    cmdPtr->flags |= CMD_DYING;

    /*
     * Call each functions and then delete the trace.
     */

    cmdPtr->nsPtr->refCount++;

    if (cmdPtr->tracePtr != NULL) {
	CommandTrace *tracePtr;
	/* CallCommandTraces() does not cmdPtr, that's
	 * done just before Tcl_DeleteCommandFromToken() returns */
	CallCommandTraces(iPtr,cmdPtr,NULL,NULL,TCL_TRACE_DELETE);

	/*
	 * Now delete these traces.
	 */

	tracePtr = cmdPtr->tracePtr;
	while (tracePtr != NULL) {
	    CommandTrace *nextPtr = tracePtr->nextPtr;

	    if (tracePtr->refCount-- <= 1) {
		Tcl_Free(tracePtr);
	    }
	    tracePtr = nextPtr;
	}
	cmdPtr->tracePtr = NULL;
    }

    /*
     * The list of commands exported from the namespace might have changed.
     * However, we do not need to recompute this just yet; next time we need
     * the info will be soon enough.
     */

    TclInvalidateNsCmdLookup(cmdPtr->nsPtr);
    TclNsDecrRefCount(cmdPtr->nsPtr);

    /*
     * If the command being deleted has a compile function, increment the
     * interpreter's compileEpoch to invalidate its compiled code. This makes
     * sure that we don't later try to execute old code compiled with
     * command-specific (i.e., inline) bytecodes for the now-deleted command.
     * This field is checked in Tcl_EvalObj and ObjInterpProc, and code whose
     * compilation epoch doesn't match is recompiled.
     */

    if (cmdPtr->compileProc != NULL) {
	iPtr->compileEpoch++;
    }

    if (!(cmdPtr->flags & CMD_REDEF_IN_PROGRESS)) {
	/*
	 * Delete any imports of this routine before deleting this routine itself.
	 * See issue 688fcc7082fa.
	 */
	for (refPtr = cmdPtr->importRefPtr; refPtr != NULL;
		refPtr = nextRefPtr) {
	    nextRefPtr = refPtr->nextPtr;
	    importCmd = (Tcl_Command) refPtr->importedCmdPtr;
	    Tcl_DeleteCommandFromToken(interp, importCmd);
	}
    }

    if (cmdPtr->deleteProc != NULL) {
	/*
	 * Delete the command's client data. If this was an imported command
	 * created when a command was imported into a namespace, this client
	 * data will be a pointer to a ImportedCmdData structure describing
	 * the "real" command that this imported command refers to.
	 *
	 * If you are getting a crash during the call to deleteProc and
	 * cmdPtr->deleteProc is a pointer to the function free(), the most
	 * likely cause is that your extension allocated memory for the
	 * clientData argument to Tcl_CreateObjCommand with the Tcl_Alloc()
	 * macro and you are now trying to deallocate this memory with free()
	 * instead of Tcl_Free(). You should pass a pointer to your own method
	 * that calls Tcl_Free().
	 */

	cmdPtr->deleteProc(cmdPtr->deleteData);
    }

    /*
     * Don't use hPtr to delete the hash entry here, because it's possible
     * that the deletion callback renamed the command. Instead, use
     * cmdPtr->hptr, and make sure that no-one else has already deleted the
     * hash entry.
     */

    if (cmdPtr->hPtr != NULL) {
	Tcl_DeleteHashEntry(cmdPtr->hPtr);
	cmdPtr->hPtr = NULL;

	/*
	 * Bump the command epoch counter. This will invalidate all cached
	 * references that point to this command.
	 */

	cmdPtr->cmdEpoch++;
    }

    /*
     * A number of tests for particular kinds of commands are done by checking
     * whether the objProc field holds a known value. Set the field to NULL so
     * that such tests won't have false positives when applied to deleted
     * commands.
     */

    cmdPtr->objProc2 = NULL;

    /*
     * Now free the Command structure, unless there is another reference to it
     * from a CmdName Tcl object in some ByteCode code sequence. In that case,
     * delay the cleanup until all references are either discarded (when a
     * ByteCode is freed) or replaced by a new reference (when a cached
     * CmdName Command reference is found to be invalid and
     * TclNRExecuteByteCode looks up the command in the command hashtable).
     */

    cmdPtr->flags |= CMD_DEAD;
    TclCleanupCommandMacro(cmdPtr);
    return 0;
}

```

jim 0.84-9-g5bac7c9, revision `Jim commit5bac7c9 and captured whole source hash`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `JimIncrCmdRefCount`, lines 3881–3884. Line unit: physical LF only. Original byte range [115019, 115092). Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `84004aa829ae81eb24e6c27f33f7122d423135f7cf9df520840daf904a28f331`; retained evidence `active671-whole-source-jim`.

```text
static void JimIncrCmdRefCount(Jim_Cmd *cmdPtr)
{
    cmdPtr->inUse++;
}

```

jim 0.84-9-g5bac7c9, revision `Jim commit5bac7c9 and captured whole source hash`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `JimDecrCmdRefCount`, lines 3887–3922. Line unit: physical LF only. Original byte range [115176, 116564). Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `b4469d6dbcfcb67df151f4cd74b18204e183f9d58f8c7de0dcc708fd1320bbd0`; retained evidence `active671-whole-source-jim`.

```text
static void JimDecrCmdRefCount(Jim_Interp *interp, Jim_Cmd *cmdPtr)
{
    if (--cmdPtr->inUse == 0) {
        if (cmdPtr->flags & JIM_CMD_ISPROC) {
            Jim_DecrRefCount(interp, cmdPtr->u.proc.argListObjPtr);
            Jim_DecrRefCount(interp, cmdPtr->u.proc.bodyObjPtr);
            Jim_DecrRefCount(interp, cmdPtr->u.proc.nsObj);
            if (cmdPtr->u.proc.staticVars) {
                Jim_FreeHashTable(cmdPtr->u.proc.staticVars);
                Jim_Free(cmdPtr->u.proc.staticVars);
            }
        }
        else {
            /* native (C) */
            if (cmdPtr->u.native.delProc) {
                cmdPtr->u.native.delProc(interp, cmdPtr->u.native.privData);
            }
        }
        if (cmdPtr->prevCmd) {
            /* Delete any pushed command too */
            JimDecrCmdRefCount(interp, cmdPtr->prevCmd);
        }

        /* Preserve the structure with inUse = 0 so that
         * cached references will continue to work.
         * These will be discarding at the next procEpoch increment
         * or once 1000 have been accumulated (unless quitting
         * since we can only be unwinding in that case).
         */
        cmdPtr->prevCmd = interp->oldCmdCache;
        interp->oldCmdCache = cmdPtr;
        if (!interp->quitting && ++interp->oldCmdCacheSize >= 1000) {
            Jim_InterpIncrProcEpoch(interp);
        }
    }
}

```

jim 0.84-9-g5bac7c9, revision `Jim commit5bac7c9 and captured whole source hash`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_DeleteCommand`, lines 4447–4460. Line unit: physical LF only. Original byte range [133824, 134210). Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `2e1cb096fe5693cc82d34496f1626c2211372fe716777bf88c71c3127ce973e8`; retained evidence `active671-whole-source-jim`.

```text
int Jim_DeleteCommand(Jim_Interp *interp, Jim_Obj *nameObj)
{
    int ret = JIM_OK;

    nameObj = JimQualifyName(interp, nameObj);

    if (Jim_DeleteHashEntry(&interp->commands, nameObj) == JIM_ERR) {
        Jim_SetResultFormatted(interp, "can't delete \"%#s\": command doesn't exist", nameObj);
        ret = JIM_ERR;
    }
    Jim_DecrRefCount(interp, nameObj);

    return ret;
}

```

jim 0.84-9-g5bac7c9, revision `Jim commit5bac7c9 and captured whole source hash`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `JimInvokeCommand`, lines 11101–11215. Line unit: physical LF only. Original byte range [350892, 354599). Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `4700e8a5500db11ee1288c0000dca1240c5ecd8da248a1f1a186b7dad28702fe`; retained evidence `active671-whole-source-jim`.

```text
static int JimInvokeCommand(Jim_Interp *interp, int objc, Jim_Obj *const *objv)
{
    int retcode;
    Jim_Cmd *cmdPtr;
    void *prevPrivData;
    Jim_Obj *tailcallObj = NULL;

#if 0
    printf("invoke");
    int j;
    for (j = 0; j < objc; j++) {
        printf(" '%s'", Jim_String(objv[j]));
    }
    printf("\n");
#endif

    cmdPtr = Jim_GetCommand(interp, objv[0], JIM_ERRMSG);
    if (cmdPtr == NULL) {
        return JimUnknown(interp, objc, objv);
    }
    JimIncrCmdRefCount(cmdPtr);

    if (interp->evalDepth == interp->maxEvalDepth) {
        Jim_SetResultString(interp, "Infinite eval recursion", -1);
        retcode = JIM_ERR;
        goto out;
    }
    interp->evalDepth++;
    prevPrivData = interp->cmdPrivData;

tailcall:

    interp->evalFrame->argc = objc;
    interp->evalFrame->argv = objv;
    interp->evalFrame->cmd = cmdPtr;

    if (!interp->traceCmdObj ||
        (retcode = JimTraceCallback(interp, "cmd", objc, objv)) == JIM_OK) {
        /* Call it -- Make sure result is an empty object. */
        Jim_SetEmptyResult(interp);
        interp->taint = Jim_CalcTaint(objc, objv);
        if (cmdPtr->flags & JIM_CMD_ISPROC) {
            retcode = JimCallProcedure(interp, cmdPtr, objc, objv);
        }
        else if ((cmdPtr->flags & JIM_CMD_NOTAINT) && Jim_CheckTaint(interp, JIM_TAINT_ANY)) {
            Jim_SetTaintError(interp, 1, objv);
            retcode = JIM_ERR;
        }
        else {
            retcode = JimCallNative(interp, cmdPtr, objc, objv);
            /* XXX Could it return -2 for tainted? */
        }
        if (retcode == JIM_ERR) {
            JimSetErrorStack(interp, NULL);
        }
    }

    if (tailcallObj) {
        /* clean up previous tailcall if we were invoking one */
        Jim_DecrRefCount(interp, tailcallObj);
        tailcallObj = NULL;
    }

    /* These are now invalid */
    interp->evalFrame->argc = 0;
    interp->evalFrame->argv = NULL;

    /* If a tailcall is returned for this frame, loop to invoke the new command */
    if (retcode == JIM_EVAL && interp->framePtr->tailcallObj) {
        JimDecrCmdRefCount(interp, cmdPtr);

        /* Replace the current command with the new tailcall command */
        cmdPtr = interp->framePtr->tailcallCmd;
        interp->framePtr->tailcallCmd = NULL;
        tailcallObj = interp->framePtr->tailcallObj;
        interp->framePtr->tailcallObj = NULL;
        /* We can access the internal rep here because the object can only
         * be constructed by the tailcall command
         */
        objc = tailcallObj->internalRep.listValue.len;
        objv = tailcallObj->internalRep.listValue.ele;
        goto tailcall;
    }

    interp->cmdPrivData = prevPrivData;
    interp->evalDepth--;

out:
    JimDecrCmdRefCount(interp, cmdPtr);

    if (retcode == JIM_ERR) {
        JimSetErrorStack(interp, NULL);
    }

    if (interp->framePtr->tailcallObj) {
        /* We might have skipped invoking a tailcall, perhaps because of an error
         * in defer handling so cleanup now
         */
        JimDecrCmdRefCount(interp, interp->framePtr->tailcallCmd);
        Jim_DecrRefCount(interp, interp->framePtr->tailcallObj);
        interp->framePtr->tailcallCmd = NULL;
        interp->framePtr->tailcallObj = NULL;
    }

    return retcode;
}

/* Eval the object vector 'objv' composed of 'objc' elements.
 * Every element is used as single argument.
 * Jim_EvalObj() will call this function every time its object
 * argument is of "list" type, with no string representation.
 *
 * This is possible because the string representation of a
 * list object generated by the UpdateStringOfList is made
 * in a way that ensures that every list element is a different

```


## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Restore the exact probe/runner, captured SDK/header/static-library/CLI inputs and explicit per-process environment, original ELF hashes and independently supplied hash-only system dependencies. Preserve two separate initialized active-case interpreters, the independent metadata interpreter, legacy C callback or actual Jim vector door and stack-backed observer records through teardown. Every original runner output is immutable; a separate output directory and capture are needed for a new execution. No replay was performed by record preparation, and no client-data free/refcount, Procedure/OO/import/recursive/rename/hide, Rust or BIG-IP equivalence is implied.
