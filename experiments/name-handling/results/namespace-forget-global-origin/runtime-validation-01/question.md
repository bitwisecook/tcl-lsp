# Runtime Native C8.6 namespace-forget consumer validation

Do the three fixed Native C8.6 tests reproduce the retained original #2300
witness/adverse answers and the separate import-chain oracle answers using the
genuine native interpreter fixture and actual command generations?

Run only the `native_c86_forget_` filter with one test thread, offline, against
the standalone Runtime manifest and exclusive issues target cache. Capture the
actual argument vector, source before/after hashes, process outcome, complete
log and executed test binary hash. Declared local dependency roots are a
conservative source envelope including optional/target dependencies; the binary
and dep-info provide narrower executed-artifact observations. No broader
namespace suite, Tcltest harness, F5 execution or #1778 lifetime claim follows.
