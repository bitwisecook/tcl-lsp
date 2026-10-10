# Contract: native scalar getters

Primitive Int, Long, Wide, Double and Boolean queries preserve the original
object's cache, string residency, returned value and reached error effects.
Consumers select the getter purpose and actual object context independently of
source numeral grammar. A parsed value or a dialect label supplies no object,
interpreter, handler or execution permission.

The public observations are recorded in
[numeric-original-capi-scalar-publication-width.md](../analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md).
Expression Boolean purposes have their independent record in
[numeric-original-primitive-boolean-vs-expression-truth.md](../analysis/name-resolution-proofs/numeric-original-primitive-boolean-vs-expression-truth.md).
The broader number and expression contract is
[numeric-tower-and-expr-semantics.md](numeric-tower-and-expr-semantics.md).

## Owners and entry points

| Responsibility | Owner |
| --- | --- |
| Selected primitive conversion, cache transition and failure recipe | `tcl_syntax::scalar_getter::NativeScalarGetterProtocol` |
| Actual host layout and numeric thread state | `tcl_platform::NumericEnvironment`; `tcl_host_c_abi::NativeNumericEnvironment` |
| Reached C84/Jim conversion stages | `tcl_cmd_core::native_numeric::{fresh_c84_conversion, fresh_jim_conversion}` |
| Original Runtime object probe and failure rendering | `typed_value::{native_scalar_probe_with_environment, native_scalar_failure_presentation}` |
| Original VM object probe and failure rendering | `Value::{native_scalar_probe_with_environment, native_scalar_failure_presentation}` |

The Runtime and VM entry points validate the original live header and selected
protocol, inspect its genuine cache, obtain required target facts, and apply the
selected cache transition. A guest conversion failure can still change that
cache. Keep those changes when returning the failure. Foreign or retired headers
and missing capabilities return typed host refusals; consumers preserve the
first cause instead of publishing a guessed Tcl diagnostic.

## Query the actual target

`NumericEnvironment::c_integer_abi()` reports `NativeCIntegerAbi` fields:
`char_bits`, `int_bytes` and `long_bytes`. Sizes count C characters. The selected
actual host owns these facts. Source grammar, a cached integer, the compiler's
build target and an interpreter name cannot replace this query.

`NativeScalarGetterTarget::from_c_integer_abi(char_bits, int_bytes, long_bytes)`
checks the supported recipe: eight-bit C characters, four-byte C int and
eight-byte C long. Other layouts remain unavailable. The checked descriptor
establishes integer layout only; it does not authenticate an engine, original
object, source unit or selected handler. Captured LP64 widths make no claim for
LLP64 or wasm32.

Physical adapters use the shared query:

```rust
let target = tcl_cmd_core::native_numeric::scalar_getter_target(environment)?;
```

Backend owners can pass `Some(target)` to
`protocol.cached_conversion(kind, original_cache, Some(target))` or
`protocol.fresh_conversion_with_target(kind, original_bytes, Some(target))`.
Ordinary consumers use the original-object probe so they retain its header,
cache and environment checks. `requires_target(kind, original_cache)` describes
cache/getter obligations; C84 fresh numeric Boolean discovers its additional
target obligation in the shared fresh-conversion stage.

`c_integer_abi`, `reset`, `unsigned_c84` and `signed_long` default to
`NumericEnvironmentUnavailable::Target`. Required `state`, `unsigned` and
`double` methods must also report actual availability. Missing methods do not
mean zero errno or permission to reparse a reached cache. A target-dependent
cache refusal is terminal. Do not retry it through string conversion.

The host's numeric environment observes the current thread's errno. Conversion
receipts retain their before/after state and end pointer; reset is an explicit
selected operation. An interpreter-local mirror cannot replace that owner.

## Preserve the selected conversion

Long and Wide are independent getter kinds. In the measured LP64 C9 controls,
Long accepts positive unsigned edges that Wide rejects. The shared Long recipe
retains the parsed magnitude/cache separately from its wrapped returned value.
Fresh Jim Long uses the existing actual-host GetWide stages and then its selected
Long cast; it does not reuse a C Wide policy or a local integer parser. The
unavailable public Jim Int API cannot be substituted with Jim Long or a C Int
recipe.

C84 fresh Boolean follows its audited library `TCL_WIDE_INT_IS_LONG` branch:
`strtol` produces signed long, that value narrows to C int, and the narrowed
integer determines zero/one. An incomplete counted integer reaches `strtod`.
Neither Boolean stage resets errno or treats ERANGE as a Boolean guest error.
The pure explicit-target recipe shares the integer parser,
`number::native_signed64_saturating_integer` and
`number::native_int32_low_bits`; it supplies no C-call or errno effects.
Targetless successful numeric conversion refuses, while the shared Boolean-word
and definite-invalid recipes remain available.

