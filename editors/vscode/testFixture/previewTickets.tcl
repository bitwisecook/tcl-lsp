# Fixture for the "preview version:" regression tickets #720/#721/#723/#725.
#
# `after <ms>` is a millisecond delay, not an unknown subcommand (#720) -> no W001.
after 200 {puts "Hello world!"}

# A fully-qualified global read is never "read before set" (#725) -> no W210,
# even though ::myVar is defined in another file.
proc foo {} {
    puts $::myVar
}

# An unknown third-party package may load Tk internally (#723) -> no W120 on
# the Tk commands below.
package require myTkPackage
if {[tk windowingsystem] eq "aqua"} {
    # nothing
} else {
    set fpixels [winfo fpixels . 1i]
}

# A genuine error that must appear exactly ONCE (#721): too many arguments.
set var 10 10
