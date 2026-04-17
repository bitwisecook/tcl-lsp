# W115: Backslash-newline in comment silently swallows the next line
# Source: SpiceGenTcl/src/generalClasses.tcl:800-801,2523-2524
#
# A backslash at the end of a comment line causes Tcl to treat the
# next line as a continuation of the comment, silently hiding code.
# This is a common source of subtle bugs.
#
# Expected: tclsh runs — but "hidden" line is swallowed by the comment.

# This comment has a trailing backslash \
set hidden "you will never see this"

set visible "this line is fine"
puts "visible=$visible"

# The variable 'hidden' was never set because the line was eaten
if {[info exists hidden]} {
    puts "hidden=$hidden"
} else {
    puts "hidden: DOES NOT EXIST (swallowed by comment)"
}

puts "W115 test complete"
