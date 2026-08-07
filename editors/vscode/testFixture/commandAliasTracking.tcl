# `interp alias` COMMAND aliasing (issue #923 audit idx 21 / idx 89).
#
# Verified on tclsh 9.0.4 and 8.6.16, byte-identical:
#   * both `[sayHi]` calls print `hi` — they really run `greet`'s body;
#   * `::ttk::spinbox .sb` prints `classic tk::spinbox: .sb`, so the
#     `::ttk::spinbox` proc declared on line 17 is dead from line 23 onward.
#
# Line numbers are load-bearing for commandAliasTracking.test.ts.
proc greet {} {
    return hi
}
interp alias {} sayHi {} greet
puts [sayHi]
puts [sayHi]

namespace eval ::ttk {}
namespace eval ::tk {}
proc ::ttk::spinbox {w args} {
    puts "themed ttk::spinbox: $w $args"
}
proc ::tk::spinbox {w args} {
    puts "classic tk::spinbox: $w $args"
}
interp alias {} ::ttk::spinbox {} ::tk::spinbox
::ttk::spinbox .sb
