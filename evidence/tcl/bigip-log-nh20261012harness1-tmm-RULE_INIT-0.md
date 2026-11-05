# BIG-IP Tcl log: nh20261012harness1, tmm-RULE_INIT-0

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **real-harness-explicit-source-child:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

## Assumptions

- **real-harness-explicit-source-child:** Independent child-interpreter real-package capability. Does not substitute for native-parent package require or original suite.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-rule-init-1](fixtures/bigip-tmm-rule-init-1.md)

- run: `nh20261012harness1`
- case: `real-harness-explicit-source-child`

### nh_setup

```tcl-visible
set nh_child [interp create]; set nh_module {}; append nh_module [call /Common/__tcl_lsp_probe_nh20261012harness1_0_module0::bytes] [call /Common/__tcl_lsp_probe_nh20261012harness1_0_module1::bytes] [call /Common/__tcl_lsp_probe_nh20261012harness1_0_module2::bytes] [call /Common/__tcl_lsp_probe_nh20261012harness1_0_module3::bytes] [call /Common/__tcl_lsp_probe_nh20261012harness1_0_module4::bytes] [call /Common/__tcl_lsp_probe_nh20261012harness1_0_module5::bytes] [call /Common/__tcl_lsp_probe_nh20261012harness1_0_module6::bytes] [call /Common/__tcl_lsp_probe_nh20261012harness1_0_module7::bytes] [call /Common/__tcl_lsp_probe_nh20261012harness1_0_module8::bytes]⟦EOF⟧
```

### nh_operation

```tcl-visible
set nh_child_version [interp eval $nh_child {list [info patchlevel] [info tclversion] [namespace current]}]; set nh_module_code [catch {interp eval $nh_child $nh_module} nh_module_result]; set nh_package_code [catch {interp eval $nh_child {package require tcltest}} nh_package_result]; list child-provider $nh_child_version module-code $nh_module_code module-result $nh_module_result require-code $nh_package_code require-result $nh_package_result⟦EOF⟧
```

### nh_cleanup

```tcl-visible
interp delete $nh_child⟦EOF⟧
```

## Results

### tmm-RULE_INIT, unit 0:0, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- real-harness-explicit-source-child, setup, completion 0, result:

```text-visible
# tcltest.tcl --
#
#⟦TAB⟧This file contains support code for the Tcl test suite.  It
#       defines the tcltest namespace and finds and defines the output
#       directory, constraints available, output and error channels,
#⟦TAB⟧etc. used by Tcl tests.  See the tcltest man page for more
#⟦TAB⟧details.
#
#       This design was based on the Tcl testing approach designed and
#       initially implemented by Mary Ann May-Pumphrey of Sun
#⟦TAB⟧Microsystems.
#
# Copyright (c) 1994-1997 Sun Microsystems, Inc.
# Copyright (c) 1998-1999 by Scriptics Corporation.
# Copyright (c) 2000 by Ajuba Solutions
# Contributions from Don Porter, NIST, 2002.  (not subject to US copyright)
# All rights reserved.
#
# RCS: @(#) $Id: tcltest.tcl,v 1.78.2.13 2005/02/24 18:03:36 dgp Exp $

package require Tcl 8.3⟦TAB⟧⟦TAB⟧;# uses [glob -directory]
namespace eval tcltest {

    # When the version number changes, be sure to update the pkgIndex.tcl file,
    # and the install directory in the Makefiles.  When the minor version
    # changes (new feature) be sure to update the man page as well.
    variable Version 2.2.8

    # Compatibility support for dumb variables defined in tcltest 1
    # Do not use these.  Call [package provide Tcl] and [info patchlevel]
    # yourself.  You don't need tcltest to wrap it for you.
    variable version [package provide Tcl]
    variable patchLevel [info patchlevel]

##### Export the public tcltest procs; several categories
    #
    # Export the main functional commands that do useful things
    namespace export cleanupTests loadTestedCommands makeDirectory \
⟦TAB⟧makeFile removeDirectory removeFile runAllTests test

    # Export configuration commands that control the functional commands
    namespace export configure customMatch errorChannel interpreter \
⟦TAB⟧    outputChannel testConstraint

    # Export commands that are duplication (candidates for deprecation)
    namespace export bytestring⟦TAB⟧⟦TAB⟧;# dups [encoding convertfrom identity]
    namespace export debug⟦TAB⟧⟦TAB⟧;#⟦TAB⟧[configure -debug]
    namespace export errorFile⟦TAB⟧⟦TAB⟧;#⟦TAB⟧[configure -errfile]
    namespace export limitConstraints⟦TAB⟧;#⟦TAB⟧[configure -limitconstraints]
    namespace export loadFile⟦TAB⟧⟦TAB⟧;#⟦TAB⟧[configure -loadfile]
    namespace export loadScript⟦TAB⟧⟦TAB⟧;#⟦TAB⟧[configure -load]
    namespace export match⟦TAB⟧⟦TAB⟧;#⟦TAB⟧[configure -match]
    namespace export matchFiles⟦TAB⟧⟦TAB⟧;#⟦TAB⟧[configure -file]
    namespace export matchDirectories⟦TAB⟧;#⟦TAB⟧[configure -relateddir]
    namespace export normalizeMsg⟦TAB⟧;#⟦TAB⟧application of [customMatch]
    namespace export normalizePath⟦TAB⟧;#⟦TAB⟧[file normalize] (8.4)
    namespace export outputFile⟦TAB⟧⟦TAB⟧;#⟦TAB⟧[configure -outfile]
    namespace export preserveCore⟦TAB⟧;#⟦TAB⟧[configure -preservecore]
    namespace export singleProcess⟦TAB⟧;#⟦TAB⟧[configure -singleproc]
    namespace export skip⟦TAB⟧⟦TAB⟧;#⟦TAB⟧[configure -skip]
    namespace export skipFiles⟦TAB⟧⟦TAB⟧;#⟦TAB⟧[configure -notfile]
    namespace export skipDirectories⟦TAB⟧;#⟦TAB⟧[configure -asidefromdir]
    namespace export temporaryDirectory⟦TAB⟧;#⟦TAB⟧[configure -tmpdir]
    namespace export testsDirectory⟦TAB⟧;#⟦TAB⟧[configure -testdir]
    namespace export verbose⟦TAB⟧⟦TAB⟧;#⟦TAB⟧[configure -verbose]
    namespace export viewFile⟦TAB⟧⟦TAB⟧;#⟦TAB⟧binary encoding [read]
    namespace export workingDirectory⟦TAB⟧;#⟦TAB⟧[cd] [pwd]

    # Export deprecated commands for tcltest 1 compatibility
    namespace export getMatchingFiles mainThread restoreState saveState \
⟦TAB⟧    threadReap

    # tcltest::normalizePath --
    #
    #     This procedure resolves any symlinks in the path thus creating
    #     a path without internal redirection. It assumes that the
    #     incoming path is absolute.
    #
    # Arguments
    #     pathVar - name of variable containing path to modify.
    #
    # Results
    #     The path is modified in place.
    #
    # Side Effects:
    #     None.
    #
    proc normalizePath {pathVar} {
⟦TAB⟧upvar $pathVar path
⟦TAB⟧set oldpwd [pwd]
⟦TAB⟧catch {cd $path}
⟦TAB⟧set path [pwd]
⟦TAB⟧cd $oldpwd
⟦TAB⟧return $path
    }

##### Verification commands used to test values of variables and options
    #
    # Verification command that accepts everything
    proc AcceptAll {value} {
⟦TAB⟧return $value
    }

    # Verification command that accepts valid Tcl lists
    proc AcceptList { list } {
⟦TAB⟧return [lrange $list 0 end]
    }

    # Verification command that accepts a glob pattern
    proc AcceptPattern { pattern } {
⟦TAB⟧return [AcceptAll $pattern]
    }

    # Verification command that accepts integers
    proc AcceptInteger { level } {
⟦TAB⟧return [incr level 0]
    }

    # Verification command that accepts boolean values
    proc AcceptBoolean { boolean } {
⟦TAB⟧return [expr {$boolean && $boolean}]
    }

    # Verification command that accepts (syntactically) valid Tcl scripts
    proc AcceptScript { script } {
⟦TAB⟧if {![info complete $script]} {
⟦TAB⟧    return -code error "invalid Tcl script: $script"
⟦TAB⟧}
⟦TAB⟧return $script
    }

    # Verification command that accepts (converts to) absolute pathnames
    proc AcceptAbsolutePath { path } {
⟦TAB⟧return [file join [pwd] $path]
    }

    # Verification command that accepts existing readable directories
    proc AcceptReadable { path } {
⟦TAB⟧if {![file readable $path]} {
⟦TAB⟧    return -code error "\"$path\" is not readable"
⟦TAB⟧}
⟦TAB⟧return $path
    }
    proc AcceptDirectory { directory } {
⟦TAB⟧set directory [AcceptAbsolutePath $directory]
⟦TAB⟧if {![file exists $directory]} {
⟦TAB⟧    return -code error "\"$directory\" does not exist"
⟦TAB⟧}
⟦TAB⟧if {![file isdir $directory]} {
⟦TAB⟧    return -code error "\"$directory\" is not a directory"
⟦TAB⟧}
⟦TAB⟧return [AcceptReadable $directory]
    }

##### Initialize internal arrays of tcltest, but only if the caller
    # has not already pre-initialized them.  This is done to support
    # compatibility with older tests that directly access internals
    # rather than go through command interfaces.
    #
    proc ArrayDefault {varName value} {
⟦TAB⟧variable $varName
⟦TAB⟧if {[array exists $varName]} {
⟦TAB⟧    return
⟦TAB⟧}
⟦TAB⟧if {[info exists $varName]} {
⟦TAB⟧    # Pre-initialized value is a scalar: destroy it!
⟦TAB⟧    unset $varName
⟦TAB⟧}
⟦TAB⟧array set $varName $value
    }

    # save the original environment so that it can be restored later
    ArrayDefault originalEnv [array get ::env]

    # initialize numTests array to keep track of the number of tests
    # that pass, fail, and are skipped.
    ArrayDefault numTests [list Total 0 Passed 0 Skipped 0 Failed 0]

    # createdNewFiles will store test files as indices and the list of
    # files (that should not have been) left behind by the test files
    # as values.
    ArrayDefault createdNewFiles {}

    # initialize skippedBecause array to keep track of constraints that
    # kept tests from running; a constraint name of "userSpecifiedSkip"
    # means that the test appeared on the list of tests that matched the
    # -skip value given to the flag; "userSpecifiedNonMatch" means that
    # the test didn't match the argument given to the -match flag; both
    # of these constraints are counted only if tcltest::debug is set to
    # true.
    ArrayDefault skippedBecause {}

    # initialize the testConstraints array to keep track of valid
    # predefined constraints (see the explanation for the
    # InitConstraints proc for more details).
    ArrayDefault testConstraints {}

##### Initialize internal variables of tcltest, but only if the caller
    # has not already pre-initialized them.  This is done to support
    # compatibility with older tests that directly access internals
    # rather than go through command interfaces.
    #
    proc Default {varName value {verify AcceptAll}} {
⟦TAB⟧variable $varName
⟦TAB⟧if {![info exists $varName]} {
⟦TAB⟧    variable $varName [$verify $value]
⟦TAB⟧} else {
⟦TAB⟧    variable $varName [$verify [set $varName]]
⟦TAB⟧}
    }

    # Save any arguments that we might want to pass through to other
    # programs.  This is used by the -args flag.
    # FINDUSER
    Default parameters {}

    # Count the number of files tested (0 if runAllTests wasn't called).
    # runAllTests will set testSingleFile to false, so stats will
    # not be printed until runAllTests calls the cleanupTests proc.
    # The currentFailure var stores the boolean value of whether the
    # current test file has had any failures.  The failFiles list
    # stores the names of test files that had failures.
    Default numTestFiles 0 AcceptInteger
    Default testSingleFile true AcceptBoolean
    Default currentFailure false AcceptBoolean
    Default failFiles {} AcceptList

    # Tests should remove all files they create.  The test suite will
    # check the current working dir for files created by the tests.
    # filesMade keeps track of such files created using the makeFile and
    # makeDirectory procedures.  filesExisted stores the names of
    # pre-existing files.
    #
    # Note that $filesExisted lists only those files that exist in
    # the original [temporaryDirectory].
    Default filesMade {} AcceptList
    Default filesExisted {} AcceptList
    proc FillFilesExisted {} {
⟦TAB⟧variable filesExisted

⟦TAB⟧# Save the names of files that already exist in the scratch directory.
⟦TAB⟧foreach file [glob -nocomplain -directory [temporaryDirectory] *] {
⟦TAB⟧    lappend filesExisted [file tail $file]
⟦TAB⟧}

⟦TAB⟧# After successful filling, turn this into a no-op.
⟦TAB⟧proc FillFilesExisted args {}
    }

    # Kept only for⟦EOF⟧
```

