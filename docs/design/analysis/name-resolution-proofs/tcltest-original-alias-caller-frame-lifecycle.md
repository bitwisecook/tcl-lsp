# naming.tcltest.original-alias-caller-frame-lifecycle

Kind: `native-observation`

## Problem statement

Tcltest setup, body and cleanup source reads must remain associated with the actual selected package and original caller through import and captured alias routes. Treating each script as an unrelated source body can report the original caller variables as undefined. This public experiment asks which values the original loaded package makes visible; it does not execute the reported tclopt constructors or validate Compiler diagnostics.

## Question

For the exact loaded original Tcltest package and fixed original CLI source, do qualified, imported, direct interp alias and captured-name/description alias invocations preserve setup/body/cleanup caller-visible values? What actual namespace-imported caller result is observed? What independent global-test/qualified-wrapper availability and setup-error behavior is observed on the other package implementation?

## Conclusion

All five original C releases load their measured Tcltest package and expose ::tcltest::test. Qualified, namespace-imported, direct interp-alias and captured-name/description-alias calls retain the same setup/body/cleanup caller values for optimizer, pdata, par0, par1 and pars. Every C_CALLER row catches code `0` and returns the full setup/body/cleanup event list, cleanup_seen PDATA, absent_seen `0` and data_only existence `0`. The separately namespace-imported procedure returns {{setup body READY cleanup READY} READY}. Bracket-looking expected/result text stays literal in this exact probe; it does not set data_only. Actual package versions are 2.2.11 on C8.4.20, 2.3.8 on C8.5.19, 2.5.11 on C8.6.18/C9.0.4 and 2.6.0 on C9.1.0. Current Jim loads Tcltest 1.0 and exposes global test, with no C-qualified wrapper. Its independent global caller returns the READY event list; the measured setup-error case still reaches body and cleanup and the enclosing caller catches code `0`. Jim therefore supplies no C-qualified alias-lifecycle answer. All six main processes exit `0` with empty stderr. Separate metadata processes report the actual interpreter/package versions; all five C queries exit `0`, while Jim exits1 after those two successful rows because package ifneeded is unsupported. The Jim partial query and whole stderr are retained, not a sixth successful metadata query.

## Scope

Exact fixed ASCII LF CLI script, real package loading, genuine interp aliases and public catch/list results only. The original problem is setup/body/cleanup source-read advice around Tcltest aliases; the Native probe deliberately substitutes benign values for the reported variable names and does not run #2286 tclopt constructors. Main lifecycle observations, independent metadata queries and 22 source excerpts remain different channels. The original receipts record 239 provider/input associations checked before and after, prior byte-identical original build/input receipt references, exact argv and explicit TCL_LIBRARY overlays. Publication rechecks every listed byte identity without executing or recompiling. Compiler version and unlisted inherited environment are unrecorded. The original prelaunch request remains unchanged, including its not-run validation field; the actual process receipts establish the executed outcomes. No capture of private frame, Native argv/header/object/token/cache, Registry attestation, entered worker, generic Tcltest failure behavior, Normal/effect/SSA/editor permission or BIG-IP transfer follows. No Rust assertions or full compiler parity are established. Existing source-only W210 controls and original public records retain their own scope.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Exact original ASCII LF distribution CLI source in fresh main and separate metadata processes, with actual package loading and full raw streams. Version: tcl8.4 8.4.20.

Qualified/imported/direct-alias/captured-alias calls and namespace-imported caller retain the full measured setup/body/cleanup values; literal result text leaves data_only absent. Tcltest 2.2.11. Main exit `0`/empty stderr; separate version/package/ifneeded/auto_path process exit `0`/empty stderr.

### tcl8.5

Status: `observed`. Channel: Exact original ASCII LF distribution CLI source in fresh main and separate metadata processes, with actual package loading and full raw streams. Version: tcl8.5 8.5.19.

Qualified/imported/direct-alias/captured-alias calls and namespace-imported caller retain the full measured setup/body/cleanup values; literal result text leaves data_only absent. Tcltest 2.3.8. Main exit `0`/empty stderr; separate version/package/ifneeded/auto_path process exit `0`/empty stderr.

