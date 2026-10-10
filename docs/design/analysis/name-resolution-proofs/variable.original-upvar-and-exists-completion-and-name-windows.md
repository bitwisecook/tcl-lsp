# naming.variable.original-upvar-and-exists-completion-and-name-windows

Kind: `native-observation`

## Problem statement

Alias-pair installation, selected caller frames, left-to-right operand evaluation, an info-exists probe and ordinary reads have different completion/observer paths. Original name objects and opcode presence must be measured separately from the logical variable name, especially when a later pair or callback fails.

## Question

For the exact 28 retained upvar/info-exists programs per C release, what original completion/result, emitted opcode and four name-header windows occur?

## Conclusion

The complete five-provider transcripts retain 140 original completion/result/opcode windows plus 20 name-header windows. They distinguish local/global alias pairs, partial evaluation failures, array/scalar alias rejection, info-exists result routes and root destruction. Each header belongs only to its observed original operand; opcode presence does not license executing it in another environment or certify arbitrary read/alias completion. Structured WrongArguments presentation delegates NativeWrongArgumentsProtocol::string_result to select the C append-based String producer. A generic message lookalike cannot select this receipt; Jim and explicit Logical protocol rows have no C String producer. Runtime command_error_string_result/report_cmd_error and VM completion_from_cmd_error consume the same selected recipe. Linked presenter controls establish implementation coverage, not another native object/header observation or an executed Rust result. NativeVariableNameProtocol::uses_original_name_cache keeps lookup purpose independent of current name bytes. C8.4 Exists follows its selected legacy byte lookup and genuine current scope/cells without inspecting, installing or retiring the original parsed/local name primary. A later actual object Read independently adopts its own cache; other releases and Read/Write/Array purposes keep their selected object paths. The pure recipe and original list/combined/NUL API controls are software/source bindings, distinct from the original case16 native scalar-primary observation and its completed native scope. Required UPVAR level operands use NativeFrameLevelProtocol::resolve_required_object independently from optional leading frame getters. A required numeric/name level cannot borrow the optional current-frame default, and width/unknown/retired-owner refusal remains typed across Runtime and VM frame selection. NativeFrameLevelFailure retains its selected bad-level String-result producer alongside exact level bytes and error-code classification. NativeFrameLevelProtocol::bad_level_string_result chooses C8.4/C8.5 append-String or C8.6+ formatted-String production; Jim supplies no C String producer. Shared CmdCore native_frame_error::present preserves a reached primitive getter failure rather than reconstruct it from rendered text, and both Runtime and VM consume that typed presenter. The required-level API control checks actual String type/resident result bytes independently of the optional leading-frame default. Those software producer controls do not add a private native header or lifetime observation to the unchanged original140 completion/20 name-header windows.

## Scope

Exact probe.c and cases.rs programs under C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 private compiled body artifacts and original counted native objects. Legacy trace syntax errors remain captured guest outcomes. Jim/BIG-IP not queried. This is distinct from modern quiet-exists trace controls and physical general formal/cache inventories. The reached frame failure is transported by Syntax NativeFrameLevelFailure with its Registry reexport; both Runtime and VM consume the same CmdCore presenter. The shared lower transport type and Registry reexport add no native result/header/lifetime observations or executed assertion.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Archive SHA d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; probe SHA 90b9f81776a024bc0f5c02a65cafb8cf6edb30ea7d3bd6c3e5f625c3f757f135. Channel: Original source bodies through private compiled Tcl procedure artifact. Dialect: Tcl.

28 result windows and four name-header windows; compile/run exit 0. All fields remain separated by their original R/H prefixes.

