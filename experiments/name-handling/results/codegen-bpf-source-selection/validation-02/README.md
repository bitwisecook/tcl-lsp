# BPF source-selection acceptance scope

Both production libtest binaries build. eBPF codegen passes all 28 controls; typed BPF IR passes59/60. All five new naming controls pass, including rooted framework/policy selection and separate original-source vs exact-descriptor boundaries. The existing plain_set_is_rejected control fails because a retained original `set` call is reported as UnknownCommand instead of OutOfSubset. That failed diagnostic is retained unchanged.

The 10,221 pre-captured inputs are unchanged. All 3,018 dependency inputs bound to selected local Cargo outputs are covered before/after; no unresolved, unbound, changed or post-only selected input remains. Actual binaries, tools, exact command/environment and raw outputs are retained. No Runtime, WASM, native-entry or C/Jim/F5 oracle execution is claimed.
