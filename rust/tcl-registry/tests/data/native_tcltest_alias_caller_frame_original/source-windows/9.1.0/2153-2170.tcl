
    # Always run the cleanup script (or a hook if it presents):
    if {[set cmd [namespace which -command [namespace current]::CleanupTest]] ne ""} {
	set cleanup [list $cmd $cleanup]
    }
    set code [catch {uplevel 1 $cleanup} cleanupMsg]
    if {$code == 1} {
	set errorInfo(cleanup) $::errorInfo
	set errorCodeRes(cleanup) $::errorCode
    }
    set cleanupFailure [expr {$code != 0}]

    set coreFailure 0
    set coreMsg ""
    # check for a core file first - if one was created by the test,
    # then the test failed
    if {[preserveCore]} {
	if {[file exists [file join [workingDirectory] core]]} {
