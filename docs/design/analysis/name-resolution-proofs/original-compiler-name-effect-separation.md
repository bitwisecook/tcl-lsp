# naming.compiler.original-name-effect-separation

Kind: `implementation-contract`

## Problem statement

A genuine stock set handler may be selected while its caller compilation mode is unknown. Treating uncertain opcode admission as an arbitrary compiler mutation erases previously installed commands before this invocation evaluates any arguments. The opposing risk is assuming compiler purity from the Set handler footprint: source substitutions can cause nested compilation, and an unrelated or replaced compiler has its own effects. The question is limited to the selected stock Set compiler and a complete substitution-free original vector.

## Question

How does the shared source selection separate compiler tracked-name effects from uncertain native admission without lending an arbitrary handler or original vector?

## Conclusion

The Registry query separates tracked-name effects from native admission for independently selected C NoHook and complete literal Set vectors; additional direct-emitter families have their own pinned source explanation. Source selection meets this effect closure across compiler alternatives and preserves Unknown admission and absent native preparations. Missing compiler ownership, dependencies or unrepresented nested compilation withdraw the closure. The common source driver uses it only to decide whether compilation makes tracked command/namespace state opaque before argv; actual post-argv handler lookup, variable receiver, normal completion and body entry retain their own proofs. A private source binding projection retains this existing effect closure from the authentic compiler point. Its readonly query checks the complete original vector and configuration and meets the closure across alternatives; it does not change native admission or issue a normal result.

## Scope

Current Rust original-source selection and tracked-name barrier contract. Exact full lexer config/channel/word ownership comes from NativeCompilerWords capture and source projection. Static opaque NativeValue bytes are accepted for this effect purpose only. C source support is recorded separately; no native test execution or physical object/compiler authority is claimed.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `NativeCompilationSpec::preserves_names_before_original_arguments`: Select only the source-supported tracked-name effect closure for a genuine original compiler vector; no opcode or handler grant.
- [rust/tcl-compiler/src/command_binding/compiled_invocation.rs](../../../../rust/tcl-compiler/src/command_binding/compiled_invocation.rs), `original_compiler_preserves_names`: Project exact same original image/vector/full config and selected source protocol into the Registry effect query.
- [rust/tcl-compiler/src/command_binding/compiled_invocation.rs](../../../../rust/tcl-compiler/src/command_binding/compiled_invocation.rs), `CompiledInvocationSelection::join`: Meet independent compiler name-effect closure without strengthening native admission or handler identity.
- [rust/tcl-compiler/src/command_binding.rs](../../../../rust/tcl-compiler/src/command_binding.rs), `SourceBindingDriver::walk_source_command`: Withdraw tracked names only for unclosed compiler effects before authentic argv evaluation.
- [rust/tcl-compiler/src/command_binding/compiled_invocation.rs](../../../../rust/tcl-compiler/src/command_binding/compiled_invocation.rs), `original_compiler_names_preserved_for_tokens`: Project the independently retained original compiler name-effect closure under the unchanged full token vector; admission and completion remain separate.
- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `native_compilation::tests::original_literal_set_compiler_effects_do_not_grant_opcode_admission` (linked): Original static Set vectors close names while admission remains Unknown; dynamic or nested words, wrong argc/operation/ordinal or missing dialect decline.
- [rust/tcl-compiler/src/command_binding/compiled_invocation.rs](../../../../rust/tcl-compiler/src/command_binding/compiled_invocation.rs), `command_binding::compiled_invocation::tests::original_literal_compiler_name_effects_remain_separate_from_unknown_admission` (linked): Keep Unknown native admission and absent preparations while projecting actual stock compiler name-effect closure; foreign original word values and a joined nested unknown compiler effect withdraw the readonly query.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cargo",
  "test",
  "-p",
  "tcl-registry",
  "original_literal_set_compiler_effects_do_not_grant_opcode_admission"
]
```

Run the listed selectors against the complete current implementation. A linked Rust selector is coverage, not a result receipt.
