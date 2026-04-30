# W306: Literal expected in regexp pattern — found '$'
# Source: SpiceGenTcl/src/ngspice/netlistParserClassNgspice.tcl:480,540,605,683
#
# A regexp pattern that mixes literal text with $-substitution (typically
# inside double quotes) is often unintentional — the $ might be meant as
# the regex end-of-line anchor.  Bracing prevents the substitution.
#
# A bare single $var or ${var} as the entire pattern is the canonical Tcl
# idiom for a parameterised pattern and is NOT flagged (issue #235): there
# is no equivalent {...}-braced form, so the warning would be a false
# positive.  Bare [cmd] is still flagged — a literal like [a-z] looks like
# a regex character class but is actually parsed as command substitution.
#
# Expected: tclsh runs — shows the difference.

set data "hello123"
set pattern "^hello"

# Bare $pattern as the entire argument — the canonical idiom, not flagged.
if {[regexp $pattern $data]} {
    puts "bare-var pattern matched"
}

# Braced literal — preferred when the pattern is itself a literal.
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