### tcl8.6, tcl9.0

Status: `observed`. Channel: Exact original ASCII LF distribution CLI source in fresh main and separate metadata processes, with actual package loading and full raw streams. Version: tcl8.6 8.6.18; tcl9.0 9.0.4.

Qualified/imported/direct-alias/captured-alias calls and namespace-imported caller retain the full measured setup/body/cleanup values; literal result text leaves data_only absent. Tcltest 2.5.11. Main exit `0`/empty stderr; separate version/package/ifneeded/auto_path process exit `0`/empty stderr.

### tcl9.1

Status: `observed`. Channel: Exact original ASCII LF distribution CLI source in fresh main and separate metadata processes, with actual package loading and full raw streams. Version: tcl9.1 9.1.0.

Qualified/imported/direct-alias/captured-alias calls and namespace-imported caller retain the full measured setup/body/cleanup values; literal result text leaves data_only absent. Tcltest 2.6.0. Main exit `0`/empty stderr; separate version/package/ifneeded/auto_path process exit `0`/empty stderr.

### jim

Status: `observed`. Channel: Exact original ASCII LF distribution CLI source in fresh main and separate metadata processes, with actual package loading and full raw streams. Version: jim 0.84-9-g5bac7c9.

Tcltest 1.0 exposes global test only. Global READY caller succeeds; setup-error records setup body cleanup with caught code `0`. C-qualified alias lifecycle is unavailable. Main exit `0`/empty stderr. Separate metadata query returns version/package then fails exit `1` on unsupported package ifneeded; no auto_path result is available.

### bigip

Status: `not-tested`.

No BIG-IP package process or caller-lifecycle observation was requested or executed.

## Source inspection

tcl8.4 8.4.20, revision `Exact pinned original release/distribution source; source-control revision unrecorded.`, `/workspace/tcl-lsp/tmp/tcl8.4.20/library/tcltest/tcltest.tcl`, function `tcltest::test setup and body scheduling`, lines 1948–1983. Retained reference `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--library--tcltest--tcltest.tcl`.

```text
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

```

tcl8.4 8.4.20, revision `Exact pinned original release/distribution source; source-control revision unrecorded.`, `/workspace/tcl-lsp/tmp/tcl8.4.20/library/tcltest/tcltest.tcl`, function `tcltest::test cleanup`, lines 1978–1995. Retained reference `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--library--tcltest--tcltest.tcl`.

```text
	    set errorCode(body) $::errorCode
	}
    }

    # Always run the cleanup script
    set code [catch {uplevel 1 $cleanup} cleanupMsg]
    if {$code == 1} {
	set errorInfo(cleanup) $::errorInfo
	set errorCode(cleanup) $::errorCode
    }
    set cleanupFailure [expr {$code != 0}]

    set coreFailure 0
    set coreMsg ""
    # check for a core file first - if one was created by the test,
    # then the test failed
    if {[preserveCore]} {
	if {[file exists [file join [workingDirectory] core]]} {

```

tcl8.4 8.4.20, revision `Exact pinned original release/distribution source; source-control revision unrecorded.`, `/workspace/tcl-lsp/tmp/tcl8.4.20/library/tcltest/tcltest.tcl`, function `tcltest::RunTest`, lines 2269–2290. Retained reference `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--library--tcltest--tcltest.tcl`.

```text
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

```

tcl8.4 8.4.20, revision `Exact pinned original release/distribution source; source-control revision unrecorded.`, `/workspace/tcl-lsp/tmp/tcl8.4.20/library/tcltest/tcltest.tcl`, function `tcltest::Eval`, lines 1610–1628. Retained reference `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--library--tcltest--tcltest.tcl`.

