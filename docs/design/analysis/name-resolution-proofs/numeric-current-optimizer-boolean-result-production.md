# naming.numeric.current-optimizer-boolean-result-production

Kind: `implementation-contract` (current source interpretation; all external execution statuses not tested).

## Problem statement

A reached numeric instruction must execute conversion. An inline Boolean condition may have that instruction removed before execution. Applying one result recipe to both changes original cache/String effects; a public expression API has its own independent result producer.

## Question

Which current configured C Tcl sources select numeric-conversion removal before a Boolean jump, and how does that source-level rule distinguish a reached TRY_CVT_TO_NUMERIC from inline and public-API result production?

## Conclusion

In the inspected configured C8.6.18/9.0.4/9.1.0 source, TclOptimizeBytecode blanks TRY_CVT_TO_NUMERIC to NOP when the next instruction is one of its listed conditional jumps or other listed consumers. The source separately installs the optimizer in Tcl_CreateInterp and calls it from TclSetByteCodeFromAny. C9.1 guards deprecated short jump opcodes with REMOVE_DEPRECATED_OPCODES. A genuinely reached numeric instruction therefore has a distinct current implementation purpose from an inline condition whose source recipe removes conversion; public API result production remains independent. C8.4/C8.5 selected tclOptimize.c absence establishes only that file fact. No Native or Rust execution answers this source interpretation question.

## Scope

Exactly five configured source-tree states with full retained headers, compile/basic/expression sources, Makefile and tclConfig.sh, three whole optimizer files and nine exact rewrite/install/call excerpts. Version macros and independently captured original465 CLI version bytes are retained and matched; each source/header/config/library hash explicitly records whether it was already an original465 launch input. Optimizer files and tclCompile.c are current separately retained source and are not added retroactively to original465. Five existing static-library identities are byte-verified by hash/size and retained as references in the unchanged original465 input receipts; no rebuild/source-to-library causal equivalence is inferred. No opcode listing, Native compiler/execution result, original object/header/cache/frame/issuer, absence of optimisation elsewhere, Jim or BIG-IP behavior or completed software assertion follows. All seven external execution statuses remain not-tested.

The [independent original logical-site observations](numeric-original-logical-operand-truth-sites.md) and [primitive versus expression API observations](numeric-original-primitive-boolean-vs-expression-truth.md) preserve their actual route/cache/result purpose. No source window is added to their original launch inputs.

## Provider answers

| Provider | Version/source state | Execution status | Source answer and strict limit |
| --- | --- | --- | --- |
| `tcl8.4` | 8.4.20 | `not-tested` | Selected tclOptimize.c is absent in the inspected configured tree. This source-file fact supplies no opcode, compile, execution or absence-of-optimisation conclusion. |
| `tcl8.5` | 8.5.19 | `not-tested` | Selected tclOptimize.c is absent in the inspected configured tree. This source-file fact supplies no opcode, compile, execution or absence-of-optimisation conclusion. |
| `tcl8.6` | 8.6.18 | `not-tested` | The retained current source blanks TRY_CVT_TO_NUMERIC before its listed conditional jumps/consumers and separately installs/calls this optimizer; exact macro/config/whole-file state is retained. This is source interpretation only, without measured execution/opcode or source-to-library equivalence. |
| `tcl9.0` | 9.0.4 | `not-tested` | The retained current source blanks TRY_CVT_TO_NUMERIC before its listed conditional jumps/consumers and separately installs/calls this optimizer; exact macro/config/whole-file state is retained. This is source interpretation only, without measured execution/opcode or source-to-library equivalence. |
| `tcl9.1` | 9.1.0 | `not-tested` | The retained current source blanks TRY_CVT_TO_NUMERIC before its listed conditional jumps/consumers and separately installs/calls this optimizer; exact macro/config/whole-file state is retained. This is source interpretation only, without measured execution/opcode or source-to-library equivalence. |
| `jim` | not inspected for this source question | `not-tested` | No source inspection or execution answers this Jim/BIG-IP optimizer question. |
| `bigip` | not inspected for this source question | `not-tested` | No source inspection or execution answers this Jim/BIG-IP optimizer question. |

## Exact evidence

