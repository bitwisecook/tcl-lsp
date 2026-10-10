# naming.list.assignment-literal-versus-alias-target-evaluation

Kind: `native-observation`

## Problem statement

A later lassign target word can read a variable assigned by an earlier target. Ordinary argv evaluation suggests that read must happen before all stores, but a selected native compiler may prepare each target immediately before its store. Applying either interpretation without the actual compilation purpose changes both completion and destination cells. These controls compare a literal command with an alias forcing generic dispatch; they do not prove original name-object conversion, trace continuity or normal-write release closure.

## Question

For lassign {one two} first [set first], does the later target read before the first store or after it, under literal compiled versus alias-forced generic execution, with first initially absent or old?

## Conclusion

C8.5/C8.6/C9.0/C9.1 literal commands perform the first store before evaluating the later target: both initial states succeed with first=one, one=two and old absent. Their interp-alias generic controls evaluate the read before any store: absent first errors with all three targets absent; initial old succeeds with first=one, old=two and one absent. Current Jim literal and Jim native-alias controls both follow the latter behaviour. C8.4 has no lassign. These results concern actual source evaluation and do not grant compiler preparation, normal writes, object conversion or Rust execution authority.

Original list-assignment recipe selection preserves opaque literal target bytes across document and native source channels while keeping bytecode, direct and unknown compilation modes independent.

## Scope

Two immutable exact ASCII source-file programs, fresh native shells, procedure activation per case, literal and actual alias routes, absent/old first states and existence/value of first/old/one. v1 Jim aborts at unsupported Tcl-style interp alias syntax before any lassign case; v2 independently selects its actual alias command and completes all cases. That v1 process failure is retained as a setup limitation, not a lassign answer. BIGIP untested. Source inspection and output observations are distinct.

Selection cannot establish alias-target evaluation, result/cell identity, a reached assignment, physical frame or rewrite equivalence. The original literal-versus-alias target experiment remains independent.

## Provider answers

### tcl8.4

Status: `unsupported`. Version: 8.4.20. Build: Original shell ELF SHA-256 f0e742ccc08a150afd9a0a236ce24c8e12098dae38c0efd46e59f9b1a28bf58a; exact header/static library/Makefile/source-owner hashes, startup version and raw process streams retained.. Channel: Exact ASCII source file argument to the actual native shell; file character ingress. Separate ASCII stdin startup version query.. Dialect: Tcl.

Actual available query is zero, all four controls explicitly unavailable; no lassign invocation.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Original shell ELF SHA-256 e7eb17821aeb7b1f84619f431e89ecff667bc5461fa8e00d67498b6b7d5dd61a; exact header/static library/Makefile/source-owner hashes, startup version and raw process streams retained.. Channel: Exact ASCII source file argument to the actual native shell; file character ingress. Separate ASCII stdin startup version query.. Dialect: Tcl.

Literal both initial states succeed: first=one, one=two, old absent. Generic alias absent state errors without stores; initialized state succeeds with first=one, old=two, one absent.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Original shell ELF SHA-256 9b156375d068a58fcfb82a2a0afd170afd2b03912cbb6e588676c7688118c8ff; exact header/static library/Makefile/source-owner hashes, startup version and raw process streams retained.. Channel: Exact ASCII source file argument to the actual native shell; file character ingress. Separate ASCII stdin startup version query.. Dialect: Tcl.

Literal both initial states succeed: first=one, one=two, old absent. Generic alias absent state errors without stores; initialized state succeeds with first=one, old=two, one absent.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Original shell ELF SHA-256 f4748357cdbf2762f1c5c05916fa53e54718d087120ae9de9c403bbdfaf764b1; exact header/static library/Makefile/source-owner hashes, startup version and raw process streams retained.. Channel: Exact ASCII source file argument to the actual native shell; file character ingress. Separate ASCII stdin startup version query.. Dialect: Tcl.

