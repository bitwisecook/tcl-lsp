# Deep optimiser stress sample.
#
# Run: tcl opt --profile aggressive samples/optimiser/deep_pipeline.tcl
#
# This program is deliberately layered so that each optimisation exposes
# the next, driving the multi-pass (aggressive) optimiser through many
# iterations across SEVERAL different optimisation kinds:
#
#   * O103 interprocedural folding of pure proc calls with constant args,
#   * O100 constant propagation feeding the *next* call's arguments
#     (so each `square $prev` only becomes foldable a pass later),
#   * O102/O110 expr folding + canonicalisation,
#   * O115 redundant nested-expr unwrap,
#   * O104 static string-build chains,
#   * O109/O126 dead-store and unused-assignment elimination.
#
# Because folding `metrics`/`version` to constants only happens once their
# whole bodies collapse, the calls to them in `headline` fold in a still
# later pass — a genuine 5+-deep cascade.
#
# It runs unchanged on C Tcl 9 (tclsh9.0) and prints one deterministic
# line, so the optimised / formatted / minified forms can each be executed
# and diffed for behavioural equivalence:
#
#   for form in "opt --profile aggressive" format minify; do
#       tcl $form samples/optimiser/deep_pipeline.tcl | tclsh9.0
#   done
#
# Expected output:  tcl v9.0 score=62 x2=124

proc square {n} { return [expr {$n * $n}] }
proc add {a b} { return [expr {$a + $b}] }
proc dbl {n}   { return [expr {$n * 2}] }
proc pct {part whole} { return [expr {($part * 100) / $whole}] }

# Each level only becomes foldable once the previous proc-call result is
# folded and propagated — this is what forces pass after pass.
proc metrics {} {
    set a [square 2]
    set b [square $a]
    set c [dbl $b]
    set d [add $c $b]
    set e [expr {[expr {$d + 2}]}]
    set ratio [pct $a $c]
    set tmp 0
    set tmp [add $e $ratio]
    return $tmp
}

# String-build chain that collapses into a single return value.
proc version {} {
    set v {v}
    append v {9}
    append v {.}
    append v {0}
    return $v
}

proc headline {} {
    set score [metrics]
    set ver [version]
    set doubled [dbl $score]
    return "tcl $ver score=$score x2=$doubled"
}

puts [headline]