- `optimizer-current-request` (input): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/request.json](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/request.json). SHA-256 `b228c1ae1c97e968776ec67791d2c2a24317298fefbf798e05fa0ac774d19df1`. Immutable source-interpretation question, exact versions/mappings/absence/windows and explicit zero launches.
- `optimizer-current-preparer` (input): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/prepare.py](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/prepare.py). SHA-256 `d2732fe9a4491fa2e4fe8279b2559049d38aeee1148770cc69605fd32ddb6b25`. Complete original read-only preparation program; no Native or Rust launch, and no source-to-library build equivalence.
- `optimizer-current-retention` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/source-retention-manifest.json](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/source-retention-manifest.json). SHA-256 `b5f76ed5a93db8b2c413ca2ed25220016a03975d9a858fcf70ee40b9f7252b08`. Exact retained whole-file/window hashes and library references; original record is unchanged.
- `optimizer-current-file-0` (provider): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.4.20/context/original465-cli-version.stdout](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.4.20/context/original465-cli-version.stdout). SHA-256 `ede2f9015aff14bc8edd1fd1b2ec6ecc3dc7b6a649207bbb9c4c2f7aa18b31a9`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-1` (provider): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.4.20/context/original465-receipt.json](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.4.20/context/original465-receipt.json). SHA-256 `0a92bce78100ac13b0b8df1015014466fef913b7919c6b97bd3f814767c565b4`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-2` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.4.20/sources/generic/tcl.h](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.4.20/sources/generic/tcl.h). SHA-256 `824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-3` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.4.20/sources/generic/tclBasic.c](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.4.20/sources/generic/tclBasic.c). SHA-256 `cab6d4decd1a3e365fe3cef49a2d5430dfef01cd775dd7d34f98064032f3a81a`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-4` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.4.20/sources/generic/tclCompExpr.c](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.4.20/sources/generic/tclCompExpr.c). SHA-256 `c4138e547b833c39d0c6da3bb4042fc7a7052a6146a2e7635aedcd53ac492514`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-5` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.4.20/sources/generic/tclCompile.c](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.4.20/sources/generic/tclCompile.c). SHA-256 `0bc55b283d6cb62a4298d4dc30e050b587d5d1b4e7f145e4f644e237dd7639f8`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-6` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.4.20/sources/generic/tclCompile.h](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.4.20/sources/generic/tclCompile.h). SHA-256 `3d7d3b604522a9c5e673076fab8df53316951da1406892d03fe52dd678e2d0ec`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-7` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.4.20/sources/generic/tclInt.h](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.4.20/sources/generic/tclInt.h). SHA-256 `f51c8e66c562de424b84a35a04b6b86d9a135ef7d4ca891b35c3b0586bc0ce1a`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-8` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.4.20/sources/unix/Makefile](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.4.20/sources/unix/Makefile). SHA-256 `0fb0c580d2402093bb212ad340ea87f88822aa5844a8114e112bec4c990411b1`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-9` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.4.20/sources/unix/tclConfig.sh](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.4.20/sources/unix/tclConfig.sh). SHA-256 `e67ced9d9a920741fd6ed13a7462acf7f7ccd75283e7ce1daa0ef8f3e7f712ca`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-10` (provider): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.5.19/context/original465-cli-version.stdout](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.5.19/context/original465-cli-version.stdout). SHA-256 `510e2d09d60dc006dfc5d5878788779520c3f1bc62afb4fe7e48870f9de74fe1`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-11` (provider): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.5.19/context/original465-receipt.json](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.5.19/context/original465-receipt.json). SHA-256 `455351d44073645b3adb314743a40c1fe9bf0009e119e45bd3d725de347abcd5`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-12` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.5.19/sources/generic/tcl.h](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.5.19/sources/generic/tcl.h). SHA-256 `c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-13` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.5.19/sources/generic/tclBasic.c](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.5.19/sources/generic/tclBasic.c). SHA-256 `d09c16386ea0376dc3590a74ded2c6e2bcef5079f2b1a3785545bad37c7ead63`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-14` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.5.19/sources/generic/tclCompExpr.c](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.5.19/sources/generic/tclCompExpr.c). SHA-256 `0a6d97e0800151eeb85e60635c9862ac111022a56d8535d38d8d2fd34fd779f5`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-15` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.5.19/sources/generic/tclCompile.c](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.5.19/sources/generic/tclCompile.c). SHA-256 `8c2ec76dbbe697201bcad79f9e32c0c1db23faf7ca01eb19e08f3ea9e68dd988`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-16` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.5.19/sources/generic/tclCompile.h](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.5.19/sources/generic/tclCompile.h). SHA-256 `9ef1b2690de80b9c193d866d8ef8eb3271e57802c91a96f0f0a01f87c1d1a645`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-17` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.5.19/sources/generic/tclInt.h](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.5.19/sources/generic/tclInt.h). SHA-256 `72078ca207924e61b4fbf637eff263d280a0dbc78e6a5fbb39e2cd980d08a8aa`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-18` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.5.19/sources/unix/Makefile](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.5.19/sources/unix/Makefile). SHA-256 `26ae775d2e4657ecfcb45421cc77c4c26170cb2a21985fbb002eb4791bc766a5`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-19` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.5.19/sources/unix/tclConfig.sh](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.5.19/sources/unix/tclConfig.sh). SHA-256 `305b56696cc9b02162fef44ffcaa8f2a99b096aede7e53986d2a53acf7d8cac7`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-20` (provider): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.6.18/context/original465-cli-version.stdout](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.6.18/context/original465-cli-version.stdout). SHA-256 `7ca7e2ae25a10e99a9945869f21ac489508c93f296803490941ae66587c10ec0`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-21` (provider): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.6.18/context/original465-receipt.json](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.6.18/context/original465-receipt.json). SHA-256 `6d30d6e4f3dd67ecdd929cfab8428c8ee4213a097f6e5726cac2a73f9c1b6e6c`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-22` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.6.18/sources/generic/tcl.h](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.6.18/sources/generic/tcl.h). SHA-256 `aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-23` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.6.18/sources/generic/tclBasic.c](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.6.18/sources/generic/tclBasic.c). SHA-256 `19e28c2c9fbcd27e7e538a06960f0d806d04f8c25d94499cf9eb9031f8440198`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-24` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.6.18/sources/generic/tclCompExpr.c](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.6.18/sources/generic/tclCompExpr.c). SHA-256 `970b54cc3b24299ff47e64ec6dccc14fdbffd6fdc9f0904170c2071bfd3f8ca0`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-25` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.6.18/sources/generic/tclCompile.c](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.6.18/sources/generic/tclCompile.c). SHA-256 `d1a494975cb56b8850804669e17fb7c5aacc1d70de505ea661f59ca365ab45c8`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-26` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.6.18/sources/generic/tclCompile.h](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.6.18/sources/generic/tclCompile.h). SHA-256 `be854eab265b25091f3e5a9b8d268d3aeca520d12382f135e3b3ddede49e6ed9`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-27` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.6.18/sources/generic/tclInt.h](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.6.18/sources/generic/tclInt.h). SHA-256 `e9e70f6461d7c4b37173173956040b647914157c28425ec859730cd8eaef4d2e`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-28` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.6.18/sources/generic/tclOptimize.c](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.6.18/sources/generic/tclOptimize.c). SHA-256 `058fea33a68fd490f4550fb4b4d53116ed134901b8cf715674b88d9a5c2d1c64`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-29` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.6.18/sources/unix/Makefile](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.6.18/sources/unix/Makefile). SHA-256 `8afb8697cb70b90518876861086bdb43f6e31b5e96e6d8091ae7b1de33d90d7e`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-30` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.6.18/sources/unix/tclConfig.sh](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.6.18/sources/unix/tclConfig.sh). SHA-256 `beddc87cb1aa1dd414e2be975085bfa07d46e4a9822710e8974da16e8868dfd9`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-31` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.6.18/windows/optimizer-call.txt](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.6.18/windows/optimizer-call.txt). SHA-256 `16be4e717e02ac593c245f03cbf3f67699e75edfe702081181ec316ec49ee1e1`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-32` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.6.18/windows/optimizer-installation.txt](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.6.18/windows/optimizer-installation.txt). SHA-256 `ff3fb243215a0865acfc474db2cbf9b0cf2caccbcdd5e0a785a7830fab42f394`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-33` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.6.18/windows/try-numeric-next-instruction.txt](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.6.18/windows/try-numeric-next-instruction.txt). SHA-256 `c4e6f971b7b46ea972750451a00ae26ec41fa0c8785274da1ae3f58570b82373`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-34` (provider): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.0.4/context/original465-cli-version.stdout](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.0.4/context/original465-cli-version.stdout). SHA-256 `32ada7140f59d3bb7c967706b61cdf727ee4051c1f011f7477694f5b232d1ec9`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-35` (provider): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.0.4/context/original465-receipt.json](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.0.4/context/original465-receipt.json). SHA-256 `9a788720eee4080be87fcf17f3f7ea1df4a1cba0e49bb61f3a09a8ebb2ec753c`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-36` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.0.4/sources/generic/tcl.h](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.0.4/sources/generic/tcl.h). SHA-256 `eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-37` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.0.4/sources/generic/tclBasic.c](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.0.4/sources/generic/tclBasic.c). SHA-256 `6d995c1af94fea92717183882ca41775fe8c0e088def163f6ac8fe699cbffc67`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-38` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.0.4/sources/generic/tclCompExpr.c](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.0.4/sources/generic/tclCompExpr.c). SHA-256 `f82f94056112b9c76292b384ddb0f84b1d4889d1c14c4776fa0929e02767ded1`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-39` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.0.4/sources/generic/tclCompile.c](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.0.4/sources/generic/tclCompile.c). SHA-256 `51b709e7bf2c1f25bb7b3ab371c7dcbd6cf6d2bce509931397303cb770ee21f3`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-40` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.0.4/sources/generic/tclCompile.h](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.0.4/sources/generic/tclCompile.h). SHA-256 `22f512199d2e57370496d8a6713921d9ac57538497a3a64966fa1dee7a6a5dc5`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-41` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.0.4/sources/generic/tclInt.h](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.0.4/sources/generic/tclInt.h). SHA-256 `f000e14fdc4e00da1c5ab98a8087c41be3cbf26564a274152236e0531bb55053`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-42` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.0.4/sources/generic/tclOptimize.c](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.0.4/sources/generic/tclOptimize.c). SHA-256 `3e0166030d973686e2c974ba88f7010437cf9e82b6b36e96e0839d27bce95424`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-43` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.0.4/sources/unix/Makefile](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.0.4/sources/unix/Makefile). SHA-256 `69f1915c208d66c7e38e6871c7f51c8f7be7c2138b45a5361641f9e91854fea5`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-44` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.0.4/sources/unix/tclConfig.sh](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.0.4/sources/unix/tclConfig.sh). SHA-256 `7e808c1d9a006c98ca7b819a43a206ef9e0527ec2ea8dfbb9a1d348659a2ebfe`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-45` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.0.4/windows/optimizer-call.txt](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.0.4/windows/optimizer-call.txt). SHA-256 `c6f975cf2e19121afc134b9e23b946a58f7cfaadc59d17356a340c5658177195`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-46` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.0.4/windows/optimizer-installation.txt](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.0.4/windows/optimizer-installation.txt). SHA-256 `c1cfc45b4b2a0430c36166e28f2f982db7645dcd121df8c9828a65f32fc66ee1`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-47` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.0.4/windows/try-numeric-next-instruction.txt](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.0.4/windows/try-numeric-next-instruction.txt). SHA-256 `8fa38ec8ebff2d341a27ffa4c69b416e4f25f226b94f3bece09fe179259bd174`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-48` (provider): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.1.0/context/original465-cli-version.stdout](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.1.0/context/original465-cli-version.stdout). SHA-256 `a88879ffa7ac4f34863959ff7f314ea863fd002511e61073c6fb8f586394d310`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-49` (provider): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.1.0/context/original465-receipt.json](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.1.0/context/original465-receipt.json). SHA-256 `fd54d48a651dbc7bdddd56f98aaeef82e07719e337f2c283f10e47be0a4caeb6`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-50` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.1.0/sources/generic/tcl.h](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.1.0/sources/generic/tcl.h). SHA-256 `30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-51` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.1.0/sources/generic/tclBasic.c](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.1.0/sources/generic/tclBasic.c). SHA-256 `5f52206bed251629baedd8ef8cc0f7ae1237bf9a7edc7d4bca76d3932f8870d2`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-52` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.1.0/sources/generic/tclCompExpr.c](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.1.0/sources/generic/tclCompExpr.c). SHA-256 `16191591d5dd04ac05c3e3200799bf27167ec1d90c9985c3dbbed04e3c07d1b0`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-53` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.1.0/sources/generic/tclCompile.c](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.1.0/sources/generic/tclCompile.c). SHA-256 `941eaa2c57e4d7d5266134c5b713dca5218d29eea951c89a0676c14b8260eb4f`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-54` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.1.0/sources/generic/tclCompile.h](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.1.0/sources/generic/tclCompile.h). SHA-256 `70203ece61377d4cf22a1eae4348d6216bf77372fd7f8a49274f05d8c546d44e`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-55` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.1.0/sources/generic/tclInt.h](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.1.0/sources/generic/tclInt.h). SHA-256 `fea9d99020df9aec6efebf6e3f18592ee267a9bf80d3843df0d6439efebe65cc`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-56` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.1.0/sources/generic/tclOptimize.c](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.1.0/sources/generic/tclOptimize.c). SHA-256 `fd9b0b5f0d289d5e09780de2acf3259e0373cfb5ba243abff566ac3a2b9035e4`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-57` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.1.0/sources/unix/Makefile](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.1.0/sources/unix/Makefile). SHA-256 `c1ecfb5a77697f0dc6f1057f63ff7aabd75b444f62b5954480aff01ff0565ab1`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-58` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.1.0/sources/unix/tclConfig.sh](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.1.0/sources/unix/tclConfig.sh). SHA-256 `3151ba756040c3f86a08eb4a249c26f71d6f2d600b3f7425385c9434c1d41eb2`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-59` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.1.0/windows/optimizer-call.txt](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.1.0/windows/optimizer-call.txt). SHA-256 `b79125fd66bcf2ab18a763cc379f0ecce631f109ffcf88d7a1a615da5dd334fc`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-60` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.1.0/windows/optimizer-installation.txt](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.1.0/windows/optimizer-installation.txt). SHA-256 `c1cfc45b4b2a0430c36166e28f2f982db7645dcd121df8c9828a65f32fc66ee1`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-61` (source-anchor): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.1.0/windows/try-numeric-next-instruction.txt](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.1.0/windows/try-numeric-next-instruction.txt). SHA-256 `1a009049af27edb4701b780af0790802554be396f07793512ea3ba242b01c593`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-62` (provider): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/context/original465-closure.json](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/context/original465-closure.json). SHA-256 `f7a3ee2db11c0177934d846a9f88c3bad404069c6c675bd8a0d20d12cc8ce5d4`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.
- `optimizer-current-file-63` (provider): [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/context/original465-request.json](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/context/original465-request.json). SHA-256 `b554cf39546d1978dfe3311329f3f7e58e3275e4121abf52aab5dc4ceccabee9`. Exact current source/config/header/window bytes or unchanged original465 context used only for version/input identity correspondence; no execution or opcode result for this question.

## Source inspection

tcl8.6 8.6.18, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOptimize.c`, TclOptimizeBytecode, lines275–330. Whole source SHA-256 `058fea33a68fd490f4550fb4b4d53116ed134901b8cf715674b88d9a5c2d1c64`; snippet SHA-256 `c4e6f971b7b46ea972750451a00ae26ec41fa0c8785274da1ae3f58570b82373`. Revision is the exact whole-file byte hash, with no upstream commit inferred.