Literal both initial states succeed: first=one, one=two, old absent. Generic alias absent state errors without stores; initialized state succeeds with first=one, old=two, one absent.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Original shell ELF SHA-256 2a82f721b68f2ed2347ca4e3daa9a56c49c9e3efce77d00f22fc3b2012d8814d; exact header/static library/Makefile/source-owner hashes, startup version and raw process streams retained.. Channel: Exact ASCII source file argument to the actual native shell; file character ingress. Separate ASCII stdin startup version query.. Dialect: Tcl.

Literal both initial states succeed: first=one, one=two, old absent. Generic alias absent state errors without stores; initialized state succeeds with first=one, old=two, one absent.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Original shell ELF SHA-256 e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806; exact header/static library/Makefile/source-owner hashes, startup version and raw process streams retained.. Channel: Exact ASCII source file argument to the actual native shell; file character ingress. Separate ASCII stdin startup version query.. Dialect: Jim Tcl.

Literal and native alias both evaluate the later target before stores: absent first errors/no targets; old first succeeds with first=one, old=two, one absent. The independent v1 Tcl-style interp-alias setup exits1 before any case.

### bigip

Status: `not-tested`. Version: not tested. Build: not recorded. Channel: not tested. Dialect: F5 iRules.

No appliance comparison for these exact source contexts.

## Exact evidence