```text
proc tcltest::Eval {script {ignoreOutput 1}} {
    variable outData
    variable errData
    DebugPuts 3 "[lindex [info level 0] 0] called"
    if {!$ignoreOutput} {
	set outData {}
	set errData {}
	rename ::puts [namespace current]::Replace::Puts
	namespace eval :: \
		[list namespace import [namespace origin Replace::puts]]
	namespace import Replace::puts
    }
    set result [uplevel 1 $script]
    if {!$ignoreOutput} {
	namespace forget puts
	namespace eval :: namespace forget puts
	rename [namespace current]::Replace::Puts ::puts
    }
    return $result

```

tcl8.5 8.5.19, revision `Exact pinned original release/distribution source; source-control revision unrecorded.`, `/workspace/tcl-lsp/tmp/tcl8.5.19/library/tcltest/tcltest.tcl`, function `tcltest::test setup and body scheduling`, lines 1960–1995. Retained reference `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--library--tcltest--tcltest.tcl`.

```text
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
	lassign $testResult actualAnswer returnCode
	if {$returnCode == 1} {
	    set errorInfo(body) $::errorInfo
	    set errorCode(body) $::errorCode
	}
    }

    # check if the return code matched the expected return code
    set codeFailure 0

```

tcl8.5 8.5.19, revision `Exact pinned original release/distribution source; source-control revision unrecorded.`, `/workspace/tcl-lsp/tmp/tcl8.5.19/library/tcltest/tcltest.tcl`, function `tcltest::test cleanup`, lines 2034–2051. Retained reference `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--library--tcltest--tcltest.tcl`.

```text
    } else {
	set scriptFailure 1
    }

    # Always run the cleanup script
    set code [catch {uplevel 1 $cleanup} cleanupMsg]
    if {$code == 1} {
	set errorInfo(cleanup) $::errorInfo
	set errorCode(cleanup) $::errorCode
    }
    set cleanupFailure [expr {$code != 0}]

    set coreFailure 0
    set coreMsg ""
    # check for a core file first - if one was created by the test,
    # then the test failed
    if {[preserveCore]} {
	if {[file exists [file join [workingDirectory] core]]} {

```

tcl8.5 8.5.19, revision `Exact pinned original release/distribution source; source-control revision unrecorded.`, `/workspace/tcl-lsp/tmp/tcl8.5.19/library/tcltest/tcltest.tcl`, function `tcltest::RunTest`, lines 2302–2323. Retained reference `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--library--tcltest--tcltest.tcl`.

```text
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

```

tcl8.5 8.5.19, revision `Exact pinned original release/distribution source; source-control revision unrecorded.`, `/workspace/tcl-lsp/tmp/tcl8.5.19/library/tcltest/tcltest.tcl`, function `tcltest::Eval`, lines 1627–1645. Retained reference `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--library--tcltest--tcltest.tcl`.

```text
proc tcltest::Eval {script {ignoreOutput 1}} {
    variable outData
    variable errData
    DebugPuts 3 "[lindex [info level 0] 0] called"
    if {!$ignoreOutput} {
	set outData {}
	set errData {}
	rename ::puts [namespace current]::Replace::Puts
	namespace eval :: [list namespace import [namespace origin Replace::puts]]
	namespace import Replace::puts
    }
    set result [uplevel 1 $script]
    if {!$ignoreOutput} {
	namespace forget puts
	namespace eval :: namespace forget puts
	rename [namespace current]::Replace::Puts ::puts
    }
    return $result
}

```

tcl8.6 8.6.18, revision `Exact pinned original release/distribution source; source-control revision unrecorded.`, `/workspace/tcl-lsp/tmp/tcl8.6.18/library/tcltest/tcltest.tcl`, function `tcltest::test setup and body scheduling`, lines 2044–2079. Retained reference `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--library--tcltest--tcltest.tcl`.

```text
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

```

tcl8.6 8.6.18, revision `Exact pinned original release/distribution source; source-control revision unrecorded.`, `/workspace/tcl-lsp/tmp/tcl8.6.18/library/tcltest/tcltest.tcl`, function `tcltest::test cleanup`, lines 2141–2158. Retained reference `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--library--tcltest--tcltest.tcl`.

```text

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

```

