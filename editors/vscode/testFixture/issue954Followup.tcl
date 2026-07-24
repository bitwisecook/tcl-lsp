# Issue #954 follow-up: placeholder fixture. `issue954Followup.test.ts`
# replaces this content at runtime (via setTestContent) with each specific
# repro/regression snippet -- checking in one file per tiny snippet would
# bloat the repo for no benefit.
proc placeholder {} {
    return 1
}
