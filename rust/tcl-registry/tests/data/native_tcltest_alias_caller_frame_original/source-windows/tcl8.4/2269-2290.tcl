proc tcltest::RunTest {name script} {
    DebugPuts 3 "Running $name {$script}"

    # If there is no "memory" command (because memory debugging isn't
    # enabled), then don't attempt to use the command.

    if {[llength [info commands memory]] == 1} {
	memory tag $name
    }

    set code [catch {uplevel 1 $script} actualAnswer]

    return [list $actualAnswer $code]
}

#####################################################################

# tcltest::cleanupTestsHook --
#
#	This hook allows a harness that builds upon tcltest to specify
#       additional things that should be done at cleanup.
#