- `v1-source` (input): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/cases.tcl](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/cases.tcl). SHA-256 `92211b594dbcc25c5aef18af27b8f0df27fadbd10176513d0fe7e01dc42ec030`. Exact ASCII file source, including selected alias door and four procedure activations.
- `v1-aggregate` (provider): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/receipt.json](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/receipt.json). SHA-256 `463323f59dc93d8e7b923fe6e8c53dccb936a65ca497498fbe63366d136ac57c`. Original complete six-provider build/startup/source/process correspondence.
- `v1-tcl8.4-receipt.json` (provider): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/tcl8.4/receipt.json](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/tcl8.4/receipt.json). SHA-256 `256e8b20ebb53ba213d7025bda1516b43cfa88240b70950f7f1feb6f82dcd23c`. Original native stream/process or exact provider/build/startup/source correspondence.
- `v1-tcl8.4-stdout` (observation): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/tcl8.4/stdout](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/tcl8.4/stdout). SHA-256 `546d45b3157bc8fe347b5c21f7ff73e852afde180265130bdb741503a50c446d`. Original native stream/process or exact provider/build/startup/source correspondence.
- `v1-tcl8.4-stderr` (observation): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/tcl8.4/stderr](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/tcl8.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original native stream/process or exact provider/build/startup/source correspondence.
- `v1-tcl8.5-receipt.json` (provider): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/tcl8.5/receipt.json](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/tcl8.5/receipt.json). SHA-256 `71f51c7e60f6f99767fe9b484f08d941de1488651939a14e2ad542dff6119f41`. Original native stream/process or exact provider/build/startup/source correspondence.
- `v1-tcl8.5-stdout` (observation): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/tcl8.5/stdout](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/tcl8.5/stdout). SHA-256 `c8f3d6e4cb0b728b143e839bfc1abc29f281f23ff368736c9cd0e11875ed9393`. Original native stream/process or exact provider/build/startup/source correspondence.
- `v1-tcl8.5-stderr` (observation): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/tcl8.5/stderr](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/tcl8.5/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original native stream/process or exact provider/build/startup/source correspondence.
- `v1-tcl8.6-receipt.json` (provider): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/tcl8.6/receipt.json](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/tcl8.6/receipt.json). SHA-256 `0860e200928a87578eeeed20e7c30e8253caea72a0a56e3861f199e9563fda16`. Original native stream/process or exact provider/build/startup/source correspondence.
- `v1-tcl8.6-stdout` (observation): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/tcl8.6/stdout](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/tcl8.6/stdout). SHA-256 `b8b6ce8d0187c6f723db960929a2eac18259cc8133298fb2028044674cc531c3`. Original native stream/process or exact provider/build/startup/source correspondence.
- `v1-tcl8.6-stderr` (observation): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/tcl8.6/stderr](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/tcl8.6/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original native stream/process or exact provider/build/startup/source correspondence.
- `v1-tcl9.0-receipt.json` (provider): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/tcl9.0/receipt.json](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/tcl9.0/receipt.json). SHA-256 `54e81c10a09df971d300d0e027412862da7cb37faf17ebb77ab4282dd4b26a16`. Original native stream/process or exact provider/build/startup/source correspondence.
- `v1-tcl9.0-stdout` (observation): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/tcl9.0/stdout](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/tcl9.0/stdout). SHA-256 `b4a96df1def354d4ceda09c45337074d65000c713b397ff27446012df35ce5a6`. Original native stream/process or exact provider/build/startup/source correspondence.
- `v1-tcl9.0-stderr` (observation): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/tcl9.0/stderr](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/tcl9.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original native stream/process or exact provider/build/startup/source correspondence.
- `v1-tcl9.1-receipt.json` (provider): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/tcl9.1/receipt.json](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/tcl9.1/receipt.json). SHA-256 `ba976f1b2bee0e3e78e255ccd37a790a6f6404f377e119540d4d71a855731d96`. Original native stream/process or exact provider/build/startup/source correspondence.
- `v1-tcl9.1-stdout` (observation): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/tcl9.1/stdout](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/tcl9.1/stdout). SHA-256 `3d5d24dedf56f69193a9ddb87fa5cb6aa6429ad499023dafdddfc22befb9d657`. Original native stream/process or exact provider/build/startup/source correspondence.
- `v1-tcl9.1-stderr` (observation): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/tcl9.1/stderr](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/tcl9.1/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original native stream/process or exact provider/build/startup/source correspondence.
- `v1-jim-receipt.json` (provider): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/jim/receipt.json](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/jim/receipt.json). SHA-256 `68c60f52a0dd72ee8911a6ae9d40ef0997328efae86767605459172f25ae1a8e`. Original exit and complete raw stream; v1 Jim exits before lassign controls at the unsupported Tcl interp-alias syntax.
- `v1-jim-stdout` (observation): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/jim/stdout](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/jim/stdout). SHA-256 `79e3518cf6e3b780f757301630cddd54454ea89983b9a54f91a82f171a5e4270`. Original exit and complete raw stream; v1 Jim exits before lassign controls at the unsupported Tcl interp-alias syntax.
- `v1-jim-stderr` (limitation): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/jim/stderr](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1/jim/stderr). SHA-256 `6a4edd09965de8e44738b3e0459fa1c99647dd6c6bdafefbda3813d9ffbf27fc`. Original exit and complete raw stream; v1 Jim exits before lassign controls at the unsupported Tcl interp-alias syntax.
- `v2-source` (input): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/cases.tcl](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/cases.tcl). SHA-256 `d6d27c3709c486bcaf6a4a5550d502f7090dd8d1bbc07660144fda3fbccd51bb`. Exact ASCII file source, including selected alias door and four procedure activations.
- `v2-aggregate` (provider): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/receipt.json](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/receipt.json). SHA-256 `caf301f70874f5d7d4701f72bb33d3e1d820d4f4eb8eb056f0485bf51987541a`. Original complete six-provider build/startup/source/process correspondence.
- `v2-tcl8.4-receipt.json` (provider): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/tcl8.4/receipt.json](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/tcl8.4/receipt.json). SHA-256 `596599ce1b6d36a92062ec28ae5741007e5a3e608755fe66b7410e1b37bec90e`. Original native stream/process or exact provider/build/startup/source correspondence.
- `v2-tcl8.4-stdout` (observation): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/tcl8.4/stdout](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/tcl8.4/stdout). SHA-256 `546d45b3157bc8fe347b5c21f7ff73e852afde180265130bdb741503a50c446d`. Original native stream/process or exact provider/build/startup/source correspondence.
- `v2-tcl8.4-stderr` (observation): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/tcl8.4/stderr](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/tcl8.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original native stream/process or exact provider/build/startup/source correspondence.
- `v2-tcl8.5-receipt.json` (provider): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/tcl8.5/receipt.json](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/tcl8.5/receipt.json). SHA-256 `7171a944ffa814a320ff2b1131e3f89357bebfce114ce86305e9d889f83f31e7`. Original native stream/process or exact provider/build/startup/source correspondence.
- `v2-tcl8.5-stdout` (observation): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/tcl8.5/stdout](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/tcl8.5/stdout). SHA-256 `67e97718ec390a6f4c6eb1d5d82589899c1706dfa08da1d05189eeac8253c8aa`. Original native stream/process or exact provider/build/startup/source correspondence.
- `v2-tcl8.5-stderr` (observation): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/tcl8.5/stderr](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/tcl8.5/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original native stream/process or exact provider/build/startup/source correspondence.
- `v2-tcl8.6-receipt.json` (provider): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/tcl8.6/receipt.json](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/tcl8.6/receipt.json). SHA-256 `4901a3658ce35a06e52b5a6047a43fcd7ddebc73ecc473d8d44a0b8ea9a68fbf`. Original native stream/process or exact provider/build/startup/source correspondence.
- `v2-tcl8.6-stdout` (observation): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/tcl8.6/stdout](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/tcl8.6/stdout). SHA-256 `93387d6d5d1c30873b46f05425aa5109d32a43f64979f410d789d0486c2cc86c`. Original native stream/process or exact provider/build/startup/source correspondence.
- `v2-tcl8.6-stderr` (observation): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/tcl8.6/stderr](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/tcl8.6/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original native stream/process or exact provider/build/startup/source correspondence.
- `v2-tcl9.0-receipt.json` (provider): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/tcl9.0/receipt.json](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/tcl9.0/receipt.json). SHA-256 `1f4e157025587a3eb4ef45a4433d73cf1f5025453ec31a562c12ca37fe85574b`. Original native stream/process or exact provider/build/startup/source correspondence.
- `v2-tcl9.0-stdout` (observation): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/tcl9.0/stdout](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/tcl9.0/stdout). SHA-256 `41a961ef33055b5385e996e1d8618b57edf2d363e4a023c8861137b08b615fdd`. Original native stream/process or exact provider/build/startup/source correspondence.
- `v2-tcl9.0-stderr` (observation): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/tcl9.0/stderr](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/tcl9.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original native stream/process or exact provider/build/startup/source correspondence.
- `v2-tcl9.1-receipt.json` (provider): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/tcl9.1/receipt.json](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/tcl9.1/receipt.json). SHA-256 `fb1bcc8fe0b6b4dcf1bb8a2c498abd5b15cf91b4b558db108dac8fbafab2ba89`. Original native stream/process or exact provider/build/startup/source correspondence.
- `v2-tcl9.1-stdout` (observation): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/tcl9.1/stdout](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/tcl9.1/stdout). SHA-256 `957f6d8fd7742da77ab92ff7fedee044a9516b5935482d722da3d15441bfffca`. Original native stream/process or exact provider/build/startup/source correspondence.
- `v2-tcl9.1-stderr` (observation): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/tcl9.1/stderr](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/tcl9.1/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original native stream/process or exact provider/build/startup/source correspondence.
- `v2-jim-receipt.json` (provider): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/jim/receipt.json](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/jim/receipt.json). SHA-256 `6dd62f5680ce6689e91baf2e8b646cb9154b98be57bab38a2969bf1057dbf6ec`. Original native stream/process or exact provider/build/startup/source correspondence.
- `v2-jim-stdout` (observation): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/jim/stdout](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/jim/stdout). SHA-256 `c62f3b816e00d45cffd45f7456f94fb38b21ee83526ad888a6527cf158667247`. Original native stream/process or exact provider/build/startup/source correspondence.
- `v2-jim-stderr` (observation): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/jim/stderr](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original native stream/process or exact provider/build/startup/source correspondence.
- `tcl8.5-compiler-source` (source-anchor): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/source-anchors/tcl8.5.json](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/source-anchors/tcl8.5.json). SHA-256 `eb1a8ad6d0de0d0f9da1e090e48b6715c6b2aa69eadbcb5d0e599a2884c8fd63`. JSON pointer `/snippet`. Pinned selected compiler loop visits a target then stores before visiting the next target, or Jim receives the already evaluated argv; not native execution evidence.
- `tcl8.6-compiler-source` (source-anchor): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/source-anchors/tcl8.6.json](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/source-anchors/tcl8.6.json). SHA-256 `73c7c4b3fe9d42242fb4f1385bdcb37d2d868a8b0dd20047edbc927fcb334da8`. JSON pointer `/snippet`. Pinned selected compiler loop visits a target then stores before visiting the next target, or Jim receives the already evaluated argv; not native execution evidence.
- `tcl9.0-compiler-source` (source-anchor): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/source-anchors/tcl9.0.json](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/source-anchors/tcl9.0.json). SHA-256 `b7c68842437cdad2a78ca600b1a9e4d11f83a2a3b0e9bd787712247b1d31ea35`. JSON pointer `/snippet`. Pinned selected compiler loop visits a target then stores before visiting the next target, or Jim receives the already evaluated argv; not native execution evidence.
- `tcl9.1-compiler-source` (source-anchor): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/source-anchors/tcl9.1.json](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/source-anchors/tcl9.1.json). SHA-256 `f19f411dd5d46a6017981b9d40a2985df61851cb1dfb8df6f9319e23f77f9ec2`. JSON pointer `/snippet`. Pinned selected compiler loop visits a target then stores before visiting the next target, or Jim receives the already evaluated argv; not native execution evidence.
- `jim-compiler-source` (source-anchor): [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/source-anchors/jim.json](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/source-anchors/jim.json). SHA-256 `043f1853eb9acead3e272f2d7e2ae9f5fb5d4de18add113d5ddd23d03634a2f5`. JSON pointer `/snippet`. Pinned selected compiler loop visits a target then stores before visiting the next target, or Jim receives the already evaluated argv; not native execution evidence.