Fresh String `4294967296` therefore produces primitive zero in the recorded C84
build; an already cached Wide `4294967296` produces one through its distinct
cache branch. The library's retained configuration selects that branch. A probe
translation unit's width macro does not establish the library configuration.
Counted completeness and byte-array materialisation retain their own recipes;
these examples grant no general numeric permission for NUL-bearing strings.

## Probe once, render only when required

The neutral probe returns `Result<Result<NativeScalarGetterValue,
NativeScalarGetterFailure>, ValueError>`. The outer error is host unavailability;
the inner error is the reached guest conversion failure. Neutrality means no
guest diagnostic publication, not absence of cache changes or selected C-call
effects.

For an internal Runtime nullable adapter, the composition is:

```rust
let outcome = native_scalar_probe_with_environment(
    original, dialect, kind, Some(environment),
)?;
if let Err(failure) = outcome {
    if has_live_interpreter {
        let diagnostic = native_scalar_failure_presentation(
            original, dialect, kind, failure,
        )?;
        // The consuming adapter publishes this reached diagnostic.
    }
}
```

The VM uses the corresponding methods on the same original `Value`. A live
interpreter renders the already observed failure once. A NULL-interpreter route
returns the conversion status without rendering or guest publication. It still
validates the actual object/target and retains probe effects. Neither route
replays the getter after an error. Write a C output parameter only on success;
preserve its sentinel on failure.

`failure_requires_original_string(kind, failure)` returns `Some(true)` for
selected operand-dependent diagnostics, `Some(false)` for selected constants,
and `None` when the stage is unmodelled. For `true`, the backend renderer uses
the checked original String getter before `failure_presentation`. For `false`,
it uses `failure_presentation_without_original_string`; that method refuses
operand-dependent rendering. An unavailable obligation stays a host refusal.

This distinction matters for cached NaN: the measured C85+ Boolean constant
failure keeps its absent String. Invalid spelling and selected cached
noninteger errors need their original operand bytes. Calling a String getter
before consulting the obligation would change an observed failure window.

## Keep raw Boolean separate from expression truth

`NativeScalarGetterValue::Boolean` carries `NativeBooleanGetterValue`.
`returned_integer()` exposes the exact public primitive output. The measured Jim
cached integer 17 returns 17; cached `4294967296` returns zero after the selected
C-int cast. CAPI adapters publish that integer without normalising it.

`is_true()` is an explicit logical projection of the returned primitive integer
only. It cannot implement Jim ExprBool, unary-not, logical operands, command
conditions or a compiled Boolean expression. The original expression controls
return true for Jim cached `4294967296` even though its primitive getter returns
zero. NaN routes also differ in success, diagnostic and String effects. Select
the independent expression purpose and its original operand/context whenever a
consumer asks one of those questions.

## Add or migrate a consumer

Retain the original object and selected physical getter context through the
query. Pass its actual host environment to the probe, preserve the returned
cache effects, and distinguish guest failure from typed host refusal. Reuse the
shared failure renderer and output projection required by the consumer's
purpose. An authored source value can use a separately admitted analytical
model; it cannot donate the physical object or target context.

Extend a missing conversion or diagnostic obligation in the shared owner, then
supply the necessary host capability and backend adapter. Keep unknown layouts
unavailable. Do not duplicate integer parsing, infer ABI from a profile, render
from borrowed context-free bytes, or turn a failed cache query into fresh input.

Discriminating controls are in
[Syntax target tests](../../../rust/tcl-syntax/src/scalar_getter/target_tests.rs),
[CmdCore host-stage tests](../../../rust/tcl-cmd-core/src/native_numeric_float_tests.rs)
and [VM target tests](../../../rust/tcl-vm/src/value_scalar_target_tests.rs):

- `original_jim_cached_boolean_returns_the_measured_public_integer` compares the
  raw public result and preserved cache/residency.
- `original_long64_public_value_cache_and_live_failure_fields_match_captures`
  keeps Long's selected value, cache and live diagnostic windows.
- `original_c84_fresh_boolean_narrows_signed_long_before_truth_and_cached_wide_does_not`
  distinguishes the fresh and cached branches.
- `c84_boolean_preserves_reached_errno_and_refuses_missing_target_before_numeric_calls`
  checks reached stages, no implicit reset and terminal missing-target refusal.
- `original_cached_nan_boolean_failure_keeps_absent_string_and_public_error_fields`
  distinguishes constant failure from operand-dependent String access.
- `descriptive_target_does_not_admit_retired_or_foreign_original_cache` keeps
  layout facts independent of original-header admission.

The unchanged `primitive_boolean_storage_and_followup_wide_match_all_native_fixtures`
controls in [Runtime](../../../runtime/rust/src/typed_value.rs) and
[VM](../../../rust/tcl-vm/src/value_scalar_tests.rs) retain all 77 original
storage/followup rows. Native process observations, source/policy controls and
successful Rust assertions remain separate validation scopes in the linked
proof records.
