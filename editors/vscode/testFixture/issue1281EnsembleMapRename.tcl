# Issue #1281 — the ensemble `-map` key is an arbitrary name with no required
# relationship to its target's tail, so renaming `::app::widget::Show` must
# rewrite the declaration and the `-map` value but leave the dispatch word
# `show` alone.
#
# tclsh 8.6.14 / 9.0.4, identical: `::app::widget show` prints "showing";
# `::app::widget Show` is `unknown or ambiguous subcommand "Show": must be show`.
namespace eval ::app::widget {}
proc ::app::widget::Show {} { return "showing" }
namespace ensemble create -command ::app::widget -map {show ::app::widget::Show}
puts [::app::widget show]
