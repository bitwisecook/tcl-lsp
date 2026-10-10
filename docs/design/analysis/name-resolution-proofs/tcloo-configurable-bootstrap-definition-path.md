# naming.tcloo.configurable-bootstrap-definition-path

Kind: `native-observation`

## Problem statement

The configurable definition namespaces use their own ordered paths. A worker implementation namespace alone cannot stand in for the selected script context.

## Question

What namespace path is reported in each preexisting configurable definition namespace?

## Conclusion

C9.0.4 and C9.1.0 report ::oo::define as the configurableclass namespace path and ::oo::objdefine as the configurableobject namespace path. This probe does not observe the namespace of an entered definition body.

The linked outline control keeps ordinary class source headers separate from unavailable Tcl9 member vocabulary and configurable factories under actual Tcl86 source metadata. A reporting label cannot recover an unavailable declaration or factory header.

## Scope

Exact unchanged ASCII cases.tcl source-file ingress to C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and pinned Jim 0.84-9-g5bac7c9. A separate unchanged ASCII stdin query records startup patchlevels. C8.4/C8.5/C8.6/Jim record configurable availability 0 and skip later controls; this does not classify general TclOO availability. No BIG-IP invocation, counted raw-string NUL input, native header/cache identity, Rust execution or entered body frame is inferred.

This source control is independent of original Native configurable bootstrap publication and imported member behaviour. It supplies no successful factory invocation, allocation, entry/frame or provider observation; existing versioned captures remain unchanged and no assertion outcome is attached.

## Provider answers

### tcl8.4

Status: `unsupported`. Version: 8.4.20. Build: Executable SHA-256 f0e742ccc08a150afd9a0a236ce24c8e12098dae38c0efd46e59f9b1a28bf58a; pinned header/library/Makefile/source hashes retained in tcl8.4-receipt.. Channel: Exact ASCII source file argument to the actual native shell; Tcl file-character ingress/Jim file evaluator. Separate ASCII stdin startup version query.. Dialect: Tcl.

The reached available control reports code 0 and value 0 for ::oo::configurable; later controls for this question are skipped.

### tcl8.5

Status: `unsupported`. Version: 8.5.19. Build: Executable SHA-256 e7eb17821aeb7b1f84619f431e89ecff667bc5461fa8e00d67498b6b7d5dd61a; pinned header/library/Makefile/source hashes retained in tcl8.5-receipt.. Channel: Exact ASCII source file argument to the actual native shell; Tcl file-character ingress/Jim file evaluator. Separate ASCII stdin startup version query.. Dialect: Tcl.

The reached available control reports code 0 and value 0 for ::oo::configurable; later controls for this question are skipped.

### tcl8.6

Status: `unsupported`. Version: 8.6.18. Build: Executable SHA-256 9b156375d068a58fcfb82a2a0afd170afd2b03912cbb6e588676c7688118c8ff; pinned header/library/Makefile/source hashes retained in tcl8.6-receipt.. Channel: Exact ASCII source file argument to the actual native shell; Tcl file-character ingress/Jim file evaluator. Separate ASCII stdin startup version query.. Dialect: Tcl.

The reached available control reports code 0 and value 0 for ::oo::configurable; later controls for this question are skipped.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Executable SHA-256 f4748357cdbf2762f1c5c05916fa53e54718d087120ae9de9c403bbdfaf764b1; pinned header/library/Makefile/source hashes retained in tcl9.0-receipt.. Channel: Exact ASCII source file argument to the actual native shell; Tcl file-character ingress/Jim file evaluator. Separate ASCII stdin startup version query.. Dialect: Tcl.

scope.class.path: code 0, ::oo::define; scope.object.path: code 0, ::oo::objdefine.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Executable SHA-256 2a82f721b68f2ed2347ca4e3daa9a56c49c9e3efce77d00f22fc3b2012d8814d; pinned header/library/Makefile/source hashes retained in tcl9.1-receipt.. Channel: Exact ASCII source file argument to the actual native shell; Tcl file-character ingress/Jim file evaluator. Separate ASCII stdin startup version query.. Dialect: Tcl.

scope.class.path: code 0, ::oo::define; scope.object.path: code 0, ::oo::objdefine.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Executable SHA-256 e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806; pinned header/library/Makefile/source hashes retained in jim-receipt.. Channel: Exact ASCII source file argument to the actual native shell; Tcl file-character ingress/Jim file evaluator. Separate ASCII stdin startup version query.. Dialect: Jim Tcl.