```c
		break;
	    }
	    break;

	case INST_TRY_CVT_TO_NUMERIC:
	    switch (nextInst) {
	    case INST_JUMP_TRUE1:
	    case INST_JUMP_TRUE4:
	    case INST_JUMP_FALSE1:
	    case INST_JUMP_FALSE4:
	    case INST_INCR_SCALAR1:
	    case INST_INCR_ARRAY1:
	    case INST_INCR_ARRAY_STK:
	    case INST_INCR_SCALAR_STK:
	    case INST_INCR_STK:
	    case INST_LOR:
	    case INST_LAND:
	    case INST_EQ:
	    case INST_NEQ:
	    case INST_LT:
	    case INST_LE:
	    case INST_GT:
	    case INST_GE:
	    case INST_MOD:
	    case INST_LSHIFT:
	    case INST_RSHIFT:
	    case INST_BITOR:
	    case INST_BITXOR:
	    case INST_BITAND:
	    case INST_EXPON:
	    case INST_ADD:
	    case INST_SUB:
	    case INST_DIV:
	    case INST_MULT:
	    case INST_LNOT:
	    case INST_BITNOT:
	    case INST_UMINUS:
	    case INST_UPLUS:
	    case INST_TRY_CVT_TO_NUMERIC:
		blank = size;
		break;
	    }
	    break;
	}

	if (blank > 0) {
	    for (i=0 ; i<blank ; i++) {
		*(currentInstPtr + i) = INST_NOP;
	    }
	    size = blank;
	}
    }
    Tcl_DeleteHashTable(&targets);
}

/*
```

