# naming.source.static-template-normal-completion

Kind: `implementation-contract`

## Problem statement

Quoted and escaped static words use the template evaluator even when every component is source text. A retained native byte result alone cannot certify the component evaluation, but failing to retain an independently completed literal evaluation makes later source control flow unnecessarily unknown.

## Question

Which Text-only original templates can retain a Normal evaluation result without borrowing a produced byte value as completion authority?

## Conclusion

The actual evaluator requires an authentic whole NativeWord, exclusively Text components, a current final produced carrier and no abrupt alternatives before retaining complete word evaluation. C source templates require their current ordinary literal-pool effect receipt. Independently authored Jim source templates require their original text/effect world; a supplied C Native entry or C literal pool cannot donate it. Unknown substitutions, cross-family entries and withdrawn original worlds remain incomplete.

## Scope

Source-template component evaluation under the actual full LexerConfig and independently selected C Tcl or Jim source policy. The receipt closes this word evaluation only; it is not handler success, object identity, script entry, compiler-hook admission or a physical literal-table proof. Jim original text controls and C physical literal effects retain separate purposes.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust ownership contract. No interpreter or appliance observation is attached.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust ownership contract. No interpreter or appliance observation is attached.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust ownership contract. No interpreter or appliance observation is attached.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust ownership contract. No interpreter or appliance observation is attached.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust ownership contract. No interpreter or appliance observation is attached.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

This is a Rust ownership contract. No interpreter or appliance observation is attached.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

This is a Rust ownership contract. No interpreter or appliance observation is attached.

## Exact evidence

- `implementation-0` (implementation): [rust/tcl-compiler/src/command_binding/evaluated_word.rs](../../../../rust/tcl-compiler/src/command_binding/evaluated_word.rs). SHA-256 `46a8d10578a0e696b069be107a9483fb2aca22d1505ec9bf40eaa689e329c809`. Reviewed Rust owner for this implementation invariant; this file is not an executed provider capture.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/evaluated_word.rs](../../../../rust/tcl-compiler/src/command_binding/evaluated_word.rs), `SourceCommandBindings::walk_template_substitutions`: Select the authentic Text-only template and independent current literal-world effect boundary before retaining completed evaluation.
- [rust/tcl-compiler/src/command_binding/evaluated_word.rs](../../../../rust/tcl-compiler/src/command_binding/evaluated_word.rs), `command_binding::evaluated_word::tests::original_static_template_completion_requires_its_literal_world` (linked): Escaped and quoted opaque units and empty quotes retain their independently selected C5 or Jim original source worlds; unknown substitution, unknown entry and foreign source worlds remain incomplete.

A named test is a coverage binding, not a claim that it executed.

## Replay

The linked Rust selector is an implementation assertion, not a recorded native interpreter observation. No fresh native run is attached.