## Source inspection

tcl8.5 8.5.19, revision `Pinned release source / exact original full-file SHA-256`, `generic/tclCompCmds.c`, function `TclCompileLassignCmd`, lines 2477–2571. Full-source SHA-256 `117274255882a35828318ccbfab476c180622310187c5e3a4796867b48ba7414`; snippet SHA-256 `0a4c7c4cf69dca253ca19282f8fe8735a9a46e34965b825c640b9217ed3d6f78`; retained evidence `tcl8.5-compiler-source`.

```text
TclCompileLassignCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to defintion of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    Tcl_Token *tokenPtr;
    int simpleVarName, isScalar, localIndex, numWords, idx;
    DefineLineInformation;	/* TIP #280 */

    numWords = parsePtr->numWords;

    /*
     * Check for command syntax error, but we'll punt that to runtime.
     */

    if (numWords < 3) {
	return TCL_ERROR;
    }

    /*
     * Generate code to push list being taken apart by [lassign].
     */

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    CompileWord(envPtr, tokenPtr, interp, 1);

    /*
     * Generate code to assign values from the list to variables.
     */

    for (idx=0 ; idx<numWords-2 ; idx++) {
	tokenPtr = TokenAfter(tokenPtr);

	/*
	 * Generate the next variable name.
	 */

	PushVarNameWord(interp, tokenPtr, envPtr, TCL_CREATE_VAR, &localIndex,
			&simpleVarName, &isScalar, idx+2);

	/*
	 * Emit instructions to get the idx'th item out of the list value on
	 * the stack and assign it to the variable.
	 */

	if (simpleVarName) {
	    if (isScalar) {
		if (localIndex >= 0) {
		    TclEmitOpcode(INST_DUP, envPtr);
		    TclEmitInstInt4(INST_LIST_INDEX_IMM, idx, envPtr);
		    if (localIndex <= 255) {
			TclEmitInstInt1(INST_STORE_SCALAR1,localIndex,envPtr);
		    } else {
			TclEmitInstInt4(INST_STORE_SCALAR4,localIndex,envPtr);
		    }
		} else {
		    TclEmitInstInt4(INST_OVER, 1, envPtr);
		    TclEmitInstInt4(INST_LIST_INDEX_IMM, idx, envPtr);
		    TclEmitOpcode(INST_STORE_SCALAR_STK, envPtr);
		}
	    } else {
		if (localIndex >= 0) {
		    TclEmitInstInt4(INST_OVER, 1, envPtr);
		    TclEmitInstInt4(INST_LIST_INDEX_IMM, idx, envPtr);
		    if (localIndex <= 255) {
			TclEmitInstInt1(INST_STORE_ARRAY1, localIndex, envPtr);
		    } else {
			TclEmitInstInt4(INST_STORE_ARRAY4, localIndex, envPtr);
		    }
		} else {
		    TclEmitInstInt4(INST_OVER, 2, envPtr);
		    TclEmitInstInt4(INST_LIST_INDEX_IMM, idx, envPtr);
		    TclEmitOpcode(INST_STORE_ARRAY_STK, envPtr);
		}
	    }
	} else {
	    TclEmitInstInt4(INST_OVER, 1, envPtr);
	    TclEmitInstInt4(INST_LIST_INDEX_IMM, idx, envPtr);
	    TclEmitOpcode(INST_STORE_STK, envPtr);
	}
	TclEmitOpcode(INST_POP, envPtr);
    }

    /*
     * Generate code to leave the rest of the list on the stack.
     */

    TclEmitInstInt4(INST_LIST_RANGE_IMM, idx, envPtr);
    TclEmitInt4(-2, envPtr);	/* -2 == "end" */

    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `Pinned release source / exact original full-file SHA-256`, `generic/tclCompCmdsGR.c`, function `TclCompileLassignCmd`, lines 968–1051. Full-source SHA-256 `edda78b970816ff347e08c5f2f957c42b23915bc4fc8410d9d2cfdedba162d8d`; snippet SHA-256 `ef8fcd3d2677f9db53c8dd4847a5525d478f12482217997e1f1e3d999ebfce85`; retained evidence `tcl8.6-compiler-source`.

```text
TclCompileLassignCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    int isScalar, localIndex, numWords, idx;

    numWords = parsePtr->numWords;

    /*
     * Check for command syntax error, but we'll punt that to runtime.
     */

    if (numWords < 3) {
	return TCL_ERROR;
    }

    /*
     * Generate code to push list being taken apart by [lassign].
     */

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    CompileWord(envPtr, tokenPtr, interp, 1);

    /*
     * Generate code to assign values from the list to variables.
     */

    for (idx=0 ; idx<numWords-2 ; idx++) {
	tokenPtr = TokenAfter(tokenPtr);

	/*
	 * Generate the next variable name.
	 */

	PushVarNameWord(interp, tokenPtr, envPtr, 0, &localIndex,
		&isScalar, idx + 2);

	/*
	 * Emit instructions to get the idx'th item out of the list value on
	 * the stack and assign it to the variable.
	 */

	if (isScalar) {
	    if (localIndex >= 0) {
		TclEmitOpcode(	INST_DUP,			envPtr);
		TclEmitInstInt4(INST_LIST_INDEX_IMM, idx,	envPtr);
		Emit14Inst(	INST_STORE_SCALAR, localIndex,	envPtr);
		TclEmitOpcode(	INST_POP,			envPtr);
	    } else {
		TclEmitInstInt4(INST_OVER, 1,			envPtr);
		TclEmitInstInt4(INST_LIST_INDEX_IMM, idx,	envPtr);
		TclEmitOpcode(	INST_STORE_STK,			envPtr);
		TclEmitOpcode(	INST_POP,			envPtr);
	    }
	} else {
	    if (localIndex >= 0) {
		TclEmitInstInt4(INST_OVER, 1,			envPtr);
		TclEmitInstInt4(INST_LIST_INDEX_IMM, idx,	envPtr);
		Emit14Inst(	INST_STORE_ARRAY, localIndex,	envPtr);
		TclEmitOpcode(	INST_POP,			envPtr);
	    } else {
		TclEmitInstInt4(INST_OVER, 2,			envPtr);
		TclEmitInstInt4(INST_LIST_INDEX_IMM, idx,	envPtr);
		TclEmitOpcode(	INST_STORE_ARRAY_STK,		envPtr);
		TclEmitOpcode(	INST_POP,			envPtr);
	    }
	}
    }

    /*
     * Generate code to leave the rest of the list on the stack.
     */

    TclEmitInstInt4(		INST_LIST_RANGE_IMM, idx,	envPtr);
    TclEmitInt4(			TCL_INDEX_END,		envPtr);

    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `Pinned release source / exact original full-file SHA-256`, `generic/tclCompCmdsGR.c`, function `TclCompileLassignCmd`, lines 953–1035. Full-source SHA-256 `d0a2ad66de4a067375d2486e682a6d66e10ee08e2e91603af2fab63fc397d1db`; snippet SHA-256 `8b94a931b5a686607a08b1bf39f6c3870cfaaf8e0eb55dcc9716f4a9e84d32a4`; retained evidence `tcl9.0-compiler-source`.