```text
H|0|0|localVarName|1|3|78
R|0|0|none|1|3|4c4f43414c|
H|1|0|parsedVarName|1|3|78
R|1|0|none|1|2|474c4f42414c|
H|2|0|parsedVarName|1|3|78
R|2|0|none|1|2|474c4f42414c|
R|3|0|none|1|3|4e4557|
R|4|0|none|1|5|4e4557|
R|5|0|list|0|1|4c4f43414c204c4f43414c4152524159|
R|6|0|list|0|1|4c4f43414c204c4f43414c4152524159207b782061286b297d|
R|7|1|none|1|3|53544f50|
R|8|1|none|1|1|626164207661726961626c65206e616d6520223a3a616c696173223a20757076617220776f6e277420637265617465206e616d657370616365207661726961626c6520746861742072656665727320746f2070726f636564757265207661726961626c65|
R|9|0|int|0|1|30|
R|10|1|none|1|1|6578747261206368617261637465727320616674657220636c6f73652d6272616365|
R|11|1|string|1|1|77726f6e67202320617267733a2073686f756c6420626520227570766172203f6c6576656c3f206f74686572566172206c6f63616c566172203f6f74686572566172206c6f63616c566172202e2e2e3f22|
R|12|0|int|0|1|31|
R|13|0|int|0|1|31|
R|14|0|int|0|1|31|
R|15|0|int|0|1|31|
H|16|0|none|1|3|78
R|16|0|none|1|1||
R|17|0|int|0|1|31|
R|18|0|int|0|1|31|
R|19|0|int|0|1|31|
R|20|0|int|0|1|30|
R|21|0|int|0|1|30|
R|22|1|none|1|1|6578747261206368617261637465727320616674657220636c6f73652d6272616365|
R|23|1|none|1|3|53544f50|
R|24|0|int|0|1|31|
R|25|0|int|0|1|30|
R|26|0|int|0|1|30|
R|27|0|int|0|1|30|
```

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Archive SHA 99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; probe SHA 69a0cb47c01a1814167bb21a97a4e1145ea4c479f7d12f81d34a0d41ba6bf8bd. Channel: Original source bodies through private compiled Tcl procedure artifact. Dialect: Tcl.

28 result windows and four name-header windows; compile/run exit 0. All fields remain separated by their original R/H prefixes.

```text
H|0|0|localVarName|1|3|78
R|0|0|none|1|3|4c4f43414c|upvar:4,
H|1|0|parsedVarName|1|3|78
R|1|0|none|1|2|474c4f42414c|upvar:4,
H|2|0|parsedVarName|1|3|78
R|2|0|none|1|2|474c4f42414c|
R|3|0|none|1|3|4e4557|upvar:4,
R|4|0|none|1|4|4e4557|upvar:4,
R|5|0|list|0|1|4c4f43414c204c4f43414c4152524159|upvar:4,upvar:5,
R|6|0|list|0|1|4c4f43414c204c4f43414c4152524159207b782061286b297d|upvar:4,upvar:5,
R|7|1|none|1|3|53544f50|upvar:4,upvar:5,
R|8|1|string|1|1|626164207661726961626c65206e616d6520223a3a616c696173223a2063616e277420637265617465206e616d657370616365207661726961626c6520746861742072656665727320746f2070726f636564757265207661726961626c65|
R|9|1|string|1|1|626164207661726961626c65206e616d652022616c696173286b29223a2063616e2774206372656174652061207363616c6172207661726961626c652074686174206c6f6f6b73206c696b6520616e20617272617920656c656d656e74|existScalar:4,
R|10|0|none|1|3|4c4f43414c|upvar:4,
R|11|1|string|1|1|77726f6e67202320617267733a2073686f756c6420626520227570766172203f6c6576656c3f206f74686572566172206c6f63616c566172203f6f74686572566172206c6f63616c566172202e2e2e3f22|
R|12|0|int|0|2|31|existScalar:2,
R|13|0|int|1|2|31|existArray:3,
R|14|0|int|1|2|31|existStk,
R|15|0|int|1|2|31|existArrayStk,
H|16|0|localVarName|1|3|78
R|16|0|none|1|1||existStk,
R|17|0|int|1|2|31|existArray:3,
R|18|0|int|1|2|31|existArrayStk,
R|19|0|int|1|2|31|existStk,
R|20|0|int|0|2|30|existStk,
R|21|0|int|1|2|30|existArray:3,
R|22|0|int|1|2|31|existScalar:2,
R|23|1|none|1|3|53544f50|existStk,
R|24|0|int|1|2|31|upvar:4,existScalar:4,
R|25|0|int|1|2|30|upvar:4,existScalar:4,
R|26|0|int|1|2|30|upvar:4,existScalar:4,
R|27|0|int|1|2|30|upvar:4,existScalar:4,
```

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Archive SHA a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; probe SHA 89a051c8ad61680652b869a27c81176ea69a39e5afb24efd6f867d0f56f4c113. Channel: Original source bodies through private compiled Tcl procedure artifact. Dialect: Tcl.

28 result windows and four name-header windows; compile/run exit 0. All fields remain separated by their original R/H prefixes.