tcl8.6 8.6.18, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclBasic.c`, Tcl_CreateInterp, lines551–575. Whole source SHA-256 `19e28c2c9fbcd27e7e538a06960f0d806d04f8c25d94499cf9eb9031f8440198`; snippet SHA-256 `ff3fb243215a0865acfc474db2cbf9b0cf2caccbcdd5e0a785a7830fab42f394`. Revision is the exact whole-file byte hash, with no upstream commit inferred.

```c

    iPtr->result = iPtr->resultSpace;
    iPtr->freeProc = NULL;
    iPtr->errorLine = 0;
    TclNewObj(iPtr->objResultPtr);
    Tcl_IncrRefCount(iPtr->objResultPtr);
    iPtr->handle = TclHandleCreate(iPtr);
    iPtr->globalNsPtr = NULL;
    iPtr->hiddenCmdTablePtr = NULL;
    iPtr->interpInfo = NULL;

    TCL_CT_ASSERT(sizeof(iPtr->extra) <= sizeof(Tcl_HashTable));
    iPtr->extra.optimizer = TclOptimizeBytecode;

    iPtr->numLevels = 0;
    iPtr->maxNestingDepth = MAX_NESTING_DEPTH;
    iPtr->framePtr = NULL;	/* Initialise as soon as :: is available */
    iPtr->varFramePtr = NULL;	/* Initialise as soon as :: is available */

    /*
     * TIP #280 - Initialize the arrays used to extend the ByteCode and Proc
     * structures.
     */

    iPtr->cmdFramePtr = NULL;