```text
TclCompileLassignCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    int isScalar, localIndex, numWords, idx;

    numWords = parsePtr->numWords;

    /*
     * Check for command syntax error, but we'll punt that to runtime.
     */

    if (numWords < 3) {
	return TCL_ERROR;
    }

    /*
     * Generate code to push list being taken apart by [lassign].
     */

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    CompileWord(envPtr, tokenPtr, interp, 1);

    /*
     * Generate code to assign values from the list to variables.
     */

    for (idx=0 ; idx<numWords-2 ; idx++) {
	tokenPtr = TokenAfter(tokenPtr);

	/*
	 * Generate the next variable name.
	 */

	PushVarNameWord(interp, tokenPtr, envPtr, 0, &localIndex,
		&isScalar, idx + 2);

	/*
	 * Emit instructions to get the idx'th item out of the list value on
	 * the stack and assign it to the variable.
	 */

	if (isScalar) {
	    if (localIndex >= 0) {
		TclEmitOpcode(	INST_DUP,			envPtr);
		TclEmitInstInt4(INST_LIST_INDEX_IMM, idx,	envPtr);
		Emit14Inst(	INST_STORE_SCALAR, localIndex,	envPtr);
		TclEmitOpcode(	INST_POP,			envPtr);
	    } else {
		TclEmitInstInt4(INST_OVER, 1,			envPtr);
		TclEmitInstInt4(INST_LIST_INDEX_IMM, idx,	envPtr);
		TclEmitOpcode(	INST_STORE_STK,			envPtr);
		TclEmitOpcode(	INST_POP,			envPtr);
	    }
	} else {
	    if (localIndex >= 0) {
		TclEmitInstInt4(INST_OVER, 1,			envPtr);
		TclEmitInstInt4(INST_LIST_INDEX_IMM, idx,	envPtr);
		Emit14Inst(	INST_STORE_ARRAY, localIndex,	envPtr);
		TclEmitOpcode(	INST_POP,			envPtr);
	    } else {
		TclEmitInstInt4(INST_OVER, 2,			envPtr);
		TclEmitInstInt4(INST_LIST_INDEX_IMM, idx,	envPtr);
		TclEmitOpcode(	INST_STORE_ARRAY_STK,		envPtr);
		TclEmitOpcode(	INST_POP,			envPtr);
	    }
	}
    }

    /*
     * Generate code to leave the rest of the list on the stack.
     */

    TclEmitInstInt4(		INST_LIST_RANGE_IMM, idx,	envPtr);
    TclEmitInt4(			(int)TCL_INDEX_END,		envPtr);

    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `Pinned release source / exact original full-file SHA-256`, `generic/tclCompCmdsGR.c`, function `TclCompileLassignCmd`, lines 996–1079. Full-source SHA-256 `3656eddb1d634083343f182212b4874aa0939940382cf20252a171b155305fd1`; snippet SHA-256 `8fa26a16386e530ee1a91f14037cddabf3153bb7ede358663763f11cd13daa6a`; retained evidence `tcl9.1-compiler-source`.

```text
TclCompileLassignCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    int isScalar;
    Tcl_Size numWords = parsePtr->numWords, idx;
    Tcl_LVTIndex localIndex;
    /* TODO: Consider support for compiling expanded args. */

    /*
     * Check for command syntax error, but we'll punt that to runtime.
     */

    if (numWords < 3 || OutOfUintRange(numWords)) {
	return TCL_ERROR;
    }

    /*
     * Generate code to push list being taken apart by [lassign].
     */

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    PUSH_TOKEN(			tokenPtr, 1);

    /*
     * Generate code to assign values from the list to variables.
     */

    for (idx=0 ; idx<numWords-2 ; idx++) {
	/*
	 * Generate the next variable name.
	 */

	tokenPtr = TokenAfter(tokenPtr);
	PushVarNameWord(tokenPtr, 0, &localIndex, &isScalar, idx + 2);
	if (OutOfUintRangeUpper(localIndex)) {
	    return TCL_ERROR;
	}

	/*
	 * Emit instructions to get the idx'th item out of the list value on
	 * the stack and assign it to the variable.
	 */

	if (isScalar) {
	    if (localIndex >= 0) {
		OP(		DUP);
		OP4(		LIST_INDEX_IMM, idx);
		OP4(		STORE_SCALAR, localIndex);
		OP(		POP);
	    } else {
		OP4(		OVER, 1);
		OP4(		LIST_INDEX_IMM, idx);
		OP(		STORE_STK);
		OP(		POP);
	    }
	} else {
	    if (localIndex >= 0) {
		OP4(		OVER, 1);
		OP4(		LIST_INDEX_IMM, idx);
		OP4(		STORE_ARRAY, localIndex);
		OP(		POP);
	    } else {
		OP4(		OVER, 2);
		OP4(		LIST_INDEX_IMM, idx);
		OP(		STORE_ARRAY_STK);
		OP(		POP);
	    }
	}
    }

    /*
     * Generate code to leave the rest of the list on the stack.
     */

    OP44(		LIST_RANGE_IMM, idx, TCL_INDEX_END);

    return TCL_OK;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `jim.c`, function `Jim_LassignCoreCommand`, lines 13245–13270. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `ee04785cafd71b47e3164e0d24252dc257f3df67c2e72cd65503fc26c2f5ca5e`; retained evidence `jim-compiler-source`.