- real-harness-explicit-source-child: intended operation not recorded.

### tmm-RULE_INIT, unit 0:1, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- real-harness-explicit-source-child, setup, completion 0, result:

```text-visible
nteractive mode
⟦TAB⟧# This should be changed to determine if an event
⟦TAB⟧# loop is running, which is the real issue.
⟦TAB⟧# Actually, this doesn't belong here at all.  A package
⟦TAB⟧# really has no business [exit⟦EOF⟧
```

- real-harness-explicit-source-child: intended operation not recorded.

### tmm-RULE_INIT, unit 0:2, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- real-harness-explicit-source-child, lookup, completion 0, result:

```text-visible
child-provider {8.4.6 8.4 ::} module-code 1 module-result {invalid command name "pwd"} require-code 1 require-result {can't find package tcltest}⟦EOF⟧
```

- real-harness-explicit-source-child, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- real-harness-explicit-source-child: intended operation reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- real-harness-explicit-source-child, setup, completion 0, result:

```text-visible
# tcltest.tcl --
#
#⟦TAB⟧This file contains support code for the Tcl test suite.  It
#       defines the tcltest namespace and finds and defines the output
#       directory, constraints available, output and error channels,
#⟦TAB⟧etc. used by Tcl tests.  See the tcltest man page for more
#⟦TAB⟧details.
#
#       This design was based on the Tcl testing approach designed and
#       initially implemented by Mary Ann May-Pumphrey of Sun
#⟦TAB⟧Microsystems.
#
# Copyright (c) 1994-1997 Sun Microsystems, Inc.
# Copyright (c) 1998-1999 by Scriptics Corporation.
# Copyright (c) 2000 by Ajuba Solutions
# Contributions from Don Porter, NIST, 2002.  (not subject to US copyright)
# All rights reserved.
#
# RCS: @(#) $Id: tcltest.tcl,v 1.78.2.13 2005/02/24 18:03:36 dgp Exp $

package require Tcl 8.3⟦TAB⟧⟦TAB⟧;# uses [glob -directory]
namespace eval tcltest {

    # When the version number changes, be sure to update the pkgIndex.tcl file,
    # and the install directory in the Makefiles.  When the minor version
    # changes (new feature) be sure to update the man page as well.
    variable Version 2.2.8

    # Compatibility support for dumb variables defined in tcltest 1
    # Do not use these.  Call [package provide Tcl] and [info patchlevel]
    # yourself.  You don't need tcltest to wrap it for you.
    variable version [package provide Tcl]
    variable patchLevel [info patchlevel]

##### Export the public tcltest procs; several categories
    #
    # Export the main functional commands that do useful things
    namespace export cleanupTests loadTestedCommands makeDirectory \
⟦TAB⟧makeFile removeDirectory removeFile runAllTests test

    # Export configuration commands that control the functional commands
    namespace export configure customMatch errorChannel interpreter \
⟦TAB⟧    outputChannel testConstraint

    # Export commands that are duplication (candidates for deprecation)
    namespace export bytestring⟦TAB⟧⟦TAB⟧;# dups [encoding convertfrom identity]
    namespace export debug⟦TAB⟧⟦TAB⟧;#⟦TAB⟧[configure -debug]
    namespace export errorFile⟦TAB⟧⟦TAB⟧;#⟦TAB⟧[configure -errfile]
    namespace export limitConstraints⟦TAB⟧;#⟦TAB⟧[configure -limitconstraints]
    namespace export loadFile⟦TAB⟧⟦TAB⟧;#⟦TAB⟧[configure -loadfile]
    namespace export loadScript⟦TAB⟧⟦TAB⟧;#⟦TAB⟧[configure -load]
    namespace export match⟦TAB⟧⟦TAB⟧;#⟦TAB⟧[configure -match]
    namespace export matchFiles⟦TAB⟧⟦TAB⟧;#⟦TAB⟧[configure -file]
    namespace export matchDirectories⟦TAB⟧;#⟦TAB⟧[configure -relateddir]
    namespace export normalizeMsg⟦TAB⟧;#⟦TAB⟧application of [customMatch]
    namespace export normalizePath⟦TAB⟧;#⟦TAB⟧[file normalize] (8.4)
    namespace export outputFile⟦TAB⟧⟦TAB⟧;#⟦TAB⟧[configure -outfile]
    namespace export preserveCore⟦TAB⟧;#⟦TAB⟧[configure -preservecore]
    namespace export singleProcess⟦TAB⟧;#⟦TAB⟧[configure -singleproc]
    namespace export skip⟦TAB⟧⟦TAB⟧;#⟦TAB⟧[configure -skip]
    namespace export skipFiles⟦TAB⟧⟦TAB⟧;#⟦TAB⟧[configure -notfile]
    namespace export skipDirectories⟦TAB⟧;#⟦TAB⟧[configure -asidefromdir]
    namespace export temporaryDirectory⟦TAB⟧;#⟦TAB⟧[configure -tmpdir]
    namespace export testsDirectory⟦TAB⟧;#⟦TAB⟧[configure -testdir]
    namespace export verbose⟦TAB⟧⟦TAB⟧;#⟦TAB⟧[configure -verbose]
    namespace export viewFile⟦TAB⟧⟦TAB⟧;#⟦TAB⟧binary encoding [read]
    namespace export workingDirectory⟦TAB⟧;#⟦TAB⟧[cd] [pwd]

    # Export deprecated commands for tcltest 1 compatibility
    namespace export getMatchingFiles mainThread restoreState saveState \
⟦TAB⟧    threadReap

    # tcltest::normalizePath --
    #
    #     This procedure resolves any symlinks in the path thus creating
    #     a path without internal redirection. It assumes that the
    #     incoming path is absolute.
    #
    # Arguments
    #     pathVar - name of variable containing path to modify.
    #
    # Results
    #     The path is modified in place.
    #
    # Side Effects:
    #     None.
    #
    proc normalizePath {pathVar} {
⟦TAB⟧upvar $pathVar path
⟦TAB⟧set oldpwd [pwd]
⟦TAB⟧catch {cd $path}
⟦TAB⟧set path [pwd]
⟦TAB⟧cd $oldpwd
⟦TAB⟧return $path
    }

##### Verification commands used to test values of variables and options
    #
    # Verification command that accepts everything
    proc AcceptAll {value} {
⟦TAB⟧return $value
    }

    # Verification command that accepts valid Tcl lists
    proc AcceptList { list } {
⟦TAB⟧return [lrange $list 0 end]
    }

    # Verification command that accepts a glob pattern
    proc AcceptPattern { pattern } {
⟦TAB⟧return [AcceptAll $pattern]
    }

    # Verification command that accepts integers
    proc AcceptInteger { level } {
⟦TAB⟧return [incr level 0]
    }

    # Verification command that accepts boolean values
    proc AcceptBoolean { boolean } {
⟦TAB⟧return [expr {$boolean && $boolean}]
    }

    # Verification command that accepts (syntactically) valid Tcl scripts
    proc AcceptScript { script } {
⟦TAB⟧if {![info complete $script]} {
⟦TAB⟧    return -code error "invalid Tcl script: $script"
⟦TAB⟧}
⟦TAB⟧return $script
    }

    # Verification command that accepts (converts to) absolute pathnames
    proc AcceptAbsolutePath { path } {
⟦TAB⟧return [file join [pwd] $path]
    }

    # Verification command that accepts existing readable directories
    proc AcceptReadable { path } {
⟦TAB⟧if {![file readable $path]} {
⟦TAB⟧    return -code error "\"$path\" is not readable"
⟦TAB⟧}
⟦TAB⟧return $path
    }
    proc AcceptDirectory { directory } {
⟦TAB⟧set directory [AcceptAbsolutePath $directory]
⟦TAB⟧if {![file exists $directory]} {
⟦TAB⟧    return -code error "\"$directory\" does not exist"
⟦TAB⟧}
⟦TAB⟧if {![file isdir $directory]} {
⟦TAB⟧    return -code error "\"$directory\" is not a directory"
⟦TAB⟧}
⟦TAB⟧return [AcceptReadable $directory]
    }

##### Initialize internal arrays of tcltest, but only if the caller
    # has not already pre-initialized them.  This is done to support
    # compatibility with older tests that directly access internals
    # rather than go through command interfaces.
    #
    proc ArrayDefault {varName value} {
⟦TAB⟧variable $varName
⟦TAB⟧if {[array exists $varName]} {
⟦TAB⟧    return
⟦TAB⟧}
⟦TAB⟧if {[info exists $varName]} {
⟦TAB⟧    # Pre-initialized value is a scalar: destroy it!
⟦TAB⟧    unset $varName
⟦TAB⟧}
⟦TAB⟧array set $varName $value
    }

    # save the original environment so that it can be restored later
    ArrayDefault originalEnv [array get ::env]

    # initialize numTests array to keep track of the number of tests
    # that pass, fail, and are skipped.
    ArrayDefault numTests [list Total 0 Passed 0 Skipped 0 Failed 0]

    # createdNewFiles will store test files as indices and the list of
    # files (that should not have been) left behind by the test files
    # as values.
    ArrayDefault createdNewFiles {}

    # initialize skippedBecause array to keep track of constraints that
    # kept tests from running; a constraint name of "userSpecifiedSkip"
    # means that the test appeared on the list of tests that matched the
    # -skip value given to the flag; "userSpecifiedNonMatch" means that
    # the test didn't match the argument given to the -match flag; both
    # of these constraints are counted only if tcltest::debug is set to
    # true.
    ArrayDefault skippedBecause {}

    # initialize the testConstraints array to keep track of valid
    # predefined constraints (see the explanation for the
    # InitConstraints proc for more details).
    ArrayDefault testConstraints {}

##### Initialize internal variables of tcltest, but only if the caller
    # has not already pre-initialized them.  This is done to support
    # compatibility with older tests that directly access internals
    # rather than go through command interfaces.
    #
    proc Default {varName value {verify AcceptAll}} {
⟦TAB⟧variable $varName
⟦TAB⟧if {![info exists $varName]} {
⟦TAB⟧    variable $varName [$verify $value]
⟦TAB⟧} else {
⟦TAB⟧    variable $varName [$verify [set $varName]]
⟦TAB⟧}
    }

    # Save any arguments that we might want to pass through to other
    # programs.  This is used by the -args flag.
    # FINDUSER
    Default parameters {}

    # Count the number of files tested (0 if runAllTests wasn't called).
    # runAllTests will set testSingleFile to false, so stats will
    # not be printed until runAllTests calls the cleanupTests proc.
    # The currentFailure var stores the boolean value of whether the
    # current test file has had any failures.  The failFiles list
    # stores the names of test files that had failures.
    Default numTestFiles 0 AcceptInteger
    Default testSingleFile true AcceptBoolean
    Default currentFailure false AcceptBoolean
    Default failFiles {} AcceptList

    # Tests should remove all files they create.  The test suite will
    # check the current working dir for files created by the tests.
    # filesMade keeps track of such files created using the makeFile and
    # makeDirectory procedures.  filesExisted stores the names of
    # pre-existing files.
    #
    # Note that $filesExisted lists only those files that exist in
    # the original [temporaryDirectory].
    Default filesMade {} AcceptList
    Default filesExisted {} AcceptList
    proc FillFilesExisted {} {
⟦TAB⟧variable filesExisted

⟦TAB⟧# Save the names of files that already exist in the scratch directory.
⟦TAB⟧foreach file [glob -nocomplain -directory [temporaryDirectory] *] {
⟦TAB⟧    lappend filesExisted [file tail $file]
⟦TAB⟧}

⟦TAB⟧# After successful filling, turn this into a no-op.
⟦TAB⟧proc FillFilesExisted args {}
    }

    # Kept only for compatibility
    Default constraintsSpecified {} AcceptList
    trace variable constraintsSpecified r {set ::tcltest::constraintsSpecified \
⟦TAB⟧⟦TAB⟧[array names ::tcltest::testConstraints] ;# }

    # te⟦EOF⟧
```

- real-harness-explicit-source-child: intended operation not recorded.
