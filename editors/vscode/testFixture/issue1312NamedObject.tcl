# Fixture for issue #1312 — a named object (`CLASS create NAME`) resolves no
# members where `[CLASS new]` gives full support. Line numbers are
# load-bearing — the companion test (issue1312NamedObject.test.ts) asserts
# on them.
oo::class create C {
    method mrun {} { return 1 }
}
C create obj
obj mrun
obj nosuchmethod