```

tcl8.6 8.6.18, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCompile.c`, TclSetByteCodeFromAny, lines843–863. Whole source SHA-256 `d1a494975cb56b8850804669e17fb7c5aacc1d70de505ea661f59ca365ab45c8`; snippet SHA-256 `16be4e717e02ac593c245f03cbf3f67699e75edfe702081181ec316ec49ee1e1`. Revision is the exact whole-file byte hash, with no upstream commit inferred.

```c
	TclEmitOpcode(INST_DONE, &compEnv);
	assert (compEnv.atCmdStart > 1);
    }

    /*
     * Apply some peephole optimizations that can cross specific/generic
     * instruction generator boundaries.
     */

    if (iPtr->extra.optimizer) {
	(iPtr->extra.optimizer)(&compEnv);
    }

    /*
     * Invoke the compilation hook procedure if one exists.
     */

    if (hookProc) {
	result = hookProc(interp, &compEnv, clientData);
    }
```

tcl9.0 9.0.4, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOptimize.c`, TclOptimizeBytecode, lines275–330. Whole source SHA-256 `3e0166030d973686e2c974ba88f7010437cf9e82b6b36e96e0839d27bce95424`; snippet SHA-256 `8fa38ec8ebff2d341a27ffa4c69b416e4f25f226b94f3bece09fe179259bd174`. Revision is the exact whole-file byte hash, with no upstream commit inferred.

```c
		break;
	    }
	    break;

	case INST_TRY_CVT_TO_NUMERIC:
	    switch (nextInst) {
	    case INST_JUMP_TRUE1:
	    case INST_JUMP_TRUE4:
	    case INST_JUMP_FALSE1:
	    case INST_JUMP_FALSE4:
	    case INST_INCR_SCALAR1:
	    case INST_INCR_ARRAY1:
	    case INST_INCR_ARRAY_STK:
	    case INST_INCR_SCALAR_STK:
	    case INST_INCR_STK:
	    case INST_EQ:
	    case INST_NEQ:
	    case INST_LT:
	    case INST_LE:
	    case INST_GT:
	    case INST_GE:
	    case INST_MOD:
	    case INST_LSHIFT:
	    case INST_RSHIFT:
	    case INST_BITOR:
	    case INST_BITXOR:
	    case INST_BITAND:
	    case INST_EXPON:
	    case INST_ADD:
	    case INST_SUB:
	    case INST_DIV:
	    case INST_MULT:
	    case INST_LNOT:
	    case INST_BITNOT:
	    case INST_UMINUS:
	    case INST_UPLUS:
	    case INST_TRY_CVT_TO_NUMERIC:
		blank = size;
		break;
	    }
	    break;
	}

	if (blank > 0) {
	    for (i=0 ; i<blank ; i++) {
		currentInstPtr[i] = INST_NOP;
	    }
	    size = blank;
	}
    }
    Tcl_DeleteHashTable(&targets);
}

