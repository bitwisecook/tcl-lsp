# W302 quick-fix fixture (issue #1190).
#
# The diagnostic anchors at the `catch` word, but the quick fix must splice
# the result variable in after the *body's* closing brace. Applying it at the
# diagnostic's end would produce `catch result {error oops}` — a catch of the
# script `result`, which C Tcl reports as `invalid command name "result"`.
#
# Line 8 is the single-line body; lines 10-12 are the multi-line one.
catch {error oops}

catch {
    error other
}
