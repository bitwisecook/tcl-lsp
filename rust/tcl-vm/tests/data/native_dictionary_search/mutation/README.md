# Native dictionary search mutation

The original C API controls retain one unshared dictionary root and an active dictionary search. Replacing or adding a member mutates that same backing. Explicit search closure succeeds. Advancing the mutated search terminates native Tcl 8.5–9.1 with its epoch guard; registered callbacks inside `catch` and `try` retain that fatal boundary.

The manifests record successful closures separately from native process aborts. Abort records certify the native fatal condition, not successful guest completions. The VM represents that boundary as a typed host refusal outside guest handlers. Member reference counts are captured before string observers.

The VM `test-support` feature provides a registered callback over a sole-owned original search. Engine OriginalObject and the supported C shim have no dictionary cursor operation, so the fixture retains that closed capability inside the VM. The callback performs actual mutation and advancement, and exercises the operational refusal through engine adapters. The feature does not expose a production search API.