/*
 * ----------------------------------------------------------------------
 *
```

tcl9.0 9.0.4, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclBasic.c`, Tcl_CreateInterp, lines925–949. Whole source SHA-256 `6d995c1af94fea92717183882ca41775fe8c0e088def163f6ac8fe699cbffc67`; snippet SHA-256 `c1cfc45b4b2a0430c36166e28f2f982db7645dcd121df8c9828a65f32fc66ee1`. Revision is the exact whole-file byte hash, with no upstream commit inferred.

```c
    /* Special invalid value: Any attempt to free the legacy result
     * will cause a crash. */
    iPtr->legacyFreeProc = (void (*) (void))-1;
    iPtr->errorLine = 0;
    iPtr->stubTable = &tclStubs;
    TclNewObj(iPtr->objResultPtr);
    Tcl_IncrRefCount(iPtr->objResultPtr);
    iPtr->handle = TclHandleCreate(iPtr);
    iPtr->globalNsPtr = NULL;
    iPtr->hiddenCmdTablePtr = NULL;
    iPtr->interpInfo = NULL;

    iPtr->optimizer = TclOptimizeBytecode;

    iPtr->numLevels = 0;
    iPtr->maxNestingDepth = MAX_NESTING_DEPTH;
    iPtr->framePtr = NULL;	/* Initialise as soon as :: is available */
    iPtr->varFramePtr = NULL;	/* Initialise as soon as :: is available */

    /*
     * TIP #280 - Initialize the arrays used to extend the ByteCode and Proc
     * structures.
     */

    iPtr->cmdFramePtr = NULL;
