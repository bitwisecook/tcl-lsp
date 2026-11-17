# Suppression fixture for `tcl diag` (docs/kcs/kcs-howto-suppress-diagnostics.md).
#
# Each directive silences the *following* command; the unmarked reads below are
# the control that proves the codes still fire without a marker. Every read
# sits in its own procedure that is called under `catch`, because a read of an
# unset variable raises in Tcl and would end the script before a later read
# ran: each read is its own independent run.

proc readSuppressedByCode {} {
    # noqa: W210
    puts $suppressedByCode
}

proc readSuppressedByBareNoqa {} {
    # noqa
    puts $suppressedByBareNoqa
}

proc readReportedWithoutAMarker {} {
    puts $reportedWithoutAMarker
}

proc readReportedBesideProse {} {
    # a comment that only mentions noqa in passing is not a directive
    puts $reportedBesideProse
}

catch {readSuppressedByCode}
catch {readSuppressedByBareNoqa}
catch {readReportedWithoutAMarker}
catch {readReportedBesideProse}

set dictValue [dict create a 1 b 2]
# noqa: S100
lindex $dictValue 0

set otherDict [dict create a 1 b 2]
lindex $otherDict 0
