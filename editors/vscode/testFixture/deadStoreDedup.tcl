# W211/W220 dedup fixture.
# `x` is set once and never read: a single dead assignment that must surface as
# exactly one hint (W211 "set but never used"), not a co-located W211 + W220.
# `y` is overwritten before its read, so `set y 1` is a standalone dead store
# (W220) with no W211 — the anchor proving W220 still fires on its own.
proc single_unused {} {
    set x 1
}
proc overwritten_used {} {
    set y 1
    set y 2
    return $y
}
