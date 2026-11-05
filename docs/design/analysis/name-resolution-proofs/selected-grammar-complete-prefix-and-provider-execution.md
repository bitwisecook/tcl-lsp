# naming.source.selected-grammar-complete-prefix-and-provider-execution

Kind: `native-observation`

## Problem statement

A selected source grammar can retain a complete command prefix while the same whole source reports incomplete or raises a terminal parse error. Treating info complete as successful parsing or as permission to enter every prefix conflates source analysis with the actual provider operation. These two original inputs expose release-specific braced-variable grammar and different reached prefix effects.

## Question

Does the selected release consider the malformed braced-variable source incomplete, and which completed prefix/suffix effects occur when that same original source is evaluated? Does a terminal word-weld parse error remain complete while suppressing its suffix?

## Conclusion

For the exact braced-variable source, Tcl8.4.20/8.5.19/8.6.18 and Jim0.84-9-g5bac7c9 report complete and evaluate successfully, leaving before=1 and after=2. Tcl9.0.4/9.1.0 report incomplete, return missing close-brace for variable name, leave before=1 and leave after absent. Every provider reports the terminal word-weld source complete and returns extra characters after close-brace with after absent. The five C providers leave before=1 in that case; the recorded Jim process leaves before absent. Completeness, exact selected grammar and these reached script effects therefore require separate facts. Neither the complete flag nor the retained source prefix supplies a compiler, binding or Native execution grant. BIG-IP is not tested.

## Scope

Two exact counted ASCII script values under the retained public info complete query, then public uplevel #0 evaluation and separate post-evaluation before/after variable reads. Six actual CLI processes and twelve ROWs are retained: five measured C releases and the measured Jim build. Every process exits0 with empty stderr; catch reports each script outcome independently. VERSION, extra VALUE output, counted source bytes, exact receipts, invoked ELF and captured Tcl library .tcl input pins remain distinct. Loaded shared-library identity, full SDK/build replay, internal parser strategy, compiled bytecode, original headers, lifetimes, callbacks, frames, Host refusal metadata, compiler-purpose admission, current Rust assertions and TMM are unmeasured. These two cases establish no general Jim command-prefix effect rule.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6

Status: `observed`. Channel: Original CLI argv probe.tcl; two original hexadecimal byte values become counted ASCII script strings, public info complete then uplevel #0, followed by separate observed variable reads. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18.

The original process exits0 with empty stderr. The braced-variable case reports complete=1, script catch=0, before=1 and after=2. The terminal word-weld case reports complete=1, script catch=1, before=1 and after=MISSING. Exact error/result bytes remain in its raw stream. These are only the two measured public query/evaluation cases.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Original CLI argv probe.tcl; two original hexadecimal byte values become counted ASCII script strings, public info complete then uplevel #0, followed by separate observed variable reads. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

The original process exits0 with empty stderr. The braced-variable case reports complete=0, script catch=1, before=1 and after=MISSING. The terminal word-weld case reports complete=1, script catch=1, before=1 and after=MISSING. Exact error/result bytes remain in its raw stream. These are only the two measured public query/evaluation cases.

### jim

Status: `observed`. Channel: Original CLI argv probe.tcl; two original hexadecimal byte values become counted ASCII script strings, public info complete then uplevel #0, followed by separate observed variable reads. Version: jim 0.84-9-g5bac7c9.

The original process exits0 with empty stderr. The braced-variable case reports complete=1, script catch=0, before=1 and after=2. The terminal word-weld case reports complete=1, script catch=1, before=MISSING and after=MISSING. Exact error/result bytes remain in its raw stream. These are only the two measured public query/evaluation cases.

### bigip

Status: `not-tested`. Channel: No BIG-IP query/evaluation capture.

No actual BIG-IP response to these exact inputs is retained. Neither the six local processes nor source-only profiles establish BIG-IP/TMM behavior.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original](../../../../runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original)
- [runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/8.4.20](../../../../runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/8.4.20)
- [runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/8.5.19](../../../../runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/8.5.19)
- [runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/8.6.18](../../../../runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/8.6.18)
- [runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/9.0.4](../../../../runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/9.0.4)
- [runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/9.1.0](../../../../runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/9.1.0)
- [runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/Jim](../../../../runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original/625/Jim)