```text
H|0|0|localVarName|1|3|78
R|0|0|none|1|3|4c4f43414c|upvar:4,
H|1|0|parsedVarName|1|3|78
R|1|0|none|1|2|474c4f42414c|upvar:4,
H|2|0|parsedVarName|1|3|78
R|2|0|none|1|2|474c4f42414c|
R|3|0|none|1|3|4e4557|upvar:4,
R|4|0|none|1|4|4e4557|upvar:4,
R|5|0|list|0|1|4c4f43414c204c4f43414c4152524159|upvar:4,upvar:5,
R|6|0|list|0|1|4c4f43414c204c4f43414c4152524159207b782061286b297d|upvar:4,upvar:5,
R|7|1|none|1|4|53544f50|upvar:4,upvar:5,
R|8|1|string|1|1|626164207661726961626c65206e616d6520223a3a616c696173223a2063616e277420637265617465206e616d657370616365207661726961626c6520746861742072656665727320746f2070726f636564757265207661726961626c65|
R|9|1|string|1|1|626164207661726961626c65206e616d652022616c696173286b29223a2063616e2774206372656174652061207363616c6172207661726961626c652074686174206c6f6f6b73206c696b6520616e20617272617920656c656d656e74|existScalar:4,
R|10|0|none|1|3|4c4f43414c|upvar:4,
R|11|1|string|1|1|626164206c6576656c20226b22|
R|12|0|int|0|2|31|existScalar:2,
R|13|0|int|1|2|31|existArray:3,
R|14|0|int|1|2|31|existStk,
R|15|0|int|1|2|31|existArrayStk,
H|16|0|localVarName|1|3|78
R|16|0|none|1|1||existStk,
R|17|0|int|1|2|31|existArray:3,
R|18|0|int|1|2|31|existArrayStk,
R|19|0|int|1|2|31|existStk,
R|20|0|int|0|2|30|existStk,
R|21|0|int|1|2|30|existArray:3,
R|22|0|int|1|2|31|existScalar:2,
R|23|1|none|1|4|53544f50|existStk,
R|24|0|int|1|2|31|upvar:4,existScalar:4,
R|25|0|int|1|2|30|upvar:4,existScalar:4,
R|26|0|int|1|2|30|upvar:4,existScalar:4,
R|27|0|int|1|2|30|upvar:4,existScalar:4,
```

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Archive SHA 2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; probe SHA dfc2ab87b933c71110c970d92dff4ebdc4f05d9e944ed096f56c58e448dc5697. Channel: Original source bodies through private compiled Tcl procedure artifact. Dialect: Tcl.

28 result windows and four name-header windows; compile/run exit 0. All fields remain separated by their original R/H prefixes.

```text
H|0|0|localVarName|1|3|78
R|0|0|none|1|3|4c4f43414c|upvar:4,
H|1|0|parsedVarName|1|3|78
R|1|0|none|1|2|474c4f42414c|upvar:4,
H|2|0|parsedVarName|1|3|78
R|2|0|none|1|2|474c4f42414c|
R|3|0|none|1|3|4e4557|upvar:4,
R|4|0|none|1|4|4e4557|upvar:4,
R|5|0|list|0|1|4c4f43414c204c4f43414c4152524159|upvar:4,upvar:5,
R|6|0|list|0|1|4c4f43414c204c4f43414c4152524159207b782061286b297d|upvar:4,upvar:5,
R|7|1|none|1|4|53544f50|upvar:4,upvar:5,
R|8|1|string|1|1|626164207661726961626c65206e616d6520223a3a616c696173223a2063616e277420637265617465206e616d657370616365207661726961626c6520746861742072656665727320746f2070726f636564757265207661726961626c65|
R|9|1|string|1|1|626164207661726961626c65206e616d652022616c696173286b29223a2063616e2774206372656174652061207363616c6172207661726961626c652074686174206c6f6f6b73206c696b6520616e20617272617920656c656d656e74|existScalar:4,
R|10|0|none|1|3|4c4f43414c|upvar:4,
R|11|1|string|1|1|626164206c6576656c20226b22|
R|12|0|int|0|2|31|existScalar:2,
R|13|0|int|1|2|31|existArray:3,
R|14|0|int|1|2|31|existStk,
R|15|0|int|1|2|31|existArrayStk,
H|16|0|localVarName|1|3|78
R|16|0|none|1|1||existStk,
R|17|0|int|1|2|31|existArray:3,
R|18|0|int|1|2|31|existArrayStk,
R|19|0|int|1|2|31|existStk,
R|20|0|int|0|2|30|existStk,
R|21|0|int|1|2|30|existArray:3,
R|22|0|int|1|2|31|existScalar:2,
R|23|1|none|1|4|53544f50|existStk,
R|24|1|string|1|1|626164206f7074696f6e20227661726961626c65223a206d757374206265206164642c20696e666f2c206f722072656d6f7665|upvar:4,existScalar:4,
R|25|1|string|1|1|626164206f7074696f6e20227661726961626c65223a206d757374206265206164642c20696e666f2c206f722072656d6f7665|upvar:4,existScalar:4,
R|26|0|int|1|2|30|upvar:4,existScalar:4,
R|27|0|int|1|2|30|upvar:4,existScalar:4,
```

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Archive SHA 8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; probe SHA b94842937c64ab94d21ef98243e2f07735f052b75656e87a7e98db96c9e238f4. Channel: Original source bodies through private compiled Tcl procedure artifact. Dialect: Tcl.

