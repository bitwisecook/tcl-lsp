# naming.namespace-ensemble.original-nested-option-positions

Kind: `native-observation`

## Problem statement

Nested namespace ensemble operations have different required positional operands and option boundaries. A dash-named required command can be data; treating it as an option changes the selected operation. An exists capability failure cannot establish that bare ensemble creation is unavailable.

## Question

Where do options begin in the fixed namespace ensemble create/configure/exists source forms, and what do dash-named commands and independently caught bare-create controls actually return?

## Conclusion

C8.5–9.1 accept create options at nested operand0, consume configure command data before options at operand1, and treat exists as a command-name query without an option scan. Dash-named required configure/exists commands retain data behavior. The extra exists argument reports NONE in C8.5 and TCL WRONGARGS in C8.6–9.1. C8.4 has no namespace ensemble operation in these captures. Current Jim supports bare ensemble creation and its exported invocation, refuses the C-style optioned create shape, and declines configure/exists with its only-create-supported error. Its exists-gated capture supplies no create answer by itself.

## Scope

Twelve actual source-file CLI processes retain twelve queried versions and 134 labelled caught observations across two exact ASCII scripts. Capture167 has an exists-gated option body; capture168 independently catches fourteen labels per provider, including bare creation and invocation. Each script/provider process shares its own ordered created commands and namespace state; these are not isolated original object-vector cases. Actual result strings, errorCode and capability boundaries remain limited to the fixed source. No opaque names, physical object/header/cache/pointer, compiler admission, arbitrary handler, whole-script Normal, edit equivalence or BIG-IP answer follows.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Two fixed ASCII LF source-file CLI scripts; one actual interpreter process per script/provider, ordered caught source forms. Version: tcl8.4 8.4.20.

The capability and all independently caught ensemble forms fail because namespace ensemble is unavailable. Capture167 skips the option body; capture168 records every fixed refusal. No option-position or bare-creation behavior is inferred for C8.4.

### tcl8.5

Status: `observed`. Channel: Two fixed ASCII LF source-file CLI scripts; one actual interpreter process per script/provider, ordered caught source forms. Version: tcl8.5 8.5.19.

Create accepts options immediately. Configure consumes its required command name first, including dash-named -map and -namespace commands, then options; the fixed map queries return x ::list. Exists consumes its command name and does not scan trailing options. Bare creation and invocation succeed. The extra exists argument returns errorCode NONE. Invalid -command for configure and -namespace for create retain their distinct actual option error rosters in the streams.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Two fixed ASCII LF source-file CLI scripts; one actual interpreter process per script/provider, ordered caught source forms. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Create accepts options immediately. Configure consumes its required command name first, including dash-named -map and -namespace commands, then options; the fixed map queries return x ::list. Exists consumes its command name and does not scan trailing options. Bare creation and invocation succeed. The extra exists argument returns TCL WRONGARGS. Invalid -command for configure and -namespace for create retain their distinct actual option error rosters in the streams.

### jim

Status: `observed`. Channel: Two fixed ASCII LF source-file CLI scripts; one actual interpreter process per script/provider, ordered caught source forms. Version: jim 0.84-9-g5bac7c9.

Capture167 reports only [namespace ensemble create] is supported from exists, then skips subsequent forms. Capture168 directly records bare create code0 with r2286_nestedBare and invocation code0 with payload. C-style optioned create returns arity refusal; configure and exists return only-create-supported refusals. The ensemble surface is not wholly unavailable.

### bigip

Status: `not-tested`.

No BIG-IP attempt or corresponding source-file ensemble answer is attached.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167)
- [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168)