```

tcl9.0 9.0.4, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCompile.c`, TclSetByteCodeFromAny, lines858–878. Whole source SHA-256 `51b709e7bf2c1f25bb7b3ab371c7dcbd6cf6d2bce509931397303cb770ee21f3`; snippet SHA-256 `c6f975cf2e19121afc134b9e23b946a58f7cfaadc59d17356a340c5658177195`. Revision is the exact whole-file byte hash, with no upstream commit inferred.

```c
	TclEmitOpcode(INST_DONE, &compEnv);
	assert (compEnv.atCmdStart > 1);
    }

    /*
     * Apply some peephole optimizations that can cross specific/generic
     * instruction generator boundaries.
     */

    if (iPtr->optimizer) {
	(iPtr->optimizer)(&compEnv);
    }

    /*
     * Invoke the compilation hook procedure if there is one.
     */

    if (hookProc) {
	result = hookProc(interp, &compEnv, clientData);
    }
```

tcl9.1 9.1.0, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOptimize.c`, TclOptimizeBytecode, lines288–343. Whole source SHA-256 `fd9b0b5f0d289d5e09780de2acf3259e0373cfb5ba243abff566ac3a2b9035e4`; snippet SHA-256 `1a009049af27edb4701b780af0790802554be396f07793512ea3ba242b01c593`. Revision is the exact whole-file byte hash, with no upstream commit inferred.

```c
		break;
	    }
	    break;

	case INST_TRY_CVT_TO_NUMERIC:
	    switch (nextInst) {
#ifndef REMOVE_DEPRECATED_OPCODES
	    case INST_JUMP_TRUE1:
	    case INST_JUMP_FALSE1:
#endif
	    case INST_JUMP_TRUE:
	    case INST_JUMP_FALSE:
	    case INST_INCR_SCALAR1:
	    case INST_INCR_SCALAR:
	    case INST_INCR_ARRAY1:
	    case INST_INCR_ARRAY:
	    case INST_INCR_ARRAY_STK:
	    case INST_INCR_SCALAR_STK:
	    case INST_INCR_STK:
	    case INST_EQ:
	    case INST_NEQ:
	    case INST_LT:
	    case INST_LE:
	    case INST_GT:
	    case INST_GE:
	    case INST_MOD:
	    case INST_LSHIFT:
	    case INST_RSHIFT:
	    case INST_BITOR:
	    case INST_BITXOR:
	    case INST_BITAND:
	    case INST_EXPON:
	    case INST_ADD:
	    case INST_SUB:
	    case INST_DIV:
	    case INST_MULT:
	    case INST_LNOT:
	    case INST_BITNOT:
	    case INST_UMINUS:
	    case INST_UPLUS:
	    case INST_TRY_CVT_TO_NUMERIC:
		blank = size;
		break;
	    }
	    break;
	}

	if (blank > 0) {
	    for (i=0 ; i<blank ; i++) {
		currentInstPtr[i] = INST_NOP;
	    }
	    size = blank;
	}
    }
    Tcl_DeleteHashTable(&targets);
}
```

tcl9.1 9.1.0, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBasic.c`, Tcl_CreateInterp, lines860–884. Whole source SHA-256 `5f52206bed251629baedd8ef8cc0f7ae1237bf9a7edc7d4bca76d3932f8870d2`; snippet SHA-256 `c1cfc45b4b2a0430c36166e28f2f982db7645dcd121df8c9828a65f32fc66ee1`. Revision is the exact whole-file byte hash, with no upstream commit inferred.