tcl8.6 8.6.18, revision `Exact pinned original release/distribution source; source-control revision unrecorded.`, `/workspace/tcl-lsp/tmp/tcl8.6.18/library/tcltest/tcltest.tcl`, function `tcltest::RunTest`, lines 2468–2489. Retained reference `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--library--tcltest--tcltest.tcl`.

```text
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

```

tcl8.6 8.6.18, revision `Exact pinned original release/distribution source; source-control revision unrecorded.`, `/workspace/tcl-lsp/tmp/tcl8.6.18/library/tcltest/tcltest.tcl`, function `tcltest::Eval`, lines 1695–1713. Retained reference `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--library--tcltest--tcltest.tcl`.

```text
proc tcltest::Eval {script {ignoreOutput 1}} {
    variable outData
    variable errData
    DebugPuts 3 "[lindex [info level 0] 0] called"
    if {!$ignoreOutput} {
	set outData {}
	set errData {}
	rename ::puts [namespace current]::Replace::Puts
	namespace eval :: [list namespace import [namespace origin Replace::puts]]
	namespace import Replace::puts
    }
    set result [uplevel 1 $script]
    if {!$ignoreOutput} {
	namespace forget puts
	namespace eval :: namespace forget puts
	rename [namespace current]::Replace::Puts ::puts
    }
    return $result
}

```

tcl9.0 9.0.4, revision `Exact pinned original release/distribution source; source-control revision unrecorded.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/library/tcltest/tcltest.tcl`, function `tcltest::test setup and body scheduling`, lines 2044–2079. Retained reference `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--library--tcltest--tcltest.tcl`.

```text
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

```

tcl9.0 9.0.4, revision `Exact pinned original release/distribution source; source-control revision unrecorded.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/library/tcltest/tcltest.tcl`, function `tcltest::test cleanup`, lines 2141–2158. Retained reference `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--library--tcltest--tcltest.tcl`.

```text

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

```

tcl9.0 9.0.4, revision `Exact pinned original release/distribution source; source-control revision unrecorded.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/library/tcltest/tcltest.tcl`, function `tcltest::RunTest`, lines 2468–2489. Retained reference `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--library--tcltest--tcltest.tcl`.

```text
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

```

tcl9.0 9.0.4, revision `Exact pinned original release/distribution source; source-control revision unrecorded.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/library/tcltest/tcltest.tcl`, function `tcltest::Eval`, lines 1695–1713. Retained reference `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--library--tcltest--tcltest.tcl`.

```text
proc tcltest::Eval {script {ignoreOutput 1}} {
    variable outData
    variable errData
    DebugPuts 3 "[lindex [info level 0] 0] called"
    if {!$ignoreOutput} {
	set outData {}
	set errData {}
	rename ::puts [namespace current]::Replace::Puts
	namespace eval :: [list namespace import [namespace origin Replace::puts]]
	namespace import Replace::puts
    }
    set result [uplevel 1 $script]
    if {!$ignoreOutput} {
	namespace forget puts
	namespace eval :: namespace forget puts
	rename [namespace current]::Replace::Puts ::puts
    }
    return $result
}

```

tcl9.1 9.1.0, revision `Exact pinned original release/distribution source; source-control revision unrecorded.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/library/tcltest/tcltest.tcl`, function `tcltest::test setup and body scheduling`, lines 2056–2091. Retained reference `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--library--tcltest--tcltest.tcl`.

```text
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

```

tcl9.1 9.1.0, revision `Exact pinned original release/distribution source; source-control revision unrecorded.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/library/tcltest/tcltest.tcl`, function `tcltest::test cleanup`, lines 2153–2170. Retained reference `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--library--tcltest--tcltest.tcl`.

```text

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

```

tcl9.1 9.1.0, revision `Exact pinned original release/distribution source; source-control revision unrecorded.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/library/tcltest/tcltest.tcl`, function `tcltest::RunTest`, lines 2480–2501. Retained reference `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--library--tcltest--tcltest.tcl`.

```text
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

```

