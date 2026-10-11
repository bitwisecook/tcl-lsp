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
| Original cache layout and reached primitive eligibility | `NativeScalarGetterProtocol::{validate_cache_abi, validate_primitive_abi}`; `tcl_cmd_core::native_numeric::scalar_getter_recipe_abi` |
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

Runtime object issuance retains its actual engine, Host and queried ABI,
checks eight-bit C characters, four-byte C int and four- or eight-byte C long,
and compares the output widths with that Runtime's FFI types. This descriptive
output identity is independent of the reached getter recipe.

`NativeScalarGetterTarget::from_c_integer_abi(char_bits, int_bytes, long_bytes)`
continues to admit only the selected eight-bit/four-byte/eight-byte recipe
capability. An admitted output layout with four-byte long does not widen that
capability. Neither descriptive layout nor the checked recipe target supplies
an engine, original object, source unit or selected handler. The original LP64
getter captures make no claim for LLP64 or wasm32.

Physical adapters obtain the recipe target only when selected:

```rust
let target = tcl_cmd_core::native_numeric::scalar_getter_target(environment)?;
```

Backend owners can pass `Some(target)` to
`protocol.cached_conversion(kind, original_cache, Some(target))` or
`protocol.fresh_conversion_with_target(kind, original_bytes, Some(target))`.
Ordinary consumers use the original-object probe so they retain its header,
cache and environment checks. The probe separately validates the actual
`NativeScalarGetterStage::Cached(original_cache)` or reached `Fresh` stage via
`scalar_getter_recipe_abi`. `requires_target(kind, original_cache)` selects an
additional recipe-target obligation; it is not the complete eligibility check.
C84 Boolean words and definite-invalid spellings remain before its fresh numeric
ABI obligation. A failed reached recipe cannot fall through to reparsing.

`validate_cache_abi` checks a retained C84 native-long primary against the actual
long width. Runtime binding validates an existing cache before publishing its
issuer; later cache adoption uses that same retained ABI before mutation,
without a fresh Host query. The engine scalar-carrier producer binds its already
issued owner before initial cache adoption. Neutral constructors remain
separately unissued and supply no physical ABI permission.

With four-byte long, genuine cached C84 long or wide integer values in signed
32-bit range retain the selected Long direct-return equation. Wider wide values
remain separate; a C84 native-long primary outside that range refuses. Fresh
C84 Int, Long, Wide and numeric Boolean still require their original build's
parser/cache or `TCL_WIDE_INT_IS_LONG` recipe and refuse on this layout. Double's
actual `strtod` stage is independent. Modern C/Jim Long64 and cached Jim raw
Boolean keep their separately measured eight-byte-long capability.

`c_integer_abi`, `reset`, `unsigned_c84`, `signed_long` and `unsigned_long` default to
`NumericEnvironmentUnavailable::Target`. Required `state`, `unsigned` and
`double` methods must also report actual availability. Missing methods do not
mean zero errno or permission to reparse a reached cache. A target-dependent
cache refusal is terminal. Do not retry it through string conversion.

The host's numeric environment observes the current thread's errno. Conversion
receipts retain their before/after state and end pointer; reset is an explicit
selected operation. An interpreter-local mirror cannot replace that owner.

`WasiHost` supplies an environment only for `wasm32-wasip1`, using that compiled
guest's libc types, conversion functions and errno cell. Browser and preview-2
routes remain independently unavailable. The SDK and standalone Rust guest
observations in
[numeric-wasi-libc-environment.md](../analysis/name-resolution-proofs/numeric-wasi-libc-environment.md)
record environment facts. Applying them to a Runtime requires its own selected
libc/link closure and separate Runtime validation; they establish no original
Tcl Long32, expression, object or execution recipe.

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

## Linked precision callbacks

The Runtime precision callback enters the original `NativeOperationScope` before
reading the selected receiver or changing precision and pending guest errors.
Its checked installer and every callback caller preserve an outer
`NativeExecutionError` separately from `VariableTraceGuestOutcome`. A safe
interpreter rejects a write before either String or numeric access.

