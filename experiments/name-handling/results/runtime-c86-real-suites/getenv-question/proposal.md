# Unimplemented bounded support proposal

Admit only the measured selected native C8.6 `::tcl::clock::getenv` worker
through registry-owned constructor metadata and a direct C8.6 CommandSpec.
Declare its shipped backing, exact one-name arity and independently measured
NoHook compiler descriptor. Leave handler purity conservative. Preserve ordinary
relative lookup in original GetSystemTimeZone; add no compiler/analyser
command-name branch and no bypass of real clock startup.

The existing Unicode-only Env getter suffices for the two ASCII timezone names
only when their values are Unicode. It cannot represent the measured raw Unix
name/value channel and must not turn an unsupported byte read into a missing
variable or lossy text. The smallest full getter support therefore needs an
explicit native environment byte capability on the platform Host/Env owner,
with a NativeHost Unix implementation reading current process bytes and actual
per-host overrides. Follow the existing optional numeric/environment capability
pattern: absence retains an operational refusal. Keep Windows unsupported by
this Unix evidence until separately measured; do not infer a wide-character
conversion protocol from the Unix result.

A shared `tcl-cmd-core` getter can consume `ValueOps::native_string_bytes` on
the original name, select its CString extent through the existing
`tcl_core_types::c_string_extent` owner, call that capability, then construct
the original native result through `ValueOps::new_bytes`. An actual missing
environment entry becomes an empty value. Arity uses the existing runtime
`wrong_args_for_invocation(argv, b"name")` owner, which preserves invoked words
and selected errorCode rather than fixing a worker name into a message.

Registration should share the existing genuine ClockClientData-style pool with
ParseFormatArgs across the two actual implemented object-command owners. Retire
each real callback independently; add no phantom holds for the five still
absent support workers. Full original seven-worker pool lifetime remains a
separate unproved boundary. Normal env element write/unset synchronisation is
also a separate existing runtime gap: the getter must read current host data,
and getter implementation alone cannot claim the original env-mutation cases
pass until that shared variable owner is repaired.

After all shared builds are released, add selected C8.6 stock backing evidence
to the focused bootstrap test using these existing APIs. This assertion is
proposed only; it has not been executed:

```rust
for version in tcl_dialect::TclVersion::ALL {
    let profile = crate::environment::profile_for_dialect(version.dialect_profile_name());
    let dialect = tcl_registry::InvocationDialect::of_profile(profile);
    let registry = crate::environment::store_for_profile(profile);
    let identity = "tcl::clock::ParseFormatArgs";
    let declared = registry
        .get_for_surface(identity, dialect.authoring_query())
        .map(|spec| spec.runtime_backing);
    let interp = Interp::with_native_core(
        crate::interp::default_host(), profile,
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    ).unwrap();
    let actual = interp.backing_report().into_iter()
        .find(|(name, _)| name == identity).map(|(_, backing)| backing);
    assert_eq!(declared, (version == tcl_dialect::TclVersion::V8_6)
        .then_some(tcl_registry::RuntimeBacking::shipped(identity)));
    assert_eq!(actual, (version == tcl_dialect::TclVersion::V8_6)
        .then_some(tcl_runtime_api::RegisteredBacking::Builtin));
}
```

This proves the implemented ParseFormatArgs declaration/report at selected C8.6
and its intentionally absent implementation on the other supported C profiles.
It makes no claim that the original C8.5 provider lacked that worker. Reuse the
same selected-profile assertion for getenv only after its actual admission and
backing exist. Add fixed original-object getter controls for the measured byte,
arity and trace cases, retain any env synchronisation failure explicitly, then
retry original all.tcl with unchanged real init/tcltest. Do not interpret the
C9-only command-backing gate as evidence for this C8.6 registration.
