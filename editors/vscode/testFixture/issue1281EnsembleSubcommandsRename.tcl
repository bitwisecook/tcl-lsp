# Issue #1281 — the `-subcommands` entry IS the target's tail, so renaming
# `alpha` must rewrite the declaration, the `-subcommands` entry and the
# dispatch word together.
#
# tclsh 8.6.14 / 9.0.4, identical: with the proc renamed to `beta` but the
# entry left at `alpha`, `::app::widget alpha` is `invalid command name "alpha"`.
namespace eval ::app::widget {
    proc alpha {} { return "alpha!" }
    namespace ensemble create -command ::app::widget -subcommands {alpha}
}
puts [::app::widget alpha]
