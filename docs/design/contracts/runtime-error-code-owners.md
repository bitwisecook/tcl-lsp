# Runtime error-code owners

The explicit C object setter retains the supplied object header. It does not obtain a String representation or reconstruct an error-code object from display bytes. The public `Tcl_SetObjErrorCode` adapter passes its original header to `Interp::set_original_c_error_code` under its actual entered-operation owner.

Tcl 8.4 publishes that header to the genuine global `errorCode` cell through `publish_original_c84_error_code`. The write callbacks belong to that setter, and the error-code-set flag is settled after the callbacks. Tcl 8.5–9.1 retain the original header in the selected private error-code owner. Replacing a private header retires its old backing outside the exception borrow before installing the incoming owner; a reached Host refusal stops the continuation. These storage choices require their actual native protocols.

`Interp::set_c_error_code` authors a new String object from supplied bytes and delegates its physical publication to the same object setter. Its byte-authored exception metadata and fresh error state belong to that explicit byte producer. Those bytes do not identify another native header, parser, frame or provider. Consumers that need current error-code bytes use `error_code_bytes_checked`, which projects the actual selected global or private header and checks the original getter and Host cause.

An incoming allocation lease protects header lifetime without adding a visible C reference. The selected global or private store takes the actual owning reference. Old backing retirement, setter callbacks and final lease cleanup retain the first Host cause and effects already reached. Guest result formatting cannot replace that cause. A pending Host refusal reaches no new setter publication or getter.

Callers must preserve the original object and entered interpreter through the call. They must not reconstruct object authority from result text, a dialect name or a descriptive exception stamp. A software completion projection on Tcl 8.4 does not imply availability of `Tcl_GetReturnOptions` on that release.

The [C8.4 error-code publication proof](../analysis/name-resolution-proofs/error-c84-publication-order.md) records its native observer scope. Runtime controls separately check actual software header identity, getter counts, references, retirement and callback error projection. The [embedding proof](../analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md) records the separate Host transport question.
