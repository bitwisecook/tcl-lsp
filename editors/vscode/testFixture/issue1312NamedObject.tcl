# Fixture for issue #1312 — a named object (`CLASS create NAME`) resolves no
# members where `[CLASS new]` gives full support. Line numbers are
# load-bearing — the companion test (issue1312NamedObject.test.ts) asserts
# on them. Names are unique to avoid colliding with another file's `C`/`obj`.
oo::class create Issue1312Class {
    method mrun {} { return 1 }
}
Issue1312Class create issue1312Obj
issue1312Obj mrun
issue1312Obj nosuchmethod
