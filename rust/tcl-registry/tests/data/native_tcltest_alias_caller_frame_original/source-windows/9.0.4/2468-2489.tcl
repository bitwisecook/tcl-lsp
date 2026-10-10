proc tcltest::RunTest {name script} {
    DebugPuts 3 "Running $name {$script}"

    # If there is no "memory" command (because memory debugging isn't
    # enabled), then don't attempt to use the command.

    if {[llength [info commands memory]] == 1} {
	memory tag $name
    }

    # run the test script (or a hook if it presents):
    if {[set cmd [namespace which -command [namespace current]::EvalTest]] ne ""} {
	set script [list $cmd $script]
    }
    set code [catch {uplevel 1 $script} actualAnswer]

    return [list $actualAnswer $code]
}

#####################################################################

# tcltest::cleanupTestsHook --
