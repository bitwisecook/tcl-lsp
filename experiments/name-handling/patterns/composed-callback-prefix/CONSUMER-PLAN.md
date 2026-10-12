# Can the original source-reference consumer accept this pair?

Status: current API and test definitions inspected. The proposed assertions
below have **not** been compiled or executed. No Rust, Cargo, canonical proof,
or native capture files were changed. The existing native question remains
“What setup completion, scheduled callback error and final observed-variable
lookup does this exact source produce?” Static source selection is independent.

## Which current entry points own this source?

The public compiler facade is
`registry_invocation::source_structure::source_authored_command_prefix(source,
analysis, producer)`. It takes the complete source image and actual original
`SegmentedCommand` for `::makePrefix my tick {a b}`. It does not take the outer
`set`, `namespace code`, `after`, or a reconstructed `my tick {a b}` string.
Its guard requires the retained analysis input, identical full document bytes
and lexer configuration, body configuration and retained realm/input agreement.

`source_registry_words` independently selects the actual builder schema.
`command_prefix_targets_from_word` in core then requires the builder's
`BUILDS_COMMAND_PREFIX`, `ListArguments { from: 0 }`, zero argument offset and
accepted frozen arity. The authored target's schema must have
`TCLOO_SELF_DISPATCH`; its original argument zero supplies the `tick` selector.
`namespace code` is a selected `WRAPS_COMMAND_PREFIX`/`DEFERS_BODY` wrapper and
retains its own original script operand. These registry facts come from the
actual selected source schema, not command spelling.

`stored_callback_targets` and `collect_stored_callback_writes` separately
inventory all selected same-frame writes. A nonliteral VarWrite operand sets
`has_scope_alias`; all stored targets are then withdrawn. The earlier authored
builder receipt can remain valid in the adversarial input. Therefore do **not**
assert that `source_authored_command_prefix` itself returns `None` merely because
`set $destination` occurs later. Assert withdrawal at the source-reference
consumer separately.

`source_produced_command_prefix_words` is a different facade for a builder in
an independently selected deferred operand. The stored `after ... $cb` word
supplies no direct `[builder]` parent. These consumer checks must not invent an
`after` parent or require a produced-prefix receipt as a substitute for authored
source data. If a separate produced-prefix query at this inner invocation is
investigated, any retained parent must come from its actual containing original
wrapper, with its own obligations; absence is a separate observation.

## Which exact fixtures should a consumer read?

- [Positive](probes/positive.tcl): preserve complete bytes and identity, including
  the existing final native question line. Desired static result: declaration
  plus the original inner-builder `tick` selector.
- [Unknown writer](probes/unknown-writer.tcl): preserve complete bytes and identity.
  Desired static result: declaration retained, original callback selector withheld.
  The exact native call passes `cb`; that runtime value must not be imported as
  a static proof for the method's variable `destination`.
- [Private positive](fixtures/private-positive.tcl): a **new**, unexecuted
  consumer-only fixture with explicit `unexport tick`, empty method body and no
  native invocation. Desired readonly result: declaration plus captured internal
  selector; private export cannot suppress internal `my`. This fixture has no
  native oracle or canonical question and does not modify either existing input.
  Exact fixture SHA-256: `ebff652d09a1e981e90ced669dd0e181c9d66db749cf9c1681ee16c5801d1983`.

## What exact shared-owner assertions are proposed?

The following draft uses current compiler APIs from the existing core
`references.rs` test module. A compiler-local equivalent uses `crate::` paths
instead of the external `tcl_compiler::` paths shown here.
The analysed method's `body_span` supplies the retained original region;
`original_single_source_command_substitution` supplies each actual containing
word. The final segmentation retains the original full-image address space.
No searching by `tick` spelling establishes the target. Load the exact files
from the core crate with these constants; no copied source string replaces them:

```rust
const POSITIVE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"),
    "/../../experiments/name-handling/patterns/composed-callback-prefix/probes/positive.tcl"));
const UNKNOWN_WRITER: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"),
    "/../../experiments/name-handling/patterns/composed-callback-prefix/probes/unknown-writer.tcl"));
const PRIVATE_POSITIVE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"),
    "/../../experiments/name-handling/patterns/composed-callback-prefix/fixtures/private-positive.tcl"));
```

