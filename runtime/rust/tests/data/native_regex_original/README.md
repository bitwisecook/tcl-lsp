# Original regex objects

The probe uses the pinned C8.4–9.1 headers and static libraries. The manifest records source, header, library and executable hashes, compile results and each native execution result. It observes primary types, string residency and reference counts before materialising result bytes.

The controls cover per-match write callbacks, first setter failure, quiet versus detailed variable errors, an untouched pure List output name on no match, and C9 command-prefix substitution. The C9 callback observes the same prefix member and fresh Unicode match objects before returning that member as the callback result.

C8.4's scripted failing-trace control aborts in the actual engine with `free(): invalid pointer`. That capture is retained as an unavailable completion; it supplies no successful-result expectation. Its successful per-match and scalar-error controls remain independent.