For C84, the original cell supplies its borrowed String reference. An
allocation-only lease preserves memory safety without adding a native reference
around the getter. `tcl_cmd_core::native_numeric::c84_precision_from_host` queries
the selected eight-bit/four-byte/eight-byte recipe target and calls
`NumericEnvironment::unsigned_long(original, 0, 10)`. This is decimal `strtoul`
without an errno reset. The original NULL getter outcome selects an empty C
string and still reaches that worker. The shared validator requires the full C
string to be consumed, narrows through the existing C-int owner and accepts
precision 1 through 17. C84 restores the previous precision on a safe or invalid
write.

C85/C86 use the error-neutral Int probe on the original object and accept 0
through 17 through `number::double_precision_from_native_integer`. They do not
render a primitive conversion error or obtain String merely to validate a
cached Int or rejected Double. A cached recipe can proceed without an optional
numeric environment only when its selected protocol requires none. These
versions leave the written cell in place on a safe or invalid write.

The Runtime checks original currency and object liveness around reached getters,
conversion, publication and retirement. Its selected read/restore and
registration workers ignore their genuine guest setter failures where the
original callback does; a host refusal remains an outer error and stops later
precision or guest-error publication. The checked constructor consumes installer
failure, while compatibility installers retain the first typed cause.

`number::parse_double_precision` remains an explicit pure text utility. It does
not observe the original cell, libc errno, getter cache or interpreter currency.
The VM precision adapter uses the same selected C84 decimal worker and
C85/C86 neutral Int policy through its original entered operation scope. Its
safe-mode check precedes access; read publication and C84 restoration retire
displaced values before checking currency. Its authored math simulation remains
a separate text policy. Neither the decimal worker nor a descriptive ABI receipt
supplies an original Tcl32 getter recipe.

The source controls in
[Runtime precision stage tests](../../../runtime/rust/src/interp/native_precision/original_stage_tests.rs)
check safe-before-getter order, decimal C-int narrowing, borrowed reference role,
NULL-getter worker selection, missing capabilities, original-cache access and
callback/retirement refusal. Their software scope is recorded under
[embedding host publication](../analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md).
The independent six-provider
[precision unset observations](../analysis/name-resolution-proofs/variable-original-precision-unset-publication.md)
retain their public unset purpose; they do not establish precision-write getter
or callback chronology.

## Checked numeric stages

`tcl_platform::CheckedNumericEnvironment` forwards the independently supplied
environment under one before/after checker and the caller's ABI observation
cell. It selects no parser, conversion recipe or errno reset. A changed observed
ABI refuses before continuation. Runtime's checked environment delegates this
forwarding and retains its actual operation receipt and full first cause.

The VM `Value` conversion and cache owner accepts an optional stage checker.
Public pure probe/getter APIs pass no checker and keep their primitive purpose.
Original execution uses `interp::native_numeric_ops::OriginalNumericOps` with
the genuine existing `NativeOperationScope`. Checked scalar/number and Boolean
children share the same conversion kernel. They check after reached getters,
Host numeric stages and old-primary retirement before later cache or precision
publication. Primary retirement runs outside the representation borrow; the
checker adds no native object reference.

A checked Jim fresh conversion retains the object context's actual Host. The
same worker receives checked forwarding at each numeric stage; an original
stage error remains typed independently of the neutral availability result.
An outer supplied environment cannot replace that retained worker. The pure
Jim context entry points retain their explicit primitive purpose.

The precision observer borrows the selected cell's original header with an
allocation-only lifetime lease. It adds no native object reference before the
C8.4 String getter or C8.5/C8.6 neutral integer getter, and retires that lease
before the final original-currency check and ambient precision publication.
Safe-interpreter refusal precedes both getters. A C8.4 NULL read still reaches
the independently selected decimal unsigned-long stage with empty input.

The Platform checker returns only `NumericEnvironmentUnavailable`. Runtime and
VM retain their full typed execution cause in their actual operation owner and
check that owner after the forwarding stage, before using a result or presenting
a guest error. Do not construct a Host cause from the neutral enum or diagnostic
text. Missing numeric capability remains independent of a retained Host refusal.

