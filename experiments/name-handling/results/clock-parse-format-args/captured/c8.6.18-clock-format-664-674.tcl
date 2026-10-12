proc ::tcl::clock::format { args } {

    variable FormatProc
    variable TZData

    lassign [ParseFormatArgs {*}$args] format locale timezone
    set locale [string tolower $locale]
    set clockval [lindex $args 0]

    # Get the data for time changes in the given zone

