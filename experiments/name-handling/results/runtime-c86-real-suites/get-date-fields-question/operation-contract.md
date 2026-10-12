# GetDateFields operation contract

This addendum makes the [sealed C86 proposal](proposal.md) concrete; it is an unexecuted implementation contract, not new oracle evidence. Original captures, proposal and [seal](sealed-files.json) remain unchanged. The five proposed owners and C86-only admission remain the implementation scope.

Retain one actually entered operation across the original argument vector, reached table/row getters and result publication. Nested work must inherit `NativeOperationScope::enter` currency; a changed-and-restored execution context cannot issue replacement currency. Actual C86 worker admission is independent of a lease, numeric getter or source profile.

[Runtime operation currency](../../../../../runtime/rust/src/interp/native_operation_currency.rs) supplies `NativeOperationScope::currency()` and `NativeOperationCurrency::ensure_current_or_refuse()`. Check before each reached stage, after its getter even on error, before the next row/cell, and before publication. Retain the first typed host cause before temporary cleanup; later guest errors or cleanup must not replace it.

[Runtime ValueCoercion](../../../../../runtime/rust/src/value_ops.rs) supplies `enter`, `check`, `after` and `scalar(NativeScalarGetterKind)`. Use Wide for seconds/reached transitions and Int for changeover/offset, preserving original central getter diagnostics. Inspect the actual seconds primary only after both top-level getters; preserve the sealed bignum rejection order and original overflow literal with NONE code.

Generic `ValueOps::list_elements/list_index/new_dict_checked` are insufficient by themselves: the current Runtime list route passes no operation currency and the Dict route has no stage-check callback. The proposed `NativeClockDateOps` adapter must use the existing physical helpers below, without changing those generic APIs or adding a second parser.

[Runtime List helpers](../../../../../runtime/rust/src/list.rs):

- `list_elements_native_with_currency(value, protocol, Option<&NativeOperationCurrency>) -> Result<Vec<*mut TclObj>, ValueError>`; supply the original currency.
- `list_elements_native_with_stage_check(value, protocol, currency, Option<&dyn Fn() -> Result<(), ValueError>>)` adds an independent original-storage check.
- `list_elements_native_with_stage_handlers` additionally accepts `retain_error: Option<&dyn Fn(ValueError) -> ValueError>` to retain refusal before unadopted-member cleanup.

The adapter must preserve C86's first-row then binary-search access order. Do not preconvert every row, read DST/extra cells, or turn a missing/unsafe cell extent into an invented guest error. Only reached selected children may supply transitions, offset or timezone name.

[Runtime object leases](../../../../../runtime/rust/src/obj.rs) supply `NativeObjectLifetime::retain/as_ptr` without changing native refcounts; retirement still invalidates the primary. Lease the original table, reached rows/cells and selected name before retaining raw pointers across effects. After an intended successful List conversion, retain/check the selected backing via `NativeListBacking::check_generation` and an appropriate original-primary receipt. Expected getter coercion must not be mistaken for external replacement; a lease alone does not establish liveness or backing currentness.

[Runtime Dict helper](../../../../../runtime/rust/src/dict.rs): `new_dict_obj_native_with_stage_check(pairs: &[(*mut TclObj, *mut TclObj)], bucket_count: Option<usize>, protocol: NativeStringProtocol, check: &dyn Fn() -> Result<(), NativeExecutionError>) -> Result<*mut TclObj, CmdError>`. Its check must include inherited operation and original source/receiver obligations. Preserve original field-key/CE/BCE literals, genuine fresh numeric fields and the same timezone-name child; clean up unpublished fresh fields/results on refusal.

The genuine result Dict acquires its own native child references. Working allocation leases must not inflate the measured name/key reference counts. Registration/clientdata lifetime must cover the call and each real deletion callback; three implemented workers do not reproduce seven C worker holds.

Empty tzdata reaches an explicit unavailable native-localtime capability. Neither central scalar getters nor `Clock::local_offset_secs` supply C localtime, year/range/TZ or timezone-name evidence. No UTC fallback or C9 signature is admitted.

Proposed controls, all unexecuted:

- Reuse sealed scalar/list/calendar rows and original-object Dict/key/name controls; preserve argument read effects and exact conversion/error order.
- A reached getter effect changes/restores execution context: retain its first host cause, preserve already reached effects, and prove no next transition/offset getter or Dict publication occurs.
- A reached getter effect replaces/retires the original table or selected row backing while allocations remain leased: refuse the stale receiver before reading the next child/result; preserve allowed scalar coercion.
- Use the original raw `4e00ff` name child: verify identical Dict child and shared key literals, retire Dict/table/worker roles in measured orders, and verify a genuinely retained name remains live with the expected count/bytes.
- Missing localtime and foreign/missing worker admission produce typed operational refusal, without a new guest result or oracle equivalence claim.