28 result windows and four name-header windows; compile/run exit 0. All fields remain separated by their original R/H prefixes.

```text
H|0|0|localVarName|1|3|78
R|0|0|none|1|3|4c4f43414c|upvar:4,
H|1|0|parsedVarName|1|3|78
R|1|0|none|1|2|474c4f42414c|upvar:4,
H|2|0|parsedVarName|1|3|78
R|2|0|none|1|2|474c4f42414c|
R|3|0|none|1|3|4e4557|upvar:4,
R|4|0|none|1|4|4e4557|upvar:4,
R|5|0|list|0|1|4c4f43414c204c4f43414c4152524159|upvar:4,upvar:5,
R|6|0|list|0|1|4c4f43414c204c4f43414c4152524159207b782061286b297d|upvar:4,upvar:5,
R|7|1|none|1|4|53544f50|upvar:4,upvar:5,
R|8|1|string|1|1|626164207661726961626c65206e616d6520223a3a616c696173223a2063616e277420637265617465206e616d657370616365207661726961626c6520746861742072656665727320746f2070726f636564757265207661726961626c65|
R|9|1|string|1|1|626164207661726961626c65206e616d652022616c696173286b29223a2063616e2774206372656174652061207363616c6172207661726961626c652074686174206c6f6f6b73206c696b6520616e20617272617920656c656d656e74|existScalar:4,
R|10|0|none|1|3|4c4f43414c|upvar:4,
R|11|1|string|1|1|626164206c6576656c20226b22|
R|12|0|int|0|2|31|existScalar:2,
R|13|0|int|1|2|31|existArray:3,
R|14|0|int|1|2|31|existStk,
R|15|0|int|1|2|31|existArrayStk,
H|16|0|localVarName|1|3|78
R|16|0|none|1|1||existStk,
R|17|0|int|1|2|31|existArray:3,
R|18|0|int|1|2|31|existArrayStk,
R|19|0|int|1|2|31|existStk,
R|20|0|int|0|2|30|existStk,
R|21|0|int|1|2|30|existArray:3,
R|22|0|int|1|2|31|existScalar:2,
R|23|1|none|1|4|53544f50|existStk,
R|24|1|string|1|1|626164206f7074696f6e20227661726961626c65223a206d757374206265206164642c20696e666f2c206f722072656d6f7665|upvar:4,existScalar:4,
R|25|1|string|1|1|626164206f7074696f6e20227661726961626c65223a206d757374206265206164642c20696e666f2c206f722072656d6f7665|upvar:4,existScalar:4,
R|26|0|int|1|2|30|upvar:4,existScalar:4,
R|27|0|int|1|2|30|upvar:4,existScalar:4,
```

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `e0` (provider): [rust/tcl-registry/tests/data/native_upvar_info_exists/manifest.json](../../../../rust/tcl-registry/tests/data/native_upvar_info_exists/manifest.json). SHA-256 `24daacf3a132fac16d19c2a3d786fafa89540b1f6ab94269ca74388a8ccbc15f`. Original compiler argv/library/binary/stdout hashes and window counts for all five C providers.
- `e1` (input): [rust/tcl-registry/tests/data/native_upvar_info_exists/probe.c](../../../../rust/tcl-registry/tests/data/native_upvar_info_exists/probe.c). SHA-256 `bcc9a46141da632b600910c405f88af61bc5905b3f3331c6b4ebdb9b095290de`. Original case scripts, initial globals/procedure frames and original object/header observation probe.
- `e2` (input): [rust/tcl-registry/tests/data/native_upvar_info_exists/cases.rs](../../../../rust/tcl-registry/tests/data/native_upvar_info_exists/cases.rs). SHA-256 `2bd4c21b85362214daad4bb1836ffdb4218a1a90bb7e0bcc0cecfd8d4d295bf2`. Exact shared 28-program roster retained by current comparisons.
- `e3` (observation): [rust/tcl-registry/tests/data/native_upvar_info_exists/8.4.20.txt](../../../../rust/tcl-registry/tests/data/native_upvar_info_exists/8.4.20.txt). SHA-256 `f346f6f2647489d68f259ebd6e3cd0bf11d459a267b63050b1c875e81443bda6`. All 28 completion/opcode rows and four header observation rows, including guest failures.
- `e4` (observation): [rust/tcl-registry/tests/data/native_upvar_info_exists/8.5.19.txt](../../../../rust/tcl-registry/tests/data/native_upvar_info_exists/8.5.19.txt). SHA-256 `ff2a2f35c7525c61c9951424dd06c53c8d71d4e309fbb15ff29b5c446b8b9118`. All 28 completion/opcode rows and four header observation rows, including guest failures.
- `e5` (observation): [rust/tcl-registry/tests/data/native_upvar_info_exists/8.6.18.txt](../../../../rust/tcl-registry/tests/data/native_upvar_info_exists/8.6.18.txt). SHA-256 `f29d0f4a6e8fd58a54d2864d742fdeb5698266e2a6335f5c4ee65ff5c388070b`. All 28 completion/opcode rows and four header observation rows, including guest failures.
- `e6` (observation): [rust/tcl-registry/tests/data/native_upvar_info_exists/9.0.4.txt](../../../../rust/tcl-registry/tests/data/native_upvar_info_exists/9.0.4.txt). SHA-256 `93a44c635222267d858586ef1c020a5b5bd39c9ad574f0dd9bb51f5f2b23bebf`. All 28 completion/opcode rows and four header observation rows, including guest failures.
- `e7` (observation): [rust/tcl-registry/tests/data/native_upvar_info_exists/9.1.0.txt](../../../../rust/tcl-registry/tests/data/native_upvar_info_exists/9.1.0.txt). SHA-256 `93a44c635222267d858586ef1c020a5b5bd39c9ad574f0dd9bb51f5f2b23bebf`. All 28 completion/opcode rows and four header observation rows, including guest failures.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/interp/native_body_artifact/native_upvar_info_exists_tests.rs](../../../../runtime/rust/src/interp/native_body_artifact/native_upvar_info_exists_tests.rs), `original_upvar_and_exists_artifact_preserves_140_native_completions_and_20_name_headers`: Independent current Rust comparison at the stated semantic boundary.
- [rust/tcl-vm/src/interp/native_upvar_info_exists_tests.rs](../../../../rust/tcl-vm/src/interp/native_upvar_info_exists_tests.rs), `original_upvar_and_exists_preserve_140_native_completions_and_20_name_headers`: Independent current Rust comparison at the stated semantic boundary.
- [rust/tcl-registry/src/native_wrong_arguments.rs](../../../../rust/tcl-registry/src/native_wrong_arguments.rs), `NativeWrongArgumentsProtocol::string_result`: Select the C String producer only from structured actual WrongArguments purpose, without message-based recovery.
- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `Interp::command_error_string_result`: Consume the selected structured shared String-result producer in Runtime error presentation.
- [rust/tcl-vm/src/command.rs](../../../../rust/tcl-vm/src/command.rs), `completion_from_cmd_error`: Consume the same structured error presentation receipt in VM; source implementation only.
- [rust/tcl-syntax/src/native_variable_name.rs](../../../../rust/tcl-syntax/src/native_variable_name.rs), `NativeVariableNameProtocol::uses_original_name_cache`: Select exact release/caller lookup purpose; C8.4 Exists cannot borrow the independent object Read name-cache path.
- [rust/tcl-registry/src/frame_effect/native_object.rs](../../../../rust/tcl-registry/src/frame_effect/native_object.rs), `NativeFrameLevelProtocol::resolve_required_object`: Select genuine required UPVAR object-level parsing independently of optional leading-frame default semantics.
- [rust/tcl-registry/src/frame_effect.rs](../../../../rust/tcl-registry/src/frame_effect.rs), `NativeFrameLevelProtocol::bad_level_string_result`: Retain the release-selected genuine C bad-level append/format String producer independently of error text or optional frame defaults.
- [rust/tcl-cmd-core/src/native_frame_error.rs](../../../../rust/tcl-cmd-core/src/native_frame_error.rs), `present`: Present typed reached original frame failures while preserving primitive getter metadata and selected String-result birth; Runtime and VM share this owner.
- [rust/tcl-syntax/src/native_frame_error.rs](../../../../rust/tcl-syntax/src/native_frame_error.rs), `NativeFrameLevelFailure`: Transport the selected actual frame-error cause and String-result producer through the shared lower syntax owner with unchanged Registry reexport; rendering does not recreate primitive authority.
- [runtime/rust/src/interp/native_body_artifact/native_upvar_info_exists_tests.rs](../../../../runtime/rust/src/interp/native_body_artifact/native_upvar_info_exists_tests.rs), `interp::native_body_artifact::native_upvar_info_exists_tests::original_upvar_and_exists_artifact_preserves_140_native_completions_and_20_name_headers` (linked): Compares every original completion/name header without deriving an unobserved native donor.
- [rust/tcl-vm/src/interp/native_upvar_info_exists_tests.rs](../../../../rust/tcl-vm/src/interp/native_upvar_info_exists_tests.rs), `interp::native_upvar_info_exists_tests::original_upvar_and_exists_preserve_140_native_completions_and_20_name_headers` (linked): Compares the exact retained roster through VM original artifacts.
- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `interp::tests::native_wrong_arguments_string_result_requires_the_structured_presenter` (linked): Structured C WrongArguments retains the selected String result; generic equal message bytes keep their independent untyped producer.
- [rust/tcl-vm/src/command.rs](../../../../rust/tcl-vm/src/command.rs), `command::tests::native_wrong_arguments_completion_preserves_the_selected_string_producer` (linked): VM consumes the same structured C String producer and refuses message-lookalike recovery; no new native header observation.
- [rust/tcl-syntax/src/native_variable_name.rs](../../../../rust/tcl-syntax/src/native_variable_name.rs), `native_variable_name::tests::existence_name_cache_purpose_preserves_the_original_release_boundary` (linked): Pure release-purpose control retains C8.4 Exists byte lookup separately from Read/Write/Array cache ownership; original native scalar case16 remains independent.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::legacy_existence_preserves_original_name_caches_and_object_reads_still_adopt_them` (linked): Genuine C8.4 software Exists leaves original scalar/list/combined/NUL name primaries untouched while a separately reached actual object Read adopts its own name cache. No new native header or completed read grant.
- [runtime/rust/src/cmd_eval/native_frame_reference_tests.rs](../../../../runtime/rust/src/cmd_eval/native_frame_reference_tests.rs), `cmd_eval::native_frame_reference_tests::required_upvar_level_does_not_borrow_optional_leading_frame_default` (linked): Genuine original k level objects keep optional-current-frame success distinct from required UPVAR BadLevel, with reached lookup classification and actual software String type/resident bad-level bytes. Unknown/bootstrap/width/frame purposes remain independent; native private header identity is not asserted.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cc",
  "-DSTDC_HEADERS=1",
  "-DHAVE_UNISTD_H=1",
  "-I/path/to/recorded/generic",
  "-I/path/to/recorded/unix",
  "rust/tcl-registry/tests/data/native_upvar_info_exists/probe.c",
  "/path/to/recorded/libtcl.a",
  "-lm",
  "-ldl",
  "-lpthread",
  "-lz",
  "-o",
  "/tmp/native-variable-proof"
]
```

Replace provider/header/archive paths with the recorded release/build and preserve the manifested original compiler options. Execute /tmp/native-variable-proof in a fresh process; require all stdout bytes to equal that provider's retained transcript, empty stderr and exit 0. The retained probe uses private interpreter/frame/compiler headers; a stock shell invocation is not an equivalent replay. Original artifact hashes identify the observed build, not a fresh rerun. No Rust pass is inferred.
