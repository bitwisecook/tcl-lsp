# W306: Literal expected in regexp pattern — found '$'
# Source: SpiceGenTcl/src/ngspice/netlistParserClassNgspice.tcl:480,540,605,683
#
# A regexp pattern containing $ without braces means the variable is
# substituted before regexp sees it. This is often unintentional —
# the $ might be meant as end-of-line anchor. Bracing prevents substitution.
#
# Expected: tclsh runs — shows the difference.

set data "hello123"
set pattern "^hello"

# Unbraced pattern — $pattern is substituted first (works but flagged)
if {[regexp $pattern $data]} {
    puts "unbraced pattern matched"
}

# Braced pattern — the literal is used directly (preferred)
if {[regexp {^hello} $data]} {
    puts "braced pattern matched"
}

# The real risk: $ inside double quotes is substituted, not end-of-line
set line "test"
set x "oops"
# This does NOT match end-of-line; it matches the value of $x
if {[regexp "test$x" "testoops"]} {
    puts "substituted \$x matched 'oops', not end-of-line"
}
# This matches end-of-line correctly
if {[regexp {test$} "test"]} {
    puts "braced \$ matched end-of-line"
}

puts "W306 test complete"
