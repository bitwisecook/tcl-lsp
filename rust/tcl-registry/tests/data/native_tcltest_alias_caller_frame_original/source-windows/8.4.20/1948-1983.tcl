	    set coreModTime [file mtime [file join [workingDirectory] core]]
	}
    }

    # First, run the setup script
    set code [catch {uplevel 1 $setup} setupMsg]
    if {$code == 1} {
	set errorInfo(setup) $::errorInfo
	set errorCode(setup) $::errorCode
    }
    set setupFailure [expr {$code != 0}]

    # Only run the test body if the setup was successful
    if {!$setupFailure} {

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
	foreach {actualAnswer returnCode} $testResult break
	if {$returnCode == 1} {
	    set errorInfo(body) $::errorInfo
	    set errorCode(body) $::errorCode
	}
    }

    # Always run the cleanup script
    set code [catch {uplevel 1 $cleanup} cleanupMsg]
