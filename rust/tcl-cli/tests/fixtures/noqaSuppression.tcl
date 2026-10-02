# Suppression fixture for `tcl diag` (docs/kcs/kcs-howto-suppress-diagnostics.md).
#
# Each directive silences the *following* command; the unmarked reads below are
# the control that proves the codes still fire without a marker.

# noqa: W210
puts $suppressedByCode

# noqa
puts $suppressedByBareNoqa

puts $reportedWithoutAMarker

# a comment that only mentions noqa in passing is not a directive
puts $reportedBesideProse

set dictValue [dict create a 1 b 2]
# noqa: S100
lindex $dictValue 0

set otherDict [dict create a 1 b 2]
lindex $otherDict 0
