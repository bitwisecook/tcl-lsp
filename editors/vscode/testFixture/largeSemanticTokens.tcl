# Issue #829: placeholder fixture for large-document semantic-token latency
# tests. `semanticTokens.test.ts` replaces this content at runtime (via
# setTestContent) with a generated multi-hundred-proc document -- checking in
# a multi-thousand-line generated file would bloat the repo for no benefit.
proc placeholder {} {
    return 1
}
