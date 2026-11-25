    # First, run the setup script (or a hook if it presents):
    if {[set cmd [namespace which -command [namespace current]::SetupTest]] ne ""} {
	set setup [list $cmd $setup]
    }
    set processTest 1
    set code [catch {uplevel 1 $setup} setupMsg]
    if {$code == 1} {
	set errorInfo(setup) $::errorInfo
	set errorCodeRes(setup) $::errorCode
	if {$errorCodeRes(setup) eq "BYPASS-SKIPPED-TEST"} {
	    _noticeSkipped $name $setupMsg
	    set processTest [set code 0]
	}
    }
    set setupFailure [expr {$code != 0}]

    # Only run the test body if the setup was successful
    if {$processTest && !$setupFailure} {

	# Register startup time
	if {[IsVerbose msec] || [IsVerbose usec]} {
	    set timeStart [clock microseconds]
	}

	# Verbose notification of $body start
	if {[IsVerbose start]} {
	    puts [outputChannel] "---- $name start"
	    flush [outputChannel]
	}

	set command [list [namespace origin RunTest] $name $body]
	if {[info exists output] || [info exists errorOutput]} {
	    set testResult [uplevel 1 [list [namespace origin Eval] $command 0]]
	} else {
	    set testResult [uplevel 1 [list [namespace origin Eval] $command 1]]
	}