```rust
use tcl_compiler::registry_invocation::source_structure::{
    original_single_source_command_substitution, source_authored_command_prefix,
    source_registry_words,
};

fn composed_builder(source: &str, analysis: &AnalysisResult)
    -> (tcl_compiler::segmenter::SegmentedCommand,
        tcl_lexer::NativeWord, tcl_lexer::NativeWord)
{
    let input = analysis.resolved_input.as_ref().unwrap();
    let config = input.lexer_config();
    let body = analysis.all_classes["::C"].methods["setup"].body_span;
    let plan = tcl_lexer::native_script_words_in(
        tcl_lexer::SourceImage::document(source), body, config,
    ).unwrap();
    assert!(plan.fatal_tail.is_none());
    let setter = &plan.commands[0];
    assert_eq!(setter.words[0].bytes(), b"set");
    assert_eq!(setter.words[1].bytes(), b"cb");
    let wrapper = original_single_source_command_substitution(
        &setter.words[2],
    ).unwrap();
    assert_eq!(wrapper.command().words[0].bytes(), b"namespace");
    assert_eq!(wrapper.command().words[1].bytes(), b"code");
    let child = original_single_source_command_substitution(
        &wrapper.command().words[2],
    ).unwrap();
    assert_eq!(child.command().words.len(), 4);
    assert_eq!(child.command().words[0].bytes(), b"::makePrefix");
    let selector = child.command().words[2].clone();
    let fixed = child.command().words[3].clone();
    let span = child.command().span;
    let commands = tcl_compiler::segmenter::segment_commands_with_offset_and_config(
        &source[span.as_range()], span.start(), config,
    );
    assert_eq!(commands.len(), 1);
    (commands[0].clone(), selector, fixed)
}

let (producer, selector, fixed) = composed_builder(source, &analysis);
let input = analysis.resolved_input.as_ref().unwrap();
let context = input.context_registry();
let words = source_registry_words(source, &analysis, &producer)
    .expect("the actual aliased builder must have source schema");
assert_eq!(words.with_source_schema(&context, |schema| {
    schema.semantics.traits.contains(tcl_registry::Traits::BUILDS_COMMAND_PREFIX)
        && schema.semantics.native_result == Some(
            tcl_registry::native_result::NativeResultContract::ListArguments { from: 0 }
        )
        && schema.semantics.argument_offset == 0
        && schema.facts().arity_accepts_frozen_arguments() == Some(true)
}), Some(true));
let prefix = source_authored_command_prefix(source, &analysis, &producer)
    .expect("conditional original builder metadata is retained");
assert_eq!(prefix.producer().site().offset, producer.argv[0].span.start());
assert_eq!(prefix.producer().original_words()[0].bytes(), b"::makePrefix");
assert_eq!(prefix.producer().original_words().len(), 4);
assert!(!prefix.producer().lineage().is_empty());
assert_eq!(prefix.original_head().original_word().unwrap().bytes(), b"my");
assert_eq!(prefix.original_argument(0).unwrap().span(), selector.span());
assert_eq!(prefix.original_argument(0).unwrap().bytes(), b"tick");
assert_eq!(prefix.original_argument(1).unwrap().span(), fixed.span());
assert_eq!(prefix.original_argument(1).unwrap().bytes(), b"{a b}");
assert_eq!(prefix.argument_origin(0), Some(
    tcl_compiler::registry_invocation::InvocationWordOrigin::Written(2)
));
assert_eq!(prefix.argument_origin(1), Some(
    tcl_compiler::registry_invocation::InvocationWordOrigin::Written(3)
));
assert_eq!(prefix.with_source_schema(&context, |schema| {
    schema.semantics.traits.contains(tcl_registry::Traits::TCLOO_SELF_DISPATCH)
}), Some(true));
assert!(prefix.matches_source_context(
    &tcl_lexer::SourceImage::document(source), input.lexer_config(), &context
));
assert!(!prefix.matches_source_context(
    &tcl_lexer::SourceImage::native(source.as_bytes()), input.lexer_config(), &context
));
```

The `NativeWord::bytes()` assertions concern original lexical bytes; `{a b}`
includes braces here. They do not claim an evaluated runtime list object.
The same builder-level assertions are proposed for both preserved native inputs:
unknown later writes withdraw sink selection, not the original earlier words.
If current source advice cannot select the aliased builder in this composed
method context, report that failure as a consumer/source-owner gap.

## What exact reference assertions are proposed?

Build each full source independently with `Analyser::new().with_resolved_input`
and `ResolvedAnalysisInput::new(profile, profile, context, config)`, using
`resolve_environment(release).analyser_profile()`, its
`default_context_registry()` and `LexerConfig::for_file_grammar(profile.grammar)`
for `tcl8.6`, `tcl9.0` and `tcl9.1`. Do not analyse a detached method body.

In the existing `references.rs` test module, use its actual `references`,
`LineIndex` and `span_to_range` helpers:

```rust
let declaration = analysis.all_classes["::C"].methods["tick"].name_span;
let (_, selector, _) = composed_builder(source, &analysis);
let index = LineIndex::new(source);
let at = index.position_at_utf16(declaration.start(), source);
let refs = references(source, profile, at.line, at.character.get(), &analysis, true);
let declaration_range = span_to_range(source, &index, declaration);
let selector_range = span_to_range(source, &index, selector.span());
assert!(refs.contains(&declaration_range));
assert_eq!(refs.contains(&selector_range), expected_callback);
let refs_without_decl = references(
    source, profile, at.line, at.character.get(), &analysis, false,
);
assert!(!refs_without_decl.contains(&declaration_range));
assert_eq!(refs_without_decl.contains(&selector_range), expected_callback);
```

`expected_callback` is true for the positive and private-positive fixtures and
false for unknown-writer. Compare complete UTF-16 ranges, not just line numbers
or result counts. For private-positive additionally assert
`analysis.all_classes["::C"].unexports.contains("tick")`; current definitions
`wrapped_my_callback_cursor_resolves_private_dispatch` and
`list_built_self_callback_cannot_reach_an_unexported_method` cover the internal
and external visibility ingredients, but neither composes this aliased stored
fixture. An independently derived private external control replacing the
builder's `my` operand with `[self]` should withhold its selector; its changed
bytes have their own consumer-fixture identity and no native result.

## Which exact negative barriers should remain independent?

Derive each mutation into a separately named consumer fixture and analyse its
whole changed source afresh. Retain its bytes/hash before treating it as tested.
These are proposed mutations, not retained executions or new source-site counts.

| Separate mutation / retained-input check | Assertion boundary |
| --- | --- |
| Exact unknown-writer input | Original authored builder metadata may survive; stored selector is absent from references. |
| Insert `opaque $cb` before `after idle $cb` | Missing selected source schema blocks stored targets; no callback selector. |
| Insert `if 1 {set cb {my unrelated}}` before the sink | Same-frame nested writes count twice; no old stored selector. |
| Insert `upvar 1 other cb` before the sink | Selected scope-alias effect blocks stored targets. |
| Replace inner producer with `::makePrefix my $method {a b}` | Dynamic selector gives no literal `tick` source target. |
| Replace inner producer with `::makePrefix {*}$prefix` | Expanded unknown argv cannot donate an original selector. |
| Replace inner producer with `::makePrefix my tick {a b}; list noop` inside the substitution | Multiple original child commands decline the single-builder projection. |
| Add `Suffix` after the value's outer command substitution | Compound token declines the one-whole-value prefix. |
| Replace sink operand `$cb` with `{*}$cb` | Expanded sink cannot invent an eligible original callback word. |
| Replace alias creation with `proc ::makePrefix args {}` | Written builder name cannot inherit list-builder traits. |
| Define `proc ::list args {}` before creating the alias | Selected shadow of the alias target cannot inherit registry list facts. |
| Remove or shadow the actual `my` target before the relevant original region | Require the selected target schema; no nominal-name fallback. |
| Pass changed complete source to the original analysis/producer | `source_authored_command_prefix` returns `None`; readonly scan withholds the callback selector. |
| Clear `resolved_input`, or replace it with a foreign registry/context receipt | No source-owner fallback and no callback selector. |
| Toggle retained `body_lexer_config.expand_syntax` while keeping the original input | Authored-prefix facade and retained dispatch context refuse. |
| Retain a fresh `tcl8.4` context while selecting modern grammar | `my`/TclOO availability cannot come from nominal grammar; intended modern callback target withheld. |
| Query the retained prefix with a native-channel image or foreign full context | `matches_source_context`/`with_source_schema` refuse; no donated ownership. |

Existing `authored_prefix_data_keeps_builder_point_without_a_deferred_parent`,
`authored_prefix_keeps_alias_origins_and_known_target_barriers`,
`callback_source_roles_keep_actual_availability_and_original_builder_geometry`,
`callback_source_targets_withdraw_known_shadows_and_incomplete_stored_writes`
and `stored_callback_prefixes_count_nested_same_frame_reassignment` ground these
API boundaries. Their definitions were inspected; no tests ran in this job.
Where a negative prevents class/declaration parsing, assert selector abstention
without falsely requiring a parsed declaration. Missing owner/context checks
may separately preserve presentation declarations while withholding callbacks.

These assertions can establish conditional readonly geometry only. No retained
source prefix issues a later entered frame, native dispatch result, current
editable target, rename permission or optimisation authority. Report wrong
selectors and missed positive selectors independently of native observations.
