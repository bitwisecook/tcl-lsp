# naming.runtime.compiled-procedure-fallback-and-source-log

Kind: `implementation-contract`

## Problem statement

A declined or unavailable AOT entry must preserve the actual preselected procedure body producer. A reconstructed source evaluator can change error source/instruction context or reset an error episode when public error variables are read.

## Question

How does a declined or absent compiled procedure entry retain its actual original body producer and public source/error episode without inventing compiled admission or identical native instruction context?

## Conclusion

Interp::run_selected_procedure_body consumes the actual preselected C NativeBodyArtifact or Jim original body object when the AOT entry declines or is absent; only independently missing producers select ordinary source evaluation. The original call frame remains owned by procedure entry. Compiled activation leave retains error result/options for subsequent public native error-variable Read without starting a new evaluation or resetting that episode. Manual ABI source logging preserves original source trace and truncation rules but does not establish the inner native instruction context observed from original provider source. ABI argument objects, original body/header, frame and compiler admission keep their independent issuers. A successful source selection or similar errorInfo grants none of them. The shared evaluated-command logging entry retains the selected error-log protocol. C8.4 direct logging and evaluator-owned already-logged state remain separate, including setter callback effects. Public native error reads preserve the actual existing episode rather than creating a new evaluation. These linked source owners issue no extra original instruction identity or new external observation.

## Scope

Current Runtime implementation API contract. Three fixed controls exercise actual preselected original fallback, manual ABI source trace without instruction identity and public error-variable Read after compiled leave. All seven providers are not tested for this implementation contract. Independently observed original source errors and error-stack instruction labels belong to naming.runtime.original-procedure-error-context; they do not execute an AOT decline or manual ABI logging path.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

## Exact evidence

- `naming-runtime-compiled-procedure-fallback-and-source-log-codegen_abi.rs` (implementation): [runtime/rust/src/codegen_abi.rs](../../../../runtime/rust/src/codegen_abi.rs). SHA-256 `ed85233d3d145ebf36226a423ae10544c3023d4cec9708e04ead04af59803bbf`. Current original source/API owner and fixed assertion bodies; these bytes are not native provider observations or an executed Rust result.
- `naming-runtime-compiled-procedure-fallback-and-source-log-interp.rs` (implementation): [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs). SHA-256 `c068052ef1740e5e3eb16b40e886788c6420bad9f9158bce47319073cf0f6be3`. Current original source/API owner and fixed assertion bodies; these bytes are not native provider observations or an executed Rust result.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `Interp::run_selected_procedure_body`: Retain the actual original C artifact/Jim body producer across absent or declined AOT entry without manufacturing new admission.
- [rust/tcl-registry/src/native_error_log.rs](../../../../rust/tcl-registry/src/native_error_log.rs), `NativeErrorLogProtocol::evaluation_owns_logged_flag`: Select evaluator-owned logged state independently of the direct C logger source recipe.
- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `Interp::log_evaluated_command_bytes`: Share selected evaluated-command logging and independent protocol-owned flag state between Runtime and ABI entry.
- [runtime/rust/src/codegen_abi.rs](../../../../runtime/rust/src/codegen_abi.rs), `codegen_abi::tests::a_declining_entry_falls_back_to_the_source_body_observably_unchanged` (linked): Declined original compiled entry consumes its genuine preselected body/frame and preserves the public guest completion/source behaviour; no new Native body admission is inferred.
- [runtime/rust/src/codegen_abi.rs](../../../../runtime/rust/src/codegen_abi.rs), `codegen_abi::tests::a_logged_statement_site_preserves_the_source_trace_without_instruction_identity` (linked): Manual ABI source log keeps source error trace separately from actual native inner instruction context; equality of public source text does not grant bytecode identity.
- [runtime/rust/src/codegen_abi.rs](../../../../runtime/rust/src/codegen_abi.rs), `codegen_abi::tests::compiled_activation_leave_retains_native_errors_for_public_read` (linked): Genuine selected NativeCore source-log control preserves original guest error variables through public read; no source body or header donor grants another compiled frame.

A named test is a coverage binding, not a claim that it executed.

## Replay

Named Rust coverage is linked, not an executed-result claim. Exact source/executable receipts retain actual outcomes separately. Native evidence and runtime/effect/frame/edit obligations remain independent.