```text
static int Jim_LassignCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    int result = JIM_ERR;
    int i;
    Jim_ListIter iter;
    Jim_Obj *resultObj;

    JimListIterInit(&iter, argv[1]);

    for (i = 2; i < argc; i++) {
        Jim_Obj *valObj = JimListIterNext(interp, &iter);
        result = Jim_SetVariable(interp, argv[i], valObj ? valObj : interp->emptyObj);
        if (result != JIM_OK) {
            return result;
        }
    }

    resultObj = Jim_NewListObj(interp, NULL, 0);
    while (!JimListIterDone(interp, &iter)) {
        Jim_ListAppendElement(interp, resultObj, JimListIterNext(interp, &iter));
    }

    Jim_SetResult(interp, resultObj);

    return JIM_OK;
}

```


## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `NativeCompilationSpec::select_native_words`: Select the recipe from authentic captured source words plus explicit dialect/mode/context; missing premises retain Generic or Unknown rather than a donated compiler entry.
- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `native_compilation::tests::original_assignment_selection_keeps_opaque_target_bytes_and_modes_separate` (linked): Original assignment selection retains literal document/native opaque target bytes and exact compiler mode: supported bytecode selection, direct generic execution and missing/unknown context remain separate without assuming alias target evaluation equivalence. This is the current software/API definition; no assertion outcome or new original-provider observation is attached to this binding.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-vm/tests/data/native_lassign_argv_evaluation/replay.py",
  "--c-root",
  "tmp",
  "--jim-root",
  "/workspace/.proofs/native-providers/jimtcl",
  "--output",
  "/tmp/native-lassign-argv-reconfirmation"
]
```

Requires exact retained source, executable/header/library/Makefile/source-owner and raw-stream hashes. Compares process exit and complete stdout/stderr. v1 Jim exact error path requires its original source-file argument to exist with the retained bytes; v2 is path-independent on success. --verify-only checks original bytes without launching native code. Guest errors and unsupported controls are expected observations; no compiler/normal/write or Rust pass is inferred.
