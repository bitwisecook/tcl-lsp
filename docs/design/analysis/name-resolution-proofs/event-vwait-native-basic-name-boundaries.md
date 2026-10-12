# naming.event.vwait-native-basic-name-boundaries

Kind: `native-observation`

## Problem statement

An option-like lone variable name must retain the selected basic-vwait exception instead of entering an authored option scan.

## Question

How do basic x, lone -all, lone -signal and lone -- behave at each captured public vwait worker?

## Conclusion

The basic and lone -all/-signal controls return NEW on all six providers. Lone -- returns an empty result on every provider, but pinned C9 source selects the extended parser while legacy C/Jim use the literal basic variable. Equal output here does not establish equal handler input purpose.

## Scope

Four ASCII source controls per provider. Current public worker argv/name exception only; no raw binary or encoded-zero argv, observer cache, native compiler preparation or notifier completeness claim. All ninety original processes completed with exit 0 and no external timeout. Return-options objects, headers, references and command-evaluation stages were not measured. This public direct-source worker question does not certify compiled alias selection.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Fresh configured provider linked by exact header/library/executable hashes in original receipts; info patchlevel queried in each process.. Channel: ASCII NUL-terminated C source literals; Tcl_EvalEx length -1/TCL_EVAL_DIRECT or Jim_Eval; counted public result getter after whole script.. Dialect: tcl8.4.

basic: guest code 0, 3 bytes b'NEW'; lone-all: guest code 0, 3 bytes b'NEW'; lone-signal: guest code 0, 3 bytes b'NEW'; lone-end: guest code 0, 0 bytes b''

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Fresh configured provider linked by exact header/library/executable hashes in original receipts; info patchlevel queried in each process.. Channel: ASCII NUL-terminated C source literals; Tcl_EvalEx length -1/TCL_EVAL_DIRECT or Jim_Eval; counted public result getter after whole script.. Dialect: tcl8.5.

basic: guest code 0, 3 bytes b'NEW'; lone-all: guest code 0, 3 bytes b'NEW'; lone-signal: guest code 0, 3 bytes b'NEW'; lone-end: guest code 0, 0 bytes b''

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Fresh configured provider linked by exact header/library/executable hashes in original receipts; info patchlevel queried in each process.. Channel: ASCII NUL-terminated C source literals; Tcl_EvalEx length -1/TCL_EVAL_DIRECT or Jim_Eval; counted public result getter after whole script.. Dialect: tcl8.6.

basic: guest code 0, 3 bytes b'NEW'; lone-all: guest code 0, 3 bytes b'NEW'; lone-signal: guest code 0, 3 bytes b'NEW'; lone-end: guest code 0, 0 bytes b''

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Fresh configured provider linked by exact header/library/executable hashes in original receipts; info patchlevel queried in each process.. Channel: ASCII NUL-terminated C source literals; Tcl_EvalEx length -1/TCL_EVAL_DIRECT or Jim_Eval; counted public result getter after whole script.. Dialect: tcl9.0.

basic: guest code 0, 3 bytes b'NEW'; lone-all: guest code 0, 3 bytes b'NEW'; lone-signal: guest code 0, 3 bytes b'NEW'; lone-end: guest code 0, 0 bytes b''

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Fresh configured provider linked by exact header/library/executable hashes in original receipts; info patchlevel queried in each process.. Channel: ASCII NUL-terminated C source literals; Tcl_EvalEx length -1/TCL_EVAL_DIRECT or Jim_Eval; counted public result getter after whole script.. Dialect: tcl9.1.

basic: guest code 0, 3 bytes b'NEW'; lone-all: guest code 0, 3 bytes b'NEW'; lone-signal: guest code 0, 3 bytes b'NEW'; lone-end: guest code 0, 0 bytes b''

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Fresh configured provider linked by exact header/library/executable hashes in original receipts; info patchlevel queried in each process.. Channel: ASCII NUL-terminated C source literals; Tcl_EvalEx length -1/TCL_EVAL_DIRECT or Jim_Eval; counted public result getter after whole script.. Dialect: jim.

basic: guest code 0, 3 bytes b'NEW'; lone-all: guest code 0, 3 bytes b'NEW'; lone-signal: guest code 0, 3 bytes b'NEW'; lone-end: guest code 0, 0 bytes b''

### bigip

Status: `not-tested`. Version: not tested. Build: No attached build.. Channel: Not exercised.. Dialect: bigip.

No BIG-IP provider was launched or inspected for these controls.

## Exact evidence