```c
    /* Special invalid value: Any attempt to free the legacy result
     * will cause a crash. */
    iPtr->legacyFreeProc = (void (*) (void))-1;
    iPtr->errorLine = 0;
    iPtr->stubTable = &tclStubs;
    TclNewObj(iPtr->objResultPtr);
    Tcl_IncrRefCount(iPtr->objResultPtr);
    iPtr->handle = TclHandleCreate(iPtr);
    iPtr->globalNsPtr = NULL;
    iPtr->hiddenCmdTablePtr = NULL;
    iPtr->interpInfo = NULL;

    iPtr->optimizer = TclOptimizeBytecode;

    iPtr->numLevels = 0;
    iPtr->maxNestingDepth = MAX_NESTING_DEPTH;
    iPtr->framePtr = NULL;	/* Initialise as soon as :: is available */
    iPtr->varFramePtr = NULL;	/* Initialise as soon as :: is available */

    /*
     * TIP #280 - Initialize the arrays used to extend the ByteCode and Proc
     * structures.
     */

    iPtr->cmdFramePtr = NULL;
```

tcl9.1 9.1.0, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCompile.c`, TclSetByteCodeFromAny, lines1183–1203. Whole source SHA-256 `941eaa2c57e4d7d5266134c5b713dca5218d29eea951c89a0676c14b8260eb4f`; snippet SHA-256 `b79125fd66bcf2ab18a763cc379f0ecce631f109ffcf88d7a1a615da5dd334fc`. Revision is the exact whole-file byte hash, with no upstream commit inferred.

```c
	TclEmitOpcode(		INST_DONE,			&compEnv);
	assert (compEnv.atCmdStart > 1);
    }

    /*
     * Apply some peephole optimizations that can cross specific/generic
     * instruction generator boundaries.
     */

    if (iPtr->optimizer) {
	(iPtr->optimizer)(&compEnv);
    }

    /*
     * Invoke the compilation hook procedure if there is one.
     */

    if (hookProc) {
	result = hookProc(interp, &compEnv, clientData);
    }
```

## Consumer bindings

- [rust/tcl-syntax/src/native_boolean_truth.rs](../../../../rust/tcl-syntax/src/native_boolean_truth.rs), `NativeExpressionResultProducer`: Keep an explicitly reached NumericInstruction result producer separate from a Boolean inline or public-API producer; a descriptor supplies no original instruction or completed physical conversion.
- [rust/tcl-registry/src/native_boolean_truth.rs](../../../../rust/tcl-registry/src/native_boolean_truth.rs), `InvocationDialect::native_numeric_instruction_result_protocol`: Select the actual retained C scalar engine for an independently reached numeric instruction, separately from a Boolean condition rewrite or public expression API copy.
- [rust/tcl-cmd-core/src/native_boolean_truth.rs](../../../../rust/tcl-cmd-core/src/native_boolean_truth.rs), `original_boolean_expression_result`: Perform only the selected reached original result stages through supplied original object/getter/publication operations, preserving first Host and independent original ownership; source interpretation supplies no execution authority.

No Rust assertion is attached to this source interpretation record. Current source definitions and any independently pinned implementation executions remain separate.

## Replay

Read retained source windows against whole-file bytes and verify version/header/config/input mappings only. No native process, Cargo build, Rust assertion or opcode listing was executed for this question. Original465 request, inputs, receipts and raw streams remain immutable and answer their independently measured route/cache question. Referenced static libraries require their exact recorded input paths/hashes; no source-to-library rebuild proof is supplied.