The reached available control reports code 0 and value 0 for ::oo::configurable; later controls for this question are skipped.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `source` (input): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/cases.tcl](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/cases.tcl). SHA-256 `981fbf5717fed4be0452f492405a6d104c8faf92ff6d573a9163cf4f92452983`. Exact unchanged ASCII public source program.
- `aggregate` (observation): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/receipt.json). SHA-256 `31890ae9cd0633d63d44cd551a130f1cac4cfef713146e1d6327e9275f9d9f5e`. Six original aggregate provider rows with source, executable, header, library and startup hashes.
- `tcl8.4-receipt` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.4/receipt.json). SHA-256 `22e72bec79e0a534a5e7a22fa211066bb6ad8b056f3ba99ceb034493d004ba8d`. Original provider identity and exact source/stream/build hashes.
- `tcl8.4-stdout` (observation): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.4/stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.4/stdout). SHA-256 `474d1a121decfed79f338824c5c07d9857091d6c40c09bdcd1c9906bdf7ea928`. Original status/value rows for scope.class.path, scope.object.path, or the reached configurable availability refusal.
- `tcl8.4-startup` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.4/startup.stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.4/startup.stdout). SHA-256 `c4a268983bec029286482df6caa3aaae81571c33d467c0a13f258e335da5954f`. Independent actual startup patchlevel output.
- `tcl8.5-receipt` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.5/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.5/receipt.json). SHA-256 `bee6f5f2dc7df63620abe6645e85f89aee8d914031d4913026b7264310d9d47d`. Original provider identity and exact source/stream/build hashes.
- `tcl8.5-stdout` (observation): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.5/stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.5/stdout). SHA-256 `474d1a121decfed79f338824c5c07d9857091d6c40c09bdcd1c9906bdf7ea928`. Original status/value rows for scope.class.path, scope.object.path, or the reached configurable availability refusal.
- `tcl8.5-startup` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.5/startup.stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.5/startup.stdout). SHA-256 `6b58793c92a3db92926bdacbe0a52a520d417fe876aaa7cf8c29fed54a1fea71`. Independent actual startup patchlevel output.
- `tcl8.6-receipt` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.6/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.6/receipt.json). SHA-256 `a401ff099fe6f3d482f6c11239dbc6656d9dcd4a911d8877b49dff0094187455`. Original provider identity and exact source/stream/build hashes.
- `tcl8.6-stdout` (observation): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.6/stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.6/stdout). SHA-256 `474d1a121decfed79f338824c5c07d9857091d6c40c09bdcd1c9906bdf7ea928`. Original status/value rows for scope.class.path, scope.object.path, or the reached configurable availability refusal.
- `tcl8.6-startup` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.6/startup.stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.6/startup.stdout). SHA-256 `7523e113df6c321e18099a10ec22086db746e2fcc31a879e16d961e5d3878d01`. Independent actual startup patchlevel output.
- `tcl9.0-receipt` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.0/receipt.json). SHA-256 `e90aa04df8115160a47fcfc2c5bcf2a3e0e206a7fd307764f2acb314238196e5`. Original provider identity and exact source/stream/build hashes.
- `tcl9.0-stdout` (observation): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.0/stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.0/stdout). SHA-256 `47a16dfa64a74610444c4ec1467751beb07910cd092dd93a06fffc9a2137d4d8`. Original status/value rows for scope.class.path, scope.object.path, or the reached configurable availability refusal.
- `tcl9.0-startup` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.0/startup.stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.0/startup.stdout). SHA-256 `b83920bac1b2d47c3840f588f95bee9dc69b85e26117f471541cdf85d7da52a2`. Independent actual startup patchlevel output.
- `tcl9.1-receipt` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.1/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.1/receipt.json). SHA-256 `4da39724762f9df50b7ff33503b2e32c16b5b852fdbaedaec48d1c353558ab9e`. Original provider identity and exact source/stream/build hashes.
- `tcl9.1-stdout` (observation): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.1/stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.1/stdout). SHA-256 `47a16dfa64a74610444c4ec1467751beb07910cd092dd93a06fffc9a2137d4d8`. Original status/value rows for scope.class.path, scope.object.path, or the reached configurable availability refusal.
- `tcl9.1-startup` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.1/startup.stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.1/startup.stdout). SHA-256 `077ee5b3a3a7fc2622ceff6466fea0c28f3fb08b24653dcc7f4baab29b9aef36`. Independent actual startup patchlevel output.
- `jim-receipt` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/jim/receipt.json). SHA-256 `6953861b00fc21791263799038df74bba085bdb94b8ba2d299f2978c83252569`. Original provider identity and exact source/stream/build hashes.
- `jim-stdout` (observation): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/jim/stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/jim/stdout). SHA-256 `474d1a121decfed79f338824c5c07d9857091d6c40c09bdcd1c9906bdf7ea928`. Original status/value rows for scope.class.path, scope.object.path, or the reached configurable availability refusal.
- `jim-startup` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/jim/startup.stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/jim/startup.stdout). SHA-256 `f9752899fa012cf994a528e0250402609067a6f67a144a1172644a9329386803`. Independent actual startup patchlevel output.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

- [rust/tcl-lsp-core/src/document_symbols.rs](../../../../rust/tcl-lsp-core/src/document_symbols.rs), `document_symbols_from_analysis`: Read genuine selected source class/member headers and withdraw unavailable member/factory roles before generating document outline labels.
- [rust/tcl-lsp-core/src/document_symbols.rs](../../../../rust/tcl-lsp-core/src/document_symbols.rs), `document_symbols::tests::original_outline_does_not_borrow_tcl9_member_roles_under_tcl86` (linked): Actual Tcl86 source metadata may retain an ordinary class header while unavailable Tcl9 classmethod/property roles supply no member declaration; an unavailable configurable factory supplies no header or outline. This is selected source/header advice, not bootstrap publication.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Replay

The retained capture.py and unchanged six provider receipts retain the original source-file/startup invocation paths. No portable executable replay or Rust test execution is asserted.