tcl9.1 9.1.0, revision `Exact pinned original release/distribution source; source-control revision unrecorded.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/library/tcltest/tcltest.tcl`, function `tcltest::Eval`, lines 1701–1719. Retained reference `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--library--tcltest--tcltest.tcl`.

```text
proc tcltest::Eval {script {ignoreOutput 1}} {
    variable outData
    variable errData
    DebugPuts 3 "[lindex [info level 0] 0] called"
    if {!$ignoreOutput} {
	set outData {}
	set errData {}
	rename ::puts [namespace current]::Replace::Puts
	namespace eval :: [list namespace import [namespace origin Replace::puts]]
	namespace import Replace::puts
    }
    set result [uplevel 1 $script]
    if {!$ignoreOutput} {
	namespace forget puts
	namespace eval :: namespace forget puts
	rename [namespace current]::Replace::Puts ::puts
    }
    return $result
}

```

jim 0.84-9-g5bac7c9, revision `Exact pinned original release/distribution source; source-control revision unrecorded.`, `/workspace/.proofs/native-providers/jimtcl/tcltest.tcl`, function `test literal result option parsing`, lines 314–339. Retained reference `native_tcltest_alias_caller_frame_original-original-inputs-workspace--.proofs--native-providers--jimtcl--tcltest.tcl`.

```text
proc test {id descr args} {
	set default [dict create -returnCodes {ok return} -match exact -result {} -constraints {} -body {} -setup {} -cleanup {}]
	set a $default
	if {[lindex $args 0] ni [dict keys $a]} {
		if {[llength $args] == 2} {
			lassign $args body result constraints
		} elseif {[llength $args] == 3} {
			lassign $args constraints body result
		} else {
			return -code error "$id: Wrong syntax for tcltest::test v1"
		}
		tailcall test $id $descr -body $body -result $result -constraints $constraints
	}
	# tcltest::test v2 syntax
	array set a $args

	incr ::testinfo(numtests)
	if {$::testinfo(verbose)} {
		puts -nonewline "$id "
	}

	foreach c $a(-constraints) {
		if {![testConstraint $c]} {
			incr ::testinfo(numskip)
			if {$::testinfo(verbose)} {
				puts "SKIP $descr"

```

jim 0.84-9-g5bac7c9, revision `Exact pinned original release/distribution source; source-control revision unrecorded.`, `/workspace/.proofs/native-providers/jimtcl/tcltest.tcl`, function `test setup/body/cleanup and literal comparison`, lines 345–372. Retained reference `native_tcltest_alias_caller_frame_original-original-inputs-workspace--.proofs--native-providers--jimtcl--tcltest.tcl`.

```text
	if {[catch {uplevel 1 $a(-setup)} msg]} {
		if {$::testinfo(verbose)} {
			puts "-setup failed: $msg"
		}
	}
	set rc [catch {uplevel 1 $a(-body)} result opts]
	if {[catch {uplevel 1 $a(-cleanup)} msg]} {
		if {$::testinfo(verbose)} {
			puts "-cleanup failed: $msg"
		}
	}

	if {[info return $rc] ni $a(-returnCodes) && $rc ni $a(-returnCodes)} {
		set ok 0
		set expected "rc=[list $a(-returnCodes)] result=[list $a(-result)]"
		set actual "rc=[info return $rc] result=[list $result]"
		# Now for the template, update -returnCodes
		set a(-returnCodes) [info return $rc]
	} else {
		if {$a(-match) eq "exact"} {
			set ok [string equal $a(-result) $result]
		} elseif {$a(-match) eq "glob"} {
			set ok [string match $a(-result) $result]
		} elseif {$a(-match) eq "regexp"} {
			set ok [regexp $a(-result) $result]
		} else {
			return -code error "$id: unknown match type: $a(-match)"
		}

```

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs)
- [rust/tcl-registry/tests/data/native_child_bootstrap_context303/source](../../../../rust/tcl-registry/tests/data/native_child_bootstrap_context303/source)
- [rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/jim](../../../../rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/jim)
- [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/source-windows/jim](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/source-windows/jim)
