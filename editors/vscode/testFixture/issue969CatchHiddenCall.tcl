# Issue #969 follow-up: a call site embedded inside a `catch { ... }` body
# is a real caller with a differing argument, but `catch`'s body is an
# ArgRole::Body argument of the builtin `catch` (never a user proc), so a
# flat, one-level call-site scan resolves `catch`, finds no matching proc,
# and never notices `is_even 4` sitting inside it. is_even is called with
# both 3 and 4 (the second one hidden inside catch) — must draw no I230.
proc is_even {n} {
    if {$n % 2 == 0} {
        return 1
    } else {
        return 0
    }
}
proc main {} {
    is_even 3
    catch { is_even 4 }
}
main