VM `Value` does not expose Runtime's extension object-type updater/free issuer.
Its software controls exercise actual Host numeric forwarding, original header
and cache transport, and the stage checker after primary retirement. They do
not establish an extension callback chronology or a new physical object issuer.
The existing provider comparisons retain their own narrower measured purposes.

## VM consumer interfaces

`Vm::authored_numeric_context` selects the actual configured Logical simulation
without querying a native Host. `numeric_policy_context` is callback-free
policy data and supplies no environment or execution receipt. Use these for
routing; a native branch enters `OriginalNumericOps` before any Host query.
The explicit pure `numeric_context` route still queries its supplied environment
and cannot replace the checked execution facade.

`OriginalNumericOps::entered` captures or inherits the actual operation scope.
`getter`, `probe`, `number` and `failure` retain the same original object,
selected physical dialect, Host and ABI observation. Jim getter/probe ingress
binds the actual retained interpreter context centrally. `dialect` exposes the
captured policy for an independent consumer protocol join. `current` checks
the original interpreter receipt for ambient stages. `current_for_value`
additionally checks the exact original header and retains a genuine header
refusal in the VM's existing typed first-cause owner. It supplies no new object,
getter, frame or runtime identity.

Operand-dependent environment queries use `with_environment_for_value`.
It shares the facade's captured Host and ABI observation cell and checks that
same original header before and after each reached numeric worker, including
failed workers. Getter, probe, Number, Boolean, failure and String methods use
this operand check; ambient precision and nonfinite error queries keep their
independent operation-only purpose. A frame adapter must not recreate a local
environment checker or issue a fresh receipt after a callback.

The legacy ValueOps, ListIndex, duration and info integer seams return
`ValueError`. Their shared facade uses a constant unavailable-operation token
only to unwind a retained Host cause. That token is not a reconstructed cause.
Consumers must propagate it before Guest rendering or later work; the real
VM completion boundary checks `native_access_refusal` and the public checked
execution boundary returns the full retained first cause. Genuine primitive
Guest scalar records keep their existing typed fields. Quiet ListIndex misses
and Event option fallback may swallow only genuine Guest failure.

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

The source controls
`queried_output_layout_does_not_supply_long64_or_a_c84_build_recipe` and
`c84_primary_width_and_reached_fresh_recipe_are_independent` in
[Syntax ABI tests](../../../rust/tcl-syntax/src/scalar_getter/abi_tests.rs)
check layout/cache/recipe boundaries without an original Tcl32 getter or engine
claim. Discriminating original-field controls are in
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

The `primitive_boolean_storage_and_followup_wide_match_all_native_fixtures`
controls in [Runtime](../../../runtime/rust/src/typed_value.rs) and
[VM](../../../rust/tcl-vm/src/value_scalar_tests.rs) retain all 77 original
storage/followup rows. Native process observations, source/policy controls and
successful Rust assertions remain separate validation scopes in the linked
proof records.

Original VM String materialisation uses `OriginalNumericOps::string` (or the
ValueError-compatible `string_value`) and `Value::native_string_bytes_checked`.
The selected protocol must match the captured actual invocation; the same
entered scope and exact original header check Host formatter acquisition,
formatter build and format, compound children, and publication of String bytes.
Actual header withdrawal remains a typed first cause even when the formatter
also returns an unavailable result. The existing worker's optional checker
receives each reached original value. The canonical facade checks both its
original root and that actual child before further work; a live root cannot
replace a retired child's header or cause after backing retirement. Compound backing leases
retain original children without adding native object references. A refused
child does not publish its bytes or run later child updaters. Previously reached
child effects remain observable.

The public Value String APIs are independent adapters with no entered-operation
receipt. Their explicit formatter capability supplies no VM context authority.
A consumer with an actual VM must use the checked adapter. ValueError is only
a compatibility unwind carrier: the actual VM retains its full first cause and
the execution boundary checks it before guest presentation.

The VM software controls release the actual original value owner during a
supplied numeric or formatter callback while retaining only allocation leases.
They check first-cause transport and suppression of later workers or cache
publication. They do not establish a Runtime extension object issuer or an
external Tcl updater/free callback chronology.