- `eb2de2e2608b01205a4fc` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.4.20/basic.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.4.20/basic.json). SHA-256 `c225808cbb2a46dd2b110e27b10c2829ee7d8b3445dba95ae58813606de30809`. Exact fresh-case outside process status, queried version and counted public result rows.
- `ea1ff24286566d7316733` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.4.20/basic.stdout.tsv](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.4.20/basic.stdout.tsv). SHA-256 `911dde75d6483ed366771b9c047c89e918232c1e976005f71662ef004aed91a4`. Immutable original version/result stream; explicit byte count checked against hex.
- `e62b31848d4137cd01ba5` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.4.20/basic.stderr](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.4.20/basic.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Immutable fresh-case stderr.
- `ee448b8e66d0951a9e5d1` (provider): [rust/tcl-registry/tests/data/native_vwait_original/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.4.20/receipt.json). SHA-256 `56e65e1310881f86e2fca7a88fa3ea367f006e6bfbbd5a1abea3c60420243870`. Exact compiler, matching header/library/executable hashes, compile command and process attempts.
- `e2c3adfafa8230aba54b8` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.4.20/lone-all.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.4.20/lone-all.json). SHA-256 `ad99c6bf8a1b69fc00c30736d44f36692e174238285f4e3a1988cb7484f0f9fb`. Exact fresh-case outside process status, queried version and counted public result rows.
- `ee3da274461107b131307` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.4.20/lone-all.stdout.tsv](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.4.20/lone-all.stdout.tsv). SHA-256 `4396c1b95683f4546001e88d26e3d1569b82e6cc5542085af63075d03ed6f507`. Immutable original version/result stream; explicit byte count checked against hex.
- `e5be17d4d4bec51a894f8` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.4.20/lone-all.stderr](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.4.20/lone-all.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Immutable fresh-case stderr.
- `e154e87bf77c88e8132ba` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.4.20/lone-signal.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.4.20/lone-signal.json). SHA-256 `bcf0d71b9b0545237b99bd271a184002309875020f52d56b37cae1a7a31ef2e5`. Exact fresh-case outside process status, queried version and counted public result rows.
- `e3da696aeb16217f6e19e` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.4.20/lone-signal.stdout.tsv](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.4.20/lone-signal.stdout.tsv). SHA-256 `e54198bc4efbf8c304f633e976b3ce954b14f03a8b67840190d643ab1913fd48`. Immutable original version/result stream; explicit byte count checked against hex.
- `e711ae31e765754698b38` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.4.20/lone-signal.stderr](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.4.20/lone-signal.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Immutable fresh-case stderr.
- `e3402f90c46b8f0652dbc` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.4.20/lone-end.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.4.20/lone-end.json). SHA-256 `938c5854906d8d4eb0adab986a6e084a72c0bb9f30b261081a53a5706c7999a5`. Exact fresh-case outside process status, queried version and counted public result rows.
- `eedd77334917d42ef7809` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.4.20/lone-end.stdout.tsv](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.4.20/lone-end.stdout.tsv). SHA-256 `56c10fe1ec36cf4d75886737e1785fe7380d5ce53f5efc2cb1a3624fc6e73ba8`. Immutable original version/result stream; explicit byte count checked against hex.
- `ecf4f550c79a52604bfe7` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.4.20/lone-end.stderr](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.4.20/lone-end.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Immutable fresh-case stderr.
- `ed04d07a1274fc2b1a4b2` (input): [rust/tcl-registry/tests/data/native_vwait_original/probe.c](../../../../rust/tcl-registry/tests/data/native_vwait_original/probe.c). SHA-256 `ac4d4e8e34b71daaf93e4c35d9f92d380d53a0eeafa170a495fabb5bfe98de8b`. Exact original source/queue/harness; no new capture or repaired source. Derived inputs are definitions only.
- `ef0a2c0be217aa0bff877` (input): [rust/tcl-registry/tests/data/native_vwait_original/inputs.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/inputs.json). SHA-256 `d92c0f3d3a95a2fffacfebb864548e895e2c30bc2e380f43225a56b7e2bd7994`. Exact original source/queue/harness; no new capture or repaired source. Derived inputs are definitions only.
- `ee268c728a516f27332ef` (provider): [rust/tcl-registry/tests/data/native_vwait_original/capture.py](../../../../rust/tcl-registry/tests/data/native_vwait_original/capture.py). SHA-256 `5b0c5aa924f5fc6b18b3d3bdb783b163123afd47687cea49d36184a8b06eabe1`. Exact original source/queue/harness; no new capture or repaired source. Derived inputs are definitions only.
- `e270df4a56a72c8a2c89e` (provider): [rust/tcl-registry/tests/data/native_vwait_original/queue.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/queue.json). SHA-256 `5ccf57b0f68527dfa722dd477c174d84d11dcff996a91fa2884f4767704d2fb4`. Exact original source/queue/harness; no new capture or repaired source. Derived inputs are definitions only.
- `e44afe90097a4ce86849b` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.5.19/basic.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.5.19/basic.json). SHA-256 `17e950b58ed94bdd81840e3df1059f21dbedf1f9a3d79f41e720b7783555398d`. Exact fresh-case outside process status, queried version and counted public result rows.
- `e8f90bbc8d14e2fb99f1d` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.5.19/basic.stdout.tsv](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.5.19/basic.stdout.tsv). SHA-256 `2ddf67ea52a3b27c31de0d5364eb6383e62548af2398a6664d4e0bb24f3f3875`. Immutable original version/result stream; explicit byte count checked against hex.
- `e2e9e67c2dc8f727f8a9c` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.5.19/basic.stderr](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.5.19/basic.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Immutable fresh-case stderr.
- `ea4649d4dd03aa4036a6f` (provider): [rust/tcl-registry/tests/data/native_vwait_original/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.5.19/receipt.json). SHA-256 `e9d0897b97b41b5ecb9bab762d8a633456bae5a44eb0dc120dc23cc16e2ee1bf`. Exact compiler, matching header/library/executable hashes, compile command and process attempts.
- `e4a67f8449fb2b55a1be2` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.5.19/lone-all.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.5.19/lone-all.json). SHA-256 `85c4567cb741e13423ed57011f86765502382bf984a9edbf2112a1df9d02940f`. Exact fresh-case outside process status, queried version and counted public result rows.
- `e286932523c8b4074db3d` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.5.19/lone-all.stdout.tsv](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.5.19/lone-all.stdout.tsv). SHA-256 `4560bf486c08a521d9752c028eef69e30abb548eea2eede7d3dee99730dbf3f5`. Immutable original version/result stream; explicit byte count checked against hex.
- `e38dd4808f0e66df6a0bd` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.5.19/lone-all.stderr](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.5.19/lone-all.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Immutable fresh-case stderr.
- `e26a09a1f082361705326` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.5.19/lone-signal.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.5.19/lone-signal.json). SHA-256 `0842f38660d0b475ac9f9e8adfd60197d2f2a2663812394730023f1473e2d1e6`. Exact fresh-case outside process status, queried version and counted public result rows.
- `ea945b03571e2e9f29c82` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.5.19/lone-signal.stdout.tsv](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.5.19/lone-signal.stdout.tsv). SHA-256 `625b2aadc115f428b33b419fb96014d3f48219b84790c7055d1f7cc09fa07338`. Immutable original version/result stream; explicit byte count checked against hex.
- `e10afb3acbe0d6be60f7f` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.5.19/lone-signal.stderr](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.5.19/lone-signal.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Immutable fresh-case stderr.
- `ecdab248fb7a31fece0d0` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.5.19/lone-end.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.5.19/lone-end.json). SHA-256 `9c21b8ba09f284e04e3bc3d26d89c154ca724a00f8be1661ed16f3b4610b2856`. Exact fresh-case outside process status, queried version and counted public result rows.
- `e065f49d7152ad11ef232` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.5.19/lone-end.stdout.tsv](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.5.19/lone-end.stdout.tsv). SHA-256 `1801a51ba1fede12079bc42665a60daa9d4d69ccce58aba370c9c3dc44555c75`. Immutable original version/result stream; explicit byte count checked against hex.
- `e2334051472216d977dde` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.5.19/lone-end.stderr](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.5.19/lone-end.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Immutable fresh-case stderr.
- `ee5a4e06fac93a0705e0a` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.6.18/basic.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.6.18/basic.json). SHA-256 `fa7c74604a3cc25a94bdb8a9e186ebb8fb8e29d6a59118c3df0ce92009bcb823`. Exact fresh-case outside process status, queried version and counted public result rows.
- `e40f9cf0070de7f3835d5` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.6.18/basic.stdout.tsv](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.6.18/basic.stdout.tsv). SHA-256 `11dd69004035ac86c13be78ee6319fc2ca0e3cc5f1d753a297217446e45f40df`. Immutable original version/result stream; explicit byte count checked against hex.
- `e577a41b01e1c3af1d74e` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.6.18/basic.stderr](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.6.18/basic.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Immutable fresh-case stderr.
- `e59663020753e1d549bd1` (provider): [rust/tcl-registry/tests/data/native_vwait_original/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.6.18/receipt.json). SHA-256 `36e0b31eb921934c694c56ffd7df3c71913add0acfc3f66697aa09731e7f1f4c`. Exact compiler, matching header/library/executable hashes, compile command and process attempts.
- `e1b2864bf24338a4d300d` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.6.18/lone-all.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.6.18/lone-all.json). SHA-256 `8905203d654e1901e93c7c012e6ccfea6927f6df11d232dd7d463db74f3d5fd1`. Exact fresh-case outside process status, queried version and counted public result rows.
- `ea28ee61cdfb0cc0b3190` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.6.18/lone-all.stdout.tsv](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.6.18/lone-all.stdout.tsv). SHA-256 `72de99e002cecbd793ce89c26879558bcb705b9dd315c03f3652a4aa81d86636`. Immutable original version/result stream; explicit byte count checked against hex.
- `e10093927e705d370e4d5` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.6.18/lone-all.stderr](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.6.18/lone-all.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Immutable fresh-case stderr.
- `e6e4295abe29cb8e44d39` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.6.18/lone-signal.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.6.18/lone-signal.json). SHA-256 `a098478e6127eaeadd04bf4d3b828ad43903b7a8bb897a3b20a89f300fea67b4`. Exact fresh-case outside process status, queried version and counted public result rows.
- `e8c44012f687acc868299` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.6.18/lone-signal.stdout.tsv](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.6.18/lone-signal.stdout.tsv). SHA-256 `a5fec144ed419c3c6a43b833bb2ea6fa3a1931ee040a3dcd6b687bd70d48baa1`. Immutable original version/result stream; explicit byte count checked against hex.
- `e84ac820c0b5dbbdc08f5` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.6.18/lone-signal.stderr](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.6.18/lone-signal.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Immutable fresh-case stderr.
- `ea9c620df3579ce97a5ae` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.6.18/lone-end.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.6.18/lone-end.json). SHA-256 `b078f7a808e5fbe2452309f125a900ac3bf8c85b5d0130e8283e0eae841f9d22`. Exact fresh-case outside process status, queried version and counted public result rows.
- `ef6858321cfafe1607b1a` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.6.18/lone-end.stdout.tsv](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.6.18/lone-end.stdout.tsv). SHA-256 `49c9590f2771706db782758a62071d154a732e7ac8cc057926db881650fe054a`. Immutable original version/result stream; explicit byte count checked against hex.
- `e9e0b09003225c2d96e88` (observation): [rust/tcl-registry/tests/data/native_vwait_original/8.6.18/lone-end.stderr](../../../../rust/tcl-registry/tests/data/native_vwait_original/8.6.18/lone-end.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Immutable fresh-case stderr.
- `e80279687a5eea4ba6b18` (observation): [rust/tcl-registry/tests/data/native_vwait_original/9.0.4/basic.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.0.4/basic.json). SHA-256 `7a8390f69ed6375522e11f4efad9cd0e1841a882aa430a28f93f9c2280caa659`. Exact fresh-case outside process status, queried version and counted public result rows.
- `e1c7507ffaa9c00e91994` (observation): [rust/tcl-registry/tests/data/native_vwait_original/9.0.4/basic.stdout.tsv](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.0.4/basic.stdout.tsv). SHA-256 `3ee6abf48907d7c164732b9168888a7182cebe0adc373d898286ef399207fba6`. Immutable original version/result stream; explicit byte count checked against hex.
- `e1590aaff02e177abb075` (observation): [rust/tcl-registry/tests/data/native_vwait_original/9.0.4/basic.stderr](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.0.4/basic.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Immutable fresh-case stderr.
- `ef1450973f13447d981ba` (provider): [rust/tcl-registry/tests/data/native_vwait_original/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.0.4/receipt.json). SHA-256 `b28141dad46630d7c783802d7cbf82f4fdea55a3e46d25c4452a4f9e89d4c318`. Exact compiler, matching header/library/executable hashes, compile command and process attempts.
- `e9d3837b7aef2a9efc732` (observation): [rust/tcl-registry/tests/data/native_vwait_original/9.0.4/lone-all.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.0.4/lone-all.json). SHA-256 `02897cd22295a92a46a31e8e96ffc3a4181749da62636a27cefcc8d0930e4b49`. Exact fresh-case outside process status, queried version and counted public result rows.
- `e595ad3f43adc75105128` (observation): [rust/tcl-registry/tests/data/native_vwait_original/9.0.4/lone-all.stdout.tsv](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.0.4/lone-all.stdout.tsv). SHA-256 `9fbad0fe1c987bcde7f784bcd61400c34a540dec4122a94fd92549e2a7524d4f`. Immutable original version/result stream; explicit byte count checked against hex.
- `ef6f3bab745b8a54b83ff` (observation): [rust/tcl-registry/tests/data/native_vwait_original/9.0.4/lone-all.stderr](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.0.4/lone-all.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Immutable fresh-case stderr.
- `ea2402aee3ffcb04b7a02` (observation): [rust/tcl-registry/tests/data/native_vwait_original/9.0.4/lone-signal.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.0.4/lone-signal.json). SHA-256 `9db045d9b7039610cfebdf8c34d705f08b1a71255d6c4198b9c1de0aee8fab78`. Exact fresh-case outside process status, queried version and counted public result rows.
- `e745fcf483daf448920cf` (observation): [rust/tcl-registry/tests/data/native_vwait_original/9.0.4/lone-signal.stdout.tsv](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.0.4/lone-signal.stdout.tsv). SHA-256 `1d81397350bbdb6522e139d05c163cb6d86c9157d46001a439762fc832878fa2`. Immutable original version/result stream; explicit byte count checked against hex.
- `e245dd9ae11b01780ce1c` (observation): [rust/tcl-registry/tests/data/native_vwait_original/9.0.4/lone-signal.stderr](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.0.4/lone-signal.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Immutable fresh-case stderr.
- `e341c0fe2bd054d6b58dc` (observation): [rust/tcl-registry/tests/data/native_vwait_original/9.0.4/lone-end.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.0.4/lone-end.json). SHA-256 `784db36e10829d3d6ce2b5384c93c55f1b1bb7c19c5b438cd031b66c65f54a5f`. Exact fresh-case outside process status, queried version and counted public result rows.
- `e75c96f698d0b31f5c0b8` (observation): [rust/tcl-registry/tests/data/native_vwait_original/9.0.4/lone-end.stdout.tsv](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.0.4/lone-end.stdout.tsv). SHA-256 `40f6c81869403f6b3bd05f4b6b09be176180a6c15891c6bc255dea7583975fb8`. Immutable original version/result stream; explicit byte count checked against hex.
- `ea9141c748760d71a852f` (observation): [rust/tcl-registry/tests/data/native_vwait_original/9.0.4/lone-end.stderr](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.0.4/lone-end.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Immutable fresh-case stderr.
- `ebcdbcc3e58138429750c` (observation): [rust/tcl-registry/tests/data/native_vwait_original/9.1.0/basic.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.1.0/basic.json). SHA-256 `fdc7f67b773d591d005d67411a4da9e1eab239e3a2de84c4630dccb0a293e0a7`. Exact fresh-case outside process status, queried version and counted public result rows.
- `e3d29d218b53b6757dad2` (observation): [rust/tcl-registry/tests/data/native_vwait_original/9.1.0/basic.stdout.tsv](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.1.0/basic.stdout.tsv). SHA-256 `f15220d9f6f9a3990221f4f71b8e80b35842e7a51c664a52b43bc2ef29c12a0b`. Immutable original version/result stream; explicit byte count checked against hex.
- `eda0dd976aa4e4d891a4a` (observation): [rust/tcl-registry/tests/data/native_vwait_original/9.1.0/basic.stderr](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.1.0/basic.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Immutable fresh-case stderr.
- `e966e06b74b40b0e83972` (provider): [rust/tcl-registry/tests/data/native_vwait_original/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.1.0/receipt.json). SHA-256 `3d655dffe2f13b503f2480deab9ebe9c5423366eb9d2887283626e7db3ab7b98`. Exact compiler, matching header/library/executable hashes, compile command and process attempts.
- `e02c6f62fa251169dc904` (observation): [rust/tcl-registry/tests/data/native_vwait_original/9.1.0/lone-all.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.1.0/lone-all.json). SHA-256 `5972f4fa75168302c63edcc9b0b645050ffc899fccc7990b50496a3af1d48e99`. Exact fresh-case outside process status, queried version and counted public result rows.
- `ec2aa87729d76b3faf7c2` (observation): [rust/tcl-registry/tests/data/native_vwait_original/9.1.0/lone-all.stdout.tsv](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.1.0/lone-all.stdout.tsv). SHA-256 `bf1cf9ce584fec092cb1c60319c05a01fc045f527f5d91f2d9c07fe824195d84`. Immutable original version/result stream; explicit byte count checked against hex.
- `ead6d9936ef3f8d21d623` (observation): [rust/tcl-registry/tests/data/native_vwait_original/9.1.0/lone-all.stderr](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.1.0/lone-all.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Immutable fresh-case stderr.
- `e5f446fa5de4294e3f877` (observation): [rust/tcl-registry/tests/data/native_vwait_original/9.1.0/lone-signal.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.1.0/lone-signal.json). SHA-256 `256862f2712b4a28c3cbab69b9cec2a4e3ac317a7da20a57f9dbd9719b4879ce`. Exact fresh-case outside process status, queried version and counted public result rows.
- `ebb9fecad097919c16b90` (observation): [rust/tcl-registry/tests/data/native_vwait_original/9.1.0/lone-signal.stdout.tsv](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.1.0/lone-signal.stdout.tsv). SHA-256 `89b493e1f376201ff171bd323bc854fcc81aac830500f7e95d1f6a8c9add2827`. Immutable original version/result stream; explicit byte count checked against hex.
- `eb154702ac1f3077da826` (observation): [rust/tcl-registry/tests/data/native_vwait_original/9.1.0/lone-signal.stderr](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.1.0/lone-signal.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Immutable fresh-case stderr.
- `ebb5cb42fbbfad1dce2f9` (observation): [rust/tcl-registry/tests/data/native_vwait_original/9.1.0/lone-end.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.1.0/lone-end.json). SHA-256 `f7ba04771e1b691dc4bfc20994e5aee9aec6985770eb710167d0820e2271c5be`. Exact fresh-case outside process status, queried version and counted public result rows.
- `e52adc9aff67edd6eea24` (observation): [rust/tcl-registry/tests/data/native_vwait_original/9.1.0/lone-end.stdout.tsv](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.1.0/lone-end.stdout.tsv). SHA-256 `c705dcd0d2a535a48f216582aa611d1be42cdfdb096eb626f584993ff39c60c4`. Immutable original version/result stream; explicit byte count checked against hex.
- `e5fc4941431983bd50fff` (observation): [rust/tcl-registry/tests/data/native_vwait_original/9.1.0/lone-end.stderr](../../../../rust/tcl-registry/tests/data/native_vwait_original/9.1.0/lone-end.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Immutable fresh-case stderr.
- `e12a203f7cbc23886fe7e` (observation): [rust/tcl-registry/tests/data/native_vwait_original/jim/basic.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/jim/basic.json). SHA-256 `174834dec99e14d86528547932716784ac36fbf91d3a93e2661e19a0cfb46536`. Exact fresh-case outside process status, queried version and counted public result rows.
- `e1176b9b87877f80f4ad5` (observation): [rust/tcl-registry/tests/data/native_vwait_original/jim/basic.stdout.tsv](../../../../rust/tcl-registry/tests/data/native_vwait_original/jim/basic.stdout.tsv). SHA-256 `a8f62f23e7aafffaa08c5bcc63ada9b4f3921668de58b9974fc6aeeb8cb974c0`. Immutable original version/result stream; explicit byte count checked against hex.
- `e338a9cec8a3443268093` (observation): [rust/tcl-registry/tests/data/native_vwait_original/jim/basic.stderr](../../../../rust/tcl-registry/tests/data/native_vwait_original/jim/basic.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Immutable fresh-case stderr.
- `e4ee339a7a62e6661e884` (provider): [rust/tcl-registry/tests/data/native_vwait_original/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/jim/receipt.json). SHA-256 `04b0cf9b726b0823a589ba1e1cc46384ec861d88afdbe44908a1b3f2ac82d464`. Exact compiler, matching header/library/executable hashes, compile command and process attempts.
- `e9e13c7beaf0d75309fc1` (observation): [rust/tcl-registry/tests/data/native_vwait_original/jim/lone-all.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/jim/lone-all.json). SHA-256 `8c6df3efbc18431c0a2a357812771024b1fa732ced311af8077befef3be4428a`. Exact fresh-case outside process status, queried version and counted public result rows.
- `e4cff20db3ff99f4d6aea` (observation): [rust/tcl-registry/tests/data/native_vwait_original/jim/lone-all.stdout.tsv](../../../../rust/tcl-registry/tests/data/native_vwait_original/jim/lone-all.stdout.tsv). SHA-256 `ff6a31928174948d4d268aacc66a806173b5e492bb40f605bd919c010ee53c47`. Immutable original version/result stream; explicit byte count checked against hex.
- `e857baa2c0d6485ca41ae` (observation): [rust/tcl-registry/tests/data/native_vwait_original/jim/lone-all.stderr](../../../../rust/tcl-registry/tests/data/native_vwait_original/jim/lone-all.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Immutable fresh-case stderr.
- `ea9b85c99402631e6f7eb` (observation): [rust/tcl-registry/tests/data/native_vwait_original/jim/lone-signal.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/jim/lone-signal.json). SHA-256 `928294ae7c4f57a08d3ed629c54a84e0e87d198faa7b8d74e5de84b911fbd594`. Exact fresh-case outside process status, queried version and counted public result rows.
- `e7c3f5b80e036bfa41a0b` (observation): [rust/tcl-registry/tests/data/native_vwait_original/jim/lone-signal.stdout.tsv](../../../../rust/tcl-registry/tests/data/native_vwait_original/jim/lone-signal.stdout.tsv). SHA-256 `39474dbd1e79d0e5c8bb8cbb71c762089d67d65a8e0a6481e40bce98bed40686`. Immutable original version/result stream; explicit byte count checked against hex.
- `ed1fdb2a55ee151b9a62a` (observation): [rust/tcl-registry/tests/data/native_vwait_original/jim/lone-signal.stderr](../../../../rust/tcl-registry/tests/data/native_vwait_original/jim/lone-signal.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Immutable fresh-case stderr.
- `e417c60fffce74c24cdd0` (observation): [rust/tcl-registry/tests/data/native_vwait_original/jim/lone-end.json](../../../../rust/tcl-registry/tests/data/native_vwait_original/jim/lone-end.json). SHA-256 `914636f5c403f3284491aa6986f3e52ed97937899e1d20087ced8515e8f6b0d5`. Exact fresh-case outside process status, queried version and counted public result rows.
- `e10edfd9343205729d230` (observation): [rust/tcl-registry/tests/data/native_vwait_original/jim/lone-end.stdout.tsv](../../../../rust/tcl-registry/tests/data/native_vwait_original/jim/lone-end.stdout.tsv). SHA-256 `a0cc447846b99baa8274feef9b81a486402eefdaffc15641b46f0a3fb23dfdaa`. Immutable original version/result stream; explicit byte count checked against hex.
- `eec22ccfe1129a38cdea0` (observation): [rust/tcl-registry/tests/data/native_vwait_original/jim/lone-end.stderr](../../../../rust/tcl-registry/tests/data/native_vwait_original/jim/lone-end.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Immutable fresh-case stderr.
- `e98d4bc726037374525cd` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.4.20/tclEvent.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.4.20/tclEvent.c). SHA-256 `020cc8b9d0b020c2b1dac904f08b03c3e59889dd2ff493677f04696524d40673`. Pinned source inspection from prior Event source archive; not a new compiled-worker execution.
- `ebcd14ee056c41f642dc1` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclEvent.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclEvent.c). SHA-256 `3d05eb479c767520e0b44459e91a0b4930cdb47139c4bfdb86e0beba6d04fb90`. Pinned source inspection from prior Event source archive; not a new compiled-worker execution.
- `e1799f8f23cd143629768` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclEvent.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclEvent.c). SHA-256 `81c0e0b655ecd46bd981750c9d710ec11dd1368b7f3f156886e0965c81788dd6`. Pinned source inspection from prior Event source archive; not a new compiled-worker execution.
- `e30bb9be41c1824d28e5d` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclEvent.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclEvent.c). SHA-256 `e6b69ac0fc6c9c335af90623398d06eb2d8ef7d2df5574327a52f622a37e31e4`. Pinned source inspection from prior Event source archive; not a new compiled-worker execution.
- `eaa5f919dc246958bc5d1` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclEvent.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclEvent.c). SHA-256 `9451a2c540dcd5ed61675c15f833f5ad677b551b61deb210ee6837216f477a54`. Pinned source inspection from prior Event source archive; not a new compiled-worker execution.
- `e4521e47b782c5165c471` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/jim/jim-eventloop.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/jim/jim-eventloop.c). SHA-256 `b9536f7021919026f8ee2cfcdb35a59305db07a23e60daa50d5da966d52a1a12`. Pinned source inspection from prior Event source archive; not a new compiled-worker execution.

## Source inspection

tcl8.4 8.4.20, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclEvent.c`, function `Tcl_VwaitObjCmd`, lines 1176–1216. Full-source SHA-256 `020cc8b9d0b020c2b1dac904f08b03c3e59889dd2ff493677f04696524d40673`; snippet SHA-256 `1328a8f513834f5b7a5b16fc36f88138ec507eb04f53c43f8b8a9edda31a52ef`; retained evidence `e98d4bc726037374525cd`.

```text
Tcl_VwaitObjCmd(clientData, interp, objc, objv)
    ClientData clientData;	/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    int done, foundEvent;
    char *nameString;

    if (objc != 2) {
        Tcl_WrongNumArgs(interp, 1, objv, "name");
	return TCL_ERROR;
    }
    nameString = Tcl_GetString(objv[1]);
    if (Tcl_TraceVar(interp, nameString,
	    TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
	    VwaitVarProc, (ClientData) &done) != TCL_OK) {
	return TCL_ERROR;
    };
    done = 0;
    foundEvent = 1;
    while (!done && foundEvent) {
	foundEvent = Tcl_DoOneEvent(TCL_ALL_EVENTS);
    }
    Tcl_UntraceVar(interp, nameString,
	    TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
	    VwaitVarProc, (ClientData) &done);

    /*
     * Clear out the interpreter's result, since it may have been set
     * by event handlers.
     */

    Tcl_ResetResult(interp);
    if (!foundEvent) {
	Tcl_AppendResult(interp, "can't wait for variable \"", nameString,
		"\":  would wait forever", (char *) NULL);
	return TCL_ERROR;
    }
    return TCL_OK;
}
```

tcl8.5 8.5.19, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclEvent.c`, function `Tcl_VwaitObjCmd`, lines 1321–1368. Full-source SHA-256 `3d05eb479c767520e0b44459e91a0b4930cdb47139c4bfdb86e0beba6d04fb90`; snippet SHA-256 `cf24836c48502fc46b580d223b5cf8b21de1d5cad5eef848cd57b5caa2d4b60f`; retained evidence `ebcd14ee056c41f642dc1`.

```text
Tcl_VwaitObjCmd(
    ClientData clientData,	/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *CONST objv[])	/* Argument objects. */
{
    int done, foundEvent;
    char *nameString;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "name");
	return TCL_ERROR;
    }
    nameString = Tcl_GetString(objv[1]);
    if (Tcl_TraceVar(interp, nameString,
	    TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
	    VwaitVarProc, (ClientData) &done) != TCL_OK) {
	return TCL_ERROR;
    };
    done = 0;
    foundEvent = 1;
    while (!done && foundEvent) {
	foundEvent = Tcl_DoOneEvent(TCL_ALL_EVENTS);
	if (Tcl_LimitExceeded(interp)) {
	    break;
	}
    }
    Tcl_UntraceVar(interp, nameString,
	    TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
	    VwaitVarProc, (ClientData) &done);

    /*
     * Clear out the interpreter's result, since it may have been set by event
     * handlers.
     */

    Tcl_ResetResult(interp);
    if (!foundEvent) {
	Tcl_AppendResult(interp, "can't wait for variable \"", nameString,
		"\": would wait forever", NULL);
	return TCL_ERROR;
    }
    if (!done) {
	Tcl_AppendResult(interp, "limit exceeded", NULL);
	return TCL_ERROR;
    }
    return TCL_OK;
}
```

tcl8.6 8.6.18, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEvent.c`, function `Tcl_VwaitObjCmd`, lines 1387–1447. Full-source SHA-256 `81c0e0b655ecd46bd981750c9d710ec11dd1368b7f3f156886e0965c81788dd6`; snippet SHA-256 `6a720ec0b86320ee0c3eb9ac57fe7dda865390cee80306fd1844c2ccb65b85a7`; retained evidence `e1799f8f23cd143629768`.

```text
Tcl_VwaitObjCmd(
    ClientData clientData,	/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int done, foundEvent;
    const char *nameString;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "name");
	return TCL_ERROR;
    }
    nameString = Tcl_GetString(objv[1]);
    if (Tcl_TraceVar2(interp, nameString, NULL,
	    TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
	    VwaitVarProc, &done) != TCL_OK) {
	return TCL_ERROR;
    };
    done = 0;
    foundEvent = 1;
    while (!done && foundEvent) {
	foundEvent = Tcl_DoOneEvent(TCL_ALL_EVENTS);
	if (Tcl_Canceled(interp, TCL_LEAVE_ERR_MSG) == TCL_ERROR) {
	    break;
	}
	if (Tcl_LimitExceeded(interp)) {
	    Tcl_ResetResult(interp);
	    Tcl_SetObjResult(interp, Tcl_NewStringObj("limit exceeded", -1));
	    break;
	}
    }
    Tcl_UntraceVar2(interp, nameString, NULL,
	    TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
	    VwaitVarProc, &done);

    if (!foundEvent) {
	Tcl_ResetResult(interp);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"can't wait for variable \"%s\": would wait forever",
		nameString));
	Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_SOURCES", (char *)NULL);
	return TCL_ERROR;
    }
    if (!done) {
	/*
	 * The interpreter's result was already set to the right error message
	 * prior to exiting the loop above.
	 */

	return TCL_ERROR;
    }

    /*
     * Clear out the interpreter's result, since it may have been set by event
     * handlers.
     */

    Tcl_ResetResult(interp);
    return TCL_OK;
}
```

tcl9.0 9.0.4, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEvent.c`, function `Tcl_VwaitObjCmd`, lines 1498–1867. Full-source SHA-256 `e6b69ac0fc6c9c335af90623398d06eb2d8ef7d2df5574327a52f622a37e31e4`; snippet SHA-256 `51c2ed4fc19f0138783d2623b8c15f51f5485a04d976d7e8bbb2432d5a59c33b`; retained evidence `e30bb9be41c1824d28e5d`.

```text
Tcl_VwaitObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int i, done = 0, timedOut = 0, foundEvent, any = 1, timeout = 0;
    int numItems = 0, extended = 0, result, mode, mask = TCL_ALL_EVENTS;
    Tcl_InterpState saved = NULL;
    Tcl_TimerToken timer = NULL;
    Tcl_Time before, after;
    Tcl_Channel chan;
    Tcl_WideInt diff = -1;
    VwaitItem localItems[32], *vwaitItems = localItems;
    static const char *const vWaitOptionStrings[] = {
	"-all",	"-extended", "-nofileevents", "-noidleevents",
	"-notimerevents", "-nowindowevents", "-readable",
	"-timeout", "-variable", "-writable", "--", NULL
    };
    enum vWaitOptions {
	OPT_ALL, OPT_EXTD, OPT_NO_FEVTS, OPT_NO_IEVTS,
	OPT_NO_TEVTS, OPT_NO_WEVTS, OPT_READABLE,
	OPT_TIMEOUT, OPT_VARIABLE, OPT_WRITABLE, OPT_LAST
    } index;

    if ((objc == 2) && (strcmp(Tcl_GetString(objv[1]), "--") != 0)) {
	/*
	 * Legacy "vwait" syntax, skip option handling.
	 */
	i = 1;
	goto endOfOptionLoop;
    }

    if ((unsigned) objc - 1 > sizeof(localItems) / sizeof(localItems[0])) {
	vwaitItems = (VwaitItem *)Tcl_Alloc(sizeof(VwaitItem) * (objc - 1));
    }

    for (i = 1; i < objc; i++) {
	const char *name;

	name = TclGetString(objv[i]);
	if (name[0] != '-') {
	    break;
	}
	if (Tcl_GetIndexFromObj(interp, objv[i], vWaitOptionStrings, "option", 0,
		&index) != TCL_OK) {
	    result = TCL_ERROR;
	    goto done;
	}
	switch (index) {
	case OPT_ALL:
	    any = 0;
	    break;
	case OPT_EXTD:
	    extended = 1;
	    break;
	case OPT_NO_FEVTS:
	    mask &= ~TCL_FILE_EVENTS;
	    break;
	case OPT_NO_IEVTS:
	    mask &= ~TCL_IDLE_EVENTS;
	    break;
	case OPT_NO_TEVTS:
	    mask &= ~TCL_TIMER_EVENTS;
	    break;
	case OPT_NO_WEVTS:
	    mask &= ~TCL_WINDOW_EVENTS;
	    break;
	case OPT_TIMEOUT:
	    if (++i >= objc) {
	needArg:
		Tcl_ResetResult(interp);
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"argument required for \"%s\"", vWaitOptionStrings[index]));
		Tcl_SetErrorCode(interp, "TCL", "EVENT", "ARGUMENT", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	    if (Tcl_GetIntFromObj(interp, objv[i], &timeout) != TCL_OK) {
		result = TCL_ERROR;
		goto done;
	    }
	    if (timeout < 0) {
		Tcl_ResetResult(interp);
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"timeout must be positive", -1));
		Tcl_SetErrorCode(interp, "TCL", "EVENT", "NEGTIME", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	    break;
	case OPT_LAST:
	    i++;
	    goto endOfOptionLoop;
	case OPT_VARIABLE:
	    if (++i >= objc) {
		goto needArg;
	    }
	    result = Tcl_TraceVar2(interp, TclGetString(objv[i]), NULL,
		    TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
		    VwaitVarProc, &vwaitItems[numItems]);
	    if (result != TCL_OK) {
		goto done;
	    }
	    vwaitItems[numItems].donePtr = &done;
	    vwaitItems[numItems].sequence = -1;
	    vwaitItems[numItems].mask = 0;
	    vwaitItems[numItems].sourceObj = objv[i];
	    numItems++;
	    break;
	case OPT_READABLE:
	    if (++i >= objc) {
		goto needArg;
	    }
	    if (TclGetChannelFromObj(interp, objv[i], &chan, &mode, 0)
		    != TCL_OK) {
		result = TCL_ERROR;
		goto done;
	    }
	    if (!(mode & TCL_READABLE)) {
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"channel \"%s\" wasn't open for reading",
			TclGetString(objv[i])));
		result = TCL_ERROR;
		goto done;
	    }
	    Tcl_CreateChannelHandler(chan, TCL_READABLE,
		    VwaitChannelReadProc, &vwaitItems[numItems]);
	    vwaitItems[numItems].donePtr = &done;
	    vwaitItems[numItems].sequence = -1;
	    vwaitItems[numItems].mask = TCL_READABLE;
	    vwaitItems[numItems].sourceObj = objv[i];
	    numItems++;
	    break;
	case OPT_WRITABLE:
	    if (++i >= objc) {
		goto needArg;
	    }
	    if (TclGetChannelFromObj(interp, objv[i], &chan, &mode, 0)
		    != TCL_OK) {
		result = TCL_ERROR;
		goto done;
	    }
	    if (!(mode & TCL_WRITABLE)) {
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"channel \"%s\" wasn't open for writing",
			TclGetString(objv[i])));
		result = TCL_ERROR;
		goto done;
	    }
	    Tcl_CreateChannelHandler(chan, TCL_WRITABLE,
		    VwaitChannelWriteProc, &vwaitItems[numItems]);
	    vwaitItems[numItems].donePtr = &done;
	    vwaitItems[numItems].sequence = -1;
	    vwaitItems[numItems].mask = TCL_WRITABLE;
	    vwaitItems[numItems].sourceObj = objv[i];
	    numItems++;
	    break;
	default:
	    TCL_UNREACHABLE();
	}
    }

  endOfOptionLoop:
    if ((mask & (TCL_FILE_EVENTS | TCL_IDLE_EVENTS |
	    TCL_TIMER_EVENTS | TCL_WINDOW_EVENTS)) == 0) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"can't wait: would block forever", -1));
	Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_SOURCES", (char *)NULL);
	result = TCL_ERROR;
	goto done;
    }

    if ((timeout > 0) && ((mask & TCL_TIMER_EVENTS) == 0)) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"timer events disabled with timeout specified", -1));
	Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_TIME", (char *)NULL);
	result = TCL_ERROR;
	goto done;
    }

    for (result = TCL_OK; i < objc; i++) {
	result = Tcl_TraceVar2(interp, TclGetString(objv[i]), NULL,
		TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
		VwaitVarProc, &vwaitItems[numItems]);
	if (result != TCL_OK) {
	    break;
	}
	vwaitItems[numItems].donePtr = &done;
	vwaitItems[numItems].sequence = -1;
	vwaitItems[numItems].mask = 0;
	vwaitItems[numItems].sourceObj = objv[i];
	numItems++;
    }
    if (result != TCL_OK) {
	result = TCL_ERROR;
	goto done;
    }

    if (!(mask & TCL_FILE_EVENTS)) {
	for (i = 0; i < numItems; i++) {
	    if (vwaitItems[i].mask) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"file events disabled with channel(s) specified", -1));
		Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_FILE_EVENT", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	}
    }

    if (timeout > 0) {
	vwaitItems[numItems].donePtr = &timedOut;
	vwaitItems[numItems].sequence = -1;
	vwaitItems[numItems].mask = 0;
	vwaitItems[numItems].sourceObj = NULL;
	timer = Tcl_CreateTimerHandler(timeout, VwaitTimeoutProc,
		&vwaitItems[numItems]);
	Tcl_GetTime(&before);
    } else {
	timeout = 0;
    }

    if ((numItems == 0) && (timeout == 0)) {
	/*
	 * "vwait" is equivalent to "update",
	 * "vwait -nofileevents -notimerevents -nowindowevents"
	 * is equivalent to "update idletasks"
	 */
	any = 1;
	mask |= TCL_DONT_WAIT;
    }

    foundEvent = 1;
    while (!timedOut && foundEvent &&
	   ((!any && (done < numItems)) || (any && !done))) {
	foundEvent = Tcl_DoOneEvent(mask);
	if (Tcl_Canceled(interp, TCL_LEAVE_ERR_MSG) == TCL_ERROR) {
	    break;
	}
	if (Tcl_LimitExceeded(interp)) {
	    Tcl_ResetResult(interp);
	    Tcl_SetObjResult(interp, Tcl_NewStringObj("limit exceeded", -1));
	    Tcl_SetErrorCode(interp, "TCL", "EVENT", "LIMIT", (char *)NULL);
	    break;
	}
	if ((numItems == 0) && (timeout == 0)) {
	    /*
	     * Behavior like "update": clear interpreter's result because
	     * event handlers could have executed commands.
	     */
	    Tcl_ResetResult(interp);
	    result = TCL_OK;
	    goto done;
	}
    }

    if (!foundEvent) {
	Tcl_ResetResult(interp);
	Tcl_SetObjResult(interp, Tcl_NewStringObj((numItems == 0) ?
		"can't wait: would wait forever" :
		"can't wait for variable(s)/channel(s): would wait forever",
		-1));
	Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_SOURCES", (char *)NULL);
	result = TCL_ERROR;
	goto done;
    }

    if (!done && !timedOut) {
	/*
	 * The interpreter's result was already set to the right error message
	 * prior to exiting the loop above.
	 */
	result = TCL_ERROR;
	goto done;
    }

    result = TCL_OK;
    if (timeout <= 0) {
	/*
	 * Clear out the interpreter's result, since it may have been set
	 * by event handlers.
	 */
	Tcl_ResetResult(interp);
	goto done;
    }

    /*
     * When timeout was specified, report milliseconds left or -1 on timeout.
     */
    if (timedOut) {
	diff = -1;
    } else {
	Tcl_GetTime(&after);
	diff = after.sec * 1000 + after.usec / 1000;
	diff -= before.sec * 1000 + before.usec / 1000;
	diff = timeout - diff;
	if (diff < 0) {
	    diff = 0;
	}
    }

  done:
    if ((timeout > 0) && (timer != NULL)) {
	Tcl_DeleteTimerHandler(timer);
    }
    if (result != TCL_OK) {
	saved = Tcl_SaveInterpState(interp, result);
    }
    for (i = 0; i < numItems; i++) {
	if (vwaitItems[i].mask & TCL_READABLE) {
	    if (TclGetChannelFromObj(interp, vwaitItems[i].sourceObj,
		    &chan, &mode, 0) == TCL_OK) {
		Tcl_DeleteChannelHandler(chan, VwaitChannelReadProc,
			&vwaitItems[i]);
	    }
	} else if (vwaitItems[i].mask & TCL_WRITABLE) {
	    if (TclGetChannelFromObj(interp, vwaitItems[i].sourceObj,
		    &chan, &mode, 0) == TCL_OK) {
		Tcl_DeleteChannelHandler(chan, VwaitChannelWriteProc,
			&vwaitItems[i]);
	    }
	} else {
	    Tcl_UntraceVar2(interp, TclGetString(vwaitItems[i].sourceObj),
		    NULL, TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
		    VwaitVarProc, &vwaitItems[i]);
	}
    }

    if (result == TCL_OK) {
	if (extended) {
	    int k;
	    Tcl_Obj *listObj, *keyObj;

	    TclNewObj(listObj);
	    for (k = 0; k < done; k++) {
		for (i = 0; i < numItems; i++) {
		    if (vwaitItems[i].sequence != k) {
			continue;
		    }
		    if (vwaitItems[i].mask & TCL_READABLE) {
			TclNewLiteralStringObj(keyObj, "readable");
		    } else if (vwaitItems[i].mask & TCL_WRITABLE) {
			TclNewLiteralStringObj(keyObj, "writable");
		    } else {
			TclNewLiteralStringObj(keyObj, "variable");
		    }
		    Tcl_ListObjAppendElement(NULL, listObj, keyObj);
		    Tcl_ListObjAppendElement(NULL, listObj,
			    vwaitItems[i].sourceObj);
		}
	    }
	    if (timeout > 0) {
		TclNewLiteralStringObj(keyObj, "timeleft");
		Tcl_ListObjAppendElement(NULL, listObj, keyObj);
		Tcl_ListObjAppendElement(NULL, listObj,
			Tcl_NewWideIntObj(diff));
	    }
	    Tcl_SetObjResult(interp, listObj);
	} else if (timeout > 0) {
	    Tcl_SetObjResult(interp, Tcl_NewWideIntObj(diff));
	}
    } else {
	result = Tcl_RestoreInterpState(interp, saved);
    }
    if (vwaitItems != localItems) {
	Tcl_Free(vwaitItems);
    }
    return result;
}
```

tcl9.1 9.1.0, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEvent.c`, function `Tcl_VwaitObjCmd`, lines 1536–1906. Full-source SHA-256 `9451a2c540dcd5ed61675c15f833f5ad677b551b61deb210ee6837216f477a54`; snippet SHA-256 `0f13359ad3fe496d90ff98f311f7951d3e6c13a770b2f9b2a8e4fdc9fd49d291`; retained evidence `eaa5f919dc246958bc5d1`.

```text
Tcl_VwaitObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Tcl_Size i, done = 0, numItems = 0, timedOut = 0;
    int foundEvent, any = 1, timeout = 0;
    int extended = 0, result, mode, mask = TCL_ALL_EVENTS;
    Tcl_InterpState saved = NULL;
    Tcl_TimerToken timer = NULL;
    long long before = -1, after;
    Tcl_Channel chan;
    Tcl_WideInt diff = -1;
    VwaitItem localItems[32], *vwaitItems = localItems;
    static const char *const vWaitOptionStrings[] = {
	"-all",	"-extended", "-nofileevents", "-noidleevents",
	"-notimerevents", "-nowindowevents", "-readable",
	"-timeout", "-variable", "-writable", "--", NULL
    };
    enum vWaitOptions {
	OPT_ALL, OPT_EXTD, OPT_NO_FEVTS, OPT_NO_IEVTS,
	OPT_NO_TEVTS, OPT_NO_WEVTS, OPT_READABLE,
	OPT_TIMEOUT, OPT_VARIABLE, OPT_WRITABLE, OPT_LAST
    } index;

    if ((objc == 2) && (strcmp(Tcl_GetString(objv[1]), "--") != 0)) {
	/*
	 * Legacy "vwait" syntax, skip option handling.
	 */
	i = 1;
	goto endOfOptionLoop;
    }

    if ((unsigned) objc - 1 > sizeof(localItems) / sizeof(localItems[0])) {
	vwaitItems = (VwaitItem *)Tcl_Alloc(sizeof(VwaitItem) * (objc - 1));
    }

    for (i = 1; i < objc; i++) {
	const char *name;

	name = TclGetString(objv[i]);
	if (name[0] != '-') {
	    break;
	}
	if (Tcl_GetIndexFromObj(interp, objv[i], vWaitOptionStrings, "option", 0,
		&index) != TCL_OK) {
	    result = TCL_ERROR;
	    goto done;
	}
	switch (index) {
	case OPT_ALL:
	    any = 0;
	    break;
	case OPT_EXTD:
	    extended = 1;
	    break;
	case OPT_NO_FEVTS:
	    mask &= ~TCL_FILE_EVENTS;
	    break;
	case OPT_NO_IEVTS:
	    mask &= ~TCL_IDLE_EVENTS;
	    break;
	case OPT_NO_TEVTS:
	    mask &= ~TCL_TIMER_EVENTS;
	    break;
	case OPT_NO_WEVTS:
	    mask &= ~TCL_WINDOW_EVENTS;
	    break;
	case OPT_TIMEOUT:
	    if (++i >= objc) {
	needArg:
		Tcl_ResetResult(interp);
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"argument required for \"%s\"", vWaitOptionStrings[index]));
		Tcl_SetErrorCode(interp, "TCL", "EVENT", "ARGUMENT", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	    if (Tcl_GetIntFromObj(interp, objv[i], &timeout) != TCL_OK) {
		result = TCL_ERROR;
		goto done;
	    }
	    if (timeout < 0) {
		Tcl_ResetResult(interp);
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"timeout must be positive", -1));
		Tcl_SetErrorCode(interp, "TCL", "EVENT", "NEGTIME", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	    break;
	case OPT_LAST:
	    i++;
	    goto endOfOptionLoop;
	case OPT_VARIABLE:
	    if (++i >= objc) {
		goto needArg;
	    }
	    result = Tcl_TraceVar2(interp, TclGetString(objv[i]), NULL,
		    TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
		    VwaitVarProc, &vwaitItems[numItems]);
	    if (result != TCL_OK) {
		goto done;
	    }
	    vwaitItems[numItems].donePtr = &done;
	    vwaitItems[numItems].sequence = TCL_INDEX_NONE;
	    vwaitItems[numItems].mask = 0;
	    vwaitItems[numItems].sourceObj = objv[i];
	    numItems++;
	    break;
	case OPT_READABLE:
	    if (++i >= objc) {
		goto needArg;
	    }
	    if (TclGetChannelFromObj(interp, objv[i], &chan, &mode, 0)
		    != TCL_OK) {
		result = TCL_ERROR;
		goto done;
	    }
	    if (!(mode & TCL_READABLE)) {
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"channel \"%s\" wasn't open for reading",
			TclGetString(objv[i])));
		result = TCL_ERROR;
		goto done;
	    }
	    Tcl_CreateChannelHandler(chan, TCL_READABLE,
		    VwaitChannelReadProc, &vwaitItems[numItems]);
	    vwaitItems[numItems].donePtr = &done;
	    vwaitItems[numItems].sequence = TCL_INDEX_NONE;
	    vwaitItems[numItems].mask = TCL_READABLE;
	    vwaitItems[numItems].sourceObj = objv[i];
	    numItems++;
	    break;
	case OPT_WRITABLE:
	    if (++i >= objc) {
		goto needArg;
	    }
	    if (TclGetChannelFromObj(interp, objv[i], &chan, &mode, 0)
		    != TCL_OK) {
		result = TCL_ERROR;
		goto done;
	    }
	    if (!(mode & TCL_WRITABLE)) {
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"channel \"%s\" wasn't open for writing",
			TclGetString(objv[i])));
		result = TCL_ERROR;
		goto done;
	    }
	    Tcl_CreateChannelHandler(chan, TCL_WRITABLE,
		    VwaitChannelWriteProc, &vwaitItems[numItems]);
	    vwaitItems[numItems].donePtr = &done;
	    vwaitItems[numItems].sequence = TCL_INDEX_NONE;
	    vwaitItems[numItems].mask = TCL_WRITABLE;
	    vwaitItems[numItems].sourceObj = objv[i];
	    numItems++;
	    break;
	default:
	    TCL_UNREACHABLE();
	}
    }

  endOfOptionLoop:
    if ((mask & (TCL_FILE_EVENTS | TCL_IDLE_EVENTS |
	    TCL_TIMER_EVENTS | TCL_WINDOW_EVENTS)) == 0) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"can't wait: would block forever", -1));
	Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_SOURCES", (char *)NULL);
	result = TCL_ERROR;
	goto done;
    }

    if ((timeout > 0) && ((mask & TCL_TIMER_EVENTS) == 0)) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"timer events disabled with timeout specified", -1));
	Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_TIME", (char *)NULL);
	result = TCL_ERROR;
	goto done;
    }

    for (result = TCL_OK; i < objc; i++) {
	result = Tcl_TraceVar2(interp, TclGetString(objv[i]), NULL,
		TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
		VwaitVarProc, &vwaitItems[numItems]);
	if (result != TCL_OK) {
	    break;
	}
	vwaitItems[numItems].donePtr = &done;
	vwaitItems[numItems].sequence = TCL_INDEX_NONE;
	vwaitItems[numItems].mask = 0;
	vwaitItems[numItems].sourceObj = objv[i];
	numItems++;
    }
    if (result != TCL_OK) {
	result = TCL_ERROR;
	goto done;
    }

    if (!(mask & TCL_FILE_EVENTS)) {
	for (i = 0; i < numItems; i++) {
	    if (vwaitItems[i].mask) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"file events disabled with channel(s) specified", -1));
		Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_FILE_EVENT", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	}
    }

    if (timeout > 0) {
	vwaitItems[numItems].donePtr = &timedOut;
	vwaitItems[numItems].sequence = TCL_INDEX_NONE;
	vwaitItems[numItems].mask = 0;
	vwaitItems[numItems].sourceObj = NULL;
	timer = Tcl_CreateTimerHandler(timeout, VwaitTimeoutProc,
		&vwaitItems[numItems]);
	before = Tcl_GetDayTime();
    } else {
	timeout = 0;
    }

    if ((numItems == 0) && (timeout == 0)) {
	/*
	 * "vwait" is equivalent to "update",
	 * "vwait -nofileevents -notimerevents -nowindowevents"
	 * is equivalent to "update idletasks"
	 */
	any = 1;
	mask |= TCL_DONT_WAIT;
    }

    foundEvent = 1;
    while (!timedOut && foundEvent &&
	    ((!any && (done < numItems)) || (any && !done))) {
	foundEvent = Tcl_DoOneEvent(mask);
	if (Tcl_Canceled(interp, TCL_LEAVE_ERR_MSG) == TCL_ERROR) {
	    break;
	}
	if (Tcl_LimitExceeded(interp)) {
	    Tcl_ResetResult(interp);
	    Tcl_SetObjResult(interp, Tcl_NewStringObj("limit exceeded", -1));
	    Tcl_SetErrorCode(interp, "TCL", "EVENT", "LIMIT", (char *)NULL);
	    break;
	}
	if ((numItems == 0) && (timeout == 0)) {
	    /*
	     * Behavior like "update": clear interpreter's result because
	     * event handlers could have executed commands.
	     */
	    Tcl_ResetResult(interp);
	    result = TCL_OK;
	    goto done;
	}
    }

    if (!foundEvent) {
	Tcl_ResetResult(interp);
	Tcl_SetObjResult(interp, Tcl_NewStringObj((numItems == 0) ?
		"can't wait: would wait forever" :
		"can't wait for variable(s)/channel(s): would wait forever",
		-1));
	Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_SOURCES", (char *)NULL);
	result = TCL_ERROR;
	goto done;
    }

    if (!done && !timedOut) {
	/*
	 * The interpreter's result was already set to the right error message
	 * prior to exiting the loop above.
	 */
	result = TCL_ERROR;
	goto done;
    }

    result = TCL_OK;
    if (timeout <= 0) {
	/*
	 * Clear out the interpreter's result, since it may have been set
	 * by event handlers.
	 */
	Tcl_ResetResult(interp);
	goto done;
    }

    /*
     * When timeout was specified, report milliseconds left or -1 on timeout.
     */
    if (timedOut) {
	diff = -1;
    } else {
	after = Tcl_GetDayTime();
	diff = after / 1000;
	diff -= before / 1000;
	diff = timeout - diff;
	if (diff < 0) {
	    diff = 0;
	}
    }

  done:
    if ((timeout > 0) && (timer != NULL)) {
	Tcl_DeleteTimerHandler(timer);
    }
    if (result != TCL_OK) {
	saved = Tcl_SaveInterpState(interp, result);
    }
    for (i = 0; i < numItems; i++) {
	if (vwaitItems[i].mask & TCL_READABLE) {
	    if (TclGetChannelFromObj(interp, vwaitItems[i].sourceObj,
		    &chan, &mode, 0) == TCL_OK) {
		Tcl_DeleteChannelHandler(chan, VwaitChannelReadProc,
			&vwaitItems[i]);
	    }
	} else if (vwaitItems[i].mask & TCL_WRITABLE) {
	    if (TclGetChannelFromObj(interp, vwaitItems[i].sourceObj,
		    &chan, &mode, 0) == TCL_OK) {
		Tcl_DeleteChannelHandler(chan, VwaitChannelWriteProc,
			&vwaitItems[i]);
	    }
	} else {
	    Tcl_UntraceVar2(interp, TclGetString(vwaitItems[i].sourceObj),
		    NULL, TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
		    VwaitVarProc, &vwaitItems[i]);
	}
    }

    if (result == TCL_OK) {
	if (extended) {
	    Tcl_Size k;
	    Tcl_Obj *listObj, *keyObj;

	    TclNewObj(listObj);
	    for (k = 0; k < done; k++) {
		for (i = 0; i < numItems; i++) {
		    if (vwaitItems[i].sequence != k) {
			continue;
		    }
		    if (vwaitItems[i].mask & TCL_READABLE) {
			TclNewLiteralStringObj(keyObj, "readable");
		    } else if (vwaitItems[i].mask & TCL_WRITABLE) {
			TclNewLiteralStringObj(keyObj, "writable");
		    } else {
			TclNewLiteralStringObj(keyObj, "variable");
		    }
		    Tcl_ListObjAppendElement(NULL, listObj, keyObj);
		    Tcl_ListObjAppendElement(NULL, listObj,
			    vwaitItems[i].sourceObj);
		}
	    }
	    if (timeout > 0) {
		TclNewLiteralStringObj(keyObj, "timeleft");
		Tcl_ListObjAppendElement(NULL, listObj, keyObj);
		Tcl_ListObjAppendElement(NULL, listObj,
			Tcl_NewWideIntObj(diff));
	    }
	    Tcl_SetObjResult(interp, listObj);
	} else if (timeout > 0) {
	    Tcl_SetObjResult(interp, Tcl_NewWideIntObj(diff));
	}
    } else {
	result = Tcl_RestoreInterpState(interp, saved);
    }
    if (vwaitItems != localItems) {
	Tcl_Free(vwaitItems);
    }
    return result;
}
```

jim 0.84-9-g5bac7c9, revision `5bac7c9`, `/workspace/.proofs/native-providers/jimtcl/jim-eventloop.c`, function `JimELVwaitCommand`, lines 565–645. Full-source SHA-256 `b9536f7021919026f8ee2cfcdb35a59305db07a23e60daa50d5da966d52a1a12`; snippet SHA-256 `0309243c6a2a7cca9a072f5c5f7ed82248d291eae31886f251c35eb3f730c45a`; retained evidence `e4521e47b782c5165c471`.

```text
static int JimELVwaitCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    Jim_EventLoop *eventLoop = Jim_CmdPrivData(interp);
    Jim_Obj *oldValue = NULL;
    Jim_Obj *scriptObjPtr = NULL;
    int rc;
    int signal = 0;

    if (argc > 2 && Jim_CompareStringImmediate(interp, argv[1], "-signal")) {
        signal++;
    }

    if (argc - signal == 3) {
        scriptObjPtr = argv[2 + signal];
    }
    else if (argc - signal != 2) {
        return JIM_USAGE;
    }

    oldValue = Jim_GetGlobalVariable(interp, argv[1 + signal], JIM_NONE);

    if (oldValue) {
        Jim_IncrRefCount(oldValue);
    }
    else {
        /* If a result was left, it is an error */
        if (Jim_Length(Jim_GetResult(interp))) {
            return JIM_ERR;
        }
    }

    eventLoop->suppress_bgerror = 0;

    while ((rc = Jim_ProcessEvents(interp, JIM_ALL_EVENTS)) >= 0) {
        Jim_Obj *currValue;

        if (signal && interp->sigmask) {
            /* vwait -signal and handled signals were received, so transfer them
             * to ignored signals so that 'signal check -clear' will return them.
             * It's possible that if signals aren't supported we shouldn't even
             * allow the -signal option.
             */
#ifdef jim_ext_signal
            Jim_SignalSetIgnored(interp->sigmask);
#endif
            interp->sigmask = 0;
            break;
        }

        currValue = Jim_GetGlobalVariable(interp, argv[1 + signal], JIM_NONE);
        /* Stop the loop if the vwait-ed variable changed value,
         * or if was unset and now is set (or the contrary)
         * or if a signal was caught
         */
        if ((oldValue && !currValue) ||
            (!oldValue && currValue) ||
            (oldValue && currValue && !Jim_StringEqObj(oldValue, currValue)) ||
            Jim_CheckSignal(interp)) {
            break;
        }
        if (scriptObjPtr) {
            /* Stop the loop if a provided script returns BREAK or ERR */
            int retval = Jim_EvalObj(interp, scriptObjPtr);
            if (retval == JIM_ERR || retval == JIM_BREAK) {
                if (retval == JIM_ERR) {
                    rc = -2;
                }
                break;
            }
        }
    }
    if (oldValue)
        Jim_DecrRefCount(interp, oldValue);

    if (rc == -2) {
        return JIM_ERR;
    }

    Jim_SetEmptyResult(interp);
    return JIM_OK;
}
```


## Consumer bindings

- [rust/tcl-registry/src/native_vwait/native_observation_tests.rs](../../../../rust/tcl-registry/src/native_vwait/native_observation_tests.rs), `native_vwait::native_observation_tests::original_vwait_basic_boundaries_match_twenty_four_native_source_rows` (linked): Compare 24 retained native ASCII source results and queried provider versions, then independently check selected basic versus lone -- parser purpose and source roles. No raw original argv, options object, event capability or Rust execution result is inferred.

A named test is a coverage binding, not a claim that it executed.

## Replay

No native/Rust launch is performed by this record. Original immutable receipt commands/probe/queue/harness hashes remain available; original capture directory must not be overwritten. Source windows remain independent inspected Event source evidence.
