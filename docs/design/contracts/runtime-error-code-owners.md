# Runtime error-code owners

The explicit C object setter retains the supplied object header. It does not obtain a String representation or reconstruct an error-code object from display bytes. The public `Tcl_SetObjErrorCode` adapter passes its original header to `Interp::set_original_c_error_code` under its actual entered-operation owner.

Tcl 8.4 publishes that header to the genuine global `errorCode` cell through `publish_original_c84_error_code`. The write callbacks belong to that setter, and the error-code-set flag is settled after the callbacks. Tcl 8.5–9.1 retain the original header in the selected private error-code owner. Replacing a private header retires its old backing outside the exception borrow before installing the incoming owner; a reached Host refusal stops the continuation. These storage choices require their actual native protocols.

`Interp::set_c_error_code` authors a new String object from supplied bytes and delegates its physical publication to the same object setter. Its byte-authored exception metadata and fresh error state belong to that explicit byte producer. Those bytes do not identify another native header, parser, frame or provider. Consumers that need current error-code bytes use `error_code_bytes_checked`, which projects the actual selected global or private header and checks the original getter and Host cause.

An incoming allocation lease protects header lifetime without adding a visible C reference. The selected global or private store takes the actual owning reference. Old backing retirement, setter callbacks and final lease cleanup retain the first Host cause and effects already reached. Guest result formatting cannot replace that cause. A pending Host refusal reaches no new setter publication or getter.

Callers must preserve the original object and entered interpreter through the call. They must not reconstruct object authority from result text, a dialect name or a descriptive exception stamp. A software completion projection on Tcl 8.4 does not imply availability of `Tcl_GetReturnOptions` on that release.

The [C8.4 error-code publication proof](../analysis/name-resolution-proofs/error-c84-publication-order.md) records its native observer scope. Runtime controls separately check actual software header identity, getter counts, references, retirement and callback error projection. The [embedding proof](../analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md) records the separate Host transport question.

## Counted error-info appends

`NativeErrorInfoAppend::from_accumulated` selects a counted pending suffix
before the physical error-variable setters can invoke traces. Both runtime
adapters retain this original suffix and its accumulator extent across those
callbacks. A callback's own error episode cannot select a different suffix.
The descriptor supplies no header, receiver or getter authority; the adapters
retain and check those original owners independently.

The C Tcl 8.4 `Tcl_AddObjErrorInfo` purpose ends after its counted append
setter. It performs no trailing global-variable or String getter. A consumer
that needs the global value must use its own actual read purpose. Setter Guest
failure and Host refusal retain distinct outcomes; reached cleanup and flag
settlement preserve the first Host cause.

The original callback comparisons are recorded by
[naming.error.c84-publication-order](../analysis/name-resolution-proofs/error-c84-publication-order.md)
and the original-object append observations by
[naming.variable.error-info-append-original-object](../analysis/name-resolution-proofs/variable.error-info-append-original-object.md).
Those observations cover their exact input and capture frontiers; the shared
suffix descriptor is software accumulator geometry.

## Package failure code effects

`NativePackageProtocol` selects `NativePackageErrorCode` at the actual conflict,
failure, index or version-validation purpose. `Default` means that the command
supplies no explicit code setter. `Set` carries the exact error-code List bytes
for the explicit setter. The visible `NONE` value cannot distinguish these
operations and must not select them.

The VM package error factory consumes the structured effect directly. Runtime
package command and C API provision factories consume the same selector before
publishing their error state.
The C8.4/C8.5 conflict and legacy version-validation paths supply the default;
C8.5's present lookup has its own explicit code. C8.6–9.1 retain their selected
structured codes. Jim's direct package diagnostics supply the default.

The [original C8.4 callback comparisons](../analysis/name-resolution-proofs/error-c84-publication-order.md)
and [package publication evidence](../analysis/name-resolution-proofs/package-ordinary-conflict.md)
retain the exact measured outputs. A code effect does not authorise another
command, native header, package table, source grammar or operand getter.
