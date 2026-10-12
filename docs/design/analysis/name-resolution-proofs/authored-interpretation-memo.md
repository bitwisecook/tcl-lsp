# naming.source.authored-interpretation-memo

Kind: `implementation-contract`

## Problem statement

Different consumers can repeat the same authored-source interpretation. Reusing a result by display, offsets, source digest alone or partial entry state could substitute another original frame or provider obligation.

## Question

Which complete immutable inputs permit bounded reuse of the existing authored-source driver result?

## Conclusion

Only a complete source/channel/configuration/frame/owned-entry/Registry match permits reuse. Native entries and oversized inputs remain uncached. Every lookup clones an independently mutable builder, while the bounded worker memo stays outside semantic equality and hashing.

## Scope

Authored-source interpretation with no retained native entry. Exact source bytes/channel, all lexer configuration, full original frame and entry contracts, selected policies/realm/compiler request and Registry semantic identity remain in the key. No physical provider freshness, table/activation/Normal/native-admission receipt is created. Timing and native observations are separate.

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

- [rust/tcl-compiler/src/command_binding/source_analysis_cache.rs](../../../../rust/tcl-compiler/src/command_binding/source_analysis_cache.rs), `SourceAnalysisCacheKey::at_entry`: Retain all complete immutable authored source inputs while refusing physical provider currency.
- [rust/tcl-compiler/src/command_binding/source_analysis_cache.rs](../../../../rust/tcl-compiler/src/command_binding/source_analysis_cache.rs), `lookup`: Return independent builder clones from a bounded worker-local immutable memo.
- [rust/tcl-compiler/src/command_binding.rs](../../../../rust/tcl-compiler/src/command_binding.rs), `SourceCommandBindings::analyse_source_in_frame_with_options`: Keep the shared source driver as the sole producer and retain only completed interpretations.
- [rust/tcl-compiler/src/command_binding/source_analysis_cache.rs](../../../../rust/tcl-compiler/src/command_binding/source_analysis_cache.rs), `command_binding::source_analysis_cache::tests::authored_source_memo_matches_complete_input_and_independent_builder_clones` (linked): An exact memo hit retains the same immutable state while a returned builder can mutate its table/layout inventories without affecting the cached result.
- [rust/tcl-compiler/src/command_binding/source_analysis_cache.rs](../../../../rust/tcl-compiler/src/command_binding/source_analysis_cache.rs), `command_binding::source_analysis_cache::tests::authored_source_memo_distinguishes_channel_config_entry_frame_and_registry` (linked): Different source channels/configurations/entry uncertainty/frame/Registry miss the cache, and retained native entries are ineligible.
- [rust/tcl-compiler/src/command_binding/source_analysis_cache.rs](../../../../rust/tcl-compiler/src/command_binding/source_analysis_cache.rs), `command_binding::source_analysis_cache::tests::authored_source_memo_bounds_history_and_refuses_oversized_sources` (linked): A fixed seventeen-source history retains sixteen entries, evicts the oldest exact key, keeps the newest exact key, and refuses oversized complete source inputs.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact Rust selectors exercise the scoped implementation premises. No Rust execution receipt or native provider observation is attached.
