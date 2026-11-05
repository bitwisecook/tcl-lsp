# BIG-IP Tcl log: nh20261012harness1, tmm-traffic-event-0

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **real-harness-explicit-source-child:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

## Assumptions

- **real-harness-explicit-source-child:** Independent child-interpreter real-package capability. Does not substitute for native-parent package require or original suite.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-traffic-event-1](fixtures/bigip-tmm-traffic-event-1.md)

- run: `nh20261012harness1`
- case: `real-harness-explicit-source-child`

### nh_setup

```tcl-visible
set nh_child [interp create]; set nh_module {}; append nh_module [call /Common/__tcl_lsp_probe_nh20261012harness1_2_module0::bytes] [call /Common/__tcl_lsp_probe_nh20261012harness1_2_module1::bytes] [call /Common/__tcl_lsp_probe_nh20261012harness1_2_module2::bytes] [call /Common/__tcl_lsp_probe_nh20261012harness1_2_module3::bytes] [call /Common/__tcl_lsp_probe_nh20261012harness1_2_module4::bytes] [call /Common/__tcl_lsp_probe_nh20261012harness1_2_module5::bytes] [call /Common/__tcl_lsp_probe_nh20261012harness1_2_module6::bytes] [call /Common/__tcl_lsp_probe_nh20261012harness1_2_module7::bytes] [call /Common/__tcl_lsp_probe_nh20261012harness1_2_module8::bytes]⟦EOF⟧
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

### tmm-traffic-event, unit 0:0, invocation 0

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
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

    # tests that use threads need to know which is the main thread
    Default mainThread 1
    variable mainThread
    if {[info commands thread::id] != {}} {
⟦TAB⟧set mainThread [thread::id]
    } elseif {[info commands testthread] != {}} {
⟦TAB⟧set mainThread [testthread id]
    }

    # Set workingDirectory to [pwd]. The default output directory for
    # Tcl tests is the working directory.  Whenever this value changes
    # change to that directory.
    variable workingDirectory
    trace variable workingDirectory w \
⟦TAB⟧    [namespace code {cd $workingDirectory ;#}]

    Default workingDirectory [pwd] AcceptAbsolutePath
    proc workingDirectory { {dir ""} } {
⟦TAB⟧variable workingDirectory
⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $workingDirectory
⟦TAB⟧}
⟦TAB⟧set workingDirectory [AcceptAbsolutePath $dir]
    }

    # Set the location of the execuatble
    Default tcltest [info nameofexecutable]
    trace variable tcltest w [namespace code {testConstraint stdio \
⟦TAB⟧    [eval [ConstraintInitializer stdio]] ;#}]

    # save the platform information so it can be restored later
    Default originalTclPlatform [array get ::tcl_platform]

    # If a core file exists, save its modification time.
    if {[file exists [file join [workingDirectory] core]]} {
⟦TAB⟧Default coreModTime \
⟦TAB⟧⟦TAB⟧[file mtime [file join [workingDirectory] core]]
    }

    # stdout and stderr buffers for use when we want to store them
    Default outData {}
    Default errData {}

    # keep track of test level for nested test commands
    variable testLevel 0

    # the variables and procs that existed when saveState was called are
    # stored in a variable of the same name
    Default saveState {}

    # Internationalization support -- used in [SetIso8859_1_Locale] and
    # [RestoreLocale]. Those commands are used in cmdIL.test.

    if {![info exists [namespace current]::isoLocale]} {
⟦TAB⟧variable isoLocale fr
⟦TAB⟧switch -- $::tcl_platform(platform) {
⟦TAB⟧    "unix" {

⟦TAB⟧⟦TAB⟧# Try some 'known' values for some platforms:

⟦TAB⟧⟦TAB⟧switch -exact -- $::tcl_platform(os) {
⟦TAB⟧⟦TAB⟧    "FreeBSD" {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr_FR.ISO_8859-1
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    HP-UX {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr_FR.iso88591
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    Linux -
⟦TAB⟧⟦TAB⟧    IRIX {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    default {

⟦TAB⟧⟦TAB⟧⟦TAB⟧# Works on SunOS 4 and Solaris, and maybe
⟦TAB⟧⟦TAB⟧⟦TAB⟧# others...  Define it to something else on your
⟦TAB⟧⟦TAB⟧⟦TAB⟧# system if you want to test those.

⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale iso_8859_1
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧    "windows" {
⟦TAB⟧⟦TAB⟧set isoLocale French
⟦TAB⟧    }
⟦TAB⟧}
    }

    variable ChannelsWeOpened; array set ChannelsWeOpened {}
    # output goes to stdout by default
    Default outputChannel stdout
    proc outputChannel { {filename ""} } {
⟦TAB⟧variable outputChannel
⟦TAB⟧variable ChannelsWeOpened

⟦TAB⟧# This is very subtle and tricky, so let me try to explain.
⟦TAB⟧# (Hopefully this longer comment will be clear when I come
⟦TAB⟧# back in a few months, unlike its predecessor :) )
⟦TAB⟧#⟦SPACE⟧
⟦TAB⟧# The [outputChannel] command (and underlying variable) have to
⟦TAB⟧# be kept in sync with the [configure -outfile] configuration
⟦TAB⟧# option ( and underlying variable Option(-outfile) ).  This is
⟦TAB⟧# accomplished with a write trace on Option(-outfile) that will
⟦TAB⟧# update [outputChannel] whenver a new value is written.  That
⟦TAB⟧# much is easy.
⟦TAB⟧#
⟦TAB⟧# The trick is that in order to maintain compatibility with
⟦TAB⟧# version 1 of tcltest, we must allow every configuration option
⟦TAB⟧# to get its inital value from command line arguments.  This is
⟦TAB⟧# accomplished by setting initial read traces on all the
⟦TAB⟧# configuration options to parse the command line option the first
⟦TAB⟧# time they are read.  These traces are cancelled whenever the
⟦TAB⟧# program itself calls [configure].
⟦TAB⟧#⟦SPACE⟧
⟦TAB⟧# OK, then so to support tcltest 1 compatibility, it seems we want
⟦TAB⟧# to get the return from [outputFile] to trigger the read traces,
⟦TAB⟧# just in case.
⟦TAB⟧#
⟦TAB⟧# BUT!  A little known feature of Tcl variable traces is that⟦SPACE⟧
⟦TAB⟧# traces are disabled during the handling of other traces.  So,
⟦TAB⟧# if we trigger read traces on Option(-outfile) and that triggers
⟦TAB⟧# command line parsing which turns around and sets an initial
⟦TAB⟧# value for Option(-outfile) -- <whew!> -- the write trace that
⟦TAB⟧# would keep [outputChannel] in sync with that new initial value
⟦TAB⟧# would not fire!
⟦TAB⟧#
⟦TAB⟧# SO, finally, as a workaround, instead of triggering read traces
⟦TAB⟧# by invoking [outputFile], we instead trigger the same set of
⟦TAB⟧# read traces by invoking [debug].  Any command that reads a
⟦TAB⟧# configuration option would do.  [debug] is just a handy one.
⟦TAB⟧# The end result is that we support tcltest 1 compatibility and
⟦TAB⟧# keep outputChannel and -outfile in sync in all cases.
⟦TAB⟧debug

⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $outputChannel
⟦TAB⟧}
⟦TAB⟧if {[info exists ChannelsWeOpened($outputChannel)]} {
⟦TAB⟧    close $outputChannel
⟦TAB⟧    unset ChannelsWeOpened($outputChannel)
⟦TAB⟧}
⟦TAB⟧switch -exact -- $filename {
⟦TAB⟧    stderr -
⟦TAB⟧    stdout {
⟦TAB⟧⟦TAB⟧set outputChannel $filename
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧set outputChannel [open $filename a]
⟦TAB⟧⟦TAB⟧set ChannelsWeOpened($outputChannel) 1

⟦TAB⟧⟦TAB⟧# If we created the file in [temporaryDirectory], then
⟦TAB⟧⟦TAB⟧# [cleanupTests] will delete it, unless we claim it was
⟦TAB⟧⟦TAB⟧# already there.
⟦TAB⟧⟦TAB⟧set outdir [normalizePath [file dirname \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [pwd] $filename]]]
⟦TAB⟧⟦TAB⟧if {[string equal $outdir [temporaryDirectory]]} {
⟦TAB⟧⟦TAB⟧    variable filesExisted
⟦TAB⟧⟦TAB⟧    FillFilesExisted
⟦TAB⟧⟦TAB⟧    set filename [file tail $filename]
⟦TAB⟧⟦TAB⟧    if {[lsearch -exact $filesExisted $filename] == -1} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧lappend filesExisted $filename
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $outputChannel
    }

    # errors go to stderr by default
    Default errorChannel stderr
    proc errorChannel { {filename ""} } {
⟦TAB⟧variable errorChannel
⟦TAB⟧variable ChannelsWeOpened

⟦TAB⟧# This is subtle and tricky.  See the comment above in
⟦TAB⟧# [outputChannel] for a detailed explanation.
⟦TAB⟧debug

⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $errorChannel
⟦TAB⟧}
⟦TAB⟧if {[info exists ChannelsWeOpened($errorChannel)]} {
⟦TAB⟧    close $errorChannel
⟦TAB⟧    unset ChannelsWeOpened($errorChannel)
⟦TAB⟧}
⟦TAB⟧switch -exact -- $filename {
⟦TAB⟧    stderr -
⟦TAB⟧    stdout {
⟦TAB⟧⟦TAB⟧set errorChannel $filename
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧set errorChannel [open $filename a]
⟦TAB⟧⟦TAB⟧set ChannelsWeOpened($errorChannel) 1

⟦TAB⟧⟦TAB⟧# If we created the file in [temporaryDirectory], then
⟦TAB⟧⟦TAB⟧# [cleanupTests] will delete it, unless we claim it was
⟦TAB⟧⟦TAB⟧# already there.
⟦TAB⟧⟦TAB⟧set outdir [normalizePath [file dirname \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [pwd] $filename]]]
⟦TAB⟧⟦TAB⟧if {[string equal $outdir [temporaryDirectory]]} {
⟦TAB⟧⟦TAB⟧    variable filesExisted
⟦TAB⟧⟦TAB⟧    FillFilesExisted
⟦TAB⟧⟦TAB⟧    set filename [file tail $filename]
⟦TAB⟧⟦TAB⟧    if {[lsearch -exact $filesExisted $filename] == -1} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧lappend filesExisted $filename
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $errorChannel
    }

##### Set up the configurable options
    #
    # The configurable options of the package
    variable Option; array set Option {}

    # Usage strings for those options
    variable Usage; array set Usage {}

    # Verification commands for those options
    variable Verify; array set Verify {}

    # Initialize the default values of the configurable options that are
    # historically associated with an exported variable.  If that variable
    # is already set, support compatibility by accepting its pre-set value.
    # Use [trace] to establish ongoing connection between the deprecated
    # exported variable and the modern option kept as a true internal var.
    # Also set up usage string and value testing for the option.
    proc Option {option value usage {verify AcceptAll} {varName {}}} {
⟦TAB⟧variable Option
⟦TAB⟧variable Verify
⟦TAB⟧variable Usage
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧set Usage($option) $usage
⟦TAB⟧set Verify($option) $verify
⟦TAB⟧if {[catch {$verify $value} msg]} {
⟦TAB⟧    return -code error $msg
⟦TAB⟧} else {
⟦TAB⟧    set Option($option) $msg
⟦TAB⟧}
⟦TAB⟧if {[string length $varName]} {
⟦TAB⟧    variable $varName
⟦TAB⟧    if {[info exists $varName]} {
⟦TAB⟧⟦TAB⟧if {[catch {$verify [set $varName]} msg]} {
⟦TAB⟧⟦TAB⟧    return -code error $msg
⟦TAB⟧⟦TAB⟧} else {
⟦TAB⟧⟦TAB⟧    set Option($option) $msg
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧unset $varName
⟦TAB⟧    }
⟦TAB⟧    namespace eval [namespace current] \
⟦TAB⟧    ⟦TAB⟧    [list upvar 0 Option($option) $varName]
⟦TAB⟧    # Workaround for Bug (now Feature Request) 572889.  Grrrr....
⟦TAB⟧    # Track all the variables tied to options
⟦TAB⟧    lappend OptionControlledVariables $varName
⟦TAB⟧    # Later, set auto-configure read traces on all
⟦TAB⟧    # of them, since a single trace on Option does not work.
⟦TAB⟧    proc $varName {{value {}}} [subst -nocommands {
⟦TAB⟧⟦TAB⟧if {[llength [info level 0]] == 2} {
⟦TAB⟧⟦TAB⟧    Configure $option [set value]
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧return [Configure $option]
⟦TAB⟧    }]
⟦TAB⟧}
    }

    proc MatchingOption {option} {
⟦TAB⟧variable Option
⟦TAB⟧set match [array names Option $option*]
⟦TAB⟧switch -- [llength $match] {
⟦TAB⟧    0 {
⟦TAB⟧⟦TAB⟧set sorted [lsort [array names Option]]
⟦TAB⟧⟦TAB⟧set values [join [lrange $sorted 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append values ", or [lindex $sorted end]"
⟦TAB⟧⟦TAB⟧return -code error "unknown option $option: should be\
⟦TAB⟧⟦TAB⟧⟦TAB⟧one of $values"
⟦TAB⟧    }
⟦TAB⟧    1 {
⟦TAB⟧⟦TAB⟧return [lindex $match 0]
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧# Exact match trumps ambiguity
⟦TAB⟧⟦TAB⟧if {[lsearch -exact $match $option] >= 0} {
⟦TAB⟧⟦TAB⟧    return $option
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧set values [join [lrange $match 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append values ", or [lindex $match end]"
⟦TAB⟧⟦TAB⟧return -code error "ambiguous option $option:\
⟦TAB⟧⟦TAB⟧⟦TAB⟧could match $values"
⟦TAB⟧    }
⟦TAB⟧}
    }

    proc EstablishAutoConfigureTracee files produced during testing in the directory
⟦TAB⟧specified by -tmpdir. If 1, notify the user if core files are
⟦TAB⟧created.
    } AcceptInteger preserveCore

    # debug output doesn't get printed by dee ReadLoadScript]

    proc AcceptOutFile { file } {
⟦TAB⟧if {[string equal stderr $file]} {return $file}
⟦TAB⟧if {[string equal stdout $file]} {return $file}
⟦TAB⟧return [file join [temporaryDirectory] $file]
    }

    # output goes to stdout by default
    Option -outfile stdout {
⟦TAB⟧Send output from test runs to the specified file.
    } AcceptOutFile outputFile
    trace variable Option(-outfile) w \
⟦TAB⟧    [namespace code {outputChannel $Option(-outfile) ;#}]

    # errors go to stderr by default
    Option -errfile stderr {
⟦TAB⟧Send errors from test runs to the specified file.
    } AcceptOutFile errorFil    Internal helper procedures to write out debug information
#     dependent on the chosen level. A test shell may overide
#     them, f.e. to redirect the output into a different
#     channel, or even into a GUI.

# tcltest::DebugPuts --
#
#     Prints the specified string if the current debug level is
#     higher than the provided level argument.
#
# Arguments:
#     level   The lowest debug level triggering the output
#     string  The string to print out.
#
# Results:
#     Prints the string. Nothing else is allowed.
#
# Side Effects:
#     None.
#

proc tcltest::DebugPuts {level string} {
    variable debug
    if {$debug >= $level} {
⟦TAB⟧puts $string
    }
    return
}

# tcltest::DebugPArray --
#
#     Prints the contents of the specified array if the current
#       debug level is higher than the provided level argument
#
# Arguments:
#     level           The lowest debug level triggering the output
#     arrayvar        The name of the array to print out.
#
# Results:
#     Prints the contents of the array. Nothing else is allowed.
#
# Side Effects:
#     None.
#

proc tcltest::DebugPArray {level arrayvar} {
    variable debug

    if {$debug >= $level} {
⟦TAB⟧catch {upvar  $}
    # Check for boolean values
    if {[catch {expr {$value && $value}} msg]} {
⟦TAB⟧return -code error $msg
    }
    if {[limitConstraints]⟦SPACE⟧
⟦TAB⟧    && [lsearch -exact $Option(-constraints) $constraint]  equal {} $interp]} {
⟦TAB⟧set tcltest {}
    } else {
⟦TAB⟧set tcltest $interp
    }
}

#####################################################################

# tcltest::AddToSkippedBecause --
#
#⟦TAB⟧Increments the variable used to track how many tests were
#       skipped because of a particular constraint.
#
# Arguments:
#⟦TAB⟧constraint     The name of the constraint to be modified
#
# Results:
#⟦TAB⟧Modifies tcltest::skippedBecause; sets the variable to 1 if
#       didn't previously exist - otherwise, it just increments it.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::AddToSkippedBecause { constraint {value 1}} {
    # add the constraint to the list of constraints that kept tests
    # from running
    variable skippedBecause

    if {[info exists skippedBecause($constraint)]} {
⟦TAB⟧incr skippedBecause($constraint) $value
    } else {
⟦TAB⟧set skippedBecause($constraint) $value
    }
    return
}

# tcltest::PrintError --
#
#⟦TAB⟧Prints errors to tcltest::errorChannel and then flushes that
#       channel, makingtError {errorMsg} {
    set InitialMessage "Error:  "
    set InitialMsgLen  [string length $InitialMessage]
    puts -nonewline [errorChannel] $InitialMessage

    # Keep track of where the end of the string is.
    set endingIndex [string length $errorMsg]

    if {$endingIndex < (80 - $InitialMsgLen)} {
⟦TAB⟧puts [errorChannel] $errorMsg
    } else {
⟦TAB⟧# Print up to 80 characters on the first line, including the
⟦TAB⟧# InitialMessage.
⟦TAB⟧set beginningIndex [string last " " [string range $errorMsg 0 \
⟦TAB⟧⟦TAB⟧[expr {80 - $InitialMsgLen}]]]
⟦TAB⟧puts [errorChannel] [string range $errorMsg 0 $beginningIndex]

⟦TAB⟧while {![string equal end $beginningIndex]} {
⟦TAB⟧    puts -nonewline [errorChannel] \
⟦TAB⟧⟦TAB⟧    [string repeat " " $InitialMsgLen]
⟦TAB⟧    if {($endingIndex - $beginningIndex)
⟦TAB⟧⟦TAB⟧    < (80 - $InitialMsgLen)} {
⟦TAB⟧⟦TAB⟧ptring equal {} $n2]} {return}
    if {![info exists testConstraints($n2)]} {
⟦TAB⟧if {[catch {testConstraint $n2 [eval [ConstraintInitializer $n2]]}]} {
⟦TAB⟧    testConstraint $n2 0
⟦TAB⟧}
    }
}

# tcltest::Cocl_platform(os) "Windows 98"}

    # The following Constraints switches are used to mark tests that
    # should work, but have been temporarily disabled on certain
    # platforms because they don't and we haven't gotten around to
    # fixing the underlying problem.

    ConstraintInitializer tempNotPc {expr {![testConstraint pc]}}
    ConstraintInitializer tempNotWin {expr {![testConstraint win]}}
    ConstraintInitializer tempNotMac {expr {![testConstraint mac]}}
    ConstraintInitializer tempNotUnix {expr {![testConstraint unix]}}

    # The following Constraints switches are used to mark tests that
    # crash on certain platforms, so that they can be reactivated again
    # when the underlying problem is fixed.

    ConstraintInitializer pcCrash {expr {![testConstraint pc]}}
    ConstraintInitializer winCrash {expr {![testConstraint win]}}
    ConstraintInitializer macCrash {expr {![testConstraint mac]}}
    ConstraintInitializer unixCrash {expr {![testConstraint unix]}}

    # Skip empty tests

    ConstraintInitializer emptyTest {format 0}

    # By default, tests that expose known bugs are skipped.

    ConstraintInitializer knownBug {format 0}

    # By default, non-portable tests are skipped.

    ConstraintInitializer nonPortable {format 0}

    # Some tests require user interaction.

    ConstraintInitializer userInteraction {format 0}

    # Some tests must be sk if the installation came from a CD
    # image instead of a web image.  Some tests must be skipped if you
    # are running as root on Unix.  Other tests can only be run if you
    # are running as root on Unix.

    ConstraintInitializer root {expr \
⟦TAB⟧    {[string equal unix $::tcl_platform(platform)]
⟦TAB⟧    && ([string equal root $::tcl_platform(user)]
⟦TAB⟧⟦TAB⟧|| [string equal "" $::tcl_platform(user)] $f}]} {
⟦TAB⟧⟦TAB⟧    set code 1
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set code
    }

    # Deliberately call socket with the wrong number of arguments.  The
    # error message you get will indicate whether sockets are available This can
#⟦TAB⟧be customized with the redefinition of [PrintUsageInfoHook].
#
# Arguments:
#⟦TAB⟧none
#
# Results:
#       none
#
# Side Effects:
#       none
proc tcltest::PrintUsageInfo {} {
    puts [Usage]
    PrintUsageInfoHook
}

proc tcltest::Usage { {option ""} } {
    variable Usage
    variable Verify
    if {[llength [info level 0]] == 1} {
⟦TAB⟧set msg "Usage: [file tail [info nameofexecutable]] script "
⟦TAB⟧append msg "?-help? ?flag value? ... \n"
⟦TAB⟧append msg "Available flags (and valid input values) are:"

⟦TAB⟧set max 0
⟦TAB⟧set allOpts [concat -help [Configure]]
⟦TAB⟧foreach opt $allOpts {
⟦TAB⟧    set foo [Usage $opt]
⟦TAB⟧    foreach [list x type($opt) usage($opt)] $foo break
⟦TAB⟧    set line($opt) "  $opt $type($opt)  "
⟦TAB⟧    set length($opt) [string length $line($opt)]
⟦TAB⟧    if {$length($opt) > $max} {set max $length($opt)}
⟦TAB⟧}
⟦TAB⟧set rest [expr {72 - $max}]
⟦TAB⟧foreach opt $allOpts {
⟦TAB⟧    append msg \n$line($opt)
⟦TAB⟧    append msg [string repeat " " [expr {$max - $length($opt)}]]
⟦TAB⟧    set u [string trim $usage($opt)]
⟦TAB⟧    catch {append u "  (default: \[[Configure $opt]])"}
⟦TAB⟧    regsub -all {\s*\n\s*} $u " " u
⟦TAB⟧    while {[string length $u] > $rest} {
⟦TAB⟧⟦TAB⟧set break [string wordstart $u $rest]
⟦TAB⟧⟦TAB⟧if {$break == 0} {
⟦TAB⟧⟦TAB⟧    set break [string wordend $u 0]
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧append msg [string range $u 0 [expr {$break - 1}]]
⟦TAB⟧⟦TAB⟧set u [string trim [string range $u $break end]]
⟦TAB⟧⟦TAB⟧append msg \n[string repeat " " $max]
⟦TAB⟧    }
⟦TAB⟧    append msg $u
⟦TAB⟧}
⟦TAB⟧return $msg\n
    } elseif {[string equal -help $option]} {
⟦TAB⟧return [list -help "" "Display this usage information."]
    } else {
⟦TAB⟧set type [lindex [info args $Verify($option)] 0]
⟦TAB⟧return [list $option $type $Usage($option)]
    }
}

# tcltest::ProcessFlags --
#
#⟦TAB⟧process command line arguments supplied in the flagArray - this
#⟦TAB⟧is called by processCmdLineArgs.  Modifies tclteset moreOptions [processCmdLineArgsAddFlagsHook]
⟦TAB⟧⟦TAB⟧if {[lsearch -exact $moreOptions $option] == -1} {
⟦TAB⟧⟦TAB⟧    # Nope.  Report the error, including additional options,
⟦TAB⟧⟦TAB⟧    # but keep going
⟦TAB⟧⟦TAB⟧    if {[lry
⟦TAB⟧    while {![string equal [lindex $args 0] $option]} {
⟦TAB⟧⟦TAB⟧set args [lrange $args 2 end]
⟦TAB⟧    }
⟦TAB⟧    set args [lrange $args 2 end]
⟦TAB⟧}
⟦TAB⟧if {[llength $args] == 1} {
⟦TAB⟧    puts [errorChannel] \
⟦TAB⟧⟦TAB⟧    "missing value for option [lindex $args 0]"
⟦TAB⟧    exit 1
⟦TAB⟧}
    }

    # Call the hook
    catch {
        array set flag $flagArray
        processCmdLineArgsHook [array get flag]
    }
    return
}

#     Perform configuration according to the command-line options.
#
# Arguments:
#⟦TAB⟧none
#
# Results:
#⟦TAB⟧Sets the above-named variables in the tcltest namespace.
#
# Side Effects:
#       None.
#

proc tcltest::ProcessCmdLineArgs {} {
    variable originalEnv
    variable testConstraints

    # The "argv" var doesn't exist in some cases, so use {}.
    if {![info exists ::argv]} {
⟦TAB⟧ProcessFlags {}
  OPTIONS)]} {
⟦TAB⟧DebugPuts 2 \
⟦TAB⟧⟦TAB⟧"    ::env(TCLTEST_OPTIONS): $::env(TCLTEST_OPTIONS)"
    }
    if {[info exists ::argv]} {
⟦TAB⟧DebugPuts 2 "    argv: $::argv"
    }
    DebugPuts    2 "tcltest::debug              = [debug]"
    DebugPuts    2 "tcltest::testsDirectory     = [testsDirectory]"
    DebugPuts    2 "tcltest::workingDirectory   = [workingDirectory]"
    DebugPuts    2 "tcltest::temporaryDirectory = [temporaryDirectory]"
    DebugPuts    2 "tcltest::outputChannel      = [outputChannel]"
    DebugPuts    2 "tcltest::errorChannel       = [errorChannel]"
    DebugPuts    2 "Original environm#

# Code to run the tests goes here.

# tcltest::TestPuts --
#
#⟦TAB⟧Used to redefine puts in test environment.  Stores whatever goes
#⟦TAB⟧out on stdout in tcltest::outData and stderr in errData before
#⟦TAB⟧sending it on to the regular puts.
#
# Arguments:
#⟦TAB⟧same as standard puts
#
# Results:
#⟦TAB⟧none
#
# Side effects:
#       Intercepts puts; data that would otherwise go to stdout, stderr,
#⟦TAB⟧or file channell]} {
⟦TAB⟧if {[string equal $channel [[namespace parent]::outputChannel]]
⟦TAB⟧⟦TAB⟧|| [string equal $channel stdout]} {
⟦TAB⟧    append outData [lindex $args end]$newline
⟦TAB⟧    return
⟦TAB⟧} elseif {[string equal $channel [[namespace parent]::errorChannel]]
⟦TAB⟧⟦TAB⟧|| [string equal $channel stderr]} {
⟦TAB⟧    append errData [lindex $args end]$newline
⟦TAB⟧    return
⟦TAB⟧}
    }

    # If we haven't returned by now, we don't know how to handle the
    # input.  Let puts handle it.
    return [eval Puts $args]
}

# tcltest::Eval --
#
#⟦TAB⟧Evaluate the script in the test environment.  If ignoreOutput is
#       false, store data sent to stderr and stdout in outData and
#       errData.  Otherwise, ignore this output altogether.
#
# Arguments:
#⟦TAB⟧script             Script to evaluate
#       ?ignoreOutput?     Indicates whether or not to ignore output
#⟦TAB⟧⟦TAB⟧⟦TAB⟧   sent to stdout & stderr
#
# Results:
#⟦TAB⟧result from running the script
#
# Side effects:
#⟦TAB⟧Empties the contents of outData and errData before running a
#⟦TAB⟧test if ignoreOutput is set to 0.

proc tcltest::Eval {script {ignoreOutput 1}} {
    variable outData
    variable errData
    DebugPuts 3 "[lindex [info level 0] 0] called"
    if {!$ignoreOutput} {
⟦TAB⟧set outData {}
⟦TAB⟧set errData {}
⟦TAB⟧rename ::puts [namespace current]::Replace::Puts
⟦TAB⟧namespace eval :: \
⟦TAB⟧⟦TAB⟧[list namespace import [namespace origin Replace::puts]]
⟦TAB⟧namespace import Replace::puts
    }
    set result [uplevel 1 $script]
    if {!$ignoreOutput} {
⟦TAB⟧namespace forget puts
⟦TAB⟧namespace eval :: namespace forget puts
⟦TAB⟧rename [namespace current]::Replace::Puts ::puts
    }
    return $result
}

# tcltest::CompareStrings --
#
#⟦TAB⟧compares the expected answer to the actual answer, depending on
#⟦TAB⟧the mode provided.  Mode determines whether a regexp, exact,
#⟦TAB⟧glob or custom comparison is done.
#
# Arguments:
#⟦TAB⟧actual - string containing the actual result
#       expected - pattern to be matched against
#       mode - type of comparison to be done
#
# Results:
#⟦TAB⟧result of the match
#
# Side effects:
#⟦TAB⟧None.

:
#     None.
#

proc tcltest::SubstArguments {argList} {

    # We need to split the argList up into tokens but cannot use list
    # operations as they throw away some significant quoting, and
    # [split] ignores braces as it should.  Therefore what we do is
    # gradually build up a string out of whitespace seperated strings.
    # We cannot use [split] to split the argList into whitespace
    # separated strings as it throws away the whitespace which maybe
    # important so we have to do it all by hand.

    set result {}
    set token ""

    while {[string length $argList]} {
        # Look for the next word containing a quote: " { }
        if {[regexp -indices {[^ \t\n]*[\"\{\}]+[^ \t\n]*} \
⟦TAB⟧⟦TAB⟧$argList all]} {
            # Get the text leading up to this word, but not including
⟦TAB⟧    # this word, from the argList.
            set text [string range $argList 0 \
⟦TAB⟧⟦TAB⟧    [expr {[lindex $all 0] - 1}]]
            # Get the word with the quote
            set word [string range $argList \
                    [lindex $all 0] [lindex $all 1]]

            # Remove all text up to and including the word from the
            # argList.
            set argList [string range $argList \
                    [expr {[lindex $all 1] + 1}] end]
        } else {
            # Take everything up to the end of the argList.
            set text $argList
            set word {}
            set argList {}
        }

        if {$token != {}} {
            # If we saw a word with quote before, then there is a
            # multi-word token starting with that word.  In this case,
            # add the text and the current word to this token.
            append token $text $word
        } else {
            # Add the text to the result.  There is no need to parse
            # the text because it couldn't be a part of any multi-word
            # token.  Then start a new multi-word token with the word
            # because we need to pass this token to the Tcl parser to
           testLevel == 1} {
⟦TAB⟧    incr numTests(Passed)
⟦TAB⟧    if {[IsVerbose pass]} {
⟦TAB⟧⟦TAB⟧puts [outputChannel] "++++ $name PASSED"
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧incr testLevel -1
⟦TAB⟧return
    }

    # We know the test failed, tally it if {$setupFailure} {
⟦TAB⟧puts [outputChannel] "---- Test setup\
⟦TAB⟧⟦TAB⟧failed:\n$setupMsg"
⟦TAB⟧if {[info exists errorInfo(setup)]} {
⟦TAB⟧    puts [outputChannel] "---- errorInfo(setup): $errorInfo(setup)"
⟦TAB⟧    puts [outputChannel] "---- errorCode(setup): $errorCode(setup)"
⟦TAB⟧}
    }
    if {$scriptFailure} {
⟦TAB⟧if {$scriptCompare} {
⟦TAB⟧    puts [outputChannel] "---- Error testing result: $scriptMatch"
⟦TAB⟧} else {
⟦TAB⟧    puts [outputChannel] "---- Result was:\n$actualAnswer"
⟦TAB⟧    puts [outputChannel] "---- Result should have been\
⟦TAB⟧⟦TAB⟧    ($match matching):\n$result"
⟦TAB⟧}
    }
    if {$codeFailure} {
⟦TAB⟧switch -- $returnCode {
⟦TAB⟧    0 { set msg "Test completed normally" }
⟦TAB⟧    1 { set msg "Test generated error" }
⟦TAB⟧    2 { set msg "Test generated return exception" }
⟦TAB⟧    3 { set msg "Test generated break exception" }
⟦TAB⟧    4 { set msg "Test generated continue exception" }
⟦TAB⟧    default { set msg "Test generated exception" }
⟦TAB⟧}
⟦TAB⟧puts [outputChannel] "---- $msg; Return code was: $returnCode"
⟦TAB⟧puts [outputChannel] "----rInfo(body)"
⟦TAB⟧⟦TAB⟧puts [outputChannel] "---- errorCode: $errorCode(body)"
⟦TAB⟧    }
⟦TAB⟧}
    }
    if {$outputFailure} {
⟦TAB⟧if {$outputCompare} {
⟦TAB⟧    puts [outputChannel] "---- Error testing output: $outputMatch"
⟦TAB⟧} else {
⟦TAB⟧    puts [outputChannel] "---- Output was:\n$outData"
⟦TAB⟧    puts [outputChannel] "---- Output should have been\
⟦TAB⟧⟦TAB⟧    ($match matching):\n$output"
⟦TAB⟧}
    }
    if {$errorFailure} {
⟦TAB⟧if {$errorCompare} {
⟦TAB⟧    puts [outputChannel] "---- Error testing errorOutput: $errorMatch"
⟦TAB⟧} else {
⟦TAB⟧    puts [outputChannel] "---- Error output was:\n$errData"
⟦TAB⟧    puts [outputChannel] "---- Error output should have\
⟦TAB⟧⟦TAB⟧    been ($match matching):\n$errorOutput"
⟦TAB⟧}
    }
    if {$cleanupFailure} {
⟦TAB⟧puts [outputChannel] "---- Test cleanup failed:\n$cleanupMsg"
⟦TAB⟧if {[info exists errorInfo(cleanup)]}upon tcltest to specify
#       additional things that should be done at cleanup.
#

if {[llength [info commands tcltest::cleanupTestsHook]] == 0} {
    proc tcltest::cleanupTestsHook {} {}
}

# tcltest::cleanupTests --
#
# Remove files and dirs created using the makeFile and makeDirectory
# commands since the last time this proc was invoked.
#
# Print the names of the files created without the makeFile command
# since the tests were invoked.
#
# Print the number tests (total, passed, failed, and skipped) since the
# tests were invoked.
#
# Restore original environment (as reported by special variable env).
#
# Arguments:
#      calledFromAllFile - if 0, behave as if we are running a single
#      test file within an entire suite of tests.  if we aren't running
#      a single test file, then don't report status.  check for new
#      files created during the test run and report on them.  if 1,
#      report collated status from all the test file runs.
#
# Results:
#      None.
#
# Side Effects:
#      None
#

proc tcltest::cleanupTests {{calledFromAllFile 0}} {
    variable filesMade
    variable filesExisted
    variable createdNewFiles
    variable testSingleFile
    variable numTests
    variable numTestFiles
    variable failFiles
    variable skippedBecause
    variable currentFailure
    variable originalEnv
    variable originalTclPlatform
    variable coreModTime

    FillFilesExisted
    set testFileName [file tail [info script]]

    # Call the cleanup hook
    cleanupTestsHook

    # Remove files and directories created by the makeFile and
    # makeDirectory procedures.  Record the names of files in
    # workingDirectory that were not pre-existing, and associate them
    # with the test file that created them.

    if {!$calledFromAllFile} {
⟦TAB⟧foreach file $filesMade {
⟦TAB⟧    if {[file exists $file]} {
⟦TAB⟧⟦TAB⟧DebugDo 1 {Warn "cleanupTests deleting $file..."}
⟦TAB⟧⟦TAB⟧catch {file delete -force $file}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set currentFiles {}
⟦TAB⟧foreach file [glob -nocomplain \
⟦TAB⟧⟦TAB⟧-lse {
⟦TAB⟧⟦TAB⟧# Print a message if there is a core file and (1) there
⟦TAB⟧⟦TAB⟧# previously wasn't one or (2) the new one is different
⟦TAB⟧⟦TAB⟧# from the old one.

⟦TAB⟧⟦TAB⟧if {[info exists coreModTime]} {
⟦TAB⟧⟦TAB⟧    if {$coreModTime != [file mtime \
⟦TAB⟧⟦TAB⟧⟦TAB⟧    [file join [workingDirectory] core]]} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧puts [outputChannel] "A core file was created!"
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧} else {
⟦TAB⟧⟦TAB⟧    puts [outputChannel] "A core file was created!"
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
    }
    flush [outputChannel]
    flush [errorChannel]
    return
}

#####################################################################

# Procs that determine which tests/test files to run

# tcltest::GetMatchingFiles
#
#       Looks at the patterns given to match and skip files and uses
#⟦TAB⟧them to put together a list of the tests that will be run.
#
# Arguments:
#       directory to search
#
# Results:
#       The constructed list is returned to the user.  This will
#⟦TAB⟧primarily be used in 'all.tcl' files.  It is used in
#⟦TAB⟧runAllTests.
#
# Side Effects:
#       None

# a lower case version is needed for compatibility with tcltest 1.0
proc tcltest::getMatchingFiles args {eval GetMatchingFiles $args}

proc tcltest::GetMatchingFiles { args } {
    if {[llength $args]} {
⟦TAB⟧set dirList $args
    } else {
⟦TAB⟧# Finding tests only in [testsDirectory] is normal operation.
⟦TAB⟧# This procedure is written to accept multiple directory arguments
⟦TAB⟧# only to satisfy version 1 compatibility.
⟦TAB⟧set dirList [list [testsDirectory]]
    }

    set matchingFiles [list]
    foreach directory $dirList {

⟦TAB⟧# List files in $directory that match patterns to run.
⟦TAB⟧set matchFileList [list]
⟦TAB⟧foreach match [matchFiles] {
⟦TAB⟧    set matchFileList [concat $matchFileList \
⟦TAB⟧⟦TAB⟧    [glob -directory $directory -types {b c f p s} \
⟦TAB⟧⟦TAB⟧    -nocomplain -- $match]]
⟦TAB⟧}

⟦TAB⟧# List files in $directory that match patterns to skip.
⟦TAB⟧set skipFileList [list]
⟦TAB⟧foreach skip [skipFiles] {
⟦TAB⟧    set skipFileList [concat $skipFileList \
⟦TAB⟧⟦TAB⟧    [glob -directory $directory -types {b c f p s} \
⟦TAB⟧⟦TAB⟧    -nocomplain -- $skip]]
⟦TAB⟧}

⟦TAB⟧# Adall.tcl]]
⟦TAB⟧
⟦TAB⟧set endTime [eval $timeCmd]
⟦TAB⟧puts [outputChannel] "\n$dir test ended at $endTime"
⟦TAB⟧puts [outputChannel] ""
⟦TAB⟧puts [outputChannel] [string repeat ~ 44]
    }
    return
}

#####################################################################

# Test utility procs - not used in tcltest, but may be useful for
# testing.

# tcltest::loadTestedCommands --
#
#     Uses the specified script to load the commands to test. Allowed to
#     be empty, as the tested commands could have been compiled into the
#     interpreter.
#
# Arguments
#     none
#
# Results
#     none
#
# Side Effects:
#     none.

proc tcltest::loadTestedCommands {} {
    variable l
    if {[string equal {} [loadScript]]} {
⟦TAB⟧return
    }

    return [uplevel 1 [loadScript]]
}

# tcltest::saveState --
#
#⟦TAB⟧Save information regarding what procs and variables exist.
#
# Arguments:
#⟦TAB⟧none
#
# Results:
#⟦TAB⟧Modifies the variable saveState
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::saveState {} {
    variable saveState
    uplevel 1 [list ::set [namespace which -variable saveState]] \
⟦TAB⟧    {[::list [::info procs] [::info vars]]}
    DebugPuts  2 "[lindex [info level 0] 0]: $saveState"
    return
}

# tcltest::restoreState --
#
#⟦TAB⟧Remove procs and variables that didn't exist before the call to
#       [saveState].
#
# Arguments:
#⟦TAB⟧none
#
# Results:
#⟦TAB⟧Removes procs and variables from your environment if they don't
#⟦TAB⟧exist in the saveState variable.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::restoreState {} {
    variable saveState
    foreach p [uplevel 1 {::info procs}] {
⟦TAB⟧if {([lsearch [lindex $saveState 0] $p] < 0)
⟦TAB⟧⟦TAB⟧&& ![string equal [namespace current]::$p \
⟦TAB⟧⟦TAB⟧[uplevel 1 [list ::namespace origin $p]]]} {

⟦TAB⟧    DebugPuts 2 "[lindex [info level 0] 0]: Removing proc $p"
⟦TAB⟧    uplevel 1 [list ::catch [list ::rename $p {}]]
⟦TAB⟧}
    }
    foreach p [uplevel 1 {::info vars}] {
⟦TAB⟧if {[lsearch [lindex $saveState 1] $p] < 0} {
⟦TAB⟧    DebugPuts 2 "[lindex [info level 0] 0]:\
⟦TAB⟧⟦TAB⟧    Removing variable $p"
⟦TAB⟧    uplevel 1 [live
#
# Results:
#⟦TAB⟧return value from [file delete]
#
# Side effects:
#⟦TAB⟧None

proc tcltest::removeDirectory {name {directory ""}} {
    variable filesMade
    FillFilesExisted
    if {[llength [info level 0]] == 2} {
⟦TAB⟧set directory [temporaryDirectory]
    }
    set fullName [file join $directory $name]
    DebugPuts 3 "[lindex [info level 0] 0]: deleting $fullName"
    set idx [lsearch -exact $filesMade $fullName]
    set filesMade [lreplace $filesMade $idx $idx]
    if {$idx == -1} {
⟦TAB⟧DebugDo 1 {
⟦TAB⟧    Warn "removeDirectory removing \"$fullName\":\n  not created\
⟦TAB⟧⟦TAB⟧    by makeDirectory"
⟦TAB⟧}
    }⟦SPACE⟧
    if {![file isdirectory $fullName]} {
⟦TAB⟧DebugDo 1 {
⟦TAB⟧    Warn "removeDirectory removing \"$fullName\":\n  not a directory"
⟦TAB⟧}
    }
    return [file delete -force $fullName]
}

# tcltest::viewFile --
#
#⟦TAB⟧reads the content of a file and returns it
#
# Arguments:
#⟦TAB⟧name of the file to read
#       directory in which file is located
#
# Results:
#⟦TAB⟧content of the named file
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::viewFile {name {directory ""}} {
    FillFilesExisted
    if {[llength [info level 0]] == 2} {
⟦TAB⟧set directory [temporaryDirectory]
    }
    set fullName [file join $directory $name]
    set f [open $fullName]
    set data [read -nonewline $f]
    close $f
    return $data
}

# tcltest::bytestring --
#
# Construct a string that consists of the requested sequence of bytes,
# as opposed to a string of properly formed UTF-8 characters.
# This allows the tester to
# 1. Create denormalized or improperly formed strings to pass to C
#    procedures that are supposed to accept strings with embedded NULL
#    bytes.
# 2. Confirm that a string result has a certain pattern of bytes, for
#    instance to confirm that "\xe0\0" in a Tcl script is stored
#    internally in UTF-8 as the sequence of bytes "\xc3\xa0\xc0\x80".
#
# Generally, it's a bad idea to examine the bytes in a Tcl string or to
# construct improperly formed strings in this manner, because it involves
# exposing th⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 0, result:

```text-visible
child-provider {8.4.6 8.4 ::} module-code 1 module-result {invalid command name "pwd"} require-code 1 require-result {can't find package tcltest}⟦EOF⟧
```

- real-harness-explicit-source-child, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- real-harness-explicit-source-child: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 1

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
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

    # tests that use threads need to know which is the main thread
    Default mainThread 1
    variable mainThread
    if {[info commands thread::id] != {}} {
⟦TAB⟧set mainThread [thread::id]
    } elseif {[info commands testthread] != {}} {
⟦TAB⟧set mainThread [testthread id]
    }

    # Set workingDirectory to [pwd]. The default output directory for
    # Tcl tests is the working directory.  Whenever this value changes
    # change to that directory.
    variable workingDirectory
    trace variable workingDirectory w \
⟦TAB⟧    [namespace code {cd $workingDirectory ;#}]

    Default workingDirectory [pwd] AcceptAbsolutePath
    proc workingDirectory { {dir ""} } {
⟦TAB⟧variable workingDirectory
⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $workingDirectory
⟦TAB⟧}
⟦TAB⟧set workingDirectory [AcceptAbsolutePath $dir]
    }

    # Set the location of the execuatble
    Default tcltest [info nameofexecutable]
    trace variable tcltest w [namespace code {testConstraint stdio \
⟦TAB⟧    [eval [ConstraintInitializer stdio]] ;#}]

    # save the platform information so it can be restored later
    Default originalTclPlatform [array get ::tcl_platform]

    # If a core file exists, save its modification time.
    if {[file exists [file join [workingDirectory] core]]} {
⟦TAB⟧Default coreModTime \
⟦TAB⟧⟦TAB⟧[file mtime [file join [workingDirectory] core]]
    }

    # stdout and stderr buffers for use when we want to store them
    Default outData {}
    Default errData {}

    # keep track of test level for nested test commands
    variable testLevel 0

    # the variables and procs that existed when saveState was called are
    # stored in a variable of the same name
    Default saveState {}

    # Internationalization support -- used in [SetIso8859_1_Locale] and
    # [RestoreLocale]. Those commands are used in cmdIL.test.

    if {![info exists [namespace current]::isoLocale]} {
⟦TAB⟧variable isoLocale fr
⟦TAB⟧switch -- $::tcl_platform(platform) {
⟦TAB⟧    "unix" {

⟦TAB⟧⟦TAB⟧# Try some 'known' values for some platforms:

⟦TAB⟧⟦TAB⟧switch -exact -- $::tcl_platform(os) {
⟦TAB⟧⟦TAB⟧    "FreeBSD" {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr_FR.ISO_8859-1
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    HP-UX {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr_FR.iso88591
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    Linux -
⟦TAB⟧⟦TAB⟧    IRIX {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    default {

⟦TAB⟧⟦TAB⟧⟦TAB⟧# Works on SunOS 4 and Solaris, and maybe
⟦TAB⟧⟦TAB⟧⟦TAB⟧# others...  Define it to something else on your
⟦TAB⟧⟦TAB⟧⟦TAB⟧# system if you want to test those.

⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale iso_8859_1
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧    "windows" {
⟦TAB⟧⟦TAB⟧set isoLocale French
⟦TAB⟧    }
⟦TAB⟧}
    }

    variable ChannelsWeOpened; array set ChannelsWeOpened {}
    # output goes to stdout by default
    Default outputChannel stdout
    proc outputChannel { {filename ""} } {
⟦TAB⟧variable outputChannel
⟦TAB⟧variable ChannelsWeOpened

⟦TAB⟧# This is very subtle and tricky, so let me try to explain.
⟦TAB⟧# (Hopefully this longer comment will be clear when I come
⟦TAB⟧# back in a few months, unlike its predecessor :) )
⟦TAB⟧#⟦SPACE⟧
⟦TAB⟧# The [outputChannel] command (and underlying variable) have to
⟦TAB⟧# be kept in sync with the [configure -outfile] configuration
⟦TAB⟧# option ( and underlying variable Option(-outfile) ).  This is
⟦TAB⟧# accomplished with a write trace on Option(-outfile) that will
⟦TAB⟧# update [outputChannel] whenver a new value is written.  That
⟦TAB⟧# much is easy.
⟦TAB⟧#
⟦TAB⟧# The trick is that in order to maintain compatibility with
⟦TAB⟧# version 1 of tcltest, we must allow every configuration option
⟦TAB⟧# to get its inital value from command line arguments.  This is
⟦TAB⟧# accomplished by setting initial read traces on all the
⟦TAB⟧# configuration options to parse the command line option the first
⟦TAB⟧# time they are read.  These traces are cancelled whenever the
⟦TAB⟧# program itself calls [configure].
⟦TAB⟧#⟦SPACE⟧
⟦TAB⟧# OK, then so to support tcltest 1 compatibility, it seems we want
⟦TAB⟧# to get the return from [outputFile] to trigger the read traces,
⟦TAB⟧# just in case.
⟦TAB⟧#
⟦TAB⟧# BUT!  A little known feature of Tcl variable traces is that⟦SPACE⟧
⟦TAB⟧# traces are disabled during the handling of other traces.  So,
⟦TAB⟧# if we trigger read traces on Option(-outfile) and that triggers
⟦TAB⟧# command line parsing which turns around and sets an initial
⟦TAB⟧# value for Option(-outfile) -- <whew!> -- the write trace that
⟦TAB⟧# would keep [outputChannel] in sync with that new initial value
⟦TAB⟧# would not fire!
⟦TAB⟧#
⟦TAB⟧# SO, finally, as a workaround, instead of triggering read traces
⟦TAB⟧# by invoking [outputFile], we instead trigger the same set of
⟦TAB⟧# read traces by invoking [debug].  Any command that reads a
⟦TAB⟧# configuration option would do.  [debug] is just a handy one.
⟦TAB⟧# The end result is that we support tcltest 1 compatibility and
⟦TAB⟧# keep outputChannel and -outfile in sync in all cases.
⟦TAB⟧debug

⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $outputChannel
⟦TAB⟧}
⟦TAB⟧if {[info exists ChannelsWeOpened($outputChannel)]} {
⟦TAB⟧    close $outputChannel
⟦TAB⟧    unset ChannelsWeOpened($outputChannel)
⟦TAB⟧}
⟦TAB⟧switch -exact -- $filename {
⟦TAB⟧    stderr -
⟦TAB⟧    stdout {
⟦TAB⟧⟦TAB⟧set outputChannel $filename
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧set outputChannel [open $filename a]
⟦TAB⟧⟦TAB⟧set ChannelsWeOpened($outputChannel) 1

⟦TAB⟧⟦TAB⟧# If we created the file in [temporaryDirectory], then
⟦TAB⟧⟦TAB⟧# [cleanupTests] will delete it, unless we claim it was
⟦TAB⟧⟦TAB⟧# already there.
⟦TAB⟧⟦TAB⟧set outdir [normalizePath [file dirname \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [pwd] $filename]]]
⟦TAB⟧⟦TAB⟧if {[string equal $outdir [temporaryDirectory]]} {
⟦TAB⟧⟦TAB⟧    variable filesExisted
⟦TAB⟧⟦TAB⟧    FillFilesExisted
⟦TAB⟧⟦TAB⟧    set filename [file tail $filename]
⟦TAB⟧⟦TAB⟧    if {[lsearch -exact $filesExisted $filename] == -1} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧lappend filesExisted $filename
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $outputChannel
    }

    # errors go to stderr by default
    Default errorChannel stderr
    proc errorChannel { {filename ""} } {
⟦TAB⟧variable errorChannel
⟦TAB⟧variable ChannelsWeOpened

⟦TAB⟧# This is subtle and tricky.  See the comment above in
⟦TAB⟧# [outputChannel] for a detailed explanation.
⟦TAB⟧debug

⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $errorChannel
⟦TAB⟧}
⟦TAB⟧if {[info exists ChannelsWeOpened($errorChannel)]} {
⟦TAB⟧    close $errorChannel
⟦TAB⟧    unset ChannelsWeOpened($errorChannel)
⟦TAB⟧}
⟦TAB⟧switch -exact -- $filename {
⟦TAB⟧    stderr -
⟦TAB⟧    stdout {
⟦TAB⟧⟦TAB⟧set errorChannel $filename
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧set errorChannel [open $filename a]
⟦TAB⟧⟦TAB⟧set ChannelsWeOpened($errorChannel) 1

⟦TAB⟧⟦TAB⟧# If we created the file in [temporaryDirectory], then
⟦TAB⟧⟦TAB⟧# [cleanupTests] will delete it, unless we claim it was
⟦TAB⟧⟦TAB⟧# already there.
⟦TAB⟧⟦TAB⟧set outdir [normalizePath [file dirname \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [pwd] $filename]]]
⟦TAB⟧⟦TAB⟧if {[string equal $outdir [temporaryDirectory]]} {
⟦TAB⟧⟦TAB⟧    variable filesExisted
⟦TAB⟧⟦TAB⟧    FillFilesExisted
⟦TAB⟧⟦TAB⟧    set filename [file tail $filename]
⟦TAB⟧⟦TAB⟧    if {[lsearch -exact $filesExisted $filename] == -1} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧lappend filesExisted $filename
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $errorChannel
    }

##### Set up the configurable options
    #
    # The configurable options of the package
    variable Option; array set Option {}

    # Usage strings for those options
    variable Usage; array set Usage {}

    # Verification commands for those options
    variable Verify; array set Verify {}

    # Initialize the default values of the configurable options that are
    # historically associated with an exported variable.  If that variable
    # is already set, support compatibility by accepting its pre-set value.
    # Use [trace] to establish ongoing connection between the deprecated
    # exported variable and the modern option kept as a true internal var.
    # Also set up usage string and value testing for the option.
    proc Option {option value usage {verify AcceptAll} {varName {}}} {
⟦TAB⟧variable Option
⟦TAB⟧variable Verify
⟦TAB⟧variable Usage
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧set Usage($option) $usage
⟦TAB⟧set Verify($option) $verify
⟦TAB⟧if {[catch {$verify $value} msg]} {
⟦TAB⟧    return -code error $msg
⟦TAB⟧} else {
⟦TAB⟧    set Option($option) $msg
⟦TAB⟧}
⟦TAB⟧if {[string length $varName]} {
⟦TAB⟧    variable $varName
⟦TAB⟧    if {[info exists $varName]} {
⟦TAB⟧⟦TAB⟧if {[catch {$verify [set $varName]} msg]} {
⟦TAB⟧⟦TAB⟧    return -code error $msg
⟦TAB⟧⟦TAB⟧} else {
⟦TAB⟧⟦TAB⟧    set Option($option) $msg
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧unset $varName
⟦TAB⟧    }
⟦TAB⟧    namespace eval [namespace current] \
⟦TAB⟧    ⟦TAB⟧    [list upvar 0 Option($option) $varName]
⟦TAB⟧    # Workaround for Bug (now Feature Request) 572889.  Grrrr....
⟦TAB⟧    # Track all the variables tied to options
⟦TAB⟧    lappend OptionControlledVariables $varName
⟦TAB⟧    # Later, set auto-configure read traces on all
⟦TAB⟧    # of them, since a single trace on Option does not work.
⟦TAB⟧    proc $varName {{value {}}} [subst -nocommands {
⟦TAB⟧⟦TAB⟧if {[llength [info level 0]] == 2} {
⟦TAB⟧⟦TAB⟧    Configure $option [set value]
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧return [Configure $option]
⟦TAB⟧    }]
⟦TAB⟧}
    }

    proc MatchingOption {option} {
⟦TAB⟧variable Option
⟦TAB⟧set match [array names Option $option*]
⟦TAB⟧switch -- [llength $match] {
⟦TAB⟧    0 {
⟦TAB⟧⟦TAB⟧set sorted [lsort [array names Option]]
⟦TAB⟧⟦TAB⟧set values [join [lrange $sorted 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append values ", or [lindex $sorted end]"
⟦TAB⟧⟦TAB⟧return -code error "unknown option $option: should be\
⟦TAB⟧⟦TAB⟧⟦TAB⟧one of $values"
⟦TAB⟧    }
⟦TAB⟧    1 {
⟦TAB⟧⟦TAB⟧return [lindex $match 0]
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧# Exact match trumps ambiguity
⟦TAB⟧⟦TAB⟧if {[lsearch -exact $match $option] >= 0} {
⟦TAB⟧⟦TAB⟧    return $option
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧set values [join [lrange $match 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append values ", or [lindex $match end]"
⟦TAB⟧⟦TAB⟧return -code error "ambiguous option $option:\
⟦TAB⟧⟦TAB⟧⟦TAB⟧could match $values"
⟦TAB⟧    }
⟦TAB⟧}
    }

    proc EstablishAutoConfigureTraces {} {
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧foreach varName [concat $OptionControlledVariables Option] {
⟦TAB⟧    variable $varName
⟦TAB⟧    trace variable $varName r [namespace code {ProcessCmdLineArgs ;#}]
⟦TAB⟧}
    }

    proc RemoveAutoConfigureTraces {} {
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧foreach varName [concat $OptionControlledVariables Option] {
⟦TAB⟧    variable $varName
⟦TAB⟧    foreach pair [trace vinfo $varName] {
⟦TAB⟧⟦TAB⟧foreach {op cmd} $pair break
⟦TAB⟧⟦TAB⟧if {[string equal r $op]
⟦TAB⟧⟦TAB⟧⟦TAB⟧&& [string match *ProcessCmdLineArgs* $cmd]} {
⟦TAB⟧⟦TAB⟧    trace vdelete $varName $op $cmd
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧# Once the traces are removed, this can become a no-op
⟦TAB⟧proc RemoveAutoConfigureTraces {} {}
    }

    proc Configure args {
⟦TAB⟧variable Option
⟦TAB⟧variable Verify
⟦TAB⟧set n [llength $args]
⟦TAB⟧if {$n == 0} {
⟦TAB⟧    return [lsort [array names Option]]
⟦TAB⟧}
⟦TAB⟧if {$n == 1} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    return $Option($option)
⟦TAB⟧}
⟦TAB⟧while {[llength $args] > 1} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    if {[catch {$Verify($option) [lindex $args 1]} value]} {
⟦TAB⟧⟦TAB⟧return -code error "invalid $option\
⟦TAB⟧⟦TAB⟧⟦TAB⟧value \"[lindex $args 1]\": $value"
⟦TAB⟧    }
⟦TAB⟧    set Option($option) $value
⟦TAB⟧    set args [lrange $args 2 end]
⟦TAB⟧}
⟦TAB⟧if {[llength $args]} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    return -code error "missing value for option $option"
⟦TAB⟧}
    }
    proc configure args {
⟦TAB⟧RemoveAutoConfigureTraces
⟦TAB⟧set code [catch {eval Configure $args} msg]
⟦TAB⟧return -code $code $msg
    }
⟦SPACE⟧⟦SPACE⟧⟦SPACE⟧⟦SPACE⟧
    proc AcceptVerbose { level } {
⟦TAB⟧set level [AcceptList $level]
⟦TAB⟧if {[llength $level] == 1} {
⟦TAB⟧    if {![regexp {^(pass|body|skip|start|error)$} $level]} {
⟦TAB⟧⟦TAB⟧# translate single characters abbreviations to expanded list
⟦TAB⟧⟦TAB⟧set level [string map {p pass b body s skip t start e error} \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[split $level {}]]
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set valid [list]
⟦TAB⟧foreach v $level {
⟦TAB⟧    if {[regexp {^(pass|body|skip|start|error)$} $v]} {
⟦TAB⟧⟦TAB⟧lappend valid $v
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $valid
    }

    proc IsVerbose {level} {
⟦TAB⟧variable Option
⟦TAB⟧return [expr {[lsearch -exact $Option(-verbose) $level] != -1}]
    }

    # Default verbosity is to show bodies of failed tests
    Option -verbose {body error} {
⟦TAB⟧Takes any combination of the values 'p', 's', 'b', 't' and 'e'.
⟦TAB⟧Test suite will display all passed tests if 'p' is specified, all
⟦TAB⟧skipped tests if 's' is specified, the bodies of failed tests if
⟦TAB⟧'b' is specified, and when tests start if 't' is specified.
⟦TAB⟧ErrorInfo is displayed if 'e' is specified.
    } AcceptVerbose verbose

    # Match and skip patterns default to the empty list, except for
    # matchFiles, which defaults to all .test files in the
    # testsDirectory and matchDirectories, which defaults to all
    # directories.
    Option -match * {
⟦TAB⟧Run all tests within the specified files that match one of the
⟦TAB⟧list of glob patterns given.
    } AcceptList match

    Option -skip {} {
⟦TAB⟧Skip all tests within the specified tests (via -match) and files
⟦TAB⟧that match one of the list of glob patterns given.
    } AcceptList skip

    Option -file *.test {
⟦TAB⟧Run tests in all test files that match the glob pattern given.
    } AcceptPattern matchFiles

    # By default, skip files that appear to be SCCS lock files.
    Option -notfile l.*.test {
⟦TAB⟧Skip all test files that match the glob pattern given.
    } AcceptPattern skipFiles

    Option -relateddir * {
⟦TAB⟧Run tests in directories that match the glob pattern given.
    } AcceptPattern matchDirectories

    Option -asidefromdir {} {
⟦TAB⟧Skip tests in directories that match the glob pattern given.
    } AcceptPattern skipDirectories

    # By default, don't save core files
    Option -preservecore 0 {
⟦TAB⟧If 2, save any core files produced during testing in the directory
⟦TAB⟧specified by -tmpdir. If 1, notify the user if core files are
⟦TAB⟧created.
    } AcceptInteger preserveCore

    # debug output doesn't get printed by default; debug level 1 spits
    # up only the tests that were skipped because they didn't match or
    # were specifically skipped.  A debug level of 2 would spit up the
    # tcltest variables and flags provided; a debug level of 3 causes
    # some additional output regarding operations of the test harness.
    # The tcltest package currently implements only up to debug level 3.
    Option -debug 0 {
⟦TAB⟧Internal debug level⟦SPACE⟧
    } AcceptInteger debug

    proc SetSelectedConstraints args {
⟦TAB⟧variable Option
⟦TAB⟧foreach c $Option(-constraints) {
⟦TAB⟧    testConstraint $c 1
⟦TAB⟧}
    }
    Option -constraints {} {
⟦TAB⟧Do not skip the listed constraints listed in -constraints.
    } AcceptList
    trace variable Option(-constraints) w \
⟦TAB⟧    [namespace code {SetSelectedConstraints ;#}]

    # Don't run only the "-constraint" specified tests by default
    proc ClearUnselectedConstraints args {
⟦TAB⟧variable Option
⟦TAB⟧variable testConstraints
⟦TAB⟧if {!$Option(-limitconstraints)} {return}
⟦TAB⟧foreach c [array names testConstraints] {
⟦TAB⟧    if {[lsearch -exact $Option(-constraints) $c] == -1} {
⟦TAB⟧⟦TAB⟧testConstraint $c 0
⟦TAB⟧    }
⟦TAB⟧}
    }
    Option -limitconstraints false {
⟦TAB⟧whether to run only tests with the constraints
    } AcceptBoolean limitConstraints⟦SPACE⟧
    trace variable Option(-limitconstraints) w \
⟦TAB⟧    [namespace code {ClearUnselectedConstraints ;#}]

    # A test application has to know how to load the tested commands
    # into the interpreter.
    Option -load {} {
⟦TAB⟧Specifies the script to load the tested commands.
    } AcceptScript loadScript

    # Default is to run each test file in a separate process
    Option -singleproc 0 {
⟦TAB⟧whether to run all tests in one process
    } AcceptBoolean singleProcess⟦SPACE⟧

    proc AcceptTemporaryDirectory { directory } {
⟦TAB⟧set directory [AcceptAbsolutePath $directory]
⟦TAB⟧if {![file exists $directory]} {
⟦TAB⟧    file mkdir $directory
⟦TAB⟧}
⟦TAB⟧set directory [AcceptDirectory $directory]
⟦TAB⟧if {![file writable $directory]} {
⟦TAB⟧    if {[string equal [workingDirectory] $directory]} {
⟦TAB⟧⟦TAB⟧# Special exception: accept the default value
⟦TAB⟧⟦TAB⟧# even if the directory is not writable
⟦TAB⟧⟦TAB⟧return $directory
⟦TAB⟧    }
⟦TAB⟧    return -code error "\"$directory\" is not writeable"
⟦TAB⟧}
⟦TAB⟧return $direc  trace variable Option(-tmpdir) w \
⟦TAB⟧    [namespace code {normalizePath Option(-tmpdir) ;#}]

    # Tests should not rely on the current working directory.
    # Files that are part of the test suite should be accessed relative
    # to [testsDirectory]
    Option -testdir [workingDirectory] {
⟦TAB⟧Search tests in the specified directory.
    } AcceptDirectory testsDirectory
    trace variable Option(-testdir) w \
⟦TAB⟧    [namespace code {normalizePath Option(-testdir) ;#}]

    proc AcceptLoadFile { file } {
⟦TAB⟧if {[string equal "" $file]} {return $file}
⟦TAB⟧set file [file join [temporaryDirectory] $file]
⟦TAB⟧return [AcceptReadable $file]
    }
    proc ReadLoadScript {args} {
⟦TAB⟧variable Option
⟦TAB⟧if {[string equal "" $Option(-loadfile)]} {return}
⟦TAB⟧set tmp [open $Option(-loadfile) r]
⟦TAB⟧loadScript [read $tmp]
⟦TAB⟧close $tmp
    }
    Option -loadfile {} {
⟦TAB⟧Read the script to load the tested commands from the specified file.
    } AcceptLoadFile loadFile
    trace variable Option(-loadfile) w [namespace code ReadLoadScript]

    proc AcceptOutFile { file } {
⟦TAB⟧if {[string equal stderr $file]} {return $file}
⟦TAB⟧if {[string equal stdout $file]} {return $file}
⟦TAB⟧return [file join [temporaryDirectory] $file]
    }

    # output goes to stdout by default
    Option -outfile stdout {
⟦TAB⟧Send output from test runs to the specified file.
    } AcceptOutFile outputFile
    trace variable Option(-outfile) w \
⟦TAB⟧    [namespace code {outputChannel $Option(-outfile) ;#}]

    # errors go to stderr by default
    Option -errfile stderr {
⟦TAB⟧Send errors from test runs to the specified file.
    } AcceptOutFile errorFile
    trace variable Option(-errfile) w \
⟦TAB⟧    [namespace code {errorChannel $Option(-errfile) ;#}]

}

#####################################################################

# tcltest::Debug* --
#
# higher than the provided level argument
#
# Arguments:
#     level           The lowest debug level triggering the output
#     arrayvar        The name of the array to print out.
#
# Results:
#     Prints the contents of the array. Nothing else is allowed.
#
# Side Effects:
#     None.
#

proc tcltest::DebugPArray {level arrayvar} {
    variable debug

    if {$debug >= $level} {
⟦TAB⟧catch {upvar  $arrayvar $arrayvar}
⟦TAB⟧parray $arrayvar
    }
    return
}

# Define our own [parray] in ::tcltest that will inherit use of the [puts]
# defined in ::tcltest.  NOTE: Ought to construct with [info args] and
# [info default], but can't be bothered now.  If [parray] changes, then
# this will need changing too.
auto_load ::parray
proc tcltest::parray {a {pattern *}} [info body ::parray]

# tcltest::DebugDo --
#
#     Executes the script if the current debug level is greater than
#       the provided level argument
#
# Arguments:
#     level   The lowest debug level triggering the execution.
#     script  The tcl script executed upon a debug level high enough.
#
# Results:
#     Arbitrary side effects, dependent on the executed script.
#
# Side Effects:
#     None.
#

proc tcltest::DebugDo {level script} {
    variable debug

    if {$debug >= $level} {
⟦TAB⟧uplevel 1 $script
    }
    return
}

#####################################################################

proc tcltest::Warn {msg} {
    puts [outputChannel] "WARNING: $msg"
}

# tcltest::mainThread
#
#     Accessor command for tcltest variable mainThread.
#
proc tcltest::mainThread { {new ""} } {
    variable mainThread
    if {[llength [info level 0]] == 1} {
⟦TAB⟧return $mainThread
    }
    set mainThread $new
}

# tcltest::testConstraint --
#
#⟦TAB⟧sets a test constraint to a value; to do multiple constraints,
#       call this proc multiple times.  also returns the value of the
#       named constraint if no value was supplied.
#
# Arguments:
#⟦TAB⟧constraint - name of the constraint
#       value - new value for constraint (should be boolean) - if not
#               supplied, this is a query
#
# Results:
#⟦TAB⟧content of tcltest::testConstraints($constraint)
#
# Side effects:
#⟦TAB⟧none

proc tcltest::testConstraint {constraint {value ""}} {
    variable testConstraints
    variable Option
    DebugPuts 3 "entering testConstraint $constraint $value"
    if {[llength [info level 0]] == 2} {
⟦TAB⟧return $testConstraints($constraint)
    }
    # Check for boolean values
    if {[catch {expr {$value && $value}} msg]} {
⟦TAB⟧return -code error $msg
    }
    if {[limitConstraints]⟦SPACE⟧
⟦TAB⟧    && [lsearch -exact $Option(-constraints) $constraint] == -1} {
⟦TAB⟧set value 0
    }
    set testConstraints($constraint) $value
}

# tcltest::interpreter --
#
#⟦TAB⟧the interpreter name stored in tcltest::tcltest
#
# Arguments:
#⟦TAB⟧executable name
#
# Results:
#⟦TAB⟧content of tcltest::tcltest
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::interpreter { {interp ""} } {
    variable tcltest
    if {[llength [info level 0]] == 1} {
⟦TAB⟧return $tcltest
    }
    if {[string equal {} $interp]} {
⟦TAB⟧set tcltest {}
    } else {
⟦TAB⟧set tcltest $interp
    }
}

#####################################################################

# tcltest::AddToSkippedBecause --
#
#⟦TAB⟧Increments the variable used to track how many tests were
#       skipped because of a particular constraint.
#
# Arguments:
#⟦TAB⟧constraint     The name of the constraint to be modified
#
# Results:
#⟦TAB⟧Modifies tcltest::skippedBecause; sets the variable to 1 if
#       didn't previously exist - otherwise, it just increments it.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::AddToSkippedBecause { constraint {value 1}} {
    # add the constraint to the list of constraints that kept tests
    # from running
    variable skippedBecause

    if {[info exists skippedBecause($constraint)]} {
⟦TAB⟧incr skippedBecause($constraint) $value
    } else {
⟦TAB⟧set skippedBecause($constraint) $value
    }
    return
}

# tcltest::PrintError --
#
#⟦TAB⟧Prints errors to tcltest::errorChannel and then flushes that
#       channel, making⟦TAB⟧[expr {$beginningIndex
⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧+ (80 - $InitialMsgLen)}]
⟦TAB⟧⟦TAB⟧]] + $beginningIndex}]
⟦TAB⟧⟦TAB⟧if {($newEndingIndex <= 0)
⟦TAB⟧⟦TAB⟧⟦TAB⟧|| ($newEndingIndex <= $beginningIndex)} {
⟦TAB⟧⟦TAB⟧    set newEndingIndex end
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧puts [errofeFetch --
#
#⟦TAB⟧ The following trace procedure makes it so that we can safely
#        refer to non-existent members of the testConstraints array
#        without causing an error.  Instead, reading a non-existent
#        member will return 0. This is necessary because tests are
#        allowed to use constraint "X" without ensuring that
#        testConstraints("X") is defined.
#
# Arguments:
#⟦TAB⟧n1 - name of the array (testConstraints)
#       n2 - array key value (constraint name)
#       op - operation performed on testConstraints (generally r)
#
# Results:
#⟦TAB⟧none
#
# Side effects:
#⟦TAB⟧sets testConstraints($n2) to 0 if it's referenced but never
#       before used

proc tcltest::SafeFetch {n1 n2 op} {
    variable testConstraints
    DebugPuts 3 "entering SafeFetch $n1 $n2 $op"
    if {[string equal {} $n2]} {return}
    if {![info exists testConstraints($n2)]} {
⟦TAB⟧if {[catch {testConstraint $n2 [eval [ConstraintInitializer $n2]]}]} {
⟦TAB⟧    testConstraint $n2 0
⟦TAB⟧}
    }
}

# tcltest::ConstraintInitializer --
#
#⟦TAB⟧Get or set a script that when evaluated in the tcltest namespace
#⟦TAB⟧will return a boolean value with which to initialize the
#⟦TAB⟧associated constraint.
#
# Arguments:
#⟦TAB⟧constraint - name of the constraint initialized by the script
#⟦TAB⟧script - the initializer script
#
# Results
#⟦TAB⟧boolean value of the constraint - enabled or disabled
#
# Side effects:
#⟦TAB⟧Constraint is initialized for future reference by [test]
proc tcltest::ConstraintInitializer {constraint {script ""}} {
    variable ConstraintInitializer
    DebugPuts 3 "entering ConstraintInitializer $constraint $script"
    if {[llength [info level 0]] == 2} {
⟦TAB⟧return $ConstraintInitializer($constraint)
    }
    # Check for boolean values
    if {![info complete $script]} {
⟦TAB⟧return -code error "ConstraintInitializes are here for backward compatibility and
    # are not documented.  They have been replaced with equivalent 'win'
    # constraints.

    ConstraintInitializer unixOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) unix}
    ConstraintInitializer macOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) macintosh}
    ConstraintInitializer pcOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) windows}
    ConstraintInitializer winOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) windows}

    ConstraintInitializer unix {testConstraint unixOnly}
    ConstraintInitializer mac {testConstraint macOnly}
    ConstraintInitializer pc {testConstraint pcOnly}
    ConstraintInitializer win {testConstraint winOnly}

    ConstraintInitializer unixOrPc \
⟦TAB⟧    {expr {[testConstraint unix] || [testConstraint pc]}}
    ConstraintInitializer macOrPc \
⟦TAB⟧    {expr {[testConstraint mac] || [testConstraint pc]}}
    ConstraintInitializer unixOrWin \
⟦TAB⟧    {expr {[testConstraint unix] || [testConstraint win]}}
    ConstraintInitializer macOrWin \
⟦TAB⟧    {expr {[testConstraint mac] || [testConstraint win]}}
    ConstraintInitializer macOrUnix \
⟦TAB⟧    {expr {[testConstraint mac] || [testConstraint unix]}}

    ConstraintInitializer nt {string equal $::tcl_platform(os) "Windows NT"}
    ConstraintInitializer 95 {string equal $::tcl_platform(os) "Windows 95"}
    ConstraintInitializer 98 {string equal $::tcl_platform(os) "Windows 98"}

    # The following Constraints switches are used to mark tests that
    # should work, but have been temporarily disabled on certain
    # platforms because they don't and we haven't gotten around to
    # fixing the underlying problem.

    ConstraintInitializer tempNotPc {expr {![testConstraint pc]}}
    ConstraintInitializer tempNotWin {expr {![testConstraint win]}}
    ConstraintInitializer tempNotMac {expr {![testConstraint mac]}}
    ConstraintInitializer tempNotUnix {expr {![testConstraint unix]}}

    # The following Constraints switches are used to mark)}}
    ConstraintInitializer notRoot {expr {![testConstraint root]}}

    # Set nonBlockFiles constraint: 1 means this platform supports
    # setting files into nonblocking mode.

    ConstraintInitializer nonBlockFiles {
⟦TAB⟧    set code [expr {[catch {set f [open defs r]}]⟦SPACE⟧
⟦TAB⟧⟦TAB⟧    || [catch {fconfigure $f -blocking off}]}]
⟦TAB⟧    catch {close $f}
⟦TAB⟧    set code
    }

    # Set asyncPipeClose constraint: 1 means this platform supports
    # async flush and async close on a pipe.
    #
    # Test for SCO Unix - cannot run async flushing tests because a
    # potential problem with select is apparently interfering.
    # (Mark Diekhans).

    ConstraintInitializer asyncPipeClose {expr {
⟦TAB⟧    !([string equal unix $::tcl_platform(platform)]⟦SPACE⟧
⟦TAB⟧    && ([catch {exec uname -X | fgrep {Release = 3.2v}}] == 0))}}

    # Test to see if we have a broken version of sprintf with respect
    # to the "e" format of floating-point numbers.

    ConstraintInitializer eformat {string equal [format %g 5e-5] 5e-05}

    # Test to see if execed commands such as cat, echo, rm and so forth
    # are present on this machine.

    ConstraintInitializer unixExecs {
⟦TAB⟧set code 1
        if {[string equal macintosh $::tcl_platform(platform)]} {
⟦TAB⟧    set code 0
        }
        if {[string equal windows $::tcl_platform(platform)]} {
⟦TAB⟧    if {[catch {
⟦TAB⟧        set file _tcl_test_remove_me.txt
⟦TAB⟧        makeFile {hello} $file
⟦TAB⟧    }]} {
⟦TAB⟧        set code 0
⟦TAB⟧    } elseif {
⟦TAB⟧        [catch {exec cat $file}] ||
⟦TAB⟧        [catch {exec echo hello}] ||
⟦TAB⟧        [catch {exec sh -c echo hello}] ||
⟦TAB⟧        [catch {exec wc $file}] ||
⟦TAB⟧        [catch {exec sleep 1}] ||
⟦TAB⟧        [catch {exec echo abc > $file}] ||
⟦TAB⟧        [catch {exec chmod 644 $file}] ||
⟦TAB⟧        [catch {exec rm $file}] ||
⟦TAB⟧        [llength [auto_execok mkdir]] == 0 ||
⟦TAB⟧        [llength [auto_execok fgrep]] == 0 ||
⟦TAB⟧        [llength [auto_execok grep]] == 0 ||
⟦TAB⟧        [llength [auto_execok ps]] == 0
⟦TAB⟧    } {
⟦TAB⟧        set code 0
⟦TAB⟧    }
⟦TAB⟧    remoissing value for option [lindex $args 0]"
⟦TAB⟧    exit 1
⟦TAB⟧}
    }

    # Call the hook
    catch {
        array set flag $flagArray
        processCmdLineArgsHook [array get flag]
    }
    return
}

# tcltest::ProcessCmdLineArgs --
#
#       This procedure must be run after constraint initialization is
#⟦TAB⟧set up (by [DefineConstraintInitializers]) because some constraints
#⟦TAB⟧can be overridden.
#
#       Perform configuration according to the command-line options.
#
# Arguments:
#⟦TAB⟧none
#
# Results:
#⟦TAB⟧Sets the above-named variables in the tcltest namespace.
#
# Side Effects:
#       None.
#

proc tcltest::ProcessCmdLineArgs {} {
    variable originalEnv
    variable testConstraints

    # The "argv" var doesn't exist in some cases, so use {}.
    if {![info exists ::argv]} {
⟦TAB⟧ProcessFlags {}
    } else {
⟦TAB⟧ProcessFlags $::argv
    }

    # Spit out everything you know if we're at a debug level 2 or
    # greater
    DebugPuts 2 "Flags passed into tcltest:"
    if {[info exists ::env(TCLTEST_OPTIONS)]} {
⟦TAB⟧DebugPuts 2 \
⟦TAB⟧⟦TAB⟧"    ::env(TCLTEST_OPTIONS): $::env(TCLTEST_OPTIONS)"
    }
    if {[info exists ::argv]} {
⟦TAB⟧DebugPuts 2 "    argv: $::argv"
    }
    DebugPuts    2 "tcltest::debug              = [debug]"
    DebugPuts    2 "tcltest::testsDirectory     = [testsDirectory]"
    DebugPuts    2 "tcltest::workingDirectory   = [workingDirectory]"
    DebugPuts    2 "tcltest::temporaryDirectory = [temporaryDirectory]"
    DebugPuts    2 "tcltest::outputChannel      = [outputChannel]"
    DebugPuts    2 "tcltest::errorChannel       = [errorChannel]"
    DebugPuts    2 "Original environment (tcltest::originalEnv):"
    DebugPArray  2 originalEnv
    DebugPuts    2 "Constraints:"
    DebugPArray  2 testConstraints
}

#####################################################################

# Code to run the tests goes here.

# tcltest::TestPuts --
#
#⟦TAB⟧Used to redefine puts in test environment.  Stores whatever goes
#⟦TAB⟧out on stdout in tcltest::outData and stderr in errData before
#⟦TAB⟧seriable [namespace parent]::outData
    variable [namespace parent]::errData
    switch [llength $args] {
⟦TAB⟧1 {
⟦TAB⟧    # Only the string to be printed is specified
⟦TAB⟧    append outData [lindex $args 0]\n
⟦TAB⟧    return
⟦TAB⟧    # return [Puts [lindex $args 0]]
⟦TAB⟧}
⟦TAB⟧2 {
⟦TAB⟧    # Either -nonewline or channelId has been specified
⟦TAB⟧    if {[string equal -nonewline [lindex $args 0]]} {
⟦TAB⟧⟦TAB⟧append outData [lindex $args end]
⟦TAB⟧⟦TAB⟧return
⟦TAB⟧⟦TAB⟧# return [Puts -nonewline [lindex $args end]]
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧set channel [lindex $args 0]
⟦TAB⟧⟦TAB⟧set newline \n
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧3 {
⟦TAB⟧    if {[string equal -nonewline [lindex $args 0]]} {
⟦TAB⟧⟦TAB⟧# Both -nonewline and channelId are specified, unless
⟦TAB⟧⟦TAB⟧# it's an error.  -nonewline is supposed to be argv[0].
⟦TAB⟧⟦TAB⟧set channel [lindex $args 1]
⟦TAB⟧⟦TAB⟧set newline ""
⟦TAB⟧    }
⟦TAB⟧}
    }

    if {[info exists channel]} {
⟦TAB⟧if {[string equal $channel [[namespace parent]::outputChannel]]
⟦TAB⟧⟦TAB⟧|| [string equal $channel stdout]} {
⟦TAB⟧    append outData [lindex $args end]$newline
⟦TAB⟧    return
⟦TAB⟧} elseif {[string equal $channel [[namespace parent]::errorChannel]]
⟦TAB⟧⟦TAB⟧|| [string equal $channel stderr]} {
⟦TAB⟧    append errData [lindex $args end]$newline
⟦TAB⟧    return
⟦TAB⟧}
    }

    # If we haven't returned by now, we don't know how to handle the
    # input.  Let puts handle it.
    return [eval Puts $args]
}

# tcltest::Eval --
#
#⟦TAB⟧Evaluate the script in the test environment.  If ignoreOutput is
#       false, store data sent to stderr and stdout in outData and
#       errData.  Otherwise, ignore this output altogether.
#
# Arguments:
#⟦TAB⟧script             Script to evaluate
#       ?ignoreOutput?     Indicates whether or not to ignore output
#⟦TAB⟧⟦TAB⟧⟦TAB⟧   sent to stdout & stderr
#
# Results:
#⟦TAB⟧result from running the script
#
# Side effects:
#⟦TAB⟧Empties the contents of outData and errData before running a
#⟦TAB⟧test if ignoreOutput is set to 0.

proc tcltest::Eval {script {ignoreOutput 1}} {
    variable outData
    variable errData
    DebugPuts 3 "[lindex [info level 0] 0] called"
    if {!$ignoreOutput} {
⟦TAB⟧set outData {}
⟦TAB⟧s:
#     None.
#

proc tcltest::SubstArguments {argList} {

    # We need to split the argList up into tokens but cannot use list
    # operations as they throw away some significant quoting, and
    # [split] ignores braces as it should.  Therefore what we do is
    # gradually build up a string out of whitespace seperated strings.
    # We cannot use [split] to split the argList into whitespace
  # Look for the next word containing a quote: " { }
        if {[regexp -indices {[^ \t\n]*[\"\{\}]+[^ \t\n]*} \
⟦TAB⟧⟦TAB⟧$argList all]} {
            # Get the text leading up to this word, but not including
⟦TAB⟧    # this word, from the argList.
            set text [string range $argList 0 \
⟦TAB⟧⟦TAB⟧    [expr {[lindex $all 0] - 1}]]
            # Get the word with the quote
            set word [string range $argList \
                    [lindex $all 0] [lindex $all 1]]

            # Remove all text up to and including the word from the
            # argList.
            set argList [string range $argList \
                    [expr {[lindex $all 1] + 1}] end]
        } else {
            # Take everything up to the end of the argList.
            set text $argList
            set word {}
            set argList {}
        }

        if {$token != {}} {
            # If we saw a word with quote before, then there is a
            # multi-word token starting with that word.  In this case,
            # add the text and the current word to this token.
            append token $text $word
        } else {
            # Add the text to the result.  There is no need to parse
            # the text because it couldn't be a part of any multi-word
            # token.  Then start a new multi-word token with the word
            # because we need to pass this token to the Tcl parser to
            # check for balancing quotes
            append result $text
            set token $word
        }

        if { [catch {llength $token} length] == 0 && $length == 1} {
            # The token is a viable, if it matches an element in skip, or if one of the
# elements of "constraints" turns out not to be true.
#
# If testLevel is 1, then this is a top level test, and we record
# pass/fail information; otherwise, this information is not logged and
# is not added to running totals.
#
# Attributes:
#   Only description is a required attribute.  All others are optional.
#   Default values are indicated.
#
#   constraints -⟦TAB⟧A list of one or more keywords, each of which
#⟦TAB⟧⟦TAB⟧⟦TAB⟧must be the name of an element in the array
#⟦TAB⟧⟦TAB⟧⟦TAB⟧"testConstraints".  If any of these elements is
#⟦TAB⟧⟦TAB⟧⟦TAB⟧zero, the test is skipped. This attribute is
#⟦TAB⟧⟦TAB⟧⟦TAB⟧optional; default is {}
#   body -⟦TAB⟧        Script to run to carry out the test.  It must
#⟦TAB⟧⟦TAB⟧        return a result that can be checked for
#⟦TAB⟧⟦TAB⟧        correctness.  This attribute is optional;
#                       default is {}
#   result -⟦TAB⟧        Expected result from script.  This attribute is
#                       optional; default is {}.
#   output -            Expected output sent to stdout.  This attribute
#                       is optional; default is {}.
#   errorOutput -       Expected output sent to stderr.  This attribute
#                       is optional; default is {}.
#   returnCodes -       Expected return codes.  This attribute is
#                       optional; default is {0 2}.
#   setup -             Code to run before $script (above).  This
#                       attribute is optional; default is {}.
#   cleanup -           Code to run after $script (above).  This
#                       attribute is optional; default is {}.
#   match -             specifies type of matching to do on result,
#                       output, errorOutput; this must be a string
#⟦TAB⟧⟦TAB⟧⟦TAB⟧previously registered by a call to [customMatch].
#⟦TAB⟧⟦TAB⟧⟦TAB⟧The strings exact, glob, and regexp are pre-registered
#⟦TAB⟧⟦TAB⟧⟦TAB⟧by the tcltest package.  Default value is exact.
#
# Arguments:
#   name -⟦TAB⟧⟦TAB⟧Name of test, in the form foo-1.2.
#   description -⟦TAB⟧Short textual description of the test, to
#  ⟦TAB⟧⟦TAB⟧  ⟦TAB⟧help humans understand what it does.
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#       Just about anything is possible depending on the test.
#

proc tcltest::test {name description args} {
    global tcl_platform
    variable testLevel
    variable coreModTime
    DebugPuts 3 "test $name $args"
    DebugDo 1 {
⟦TAB⟧variable TestNames
⟦TAB⟧catch {
⟦TAB⟧    puts "test name '$name' re-used; prior use in $TestNames($name)"
⟦TAB⟧}
⟦TAB⟧set TestNames($name) [info script]
    }

    FillFilesExisted
    incr testLevel

    # Pre-define everything to null except output and errorOutput.  We
    # determine whether or not to trap output based on whether or not
    # these variables (output & errorOutput) are defined.
    foreach item {constraints setup cleanup body result returnCodes
⟦TAB⟧    match} {
⟦TAB⟧set $item {}
    }

    # Set the default match mode
    set match exact

    # Set the default match values for return codes (0 is the standard
    # expected return value if everything went well; 2 represents
    # 'return' being used in the test script).
    set returnCodes [list 0 2]

    # The old test format can't have a 3rd argument (constraints or
    # script) that starts with '-'.
    if {[string match -* [lindex $args 0]]
⟦TAB⟧    || ([llength $args] <= 1)} {
⟦TAB⟧if {[llength $args] == 1} {
⟦TAB⟧    set list [SubstArguments [lindex $args 0]]
⟦TAB⟧    foreach {element value} $list {
⟦TAB⟧⟦TAB⟧set testAttributes($element) $value
⟦TAB⟧    }
⟦TAB⟧    foreach item {constraints match setup body cleanup \
⟦TAB⟧⟦TAB⟧    result returnCodes output errorOutput} {
⟦TAB⟧⟦TAB⟧if {[info exists testAttributes(-$item)]} {
⟦TAB⟧⟦TAB⟧    set testAttributes(-$item) [uplevel 1 \
⟦TAB⟧⟦TAB⟧⟦TAB⟧    ::concat $testAttributes(-$item)]
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧} else {
⟦TAB⟧    array set testAttributes $args
⟦TAB⟧}

⟦TAB⟧set validFlags {-setup -cleanup -body -result -returnCodes \
⟦TAB⟧⟦TAB⟧-match -output -errorOutput -constraints}

⟦TAB⟧foreach flag [array names testAttributes] {
⟦TAB⟧    if {[lsearch -exact $validFlags $flag] == -1} {
⟦TAB⟧⟦TAB⟧incr testLevel -1
⟦TAB⟧⟦TAB⟧set sorted [lsort $validFlags]
⟦TAB⟧⟦TAB⟧set opf the test
# body to be captured for comparison against the expected values.

proc tcltest::RunTest {name script} {
    DebugPuts 3 "Running $name {$script}"

    # If there is no "memory" command (because memory debugging isn't
    # enabled), then don't attempt to use the command.

    if {[llength [info commands memory]] == 1} {
⟦TAB⟧memory tag $name
    }

    set code [catch {uplevel 1 $script} actualAnswer]

    return [list $actualAnswer $code]
}

#####################################################################

# tcltest::cleanupTestsHook --
#
#⟦TAB⟧This hook allows a harness that builds upon tcltest to specify
#       additional things that should be done at cleanup.
#

if {[llength [info commands tcltest::cleanupTestsHook]] == 0} {
    proc tcltest::cleanupTestsHook {} {}
}

# tcltevariable env).
#
# Arguments:
#      calledFromAllFile - if 0, behave as if we are running a single
#      test file within an entire suite of tests.  if we aren't running
#      a single test file, then don't report status.  check for new
#      files created during the test run and report on them.  if 1,
#      report collated status from all the test file runs.
#
# Results:
#      None.
#
# Side Effects:
#      None
#

proc tcltest::cleanupTests {{calledFromAllFile 0}} {
    variable filesMade
    variable filesExisted
    variable createdNewFiles
    variable testSingleFile
    variable numTests
    variable numTestFiles
    variable failFiles
    variable skippedBecause
    variable currentFailure
    variable originalEnv
    variable originalTclPlatform
    variable coreModTime

    FillFilesExisted
    set testFileName [file tail [info script]]

    # Call the cleanup hook
    cleanupTestsHook

    # Remove files and directories created by the makeFile and
    # makeDirectory procedures.  Record the names of files in
    # workingDirectory that were not pre-existing, and associate them
    # with the test file that created them.

    if {!$calledFromAllFile} {
⟦TAB⟧foreach file  }
⟦TAB⟧}

⟦TAB⟧# reset filesMade, filesExisted, and numTests

⟦TAB⟧set filesMade {}
⟦TAB⟧foreach index [list "Total" "Passed" "Skipped" "Failed"] {
⟦TAB⟧    set numTests($index) 0
⟦TAB⟧}

⟦TAB⟧# exit only if running Tk in non-interactive mode
⟦TAB⟧# This should be changed to determine if an event
⟦TAB⟧# loop is running, which is the real issue.
⟦TAB⟧# Actually, this doesn't belong here at all.  A package
⟦TAB⟧# really has no business [exit]-ing an application.
⟦TAB⟧if {![catch {package present Tk}] && ![testConstraint interactive]} {
⟦TAB⟧    exit
⟦TAB⟧}
    } else {

⟦TAB⟧# if we're deferring stat-reporting until all files are sourced,
⟦TAB⟧# then add current file to failFile list if any tests in this
⟦TAB⟧# file failed

⟦TAB⟧if {$currentFailure \
⟦TAB⟧⟦TAB⟧&& ([lsearch -exact $failFiles $testFileName] == -1)} {
⟦TAB⟧    lappend failFiles $testFileName
⟦TAB⟧}
⟦TAB⟧set currentFailure false

⟦TAB⟧# restore the environment to the state it was in before this package
⟦TAB⟧# was loaded

⟦TAB⟧set newEnv {}
⟦TAB⟧set changedEnv {}
⟦TAB⟧set removedEnv {}
⟦TAB⟧foreach index [array names ::env] {
⟦TAB⟧    if {![info exists originalEnv($index)]} {
⟦TAB⟧⟦TAB⟧lappend newEnv $index
⟦TAB⟧⟦TAB⟧unset ::env($index)
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧if {$::env($index) != $originalEnv($index)} {
⟦TAB⟧⟦TAB⟧    lappend changedEnv $index
⟦TAB⟧⟦TAB⟧    set ::env($index) $originalEnv($index)
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧foreach index [array names originalEnv] {
⟦TAB⟧    if {![info exists ::env($index)]} {
⟦TAB⟧⟦TAB⟧lappend removedEnv $index
⟦TAB⟧⟦TAB⟧set ::env($index) $originalEnv($index)
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧if {[llength $newEnv] > 0} {
⟦TAB⟧    puts [outputChannel] \
⟦TAB⟧⟦TAB⟧    "env array elements created:\t$newEnv"
⟦TAB⟧}
⟦TAB⟧if {[llength $changedEnv] > 0} {
⟦TAB⟧    puts [outputChannel] \
⟦TAB⟧⟦TAB⟧    "env array elements changed:\t$changedEnv"
⟦TAB⟧}
⟦TAB⟧if {[llength $removedEnv] > 0} {
⟦TAB⟧    puts [outputChannel] \
⟦TAB⟧⟦TAB⟧    "env array elements removed:\t$removedEnv"
⟦TAB⟧}

⟦TAB⟧set changedTclPlatform {}
⟦TAB⟧foreach index [array names originalTclPlatform] {
⟦TAB⟧    if {$::tcl_platform($index) \
⟦TAB⟧⟦TAB⟧    != $originalTclPlatform($index)} {
⟦TAB⟧⟦TAB⟧lappend changedTclPlatform $index
⟦TAB⟧⟦TAB⟧set ::tcl_platform($index) $originalTclPlatform($index)
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧if {[llenme != [file mtime \
⟦TAB⟧⟦TAB⟧⟦TAB⟧    [file join [workingDirectory] core]]} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧puts [outputChannel] "A core file was created!"
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧} else {
⟦TAB⟧⟦TAB⟧    puts [outputChannel] "A core file was created!"
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
    }
    flush [outputChannel]
    flush [errorChannel]
    return
}

#####################################################################

# Procs that determine which tests/test files to run

# tcltest::GetMatchingFiles
#
#       Looks at the patterns given to match and skip files and uses
#⟦TAB⟧them to put together a list of the tests that will be run.
#
# Arguments:
#       directory to search
#
# Results:
#       The constructed list is returned to the user.  This will
#⟦TAB⟧primarily be used in 'all.tcl' files.  It is used in
#⟦TAB⟧runAllTests.
#
# Side Effects:
#       None

# a lower case version is needed for compatibility with tcltest 1.0
proc tcltest::getMatchingFiles args {eval GetMatchingFiles $args}

proc tcltest::GetMatchingFiles { args } {
    if {[llength $args]} {
⟦TAB⟧set dirList $args
    } else {
⟦TAB⟧# Finding tests only in [testsDirectory] is normal operation.
⟦TAB⟧# This procedure is written to accept multiple directory arguments
⟦TAB⟧# only to satisfy version 1 compatibility.
⟦TAB⟧set dirList [list [testsDirectory]]
    }

    set matchingFiles [list]
    foreach directory $dirList {

⟦TAB⟧# List files in $directory that match patterns to run.
⟦TAB⟧set matchFileList [list]
⟦TAB⟧foreach match [matchFiles] {
⟦TAB⟧    set matchFileList [concat $matchFileList \
⟦TAB⟧⟦TAB⟧    [glob -directory $directory -types {b c f p s} \
⟦TAB⟧⟦TAB⟧    -nocomplain -- $match]]
⟦TAB⟧}

⟦TAB⟧# List files in $directory that match patterns to skip.
⟦TAB⟧set skipFileList [list]
⟦TAB⟧foreach skip [skipFiles] {
⟦TAB⟧    set skipFileList [concat $skipFileList \
⟦TAB⟧⟦TAB⟧    [glob -directory $directory -types {b c f p s} \
⟦TAB⟧⟦TAB⟧    -nocomplain -- $skip]]
⟦TAB⟧}

⟦TAB⟧# Add to result list all files in match list and not in skip list
⟦TAB⟧foreach file $matchFileList {
⟦TAB⟧    if {[lsearch -exact $skipFileList $file] == -1} {
⟦TAB⟧⟦TAB⟧lappend matchingFiles $file
⟦TAB⟧    }
⟦TAB⟧}
    }

    if {[llength $matchingFiles] == 0} {
⟦TAB⟧PrintError "No test files remain after applying your match and\
⟦TAB⟧⟦TAB⟧skip patterns!"
    }
    return $matchingFiles
}

# tcltest::GetMatchingDirectories --
#
#⟦TAB⟧Looks at the patterns given to match and skip directories and
#⟦TAB⟧uses them to put together a list of the test directories that we
#⟦TAB⟧should attempt to run.  (Only subdirectories containing an
#⟦TAB⟧"all.tcl" file are put into the list.)
#
# Arguments:
#⟦TAB⟧root directory from which to search
#
# Results:
#⟦TAB⟧The constructed list is returned to the user.  This is used in
#⟦TAB⟧the primary all.tcl file.
#
# Side Effects:
#       None.

proc tcltest::GetMatchingDirectories {rootdir} {

    # Determine the skip list first, to avoid [glob]-ing over subdirectories
    # we're going to throw away anyway.  Be sure we skip the $rootdir if it
    # comes up to avoid infinite loops.
    set skipDirs [list $rootdir]
    foreach pattern [skipDirectories] {
⟦TAB⟧set skipDirs [concat $skipDirs [glob -directory $rootdir -types d \
⟦TAB⟧⟦TAB⟧-nocomplain -- $pattern]]
    }

    # Now step through the matching directories, prune out the skipped ones
    # as you go.
    set matchDirs [list]
    foreach pattern [matchDirectories] {
⟦TAB⟧foreach path [glob -directory $rootdir -types d -nocomplain -- \
⟦TAB⟧⟦TAB⟧$pattern] {
⟦TAB⟧    if {[lsearch -exact $skipDirs $path] == -1} {
⟦TAB⟧⟦TAB⟧set matchDirs [concat $matchDirs [GetMatchingDirectories $path]]
⟦TAB⟧⟦TAB⟧if {[file exists [file join $path all.tcl]]} {
⟦TAB⟧⟦TAB⟧    lappend matchDirs $path
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
    }

    if {[llength $matchDirs] == 0} {
⟦TAB⟧DebugPuts 1 "No test directories remain after applying match\
⟦TAB⟧⟦TAB⟧and skip patterns!"
    }
    return $matchDirs
}

# tcltest::runAllTests --
#
#⟦TAB⟧prints output and sources test files according to the match and
#⟦TAB⟧skip patterns provided.  after sourcing test files, it goes on
#⟦TAB⟧to source all.tcl files in matching test subdirectories.
#
# Arguments:
#⟦TAB⟧shell being tested
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::runAllTests { {shell ""} } {
    varring equal native [lindex $result 0]]} {
⟦TAB⟧# If we aren't running in the native filesystem, then we must
⟦TAB⟧# run the tests in a single process (via 'source'), because
⟦TAB⟧# trying to run then via a pipe will fail since the files don't
⟦TAB⟧# really exist.
⟦TAB⟧singleProcess 1
    }

    if {[singleProcess]} {
⟦TAB⟧puts [outputChannel] \
⟦TAB⟧⟦TAB⟧"Test files sourced into current interpreter"
    } else {
⟦TAB⟧puts [outputChannel] \
⟦TAB⟧⟦TAB⟧"Test files run in separate interpreters"
    }
    if {[llength [skip]] > 0} {
⟦TAB⟧puts [outputChannel] "Skipping tests that match:  [skip]"
    }
    puts [outputChannel] "Running tests that match:  [match]"

    if {[llength [skipFiles]] > 0} {
⟦TAB⟧puts [outputChannel] \
⟦TAB⟧⟦TAB⟧"Skipping test files that match:  [skipFiles]"
    }
    if {[llength [matchFiles]] > 0} {
⟦TAB⟧puts [outputChannel] \
⟦TAB⟧⟦TAB⟧"Only running test files that match:  [matchFiles]"
    }

    set timeCmd {clock format [clock seconds]}
    puts [outputChannel] "Tests began at [eval $timeCmd]"

    # Run each of the specified tests
    foreach file [lsort [GetMatchingFiles]] {
⟦TAB⟧set tail [file tail $file]
⟦TAB⟧puts [outputChannel] $tail
⟦TAB⟧flush [outputChannel]

⟦TAB⟧if {[singleProcess]} {
⟦TAB⟧    incr numTestFiles
⟦TAB⟧    uplevel 1 [list ::source $file]
⟦TAB⟧} else {
⟦TAB⟧    # Pass along our configuration to the child processes.
⟦TAB⟧    # EXCEPT for the -outfile, because the parent process
⟦TAB⟧    # needs to read and process output of children.
⟦TAB⟧    set childargv [list]
⟦TAB⟧    foreach opt [Configure] {
⟦TAB⟧⟦TAB⟧if {[string equal $opt -outfile]} {continue}
⟦TAB⟧⟦TAB⟧lappend childargv $opt [Configure $opt]
⟦TAB⟧    }
⟦TAB⟧    set cmd [linsert $childargv 0 | $shell $file]
⟦TAB⟧    if {[catch {
⟦TAB⟧⟦TAB⟧incr numTestFiles
⟦TAB⟧⟦TAB⟧set pipeFd [open $cmd "r"]
⟦TAB⟧⟦TAB⟧while {[gets $pipeFd line] >= 0} {
⟦TAB⟧⟦TAB⟧    if {[regexp [join {
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {^([^:]+):\t}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {Total\t([0-9]+)\t}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {Passed\t([0-9]+)\t}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {Skipped\t([0-9]+)\t}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {Failed\t([0-9]+)}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    } ""] $line null testFile \
⟦TAB⟧⟦TAB⟧⟦TAB⟧    Total Passed Skipped Failed]} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧foreach index {Total Passed Skipped Failed} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧    incr numTestsxp⟦TAB⟧[list regexp --]

    # If the TCLTEST_OPTIONS environment variable exists, configure
    # tcltest according to the option values it specifies.  This has
    # the effect of resetting tcltest's default configuration.
    proc ConfigureFromEnvironment {} {
⟦TAB⟧upvar #0 env(TCLTEST_OPTIONS) options
⟦TAB⟧if {[catch {llength $options} msg]} {
⟦TAB⟧    Warn "invalid TCLTEST_OPTIONS \"$options\":\n  invalid\
⟦TAB⟧⟦TAB⟧    Tcl list: $msg"
⟦TAB⟧    return
⟦TAB⟧}
⟦TAB⟧if {[llength $::env(TCLTEST_OPTIONS)] % 2} {
⟦TAB⟧    Warn "invalid TCLTEST_OPTIONS: \"$options\":\n  should be\
⟦TAB⟧⟦TAB⟧    -option value ?-option value ...?"
⟦TAB⟧    return
⟦TAB⟧ so give it (and exit)
⟦TAB⟧    # right now.  ([configure] does not process -help)
⟦TAB⟧    set required true
⟦TAB⟧}
⟦TAB⟧foreach hook { PrintUsageInfoHook processCmdLineArgsHook
⟦TAB⟧⟦TAB⟧⟦TAB⟧processCmdLineArgsAddFlagsHook } {
⟦TAB⟧    if {[string equal [namespace current] [namespace qualifiers \
⟦TAB⟧⟦TAB⟧    [namespace which $hook]]]} {
⟦TAB⟧⟦TAB⟧set required true
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧proc $hook args {}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $required
    }

    # Only initialize configurable options from the command line arguments
    # at package load time if necessary for backward compatibility.  This
    # lets the tcltest user call [configure] for themselves if they wish.
    # Traces are established for auto-configuration from the command line
    # if any configurable options are accessed before the user calls
    # [configure].
    if {[LoadTimeCmdLineArgParsingRequired]} {
⟦TAB⟧ProcessCmdLineArgs
    } else {
⟦TAB⟧EstablishAutoConfigureTraces
    }

    package provide [namespace tail [namespace current]] $Version
}
```

- real-harness-explicit-source-child, lookup, completion 0, result:

```text-visible
child-provider {8.4.6 8.4 ::} module-code 1 module-result {invalid command name "pwd"} require-code 1 require-result {can't find package tcltest}⟦EOF⟧
```

- real-harness-explicit-source-child, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- real-harness-explicit-source-child: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 10

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
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

    # tests that use threads need to know which is the main thread
    Default mainThread 1
    variable mainThread
    if {[info commands thread::id] != {}} {
⟦TAB⟧set mainThread [thread::id]
    } elseif {[info commands testthread] != {}} {
⟦TAB⟧set mainThread [testthread id]
    }

    # Set workingDirectory to [pwd]. The default output directory for
    # Tcl tests is the working directory.  Whenever this value changes
    # change to that directory.
    variable workingDirectory
    trace variable workingDirectory w \
⟦TAB⟧    [namespace code {cd $workingDirectory ;#}]

    Default workingDirectory [pwd] AcceptAbsolutePath
    proc workingDirectory { {dir ""} } {
⟦TAB⟧variable workingDirectory
⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $workingDirectory
⟦TAB⟧}
⟦TAB⟧set workingDirectory [AcceptAbsolutePath $dir]
    }

    # Set the location of the execuatble
    Default tcltest [info nameofexecutable]
    trace variable tcltest w [namespace code {testConstraint stdio \
⟦TAB⟧    [eval [ConstraintInitializer stdio]] ;#}]

    # save the platform information so it can be restored later
    Default originalTclPlatform [array get ::tcl_platform]

    # If a core file exists, save its modification time.
    if {[file exists [file join [workingDirectory] core]]} {
⟦TAB⟧Default coreModTime \
⟦TAB⟧⟦TAB⟧[file mtime [file join [workingDirectory] core]]
    }

    # stdout and stderr buffers for use when we want to store them
    Default outData {}
    Default errData {}

    # keep track of test level for nested test commands
    variable testLevel 0

    # the variables and procs that existed when saveState was called are
    # stored in a variable of the same name
    Default saveState {}

    # Internationalization support -- used in [SetIso8859_1_Locale] and
    # [RestoreLocale]. Those commands are used in cmdIL.test.

    if {![info exists [namespace current]::isoLocale]} {
⟦TAB⟧variable isoLocale fr
⟦TAB⟧switch -- $::tcl_platform(platform) {
⟦TAB⟧    "unix" {

⟦TAB⟧⟦TAB⟧# Try some 'known' values for some platforms:

⟦TAB⟧⟦TAB⟧switch -exact -- $::tcl_platform(os) {
⟦TAB⟧⟦TAB⟧    "FreeBSD" {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr_FR.ISO_8859-1
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    HP-UX {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr_FR.iso88591
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    Linux -
⟦TAB⟧⟦TAB⟧    IRIX {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    default {

⟦TAB⟧⟦TAB⟧⟦TAB⟧# Works on SunOS 4 and Solaris, and maybe
⟦TAB⟧⟦TAB⟧⟦TAB⟧# others...  Define it to something else on your
⟦TAB⟧⟦TAB⟧⟦TAB⟧# system if you want to test those.

⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale iso_8859_1
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧    "windows" {
⟦TAB⟧⟦TAB⟧set isoLocale French
⟦TAB⟧    }
⟦TAB⟧}
    }

    variable ChannelsWeOpened; array set ChannelsWeOpened {}
    # output goes to stdout by default
    Default outputChannel stdout
    proc outputChannel { {filename ""} } {
⟦TAB⟧variable outputChannel
⟦TAB⟧variable ChannelsWeOpened

⟦TAB⟧# This is very subtle and tricky, so let me try to explain.
⟦TAB⟧# (Hopefully this longer comment will be clear when I come
⟦TAB⟧# back in a few months, unlike its predecessor :) )
⟦TAB⟧#⟦SPACE⟧
⟦TAB⟧# The [outputChannel] command (and underlying variable) have to
⟦TAB⟧# be kept in sync with the [configure -outfile] configuration
⟦TAB⟧# option ( and underlying variable Option(-outfile) ).  This is
⟦TAB⟧# accomplished with a write trace on Option(-outfile) that will
⟦TAB⟧# update [outputChannel] whenver a new value is written.  That
⟦TAB⟧# much is easy.
⟦TAB⟧#
⟦TAB⟧# The trick is that in order to maintain compatibility with
⟦TAB⟧# version 1 of tcltest, we must allow every configuration option
⟦TAB⟧# to get its inital value from command line arguments.  This is
⟦TAB⟧# accomplished by setting initial read traces on all the
⟦TAB⟧# configuration options to parse the command line option the first
⟦TAB⟧# time they are read.  These traces are cancelled whenever the
⟦TAB⟧# program itself calls [configure].
⟦TAB⟧#⟦SPACE⟧
⟦TAB⟧# OK, then so to support tcltest 1 compatibility, it seems we want
⟦TAB⟧# to get the return from [outputFile] to trigger the read traces,
⟦TAB⟧# just in case.
⟦TAB⟧#
⟦TAB⟧# BUT!  A little known feature of Tcl variable traces is that⟦SPACE⟧
⟦TAB⟧# traces are disabled during the handling of other traces.  So,
⟦TAB⟧# if we trigger read traces on Option(-outfile) and that triggers
⟦TAB⟧# command line parsing which turns around and sets an initial
⟦TAB⟧# value for Option(-outfile) -- <whew!> -- the write trace that
⟦TAB⟧# would keep [outputChannel] in sync with that new initial value
⟦TAB⟧# would not fire!
⟦TAB⟧#
⟦TAB⟧# SO, finally, as a workaround, instead of triggering read traces
⟦TAB⟧# by invoking [outputFile], we instead trigger the same set of
⟦TAB⟧# read traces by invoking [debug].  Any command that reads a
⟦TAB⟧# configuration option would do.  [debug] is just a handy one.
⟦TAB⟧# The end result is that we support tcltest 1 compatibility and
⟦TAB⟧# keep outputChannel and -outfile in sync in all cases.
⟦TAB⟧debug

⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $outputChannel
⟦TAB⟧}
⟦TAB⟧if {[info exists ChannelsWeOpened($outputChannel)]} {
⟦TAB⟧    close $outputChannel
⟦TAB⟧    unset ChannelsWeOpened($outputChannel)
⟦TAB⟧}
⟦TAB⟧switch -exact -- $filename {
⟦TAB⟧    stderr -
⟦TAB⟧    stdout {
⟦TAB⟧⟦TAB⟧set outputChannel $filename
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧set outputChannel [open $filename a]
⟦TAB⟧⟦TAB⟧set ChannelsWeOpened($outputChannel) 1

⟦TAB⟧⟦TAB⟧# If we created the file in [temporaryDirectory], then
⟦TAB⟧⟦TAB⟧# [cleanupTests] will delete it, unless we claim it was
⟦TAB⟧⟦TAB⟧# already there.
⟦TAB⟧⟦TAB⟧set outdir [normalizePath [file dirname \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [pwd] $filename]]]
⟦TAB⟧⟦TAB⟧if {[string equal $outdir [temporaryDirectory]]} {
⟦TAB⟧⟦TAB⟧    variable filesExisted
⟦TAB⟧⟦TAB⟧    FillFilesExisted
⟦TAB⟧⟦TAB⟧    set filename [file tail $filename]
⟦TAB⟧⟦TAB⟧    if {[lsearch -exact $filesExisted $filename] == -1} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧lappend filesExisted $filename
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $outputChannel
    }

    # errors go to stderr by default
    Default errorChannel stderr
    proc errorChannel { {filename ""} } {
⟦TAB⟧variable errorChannel
⟦TAB⟧variable ChannelsWeOpened

⟦TAB⟧# This is subtle and tricky.  See the comment above in
⟦TAB⟧# [outputChannel] for a detailed explanation.
⟦TAB⟧debug

⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $errorChannel
⟦TAB⟧}
⟦TAB⟧if {[info exists ChannelsWeOpened($errorChannel)]} {
⟦TAB⟧    close $errorChannel
⟦TAB⟧    unset ChannelsWeOpened($errorChannel)
⟦TAB⟧}
⟦TAB⟧switch -exact -- $filename {
⟦TAB⟧    stderr -
⟦TAB⟧    stdout {
⟦TAB⟧⟦TAB⟧set errorChannel $filename
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧set errorChannel [open $filename a]
⟦TAB⟧⟦TAB⟧set ChannelsWeOpened($errorChannel) 1

⟦TAB⟧⟦TAB⟧# If we created the file in [temporaryDirectory], then
⟦TAB⟧⟦TAB⟧# [cleanupTests] will delete it, unless we claim it was
⟦TAB⟧⟦TAB⟧# already there.
⟦TAB⟧⟦TAB⟧set outdir [normalizePath [file dirname \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [pwd] $filename]]]
⟦TAB⟧⟦TAB⟧if {[string equal $outdir [temporaryDirectory]]} {
⟦TAB⟧⟦TAB⟧    variable filesExisted
⟦TAB⟧⟦TAB⟧    FillFilesExisted
⟦TAB⟧⟦TAB⟧    set filename [file tail $filename]
⟦TAB⟧⟦TAB⟧    if {[lsearch -exact $filesExisted $filename] == -1} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧lappend filesExisted $filename
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $errorChannel
    }

##### Set up the configurable options
    #
    # The configurable options of the package
    variable Option; array set Option {}

    # Usage strings for those options
    variable Usage; array set Usage {}

    # Verification commands for those options
    variable Verify; array set Verify {}

    # Initialize the default values of the configurable options that are
    # historically associated with an exported variable.  If that variable
    # is already set, support compatibility by accepting its pre-set value.
    # Use [trace] to establish ongoing connection between the deprecated
    # exported variable and the modern option kept as a true internal var.
    # Also set up usage string and value testing for the option.
    proc Option {option value usage {verify AcceptAll} {varName {}}} {
⟦TAB⟧variable Option
⟦TAB⟧variable Verify
⟦TAB⟧variable Usage
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧set Usage($option) $usage
⟦TAB⟧set Verify($option) $verify
⟦TAB⟧if {[catch {$verify $value} msg]} {
⟦TAB⟧    return -code error $msg
⟦TAB⟧} else {
⟦TAB⟧    set Option($option) $msg
⟦TAB⟧}
⟦TAB⟧if {[string length $varName]} {
⟦TAB⟧    variable $varName
⟦TAB⟧    if {[info exists $varName]} {
⟦TAB⟧⟦TAB⟧if {[catch {$verify [set $varName]} msg]} {
⟦TAB⟧⟦TAB⟧    return -code error $msg
⟦TAB⟧⟦TAB⟧} else {
⟦TAB⟧⟦TAB⟧    set Option($option) $msg
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧unset $varName
⟦TAB⟧    }
⟦TAB⟧    namespace eval [namespace current] \
⟦TAB⟧    ⟦TAB⟧    [list upvar 0 Option($option) $varName]
⟦TAB⟧    # Workaround for Bug (now Feature Request) 572889.  Grrrr....
⟦TAB⟧    # Track all the variables tied to options
⟦TAB⟧    lappend OptionControlledVariables $varName
⟦TAB⟧    # Later, set auto-configure read traces on all
⟦TAB⟧    # of them, since a single trace on Option does not work.
⟦TAB⟧    proc $varName {{value {}}} [subst -nocommands {
⟦TAB⟧⟦TAB⟧if {[llength [info level 0]] == 2} {
⟦TAB⟧⟦TAB⟧    Configure $option [set value]
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧return [Configure $option]
⟦TAB⟧    }]
⟦TAB⟧}
    }

    proc MatchingOption {option} {
⟦TAB⟧variable Option
⟦TAB⟧set match [array names Option $option*]
⟦TAB⟧switch -- [llength $match] {
⟦TAB⟧    0 {
⟦TAB⟧⟦TAB⟧set sorted [lsort [array names Option]]
⟦TAB⟧⟦TAB⟧set values [join [lrange $sorted 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append values ", or [lindex $sorted end]"
⟦TAB⟧⟦TAB⟧return -code error "unknown option $option: should be\
⟦TAB⟧⟦TAB⟧⟦TAB⟧one of $values"
⟦TAB⟧    }
⟦TAB⟧    1 {
⟦TAB⟧⟦TAB⟧return [lindex $match 0]
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧# Exact match trumps ambiguity
⟦TAB⟧⟦TAB⟧if {[lsearch -exact $match $option] >= 0} {
⟦TAB⟧⟦TAB⟧    return $option
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧set values [join [lrange $match 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append values ", or [lindex $match end]"
⟦TAB⟧⟦TAB⟧return -code error "ambiguous option $option:\
⟦TAB⟧⟦TAB⟧⟦TAB⟧could match $values"
⟦TAB⟧    }
⟦TAB⟧}
    }

    proc EstablishAutoConfigureTraces {} {
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧foreach varName [concat $OptionControlledVariables Option] {
⟦TAB⟧    variable $varName
⟦TAB⟧    trace variable $varName r [namespace code {ProcessCmdLineArgs ;#}]
⟦TAB⟧}
    }

    proc RemoveAutoConfigureTraces {} {
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧foreach varName [concat $OptionControlledVariables Option] {
⟦TAB⟧    variable $varName
⟦TAB⟧    foreach pair [trace vinfo $varName] {
⟦TAB⟧⟦TAB⟧foreach {op cmd} $pair break
⟦TAB⟧⟦TAB⟧if {[string equal r $op]
⟦TAB⟧⟦TAB⟧⟦TAB⟧&& [string match *ProcessCmdLineArgs* $cmd]} {
⟦TAB⟧⟦TAB⟧    trace vdelete $varName $op $cmd
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧# Once the traces are removed, this can become a no-op
⟦TAB⟧proc RemoveAutoConfigureTraces {} {}
    }

    proc Configure args {
⟦TAB⟧variable Option
⟦TAB⟧variable Verify
⟦TAB⟧set n [llength $args]
⟦TAB⟧if {$n == 0} {
⟦TAB⟧    return [lsort [array names Option]]
⟦TAB⟧}
⟦TAB⟧if {$n == 1} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    return $Option($option)
⟦TAB⟧}
⟦TAB⟧while {[llength $args] > 1} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    if {[catch {$Verify($option) [lindex $args 1]} value]} {
⟦TAB⟧⟦TAB⟧return -code error "invalid $option\
⟦TAB⟧⟦TAB⟧⟦TAB⟧value \"[lindex $args 1]\": $value"
⟦TAB⟧    }
⟦TAB⟧    set Option($option) $value
⟦TAB⟧    set args [lrange $args 2 end]
⟦TAB⟧}
⟦TAB⟧if {[llength $args]} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    return -code error "missing value for option $option"
⟦TAB⟧}
    }
    proc configure args {
⟦TAB⟧RemoveAutoConfigureTraces
⟦TAB⟧set code [catch {eval Configure $args} msg]
⟦TAB⟧return -code $code $msg
    }
⟦SPACE⟧⟦SPACE⟧⟦SPACE⟧⟦SPACE⟧
    proc AcceptVerbose { level } {
⟦TAB⟧set level [AcceptList $level]
⟦TAB⟧if {[llength $level] == 1} {
⟦TAB⟧    if {![regexp {^(pass|body|skip|start|error)$} $level]} {
⟦TAB⟧⟦TAB⟧# translate single characters abbreviations to expanded list
⟦TAB⟧⟦TAB⟧set level [string map {p pass b body s skip t start e error} \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[split $level {}]]
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set valid [list]
⟦TAB⟧foreach v $level {
⟦TAB⟧    if {[regexp {^(pass|body|skip|start|error)$} $v]} {
⟦TAB⟧⟦TAB⟧lappend valid $v
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $valid
    }

    proc IsVerbose {level} {
⟦TAB⟧variable Option
⟦TAB⟧return [expr {[lsearch -exact $Option(-verbose) $level] != -1}]
    }

    # Default verbosity is to show bodies of failed tests
    Option -verbose {body error} {
⟦TAB⟧Takes any combination of the values 'p', 's', 'b', 't' and 'e'.
⟦TAB⟧Test suite will display all passed tests if 'p' is specified, all
⟦TAB⟧skipped tests if 's' is specified, the bodies of failed tests if
⟦TAB⟧'b' is specified, and when tests start if 't' is specified.
⟦TAB⟧ErrorInfo is displayed if 'e' is specified.
    } AcceptVerbose verbose

    # Match and skip patterns default to the empty list, except for
    # matchFiles, which defaults to all .test files in the
    # testsDirectory and matchDirectories, which defaults to all
    # directories.
    Option -match * {
⟦TAB⟧Run all tests within the specified files that match one of the
⟦TAB⟧list of glob patterns given.
    } AcceptList match

    Option -skip {} {
⟦TAB⟧Skip all tests within the specified tests (via -match) and files
⟦TAB⟧that match one of the list of glob patterns given.
    } AcceptList skip

    Option -file *.test {
⟦TAB⟧Run tests in all test files that match the glob pattern given.
    } AcceptPattern matchFiles

    # By default, skip files that appear to be SCCS lock files.
    Option -notfile l.*.test {
⟦TAB⟧Skip all test files that match the glob pattern given.
    } AcceptPattern skipFiles

    Option -relateddir * {
⟦TAB⟧Run tests in directories that match the glob pattern given.
    } AcceptPattern matchDirectories

    Option -asidefromdir {} {
⟦TAB⟧Skip tests in directories that match the glob pattern given.
    } AcceptPattern skipDirectories

    # By default, don't save core files
    Option -preservecore 0 {
⟦TAB⟧If 2, save any core files produced during testing in the directory
⟦TAB⟧specified by -tmpdir. If 1, notify the user if core files are
⟦TAB⟧created.
    } AcceptInteger preserveCore

    # debug output doesn't get printed by default; debug level 1 spits
    # up only the tests that were skipped because they didn't match or
    # were specifically skipped.  A debug level of 2 would spit up the
    # tcltest variables and flags provided; a debug level of 3 causes
    # some additional output regarding operations of the test harness.
    # The tcltest package currently implements only up to debug level 3.
    Option -debug 0 {
⟦TAB⟧Internal debug level⟦SPACE⟧
    } AcceptInteger debug

    proc SetSelectedConstraints args {
⟦TAB⟧variable Option
⟦TAB⟧foreach c $Option(-constraints) {
⟦TAB⟧    testConstraint $c 1
⟦TAB⟧}
    }
    Option -constraints {} {
⟦TAB⟧Do not skip the listed constraints listed in -constraints.
    } AcceptList
    trace variable Option(-constraints) w \
⟦TAB⟧    [namespace code {SetSelectedConstraints ;#}]

    # Don't run only the "-constraint" specified tests by default
    proc ClearUnselectedConstraints args {
⟦TAB⟧variable Option
⟦TAB⟧variable testConstraints
⟦TAB⟧if {!$Option(-limitconstraints)} {return}
⟦TAB⟧foreach c [array names testConstraints] {
⟦TAB⟧    if {[lsearch -exact $Option(-constraints) $c] == -1} {
⟦TAB⟧⟦TAB⟧testConstraint $c 0
⟦TAB⟧    }
⟦TAB⟧}
    }
    Option -limitconstraints false {
⟦TAB⟧whether to run only tests with the constraints
    } AcceptBoolean limitConstraints⟦SPACE⟧
    trace variable Option(-limitconstraints) w \
⟦TAB⟧    [namespace code {ClearUnselectedConstraints ;#}]

    # A test application has to know how to load the tested commands
    # into the interpreter.
    Option -load {} {
⟦TAB⟧Specifies the script to load the tested commands.
    } AcceptScript loadScript

    # Default is to run each test file in a separate process
    Option -singleproc 0 {
⟦TAB⟧whether to run all tests in one process
    } AcceptBoolean singleProcess⟦SPACE⟧

    proc AcceptTemporaryDirectory { directory } {
⟦TAB⟧set directory [AcceptAbsolutePath $directory]
⟦TAB⟧if {![file exists $directory]} {
⟦TAB⟧    file mkdir $directory
⟦TAB⟧}
⟦TAB⟧set directory [AcceptDirectory $directory]
⟦TAB⟧if {![file writable $directory]} {
⟦TAB⟧    if {[string equal [workingDirectory] $directory]} {
⟦TAB⟧⟦TAB⟧# Special exception: accept the default value
⟦TAB⟧⟦TAB⟧# even if the directory is not writable
⟦TAB⟧⟦TAB⟧return $directory
⟦TAB⟧    }
⟦TAB⟧    return -code error "\"$directory\" is not writeable"
⟦TAB⟧}
⟦TAB⟧return $directory
    }

    # Directory where files should be created
    Option -tmpdir [workingDirectory] {
⟦TAB⟧Save temporary files in the specified directory.
    } AcceptTemporaryDirectory temporaryDirectory
    trace variable Option(-tmpdir) w \
⟦TAB⟧    [namespace code {normalizePath Option(-tmpdir) ;#}]

    # Tests should not rely on the current working directory.
    # Files that are part of the test suite should be accessed relative
    # to [testsDirectory]
    Option -testdir [workingDirectory] {
⟦TAB⟧Search tests in the specified directory.
    } AcceptDirectory testsDirectory
    trace variable Option(-testdir) w \
⟦TAB⟧    [namespace code {normalizePath Option(-testdir) ;#}]

    proc AcceptLoadFile { file } {
⟦TAB⟧if {[string equal "" $file]} {return $file}
⟦TAB⟧set file [file join [temporaryDirectory] $file]
⟦TAB⟧return [AcceptReadable $file]
    }
    proc ReadLoadScript {args} {
⟦TAB⟧variable Option
⟦TAB⟧if {[string equal "" $Option(-loadfile)]} {return}
⟦TAB⟧set tmp [open $Option(-loadfile) r]
⟦TAB⟧loadScript [read $tmp]
⟦TAB⟧close $tmp
    }
    Option -loadfile {} {
⟦TAB⟧Read the script to load the tested commands from the specified file.
    } AcceptLoadFile loadFile
    trace variable Option(-loadfile) w [namespace code ReadLoadScript]

    proc AcceptOutFile { file } {
⟦TAB⟧if {[string equal stderr $file]} {return $file}
⟦TAB⟧if {[string equal stdout $file]} {return $file}
⟦TAB⟧return [file join [temporaryDirectory] $file]
    }

    # output goes to stdout by default
    Option -outfile stdout {
⟦TAB⟧Send output from test runs to the specified file.
    } AcceptOutFile outputFile
    trace variable Option(-outfile) w \
⟦TAB⟧    [namespace code {outputChannel $Option(-outfile) ;#}]

    # errors go to stderr by default
    Option -errfile stderr {
⟦TAB⟧Send errors from test runs to the specified file.
    } AcceptOutFile errorFile
    trace variable Option(-errfile) w \
⟦TAB⟧    [namespace code {errorChannel $Option(-errfile) ;#}]

}

#####################################################################

# tcltest::Debug* --
#
#     Internal helper procedures to write out debug information
#     dependent on the chosen level. A test shell may overide
#     them, f.e. to redirect the output into a different
#     channel, or even into a GUI.

# tcltest::DebugPuts --
#
#     Prints the specified string if the current debug level is
#     higher than the provided level argument.
#
# Arguments:
#     level   The lowest debug level triggering the output
#     string  The string to print out.
#
# Results:
#     Prints the string. Nothing else is allowed.
#
# Side Effects:
#     None.
#

proc tcltest::DebugPuts {level string} {
    variable debug
    if {$debug >= $level} {
⟦TAB⟧puts $string
    }
    return
}

# tcltest::DebugPArray --
#
#     Prints the contents of the specified array if the current
#       debug level is higher than the provided level argument
#
# Arguments:
#     level           The lowest debug level triggering the output
#     arrayvar        The name of the array to print out.
#
# Results:
#     Prints the contents of the array. Nothing else is allowed.
#
# Side Effects:
#     None.
#

proc tcltest::DebugPArray {level arrayvar} {
    variable debug

    if {$debug >= $level} {
⟦TAB⟧catch {upvar  $arrayvar $arrayvar}
⟦TAB⟧parray $arrayvar
    }
    return
}

# Define our own [parray] in ::tcltest that will inherit use of the [puts]
# defined in ::tcltest.  NOTE: Ought to construct with [info args] and
# [info default], but can't be bothered now.  If [parray] changes, then
# this will need changing too.
auto_load ::parray
proc tcltest::parray {a {pattern *}} [info body ::parray]

# tcltest::DebugDo --
#
#     Executes the script if the current debug level is greater than
#       the provided level argument
#
# Arguments:
#     level   The lowest debug level triggering the execution.
#     script  The tcl script executed upon a debug level high enough.
#
# Results:
#     Arbitrary side effects, dependent on the executed script.
#
# Side Effects:
#     None.
#

proc tcltest::DebugDo {level script} {
    variable debug

    if {$debug >= $level} {
⟦TAB⟧uplevel 1 $script
    }
    return
}

#####################################################################

proc tcltest::Warn {msg} {
    puts [outputChannel] "WARNING: $msg"
}

# tcltest::mainThread
#
#     Accessor command for tcltest variable mainThread.
#
proc tcltest::mainThread { {new ""} } {
    variable mainThread
    if {[llength [info level 0]] == 1} {
⟦TAB⟧return $mainThread
    }
    set mainThread $new
}

# tcltest::testConstraint --
#
#⟦TAB⟧sets a test constraint to a value; to do multiple constraints,
#       call this proc multiple times.  also returns the value of the
#       named constraint if no value was supplied.
#
# Arguments:
#⟦TAB⟧constraint - name of the constraint
#       value - new value for constraint (should be boolean) - if not
#               supplied, this is a query
#
# Results:
#⟦TAB⟧content of tcltest::testConstraints($constraint)
#
# Side effects:
#⟦TAB⟧none

proc tcltest::testConstraint {constraint {value ""}} {
    variable testConstraints
    variable Option
    DebugPuts 3 "entering testConstraint $constraint $value"
    if {[llength [info level 0]] == 2} {
⟦TAB⟧return $testConstraints($constraint)
    }
    # Check for boolean values
    if {[catch {expr {$value && $value}} msg]} {
⟦TAB⟧return -code error $msg
    }
    if {[limitConstraints]⟦SPACE⟧
⟦TAB⟧    && [lsearch -exact $Option(-constraints) $constraint] == -1} {
⟦TAB⟧set value 0
    }
    set testConstraints($constraint) $value
}

# tcltest::interpreter --
#
#⟦TAB⟧the interpreter name stored in tcltest::tcltest
#
# Arguments:
#⟦TAB⟧executable name
#
# Results:
#⟦TAB⟧content of tcltest::tcltest
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::interpreter { {interp ""} } {
    variable tcltest
    if {[llength [info level 0]] == 1} {
⟦TAB⟧return $tcltest
    }
    if {[string equal {} $interp]} {
⟦TAB⟧set tcltest {}
    } else {
⟦TAB⟧set tcltest $interp
    }
}

#####################################################################

# tcltest::AddToSkippedBecause --
#
#⟦TAB⟧Increments the variable used to track how many tests were
#       skipped because of a particular constraint.
#
# Arguments:
#⟦TAB⟧constraint     The name of the constraint to be modified
#
# Results:
#⟦TAB⟧Modifies tcltest::skippedBecause; sets the variable to 1 if
#       didn't previously exist - otherwise, it just increments it.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::AddToSkippedBecause { constraint {value 1}} {
    # add the constraint to the list of constraints that kept tests
    # from running
    variable skippedBecause

    if {[info exists skippedBecause($constraint)]} {
⟦TAB⟧incr skippedBecause($constraint) $value
    } else {
⟦TAB⟧set skippedBecause($constraint) $value
    }
    return
}

# tcltest::PrintError --
#
#⟦TAB⟧Prints errors to tcltest::errorChannel and then flushes that
#       channel, making sure that all messages are < 80 characters per
#       line.
#
# Arguments:
#⟦TAB⟧errorMsg     String containing the error to be printed
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::PrintError {errorMsg} {
    set InitialMessage "Error:  "
    set InitialMsgLen  [string length $InitialMessage]
    puts -nonewline [errorChannel] $InitialMessage

    # Keep track of where the end of the string is.
    set endingIndex [string length $errorMsg]

    if {$endingIndex < (80 - $InitialMsgLen)} {
⟦TAB⟧puts [errorChannel] $errorMsg
    } else {
⟦TAB⟧# Print up to 80 characters on the first line, including the
⟦TAB⟧# InitialMessage.
⟦TAB⟧set beginningIndex [string last " " [string range $errorMsg 0 \
⟦TAB⟧⟦TAB⟧[expr {80 - $InitialMsgLen}]]]
⟦TAB⟧puts [errorChannel] [string range $errorMsg 0 $beginningIndex]

⟦TAB⟧while {![string equal end $beginningIndex]} {
⟦TAB⟧    puts -nonewline [errorChannel] \
⟦TAB⟧⟦TAB⟧    [string repeat " " $InitialMsgLen]
⟦TAB⟧    if {($endingIndex - $beginningIndex)
⟦TAB⟧⟦TAB⟧    < (80 - $InitialMsgLen)} {
⟦TAB⟧⟦TAB⟧puts [errorChannel] [string trim \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[string range $errorMsg $beginningIndex end]]
⟦TAB⟧⟦TAB⟧break
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧set newEndingIndex [expr {[string last " " \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[string range $errorMsg $beginningIndex \
⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧[expr {$beginningIndex
⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧+ (80 - $InitialMsgLen)}]
⟦TAB⟧⟦TAB⟧]] + $beginningIndex}]
⟦TAB⟧⟦TAB⟧if {($newEndingIndex <= 0)
⟦TAB⟧⟦TAB⟧⟦TAB⟧|| ($newEndingIndex <= $beginningIndex)} {
⟦TAB⟧⟦TAB⟧    set newEndingIndex end
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧puts [errorChannel] [string trim \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[string range $errorMsg \
⟦TAB⟧⟦TAB⟧⟦TAB⟧    $beginningIndex $newEndingIndex]]
⟦TAB⟧⟦TAB⟧set beginningIndex $newEndingIndex
⟦TAB⟧    }
⟦TAB⟧}
    }
    flush [errorChannel]
    return
}

# tcltest::SafeFetch --
#
#⟦TAB⟧ The following trace procedure makes it so that we can safely
#        refer to non-existent members of the testConstraints array
#        without causing an error.  Instead, reading a non-existent
#        member will return 0. This is necessary because tests are
#        allowed to use constraint "X" without ensuring that
#        testConstraints("X") is defined.
#
# Arguments:
#⟦TAB⟧n1 - name of the array (testConstraints)
#       n2 - array key value (constraint name)
#       op - operation performed on testConstraints (generally r)
#
# Results:
#⟦TAB⟧none
#
# Side effects:
#⟦TAB⟧sets testConstraints($n2) to 0 if it's referenced but never
#       before used

proc tcltest::SafeFetch {n1 n2 op} {
    variable testConstraints
    DebugPuts 3 "entering SafeFetch $n1 $n2 $op"
    if {[string equal {} $n2]} {return}
    if {![info exists testConstraints($n2)]} {
⟦TAB⟧if {[catch {testConstraint $n2 [eval [ConstraintInitializer $n2]]}]} {
⟦TAB⟧    testConstraint $n2 0
⟦TAB⟧}
    }
}

# tcltest::ConstraintInitializer --
#
#⟦TAB⟧Get or set a script that when evaluated in the tcltest namespace
#⟦TAB⟧will return a boolean value with which to initialize the
#⟦TAB⟧associated constraint.
#
# Arguments:
#⟦TAB⟧constraint - name of the constraint initialized by the script
#⟦TAB⟧script - the initializer script
#
# Results
#⟦TAB⟧boolean value of the constraint - enabled or disabled
#
# Side effects:
#⟦TAB⟧Constraint is initialized for future reference by [test]
proc tcltest::ConstraintInitializer {constraint {script ""}} {
    variable ConstraintInitializer
    DebugPuts 3 "entering ConstraintInitializer $constraint $script"
    if {[llength [info level 0]] == 2} {
⟦TAB⟧return $ConstraintInitializer($constraint)
    }
    # Check for boolean values
    if {![info complete $script]} {
⟦TAB⟧return -code error "ConstraintInitializer must be complete script"
    }
    set ConstraintInitializer($constraint) $script
}

# tcltest::InitConstraints --
#
# Call all registered constraint initializers to force initialization
# of all known constraints.
# See the tcltest man page for the list of built-in constraints defined
# in this procedure.
#
# Arguments:
#⟦TAB⟧none
#
# Results:
#⟦TAB⟧The testConstraints array is reset to have an index for each
#⟦TAB⟧built-in test constraint.
#
# Side Effects:
#       None.
#

proc tcltest::InitConstraints {} {
    variable ConstraintInitializer
    initConstraintsHook
    foreach constraint [array names ConstraintInitializer] {
⟦TAB⟧testConstraint $constraint
    }
}

proc tcltest::DefineConstraintInitializers {} {
    ConstraintInitializer singleTestInterp {singleProcess}

    # All the 'pc' constraints are here for backward compatibility and
    # are not documented.  They have been replaced with equivalent 'win'
    # constraints.

    ConstraintInitializer unixOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) unix}
    ConstraintInitializer macOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) macintosh}
    ConstraintInitializer pcOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) windows}
    ConstraintInitializer winOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) windows}

    ConstraintInitializer unix {testConstraint unixOnly}
    ConstraintInitializer mac {testConstraint macOnly}
    ConstraintInitializer pc {testConstraint pcOnly}
    ConstraintInitializer win {testConstraint winOnly}

    ConstraintInitializer unixOrPc \
⟦TAB⟧    {expr {[testConstraint unix] || [testConstraint pc]}}
    ConstraintInitializer macOrPc \
⟦TAB⟧    {expr {[testConstraint mac] || [testConstraint pc]}}
    ConstraintInitializer unixOrWin \
⟦TAB⟧    {expr {[testConstraint unix] || [testConstraint win]}}
    ConstraintInitializer macOrWin \
⟦TAB⟧    {expr {[testConstraint mac] || [testConstraint win]}}
    ConstraintInitializer macOrUnix \
⟦TAB⟧    {expr {[testConstraint mac] || [testConstraint unix]}}

    ConstraintInitializer nt {string equal $::tcl_platform(os) "Windows NT"}
    ConstraintInitializer 95 {string equal $::tcl_platform(os) "Windows 95"}
    ConstraintInitializer 98 {string equal $::tcl_platform(os) "Windows 98"}

    # The following Constraints switches are used to mark tests that
    # should work, but have been temporarily disabled on certain
    # platforms because they don't and we haven't gotten around to
    # fixing the underlying problem.

    ConstraintInitializer tempNotPc {expr {![testConstraint pc]}}
    ConstraintInitializer tempNotWin {expr {![testConstraint win]}}
    ConstraintInitializer tempNotMac {expr {![testConstraint mac]}}
    ConstraintInitializer tempNotUnix {expr {![testConstraint unix]}}

    # The following Constraints switches are used to mark tests that
    # crash on certain platforms, so that they can be reactivated again
    # when the underlying problem is fixed.

    ConstraintInitializer pcCrash {expr {![testConstraint pc]}}
    ConstraintInitializer winCrash {expr {![testConstraint win]}}
    ConstraintInitializer macCrash {expr {![testConstraint mac]}}
    ConstraintInitializer unixCrash {expr {![testConstraint unix]}}

    # Skip empty tests

    ConstraintInitializer emptyTest {format 0}

    # By default, tests that expose known bugs are skipped.

    ConstraintInitializer knownBug {format 0}

    # By default, non-portable tests are skipped.

    ConstraintInitializer nonPortable {format 0}

    # Some tests require user interaction.

    ConstraintInitializer userInteraction {format 0}

    # Some tests must be skipped if the interpreter is not in
    # interactive mode

    ConstraintInitializer interactive \
⟦TAB⟧    {expr {[info exists ::tcl_interactive] && $::tcl_interactive}}

    # Some tests can only be run if the installation came from a CD
    # image instead of a web image.  Some tests must be skipped if you
    # are running as root on Unix.  Other tests can only be run if you
    # are running as root on Unix.

    ConstraintInitializer root {expr \
⟦TAB⟧    {[string equal unix $::tcl_platform(platform)]
⟦TAB⟧    && ([string equal root $::tcl_platform(user)]
⟦TAB⟧⟦TAB⟧|| [string equal "" $::tcl_platform(user)])}}
    ConstraintInitializer notRoot {expr {![testConstraint root]}}

    # Set nonBlockFiles constraint: 1 means this platform supports
    # setting files into nonblocking mode.

    ConstraintInitializer nonBlockFiles {
⟦TAB⟧    set code [expr {[catch {set f [open defs r]}]⟦SPACE⟧
⟦TAB⟧⟦TAB⟧    || [catch {fconfigure $f -blocking off}]}]
⟦TAB⟧    catch {close $f}
⟦TAB⟧    set code
    }

    # Set asyncPipeClose constraint: 1 means this platform supports
    # async flush and async close on a pipe.
    #
    # Test for SCO Unix - cannot run async flushing tests because a
    # potential problem with select is apparently interfering.
    # (Mark Diekhans).

    ConstraintInitializer asyncPipeClose {expr {
⟦TAB⟧    !([string equal unix $::tcl_platform(platform)]⟦SPACE⟧
⟦TAB⟧    && ([catch {exec uname -X | fgrep {Release = 3.2v}}] == 0))}}

    # Test to see if we have a broken version of sprintf with respect
    # to the "e" format of floating-point numbers.

    ConstraintInitializer eformat {string equal [format %g 5e-5] 5e-05}

    # Test to see if execed commands such as cat, echo, rm and so forth
    # are present on this machine.

    ConstraintInitializer unixExecs {
⟦TAB⟧set code 1
        if {[string equal macintosh $::tcl_platform(platform)]} {
⟦TAB⟧    set code 0
        }
        if {[string equal windows $::tcl_platform(platform)]} {
⟦TAB⟧    if {[catch {
⟦TAB⟧        set file _tcl_test_remove_me.txt
⟦TAB⟧        makeFile {hello} $file
⟦TAB⟧    }]} {
⟦TAB⟧        set code 0
⟦TAB⟧    } elseif {
⟦TAB⟧        [catch {exec cat $file}] ||
⟦TAB⟧        [catch {exec echo hello}] ||
⟦TAB⟧        [catch {exec sh -c echo hello}] ||
⟦TAB⟧        [catch {exec wc $file}] ||
⟦TAB⟧        [catch {exec sleep 1}] ||
⟦TAB⟧        [catch {exec echo abc > $file}] ||
⟦TAB⟧        [catch {exec chmod 644 $file}] ||
⟦TAB⟧        [catch {exec rm $file}] ||
⟦TAB⟧        [llength [auto_execok mkdir]] == 0 ||
⟦TAB⟧        [llength [auto_execok fgrep]] == 0 ||
⟦TAB⟧        [llength [auto_execok grep]] == 0 ||
⟦TAB⟧        [llength [auto_execok ps]] == 0
⟦TAB⟧    } {
⟦TAB⟧        set code 0
⟦TAB⟧    }
⟦TAB⟧    removeFile $file
        }
⟦TAB⟧set code
    }

    ConstraintInitializer stdio {
⟦TAB⟧set code 0
⟦TAB⟧if {![catch {set f [open "|[list [interpreter]]" w]}]} {
⟦TAB⟧    if {![catch {puts $f exit}]} {
⟦TAB⟧⟦TAB⟧if {![catch {close $f}]} {
⟦TAB⟧⟦TAB⟧    set code 1
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set code
    }

    # Deliberately call socket with the wrong number of arguments.  The
    # error message you get will indicate whether sockets are available
    # on this system.

    ConstraintInitializer socket {
⟦TAB⟧catch {socket} msg
⟦TAB⟧string compare $msg "sockets are not available on this system"
    }

    # Check for internationalization
    ConstraintInitializer hasIsoLocale {
⟦TAB⟧if {[llength [info commands testlocale]] == 0} {
⟦TAB⟧    set code 0
⟦TAB⟧} else {
⟦TAB⟧    set code [string length [SetIso8859_1_Locale]]
⟦TAB⟧    RestoreLocale
⟦TAB⟧}
⟦TAB⟧set code
    }

}
#####################################################################

# Usage and command line arguments processing.

# tcltest::PrintUsageInfo
#
#⟦TAB⟧Prints out the usage information for package tcltest.  This can
#⟦TAB⟧be customized with the redefinition of [PrintUsageInfoHook].
#
# Arguments:
#⟦TAB⟧none
#
# Results:
#       none
#
# Side Effects:
#       none
proc tcltest::PrintUsageInfo {} {
    puts [Usage]
    PrintUsageInfoHook
}

proc tcltest::Usage { {option ""} } {
    variable Usage
    variable Verify
    if {[llength [info level 0]] == 1} {
⟦TAB⟧set msg "Usage: [file tail [info nameofexecutable]] script "
⟦TAB⟧append msg "?-help? ?flag value? ... \n"
⟦TAB⟧append msg "Available flags (and valid input values) are:"

⟦TAB⟧set max 0
⟦TAB⟧set allOpts [concat -help [Configure]]
⟦TAB⟧foreach opt $allOpts {
⟦TAB⟧    set foo [Usage $opt]
⟦TAB⟧    foreach [list x type($opt) usage($opt)] $foo break
⟦TAB⟧    set line($opt) "  $opt $type($opt)  "
⟦TAB⟧    set length($opt) [string length $line($opt)]
⟦TAB⟧    if {$length($opt) > $max} {set max $length($opt)}
⟦TAB⟧}
⟦TAB⟧set rest [expr {72 - $max}]
⟦TAB⟧foreach opt $allOpts {
⟦TAB⟧    append msg \n$line($opt)
⟦TAB⟧    append msg [string repeat " " [expr {$max - $length($opt)}]]
⟦TAB⟧    set u [string trim $usage($opt)]
⟦TAB⟧    catch {append u "  (default: \[[Configure $opt]])"}
⟦TAB⟧    regsub -all {\s*\n\s*} $u " " u
⟦TAB⟧    while {[string length $u] > $rest} {
⟦TAB⟧⟦TAB⟧set break [string wordstart $u $rest]
⟦TAB⟧⟦TAB⟧if {$break == 0} {
⟦TAB⟧⟦TAB⟧    set break [string wordend $u 0]
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧append msg [string range $u 0 [expr {$break - 1}]]
⟦TAB⟧⟦TAB⟧set u [string trim [string range $u $break end]]
⟦TAB⟧⟦TAB⟧append msg \n[string repeat " " $max]
⟦TAB⟧    }
⟦TAB⟧    append msg $u
⟦TAB⟧}
⟦TAB⟧return $msg\n
    } elseif {[string equal -help $option]} {
⟦TAB⟧return [list -help "" "Display this usage information."]
    } else {
⟦TAB⟧set type [lindex [info args $Verify($option)] 0]
⟦TAB⟧return [list $option $type $Usage($option)]
    }
}

# tcltest::ProcessFlags --
#
#⟦TAB⟧process command line arguments supplied in the flagArray - this
#⟦TAB⟧is called by processCmdLineArgs.  Modifies tcltest variables
#⟦TAB⟧according to the content of the flagArray.
#
# Arguments:
#⟦TAB⟧flagArray - array containing name/value pairs of flags
#
# Results:
#⟦TAB⟧sets tcltest variables according to their values as defined by
#       flagArray
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::ProcessFlags {flagArray} {
    # Process -help first
    if {[lsearch -exact $flagArray {-help}] != -1} {
⟦TAB⟧PrintUsageInfo
⟦TAB⟧exit 1
    }

    if {[llength $flagArray] == 0} {
⟦TAB⟧RemoveAutoConfigureTraces
    } else {
⟦TAB⟧set args $flagArray
⟦TAB⟧while {[llength $args]>1 && [catch {eval configure $args} msg]} {

⟦TAB⟧    # Something went wrong parsing $args for tcltest options
⟦TAB⟧    # Check whether the problem is "unknown option"
⟦TAB⟧    if {[regexp {^unknown option (\S+):} $msg -> option]} {
⟦TAB⟧⟦TAB⟧# Could be this is an option the Hook knows about
⟦TAB⟧⟦TAB⟧set moreOptions [processCmdLineArgsAddFlagsHook]
⟦TAB⟧⟦TAB⟧if {[lsearch -exact $moreOptions $option] == -1} {
⟦TAB⟧⟦TAB⟧    # Nope.  Report the error, including additional options,
⟦TAB⟧⟦TAB⟧    # but keep going
⟦TAB⟧⟦TAB⟧    if {[llength $moreOptions]} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧append msg ", "
⟦TAB⟧⟦TAB⟧⟦TAB⟧append msg [join [lrange $moreOptions 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧⟦TAB⟧append msg "or [lindex $moreOptions end]"
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    Warn $msg
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧# error is something other than "unknown option"
⟦TAB⟧⟦TAB⟧# notify user of the error; and exit
⟦TAB⟧⟦TAB⟧puts [errorChannel] $msg
⟦TAB⟧⟦TAB⟧exit 1
⟦TAB⟧    }

⟦TAB⟧    # To recover, find that unknown option and remove up to it.
⟦TAB⟧    # then retry
⟦TAB⟧    while {![string equal [lindex $args 0] $option]} {
⟦TAB⟧⟦TAB⟧set args [lrange $args 2 end]
⟦TAB⟧    }
⟦TAB⟧    set args [lrange $args 2 end]
⟦TAB⟧}
⟦TAB⟧if {[llength $args] == 1} {
⟦TAB⟧    puts [errorChannel] \
⟦TAB⟧⟦TAB⟧    "missing value for option [lindex $args 0]"
⟦TAB⟧    exit 1
⟦TAB⟧}
    }

    # Call the hook
    catch {
        array set flag $flagArray
        processCmdLineArgsHook [array get flag]
    }
    return
}

# tcltest::ProcessCmdLineArgs --
#
#       This procedure must be run after constraint initialization is
#⟦TAB⟧set up (by [DefineConstraintInitializers]) because some constraints
#⟦TAB⟧can be overridden.
#
#       Perform configuration according to the command-line options.
#
# Arguments:
#⟦TAB⟧none
#
# Results:
#⟦TAB⟧Sets the above-named variables in the tcltest namespace.
#
# Side Effects:
#       None.
#

proc tcltest::ProcessCmdLineArgs {} {
    variable originalEnv
    variable testConstraints

    # The "argv" var doesn't exist in some cases, so use {}.
    if {![info exists ::argv]} {
⟦TAB⟧ProcessFlags {}
    } else {
⟦TAB⟧ProcessFlags $::argv
    }

    # Spit out everything you know if we're at a debug level 2 or
    # greater
    DebugPuts 2 "Flags passed into tcltest:"
    if {[info exists ::env(TCLTEST_OPTIONS)]} {
⟦TAB⟧DebugPuts 2 \
⟦TAB⟧⟦TAB⟧"    ::env(TCLTEST_OPTIONS): $::env(TCLTEST_OPTIONS)"
    }
    if {[info exists ::argv]} {
⟦TAB⟧DebugPuts 2 "    argv: $::argv"
    }
    DebugPuts    2 "tcltest::debug              = [debug]"
    DebugPuts    2 "tcltest::testsDirectory     = [testsDirectory]"
    DebugPuts    2 "tcltest::workingDirectory   = [workingDirectory]"
    DebugPuts    2 "tcltest::temporaryDirectory = [temporaryDirectory]"
    DebugPuts    2 "tcltest::outputChannel      = [outputChannel]"
    DebugPuts    2 "tcltest::errorChannel       = [errorChannel]"
    DebugPuts    2 "Original environment (tcltest::originalEnv):"
    DebugPArray  2 originalEnv
    DebugPuts    2 "Constraints:"
    DebugPArray  2 testConstraints
}

#####################################################################

# Code to run the tests goes here.

# tcltest::TestPuts --
#
#⟦TAB⟧Used to redefine puts in test environment.  Stores whatever goes
#⟦TAB⟧out on stdout in tcltest::outData and stderr in errData before
#⟦TAB⟧sending it on to the regular puts.
#
# Arguments:
#⟦TAB⟧same as standard puts
#
# Results:
#⟦TAB⟧none
#
# Side effects:
#       Intercepts puts; data that would otherwise go to stdout, stderr,
#⟦TAB⟧or file channels specified in outputChannel and errorChannel
#⟦TAB⟧does not get sent to the normal puts function.
namespace eval tcltest::Replace {
    namespace export puts
}
proc tcltest::Replace::puts {args} {
    variable [namespace parent]::outData
    variable [namespace parent]::errData
    switch [llength $args] {
⟦TAB⟧1 {
⟦TAB⟧    # Only the string to be printed is specified
⟦TAB⟧    append outData [lindex $args 0]\n
⟦TAB⟧    return
⟦TAB⟧    # return [Puts [lindex $args 0]]
⟦TAB⟧}
⟦TAB⟧2 {
⟦TAB⟧    # Either -nonewline or channelId has been specified
⟦TAB⟧    if {[string equal -nonewline [lindex $args 0]]} {
⟦TAB⟧⟦TAB⟧append outData [lindex $args end]
⟦TAB⟧⟦TAB⟧return
⟦TAB⟧⟦TAB⟧# return [Puts -nonewline [lindex $args end]]
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧set channel [lindex $args 0]
⟦TAB⟧⟦TAB⟧set newline \n
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧3 {
⟦TAB⟧    if {[string equal -nonewline [lindex $args 0]]} {
⟦TAB⟧⟦TAB⟧# Both -nonewline and channelId are specified, unless
⟦TAB⟧⟦TAB⟧# it's an error.  -nonewline is supposed to be argv[0].
⟦TAB⟧⟦TAB⟧set channel [lindex $args 1]
⟦TAB⟧⟦TAB⟧set newline ""
⟦TAB⟧    }
⟦TAB⟧}
    }

    if {[info exists channel]} {
⟦TAB⟧if {[string equal $channel [[namespace parent]::outputChannel]]
⟦TAB⟧⟦TAB⟧|| [string equal $channel stdout]} {
⟦TAB⟧    append outData [lindex $args end]$newline
⟦TAB⟧    return
⟦TAB⟧} elseif {[string equal $channel [[namespace parent]::errorChannel]]
⟦TAB⟧⟦TAB⟧|| [string equal $channel stderr]} {
⟦TAB⟧    append errData [lindex $args end]$newline
⟦TAB⟧    return
⟦TAB⟧}
    }

    # If we haven't returned by now, we don't know how to handle the
    # input.  Let puts handle it.
    return [eval Puts $args]
}

# tcltest::Eval --
#
#⟦TAB⟧Evaluate the script in the test environment.  If ignoreOutput is
#       false, store data sent to stderr and stdout in outData and
#       errData.  Otherwise, ignore this output altogether.
#
# Arguments:
#⟦TAB⟧script             Script to evaluate
#       ?ignoreOutput?     Indicates whether or not to ignore output
#⟦TAB⟧⟦TAB⟧⟦TAB⟧   sent to stdout & stderr
#
# Results:
#⟦TAB⟧result from running the script
#
# Side effects:
#⟦TAB⟧Empties the contents of outData and errData before running a
#⟦TAB⟧test if ignoreOutput is set to 0.

proc tcltest::Eval {script {ignoreOutput 1}} {
    variable outData
    variable errData
    DebugPuts 3 "[lindex [info level 0] 0] called"
    if {!$ignoreOutput} {
⟦TAB⟧set outData {}
⟦TAB⟧set errData {}
⟦TAB⟧rename ::puts [namespace current]::Replace::Puts
⟦TAB⟧namespace eval :: \
⟦TAB⟧⟦TAB⟧[list namespace import [namespace origin Replace::puts]]
⟦TAB⟧namespace import Replace::puts
    }
    set result [uplevel 1 $script]
    if {!$ignoreOutput} {
⟦TAB⟧namespace forget puts
⟦TAB⟧namespace eval :: namespace forget puts
⟦TAB⟧rename [namespace current]::Replace::Puts ::puts
    }
    return $result
}

# tcltest::CompareStrings --
#
#⟦TAB⟧compares the expected answer to the actual answer, depending on
#⟦TAB⟧the mode provided.  Mode determines whether a regexp, exact,
#⟦TAB⟧glob or custom comparison is done.
#
# Arguments:
#⟦TAB⟧a # check for balancing quotes
            append result $text
            set token $word
        }

        if { [catch {llength $token} length] == 0 && $length == 1} {
            # The token is a valid list so add it to the result.
            # lappend result [string trim $token]
            append result \{$token\}
            set token {}
        }
    }

    # If the last token has not been added to the list then there
    # is a problem.
    if { [string length $token] } {
        error "incomplete token \"$token\""
    }

    return $result
}

# tcltest::test --
#
# This procedure runs a test and prints an error message if the test
# fails.  If verbose has been set, it also prints a message even if the
# test succeeds.  The test will be skipped if it doesn't match the
# match variable, if it matches an element in skip, or if one of the
# elements of "constraints" turns out not to be true.
#
# If testLevel is 1, then this is a top level test, and we record
# pass/fail information; otherwise, this information is not logged and
# is not added to running totals.
#
# Attributes:
#   Only description is a required attribute.  All others are optional.
#   Default values are indicated.
#
#   constraints -⟦TAB⟧A list of one or more keywords, each of which
#⟦TAB⟧⟦TAB⟧⟦TAB⟧must be the name of an element in the array
#⟦TAB⟧⟦TAB⟧⟦TAB⟧"testConstraints".  If any of these elements is
#⟦TAB⟧⟦TAB⟧⟦TAB⟧zero, the test is skipped. This attribute is
#⟦TAB⟧⟦TAB⟧⟦TAB⟧optional; default is {}
#   body -⟦TAB⟧        Script to run to carry out the test.  It must
#⟦TAB⟧⟦TAB⟧        return a result that can be checked for
#⟦TAB⟧⟦TAB⟧        correctness.  This attribute is optional;
#                       default is {}
#   result -⟦TAB⟧        Expected result from script.  This attribute is
#                       optional; default is {}.
#   output -            Expected output sent to stdout.  This attribute
#                       is optional; default is {}.
#   errorOutput -       Expected output sent to stderr.  This attribute
#                       is opch"
⟦TAB⟧} else {
⟦TAB⟧    puts [outputChannel] "---- Output was:\n$outData"
⟦TAB⟧    puts [outputChannel] "---- Output should have been\
⟦TAB⟧⟦TAB⟧    ($match matching):\n$output"
⟦TAB⟧}
    }
    if {$errorFailure} {
⟦TAB⟧if {$errorCompare} {
⟦TAB⟧    puts [outputChannel] "---- Error testing errorOutput: $errorMatch"
⟦TAB⟧} else {
⟦TAB⟧    puts [outputChannel] "---- Error output was:\n$errData"
⟦TAB⟧    puts [outputChannel] "---- Error ou {
⟦TAB⟧    puts [outputChannel] "---- errorInfo(cleanup): $errorInfo(cleanup)"
⟦TAB⟧    puts [outputChannel] "---- errorCode(cleanup): $errorCode(cleanup)"
⟦TAB⟧}
    }
    if {$coreFailure} {
⟦TAB⟧puts [outputChannel] "---- Core file produced while running\
⟦TAB⟧⟦TAB⟧test!  $coreMsg"
    }
    puts [outputChannel] "==== $name FAILED\n"

    incr testLevel -1
    return
}

# Skipped --
#
# Given a test name and it constraints, returns a boolean indicating
# whether the current configuration says the test should be skipped.
#
# Side Effects:  Maintains tally of total tests seen and tests skipped.
#
proc tcltest::Skipped {name constraints} {
    variable testLevel
    variable numTests
    variable testConstraints

    if {$testLevel == 1} {
⟦TAB⟧incr numTests(Total)
    }
    # skip the test if it's name matches an element of skip
    foreach pattern [skip] {
⟦TAB⟧if {[string match $pattern $name]} {
⟦TAB⟧    if {$testLevel == 1} {
⟦TAB⟧⟦TAB⟧incr numTests(Skipped)
⟦TAB⟧⟦TAB⟧DebugDo 1 {AddToSkippedBecause userSpecifiedSkip}
⟦TAB⟧    }
⟦TAB⟧    return 1
⟦TAB⟧}
    }
    # skip the test if it's name doesn't match any element of match
    set ok 0
    foreach pattern [match] {
⟦TAB⟧if {[string match $pattern $name]} {
⟦TAB⟧    set ok 1
⟦TAB⟧    break
⟦TAB⟧}
    }
    if {!$ok} {
⟦TAB⟧if {$testLevel == 1} {
⟦TAB⟧    incr numTests(Skipped)
⟦TAB⟧    DebugDo 1 {AddToSkippedBecause userSpecifiedNonMatch}
⟦TAB⟧}
⟦TAB⟧return 1
    }
    if {[string equal {} $constraints]} {
⟦TAB⟧# If we're limited to the listed constraints and there aren't
⟦TAB⟧# any listed, then we shouldn't run the test.
⟦TAB⟧if {[limitConstraints]} {
⟦TAB⟧    AddToSkippedBecause userSpecifiedLimitConstraint
⟦TAB⟧    if {$testLevel =originalEnv($index)
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧foreach index [array names originalEnv] {
⟦TAB⟧    if {![info exists ::env($index)]} {
⟦TAB⟧⟦TAB⟧lappend removedEnv $index
⟦TAB⟧⟦TAB⟧set ::env($index) $originalEnv($index)
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧if {[llength $newEnv] > 0} {
⟦TAB⟧    puts [outputChannel] \
⟦TAB⟧⟦TAB⟧    "env array elements created:\t$newEnv"
⟦TAB⟧}
⟦TAB⟧if {[llength $changedEnv] > 0} {
⟦TAB⟧    puts [outputChannel] \
⟦TAB⟧⟦TAB⟧    "env array elements changed:\t$changedEnv"
⟦TAB⟧}
⟦TAB⟧if {[llength $removedEnv] > 0} {
⟦TAB⟧    puts [outputChannel] \
⟦TAB⟧⟦TAB⟧    "env array elements removed:\t$removedEnv"
⟦TAB⟧}

⟦TAB⟧set changedTclPlatform {}
⟦TAB⟧foreach index [array names originalTclPlatform] {
⟦TAB⟧    if {$::tcl_platform($index) \
⟦TAB⟧⟦TAB⟧    != $originalTclPlatform($index)} {
⟦TAB⟧⟦TAB⟧lappend changedTclPlatform $index
⟦TAB⟧⟦TAB⟧set ::tcl_platform($index) $originalTclPlatform($index)
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧if {[llength $changedTclPlatform] > 0} {
⟦TAB⟧    puts [outputChannel] "tcl_platform array elements\
⟦TAB⟧⟦TAB⟧    changed:\t$changedTclPlatform"
⟦TAB⟧}

⟦TAB⟧if {[file exists [file join [workingDirectory] core]]} {
⟦TAB⟧    if {[preserveCore] > 1} {
⟦TAB⟧⟦TAB⟧puts "rename core file (> 1)"
⟦TAB⟧⟦TAB⟧puts [outputChannel] "produced core file! \
⟦TAB⟧⟦TAB⟧⟦TAB⟧Moving file to: \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [temporaryDirectory] core-$testFileName]"
⟦TAB⟧⟦TAB⟧catch {file rename -force \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [workingDirectory] core] \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [temporaryDirectory] core-$testFileName]
⟦TAB⟧⟦TAB⟧} msg
⟦TAB⟧⟦TAB⟧if {[string length $msg] > 0} {
⟦TAB⟧⟦TAB⟧    PrintError "Problem renaming file: $msg"
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧# Print a message if there is a core file and (1) there
⟦TAB⟧⟦TAB⟧# previously wasn't one or (2) the new one is different
⟦TAB⟧⟦TAB⟧# from the old one.

⟦TAB⟧⟦TAB⟧if {[info exists coreModTime]} {
⟦TAB⟧⟦TAB⟧    if {$coreModTime != [file mtime \
⟦TAB⟧⟦TAB⟧⟦TAB⟧    [file join [workingDirectory] core]]} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧puts [outputChannel] "A core file was created!"
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧} else {
⟦TAB⟧⟦TAB⟧    puts [outputChannel] "A core file was created!"
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
    }
    flush [outputChannel]
    flush [errorChannel]
    return
}

#####################################################################

# Procs that determine which tests/test files to run
d to result list all files in match list and not in skip list
⟦TAB⟧foreach file $matchFileList {
⟦TAB⟧    if {[lsearch -exact $skipFileList $file] == -1} {
⟦TAB⟧⟦TAB⟧lappend matchingFiles $file
⟦TAB⟧    }
⟦TAB⟧}
    }

    if {[llength $matchingFiles] == 0} {
⟦TAB⟧PrintError "No test files remain after applying your match and\
⟦TAB⟧⟦TAB⟧skip patterns!"
    }
    return $matchingFiles
}

# tcltest::GetMatchingDirectories --
#
#⟦TAB⟧Looks at the patterns given to match and skip directories and
#⟦TAB⟧uses them to put together a list of the test directories that we
#⟦TAB⟧should attempt to run.  (Only subdirectories containing an
#⟦TAB⟧"all.tcl" file are put into the list.)
#
# Arguments:
#⟦TAB⟧root directory from which to search
#
# Results:
#⟦TAB⟧The constructed list is returned to the user.  This is used in
#⟦TAB⟧the primary all.tcl file.
#
# Side Effects:
#       None.

proc tcltest::GetMatchingDirectories {rootdir} {

    # Determine the skip list first, to avoid [glob]-ing over subdirectories
    # we're going to throw away anyway.  Be sure we skip the $rootdir if it
    # comes up to avoid infinite loops.
    set skipDirs [list $rootdir]
    foreach pattern [skipDirectories] {
⟦TAB⟧set skipDirs [concat $skipDirs [glob -directory $rootdir -types d \
⟦TAB⟧⟦TAB⟧-nocomplain -- $pattern]]
    }

    # Now step through the matching directories, prune out the skipped ones
    # as you go.
    set matchDirs [list]
    foreach pattern [matchDirectories] {
⟦TAB⟧foreach path [glob -directory $rootdir -types d -nocomplain -- \
⟦TAB⟧⟦TAB⟧$pattern] {
⟦TAB⟧    if {[lsearch -exact $skipDirs $path] == -1} {
⟦TAB⟧⟦TAB⟧set matchDirs [concat $matchDirs [GetMatchingDirectories $path]]
⟦TAB⟧⟦TAB⟧if {[file exists [file join $path all.tcl]]} {
⟦TAB⟧⟦TAB⟧    lappend matchDirs $path
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
    }

    if {[llength $matchDirs] == 0} {
⟦TAB⟧DebugPuts 1 "No test directories remain after applying match\
⟦TAB⟧⟦TAB⟧and skip patterns!"
    }
    return $matchDirs
}

# tcltest::runAllTests --
#
#⟦TAB⟧prints output and sources test files according to the match and
#⟦TAB⟧skip patterns provided.  after sourcing test files, it gce $file]
⟦TAB⟧} else {
⟦TAB⟧    # Pass along our configuration to the child processes.
⟦TAB⟧    # EXCEPT for the -outfile, because the parent process
⟦TAB⟧    # needs to read and process output of children.
⟦TAB⟧    set childargv [list]
⟦TAB⟧    foreach opt [Configure] {
⟦TAB⟧⟦TAB⟧if {[string equal $opt -outfile]} {continue}
⟦TAB⟧⟦TAB⟧lappend childargv $opt [Configure $opt]
⟦TAB⟧    }
⟦TAB⟧    set cmd [linsert $childargv 0 | $shell $file]
⟦TAB⟧    if {[catch {
⟦TAB⟧⟦TAB⟧incr numTestFiles
⟦TAB⟧⟦TAB⟧set pipeFd [open $cmd "r"]
⟦TAB⟧⟦TAB⟧while {[gets $pipeFd line] >= 0} {
⟦TAB⟧⟦TAB⟧    if {[regexp [join {
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {^([^:]+):\t}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {Total\t([0-9]+)\t}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {Passed\t([0-9]+)\t}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {Skipped\t([0-9]+)\t}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {Failed\t([0-9]+)}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    } ""] $line null testFile \
⟦TAB⟧⟦TAB⟧⟦TAB⟧    Total Passed Skipped Failed]} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧foreach index {Total Passed Skipped Failed} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧    incr numTests($index) [set $index]
⟦TAB⟧⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧⟦TAB⟧if {$Failed > 0} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧    lappend failFiles $testFile
⟦TAB⟧⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧    } elseif {[regexp [join {
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {^Number of tests skipped }
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {for each constraint:}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {|^\t(\d+)\t(.+)$}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    } ""] $line match skipped constraint]} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧if {[string match \t* $match]} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧    AddToSkippedBecause $constraint $skipped
⟦TAB⟧⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧⟦TAB⟧puts [outputChannel] $line
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧close $pipeFd
⟦TAB⟧    } msg]} {
⟦TAB⟧⟦TAB⟧puts [outputChannel] "Test file error: $msg"
⟦TAB⟧⟦TAB⟧# append the name of the test to a list to be reported
⟦TAB⟧⟦TAB⟧# later
⟦TAB⟧⟦TAB⟧lappend testFileFailures $file
⟦TAB⟧    }
⟦TAB⟧}
    }

    # cleanup
    puts [outputChannel] "\nTests ended at [eval $timeCmd]"
    cleanupTests 1
    if {[info exists testFileFailures]} {
⟦TAB⟧puts [outputChannel] "\nTest files exiting with errors:  \n"
⟦TAB⟧foreach file $testFileFailures {
⟦TAB⟧    puts [outputChannel] "  [file tail $file]\n"
⟦TAB⟧}
    }

    # Checking for subdirectories in which to run tests
    foreach directory [GetMatchingDirectories [testsDirectory]] {
⟦TAB⟧set dir [file tail $directory]
⟦TAB⟧puts [outputChannel] [string repeat ~ 44]
⟦TAB⟧puts [outputChannel] "$dir test began at [eval $timeCmd]\n"
⟦TAB⟧
⟦TAB⟧uplevel 1 [list ::source [file join $directory  sequence of bytes "\xc3\xa0\xc0\x80".
#
# Generally, it's a bad idea to examine the bytes in a Tcl string or to
# construct improperly formed strings in this manner, because it involves
# exposing thidentity $string]
}

# tcltest::OpenFiles --
#
#⟦TAB⟧used in io tests, uses testchannel
#
# Arguments:
#⟦TAB⟧None.
#
# Results:
#⟦TAB⟧???
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::OpenFiles {} {
    if {[catch {testchannel open} result]} {
⟦TAB⟧return {}
    }
    return $result
}

# tcltest::LeakFiles --
#
#⟦TAB⟧used in io tests, uses testchannel
#
# Arguments:
#⟦TAB⟧None.
#
# Results:
#⟦TAB⟧???
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::LeakFiles {old} {
    if {[catch {testchannel open} new]} {
⟦TAB⟧return {}
    }
    set leak {}
    foreach p $new {
⟦TAB⟧if {[lsearch $old $p] < 0} {
⟦TAB⟧    lappend leak $p
⟦TAB⟧}
    }
    return $leak
}

#
# Internationalization / ISO support procs     -- dl
#

# tcltest::SetIso8859_1_Locale --
#
#⟦TAB⟧used in cmdIL.test, uses testlocale
#
# Arguments:
#⟦TAB⟧None.
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::SetIso8859_1_Locale {} {
    variable previousLocale
    variable isoLocale
    if {[info commands testlocale] != ""} {
⟦TAB⟧set previousLocale [testlocale ctype]
⟦TAB⟧testlocale ctype $isoLocale
    }
    return
}

# tcltest::RestoreLocale --
#
#⟦TAB⟧used in cmdIL.test, uses testlocale
#
# Arguments:
#⟦TAB⟧None.
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::RestoreLocale {} {
    variable previousLocale
    if {[info commands testlocale] != ""} {
⟦TAB⟧testlocale ctype $previousLocale
    }
    return
}

# tcltest::threadReap --
#
#⟦TAB⟧Kill all threads except for the main thread.
#⟦TAB⟧Do nothing if testthread is not defined.
#
# Arguments:
#⟦TAB⟧none.
#
# Results:
#⟦TAB⟧Returns the number of existing threads.
#
# Side Effects:
#       none.
#

proc tcltest::threadReap {} {
    if {[info commands testthread] != {}} {

⟦TAB⟧# testthread built into tcltest

⟦TAB⟧testthread errorproc ThreadNullError
⟦TAB⟧while {[llength [testthread names]] > 1} {
⟦TAB⟧    foreach tid [testthread names] {
⟦TAB⟧⟦TAB⟧if {$tid !nly initialize configurable options from the command line arguments
    # at package load time if necessary for backward compatibility.  This
    # lets the tcltest user call [configure] for themselves if they wish.
    # Traces are established for auto-configuration from the command line
    # if any configurable options are accessed before the user calls
    # [configure].
    if {[LoadTimeCmdLineArgParsingRequired]} {
⟦TAB⟧ProcessCmdLineArgs
    } else {
⟦TAB⟧EstablishAutoConfigureTraces
    }

    package provide [namespace tail [namespace current]] $Version
}
```

- real-harness-explicit-source-child, lookup, completion 0, result:

```text-visible
child-provider {8.4.6 8.4 ::} module-code 1 module-result {invalid command name "pwd"} require-code 1 require-result {can't find package tcltest}⟦EOF⟧
```

- real-harness-explicit-source-child, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- real-harness-explicit-source-child: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 11

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
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

    # tests that use threads need to know which is the main thread
    Default mainThread 1
    variable mainThread
    if {[info commands thread::id] != {}} {
⟦TAB⟧set mainThread [thread::id]
    } elseif {[info commands testthread] != {}} {
⟦TAB⟧set mainThread [testthread id]
    }

    # Set workingDirectory to [pwd]. The default output directory for
    # Tcl tests is the working directory.  Whenever this value changes
    # change to that directory.
    variable workingDirectory
    trace variable workingDirectory w \
⟦TAB⟧    [namespace code {cd $workingDirectory ;#}]

    Default workingDirectory [pwd] AcceptAbsolutePath
    proc workingDirectory { {dir ""} } {
⟦TAB⟧variable workingDirectory
⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $workingDirectory
⟦TAB⟧}
⟦TAB⟧set workingDirectory [AcceptAbsolutePath $dir]
    }

    # Set the location of the execuatble
    Default tcltest [info nameofexecutable]
    trace variable tcltest w [namespace code {testConstraint stdio \
⟦TAB⟧    [eval [ConstraintInitializer stdio]] ;#}]

    # save the platform information so it can be restored later
    Default originalTclPlatform [array get ::tcl_platform]

    # If a core file exists, save its modification time.
    if {[file exists [file join [workingDirectory] core]]} {
⟦TAB⟧Default coreModTime \
⟦TAB⟧⟦TAB⟧[file mtime [file join [workingDirectory] core]]
    }

    # stdout and stderr buffers for use when we want to store them
    Default outData {}
    Default errData {}

    # keep track of test level for nested test commands
    variable testLevel 0

    # the variables and procs that existed when saveState was called are
    # stored in a variable of the same name
    Default saveState {}

    # Internationalization support -- used in [SetIso8859_1_Locale] and
    # [RestoreLocale]. Those commands are used in cmdIL.test.

    if {![info exists [namespace current]::isoLocale]} {
⟦TAB⟧variable isoLocale fr
⟦TAB⟧switch -- $::tcl_platform(platform) {
⟦TAB⟧    "unix" {

⟦TAB⟧⟦TAB⟧# Try some 'known' values for some platforms:

⟦TAB⟧⟦TAB⟧switch -exact -- $::tcl_platform(os) {
⟦TAB⟧⟦TAB⟧    "FreeBSD" {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr_FR.ISO_8859-1
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    HP-UX {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr_FR.iso88591
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    Linux -
⟦TAB⟧⟦TAB⟧    IRIX {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    default {

⟦TAB⟧⟦TAB⟧⟦TAB⟧# Works on SunOS 4 and Solaris, and maybe
⟦TAB⟧⟦TAB⟧⟦TAB⟧# others...  Define it to something else on your
⟦TAB⟧⟦TAB⟧⟦TAB⟧# system if you want to test those.

⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale iso_8859_1
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧    "windows" {
⟦TAB⟧⟦TAB⟧set isoLocale French
⟦TAB⟧    }
⟦TAB⟧}
    }

    variable ChannelsWeOpened; array set ChannelsWeOpened {}
    # output goes to stdout by default
    Default outputChannel stdout
    proc outputChannel { {filename ""} } {
⟦TAB⟧variable outputChannel
⟦TAB⟧variable ChannelsWeOpened

⟦TAB⟧# This is very subtle and tricky, so let me try to explain.
⟦TAB⟧# (Hopefully this longer comment will be clear when I come
⟦TAB⟧# back in a few months, unlike its predecessor :) )
⟦TAB⟧#⟦SPACE⟧
⟦TAB⟧# The [outputChannel] command (and underlying variable) have to
⟦TAB⟧# be kept in sync with the [configure -outfile] configuration
⟦TAB⟧# option ( and underlying variable Option(-outfile) ).  This is
⟦TAB⟧# accomplished with a write trace on Option(-outfile) that will
⟦TAB⟧# update [outputChannel] whenver a new value is written.  That
⟦TAB⟧# much is easy.
⟦TAB⟧#
⟦TAB⟧# The trick is that in order to maintain compatibility with
⟦TAB⟧# version 1 of tcltest, we must allow every configuration option
⟦TAB⟧# to get its inital value from command line arguments.  This is
⟦TAB⟧# accomplished by setting initial read traces on all the
⟦TAB⟧# configuration options to parse the command line option the first
⟦TAB⟧# time they are read.  These traces are cancelled whenever the
⟦TAB⟧# program itself calls [configure].
⟦TAB⟧#⟦SPACE⟧
⟦TAB⟧# OK, then so to support tcltest 1 compatibility, it seems we want
⟦TAB⟧# to get the return from [outputFile] to trigger the read traces,
⟦TAB⟧# just in case.
⟦TAB⟧#
⟦TAB⟧# BUT!  A little known feature of Tcl variable traces is that⟦SPACE⟧
⟦TAB⟧# traces are disabled during the handling of other traces.  So,
⟦TAB⟧# if we trigger read traces on Option(-outfile) and that triggers
⟦TAB⟧# command line parsing which turns around and sets an initial
⟦TAB⟧# value for Option(-outfile) -- <whew!> -- the write trace that
⟦TAB⟧# would keep [outputChannel] in sync with that new initial value
⟦TAB⟧# would not fire!
⟦TAB⟧#
⟦TAB⟧# SO, finally, as a workaround, instead of triggering read traces
⟦TAB⟧# by invoking [outputFile], we instead trigger the same set of
⟦TAB⟧# read traces by invoking [debug].  Any command that reads a
⟦TAB⟧# configuration option would do.  [debug] is just a handy one.
⟦TAB⟧# The end result is that we support tcltest 1 compatibility and
⟦TAB⟧# keep outputChannel and -outfile in sync in all cases.
⟦TAB⟧debug

⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $outputChannel
⟦TAB⟧}
⟦TAB⟧if {[info exists ChannelsWeOpened($outputChannel)]} {
⟦TAB⟧    close $outputChannel
⟦TAB⟧    unset ChannelsWeOpened($outputChannel)
⟦TAB⟧}
⟦TAB⟧switch -exact -- $filename {
⟦TAB⟧    stderr -
⟦TAB⟧    stdout {
⟦TAB⟧⟦TAB⟧set outputChannel $filename
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧set outputChannel [open $filename a]
⟦TAB⟧⟦TAB⟧set ChannelsWeOpened($outputChannel) 1

⟦TAB⟧⟦TAB⟧# If we created the file in [temporaryDirectory], then
⟦TAB⟧⟦TAB⟧# [cleanupTests] will delete it, unless we claim it was
⟦TAB⟧⟦TAB⟧# already there.
⟦TAB⟧⟦TAB⟧set outdir [normalizePath [file dirname \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [pwd] $filename]]]
⟦TAB⟧⟦TAB⟧if {[string equal $outdir [temporaryDirectory]]} {
⟦TAB⟧⟦TAB⟧    variable filesExisted
⟦TAB⟧⟦TAB⟧    FillFilesExisted
⟦TAB⟧⟦TAB⟧    set filename [file tail $filename]
⟦TAB⟧⟦TAB⟧    if {[lsearch -exact $filesExisted $filename] == -1} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧lappend filesExisted $filename
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $outputChannel
    }

    # errors go to stderr by default
    Default errorChannel stderr
    proc errorChannel { {filename ""} } {
⟦TAB⟧variable errorChannel
⟦TAB⟧variable ChannelsWeOpened

⟦TAB⟧# This is subtle and tricky.  See the comment above in
⟦TAB⟧# [outputChannel] for a detailed explanation.
⟦TAB⟧debug

⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $errorChannel
⟦TAB⟧}
⟦TAB⟧if {[info exists ChannelsWeOpened($errorChannel)]} {
⟦TAB⟧    close $errorChannel
⟦TAB⟧    unset ChannelsWeOpened($errorChannel)
⟦TAB⟧}
⟦TAB⟧switch -exact -- $filename {
⟦TAB⟧    stderr -
⟦TAB⟧    stdout {
⟦TAB⟧⟦TAB⟧set errorChannel $filename
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧set errorChannel [open $filename a]
⟦TAB⟧⟦TAB⟧set ChannelsWeOpened($errorChannel) 1

⟦TAB⟧⟦TAB⟧# If we created the file in [temporaryDirectory], then
⟦TAB⟧⟦TAB⟧# [cleanupTests] will delete it, unless we claim it was
⟦TAB⟧⟦TAB⟧# already there.
⟦TAB⟧⟦TAB⟧set outdir [normalizePath [file dirname \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [pwd] $filename]]]
⟦TAB⟧⟦TAB⟧if {[string equal $outdir [temporaryDirectory]]} {
⟦TAB⟧⟦TAB⟧    variable filesExisted
⟦TAB⟧⟦TAB⟧    FillFilesExisted
⟦TAB⟧⟦TAB⟧    set filename [file tail $filename]
⟦TAB⟧⟦TAB⟧    if {[lsearch -exact $filesExisted $filename] == -1} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧lappend filesExisted $filename
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $errorChannel
    }

##### Set up the configurable options
    #
    # The configurable options of the package
    variable Option; array set Option {}

    # Usage strings for those options
    variable Usage; array set Usage {}

    # Verification commands for those options
    variable Verify; array set Verify {}

    # Initialize the default values of the configurable options that are
    # historically associated with an exported variable.  If that variable
    # is already set, support compatibility by accepting its pre-set value.
    # Use [trace] to establish ongoing connection between the deprecated
    # exported variable and the modern option kept as a true internal var.
    # Also set up usage string and value testing for the option.
    proc Option {option value usage {verify AcceptAll} {varName {}}} {
⟦TAB⟧variable Option
⟦TAB⟧variable Verify
⟦TAB⟧variable Usage
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧set Usage($option) $usage
⟦TAB⟧set Verify($option) $verify
⟦TAB⟧if {[catch {$verify $value} msg]} {
⟦TAB⟧    return -code error $msg
⟦TAB⟧} else {
⟦TAB⟧    set Option($option) $msg
⟦TAB⟧}
⟦TAB⟧if {[string length $varName]} {
⟦TAB⟧    variable $varName
⟦TAB⟧    if {[info exists $varName]} {
⟦TAB⟧⟦TAB⟧if {[catch {$verify [set $varName]} msg]} {
⟦TAB⟧⟦TAB⟧    return -code error $msg
⟦TAB⟧⟦TAB⟧} else {
⟦TAB⟧⟦TAB⟧    set Option($option) $msg
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧unset $varName
⟦TAB⟧    }
⟦TAB⟧    namespace eval [namespace current] \
⟦TAB⟧    ⟦TAB⟧    [list upvar 0 Option($option) $varName]
⟦TAB⟧    # Workaround for Bug (now Feature Request) 572889.  Grrrr....
⟦TAB⟧    # Track all the variables tied to options
⟦TAB⟧    lappend OptionControlledVariables $varName
⟦TAB⟧    # Later, set auto-configure read traces on all
⟦TAB⟧    # of them, since a single trace on Option does not work.
⟦TAB⟧    proc $varName {{value {}}} [subst -nocommands {
⟦TAB⟧⟦TAB⟧if {[llength [info level 0]] == 2} {
⟦TAB⟧⟦TAB⟧    Configure $option [set value]
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧return [Configure $option]
⟦TAB⟧    }]
⟦TAB⟧}
    }

    proc MatchingOption {option} {
⟦TAB⟧variable Option
⟦TAB⟧set match [array names Option $option*]
⟦TAB⟧switch -- [llength $match] {
⟦TAB⟧    0 {
⟦TAB⟧⟦TAB⟧set sorted [lsort [array names Option]]
⟦TAB⟧⟦TAB⟧set values [join [lrange $sorted 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append values ", or [lindex $sorted end]"
⟦TAB⟧⟦TAB⟧return -code error "unknown option $option: should be\
⟦TAB⟧⟦TAB⟧⟦TAB⟧one of $values"
⟦TAB⟧    }
⟦TAB⟧    1 {
⟦TAB⟧⟦TAB⟧return [lindex $match 0]
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧# Exact match trumps ambiguity
⟦TAB⟧⟦TAB⟧if {[lsearch -exact $match $option] >= 0} {
⟦TAB⟧⟦TAB⟧    return $option
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧set values [join [lrange $match 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append values ", or [lindex $match end]"
⟦TAB⟧⟦TAB⟧return -code error "ambiguous option $option:\
⟦TAB⟧⟦TAB⟧⟦TAB⟧could match $values"
⟦TAB⟧    }
⟦TAB⟧}
    }

    proc EstablishAutoConfigureTraces {} {
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧foreach varName [concat $OptionControlledVariables Option] {
⟦TAB⟧    variable $varName
⟦TAB⟧    trace variable $varName r [namespace code {ProcessCmdLineArgs ;#}]
⟦TAB⟧}
    }

    proc RemoveAutoConfigureTraces {} {
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧foreach varName [concat $OptionControlledVariables Option] {
⟦TAB⟧    variable $varName
⟦TAB⟧    foreach pair [trace vinfo $varName] {
⟦TAB⟧⟦TAB⟧foreach {op cmd} $pair break
⟦TAB⟧⟦TAB⟧if {[string equal r $op]
⟦TAB⟧⟦TAB⟧⟦TAB⟧&& [string match *ProcessCmdLineArgs* $cmd]} {
⟦TAB⟧⟦TAB⟧    trace vdelete $varName $op $cmd
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧# Once the traces are removed, this can become a no-op
⟦TAB⟧proc RemoveAutoConfigureTraces {} {}
    }

    proc Configure args {
⟦TAB⟧variable Option
⟦TAB⟧variable Verify
⟦TAB⟧set n [llength $args]
⟦TAB⟧if {$n == 0} {
⟦TAB⟧    return [lsort [array names Option]]
⟦TAB⟧}
⟦TAB⟧if {$n == 1} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    return $Option($option)
⟦TAB⟧}
⟦TAB⟧while {[llength $args] > 1} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    if {[catch {$Verify($option) [lindex $args 1]} value]} {
⟦TAB⟧⟦TAB⟧return -code error "invalid $option\
⟦TAB⟧⟦TAB⟧⟦TAB⟧value \"[lindex $args 1]\": $value"
⟦TAB⟧    }
⟦TAB⟧    set Option($option) $value
⟦TAB⟧    set args [lrange $args 2 end]
⟦TAB⟧}
⟦TAB⟧if {[llength $args]} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    return -code error "missing value for option $option"
⟦TAB⟧}
    }
    proc configure args {
⟦TAB⟧RemoveAutoConfigureTraces
⟦TAB⟧set code [catch {eval Configure $args} msg]
⟦TAB⟧return -code $code $msg
    }
⟦SPACE⟧⟦SPACE⟧⟦SPACE⟧⟦SPACE⟧
    proc AcceptVerbose { level } {
⟦TAB⟧set level [AcceptList $level]
⟦TAB⟧if {[llength $level] == 1} {
⟦TAB⟧    if {![regexp {^(pass|body|skip|start|error)$} $level]} {
⟦TAB⟧⟦TAB⟧# translate single characters abbreviations to expanded list
⟦TAB⟧⟦TAB⟧set level [string map {p pass b body s skip t start e error} \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[split $level {}]]
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set valid [list]
⟦TAB⟧foreach v $level {
⟦TAB⟧    if {[regexp {^(pass|body|skip|start|error)$} $v]} {
⟦TAB⟧⟦TAB⟧lappend valid $v
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $valid
    }

    proc IsVerbose {level} {
⟦TAB⟧variable Option
⟦TAB⟧return [expr {[lsearch -exact $Option(-verbose) $level] != -1}]
    }

    # Default verbosity is to show bodies of failed tests
    Option -verbose {body error} {
⟦TAB⟧Takes any combination of the values 'p', 's', 'b', 't' and 'e'.
⟦TAB⟧Test suite will display all passed tests if 'p' is specified, all
⟦TAB⟧skipped tests if 's' is specified, the bodies of failed tests if
⟦TAB⟧'b' is specified, and when tests start if 't' is specified.
⟦TAB⟧ErrorInfo is displayed if 'e' is specified.
    } AcceptVerbose verbose

    # Match and skip patterns default to the empty list, except for
    # matchFiles, which defaults to all .test files in the
    # testsDirectory and matchDirectories, which defaults to all
    # directories.
    Option -match * {
⟦TAB⟧Run all tests within the specified files that match one of the
⟦TAB⟧list of glob patterns given.
    } AcceptList match

    Option -skip {} {
⟦TAB⟧Skip all tests within the specified tests (via -match) and files
⟦TAB⟧that match one of the list of glob patterns given.
    } AcceptList skip

    Option -file *.test {
⟦TAB⟧Run tests in all test files that match the glob pattern given.
    } AcceptPattern matchFiles

    # By default, skip files that appear to be SCCS lock files.
    Option -notfile l.*.test {
⟦TAB⟧Skip all test files that match the glob pattern given.
    } AcceptPattern skipFiles

    Option -relateddir * {
⟦TAB⟧Run tests in directories that match the glob pattern given.
    } AcceptPattern matchDirectories

    Option -asidefromdir {} {
⟦TAB⟧Skip tests in directories that match the glob pattern given.
    } AcceptPattern skipDirectories

    # By default, don't save core files
    Option -preservecore 0 {
⟦TAB⟧If 2, save any core files produced during testing in the directory
⟦TAB⟧specified by -tmpdir. If 1, notify the user if core files are
⟦TAB⟧created.
    } AcceptInteger preserveCore

    # debug output doesn't get printed by default; debug level 1 spits
    # up only the tests that were skipped because they didn't match or
    # were specifically skipped.  A debug level of 2 would spit up the
    # tcltest variables and flags provided; a debug level of 3 causes
    # some additional output regarding operations of the test harness.
    # The tcltest package currently implements only up to debug level 3.
    Option -debug 0 {
⟦TAB⟧Internal debug level⟦SPACE⟧
    } AcceptInteger debug

    proc SetSelectedConstraints args {
⟦TAB⟧variable Option
⟦TAB⟧foreach c $Option(-constraints) {
⟦TAB⟧    testConstraint $c 1
⟦TAB⟧}
    }
    Option -constraints {} {
⟦TAB⟧Do not skip the listed constraints listed in -constraints.
    } AcceptList
    trace variable Option(-constraints) w \
⟦TAB⟧    [namespace code {SetSelectedConstraints ;#}]

    # Don't run only the "-constraint" specified tests by default
    proc ClearUnselectedConstraints args {
⟦TAB⟧variable Option
⟦TAB⟧variable testConstraints
⟦TAB⟧if {!$Option(-limitconstraints)} {return}
⟦TAB⟧foreach c [array names testConstraints] {
⟦TAB⟧    if {[lsearch -exact $Option(-constraints) $c] == -1} {
⟦TAB⟧⟦TAB⟧testConstraint $c 0
⟦TAB⟧    }
⟦TAB⟧}
    }
    Option -limitconstraints false {
⟦TAB⟧whether to run only tests with the constraints
    } AcceptBoolean limitConstraints⟦SPACE⟧
    trace variable Option(-limitconstraints) w \
⟦TAB⟧    [namespace code {ClearUnselectedConstraints ;#}]

    # A test application has to know how to load the tested commands
    # into the interpreter.
    Option -load {} {
⟦TAB⟧Specifies the script to load the tested commands.
    } AcceptScript loadScript

    # Default is to run each test file in a separate process
    Option -singleproc 0 {
⟦TAB⟧whether to run all tests in one process
    } AcceptBoolean singleProcess⟦SPACE⟧

    proc AcceptTemporaryDirectory { directory } {
⟦TAB⟧set directory [AcceptAbsolutePath $directory]
⟦TAB⟧if {![file exists $directory]} {
⟦TAB⟧    file mkdir $directory
⟦TAB⟧}
⟦TAB⟧set directory [AcceptDirectory $directory]
⟦TAB⟧if {![file writable $directory]} {
⟦TAB⟧    if {[string equal [workingDirectory] $directory]} {
⟦TAB⟧⟦TAB⟧# Special exception: accept the default value
⟦TAB⟧⟦TAB⟧# even if the directory is not writable
⟦TAB⟧⟦TAB⟧return $directory
⟦TAB⟧    }
⟦TAB⟧    return -code error "\"$directory\" is not writeable"
⟦TAB⟧}
⟦TAB⟧return $directory
    }

    # Directory where files should be created
    Option -tmpdir [workingDirectory] {
⟦TAB⟧Save temporary files in the specified directory.
    } AcceptTemporaryDirectory temporaryDirectory
    trace variable Option(-tmpdir) w \
⟦TAB⟧    [namespace code {normalizePath Option(-tmpdir) ;#}]

    # Tests should not rely on the current working directory.
    # Files that are part of the test suite should be accessed relative
    # to [testsDirectory]
    Option -testdir [workingDirectory] {
⟦TAB⟧Search tests in the specified directory.
    } AcceptDirectory testsDirectory
    trace variable Option(-testdir) w \
⟦TAB⟧    [namespace code {normalizePath Option(-testdir) ;#}]

    proc AcceptLoadFile { file } {
⟦TAB⟧if {[string equal "" $file]} {return $file}
⟦TAB⟧set file [file join [temporaryDirectory] $file]
⟦TAB⟧return [AcceptReadable $file]
    }
    proc ReadLoadScript {args} {
⟦TAB⟧variable Option
⟦TAB⟧if {[string equal "" $Option(-loadfile)]} {return}
⟦TAB⟧set tmp [open $Option(-loadfile) r]
⟦TAB⟧loadScript [read $tmp]
⟦TAB⟧close $tmp
    }
    Option -loadfile {} {
⟦TAB⟧Read the script to load the tested commands from the specified file.
    } AcceptLoadFile loadFile
    trace variable Option(-loadfile) w [namespace code ReadLoadScript]

    proc AcceptOutFile { file } {
⟦TAB⟧if {[string equal stderr $file]} {return $file}
⟦TAB⟧if {[string equal stdout $file]} {return $file}
⟦TAB⟧return [file join [temporaryDirectory] $file]
    }

    # output goes to stdout by default
    Option -outfile stdout {
⟦TAB⟧Send output from test runs to the specified file.
    } AcceptOutFile outputFile
    trace variable Option(-outfile) w \
⟦TAB⟧    [namespace code {outputChannel $Option(-outfile) ;#}]

    # errors go to stderr by default
    Option -errfile stderr {
⟦TAB⟧Send errors from test runs to the specified file.
    } AcceptOutFile errorFile
    trace variable Option(-errfile) w \
⟦TAB⟧    [namespace code {errorChannel $Option(-errfile) ;#}]

}

#####################################################################

# tcltest::Debug* --
#
#     Internal helper procedures to write out debug information
#     dependent on the chosen level. A test shell may overide
#     them, f.e. to redirect the output into a different
#     channel, or even into a GUI.

# tcltest::DebugPuts --
#
#     Prints the specified string if the current debug level is
#     higher than the provided level argument.
#
# Arguments:
#     level   The lowest debug level triggering the output
#     string  The string to print out.
#
# Results:
#     Prints the string. Nothing else is allowed.
#
# Side Effects:
#     None.
#

proc tcltest::DebugPuts {level string} {
    variable debug
    if {$debug >= $level} {
⟦TAB⟧puts $string
    }
    return
}

# tcltest::DebugPArray --
#
#     Prints the contents of the specified array if the current
#       debug level is higher than the provided level argument
#
# Arguments:
#     level           The lowest debug level triggering the output
#     arrayvar        The name of the array to print out.
#
# Results:
#     Prints the contents of the array. Nothing else is allowed.
#
# Side Effects:
#     None.
#

proc tcltest::DebugPArray {level arrayvar} {
    variable debug

    if {$debug >= $level} {
⟦TAB⟧catch {upvar  $arrayvar $arrayvar}
⟦TAB⟧parray $arrayvar
    }
    return
}

# Define our own [parray] in ::tcltest that will inherit use of the [puts]
# defined in ::tcltest.  NOTE: Ought to construct with [info args] and
# [info default], but can't be bothered now.  If [parray] changes, then
# this will need changing too.
auto_load ::parray
proc tcltest::parray {a {pattern *}} [info body ::parray]

# tcltest::DebugDo --
#
#     Executes the script if the current debug level is greater than
#       the provided level argument
#
# Arguments:
#     level   The lowest debug level triggering the execution.
#     sc== -1} {
⟦TAB⟧set value 0
    }
    set testConstraints($constraint) $value
}

# tcltest::interpreter --
#
#⟦TAB⟧the interpreter name stored in tcltest::tcltest
#
# Arguments:
#⟦TAB⟧executable name
#
# Results:
#⟦TAB⟧content of tcltest::tcltest
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::interpreter { {interp ""} } {
    variable tcltest
    if {[llength [info level 0]] == 1} {
⟦TAB⟧return $tcltest
    }
    if {[string equal {} $interp]} {
⟦TAB⟧set tcltest {}
    } else {
⟦TAB⟧set tcltest $interp
    }
}

#####################################################################

# tcltest::AddToSkippedBecause --
#
#⟦TAB⟧Increments the variable used to track how many tests were
#       skipped because of a particular constraint.
#
# Arguments:
#⟦TAB⟧constraint     The name of the constraint to be modified
#
# Results:
#⟦TAB⟧Modifies tcltest::skippedBecause; sets the variable to 1 if
#       didn't previously exist - otherwise, it just increments it.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::AddToSkippedBecause { constraint {value 1}} {
    # add the constraint to the list of constraints that kept tests
    # from running
    variable skippedBecause

    if {[info exists skippedBecause($constraint)]} {
⟦TAB⟧incr skippedBecause($constraint) $value
    } else {
⟦TAB⟧set skippedBecause($constraint) $value
    }
    return
}

# tcltest::PrintError --
#
#⟦TAB⟧Prints errors to tcltest::errorChannel and then flushes that
#       channel, making sure that all messages are < 80 characters per
#       line.
#
# Arguments:
#⟦TAB⟧errorMsg     String containing the error to be printed
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::PrintError {errorMsg} {
    set InitialMessage "Error:  "
    set InitialMsgLen  [string length $InitialMessage]
    puts -nonewline [errorChannel] $InitialMessage

    # Keep track of where the end of the string is.
    set endingIndex [string length $errorMsg]

    if {$endingIndex < (80 - $InitialMsgLen)} {
⟦TAB⟧puts [errorChannel] $errorMsg
    } else {
⟦TAB⟧# Print up to 80 characters on the first line, ⟦TAB⟧[expr {$beginningIndex
⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧+ (80 - $InitialMsgLen)}]
⟦TAB⟧⟦TAB⟧]] + $beginningIndex}]
⟦TAB⟧⟦TAB⟧if {($newEndingIndex <= 0)
⟦TAB⟧⟦TAB⟧⟦TAB⟧|| ($newEndingIndex <= $beginningIndex)} {
⟦TAB⟧⟦TAB⟧    set newEndingIndex end
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧puts [errorChannel] [string trim \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[string range $errorMsg \
⟦TAB⟧⟦TAB⟧⟦TAB⟧    $beginningIndex $newEndingIndex]]
⟦TAB⟧⟦TAB⟧set beginningIndex $newEndingIndex
⟦TAB⟧    }
⟦TAB⟧}
    }
    flush [errorChannel]
    return
}

# tcltest::SafeFetch --
#
#⟦TAB⟧ The following trace procedure makes it so that we can safely
#        refer to non-existent members of the testConstraints array
#        without causing an error.  Instead, reading a non-existent
#        member will return 0. This is necessary because tests are
#        allowed to use constraint "X" without ensuring that
#        testConstraints("X") is defined.
#
# Arguments:
#⟦TAB⟧n1 - name of the array (testConstraints)
#       n2 - array key value (constraint name)
#       op - operation performed on testConstraints (generally r)
#
# Results:
#⟦TAB⟧none
#
# Side effects:
#⟦TAB⟧sets testConstraints($n2) to 0 if it's referenced but never
#       before used

proc tcltest::SafeFetch {n1 n2 op} {
    variable testConstraints
    DebugPuts 3 "entering SafeFetch $n1 $n2 $op"
    if {[string equal {} $n2]} {return}
    if {![info exists testConstraints($n2)]} {
⟦TAB⟧if {[catch {testConstraint $n2 [eval [ConstraintInitializer $n2]]}]} {
⟦TAB⟧    testConstraint $n2 0
⟦TAB⟧}
    }
}

# tcltest::ConstraintInitializer --
#
#⟦TAB⟧Get or set a script that when evaluated in the tcltest namespace
#⟦TAB⟧will return a boolean value with which to initialize the
#⟦TAB⟧associated constraint.
#
# Arguments:
#⟦TAB⟧constraint - name of the constraint initialized by the script
#⟦TAB⟧script - the initializer script
#
# Results
#⟦TAB⟧boolean value of the constraint - enabled or disabled
#
# Side effects:
#⟦TAB⟧Constraint is initialized for future reference by [test]
proc tcltest::ConstraintInitializer {constraint {script ""}} {
    variable ConstraintInitializer
    DebugPuts 3 "entering ConstraintInitializer $constraint $script"x $length($opt)}
⟦TAB⟧}
⟦TAB⟧set rest [expr {72 - $max}]
⟦TAB⟧foreach opt $allOpts {
⟦TAB⟧    append msg \n$line($opt)
⟦TAB⟧    append msg [string repeat " " [expr {$max - $length($opt)}]]
⟦TAB⟧    set u [string trim $usage($opt)]
⟦TAB⟧    catch {append u "  (default: \[[Configure $opt]])"}
⟦TAB⟧    regsub -all {\s*\n\s*} $u " " u
⟦TAB⟧    while {[string length $u] > $rest} {
⟦TAB⟧⟦TAB⟧set break [string wordstart $u $rest]
⟦TAB⟧⟦TAB⟧if {$break == 0} {
⟦TAB⟧⟦TAB⟧    set break [string wordend $u 0]
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧append msg [string range $u 0 [expr {$break - 1}]]
⟦TAB⟧⟦TAB⟧set u [string trim [string range $u $break end]]
⟦TAB⟧⟦TAB⟧append msg \n[string repeat " " $max]
⟦TAB⟧    }
⟦TAB⟧    append msg $u
⟦TAB⟧}
⟦TAB⟧return $msg\n
    } elseif {[string equal -help $option]} {
⟦TAB⟧return [list -help "" "Display this usage information."]
    } else {
⟦TAB⟧set type [lindex [info args $Verify($option)] 0]
⟦TAB⟧return [list $option $type $Usage($option)]
    }
}

# tcltest::ProcessFlags --
#
#⟦TAB⟧process command line arguments supplied in the flagArray - this
#⟦TAB⟧is called by processCmdLineArgs.  Modifies tcltest variables
#⟦TAB⟧according to the content of the flagArray.
#
# Arguments:
#⟦TAB⟧flagArray - array containing name/value pairs of flags
#
# Results:
#⟦TAB⟧sets tcltest variables according to their values as defined by
#       flagArray
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::ProcessFlags {flagArray} {
    # Process -help first
    if {[lsearch -exact $flagArray {-help}] != -1} {
⟦TAB⟧PrintUsageInfo
⟦TAB⟧exit 1
    }

    if {[llength $flagArray] == 0} {
⟦TAB⟧RemoveAutoConfigureTraces
    } else {
⟦TAB⟧set args $flagArray
⟦TAB⟧while {[llength $args]>1 && [catch {eval configure $args} msg]} {

⟦TAB⟧    # Something went wrong parsing $args for tcltest options
⟦TAB⟧    # Check whether the problem is "unknown option"
⟦TAB⟧    if {[regexp {^unknown option (\S+):} $msg -> option]} {
⟦TAB⟧⟦TAB⟧# Could be this is an option the Hook knows about
⟦TAB⟧⟦TAB⟧set moreOptions [processCmdLineArgsAddFlagsHook]
⟦TAB⟧⟦TAB⟧if {[lsearch -exact $moreOptions $option] == -1} {
⟦TAB⟧⟦TAB⟧    # Nope.  Report the error, including additional options,
⟦TAB⟧⟦TAB⟧    # but keep going
⟦TAB⟧⟦TAB⟧    if {[lontinue 4} {
⟦TAB⟧    set returnCodes [string map -nocase [list $strcode $numcode] $returnCodes]
⟦TAB⟧}
    } else {
⟦TAB⟧# This is parsing for the old test command format; it is here
⟦TAB⟧# for backward compatibility.
⟦TAB⟧set result [lindex $args end]
⟦TAB⟧if {[llength $args] == 2} {
⟦TAB⟧    set body [lindex $args 0]
⟦TAB⟧} elseif {[llength $args] == 3} {
⟦TAB⟧    set constraints [lindex $args 0]
⟦TAB⟧    set body [lindex $args 1]
⟦TAB⟧} else {
⟦TAB⟧    incr testLevel -1
⟦TAB⟧    return -code error "wrong # args:\
⟦TAB⟧⟦TAB⟧    should be \"test name desc ?options?\""
⟦TAB⟧}
    }

    if {[Skipped $name $constraints]} {
⟦TAB⟧incr testLevel -1
⟦TAB⟧return
    }

    # Save information about the core file.⟦SPACE⟧⟦SPACE⟧
    if {[preserveCore]} {
⟦TAB⟧if {[file exists [file join [workingDirectory] core]]} {
⟦TAB⟧    set coreModTime [file mtime [file join [workingDirectory] core]]
⟦TAB⟧}
    }

    # First, run the setup script
    set code [catch {uplevel 1 $setup} setupMsg]
    if {$code == 1} {
⟦TAB⟧set errorInfo(setup) $::errorInfo
⟦TAB⟧set errorCode(setup) $::errorCode
    }
    set setupFailure [expr {$code != 0}]

    # Only run the test body if the setup was successful
    if {!$setupFailure} {

⟦TAB⟧# Verbose notification of $body start
⟦TAB⟧if {[IsVerbose start]} {
⟦TAB⟧    puts [outputChannel] "---- $name start"
⟦TAB⟧    flush [outputChannel]
⟦TAB⟧}

⟦TAB⟧set command [list [namespace origin RunTest] $name $body]
⟦TAB⟧if {[info exists output] || [info exists errorOutput]} {
⟦TAB⟧    set testResult [uplevel 1 [list [namespace origin Eval] $command 0]]
⟦TAB⟧} else {
⟦TAB⟧    set testResult [uplevel 1 [list [namespace origin Eval] $command 1]]
⟦TAB⟧}
⟦TAB⟧foreach {actualAnswer returnCode} $testResult break
⟦TAB⟧if {$returnCode == 1} {
⟦TAB⟧    set errorInfo(body) $::errorInfo
⟦TAB⟧    set errorCode(body) $::errorCode
⟦TAB⟧}
    }

    # Always run the cleanup script
    set code [catch {uplevel 1 $cleanup} cleanupMsg]
    if {$code == 1} {
⟦TAB⟧set errorInfo(cleanup) $::errorInfo
⟦TAB⟧set errorCode(cleanup) $::errorCode
    }
    set cleanupFailure [expr {$code != 0}]

    set coreFailure 0
    set coreMsg ""
    # check for a core  {[set errorCompare [catch {
⟦TAB⟧    CompareStrings $errData $errorOutput $match
⟦TAB⟧} errorMatch]] == 0} {
⟦TAB⟧    set errorFailure [expr {!$errorMatch}]
⟦TAB⟧} else {
⟦TAB⟧    set errorFailure 1
⟦TAB⟧}
    }

    # checmpare [catch {
⟦TAB⟧CompareStrings $actualAnswer $result $match
    } scriptMatch]] == 0} {
⟦TAB⟧set scriptFailure [expr {!$scriptMatch}]
    } else {
⟦TAB⟧set scriptFailure 1
    }

    # if we didn't experience any failures, then we passed
    variable numTests
    if {!($setupFailure || $cleanupFailure || $coreFailure
⟦TAB⟧    || $outputFailure || $errorFailure || $codeFailure
⟦TAB⟧    || $scriptFailure)} {
⟦TAB⟧if {$testLevel == 1} {
⟦TAB⟧    incr numTests(Passed)
⟦TAB⟧    if {[IsVerbose pass]} {
⟦TAB⟧⟦TAB⟧puts [outputChannel] "++++ $name PASSED"
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧incr testLevel -1
⟦TAB⟧return
    }

    # We know the test failed, tally it...
    if {$testLevel == 1} {
⟦TAB⟧incr numTests(Failed)
    }

    # ... then report according to the type of failure
    variable currentFailure true
    if {![IsVerbose body]} {
⟦TAB⟧set body ""
    }⟦TAB⟧
    puts [outputChannel] "\n==== $name\
⟦TAB⟧    [string trim $description] FAILED"
    if {[string length $body]} {
⟦TAB⟧puts [outputChannel] "==== Contents of test case:"
⟦TAB⟧puts [outputChannel] $body
    }
    if {$setupFailure} {
⟦TAB⟧puts [outputChannel] "---- Test setup\
⟦TAB⟧⟦TAB⟧failed:\n$setupMsg"
⟦TAB⟧if {[info exists errorInfo(setup)]} {
⟦TAB⟧    puts [outputChannel] "---- errorInfo(setup): $errorInfo(setup)"
⟦TAB⟧    puts [outputChannel] "---- errorCode(setup): $errorCode(setup)"
⟦TAB⟧}
    }
    if {$scriptFailure} {
⟦TAB⟧if {$scriptCompare} {
⟦TAB⟧    puts [outputChannel] "---- Error testing result: $scriptMatch"
⟦TAB⟧} else {
⟦TAB⟧      4 { set msg "Test generated continue exception" }
⟦TAB⟧    default { set msg "Test generated exception" }
⟦TAB⟧}
⟦TAB⟧puts [outputChannel] "---- $msg; Return code was: $returnCode"
⟦TAB⟧puts [outputChannel] "---- Return code should have been\
⟦TAB⟧⟦TAB⟧one of: $returnCodes"
⟦TAB⟧if {[IsVerbose error]} {
⟦TAB⟧    if {[info exists errorInfo(body)] && ([lsearch $returnCodes 1]<0)} {
⟦TAB⟧⟦TAB⟧puts [outputChannel] "---- errorInfo: $erroped {name constraints} {
    variable testLevel
    variable numTests
    variable testConstraints

    if {$testLevel == 1} {
⟦TAB⟧incr numTests(Total)
    }
    # skip the test if it's name matches an eeturn 1
⟦TAB⟧}
    }
    # skip the test if it's name doesn't match any element of match
    set ok 0
    foreach pattern [match] {
⟦TAB⟧if {[string match $pattern $name]} {
⟦TAB⟧    set ok 1
⟦TAB⟧    break
⟦TAB⟧}
    }
    if {!$ok} {
⟦TAB⟧if {$testLevel == 1} {
⟦TAB⟧    incr numTests(Skipped)
⟦TAB⟧    DebugDo 1 {AddToSkippedBecause userSpecifiedNonMatch}
⟦TAB⟧}
⟦TAB⟧return 1
    }
    if {[string equal {} $constraints]} {
⟦TAB⟧# If we're limited to the listed constraints and there aren't
⟦TAB⟧# any listed, then we shouldn't run the test.
⟦TAB⟧if {[limitConstraints]} {
⟦TAB⟧    AddToSkippedBecause userSpecifiedLimitConstraint
⟦TAB⟧    if {$testLevel == 1} {
⟦TAB⟧⟦TAB⟧incr numTests(Skipped)
⟦TAB⟧    }
⟦TAB⟧    return 1
⟦TAB⟧}
    } else {
⟦TAB⟧# "constraints" argument exists;
⟦TAB⟧# make sure that the constraints are satisfied.

⟦TAB⟧set doTest 0
⟦TAB⟧if {[string match {*[$\[]*} $constraints] != 0} {
⟦TAB⟧    # full expression, e.g. {$foo > [info tclversion]}
⟦TAB⟧    catch {set doTest [uplevel #0 expr $constraints]}
⟦TAB⟧} elseif {[regexp {[^.a-zA-Z0-9 \n\r\t]+} $constraints] != 0} {
⟦TAB⟧    # something like {a || b} should be turned into
⟦TAB⟧    # $testConstraints(a) || $testConstraints(b).
⟦TAB⟧    regsub -all {[.\w]+} $constraints {$testConstraints(&)} c
⟦TAB⟧    catch {set doTest [eval expr $c]}
⟦TAB⟧} elseif {![catch {llength $constraints}]} {
⟦TAB⟧    # just simple constraints such as {unixOnly fonts}.
⟦TAB⟧    set doTest 1
⟦TAB⟧    foreach constraint $constraints {
⟦TAB⟧⟦TAB⟧if {(![info exists testConstraints($constraint)]) \
⟦TAB⟧⟦TAB⟧⟦TAB⟧|| (!$testConstraints($constraint))} {
⟦TAB⟧⟦TAB⟧    set doTest 0

⟦TAB⟧⟦TAB⟧    # store the constraint that kept the test from
⟦TAB⟧⟦TAB⟧    # running
⟦TAB⟧⟦TAB⟧    set constraints $constraint
⟦TAB⟧⟦TAB⟧    break
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧
⟦TAB⟧if {$doTest == 0} {
⟦TAB⟧    if {[IsVerbose skip]} {
⟦TAB⟧⟦TAB⟧puts [outputChannel] "++++ $name SKIPPED: $constraints"
⟦TAB⟧    }

⟦TAB⟧    if {$testLevel == 1} {
⟦TAB⟧⟦TAB⟧incr numTests(Skipped)
⟦TAB⟧⟦TAB⟧AddToSkippedBecause $cctualAnswer]

    return [list $actualAnswer $code]
}

#####################################################################

# tcltest::cleanupTestsHook --
#
#⟦TAB⟧This hook allows a harness that builds upon tcltest to specify
#       additional things that should be done at cleanup.
#

if {[llength [info commands tcltest::cleanupTestsHook]] == 0} {
    proc tcltest::cleanupTestsHook {} {}
}

# tclte$filesMade {
⟦TAB⟧    if {[file exists $file]} {
⟦TAB⟧⟦TAB⟧DebugDo 1 {Warn "cleanupTests deleting $file..."}
⟦TAB⟧⟦TAB⟧catch {file delete -force $file}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set currentFiles {}
⟦TAB⟧foreach file [glob -nocomplain \
⟦TAB⟧⟦TAB⟧-les $file
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set filesExisted $currentFiles
⟦TAB⟧if {[llength $newFiles] > 0} {
⟦TAB⟧    set createdNewFiles($testFileName) $newFiles
⟦TAB⟧}
    }

    if {$calledFromAllFile || $testSingleFile} {

⟦TAB⟧# print stats

⟦TAB⟧puts -nonewline [outputChannel] "$testFileName:"
⟦TAB⟧foreach index [list "Total" "Passed" "Skipped" "Failed"] {
⟦TAB⟧    puts -nonewline [outputChannel] \
⟦TAB⟧⟦TAB⟧    "\t$index\t$numTests($index)"
⟦TAB⟧}
⟦TAB⟧puts [outputChannel] ""

⟦TAB⟧# print number test files sourced
⟦TAB⟧# print names of files that ran tests which failed

⟦TAB⟧if {$calledFromAllFile} {
⟦TAB⟧    puts [outputChannel] \
⟦TAB⟧⟦TAB⟧    "Sourced $numTestFiles Test Files."
⟦TAB⟧    set numTestFiles 0
⟦TAB⟧    if {[llength $failFiles] > 0} {
⟦TAB⟧⟦TAB⟧puts [outputChannel] \
⟦TAB⟧⟦TAB⟧⟦TAB⟧"Files with failing tests: $failFiles"
⟦TAB⟧⟦TAB⟧set failFiles {}
⟦TAB⟧    }
⟦TAB⟧}

⟦TAB⟧# if any tests were skipped, print the constraints that kept
⟦TAB⟧# them from running.

⟦TAB⟧set constraintList [array names skippedBecause]
⟦TAB⟧if {[llength $constraintList] > 0} {
⟦TAB⟧    puts [outputChannel] \
⟦TAB⟧⟦TAB⟧    "Number of tests skipped for each constraint:"
⟦TAB⟧    foreach constraint [lsort $constraintList] {
⟦TAB⟧⟦TAB⟧puts [outputChannel] \
⟦TAB⟧⟦TAB⟧⟦TAB⟧"\t$skippedBecause($constraint)\t$constraint"
⟦TAB⟧⟦TAB⟧unset skippedBecause($constraint)
⟦TAB⟧    }
⟦TAB⟧}

⟦TAB⟧# report the names of test files in createdNewFiles, and reset
⟦TAB⟧# the array to be empty.

⟦TAB⟧set testFilesThatTurded [lsort [array names createdNewFiles]]
⟦TAB⟧if {[llength $testFilesThatTurded] > 0} {
⟦TAB⟧    puts [o {[llength $newEnv] > 0} {
⟦TAB⟧    puts [outputChannel] \
⟦TAB⟧⟦TAB⟧    "env array elements created:\t$newEnv"
⟦TAB⟧}
⟦TAB⟧if {[llength $changedEnv] > 0} {
⟦TAB⟧    puts [outputChannel] \
⟦TAB⟧⟦TAB⟧    "env array elements changed:\t$changedEnv"
⟦TAB⟧}
⟦TAB⟧if {[llength $removedEnv] > 0} {
⟦TAB⟧    puts [outputChannel] \
⟦TAB⟧⟦TAB⟧    "env array elements removed:\t$removedEnv"
⟦TAB⟧}

⟦TAB⟧set changedTclPlatform {}
⟦TAB⟧foreach index [array names originalTclPlatform] {
⟦TAB⟧    if {$::tcl_platform($index) \
⟦TAB⟧⟦TAB⟧    != $originalTclPlatform($index)} {
⟦TAB⟧⟦TAB⟧lappend changedTclPlatform $index
⟦TAB⟧⟦TAB⟧set ::tcl_platform($index) $originalTclPlatform($index)
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧if {[llenserveCore] > 1} {
⟦TAB⟧⟦TAB⟧puts "rename core file (> 1)"
⟦TAB⟧⟦TAB⟧puts [outputChannel] "produced core file! \
⟦TAB⟧⟦TAB⟧⟦TAB⟧Moving file to: \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [temporaryDirectory] core-$testFileName]"
⟦TAB⟧⟦TAB⟧catch {file rename -force \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [workingDirectory] core] \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [temporaryDirectory] core-$testFileName]
⟦TAB⟧⟦TAB⟧} msg
⟦TAB⟧⟦TAB⟧if {[string length $msg] > 0} {
⟦TAB⟧⟦TAB⟧    PrintError "Problem renaming file: $msg"
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧# Print a message if there is a core file and (1) there
⟦TAB⟧⟦TAB⟧# previously wasn't one or (2) the new one is different
⟦TAB⟧⟦TAB⟧# from the old one.

⟦TAB⟧⟦TAB⟧if {[info exists coreModTime]} {
⟦TAB⟧⟦TAB⟧    if {$coreModTime != [file mtime \
⟦TAB⟧⟦TAB⟧⟦TAB⟧    [file join [workingDirectory] core]]} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧puts [outputChannel] "A core file was created!"
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧} else {
⟦TAB⟧⟦TAB⟧    puts [outputChannel] "A core file was created!"
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
    }
    flush [outputChannel]
    flush [errorChannel]
    return
}

#####################################################################

# Procs that determine which tests/test files to run

# tcltest::GetMatchingFiles
#
#       Looks at the patterns given to match and skip files and uses
#⟦TAB⟧them to put together a list of the tests that will be run.
#
# Arguments:
#       directory to search
#
# Results:
#       The constructed list is returned to the user.  This will
#⟦TAB⟧primarily be used in 'all.tcl' files.  It is used in
#⟦TAB⟧runAllTests.
#
# Side Effects:
#       None

# a lower case ve are put into the list.)
#
# Arguments:
#⟦TAB⟧root directory from which to search
#
# Results:
#⟦TAB⟧The constructed list is returned to the user.  This is used in
#⟦TAB⟧the primary all.tcl file.
#
# Side Effects:
#       None.

proc tcltest::GetMatchingDirectories {rootdir} {

    # Determine the skip list first, to avoid [glob]-ing over subdirectories
    # we're going to throw away anyway.  Be sure we skip the $rootdir if it
    # comes up to avoid infinite loops.
    set skipDirs [list $rootdir]
    foreach pattern [skipDirectories] {
⟦TAB⟧set skipDirs [concat $skipDirs [glob -directory $rootdir -types d \
⟦TAB⟧⟦TAB⟧-nocomplain -- $pattern]]
    }

    # Now step through the matching directories, prune out the skipped ones
    # as you go.
    set matchDirs [list]
    foreach pattern [matchDirectories] {
⟦TAB⟧foreach path [glob -directory $rootdir -types d -nocomplain -- \
⟦TAB⟧⟦TAB⟧$pattern] {
⟦TAB⟧    if {[lsearch -exact $skipDirs $path] == -1} {
⟦TAB⟧⟦TAB⟧set matchDirs [concat $matchDirs [GetMatchingDirectories $path]]
⟦TAB⟧⟦TAB⟧if {[file exists [file join $path all.tcl]]} {
⟦TAB⟧⟦TAB⟧    lappend matchDirs $path
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
    }

    if {[llength $matchDirs] == 0} {
⟦TAB⟧DebugPuts 1 "No test directories remain after applying match\
⟦TAB⟧⟦TAB⟧and skip patterns!"
    }
    return $matchDirs
}

# tcltest::runAllTests --
#
#⟦TAB⟧prints output and sources test files according to the match and
#⟦TAB⟧skip patterns provided.  after sourcing test files, it goes on
#⟦TAB⟧to source all.tcl files in matching test subdirectories.
#
# Arguments:
#⟦TAB⟧shell being tested
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::runAllTests { {shell ""} } {
    variable testSingleFile
    variable numTestFiles
    variable numTests
    variable failFiles

    FillFilesExisted
    if {[llength [info level 0]] == 1} {
⟦TAB⟧set shell [interpreter]
    }

    set testSingleFile false

    puts [outputChannel] "Tests running in interp:  $shell"
    puts [outputChannel] "Tests located in:  [testsDirectory]"
    puts [outputChannel] "Tests running in:  [workingDirectoidentity $string]
}

# tcltest::OpenFiles --
#
#⟦TAB⟧used in io tests, uses testchannel
#
# Arguments:
#⟦TAB⟧None.
#
# Results:
#⟦TAB⟧???
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::OpenFiles {} {
    if {[catch {testchannel open} result]} {
⟦TAB⟧return {}
    }
    return $result
}

# tcltest::LeakFiles --
#
#⟦TAB⟧used in io tests, uses testchannel
#
# Arguments:
#⟦TAB⟧None.
#
# Results:
#⟦TAB⟧???
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::LeakFiles {old} {
    if {[catch {testchannel open} new]} {
⟦TAB⟧return {}
    }
    set leak {}
    foreach p $new {
⟦TAB⟧if {[lsearch $old $p] < 0} {
⟦TAB⟧    lappend leak $p
⟦TAB⟧}
    }
    return $leak
}

#
# Internationalization / ISO support procs     -- dl
#

# tcltest::SetIso8859_1_Locale --
#
#⟦TAB⟧used in cmdIL.test, uses testlocale
#
# Arguments:
#⟦TAB⟧None.
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
$isoLocale
    }
    return
}

# tcltest::RestoreLocale --
#
#⟦TAB⟧used in cmdIL.test, uses testlocale
#
# Arguments:
#⟦TAB⟧None.
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::RestoreLocale {} {
    variable previousLocale
    if {[info commands testlocale] != ""} {
⟦TAB⟧testlocale ctype $previousLocale
    }
    return
}

# tcltest::threadReap --
#
#⟦TAB⟧Kill all threads except for the main thread.
#⟦TAB⟧Do nothing if testthread is not defined.
#
# Arguments:
#⟦TAB⟧none.
#
# Results:
#⟦TAB⟧Returns the number of existing threads.
#
# Side Effects:
#       none.
#

proc tcltest::threadReap {} {
    if {[info commands testthread] != {}} {

⟦TAB⟧# testthread built into tcltest

⟦TAB⟧testthread errorproc ThreadNullError
⟦TAB⟧while {[llength [testthread names]] > 1} {
⟦TAB⟧    foreach tid [testthread names] {
⟦TAB⟧⟦TAB⟧if {$tid != [mainThread]} {
⟦TAB⟧⟦TAB⟧    catch {
⟦TAB⟧⟦TAB⟧⟦TAB⟧testthread send -async $tid {testthread exit}
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧    ## Enter a bit a sleep to give the threads enough breathing
⟦TAB⟧    ## room to kill themselves off, otherwise the end up with a
⟦TAB⟧    ## massive queue of repeated events
⟦TAB⟧    after 1
⟦TAB⟧}
⟦TAB⟧testthread errorproc ThreadError
⟦TAB⟧return [llength [testthread names]]
    } elseif {[info commands thread::id] !⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 0, result:

```text-visible
child-provider {8.4.6 8.4 ::} module-code 1 module-result {invalid command name "pwd"} require-code 1 require-result {can't find package tcltest}⟦EOF⟧
```

- real-harness-explicit-source-child, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- real-harness-explicit-source-child: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 12

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
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

    # tests that use threads need to know which is the main thread
    Default mainThread 1
    variable mainThread
    if {[info commands thread::id] != {}} {
⟦TAB⟧set mainThread [thread::id]
    } elseif {[info commands testthread] != {}} {
⟦TAB⟧set mainThread [testthread id]
    }

    # Set workingDirectory to [pwd]. The default output directory for
    # Tcl tests is the working directory.  Whenever this value changes
    # change to that directory.
    variable workingDirectory
    trace variable workingDirectory w \
⟦TAB⟧    [namespace code {cd $workingDirectory ;#}]

    Default workingDirectory [pwd] AcceptAbsolutePath
    proc workingDirectory { {dir ""} } {
⟦TAB⟧variable workingDirectory
⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $workingDirectory
⟦TAB⟧}
⟦TAB⟧set workingDirectory [AcceptAbsolutePath $dir]
    }

    # Set the location of the execuatble
    Default tcltest [info nameofexecutable]
    trace variable tcltest w [namespace code {testConstraint stdio \
⟦TAB⟧    [eval [ConstraintInitializer stdio]] ;#}]

    # save the platform information so it can be restored later
    Default originalTclPlatform [array get ::tcl_platform]

    # If a core file exists, save its modification time.
    if {[file exists [file join [workingDirectory] core]]} {
⟦TAB⟧Default coreModTime \
⟦TAB⟧⟦TAB⟧[file mtime [file join [workingDirectory] core]]
    }

    # stdout and stderr buffers for use when we want to store them
    Default outData {}
    Default errData {}

    # keep track of test level for nested test commands
    variable testLevel 0

    # the variables and procs that existed when saveState was called are
    # stored in a variable of the same name
    Default saveState {}

    # Internationalization support -- used in [SetIso8859_1_Locale] and
    # [RestoreLocale]. Those commands are used in cmdIL.test.

    if {![info exists [namespace current]::isoLocale]} {
⟦TAB⟧variable isoLocale fr
⟦TAB⟧switch -- $::tcl_platform(platform) {
⟦TAB⟧    "unix" {

⟦TAB⟧⟦TAB⟧# Try some 'known' values for some platforms:

⟦TAB⟧⟦TAB⟧switch -exact -- $::tcl_platform(os) {
⟦TAB⟧⟦TAB⟧    "FreeBSD" {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr_FR.ISO_8859-1
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    HP-UX {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr_FR.iso88591
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    Linux -
⟦TAB⟧⟦TAB⟧    IRIX {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    default {

⟦TAB⟧⟦TAB⟧⟦TAB⟧# Works on SunOS 4 and Solaris, and maybe
⟦TAB⟧⟦TAB⟧⟦TAB⟧# others...  Define it to something else on your
⟦TAB⟧⟦TAB⟧⟦TAB⟧# system if you want to test those.

⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale iso_8859_1
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧    "windows" {
⟦TAB⟧⟦TAB⟧set isoLocale French
⟦TAB⟧    }
⟦TAB⟧}
    }

    variable ChannelsWeOpened; array set ChannelsWeOpened {}
    # output goes to stdout by default
    Default outputChannel stdout
    proc outputChannel { {filename ""} } {
⟦TAB⟧variable outputChannel
⟦TAB⟧variable ChannelsWeOpened

⟦TAB⟧# This is very subtle and tricky, so let me try to explain.
⟦TAB⟧# (Hopefully this longer comment will be clear when I come
⟦TAB⟧# back in a few months, unlike its predecessor :) )
⟦TAB⟧#⟦SPACE⟧
⟦TAB⟧# The [outputChannel] command (and underlying variable) have to
⟦TAB⟧# be kept in sync with the [configure -outfile] configuration
⟦TAB⟧# option ( and underlying variable Option(-outfile) ).  This is
⟦TAB⟧# accomplished with a write trace on Option(-outfile) that will
⟦TAB⟧# update [outputChannel] whenver a new value is written.  That
⟦TAB⟧# much is easy.
⟦TAB⟧#
⟦TAB⟧# The trick is that in order to maintain compatibility with
⟦TAB⟧# version 1 of tcltest, we must allow every configuration option
⟦TAB⟧# to get its inital value from command line arguments.  This is
⟦TAB⟧# accomplished by setting initial read traces on all the
⟦TAB⟧# configuration options to parse the command line option the first
⟦TAB⟧# time they are read.  These traces are cancelled whenever the
⟦TAB⟧# program itself calls [configure].
⟦TAB⟧#⟦SPACE⟧
⟦TAB⟧# OK, then so to support tcltest 1 compatibility, it seems we want
⟦TAB⟧# to get the return from [outputFile] to trigger the read traces,
⟦TAB⟧# just in case.
⟦TAB⟧#
⟦TAB⟧# BUT!  A little known feature of Tcl variable traces is that⟦SPACE⟧
⟦TAB⟧# traces are disabled during the handling of other traces.  So,
⟦TAB⟧# if we trigger read traces on Option(-outfile) and that triggers
⟦TAB⟧# command line parsing which turns around and sets an initial
⟦TAB⟧# value for Option(-outfile) -- <whew!> -- the write trace that
⟦TAB⟧# would keep [outputChannel] in sync with that new initial value
⟦TAB⟧# would not fire!
⟦TAB⟧#
⟦TAB⟧# SO, finally, as a workaround, instead of triggering read traces
⟦TAB⟧# by invoking [outputFile], we instead trigger the same set of
⟦TAB⟧# read traces by invoking [debug].  Any command that reads a
⟦TAB⟧# configuration option would do.  [debug] is just a handy one.
⟦TAB⟧# The end result is that we support tcltest 1 compatibility and
⟦TAB⟧# keep outputChannel and -outfile in sync in all cases.
⟦TAB⟧debug

⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $outputChannel
⟦TAB⟧}
⟦TAB⟧if {[info exists ChannelsWeOpened($outputChannel)]} {
⟦TAB⟧    close $outputChannel
⟦TAB⟧    unset ChannelsWeOpened($outputChannel)
⟦TAB⟧}
⟦TAB⟧switch -exact -- $filename {
⟦TAB⟧    stderr -
⟦TAB⟧    stdout {
⟦TAB⟧⟦TAB⟧set outputChannel $filename
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧set outputChannel [open $filename a]
⟦TAB⟧⟦TAB⟧set ChannelsWeOpened($outputChannel) 1

⟦TAB⟧⟦TAB⟧# If we created the file in [temporaryDirectory], then
⟦TAB⟧⟦TAB⟧# [cleanupTests] will delete it, unless we claim it was
⟦TAB⟧⟦TAB⟧# already there.
⟦TAB⟧⟦TAB⟧set outdir [normalizePath [file dirname \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [pwd] $filename]]]
⟦TAB⟧⟦TAB⟧if {[string equal $outdir [temporaryDirectory]]} {
⟦TAB⟧⟦TAB⟧    variable filesExisted
⟦TAB⟧⟦TAB⟧    FillFilesExisted
⟦TAB⟧⟦TAB⟧    set filename [file tail $filename]
⟦TAB⟧⟦TAB⟧    if {[lsearch -exact $filesExisted $filename] == -1} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧lappend filesExisted $filename
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $outputChannel
    }

    # errors go to stderr by default
    Default errorChannel stderr
    proc errorChannel { {filename ""} } {
⟦TAB⟧variable errorChannel
⟦TAB⟧variable ChannelsWeOpened

⟦TAB⟧# This is subtle and tricky.  See the comment above in
⟦TAB⟧# [outputChannel] for a detailed explanation.
⟦TAB⟧debug

⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $errorChannel
⟦TAB⟧}
⟦TAB⟧if {[info exists ChannelsWeOpened($errorChannel)]} {
⟦TAB⟧    close $errorChannel
⟦TAB⟧    unset ChannelsWeOpened($errorChannel)
⟦TAB⟧}
⟦TAB⟧switch -exact -- $filename {
⟦TAB⟧    stderr -
⟦TAB⟧    stdout {
⟦TAB⟧⟦TAB⟧set errorChannel $filename
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧set errorChannel [open $filename a]
⟦TAB⟧⟦TAB⟧set ChannelsWeOpened($errorChannel) 1

⟦TAB⟧⟦TAB⟧# If we created the file in [temporaryDirectory], then
⟦TAB⟧⟦TAB⟧# [cleanupTests] will delete it, unless we claim it was
⟦TAB⟧⟦TAB⟧# already there.
⟦TAB⟧⟦TAB⟧set outdir [normalizePath [file dirname \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [pwd] $filename]]]
⟦TAB⟧⟦TAB⟧if {[string equal $outdir [temporaryDirectory]]} {
⟦TAB⟧⟦TAB⟧    variable filesExisted
⟦TAB⟧⟦TAB⟧    FillFilesExisted
⟦TAB⟧⟦TAB⟧    set filename [file tail $filename]
⟦TAB⟧⟦TAB⟧    if {[lsearch -exact $filesExisted $filename] == -1} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧lappend filesExisted $filename
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $errorChannel
    }

##### Set up the configurable options
    #
    # The configurable options of the package
    variable Option; array set Option {}

    # Usage strings for those options
    variable Usage; array set Usage {}

    # Verification commands for those options
    variable Verify; array set Verify {}

    # Initialize the default values of the configurable options that are
    # historically associated with an exported variable.  If that variable
    # is already set, support compatibility by accepting its pre-set value.
    # Use [trace] to establish ongoing connection between the deprecated
    # exported variable and the modern option kept as a true internal var.
    # Also set up usage string and value testing for the option.
    proc Option {option value usage {verify AcceptAll} {varName {}}} {
⟦TAB⟧variable Option
⟦TAB⟧variable Verify
⟦TAB⟧variable Usage
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧set Usage($option) $usage
⟦TAB⟧set Verify($option) $verify
⟦TAB⟧if {[catch {$verify $value} msg]} {
⟦TAB⟧    return -code error $msg
⟦TAB⟧} else {
⟦TAB⟧    set Option($option) $msg
⟦TAB⟧}
⟦TAB⟧if {[string length $varName]} {
⟦TAB⟧    variable $varName
⟦TAB⟧    if {[info exists $varName]} {
⟦TAB⟧⟦TAB⟧if {[catch {$verify [set $varName]} msg]} {
⟦TAB⟧⟦TAB⟧    return -code error $msg
⟦TAB⟧⟦TAB⟧} else {
⟦TAB⟧⟦TAB⟧    set Option($option) $msg
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧unset $varName
⟦TAB⟧    }
⟦TAB⟧    namespace eval [namespace current] \
⟦TAB⟧    ⟦TAB⟧    [list upvar 0 Option($option) $varName]
⟦TAB⟧    # Workaround for Bug (now Feature Request) 572889.  Grrrr....
⟦TAB⟧    # Track all the variables tied to options
⟦TAB⟧    lappend OptionControlledVariables $varName
⟦TAB⟧    # Later, set auto-configure read traces on all
⟦TAB⟧    # of them, since a single trace on Option does not work.
⟦TAB⟧    proc $varName {{value {}}} [subst -nocommands {
⟦TAB⟧⟦TAB⟧if {[llength [info level 0]] == 2} {
⟦TAB⟧⟦TAB⟧    Configure $option [set value]
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧return [Configure $option]
⟦TAB⟧    }]
⟦TAB⟧}
    }

    proc MatchingOption {option} {
⟦TAB⟧variable Option
⟦TAB⟧set match [array names Option $option*]
⟦TAB⟧switch -- [llength $match] {
⟦TAB⟧    0 {
⟦TAB⟧⟦TAB⟧set sorted [lsort [array names Option]]
⟦TAB⟧⟦TAB⟧set values [join [lrange $sorted 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append values ", or [lindex $sorted end]"
⟦TAB⟧⟦TAB⟧return -code error "unknown option $option: should be\
⟦TAB⟧⟦TAB⟧⟦TAB⟧one of $values"
⟦TAB⟧    }
⟦TAB⟧    1 {
⟦TAB⟧⟦TAB⟧return [lindex $match 0]
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧# Exact match trumps ambiguity
⟦TAB⟧⟦TAB⟧if {[lsearch -exact $match $option] >= 0} {
⟦TAB⟧⟦TAB⟧    return $option
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧set values [join [lrange $match 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append values ", or [lindex $match end]"
⟦TAB⟧⟦TAB⟧return -code error "ambiguous option $option:\
⟦TAB⟧⟦TAB⟧⟦TAB⟧could match $values"
⟦TAB⟧    }
⟦TAB⟧}
    }

    proc EstablishAutoConfigureTraces {} {
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧foreach varName [concat $OptionControlledVariables Option] {
⟦TAB⟧    variable $varName
⟦TAB⟧    trace variable $varName r [namespace code {ProcessCmdLineArgs ;#}]
⟦TAB⟧}
    }

    proc RemoveAutoConfigureTraces {} {
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧foreach varName [concat $OptionControlledVariables Option] {
⟦TAB⟧    variable $varName
⟦TAB⟧    foreach pair [trace vinfo $varName] {
⟦TAB⟧⟦TAB⟧foreach {op cmd} $pair break
⟦TAB⟧⟦TAB⟧if {[string equal r $op]
⟦TAB⟧⟦TAB⟧⟦TAB⟧&& [string match *ProcessCmdLineArgs* $cmd]} {
⟦TAB⟧⟦TAB⟧    trace vdelete $varName $op $cmd
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧# Once the traces are removed, this can become a no-op
⟦TAB⟧proc RemoveAutoConfigureTraces {} {}
    }

    proc Configure args {
⟦TAB⟧variable Option
⟦TAB⟧variable Verify
⟦TAB⟧set n [llength $args]
⟦TAB⟧if {$n == 0} {
⟦TAB⟧    return [lsort [array names Option]]
⟦TAB⟧}
⟦TAB⟧if {$n == 1} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    return $Option($option)
⟦TAB⟧}
⟦TAB⟧while {[llength $args] > 1} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    if {[catch {$Verify($option) [lindex $args 1]} value]} {
⟦TAB⟧⟦TAB⟧return -code error "invalid $option\
⟦TAB⟧⟦TAB⟧⟦TAB⟧value \"[lindex $args 1]\": $value"
⟦TAB⟧    }
⟦TAB⟧    set Option($option) $value
⟦TAB⟧    set args [lrange $args 2 end]
⟦TAB⟧}
⟦TAB⟧if {[llength $args]} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    return -code error "missing value for option $option"
⟦TAB⟧}
    }
    proc configure args {
⟦TAB⟧RemoveAutoConfigureTraces
⟦TAB⟧set code [catch {eval Configure $args} msg]
⟦TAB⟧return -code $code $msg
    }
⟦SPACE⟧⟦SPACE⟧⟦SPACE⟧⟦SPACE⟧
    proc AcceptVerbose { level } {
⟦TAB⟧set level [AcceptList $level]
⟦TAB⟧if {[llength $level] == 1} {
⟦TAB⟧    if {![regexp {^(pass|body|skip|start|error)$} $level]} {
⟦TAB⟧⟦TAB⟧# translate single characters abbreviations to expanded list
⟦TAB⟧⟦TAB⟧set level [string map {p pass b body s skip t start e error} \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[split $level {}]]
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set valid [list]
⟦TAB⟧foreach v $level {
⟦TAB⟧    if {[regexp {^(pass|body|skip|start|error)$} $v]} {
⟦TAB⟧⟦TAB⟧lappend valid $v
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $valid
    }

    proc IsVerbose {level} {
⟦TAB⟧variable Option
⟦TAB⟧return [expr {[lsearch -exact $Option(-verbose) $level] != -1}]
    }

    # Default verbosity is to show bodies of failed tests
    Option -verbose {body error} {
⟦TAB⟧Takes any combination of the values 'p', 's', 'b', 't' and 'e'.
⟦TAB⟧Test suite will display all passed tests if 'p' is specified, all
⟦TAB⟧skipped tests if 's' is specified, the bodies of failed tests if
⟦TAB⟧'b' is specified, and when tests start if 't' is specified.
⟦TAB⟧ErrorInfo is displayed if 'e' is specified.
    } AcceptVerbose verbose

    # Match and skip patterns default to the empty list, except for
    # matchFiles, which defaults to all .test files in the
    # testsDirectory and matchDirectories, which defaults to all
    # directories.
    Option -match * {
⟦TAB⟧Run all tests within the specified files that match one of the
⟦TAB⟧list of glob patterns given.
    } AcceptList match

    Option -skip {} {
⟦TAB⟧Skip all tests within the specified tests (via -match) and files
⟦TAB⟧that match one of the list of glob patterns given.
    } AcceptList skip

    Option -file *.test {
⟦TAB⟧Run tests in all test files that match the glob pattern given.
    } AcceptPattern matchFiles

    # By default, skip files that appear to be SCCS lock files.
    Option -notfile l.*.test {
⟦TAB⟧Skip all test files that match the glob pattern given.
    } AcceptPattern skipFiles

    Option -relateddir * {
⟦TAB⟧Run tests in directories that match the glob pattern given.
    } AcceptPattern matchDirectories

    Option -asidefromdir {} {
⟦TAB⟧Skip tests in directories that match the glob pattern given.
    } AcceptPattern skipDirectories

    # By default, don't save core files
    Option -preservecore 0 {
⟦TAB⟧If 2, save any core files produced during testing in the directory
⟦TAB⟧specified by -tmpdir. If 1, notify the user if core files are
⟦TAB⟧created.
    } AcceptInteger preserveCore

    # debug output doesn't get printed by default; debug level 1 spits
    # up only the tests that were skipped because they didn't match or
    # were specifically skipped.  A debug level of 2 would spit up the
    # tcltest variables and flags provided; a debug level of 3 causes
    # some additional output regarding operations of the test harness.
    # The tcltest package currently implements only up to debug level 3.
    Option -debug 0 {
⟦TAB⟧Internal debug level⟦SPACE⟧
    } AcceptInteger debug

    proc SetSelectedConstraints args {
⟦TAB⟧variable Option
⟦TAB⟧foreach c $Option(-constraints) {
⟦TAB⟧    testConstraint $c 1
⟦TAB⟧}
    }
    Option -constraints {} {
⟦TAB⟧Do not skip the listed constraints listed in -constraints.
    } AcceptList
    trace variable Option(-constraints) w \
⟦TAB⟧    [namespace code {SetSelectedConstraints ;#}]

    # Don't run only the "-constraint" specified tests by default
    proc ClearUnselectedConstraints args {
⟦TAB⟧variable Option
⟦TAB⟧variable testConstraints
⟦TAB⟧if {!$Option(-limitconstraints)} {return}
⟦TAB⟧foreach c [array names testConstraints] {
⟦TAB⟧    if {[lsearch -exact $Option(-constraints) $c] == -1} {
⟦TAB⟧⟦TAB⟧testConstraint $c 0
⟦TAB⟧    }
⟦TAB⟧}
    }
    Option -limitconstraints false {
⟦TAB⟧whether to run only tests with the constraints
    } AcceptBoolean limitConstraints⟦SPACE⟧
    trace variable Option(-limitconstraints) w \
⟦TAB⟧    [namespace code {ClearUnselectedConstraints ;#}]

    # A test application has to know how to load the tested commands
    # into the interpreter.
    Option -load {} {
⟦TAB⟧Specifies the script to load the tested commands.
    } AcceptScript loadScript

    # Default is to run each test file in a separate process
    Option -singleproc 0 {
⟦TAB⟧whether to run all tests in one process
    } AcceptBoolean singleProcess⟦SPACE⟧

    proc AcceptTemporaryDirectory { directory } {
⟦TAB⟧set directory [AcceptAbsolutePath $directory]
⟦TAB⟧if {![file exists $directory]} {
⟦TAB⟧    file mkdir $directory
⟦TAB⟧}
⟦TAB⟧set directory [AcceptDirectory $directory]
⟦TAB⟧if {![file writable $directory]} {
⟦TAB⟧    if {[string equal [workingDirectory] $directory]} {
⟦TAB⟧⟦TAB⟧# Special exception: accept the default value
⟦TAB⟧⟦TAB⟧# even if the directory is not writable
⟦TAB⟧⟦TAB⟧return $directory
⟦TAB⟧    }
⟦TAB⟧    return -code error "\"$directory\" is not writeable"
⟦TAB⟧}
⟦TAB⟧return $directory
    }

    # Directory where files should be created
    Option -tmpdir [workingDirectory] {
⟦TAB⟧Save temporary files in the specified directory.
    } AcceptTemporaryDirectory temporaryDirectory
    trace variable Option(-tmpdir) w \
⟦TAB⟧    [namespace code {normalizePath Option(-tmpdir) ;#}]

    # Tests should not rely on the current working directory.
    # Files that are part of the test suite should be accessed relative
    # to [testsDirectory]
    Option -testdir [workingDirectory] {
⟦TAB⟧Search tests in the specified directory.
    } AcceptDirectory testsDirectory
    trace variable Option(-testdir) w \
⟦TAB⟧    [namespace code {normalizePath Option(-testdir) ;#}]

    proc AcceptLoadFile { file } {
⟦TAB⟧if {[string equal "" $file]} {return $file}
⟦TAB⟧set file [file join [temporaryDirectory] $file]
⟦TAB⟧return [AcceptReadable $file]
    }
    proc ReadLoadScript {args} {
⟦TAB⟧variable Option
⟦TAB⟧if {[string equal "" $Option(-loadfile)]} {return}
⟦TAB⟧set tmp [open $Option(-loadfile) r]
⟦TAB⟧loadScript [read $tmp]
⟦TAB⟧close $tmp
    }
    Option -loadfile {} {
⟦TAB⟧Read the script to load the tested commands from the specified file.
    } AcceptLoadFile loadFile
    trace variable Option(-loadfile) w [namespace code ReadLoadScript]

    proc AcceptOutFile { file } {
⟦TAB⟧if {[string equal stderr $file]} {return $file}
⟦TAB⟧if {[string equal stdout $file]} {return $file}
⟦TAB⟧return [file join [temporaryDirectory] $file]
    }

    # output goes to stdout by default
    Option -outfile stdout {
⟦TAB⟧Send output from test runs to the specified file.
    } AcceptOutFile outputFile
    trace variable Option(-outfile) w \
⟦TAB⟧    [namespace code {outputChannel $Option(-outfile) ;#}]

    # errors go to stderr by default
    Option -errfile stderr {
⟦TAB⟧Send errors from test runs to the specified file.
    } AcceptOutFile errorFile
    trace variable Option(-errfile) w \
⟦TAB⟧    [namespace code {errorChannel $Option(-errfile) ;#}]

}

#####################################################################

# tcltest::Debug* --
#
#     Internal helper procedures to write out debug information
#     dependent on the chosen level. A test shell may overide
#     them, f.e. to redirect the output into a different
#     channel, or even into a GUI.

# tcltest::DebugPuts --
#
#     Prints the specified string if the current debug level is
#     higher than the provided level argument.
#
# Arguments:
#     level   The lowest debug level triggering the output
#     string  The string to print out.
#
# Results:
#     Prints the string. Nothing else is allowed.
#
# Side Effects:
#     None.
#

proc tcltest::DebugPuts {level string} {
    variable debug
    if {$debug >= $level} {
⟦TAB⟧puts $string
    }
    return
}

# tcltest::DebugPArray --
#
#     Prints the contents of the specified array if the current
#       debug level is higher than the provided level argument
#
# Arguments:
#     level           The lowest debug level triggering the output
#     arrayvar        The name of the array to print out.
#
# Results:
#     Prints the contents of the array. Nothing else is allowed.
#
# Side Effects:
#     None.
#

proc tcltest::DebugPArray {level arrayvar} {
    variable debug

    if {$debug >= $level} {
⟦TAB⟧catch {upvar  $arrayvar $arrayvar}
⟦TAB⟧parray $arrayvar
    }
    return
}

# Define our own [parray] in ::tcltest that will inherit use of the [puts]
# defined in ::tcltest.  NOTE: Ought to construct with [info args] and
# [info default], but can't be bothered now.  If [parray] changes, then
# this will need changing too.
auto_load ::parray
proc tcltest::parray {a {pattern *}} [info body ::parray]

# tcltest::DebugDo --
#
#     Executes the script if the current debug level is greater than
#       the provided level argument
#
# Arguments:
#     level   The lowest debug level triggering the execution.
#     script  The tcl script executed upon a debug level high enough.
#
# Results:
#     Arbitrary side effects, dependent on the executed script.
#
# Side Effects:
#     None.
#

proc tcltest::DebugDo {level script} {
    variable debug

    if {$debug >= $level} {
⟦TAB⟧uplevel 1 $script
    }
    return
}

#####################################################################

proc tcltest::Warn {msg} {
    puts [outputChannel] "WARNING: $msg"
}

# tcltest::mainThread
#
#     Accessor command for tcltest variable mainThread.
#
proc tcltest::mainThread { {new ""} } {
    variable mainThread
    if {[llength [info level 0]] == 1} {
⟦TAB⟧return $mainThread
    }
    set mainThread $new
}

# tcltest::testConstraint --
#
#⟦TAB⟧sets a test constraint to a value; to do multiple constraints,
#       call this proc multiple times.  also returns the value of the
#       named constraint if no value was supplied.
#
# Arguments:
#⟦TAB⟧constraint - name of the constraint
#       value - new value for constraint (should be boolean) - if not
#               supplied, this is a query
#
# Results:
#⟦TAB⟧content of tcltest::testConstraints($constraint)
#
# Side effects:
#⟦TAB⟧none

proc tcltest::testConstraint {constraint {value ""}} {
    variable testConstraints
    variable Option
    DebugPuts 3 "entering testConstraint $constraint $value"
    if {[llength [info level 0]] == 2} {
⟦TAB⟧return $testConstraints($constraint)
    }
    # Check for boolean values
    if {[catch {expr {$value && $value}} msg]} {
⟦TAB⟧return -code error $msg
    }
    if {[limitConstraints]⟦SPACE⟧
⟦TAB⟧    && [lsearch -exact $Option(-constraints) $constraint] == -1} {
⟦TAB⟧set value 0
    }
    set testConstraints($constraint) $value
}

# tcltest::interpreter --
#
#⟦TAB⟧the interpreter name stored in tcltest::tcltest
#
# Arguments:
#⟦TAB⟧executable name
#
# Results:
#⟦TAB⟧content of tcltest::tcltest
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::interpreter { {interp ""} } {
    variable tcltest
    if {[llength [info level 0]] == 1} {
⟦TAB⟧return $tcltest
    }
    if {[string equal {} $interp]} {
⟦TAB⟧set tcltest {}
    } else {
⟦TAB⟧set tcltest $interp
    }
}

#####################################################################

# tcltest::AddToSkippedBecause --
#
#⟦TAB⟧Increments the variable used to track how many tests were
#       skipped because of a particular constraint.
#
# Arguments:
#⟦TAB⟧constraint     The name of the constraint to be modified
#
# Results:
#⟦TAB⟧Modifies tcltest::skippedBecause; sets the variable to 1 if
#       didn't previously exist - otherwise, it just increments it.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::AddToSkippedBecause { constraint {value 1}} {
    # add the constraint to the list of constraints that kept tests
    # from running
    variable skippedBecause

    if {[info exists skippedBecause($constraint)]} {
⟦TAB⟧incr skippedBecause($constraint) $value
    } else {
⟦TAB⟧set skippedBecause($constraint) $value
    }
    return
}

# tcltest::PrintError --
#
#⟦TAB⟧Prints errors to tcltest::errorChannel and then flushes that
#       channel, making sure that all messages are < 80 characters per
#       line.
#
# Arguments:
#⟦TAB⟧errorMsg     String containing the error to be printed
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::PrintError {errorMsg} {
    set InitialMessage "Error:  "
    set InitialMsgLen  [string length $InitialMessage]
    puts -nonewline [errorChannel] $InitialMessage

    # Keep track of where the end of the string is.
    set endingIndex [string length $errorMsg]

    if {$endingIndex < (80 - $InitialMsgLen)} {
⟦TAB⟧puts [errorChannel] $errorMsg
    } else {
⟦TAB⟧# Print up to 80 characters on the first line, including the
⟦TAB⟧# InitialMessage.
⟦TAB⟧set beginningIndex [string last " " [string range $errorMsg 0 \
⟦TAB⟧⟦TAB⟧[expr {80 - $InitialMsgLen}]]]
⟦TAB⟧puts [errorChannel] [string range $errorMsg 0 $beginningIndex]

⟦TAB⟧while {![string equal end $beginningIndex]} {
⟦TAB⟧    puts -nonewline [errorChannel] \
⟦TAB⟧⟦TAB⟧    [string repeat " " $InitialMsgLen]
⟦TAB⟧    if {($endingIndex - $beginningIndex)
⟦TAB⟧⟦TAB⟧    < (80 - $InitialMsgLen)} {
⟦TAB⟧⟦TAB⟧puts [errorChannel] [string trim \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[string range $errorMsg $beginningIndex end]]
⟦TAB⟧⟦TAB⟧break
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧set newEndingIndex [expr {[string last " " \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[string range $errorMsg $beginningIndex \
⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧[expr {$beginningIndex
⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧+ (80 - $InitialMsgLen)}]
⟦TAB⟧⟦TAB⟧]] + $beginningIndex}]
⟦TAB⟧⟦TAB⟧if {($newEndingIndex <= 0)
⟦TAB⟧⟦TAB⟧⟦TAB⟧|| ($newEndingIndex <= $beginningIndex)} {
⟦TAB⟧⟦TAB⟧    set newEndingIndex end
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧puts [errorChannel] [string trim \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[string range $errorMsg \
⟦TAB⟧⟦TAB⟧⟦TAB⟧    $beginningIndex $newEndingIndex]]
⟦TAB⟧⟦TAB⟧set beginningIndex $newEndingIndex
⟦TAB⟧    }
⟦TAB⟧}
    }
    flush [errorChannel]
    return
}

# tcltest::SafeFetch --
#
#⟦TAB⟧ The following trace procedure makes it so that we can safely
#        refer to non-existent members of the testConstraints array
#        without causing an error.  Instead, reading a non-existent
#        member will return 0. This is necessary because tests are
#        allowed to use constraint "X" without ensuring that
#        testConstraints("X") is defined.
#
# Arguments:
#⟦TAB⟧n1 - name of the array (testConstraints)
#       n2 - array key value (constraint name)
#       op - operation performed on testConstraints (generally r)
#
# Results:
#⟦TAB⟧none
#
# Side effects:
#⟦TAB⟧sets testConstraints($n2) to 0 if it's referenced but never
#       before used

proc tcltest::SafeFetch {n1 n2 op} {
    variable testConstraints
    DebugPuts 3 "entering SafeFetch $n1 $n2 $op"
    if {[string equal {} $n2]} {return}
    if {![info exists testConstraints($n2)]} {
⟦TAB⟧if {[catch {testConstraint $n2 [eval [ConstraintInitializer $n2]]}]} {
⟦TAB⟧    testConstraint $n2 0
⟦TAB⟧}
    }
}

# tcltest::ConstraintInitializer --
#
#⟦TAB⟧Get or set a script that when evaluated in the tcltest namespace
#⟦TAB⟧will return a boolean value with which to initialize the
#⟦TAB⟧associated constraint.
#
# Arguments:
#⟦TAB⟧constraint - name of the constraint initialized by the script
#⟦TAB⟧script - the initializer script
#
# Results
#⟦TAB⟧boolean value of the constraint - enabled or disabled
#
# Side effects:
#⟦TAB⟧Constraint is initialized for future reference by [test]
proc tcltest::ConstraintInitializer {constraint {script ""}} {
    variable ConstraintInitializer
    DebugPuts 3 "entering ConstraintInitializer $constraint $script"
    if {[llength [info level 0]] == 2} {
⟦TAB⟧return $ConstraintInitializer($constraint)
    }
    # Check for boolean values
    if {![info complete $script]} {
⟦TAB⟧return -code error "ConstraintInitializer must be complete script"
    }
    set ConstraintInitializer($constraint) $script
}

# tcltest::InitConstraints --
#
# Call all registered constraint initializers to force initialization
# of all known constraints.
# See the tcltest man page for the list of built-in constraints defined
# in this procedure.
#
# Arguments:
#⟦TAB⟧none
#
# Results:
#⟦TAB⟧The testConstraints array is reset to have an index for each
#⟦TAB⟧built-in test constraint.
#
# Side Effects:
#       None.
#

proc tcltest::InitConstraints {} {
    variable ConstraintInitializer
    initConstraintsHook
    foreach constraint [array names ConstraintInitializer] {
⟦TAB⟧testConstraint $constraint
    }
}

proc tcltest::DefineConstraintInitializers {} {
    ConstraintInitializer singleTestInterp {singleProcess}

    # All the 'pc' constraints are here for backward compatibility and
    # are not documented.  They have been replaced with equivalent 'win'
    # constraints.

    ConstraintInitializer unixOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) unix}
    ConstraintInitializer macOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) macintosh}
    ConstraintInitializer pcOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) windows}
    ConstraintInitializer winOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) windows}

    ConstraintInitializer unix {testConstraint unixOnly}
    ConstraintInitializer mac {testConstraint macOnly}
    ConstraintInitializer pc {testConstraint pcOnly}
    ConstraintInitializer win {testConstraint winOnly}

    ConstraintInitializer unixOrPc \
⟦TAB⟧    {expr {[testConstraint unix] || [testConstraint pc]}}
    ConstraintInitializer macOrPc \
⟦TAB⟧    {expr {[testConstraint mac] || [testConstraint pc]}}
    ConstraintInitializer unixOrWin \
⟦TAB⟧    {expr {[testConstraint unix] || [testConstraint win]}}
    ConstraintInitializer macOrWin \
⟦TAB⟧    {expr {[testConstraint mac] || [testConstraint win]}}
    ConstraintInitializer macOrUnix \
⟦TAB⟧    {expr {[testConstraint mac] || [testConstraint unix]}}

    ConstraintInitializer nt {string equal $::tcl_platform(os) "Windows NT"}
    ConstraintInitializer 95 {string equal $::tcl_platform(os) "Windows 95"}
    ConstraintInitializer 98 {string equal $::tcl_platform(os) "Windows 98"}

    # The following Constraints switches are used to mark tests that
    # should work, but have been temporarily disabled on certain
    # platforms because they don't and we haven't gotten around to
    # fixing the underlying problem.

    ConstraintInitializer tempNotPc {expr {![testConstraint pc]}}
    ConstraintInitializer tempNotWin {expr {![testConstraint win]}}
    ConstraintInitializer tempNotMac {expr {![testConstraint mac]}}
    ConstraintInitializer tempNotUnix {expr {![testConstraint unix]}}

    # The following Constraints switches are used to mark tests that
    # crash on certain platforms, so that they can be reactivated again
    # when the underlying problem is fixed.

    ConstraintInitializer pcCrash {expr {![testConstraint pc]}}
    ConstraintInitializer winCrash {expr {![testConstraint win]}}
    ConstraintInitializer macCrash {expr {![testConstraint mac]}}
    ConstraintInitializer unixCrash {expr {![testConstraint unix]}}

    # Skip empty tests

    ConstraintInitializer emptyTest {format 0}

    # By default, tests that expose known bugs are skipped.

    ConstraintInitializer knownBug {format 0}

    # By default, non-portable tests are skipped.

    ConstraintInitializer nonPortable {format 0}

    # Some tests require user interaction.

    ConstraintInitializer userInteraction {format 0}

    # Some tests must be skipped if the interpreter is not in
    # interactive mode

    ConstraintInitializer interactive \
⟦TAB⟧    {expr {[info exists ::tcl_interactive] && $::tcl_interactive}}

    # Some tests can only be run if the installation came from a CD
    # image instead of a web image.  Some tests must be skipped if you
    # are running as root on Unix.  Other tests can only be run if you
    # are running as root on Unix.

    ConstraintInitializer root {expr \
⟦TAB⟧    {[string equal unix $::tcl_platform(platform)]
⟦TAB⟧    && ([string equal root $::tcl_platform(user)]
⟦TAB⟧⟦TAB⟧|| [string equal "" $::tcl_platform(user)])}}
    ConstraintInitializer notRoot {expr {![testConstraint root]}}

    # Set nonBlockFiles constraint: 1 means this platform supports
    # setting files into nonblocking mode.

    ConstraintInitializer nonBlockFiles {
⟦TAB⟧    set code [expr {[catch {set f [open defs r]}]⟦SPACE⟧
⟦TAB⟧⟦TAB⟧    || [catch {fconfigure $f -blocking off}]}]
⟦TAB⟧    catch {close $f}
⟦TAB⟧    set code
    }

    # Set asyncPipeClose constraint: 1 means this platform supports
    # async flush and async close on a pipe.
    #
    # Test for SCO Unix - cannot run async flushing tests because a
    # potential problem with select is apparently interfering.
    # (Mark Diekhans).

    ConstraintInitializer asyncPipeClose {expr {
⟦TAB⟧    !([string equal unix $::tcl_platform(platform)]⟦SPACE⟧
⟦TAB⟧    && ([catch {exec uname -X | fgrep {Release = 3.2v}}] == 0))}}

    # Test to see if we have a broken version of sprintf with respect
    # to the "e" format of floating-point numbers.

    ConstraintInitializer eformat {string equal [format %g 5e-5] 5e-05}

    # Test to see if execed commands such as cat, echo, rm and so forth
    # are present on this machine.

    ConstraintInitializer unixExecs {
⟦TAB⟧set code 1
        if {[string equal macintosh $::tcl_platform(platform)]} {
⟦TAB⟧    set code 0
        }
        if {[string equal windows $::tcl_platform(platform)]} {
⟦TAB⟧    if {[catch {
⟦TAB⟧        set file _tcl_test_remove_me.txt
⟦TAB⟧        makeFile {hello} $file
⟦TAB⟧    }]} {
⟦TAB⟧        set code 0
⟦TAB⟧    } elseif {
⟦TAB⟧        [catch {exec cat $file}] ||
⟦TAB⟧        [catch {exec echo hello}] ||
⟦TAB⟧        [catch {exec sh -c echo hello}] ||
⟦TAB⟧        [catch {exec wc $file}] ||
⟦TAB⟧        [catch {exec sleep 1}] ||
⟦TAB⟧        [catch {exec echo abc > $file}] ||
⟦TAB⟧        [catch {exec chmod 644 $file}] ||
⟦TAB⟧        [catch {exec rm $file}] ||
⟦TAB⟧        [llength [auto_execok mkdir]] == 0 ||
⟦TAB⟧        [llength [auto_execok fgrep]] == 0 ||
⟦TAB⟧        [llength [auto_execok grep]] == 0 ||
⟦TAB⟧        [llength [auto_execok ps]] == 0
⟦TAB⟧    } {
⟦TAB⟧        set code 0
⟦TAB⟧    }
⟦TAB⟧    removeFile $file
        }
⟦TAB⟧set code
    }

    ConstraintInitializer stdio {
⟦TAB⟧set code 0
⟦TAB⟧if {![catch {set f [open "|[list [interpreter]]" w]}]} {
⟦TAB⟧    if {![catch {puts $f exit}]} {
⟦TAB⟧⟦TAB⟧if {![catch {close $f}]} {
⟦TAB⟧⟦TAB⟧    set code 1
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set code
    }

    # Deliberately call socket with the wrong number of arguments.  The
    # error message you get will indicate whether sockets are available
    # on this system.

    ConstraintInitializer socket {
⟦TAB⟧catch {socket} msg
⟦TAB⟧string compare $msg "sockets are not available on this system"
    }

    # Check for internationalization
    ConstraintInitializer hasIsoLocale {
⟦TAB⟧if {[llength [info commands testlocale]] == 0} {
⟦TAB⟧    set code 0
⟦TAB⟧} else {
⟦TAB⟧    set code [string length [SetIso8859_1_Locale]]
⟦TAB⟧    RestoreLocale
⟦TAB⟧}
⟦TAB⟧set code
    }

}
#####################################################################

# Usage and command line arguments processing.

# tcltest::PrintUsageInfo
#
#⟦TAB⟧Prints out the usage information for package tcltest.  This can
#⟦TAB⟧be customized with the redefinition of [PrintUsageInfoHook].
#
# Arguments:
#⟦TAB⟧none
#
# Results:
#       none
#
# Side Effects:
#       none
proc tcltest::PrintUsageInfo {} {
    puts [Usage]
    PrintUsageInfoHook
}

proc tcltest::Usage { {option ""} } {
    variable Usage
    variable Verify
    if {[llength [info level 0]] == 1} {
⟦TAB⟧set msg "Usage: [file tail [info nameofexecutable]] script "
⟦TAB⟧append msg "?-help? ?flag value? ... \n"
⟦TAB⟧append msg "Available flags (and valid input values) are:"

⟦TAB⟧set max 0
⟦TAB⟧set allOpts [concat -help [Configure]]
⟦TAB⟧foreach opt $allOpts {
⟦TAB⟧    set foo [Usage $opt]
⟦TAB⟧    foreach [list x type($opt) usage($opt)] $foo break
⟦TAB⟧    set line($opt) "  $opt $type($opt)  "
⟦TAB⟧    set length($opt) [string length $line($opt)]
⟦TAB⟧    if {$length($opt) > $max} {set max $length($opt)}
⟦TAB⟧}
⟦TAB⟧set rest [expr {72 - $max}]
⟦TAB⟧foreach opt $allOpts {
⟦TAB⟧    append msg \n$line($opt)
⟦TAB⟧    append msg [string repeat " " [expr {$max - $length($opt)}]]
⟦TAB⟧    set u [string trim $usage($opt)]
⟦TAB⟧    catch {append u "  (default: \[[Configure $opt]])"}
⟦TAB⟧    regsub -all {\s*\n\s*} $u " " u
⟦TAB⟧    while {[string length $u] > $rest} {
⟦TAB⟧⟦TAB⟧set break [string wordstart $u $rest]
⟦TAB⟧⟦TAB⟧if {$break == 0} {
⟦TAB⟧⟦TAB⟧    set break [string wordend $u 0]
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧append msg [string range $u 0 [expr {$break - 1}]]
⟦TAB⟧⟦TAB⟧set u [string trim [string range $u $break end]]
⟦TAB⟧⟦TAB⟧append msg \n[string repeat " " $max]
⟦TAB⟧    }
⟦TAB⟧    append msg $u
⟦TAB⟧}
⟦TAB⟧return $msg\n
    } elseif {[string equal -help $option]} {
⟦TAB⟧return [list -help "" "Display this usage information."]
    } else {
⟦TAB⟧set type [lindex [info args $Verify($option)] 0]
⟦TAB⟧return [list $option $type $Usage($option)]
    }
}

# tcltest::ProcessFlags --
#
#⟦TAB⟧process command line arguments supplied in the flagArray - this
#⟦TAB⟧is called by processCmdLineArgs.  Modifies tcltest variables
#⟦TAB⟧according to the content of the flagArray.
#
# Arguments:
#⟦TAB⟧flagArray - array containing name/value pairs of flags
#
# Results:
#⟦TAB⟧sets tcltest variables according to their values as defined by
#       flagArray
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::ProcessFlags {flagArray} {
    # Process -help first
    if {[lsearch -exact $flagArray {-help}] != -1} {
⟦TAB⟧PrintUsageInfo
⟦TAB⟧exit 1
    }

    if {[llength $flagArray] == 0} {
⟦TAB⟧RemoveAutoConfigureTraces
    } else {
⟦TAB⟧set args $flagArray
⟦TAB⟧while {[llength $args]>1 && [catch {eval configure $args} msg]} {

⟦TAB⟧    # Something went wrong parsing $args for tcltest options
⟦TAB⟧    # Check whether the problem is "unknown option"
⟦TAB⟧    if {[regexp {^unknown option (\S+):} $msg -> option]} {
⟦TAB⟧⟦TAB⟧# Could be this is an option the Hook knows about
⟦TAB⟧⟦TAB⟧set moreOptions [processCmdLineArgsAddFlagsHook]
⟦TAB⟧⟦TAB⟧if {[lsearch -exact $moreOptions $option] == -1} {
⟦TAB⟧⟦TAB⟧    # Nope.  Report the error, including additional options,
⟦TAB⟧⟦TAB⟧    # but keep going
⟦TAB⟧⟦TAB⟧    if {[llength $moreOptions]} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧append msg ", "
⟦TAB⟧⟦TAB⟧⟦TAB⟧append msg [join [lrange $moreOptions 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧⟦TAB⟧append msg "or [lindex $moreOptions end]"
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    Warn $msg
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧# error is something other than "unknown option"
⟦TAB⟧⟦TAB⟧# notify user of the error; and exit
⟦TAB⟧⟦TAB⟧puts [errorChannel] $msg
⟦TAB⟧⟦TAB⟧exit 1
⟦TAB⟧    }

⟦TAB⟧    # To recover, find that unknown option and remove up to it.
⟦TAB⟧    # then retry
⟦TAB⟧    while {![string equal [lindex $args 0] $option]} {
⟦TAB⟧⟦TAB⟧set args [lrange $args 2 end]
⟦TAB⟧    }
⟦TAB⟧    set args [lrange $args 2 end]
⟦TAB⟧}
⟦TAB⟧if {[llength $args] == 1} {
⟦TAB⟧    puts [errorChannel] \
⟦TAB⟧⟦TAB⟧    "missing value for option [lindex $args 0]"
⟦TAB⟧    exit 1
⟦TAB⟧}
    }

    # Call the hook
    catch {
        array set flag $flagArray
        processCmdLineArgsHook [array get flag]
    }
    return
}

# tcltest::ProcessCmdLineArgs --
#
#       This procedure must be run after constraint initialization is
#⟦TAB⟧set up (by [DefineConstraintInitializers]) because some constraints
#⟦TAB⟧can be overridden.
#
#       Perform configuration according to the command-line options.
#
# Arguments:
#⟦TAB⟧none
#
# Results:
#⟦TAB⟧Sets the above-named variables in the tcltest namespace.
#
# Side Effects:
#       None.
#

proc tcltest::ProcessCmdLineArgs {} {
    variable originalEnv
    variable testConstraints

    # The "argv" var doesn't exist in some cases, so use {}.
    if {![info exists ::argv]} {
⟦TAB⟧ProcessFlags {}
    } else {
⟦TAB⟧ProcessFlags $::argv
    }

    # Spit out everything you know if we're at a debug level 2 or
    # greater
    DebugPuts 2 "Flags passed into tcltest:"
    if {[info exists ::env(TCLTEST_proc tcltest::CompareStrings {actual expected mode} {
    variable CustomMatch
    if {![info exists CustomMatch($mode)]} {
        return -code error "No matching command registered for `-match $mode'"
    }
    set match [namespace eval :: $CustomMatch($mode) [list $expected $actual]]
    if {[catch {expr {$match && $match}} result]} {
⟦TAB⟧return -code error "Invalid result from `-match $mode' command: $result"
    }
    return $match
}

# tcltest::customMatch --
#
#⟦TAB⟧registers a command to be called when a particular type of
#⟦TAB⟧matching is required.
#
# Arguments:
#⟦TAB⟧nickname - Keyword for the type of matching
#⟦TAB⟧cmd - Incomplete command that implements that type of matching
#⟦TAB⟧⟦TAB⟧when completed with expected string and actual string
#⟦TAB⟧⟦TAB⟧and then evaluated.
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#⟦TAB⟧Sets the variable tcltest::CustomMatch

proc tcltest::customMatch {mode script} {
    variable CustomMatch
    if {![info complete $script]} {
⟦TAB⟧return -code error \
⟦TAB⟧⟦TAB⟧"invalid customMatch script; can't evaluate after completion"
    }
    set CustomMatch($mode) $script
}

# tcltest::SubstArguments list
#
# This helper function takes in a list of words, then perform a
# substitution on the list as though each word in the list is a separate
# argument to the Tcl function.  For example, if this function is
# invoked as:
#
#      SubstArguments {$a {$a}}
#
# Then it is as though the function is invoked as:
#
#      SubstArguments $a {$a}
#
# This code is adapted from Paul Duffin's function "SplitIntoWords".
# The original function can be found  on:
#
#      http://purl.org/thecliff/tcl/wiki/8on of the test, to
#  ⟦TAB⟧⟦TAB⟧  ⟦TAB⟧help humans understand what it does.
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#       Just about anything is possible depending on the test.
#

proc tcltest::test {name description args} {
    global tcl_platform
    variable testLevel
    variable coreModTime
    DebugPuts 3 "test $name $args"
    DebugDo 1 {
⟦TAB⟧variable TestNames
⟦TAB⟧catch {
⟦TAB⟧    puts "test name '$name' re-usle join [temporaryDirectory] core-$name]"
⟦TAB⟧⟦TAB⟧catch {file rename -force \
⟦TAB⟧⟦TAB⟧    [file join [workingDirectory] core] \
⟦TAB⟧⟦TAB⟧    [file join [temporaryDirectory] core-$name]
⟦TAB⟧⟦TAB⟧} msg
⟦TAB⟧⟦TAB⟧if {[string length $msg]k if the answer matched the expected answer
    # Only check if we ran the body of the test (no setup failure)
    if {$setupFailure || $codeFailure} {
⟦TAB⟧set scriptFailure 0
    } elseif {[set scriptCompare [catch {
⟦TAB⟧CompareStrings $actualAnswer $result $match
    } scriptMatch]] == 0} {
⟦TAB⟧set scriptFailure [expr {!$scriptMatch}]
    } else {
⟦TAB⟧set scriptFailure 1
    }

    # if we didn't experience any failures, then we passed
    variable numTests
    if {!($setupFailure || $cleanupFailure || $coreFailure
⟦TAB⟧    || $outputFailure || $errorFailure || $codeFailure
⟦TAB⟧    || $scriptFailure)} {
⟦TAB⟧if {$testLevel == 1} {
⟦TAB⟧    incr numTests(Passed)
⟦TAB⟧    if {[IsVerbose pass]} {
⟦TAB⟧⟦TAB⟧puts [outputChannel] "++++ $name PASSED"
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧incr testLevel -1
⟦TAB⟧return
    }

    # We know the test failed, tally it...
    if {$testLevel == 1} {
⟦TAB⟧incr numTests(Failed)
    }

    # ... then report according to the type of failure
    variable currentFailure true
    if {![IsVerbose body]} {
⟦TAB⟧set body ""
    }⟦TAB⟧
    puts [outputChannel] "\n==== $name\
⟦TAB⟧    [string trim $description] FAILED"
    if {[string length $body]} {
⟦TAB⟧puts [outputChannel] "==== Contents of test case:"
⟦TAB⟧puts [outputChannel] $body
    }
    if {$setupFailure} {
⟦TAB⟧puts [outputChannel] "---- Test setup\
⟦TAB⟧⟦TAB⟧failed:\n$setupMsg"
⟦TAB⟧if {[info exists errorInfo(setup)]} {
⟦TAB⟧    puts [outputChannel] "---- errorInfo(setup): $errorInfo(setup)"
⟦TAB⟧    puts [outputChannel] "---- errorCode(setup): $errorCode(setup)"
⟦TAB⟧}
    }
    if {$scriptFailure} {
⟦TAB⟧if {$scriptCompare} {
⟦TAB⟧    puts [outputChannel] "---- Error testing result: $scriptMatch"
⟦TAB⟧} else {
⟦TAB⟧    puts [outputChannel] "---- Result was:\n$actualAnswer"
⟦TAB⟧    puts [outputChannel] "---- Result should have been\
⟦TAB⟧⟦TAB⟧    ($match matching):\n$result"
⟦TAB⟧}
    }
    if {$codeFailure} {
⟦TAB⟧switch -- $return= 1} {
⟦TAB⟧⟦TAB⟧incr numTests(Skipped)
⟦TAB⟧    }
⟦TAB⟧    return 1
⟦TAB⟧}
    } else {
⟦TAB⟧# "constraints" argument exists;
⟦TAB⟧# make sure that the constraints are satisfied.

⟦TAB⟧set doTest 0
⟦TAB⟧if {[string match {*[$\[]*} $constraints] != 0} {
⟦TAB⟧    # full expression, e.g. {$foo > [info tclversion]}
⟦TAB⟧    catch {set doTest [uplevel #0 expr $constraints]}
⟦TAB⟧} elseif {[regexp {[^.a-zA-Z0-9 \n\r\t]+} $constraints] != 0} {
⟦TAB⟧    # something like {a || b} should be turned into
⟦TAB⟧    # $testConstraints(a) || $testConstraints(b).
⟦TAB⟧    regsub -all {[.\w]+} $constraints {$testConstraints(&)} c
⟦TAB⟧    catch {set doTest [eval expr $c]}
⟦TAB⟧} elseif {![catch {llength $constraints}]} {
⟦TAB⟧    # just simple constraints such as {unixOnly fonts}.
⟦TAB⟧    set doTest 1
⟦TAB⟧    foreach constraint $constraints {
⟦TAB⟧⟦TAB⟧if {(![info exists testConstraints($constraint)]) \
⟦TAB⟧⟦TAB⟧⟦TAB⟧|| (!$testConstraints($constraint))} {
⟦TAB⟧⟦TAB⟧    set doTest 0

⟦TAB⟧⟦TAB⟧    # store the constraint that kept the test from
⟦TAB⟧⟦TAB⟧    # running
⟦TAB⟧⟦TAB⟧    set constraints $constraint
⟦TAB⟧⟦TAB⟧    break
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧
⟦TAB⟧if {$doTest == 0} {
⟦TAB⟧    if {[IsVerbose skip]} {
⟦TAB⟧⟦TAB⟧puts [outputChannel] "++++ $name SKIPPED: $constraints"
⟦TAB⟧    }

⟦TAB⟧    if {$testLevel == 1} {
⟦TAB⟧⟦TAB⟧incr numTests(Skipped)
⟦TAB⟧⟦TAB⟧AddToSkippedBecause $constraints
⟦TAB⟧    }
⟦TAB⟧    return 1
⟦TAB⟧}
    }
    return 0
}

# RunTest --
#
# This is where the body of a test is evaluated.  The combination of
# [RunTest] and [Eval] allows the output and error output of the test
# body to be captured for comparison against the expected values.

proc tcltest::RunTest {name script} {
    DebugPuts 3 "Running $name {$script}"

    # If there is no "memory" command (because memory debugging isn't
    # enabled), then don't attempt to use the command.

    if {[llength [info commands memory]] == 1} {
⟦TAB⟧memory tag $name
    }

    set code [catch {uplevel 1 $script} actualAnswer]

    return [list $actualAnswer $code]
}

#####################################################################

# tcltest::cleanupTestsHook --
#
#⟦TAB⟧This hook allows a harness that builds upon tcltest to specify
#       additional things that should be done at cleanup.
#

if {[llength [info commands tcltest::cleanupTestsHook]] == 0} {
    proc tcltest::cleanupTestsHook {} {}
}

# tcltest::cleanupTests --
#
# Remove files and dirs created using the makeFile and makeDirectory
# commands since the last time this proc was invoked.
#
# Print the names of the files created without the makeFile command
# since the tests were invoked.
#
# Print the number tests (total, passed, failed, and skipped) since the
# tests were invoked.
#
# Restore original environment (as reported by special variable env).
#
# Arguments:
#      calledFromAllFile - if 0, behave as if we are running a single
#      test file within an entire suite of tests.  if we aren't running
#      a single test file, then don't report status.  check for new
#      files created during the test run and report on them.  if 1,
#      report collated status from all the test file runs.
#
# Results:
#      None.
#
# Side Effects:
#      None
#

proc tcltest::cleanupTests {{calledFromAllFile 0}} {
    variable filesMade
    variable filesExisted
    variable createdNewFiles
    variable testSingleFile
    variable numTests
    variable numTestFiles
    variable failFiles
    variable skippedBecause
    variable currentFailure
    variable originalEnv
    variable originalTclPlatform
    variable coreModTime

    FillFilesExisted
    set testFileName [file tail [info script]]

    # Call the cleanup hook
    cleanupTestsHook

    # Remove files and directories created by the makeFile and
    # makeDirectory procedures.  Record the names of files in
    # workingDirectory that were not pre-existing, and associate them
    # with the test file that created them.

    if {!$calledFromAllFile} {
⟦TAB⟧foreach file $filesMade {
⟦TAB⟧    if {[file exists $file]} {
⟦TAB⟧⟦TAB⟧DebugDo 1 {Warn "cleanupTests deleting $file..."}
⟦TAB⟧⟦TAB⟧catch {file delete -force $file}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set currentFiles {}
⟦TAB⟧foreach file [glob -nocomplain \
⟦TAB⟧⟦TAB⟧-directory [temporaryDirectory] *] {
⟦TAB⟧    lappend currentFiles [file tail $file]
⟦TAB⟧}
⟦TAB⟧set newFiles {}
⟦TAB⟧foreach file $currentFiles {
⟦TAB⟧    if {[lsearch -exact $filesExisted $file] == -1} {
⟦TAB⟧⟦TAB⟧lappend newFiles $file
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set filesExisted $currentFiles
⟦TAB⟧if {[llength $newFiles] > 0} {
⟦TAB⟧    set createdNewFiles($testFileName) $newFiles
⟦TAB⟧}
    }

    if {$calledFromAllFile || $testSingleFile} {

⟦TAB⟧# print stats

⟦TAB⟧puts -nonewline [outputChannel] "$testFileName:"
⟦TAB⟧foreach index [list "Total" "Passed" "Skipped" "Failed"] {
⟦TAB⟧    puts -nonewline [outputChannel] \
⟦TAB⟧⟦TAB⟧    "\t$index\t$numTests($index)"
⟦TAB⟧}
⟦TAB⟧puts [outputChannel] ""

⟦TAB⟧# print number test files sourced
⟦TAB⟧# print names of files that ran tests which failed

⟦TAB⟧if {$calledFromAllFile} {
⟦TAB⟧    puts [outputChannel] \
⟦TAB⟧⟦TAB⟧    "Sourced $numTestFiles Test Files."
⟦TAB⟧    set numTestFiles 0
⟦TAB⟧    if {[llength $failFiles] > 0} {
⟦TAB⟧⟦TAB⟧puts [outputChannel] \
⟦TAB⟧⟦TAB⟧⟦TAB⟧"Files with failing tests: $failFiles"
⟦TAB⟧⟦TAB⟧set failFiles {}
⟦TAB⟧    }
⟦TAB⟧}

⟦TAB⟧# if any tests were skipped, print the constraints that kept
⟦TAB⟧# them from running.

⟦TAB⟧set constraintList [array names skippedBecause]
⟦TAB⟧if {[llength $constraintList] > 0} {
⟦TAB⟧    puts [outputChannel] \
⟦TAB⟧⟦TAB⟧    "Number of tests skipped for each constraint:"
⟦TAB⟧    foreach constraint [lsort $constraintList] {
⟦TAB⟧⟦TAB⟧puts [outputChannel] \
⟦TAB⟧⟦TAB⟧⟦TAB⟧"\t$skippedBecause($constraint)\t$constraint"
⟦TAB⟧⟦TAB⟧unset skippedBecause($constraint)
⟦TAB⟧    }
⟦TAB⟧}

⟦TAB⟧# report the names of test files in createdNewFiles, and reset
⟦TAB⟧# the array to be empty.

⟦TAB⟧set testFilesThatTurded [lsort [array names createdNewFiles]]
⟦TAB⟧if {[llength $testFilesThatTurded] > 0} {
⟦TAB⟧    puts [outputChannel] "Warning: files left behind:"
⟦TAB⟧    foreach testFile $testFilesThatTurded {
⟦TAB⟧⟦TAB⟧puts [outputChannel] \
⟦TAB⟧⟦TAB⟧⟦TAB⟧"\t$testFile:\t$createdNewFiles($testFile)"
⟦TAB⟧⟦TAB⟧unset createdNewFiles($testFile)
⟦TAB⟧    }
⟦TAB⟧}

⟦TAB⟧# reset filesMade, filesExisted, and numTests

⟦TAB⟧set filesMade {}
⟦TAB⟧foreach index [list "Total" "Passed" "Skipped" "Failed"] {
⟦TAB⟧    set numTests($index) 0
⟦TAB⟧}

⟦TAB⟧# exit only if running Tk in non-interactive mode
⟦TAB⟧# This should be changed to determine if an event
⟦TAB⟧# loop is running, which is the real issue.
⟦TAB⟧# Actually, this doesn't belong here at all.  A package
⟦TAB⟧# really has no business [exit]-ing an application.
⟦TAB⟧if {![catch {package present Tk}] && ![testConstraint interactive]} {
⟦TAB⟧    exit
⟦TAB⟧}
    } else {

⟦TAB⟧# if we're deferring stat-reporting until all files are sourced,
⟦TAB⟧# then add current file to failFile list if any tests in this
⟦TAB⟧# file failed

⟦TAB⟧if {$currentFailure \
⟦TAB⟧⟦TAB⟧&& ([lsearch -exact $failFiles $testFileName] == -1)} {
⟦TAB⟧    lappend failFiles $testFileName
⟦TAB⟧}
⟦TAB⟧set currentFailure false

⟦TAB⟧# restore the environment to the state it was in before this package
⟦TAB⟧# was loaded

⟦TAB⟧set newEnv {}
⟦TAB⟧set changedEnv {}
⟦TAB⟧set removedEnv {}
⟦TAB⟧foreach index [array names ::env] {
⟦TAB⟧    if {![info exists originalEnv($index)]} {
⟦TAB⟧⟦TAB⟧lappend newEnv $index
⟦TAB⟧⟦TAB⟧unset ::env($index)
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧if {$::env($index) != $originalEnv($index)} {
⟦TAB⟧⟦TAB⟧    lappend changedEnv $index
⟦TAB⟧⟦TAB⟧    set ::env($index) $originalEnv($index)
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧foreach index [array names originalEnv] {
⟦TAB⟧    if {![info exists ::env($index)]} {
⟦TAB⟧⟦TAB⟧lappend removedEnv $index
⟦TAB⟧⟦TAB⟧set ::env($index) $originalEnv($index)
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧if {[llength $newEnv] > 0} {
⟦TAB⟧    puts [outputChannel] \
⟦TAB⟧⟦TAB⟧    "env array elements created:\t$newEnv"
⟦TAB⟧}
⟦TAB⟧if {[llength $changedEnv] > 0} {
⟦TAB⟧    puts [outputChannel] \
⟦TAB⟧⟦TAB⟧    "env array elements changed:\t$changedEnv"
⟦TAB⟧}
⟦TAB⟧if {[llength $removedEnv] > 0} {
⟦TAB⟧    puts [outputChannel] \
⟦TAB⟧⟦TAB⟧    "env array elements removed:\t$removedEnv"
⟦TAB⟧}

⟦TAB⟧set changedTclPlatform {}
⟦TAB⟧foreach index [array names originalTclPlatform] {
⟦TAB⟧    if {$::tcl_platform($index) \
⟦TAB⟧⟦TAB⟧    != $originalTclPlatform($index)} {
⟦TAB⟧⟦TAB⟧lappend changedTclPlatform $index
⟦TAB⟧⟦TAB⟧set ::tcl_platform($index) $originalTclPlatform($index)
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧if {[llength $changedTclPlatform] > 0} {
⟦TAB⟧    puts [outputChannel] "tcl_platform array elements\
⟦TAB⟧⟦TAB⟧    changed:\t$changedTclPlatform"
⟦TAB⟧}

⟦TAB⟧if {[file exists [file join [workingDirectory] core]]} {
⟦TAB⟧    if {[preserveCore] > 1} {
⟦TAB⟧⟦TAB⟧puts "rename core file (> 1)"
⟦TAB⟧⟦TAB⟧puts [outputChannel] "produced core file! \
⟦TAB⟧⟦TAB⟧⟦TAB⟧Moving file to: \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [temporaryDirectory] core-$testFileName]"
⟦TAB⟧⟦TAB⟧catch {file rename -force \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [workingDirectory] core] \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [temporaryDirectory] core-$testFileName]
⟦TAB⟧⟦TAB⟧} msg
⟦TAB⟧⟦TAB⟧if {[string length $msg] > 0} {
⟦TAB⟧⟦TAB⟧    PrintError "Problem renaming file: $msg"
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧# Print a message if there is a core file and (1) there
⟦TAB⟧⟦TAB⟧# previously wasn't one or (2) the new one is different
⟦TAB⟧⟦TAB⟧# from the old one.

⟦TAB⟧⟦TAB⟧if {[info exists coreModTime]} {
⟦TAB⟧⟦TAB⟧    if {$coreModTime != [file mtime \
⟦TAB⟧⟦TAB⟧⟦TAB⟧    [file join [workingDirectory] core]]} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧puts [outputChannel] "A core file was created!"
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧} else {
⟦TAB⟧⟦TAB⟧    puts [outputChannel] "A core file was created!"
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
    }
    flush [outputChannel]
    flush [errorChannel]
    return
}

#####################################################################

# Procs that determine which tests/test files to run

# tcltest::GetMatchingFiles
#
#       Looks at the patterns given to match and skip files and uses
#⟦TAB⟧them to put together a list of the tests that will be run.
#
# Arguments:
#       directory to search
#
# Results:
#       The constructed list is returned to the user.  This will
#⟦TAB⟧primarily be used in 'all.tcl' files.  It is used in
#⟦TAB⟧runAllTests.
#
# Side Effects:
#       None

# a lower case version is needed for compatibility with tcltest 1.0
proc tcltest::getMatchingFiles args {eval GetMatchingFiles $args}

proc tcltest::GetMatchingFiles { args } {
    if {[llength $args]} {
⟦TAB⟧set dirList $args
    } else {
⟦TAB⟧# Finding tests only in [testsDirectory] is normal operation.
⟦TAB⟧# This procedure is written to accept multiple directory arguments
⟦TAB⟧# only to satisfy version 1 compatibility.
⟦TAB⟧set dirList [list [testsDirectory]]
    }

    set matchingFiles [list]
    foreach directory $dirList {

⟦TAB⟧# List files in $directory that match patterns to run.
⟦TAB⟧set matchFileList [list]
⟦TAB⟧foreach match [    foreach file [lsort [GetMatchingFiles]] {
⟦TAB⟧set tail [file tail $file]
⟦TAB⟧puts [outputChannel] $tail
⟦TAB⟧flush [outputChannel]

⟦TAB⟧if {[singleProcess]} {
⟦TAB⟧    incr numTestFiles
⟦TAB⟧    uplevel 1 [list ::sour childargv [list]
⟦TAB⟧    foreach opt [Configure] {
⟦TAB⟧⟦TAB⟧if {[string equal $opt -outfile]} {continue}
⟦TAB⟧⟦TAB⟧lappend childargv $opt [Configure $opt]
⟦TAB⟧    }
⟦TAB⟧    set cmd [linsert $childargv 0 | $shell $file]
⟦TAB⟧    if {[catch {
⟦TAB⟧⟦TAB⟧incr numTestFiles
⟦TAB⟧⟦TAB⟧set pipeFd [open $cmd "r"]
⟦TAB⟧⟦TAB⟧while {[gets $pipeFd line] >= 0} {
⟦TAB⟧⟦TAB⟧    if {[regexp [join {
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {^([^:]+):\t}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {Total\t([0-9]+)\t}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {Passed\t([0-9]+)\t}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {Skipped\t([0-9]+)\t}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {Failed\t([0-9]+)}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    } ""] $line null testFile \
⟦TAB⟧⟦TAB⟧⟦TAB⟧    Total Passed Skipped Failed]} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧foreach index {Total Passed Skipped Failed} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧    incr numTests($index) [set $index]
⟦TAB⟧⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧⟦TAB⟧if {$Failed > 0} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧    lappend failFiles $testFile
⟦TAB⟧⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧    } elseif {[regexp [join {
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {^Number of tests skipped }
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {for each constraint:}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {|^\t(\d+)\t(.+)$}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    } ""] $line match skipped constraint]} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧if {[string match \t* $match]} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧    AddToSkippedBecause $constraint $skipped
⟦TAB⟧⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧⟦TAB⟧puts [outputChannel] $line
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧close $pipeFd
⟦TAB⟧    } msg]} {
⟦TAB⟧⟦TAB⟧puts [outputChannel] "Test file error: $msg"
⟦TAB⟧⟦TAB⟧# append the name of the test to a list to be reported
⟦TAB⟧⟦TAB⟧# later
⟦TAB⟧⟦TAB⟧lappend testFileFailures $file
⟦TAB⟧    }
⟦TAB⟧}
    }
oreach file $testFileFailures {
⟦TAB⟧    puts [outputChannel] "  [file tail $file]\n"
⟦TAB⟧}
    }

    # Checking for subdirectories in which to run tests
    foreach directory [GetMatchingDirectories [testsDirectory]] {
⟦TAB⟧set dir [file tail $directory]
⟦TAB⟧puts [outputChannel] [string repeat ~ 44]
⟦TAB⟧puts [outputChannel] "$dir test began at [eval $timeCmd]\n"
⟦TAB⟧
⟦TAB⟧uplevel 1 [list ::source [file join $directory all.tcl]]
⟦TAB⟧
⟦TAB⟧set endTime [eval $timeCmd]
⟦TAB⟧puts [outputChannel] "\n$dir test ended at $endTime"
⟦TAB⟧puts [outputChannel] ""
⟦TAB⟧puts [outputChannel] [string repeat ~ 44]
    }
    return
}

#################st in the saveState variable.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::restoreState {} {
    variable saveState
    foreach p [uplevel 1 {::info procs}] {
⟦TAB⟧if {([lsearch [lindex $saveState 0] $p] < 0)e $p {}]]
⟦TAB⟧}
    }
    foreach p [uplevel 1 {::info vars}] {
⟦TAB⟧if {[lsearch [lindex $saveState 1] $p] < 0} {
⟦TAB⟧    DebugPuts 2 "[lindex [info level 0] 0]:\
⟦TAB⟧⟦TAB⟧    Removing variable $p"
⟦TAB⟧    uplevel 1 [list ::catch [list ::unset $p]]
⟦TAB⟧}
    }
    return
}

# tcltest::normalizeMsg --
#
#⟦TAB⟧Removes "extra" newlines from a string.
#
# Arguments:
#⟦TAB⟧msg        String to be modified
#
# Results:
#⟦TAB⟧string with extra newlines removed
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::normalizeMsg {msg} {
    regsub "\n$" [string tolower $msg] "" msg
    set msg [string map [list "\n\n" "\n"] $msg]
    return [string map [list "\n\}" "\}"] $msg]
}

# tcltest::makeFile --
#
# Create a new file with the name <name>, and write <contents> to it.
#
# If this file hasn't been created via makeFile since the last time
# cleanupTests was called, add it to the $filesMade list, so it will be
# removed by the next call to cleanupTests.
#
# Arguments:
#⟦TAB⟧contents        content of the new file
#       name            name of the new file
#       directory       directory name for new file
#
# Results:
#⟦TAB⟧absolute path to the file created
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::makeFile {contents name {directory ""}} {
    variable filesMade
    FillFilesExisted

    if {[llength [info level 0]] == 3} {
⟦TAB⟧set directory [temporaryDirectory]
    }

    set fullName [file join $directory $name]

    DebugPuts 3 "[lindex [info level 0] 0]:\
⟦TAB⟧     putting ``$contents'' into $fullName"

    set fd [open $fullName w]
    fconfigure $fd -translation lf
    if {[string equal [string index $contents end] \n]} {
⟦TAB⟧puts -nonewline $fd $contents
    } else {
⟦TAB⟧puts $fd $contents
    }
    close $fd

    if {[lsearch -exact $filesMade $fullName] == -1} {
⟦TAB⟧lappend filesMade $fullName
    }
    return $fullName
}

# tcltest::re sequence of bytes "\xc3\xa0\xc0\x80".
#
# Generally, it's a bad idea to examine the bytes in a Tcl string or to
# construct improperly formed strings in this manner, because it involves
# exposing that Tcl uses UTF-8 internally.
#
# Arguments:
#⟦TAB⟧string being converted
#
# Results:
#⟦TAB⟧result fom encoding
#
# Side effects:
#⟦TAB⟧None

proc tcltest::bytestring {string} {
    return [encoding convertfrom identity $string]
}

# tcltest::OpenFiles --
#
#⟦TAB⟧used in io tests, uses testchannel
#
# Arguments:
#⟦TAB⟧None.
#
# Results:
#⟦TAB⟧???
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::OpenFiles {} {
    if {[catch {testchannel open} result]} {
⟦TAB⟧return {}
    }
    return $result
}

# tcltest::LeakFiles --
#
#⟦TAB⟧used in io tests, uses testchannel
#
# Arguments:
#⟦TAB⟧None.
#
# Results:
#⟦TAB⟧???
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::LeakFiles {old} {
    if {[catch {testchannel open} new]} {
⟦TAB⟧return {}
    }
    set leak {}
    foreach p $new {
⟦TAB⟧if {[lsearch $old $p] < 0} {
⟦TAB⟧    lappend leak $p
⟦TAB⟧}
    }
    return $leak
}

#
# Internationalization / ISO support procs     -- dl
#

# tcltest::SetIso8859_1_Locale --
#
#⟦TAB⟧used in cmdIL.test, uses testlocale
#
# Arguments:
#⟦TAB⟧None.
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::SetIso8859_1_Locale {} {
    variable previousLocale
    variable isoLocale
    if {[info commands testlocale] != ""} {
⟦TAB⟧set previousLocale [testlocale ctype]
⟦TAB⟧testlocale ctype $isoLocale
    }
    return
}

# tcltest::RestoreLocale --
#
#⟦TAB⟧used in cmdIL.test, uses testlocale
#
# Arguments:
#⟦TAB⟧None.
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::RestoreLocale {} {
    variable previousLocale
    if {[info commands testlocale] != ""} {
⟦TAB⟧testlocale ctype $previousLocale
    }
    return
}

# tcltest::threadReap --
#
#⟦TAB⟧Kill all threads except for the main thread.
#⟦TAB⟧Do nothing if testthread is not defined.
#
# Arguments:
#⟦TAB⟧none.
#
# Results:
#⟦TAB⟧Returns the number of existing threads.
#
# Side Effects:
#       none.
#

proc tcltest::threadReap {} {
    if {[in⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 0, result:

```text-visible
child-provider {8.4.6 8.4 ::} module-code 1 module-result {invalid command name "pwd"} require-code 1 require-result {can't find package tcltest}⟦EOF⟧
```

- real-harness-explicit-source-child, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- real-harness-explicit-source-child: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 13

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
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

    # tests that use threads need to know which is the main thread
    Default mainThread 1
    variable mainThread
    if {[info commands thread::id] != {}} {
⟦TAB⟧set mainThread [thread::id]
    } elseif {[info commands testthread] != {}} {
⟦TAB⟧set mainThread [testthread id]
    }

    # Set workingDirectory to [pwd]. The default output directory for
    # Tcl tests is the working directory.  Whenever this value changes
    # change to that directory.
    variable workingDirectory
    trace variable workingDirectory w \
⟦TAB⟧    [namespace code {cd $workingDirectory ;#}]

    Default workingDirectory [pwd] AcceptAbsolutePath
    proc workingDirectory { {dir ""} } {
⟦TAB⟧variable workingDirectory
⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $workingDirectory
⟦TAB⟧}
⟦TAB⟧set workingDirectory [AcceptAbsolutePath $dir]
    }

    # Set the location of the execuatble
    Default tcltest [info nameofexecutable]
    trace variable tcltest w [namespace code {testConstraint stdio \
⟦TAB⟧    [eval [ConstraintInitializer stdio]] ;#}]

    # save the platform information so it can be restored later
    Default originalTclPlatform [array get ::tcl_platform]

    # If a core file exists, save its modification time.
    if {[file exists [file join [workingDirectory] core]]} {
⟦TAB⟧Default coreModTime \
⟦TAB⟧⟦TAB⟧[file mtime [file join [workingDirectory] core]]
    }

    # stdout and stderr buffers for use when we want to store them
    Default outData {}
    Default errData {}

    # keep track of test level for nested test commands
    variable testLevel 0

    # the variables and procs that existed when saveState was called are
    # stored in a variable of the same name
    Default saveState {}

    # Internationalization support -- used in [SetIso8859_1_Locale] and
    # [RestoreLocale]. Those commands are used in cmdIL.test.

    if {![info exists [namespace current]::isoLocale]} {
⟦TAB⟧variable isoLocale fr
⟦TAB⟧switch -- $::tcl_platform(platform) {
⟦TAB⟧    "unix" {

⟦TAB⟧⟦TAB⟧# Try some 'known' values for some platforms:

⟦TAB⟧⟦TAB⟧switch -exact -- $::tcl_platform(os) {
⟦TAB⟧⟦TAB⟧    "FreeBSD" {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr_FR.ISO_8859-1
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    HP-UX {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr_FR.iso88591
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    Linux -
⟦TAB⟧⟦TAB⟧    IRIX {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    default {

⟦TAB⟧⟦TAB⟧⟦TAB⟧# Works on SunOS 4 and Solaris, and maybe
⟦TAB⟧⟦TAB⟧⟦TAB⟧# others...  Define it to something else on your
⟦TAB⟧⟦TAB⟧⟦TAB⟧# system if you want to test those.

⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale iso_8859_1
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧    "windows" {
⟦TAB⟧⟦TAB⟧set isoLocale French
⟦TAB⟧    }
⟦TAB⟧}
    }

    variable ChannelsWeOpened; array set ChannelsWeOpened {}
    # output goes to stdout by default
    Default outputChannel stdout
    proc outputChannel { {filename ""} } {
⟦TAB⟧variable outputChannel
⟦TAB⟧variable ChannelsWeOpened

⟦TAB⟧# This is very subtle and tricky, so let me try to explain.
⟦TAB⟧# (Hopefully this longer comment will be clear when I come
⟦TAB⟧# back in a few months, unlike its predecessor :) )
⟦TAB⟧#⟦SPACE⟧
⟦TAB⟧# The [outputChannel] command (and underlying variable) have to
⟦TAB⟧# be kept in sync with the [configure -outfile] configuration
⟦TAB⟧# option ( and underlying variable Option(-outfile) ).  This is
⟦TAB⟧# accomplished with a write trace on Option(-outfile) that will
⟦TAB⟧# update [outputChannel] whenver a new value is written.  That
⟦TAB⟧# much is easy.
⟦TAB⟧#
⟦TAB⟧# The trick is that in order to maintain compatibility with
⟦TAB⟧# version 1 of tcltest, we must allow every configuration option
⟦TAB⟧# to get its inital value from command line arguments.  This is
⟦TAB⟧# accomplished by setting initial read traces on all the
⟦TAB⟧# configuration options to parse the command line option the first
⟦TAB⟧# time they are read.  These traces are cancelled whenever the
⟦TAB⟧# program itself calls [configure].
⟦TAB⟧#⟦SPACE⟧
⟦TAB⟧# OK, then so to support tcltest 1 compatibility, it seems we want
⟦TAB⟧# to get the return from [outputFile] to trigger the read traces,
⟦TAB⟧# just in case.
⟦TAB⟧#
⟦TAB⟧# BUT!  A little known feature of Tcl variable traces is that⟦SPACE⟧
⟦TAB⟧# traces are disabled during the handling of other traces.  So,
⟦TAB⟧# if we trigger read traces on Option(-outfile) and that triggers
⟦TAB⟧# command line parsing which turns around and sets an initial
⟦TAB⟧# value for Option(-outfile) -- <whew!> -- the write trace that
⟦TAB⟧# would keep [outputChannel] in sync with that new initial value
⟦TAB⟧# would not fire!
⟦TAB⟧#
⟦TAB⟧# SO, finally, as a workaround, instead of triggering read traces
⟦TAB⟧# by invoking [outputFile], we instead trigger the same set of
⟦TAB⟧# read traces by invoking [debug].  Any command that reads a
⟦TAB⟧# configuration option would do.  [debug] is just a handy one.
⟦TAB⟧# The end result is that we support tcltest 1 compatibility and
⟦TAB⟧# keep outputChannel and -outfile in sync in all cases.
⟦TAB⟧debug

⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $outputChannel
⟦TAB⟧}
⟦TAB⟧if {[info exists ChannelsWeOpened($outputChannel)]} {
⟦TAB⟧    close $outputChannel
⟦TAB⟧    unset ChannelsWeOpened($outputChannel)
⟦TAB⟧}
⟦TAB⟧switch -exact -- $filename {
⟦TAB⟧    stderr -
⟦TAB⟧    stdout {
⟦TAB⟧⟦TAB⟧set outputChannel $filename
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧set outputChannel [open $filename a]
⟦TAB⟧⟦TAB⟧set ChannelsWeOpened($outputChannel) 1

⟦TAB⟧⟦TAB⟧# If we created the file in [temporaryDirectory], then
⟦TAB⟧⟦TAB⟧# [cleanupTests] will delete it, unless we claim it was
⟦TAB⟧⟦TAB⟧# already there.
⟦TAB⟧⟦TAB⟧set outdir [normalizePath [file dirname \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [pwd] $filename]]]
⟦TAB⟧⟦TAB⟧if {[string equal $outdir [temporaryDirectory]]} {
⟦TAB⟧⟦TAB⟧    variable filesExisted
⟦TAB⟧⟦TAB⟧    FillFilesExisted
⟦TAB⟧⟦TAB⟧    set filename [file tail $filename]
⟦TAB⟧⟦TAB⟧    if {[lsearch -exact $filesExisted $filename] == -1} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧lappend filesExisted $filename
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $outputChannel
    }

    # errors go to stderr by default
    Default errorChannel stderr
    proc errorChannel { {filename ""} } {
⟦TAB⟧variable errorChannel
⟦TAB⟧variable ChannelsWeOpened

⟦TAB⟧# This is subtle and tricky.  See the comment above in
⟦TAB⟧# [outputChannel] for a detailed explanation.
⟦TAB⟧debug

⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $errorChannel
⟦TAB⟧}
⟦TAB⟧if {[info exists ChannelsWeOpened($errorChannel)]} {
⟦TAB⟧    close $errorChannel
⟦TAB⟧    unset ChannelsWeOpened($errorChannel)
⟦TAB⟧}
⟦TAB⟧switch -exact -- $filename {
⟦TAB⟧    stderr -
⟦TAB⟧    stdout {
⟦TAB⟧⟦TAB⟧set errorChannel $filename
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧set errorChannel [open $filename a]
⟦TAB⟧⟦TAB⟧set ChannelsWeOpened($errorChannel) 1

⟦TAB⟧⟦TAB⟧# If we created the file in [temporaryDirectory], then
⟦TAB⟧⟦TAB⟧# [cleanupTests] will delete it, unless we claim it was
⟦TAB⟧⟦TAB⟧# already there.
⟦TAB⟧⟦TAB⟧set outdir [normalizePath [file dirname \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [pwd] $filename]]]
⟦TAB⟧⟦TAB⟧if {[string equal $outdir [temporaryDirectory]]} {
⟦TAB⟧⟦TAB⟧    variable filesExisted
⟦TAB⟧⟦TAB⟧    FillFilesExisted
⟦TAB⟧⟦TAB⟧    set filename [file tail $filename]
⟦TAB⟧⟦TAB⟧    if {[lsearch -exact $filesExisted $filename] == -1} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧lappend filesExisted $filename
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $errorChannel
    }

##### Set up the configurable options
    #
    # The configurable options of the package
    variable Option; array set Option {}

    # Usage strings for those options
    variable Usage; array set Usage {}

    # Verification commands for those options
    variable Verify; array set Verify {}

    # Initialize the default values of the configurable options that are
    # historically associated with an exported variable.  If that variable
    # is already set, support compatibility by accepting its pre-set value.
    # Use [trace] to establish ongoing connection between the deprecated
    # exported variable and the modern option kept as a true internal var.
    # Also set up usage string and value testing for the option.
    proc Option {option value usage {verify AcceptAll} {varName {}}} {
⟦TAB⟧variable Option
⟦TAB⟧variable Verify
⟦TAB⟧variable Usage
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧set Usage($option) $usage
⟦TAB⟧set Verify($option) $verify
⟦TAB⟧if {[catch {$verify $value} msg]} {
⟦TAB⟧    return -code error $msg
⟦TAB⟧} else {
⟦TAB⟧    set Option($option) $msg
⟦TAB⟧}
⟦TAB⟧if {[string length $varName]} {
⟦TAB⟧    variable $varName
⟦TAB⟧    if {[info exists $varName]} {
⟦TAB⟧⟦TAB⟧if {[catch {$verify [set $varName]} msg]} {
⟦TAB⟧⟦TAB⟧    return -code error $msg
⟦TAB⟧⟦TAB⟧} else {
⟦TAB⟧⟦TAB⟧    set Option($option) $msg
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧unset $varName
⟦TAB⟧    }
⟦TAB⟧    namespace eval [namespace current] \
⟦TAB⟧    ⟦TAB⟧    [list upvar 0 Option($option) $varName]
⟦TAB⟧    # Workaround for Bug (now Feature Request) 572889.  Grrrr....
⟦TAB⟧    # Track all the variables tied to options
⟦TAB⟧    lappend OptionControlledVariables $varName
⟦TAB⟧    # Later, set auto-configure read traces on all
⟦TAB⟧    # of them, since a single trace on Option does not work.
⟦TAB⟧    proc $varName {{value {}}} [subst -nocommands {
⟦TAB⟧⟦TAB⟧if {[llength [info level 0]] == 2} {
⟦TAB⟧⟦TAB⟧    Configure $option [set value]
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧return [Configure $option]
⟦TAB⟧    }]
⟦TAB⟧}
    }

    proc MatchingOption {option} {
⟦TAB⟧variable Option
⟦TAB⟧set match [array names Option $option*]
⟦TAB⟧switch -- [llength $match] {
⟦TAB⟧    0 {
⟦TAB⟧⟦TAB⟧set sorted [lsort [array names Option]]
⟦TAB⟧⟦TAB⟧set values [join [lrange $sorted 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append values ", or [lindex $sorted end]"
⟦TAB⟧⟦TAB⟧return -code error "unknown option $option: should be\
⟦TAB⟧⟦TAB⟧⟦TAB⟧one of $values"
⟦TAB⟧    }
⟦TAB⟧    1 {
⟦TAB⟧⟦TAB⟧return [lindex $match 0]
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧# Exact match trumps ambiguity
⟦TAB⟧⟦TAB⟧if {[lsearch -exact $match $option] >= 0} {
⟦TAB⟧⟦TAB⟧    return $option
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧set values [join [lrange $match 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append values ", or [lindex $match end]"
⟦TAB⟧⟦TAB⟧return -code error "ambiguous option $option:\
⟦TAB⟧⟦TAB⟧⟦TAB⟧could match $values"
⟦TAB⟧    }
⟦TAB⟧}
    }

    proc EstablishAutoConfigureTraces {} {
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧foreach varName [concat $OptionControlledVariables Option] {
⟦TAB⟧    variable $varName
⟦TAB⟧    trace variable $varName r [namespace code {ProcessCmdLineArgs ;#}]
⟦TAB⟧}
    }

    proc RemoveAutoConfigureTraces {} {
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧foreach varName [concat $OptionControlledVariables Option] {
⟦TAB⟧    variable $varName
⟦TAB⟧    foreach pair [trace vinfo $varName] {
⟦TAB⟧⟦TAB⟧foreach {op cmd} $pair break
⟦TAB⟧⟦TAB⟧if {[string equal r $op]
⟦TAB⟧⟦TAB⟧⟦TAB⟧&& [string match *ProcessCmdLineArgs* $cmd]} {
⟦TAB⟧⟦TAB⟧    trace vdelete $varName $op $cmd
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧# Once the traces are removed, this can become a no-op
⟦TAB⟧proc RemoveAutoConfigureTraces {} {}
    }

    proc Configure args {
⟦TAB⟧variable Option
⟦TAB⟧variable Verify
⟦TAB⟧set n [llength $args]
⟦TAB⟧if {$n == 0} {
⟦TAB⟧    return [lsort [array names Option]]
⟦TAB⟧}
⟦TAB⟧if {$n == 1} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    return $Option($option)
⟦TAB⟧}
⟦TAB⟧while {[llength $args] > 1} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    if {[catch {$Verify($option) [lindex $args 1]} value]} {
⟦TAB⟧⟦TAB⟧return -code error "invalid $option\
⟦TAB⟧⟦TAB⟧⟦TAB⟧value \"[lindex $args 1]\": $value"
⟦TAB⟧    }
⟦TAB⟧    set Option($option) $value
⟦TAB⟧    set args [lrange $args 2 end]
⟦TAB⟧}
⟦TAB⟧if {[llength $args]} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    return -code error "missing value for option $option"
⟦TAB⟧}
    }
    proc configure args {
⟦TAB⟧RemoveAutoConfigureTraces
⟦TAB⟧set code [catch {eval Configure $args} msg]
⟦TAB⟧return -code $code $msg
    }
⟦SPACE⟧⟦SPACE⟧⟦SPACE⟧⟦SPACE⟧
    proc AcceptVerbose { level } {
⟦TAB⟧set level [AcceptList $level]
⟦TAB⟧if {[llength $level] == 1} {
⟦TAB⟧    if {![regexp {^(pass|body|skip|start|error)$} $level]} {
⟦TAB⟧⟦TAB⟧# translate single characters abbreviations to expanded list
⟦TAB⟧⟦TAB⟧set level [string map {p pass b body s skip t start e error} \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[split $level {}]]
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set valid [list]
⟦TAB⟧foreach v $level {
⟦TAB⟧    if {[regexp {^(pass|body|skip|start|error)$} $v]} {
⟦TAB⟧⟦TAB⟧lappend valid $v
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $valid
    }

    proc IsVerbose {level} {
⟦TAB⟧variable Option
⟦TAB⟧return [expr {[lsearch -exact $Option(-verbose) $level] != -1}]
    }

    # Default verbosity is to show bodies of failed tests
    Option -verbose {body error} {
⟦TAB⟧Takes any combination of the values 'p', 's', 'b', 't' and 'e'.
⟦TAB⟧Test suite will display all passed tests if 'p' is specified, all
⟦TAB⟧skipped tests if 's' is specified, the bodies of failed tests if
⟦TAB⟧'b' is specified, and when tests start if 't' is specified.
⟦TAB⟧ErrorInfo is displayed if 'e' is specified.
    } AcceptVerbose verbose

    # Match and skip patterns default to the empty list, except for
    # matchFiles, which defaults to all .test files in the
    # testsDirectory and matchDirectories, which defaults to all
    # directories.
    Option -match * {
⟦TAB⟧Run all tests within the specified files that match one of the
⟦TAB⟧list of glob patterns given.
    } AcceptList match

    Option -skip {} {
⟦TAB⟧Skip all tests within the specified tests (via -match) and files
⟦TAB⟧that match one of the list of glob patterns given.
    } AcceptList skip

    Option -file *.test {
⟦TAB⟧Run tests in all test files that match the glob pattern given.
    } AcceptPattern matchFiles

    # By default, skip files that appear to be SCCS lock files.
    Option -notfile l.*.test {
⟦TAB⟧Skip all test files that match the glob pattern given.
    } AcceptPattern skipFiles

    Option -relateddir * {
⟦TAB⟧Run tests in directories that match the glob pattern given.
    } AcceptPattern matchDirectories

    Option -asidefromdir {} {
⟦TAB⟧Skip tests in directories that match the glob pattern given.
    } AcceptPattern skipDirectories

    # By default, don't save core files
    Option -preservecore 0 {
⟦TAB⟧If 2, save any core files produced during testing in the directory
⟦TAB⟧specified by -tmpdir. If 1, notify the user if core files are
⟦TAB⟧created.
    } AcceptInteger preserveCore

    # debug output doesn't get printed by default; debug level 1 spits
    # up only the tests that were skipped because they didn't match or
    # were specifically skipped.  A debug level of 2 would spit up the
    # tcltest variables and flags provided; a debug level of 3 causes
    # some additional output regarding operations of the test harness.
    # The tcltest package currently implements only up to debug level 3.
    Option -debug 0 {
⟦TAB⟧Internal debug level⟦SPACE⟧
    } AcceptInteger debug

    proc SetSelectedConstraints args {
⟦TAB⟧variable Option
⟦TAB⟧foreach c $Option(-constraints) {
⟦TAB⟧    testConstraint $c 1
⟦TAB⟧}
    }
    Option -constraints {} {
⟦TAB⟧Do not skip the listed constraints listed in -constraints.
    } AcceptList
    trace variable Option(-constraints) w \
⟦TAB⟧    [namespace code {SetSelectedConstraints ;#}]

    # Don't run only the "-constraint" specified tests by default
    proc ClearUnselectedConstraints args {
⟦TAB⟧variable Option
⟦TAB⟧variable testConstraints
⟦TAB⟧if {!$Option(-limitconstraints)} {return}
⟦TAB⟧foreach c [array names testConstraints] {
⟦TAB⟧    if {[lsearch -exact $Option(-constraints) $c] == -1} {
⟦TAB⟧⟦TAB⟧testConstraint $c 0
⟦TAB⟧    }
⟦TAB⟧}
    }
    Option -limitconstraints false {
⟦TAB⟧whether to run only tests with the constraints
    } AcceptBoolean limitConstraints⟦SPACE⟧
    trace variable Option(-limitconstraints) w \
⟦TAB⟧    [namespace code {ClearUnselectedConstraints ;#}]

    # A test application has to know how to load the tested commands
    # into the interpreter.
    Option -load {} {
⟦TAB⟧Specifies the script to load the tested commands.
    } AcceptScript loadScript

    # Default is to run each test file in a separate process
    Option -singleproc 0 {
⟦TAB⟧whether to run all tests in one process
    } AcceptBoolean singleProcess⟦SPACE⟧

    proc AcceptTemporaryDirectory { directory } {
⟦TAB⟧set directory [AcceptAbsolutePath $directory]
⟦TAB⟧if {![file exists $directory]} {
⟦TAB⟧    file mkdir $directory
⟦TAB⟧}
⟦TAB⟧set directory [AcceptDirectory $directory]
⟦TAB⟧if {![file writable $directory]} {
⟦TAB⟧    if {[string equal [workingDirectory] $directory]} {
⟦TAB⟧⟦TAB⟧# Special exception: accept the default value
⟦TAB⟧⟦TAB⟧# even if the directory is not writable
⟦TAB⟧⟦TAB⟧return $directory
⟦TAB⟧    }
⟦TAB⟧    return -code error "\"$directory\" is not writeable"
⟦TAB⟧}
⟦TAB⟧return $directory
    }

    # Directory where files should be created
    Option -tmpdir [workingDirectory] {
⟦TAB⟧Save temporary files in the specified directory.
    } AcceptTemporaryDirectory temporaryDirectory
    trace variable Option(-tmpdir) w \
⟦TAB⟧    [namespace code {normalizePath Option(-tmpdir) ;#}]

    # Tests should not rely on the current working directory.
    # Files that are part of the test suite should be accessed relative
    # to [testsDirectory]
    Option -testdir [workingDirectory] {
⟦TAB⟧Search tests in the specified directory.
    } AcceptDirectory testsDirectory
    trace variable Option(-testdir) w \
⟦TAB⟧    [namespace code {normalizePath Option(-testdir) ;#}]

    proc AcceptLoadFile { file } {
⟦TAB⟧if {[string equal "" $file]} {return $file}
⟦TAB⟧set file [file join [temporaryDirectory] $file]
⟦TAB⟧return [AcceptReadable $file]
    }
    proc ReadLoadScript {args} {
⟦TAB⟧variable Option
⟦TAB⟧if {[string equal "" $Option(-loadfile)]} {return}
⟦TAB⟧set tmp [open $Option(-loadfile) r]
⟦TAB⟧loadScript [read $tmp]
⟦TAB⟧close $tmp
    }
    Option -loadfile {} {
⟦TAB⟧Read the script to load the tested commands from the specified file.
    } AcceptLoadFile loadFile
    trace variable Option(-loadfile) w [namespace code ReadLoadScript]

    proc AcceptOutFile { file } {
⟦TAB⟧if {[string equal stderr $file]} {return $file}
⟦TAB⟧if {[string equal stdout $file]} {return $file}
⟦TAB⟧return [file join [temporaryDirectory] $file]
    }

    # output goes to stdout by default
    Option -outfile stdout {
⟦TAB⟧Send output from test runs to the specified file.
    } AcceptOutFile outputFile
    trace variable Option(-outfile) w \
⟦TAB⟧    [namespace code {outputChannel $Option(-outfile) ;#}]

    # errors go to stderr by default
    Option -errfile stderr {
⟦TAB⟧Send errors from test runs to the specified file.
    } AcceptOutFile errorFile
    trace variable Option(-errfile) w \
⟦TAB⟧    [namespace code {errorChannel $Option(-errfile) ;#}]

}

#####################################################################

# tcltest::Debug* --
#
#     Internal helper procedures to write out debug information
#     dependent on the chosen level. A test shell may overide
#     them, f.e. to redirect the output into a different
#     channel, or even into a GUI.

# tcltest::DebugPuts --
#
#     Prints the specified string if the current debug level is
#     higher than the provided level argument.
#
# Arguments:
#     level   The lowest debug level triggering the output
#     string  The string to print out.
#
# Results:
#     Prints the string. Nothing else is allowed.
#
# Side Effects:
#     None.
#

proc tcltest::DebugPuts {level string} {
    variable debug
    if {$debug >= $level} {
⟦TAB⟧puts $string
    }
    return
}

# tcltest::DebugPArray --
#
#     Prints the contents of the specified array if the current
#       debug level is higher than the provided level argument
#
# Arguments:
#     level           The lowest debug level triggering the output
#     arrayvar        The name of the array to print out.
#
# Results:
#     Prints the contents of the array. Nothing else is allowed.
#
# Side Effects:
#     None.
#

proc tcltest::DebugPArray {level arrayvar} {
    variable debug

    if {$debug >= $level} {
⟦TAB⟧catch {upvar  $arrayvar $arrayvar}
⟦TAB⟧parray $arrayvar
    }
    return
}

# Define our own [parray] in ::tcltest that will inherit use of the [puts]
# defined in ::tcltest.  NOTE: Ought to construct with [info args] and
# [info default], but can't be bothered now.  If [parray] changes, then
# this will need changing too.
auto_load ::parray
proc tcltest::parray {a {pattern *}} [info body ::parray]

# tcltest::DebugDo --
#
#     Executes the script if the current debug level is greater than
#       the provided level argument
#
# Arguments:
#     level   The lowest debug level triggering the execution.
#     script  The tcl script executed upon a debug level high enough.
#
# Results:
#     Arbitrary side effects, dependent on the executed script.
#
# Side Effects:
#     None.
#

proc tcltest::DebugDo {level script} {
    variable debug

    if {$debug >= $level} {
⟦TAB⟧uplevel 1 $script
    }
    return
}

#####################################################################

proc tcltest::Warn {msg} {
    puts [outputChannel] "WARNING: $msg"
}

# tcltest::mainThread
#
#     Accessor command for tcltest variable mainThread.
#
proc tcltest::mainThread { {new ""} } {
    variable mainThread
    if {[llength [info level 0]] == 1} {
⟦TAB⟧return $mainThread
    }
    set mainThread $new
}

# tcltest::testConstraint --
#
#⟦TAB⟧sets a test constraint to a value; to do multiple constraints,
#       call this proc multiple times.  also returns the value of the
#       named constraint if no value was supplied.
#
# Arguments:
#⟦TAB⟧constraint - name of the constraint
#       value - new value for constraint (should be boolean) - if not
#               supplied, this is a query
#
# Results:
#⟦TAB⟧content of tcltest::testConstraints($constraint)
#
# Side effects:
#⟦TAB⟧none

proc tcltest::testConstraint {constraint {value ""}} {
    variable testConstraints
    variable Option
    DebugPuts 3 "entering testConstraint $constraint $value"
    if {[llength [info level 0]] == 2} {
⟦TAB⟧return $testConstraints($constraint)
    }
    # Check for boolean values
    if {[catch {expr {$value && $value}} msg]} {
⟦TAB⟧return -code error $msg
    }
    if {[limitConstraints]⟦SPACE⟧
⟦TAB⟧    && [lsearch -exact $Option(-constraints) $constraint] == -1} {
⟦TAB⟧set value 0
    }
    set testConstraints($constraint) $value
}

# tcltest::interpreter --
#
#⟦TAB⟧the interpreter name stored in tcltest::tcltest
#
# Arguments:
#⟦TAB⟧executable name
#
# Results:
#⟦TAB⟧content of tcltest::tcltest
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::interpreter { {interp ""} } {
    variable tcltest
    if {[llength [info level 0]] == 1} {
⟦TAB⟧return $tcltest
    }
    if {[string equal {} $interp]} {
⟦TAB⟧set tcltest {}
    } else {
⟦TAB⟧set tcltest $interp
    }
}

#####################################################################

# tcltest::AddToSkippedBecause --
#
#⟦TAB⟧Increments the variable used to track how many tests were
#       skipped because of a particular constraint.
#
# Arguments:
#⟦TAB⟧constraint     The name of the constraint to be modified
#
# Results:
#⟦TAB⟧Modifies tcltest::skippedBecause; sets the variable to 1 if
#       didn't previously exist - otherwise, it just increments it.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::AddToSkippedBecause { constraint {value 1}} {
    # add the constraint to the list of constraints that kept tests
    # from running
    variable skippedBecause

    if {[info exists skippedBecause($constraint)]} {
⟦TAB⟧incr skippedBecause($constraint) $value
    } else {
⟦TAB⟧set skippedBecause($constraint) $value
    }
    return
}

# tcltest::PrintError --
#
#⟦TAB⟧Prints errors to tcltest::errorChannel and then flushes that
#       channel, making sure that all messages are < 80 characters per
#       line.
#
# Arguments:
#⟦TAB⟧errorMsg     String containing the error to be printed
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::PrintError {errorMsg} {
    set InitialMessage "Error:  "
    set InitialMsgLen  [string length $InitialMessage]
    puts -nonewline [errorChannel] $InitialMessage

    # Keep track of where the end of the string is.
    set endingIndex [string length $errorMsg]

    if {$endingIndex < (80 - $InitialMsgLen)} {
⟦TAB⟧puts [errorChannel] $errorMsg
    } else {
⟦TAB⟧# Print up to 80 characters on the first line, including the
⟦TAB⟧# InitialMessage.
⟦TAB⟧set beginningIndex [string last " " [string range $errorMsg 0 \
⟦TAB⟧⟦TAB⟧[expr {80 - $InitialMsgLen}]]]
⟦TAB⟧puts [errorChannel] [string range $errorMsg 0 $beginningIndex]

⟦TAB⟧while {![string equal end $beginningIndex]} {
⟦TAB⟧    puts -nonewline [errorChannel] \
⟦TAB⟧⟦TAB⟧    [string repeat " " $InitialMsgLen]
⟦TAB⟧    if {($endingIndex - $beginningIndex)
⟦TAB⟧⟦TAB⟧    < (80 - $InitialMsgLen)} {
⟦TAB⟧⟦TAB⟧puts [errorChannel] [string trim \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[string range $errorMsg $beginningIndex end]]
⟦TAB⟧⟦TAB⟧break
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧set newEndingIndex [expr {[string last " " \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[string range $errorMsg $beginningIndex \
⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧[expr {$beginningIndex
⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧+ (80 - $InitialMsgLen)}]
⟦TAB⟧⟦TAB⟧]] + $beginningIndex}]
⟦TAB⟧⟦TAB⟧if {($newEndingIndex <= 0)
⟦TAB⟧⟦TAB⟧⟦TAB⟧|| ($newEndingIndex <= $beginningIndex)} {
⟦TAB⟧⟦TAB⟧    set newEndingIndex end
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧puts [errorChannel] [string trim \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[string range $errorMsg \
⟦TAB⟧⟦TAB⟧⟦TAB⟧    $beginningIndex $newEndingIndex]]
⟦TAB⟧⟦TAB⟧set beginningIndex $newEndingIndex
⟦TAB⟧    }
⟦TAB⟧}
    }
    flush [errorChannel]
    return
}

# tcltest::SafeFetch --
#
#⟦TAB⟧ The following trace procedure makes it so that we can safely
#        refer to non-existent members of the testConstraints array
#        without causing an error.  Instead, reading a non-existent
#        member will return 0. This is necessary because tests are
#        allowed to use constraint "X" without ensuring that
#        testConstraints("X") is defined.
#
# Arguments:
#⟦TAB⟧n1 - name of the array (testConstraints)
#       n2 - array key value (constraint name)
#       op - operation performed on testConstraints (generally r)
#
# Results:
#⟦TAB⟧none
#
# Side effects:
#⟦TAB⟧sets testConstraints($n2) to 0 if it's referenced but never
#       before used

proc tcltest::SafeFetch {n1 n2 op} {
    variable testConstraints
    DebugPuts 3 "entering SafeFetch $n1 $n2 $op"
    if {[string equal {} $n2]} {return}
    if {![info exists testConstraints($n2)]} {
⟦TAB⟧if {[catch {testConstraint $n2 [eval [ConstraintInitializer $n2]]}]} {
⟦TAB⟧    testConstraint $n2 0
⟦TAB⟧}
    }
}

# tcltest::ConstraintInitializer --
#
#⟦TAB⟧Get or set a script that when evaluated in the tcltest namespace
#⟦TAB⟧will return a boolean value with which to initialize the
#⟦TAB⟧associated constraint.
#
# Arguments:
#⟦TAB⟧constraint - name of the constraint initialized by the script
#⟦TAB⟧script - the initializer script
#
# Results
#⟦TAB⟧boolean value of the constraint - enabled or disabled
#
# Side effects:
#⟦TAB⟧Constraint is initialized for future reference by [test]
proc tcltest::ConstraintInitializer {constraint {script ""}} {
    variable ConstraintInitializer
    DebugPuts 3 "entering ConstraintInitializer $constraint $script"
    if {[llength [info level 0]] == 2} {
⟦TAB⟧return $ConstraintInitializer($constraint)
    }
    # Check for boolean values
    if {![info complete $script]} {
⟦TAB⟧return -code error "ConstraintInitializer must be complete script"
    }
    set ConstraintInitializer($constraint) $script
}

# tcltest::InitConstraints --
#
# Call all registered constraint initializers to force initialization
# of all known constraints.
# See the tcltest man page for the list of built-in constraints defined
# in this procedure.
#
# Arguments:
#⟦TAB⟧none
#
# Results:
#⟦TAB⟧The testConstraints array is reset to have an index for each
#⟦TAB⟧built-in test constraint.
#
# Side Effects:
#       None.
#

proc tcltest::InitConstraints {} {
    variable ConstraintInitializer
    initConstraintsHook
    foreach constraint [array names ConstraintInitializer] {
⟦TAB⟧testConstraint $constraint
    }
}

proc tcltest::DefineConstraintInitializers {} {
    ConstraintInitializer singleTestInterp {singleProcess}

    # All the 'pc' constraints are here for backward compatibility and
    # are not documented.  They have been replaced with equivalent 'win'
    # constraints.

    ConstraintInitializer unixOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) unix}
    ConstraintInitializer macOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) macintosh}
    ConstraintInitializer pcOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) windows}
    ConstraintInitializer winOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) windows}

    ConstraintInitializer unix {testConstraint unixOnly}
    ConstraintInitializer mac {testConstraint macOnly}
    ConstraintInitializer pc {testConstraint pcOnly}
    ConstraintInitializer win {testConstraint winOnly}

    ConstraintInitializer unixOrPc \
⟦TAB⟧    {expr {[testConstraint unix] || [testConstraint pc]}}
    ConstraintInitializer macOrPc \
⟦TAB⟧    {expr {[testConstraint mac] || [testConstraint pc]}}
    ConstraintInitializer unixOrWin \
⟦TAB⟧    {expr {[testConstraint unix] || [testConstraint win]}}
    ConstraintInitializer macOrWin \
⟦TAB⟧    {expr {[testConstraint mac] || [testConstraint win]}}
    ConstraintInitializer macOrUnix \
⟦TAB⟧    {expr {[testConstraint mac] || [testConstraint unix]}}

    ConstraintInitializer nt {string equal $::tcl_platform(os) "Windows NT"}
    ConstraintInitializer 95 {string equal $::tcl_platform(os) "Windows 95"}
    ConstraintInitializer 98 {string equal $::tcl_platform(os) "Windows 98"}

    # The following Constraints switches are used to mark tests that
    # should work, but have been temporarily disabled on certain
    # platforms because they don't and we haven't gotten around to
    # fixing the underlying problem.

    ConstraintInitializer tempNotPc {expr {![testConstraint pc]}}
    ConstraintInitializer tempNotWin {expr {![testConstraint win]}}
    ConstraintInitializer tempNotMac {expr {![testConstraint mac]}}
    ConstraintInitializer tempNotUnix {expr {![testConstraint unix]}}

    # The following Constraints switches are used to mark tests that
    # crash on certain platforms, so that they can be reactivated again
    # when the underlying problem is fixed.

    ConstraintInitializer pcCrash {expr {![testConstraint pc]}}
    ConstraintInitializer winCrash {expr {![testConstraint win]}}
    ConstraintInitializer macCrash {expr {![testConstraint mac]}}
    ConstraintInitializer unixCrash {expr {![testConstraint unix]}}

    # Skip empty tests

    ConstraintInitializer emptyTest {format 0}

    # By default, tests that expose known bugs are skipped.

    ConstraintInitializer knownBug {format 0}

    # By default, non-portable tests are skipped.

    ConstraintInitializer nonPortable {format 0}

    # Some tests require user interaction.

    ConstraintInitializer userInteraction {format 0}

    # Some tests must be skipped if the interpreter is not in
    # interactive mode

    ConstraintInitializer interactive \
⟦TAB⟧    {expr {[info exists ::tcl_interactive] && $::tcl_interactive}}

    # Some tests can only be run if the installation came from a CD
    # image instead of a web image.  Some tests must be skipped if you
    # are running as root on Unix.  Other tests can only be run if you
    # are running as root on Unix.

    ConstraintInitializer root {expr \
⟦TAB⟧    {[string equal unix $::tcl_platform(platform)]
⟦TAB⟧    && ([string equal root $::tcl_platform(user)]
⟦TAB⟧⟦TAB⟧|| [string equal "" $::tcl_platform(user)])}}
    ConstraintInitializer notRoot {expr {![testConstraint root]}}

    # Set nonBlockFiles constraint: 1 means this platform supports
    # setting files into nonblocking mode.

    ConstraintInitializer nonBlockFiles {
⟦TAB⟧    set code [expr {[catch {set f [open defs r]}]⟦SPACE⟧
⟦TAB⟧⟦TAB⟧    || [catch {fconfigure $f -blocking off}]}]
⟦TAB⟧    catch {close $f}
⟦TAB⟧    set code
    }

    # Set asyncPipeClose constraint: 1 means this platform supports
    # async flush and async close on a pipe.
    #
    # Test for SCO Unix - cannot run async flushing tests because a
    # potential problem with select is apparently interfering.
    # (Mark Diekhans).

    ConstraintInitializer asyncPipeClose {expr {
⟦TAB⟧    !([string equal unix $::tcl_platform(platform)]⟦SPACE⟧
⟦TAB⟧    && ([catch {exec uname -X | fgrep {Release = 3.2v}}] == 0))}}

    # Test to see if we have a broken version of sprintf with respect
    # to the "e" format of floating-point numbers.

    ConstraintInitializer eformat {string equal [format %g 5e-5] 5e-05}

    # Test to see if execed commands such as cat, echo, rm and so forth
    # are present on this machine.

    ConstraintInitializer unixExecs {
⟦TAB⟧set code 1
        if {[string equal macintosh $::tcl_platform(platform)]} {
⟦TAB⟧    set code 0
        }
        if {[string equal windows $::tcl_platform(platform)]} {
⟦TAB⟧    if {[catch {
⟦TAB⟧        set file _tcl_test_remove_me.txt
⟦TAB⟧        makeFile issing value for option [lindex $args 0]"
⟦TAB⟧    exit 1
⟦TAB⟧}
    }

    # Call the hook
    catch {
        array set flag $flagArray
        processCmdLineArgsHook [array get flag]
    }
    return
}

# tcltest::ProcessCmdLineArgs --
#
#       This procedure must be run after constraint initialization is
#⟦TAB⟧set up (by [DefineConstraintInitializers]) because some constraints
#⟦TAB⟧can be overridden.
#
#       Perform configuration according to the command-line options.
#
# Arguments:
#⟦TAB⟧none
#
# Results:
#⟦TAB⟧Sets the above-named variables in the tcltest namespace.
#
# Side Effects:
#       None.
#

proc tcltest::ProcessCmdLineArgs {} {
    variable originalEnv
    variable testConstraints

    # The "argv" var doesn't exist in some cases, so use {}.
    if {![info exists ::argv]} {
⟦TAB⟧ProcessFlags {}
    } else {
⟦TAB⟧ProcessFlags $::argv
    }

    # Spit out everything you know if we're at a debug level 2 or
    # greater
    DebugPuts 2 "Flags passed into tcltest:"
    if {[info exists ::env(TCLTEST_OPTIONS)]} {
⟦TAB⟧DebugPuts 2 \
⟦TAB⟧⟦TAB⟧"    ::env(TCLTEST_OPTIONS): $::env(TCLTEST_OPTIONS)"
    }
    if {[info exists ::argv]} {
⟦TAB⟧DebugPuts 2 "    argv: $::argv"
    }
    DebugPuts    2 "tcltest::debug     ent (tcltest::originalEnv):"
    DebugPArray  2 originalEnv
    DebugPuts    2 "Constraints:"
    DebugPArray  2 testConstraints
}

#####################################################################

# Code to run the tests goes here.

# tcltest::TestPuts --
#
#⟦TAB⟧Used to redefine puts in test environment.  Stores whatever goes
#⟦TAB⟧out on stdout in tcltest::outData and stderr in errData before
#⟦TAB⟧sending it on to the regular puts.
#
# Arguments:
#⟦TAB⟧same as standard puts
#
# Results:
#⟦TAB⟧none
#
# Side effects:
#       Intercepts puts; data that would otherwise go to stdout, stderr,
#⟦TAB⟧or file channels specified in outputChannel and errorChannel
#⟦TAB⟧does not get sent to the normal puts function.
namespace eval tcltest::Replace {
    namespace export puts
}
proc tcltest::Replace::puts {args} {
    vaet errData {}
⟦TAB⟧rename ::puts [namespace current]::Replace::Puts
⟦TAB⟧namespace eval :: \
⟦TAB⟧⟦TAB⟧[list namespace import [namespace origin Replace::puts]]
⟦TAB⟧namespace import Replace::puts
    }
    set result [uplevel 1 $script]
    if {!$ignoreOutput} {
⟦TAB⟧namespace forget puts
⟦TAB⟧namespace eval :: namespace forget puts
⟦TAB⟧rename [namespace current]::Replace::Puts ::puts
    }
    return $result
}

# tcltest::CompareStrings --
#
#⟦TAB⟧compares the expected answer to the actual answer, depending on
#⟦TAB⟧the mode provided.  Mode determines whether a regexp, exact,
#⟦TAB⟧glob or custom comparison is done.
#
# Arguments:
#⟦TAB⟧actual - string containing the actual result
#       expected - pattern to be matched against
#       mode - type of comparison to be done
#
# Results:
#⟦TAB⟧result of the match
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::CompareStrings {actual expected mode} {
    variable CustomMatch
    if {![info exists CustomMatch($mode)]} {
        return -code error "No matching command registered for `-match $mode'"
    }
    set match [namespace eval :: $CustomMatch($mode) [list $expected $actual]]
    if {[catch {expr {$match && $match}} result]} {
⟦TAB⟧return -code error "Invalid result from `-match $mode' command: $result"
    }
    return $match
}

# tcltest::customMatch --
#
#⟦TAB⟧registers a command to be called when a particular type of
#⟦TAB⟧matching is required.
#
# Arguments:
#⟦TAB⟧nickname - Keyword for the type of matching
#⟦TAB⟧cmd - Incomplete command that implements that type of matching
#⟦TAB⟧⟦TAB⟧when completed with expected string and actual string
#⟦TAB⟧⟦TAB⟧and then evaluated.
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#⟦TAB⟧Sets the variable tcltest::CustomMatch

proc tcltest::customMatch {mode script} {
    variable CustomMatch
    if {![info complete $script]} {
⟦TAB⟧return -code error \
⟦TAB⟧⟦TAB⟧"invalid customMatch script; can't evaluate after completion"
    }
    set CustomMatch($mode) $script
}

# tcltest::SubstArguments list
#
# This helper function takes in a list of words, then perform a
# substitution on the list as [split] ignores braces as it should.  Therefore what we do is
    # gradually build up a string out of whitespace seperated strings.
    # We cannot use [split] to split the argList into whitespace
 g
⟦TAB⟧    # this word, from the argList.
            set text [string range $argList 0 \
⟦TAB⟧⟦TAB⟧    [expr {[lindex $all 0] - 1}]]
            # Get the word with the quote
            set word [string range $argList \
                    [lindex $all 0] [lindex $all 1]]

            # Remove all text up to and including the word from the
            # argList.
            set argList [string range $argList \
                    [expr {[lindex $all 1] + 1}] end]
        } else {
            # Take everything up to the end of the argList.
            set text $argList
            set word {}
            set argList {}
        }

        if {$token != {}} {
            # If we saw a word with quote before, then there is a
            # multi-word token starting with that word.  In this case,
            # add the text and the current word to this token.
            append token $text $word
        } else {
            # Add the text to the result.  There is no need to parse
            # the text because it couldn't be a part of any multi-word
            # token.  Then start a new multi-word token with the word
            # because we need to pass this token to the Tcl parser to
            # check for balancing quotes
            append result $text
            set token $word
        }

        if { [catch {llength $token} length] == 0 && $length == 1} {
            # The token is a valid list so add it to the result.
            # lappend result [string trim $token]
            append result \{$token\}
            set token {}
        }
    }

    # If the last token has not been added to the list then there
    # is a problem.
    if { [string length $token] } {
        error "incomplete token \"$token\""
    }

    return $result
}

# tcltest::test --
#
# This procedure runs a test and prints an error message if the test
# fails.  If verbose has been set, it also prints a message even if the
# test succeeds.  The test will be skipped if it doesn't match the
# match variable, if it matches an element in skip, or if one of the
# elements of "constraints" turns out not to be true.
#
# If testLevel is 1, then this is a top level test, and we record
# pass/fail information; otherwise, this information is not logged and
# is not added to running totals.
#
# Attributes:
#   Only description is a required attribute.  All others are optional.
#   Default values are indicated.
#
#   constraints -⟦TAB⟧A list of one or more keywords, each of which
#⟦TAB⟧⟦TAB⟧⟦TAB⟧must be the name of an element in the array
#⟦TAB⟧⟦TAB⟧⟦TAB⟧"testConstraints".  If any of these elements is
#⟦TAB⟧⟦TAB⟧⟦TAB⟧zero, the test is skipped. This attribute is
#⟦TAB⟧⟦TAB⟧⟦TAB⟧optional; default is {}
#   body -⟦TAB⟧        Script to run to carry out the test.  It must
#⟦TAB⟧⟦TAB⟧        return a result that can be checked for
#⟦TAB⟧⟦TAB⟧        correctness.  This attribute is optional;
#                       default is {}
#   result -⟦TAB⟧        Expected result from script.  This attribute is
#                       optional; default is {}.
#   output -            Expected output sent to stdout.  This attribute
#                       is optional; default is {}.
#   errorOutput -       Expected output sent to stderr.  This attribute
#                       is optional; default is {}.
#   returnCodes -       Expected return codes.  This attribute is
#                       optional; default is {0 2}.
#   setup -             Code to run before $script (above).  This
#                       attribute is optional; default is {}.
#   cleanup -           Code to run after $script (above).  This
#                       attribute is optional; default is {}.
#   match -             specifies type of matching to do on result,
#                       output, errorOutput; this must be a string
#⟦TAB⟧⟦TAB⟧⟦TAB⟧previously registered by a call to [customMatch].
#⟦TAB⟧⟦TAB⟧⟦TAB⟧The stringson of the test, to
#  ⟦TAB⟧⟦TAB⟧  ⟦TAB⟧help humans understand what it does.
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#       Just about anything is possible depending on the test.
#

proc tcltest::test {name description args} {
    global tcl_platform
    variable testLevel
    variable coreModTime
    DebugPuts 3 "test $name $args"
    DebugDo 1 {
⟦TAB⟧variable TestNames
⟦TAB⟧catch {
⟦TAB⟧    puts "test name '$name' re-used; prior use in $TestNames($name)"
⟦TAB⟧}
⟦TAB⟧set TestNames($name) [info script]
    }

    FillFilesExisted
    incr testLevel

    # Pre-define everything to null except output and errorOutput.  We
    # determine whether or not to trap output based on whether or not
    # these variables (output & errorOutput) are defined.
    foreach item {constraints setup cleanup body result returnCodes
⟦TAB⟧    match} {
⟦TAB⟧set $item {}
    }

    # Set the default match mode
    set match exact

    # Set the default match values for return codes (0 is the standard
    # expected return value if everything went well; 2 represents
    # 'return' being used in the test script).
    set returnCodes [list 0 2]

    # The old test format can't have a 3rd argument (constraints or
    # script) that starts with '-'.
    if {[string match -* [lindex $args 0]]
⟦TAB⟧    || ([llength $args] <= 1)} {
⟦TAB⟧if {[llength $args] == 1} {
⟦TAB⟧    set list [SubstArguments [lindex $args 0]]
⟦TAB⟧    foreach {element value} $list {
⟦TAB⟧⟦TAB⟧set testAttributes($element) $value
⟦TAB⟧    }
⟦TAB⟧    foreach item {constraints match setup body cleanup \
⟦TAB⟧⟦TAB⟧    result returnCodes output errorOutput} {
⟦TAB⟧⟦TAB⟧if {[info exists testAttributes(-$item)]} {
⟦TAB⟧⟦TAB⟧    set testAttributes(-$item) [uplevel 1 \
⟦TAB⟧⟦TAB⟧⟦TAB⟧    ::concat $testAttributes(-$item)]
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧} else {
⟦TAB⟧    array set testAttributes $args
⟦TAB⟧}

⟦TAB⟧set validFlags {-setup -cleanup -body -result -returnCodes \
⟦TAB⟧⟦TAB⟧-match -output -errorOutput -constraints}

⟦TAB⟧foreach flag [array names testAttributes] {
⟦TAB⟧    if {[lsearch -exact $validFlags $flag] == -1} {
⟦TAB⟧⟦TAB⟧incr testLevel -1
⟦TAB⟧⟦TAB⟧set sorted [lsort $validFlags]
⟦TAB⟧⟦TAB⟧set op else {
⟦TAB⟧    incr testLevel -1
⟦TAB⟧    return -code error "wrong # args:\
⟦TAB⟧⟦TAB⟧    should be \"test name desc ?options?\""
⟦TAB⟧}
    }

    if {[Skipped $name $constraints]} {
⟦TAB⟧incr testLevel -1
⟦TAB⟧return
    }

    # Save information about the core file.⟦SPACE⟧⟦SPACE⟧
    if {[preserveCore]} {
⟦TAB⟧if {[file exists [file join [workingDirectory] core]]} {
⟦TAB⟧    set coreModTime [file mtime [file join [workingDirectory] core]]
⟦TAB⟧}
    }

    # First, run the setup script
    set code [catch {uplevel 1 $setup} setupMsg]
    if {$code == 1} {
⟦TAB⟧set errorInfo(setup) $::errorInfo
⟦TAB⟧set errorCode(setup) $::errorCode
    }
    set setupFailure [expr {$code != 0}]

    # Only run the test body if the setup was successful
    if {!$setupFailure} {

⟦TAB⟧# Verbose notification of $body start
⟦TAB⟧if {[IsVerbose start]} {
⟦TAB⟧    puts [outputChannel] "---- $name start"
⟦TAB⟧    flush [outputChannel]
⟦TAB⟧}

⟦TAB⟧set command [list [namespace origin RunTest] $name $body]
⟦TAB⟧if {[info exists output] || [info exists errorOutput]} {
⟦TAB⟧    set testResult [uplevel 1 [list [namespace origin Eval] $command 0]]
⟦TAB⟧} else {
⟦TAB⟧    set testResult [uplevel 1 [list [namespace origin Eval] $command 1]]
⟦TAB⟧}
⟦TAB⟧foreach {actualAnswer returnCode} $testResult break
⟦TAB⟧if {$returnCode == 1} {
⟦TAB⟧    set errorInfo(body) $::errorInfo
⟦TAB⟧    set errorCode(body) $::errorCode
⟦TAB⟧}
    }

    # Always run the cleanup script
    set code [catch {uplevel 1 $cleanup} cleanupMsg]
    if {$code == 1} {
⟦TAB⟧set errorInfo(cleanup) $::errorInfo
⟦TAB⟧set errorCode(cleanup) $::errorCode
    }
    set cleanupFailure [expr {$code != 0}]

    set coreFailure 0
    set coreMsg ""
    # check for a core file first - if one was created by the test,
    # then the test failed
    if {[preserveCore]} {
⟦TAB⟧if {[file exists [file join [workingDirectory] core]]} {
⟦TAB⟧    # There's only a test failure if there is a core file
⟦TAB⟧    # and (1) there previously wasn't one or (2) the new
⟦TAB⟧    # one is different from the old one.
⟦TAB⟧    if {[info exists coreModTime]} {
⟦TAB⟧⟦TAB⟧if {$coreModTime != [file mtime \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file jod the test.
    set outputFailure 0
    variable outData
    if {[info exists output] && !$codeFailure} {
⟦TAB⟧if {[set outputCompare [catch {
⟦TAB⟧    CompareStrings $outData $output $match
⟦TAB⟧} outputMatch]]  {[set errorCompare [catch {
⟦TAB⟧    CompareStrings $errData $errorOutput $match
⟦TAB⟧} errorMatch]] == 0} {
⟦TAB⟧    set errorFailure [expr {!$errorMatch}]
⟦TAB⟧} else {
⟦TAB⟧    set errorFailure 1
⟦TAB⟧}
    }

    # check if the answer matched the expected answer
    # Only check if we ran the body of the test (no setup failure)
    if {$setupFailure || $codeFailure} {
⟦TAB⟧set scriptFailure 0
    } elseif {[set scriptCompare [catch {
⟦TAB⟧CompareStrings $actualAnswer $result $match
    } scriptMatch]] == 0} {
⟦TAB⟧set scriptFailure [expr {!$scriptMatch}]
    } else {
⟦TAB⟧set scriptFailure 1
    }

    # if we didn't experience any failures, then we passed
    variable numTests
    if {!($setupFailure || $cleanupFailure || $coreFailure
⟦TAB⟧    || $outputFailure || $errorFailure || $codeFailure
⟦TAB⟧    || $scriptFailure)} {
⟦TAB⟧if {$testLevel == 1} {
⟦TAB⟧    incr numTests(Passed)
⟦TAB⟧    if {[IsVerbose pass]} {
⟦TAB⟧⟦TAB⟧puts [outputChannel] "++++ $name PASSED"
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧incr testLevel -1
⟦TAB⟧return
    }

    # We know the test failed, tally it...
    if {$testLevel == 1} {
⟦TAB⟧incr numTests(Failed)
    }

    # ... then report according to the type of failure
    variable currentFailure true
    if {![IsVerbose body]} {
⟦TAB⟧set body ""
    }⟦TAB⟧
    puts [outputChannel] "\n==== $name\
⟦TAB⟧    [string trim $description] FAILED"
    if {[string length $body]} {
⟦TAB⟧puts [outputChannel] "==== Contents of test case:"
⟦TAB⟧puts [outputChannel] $body
    }
    if {$setupFailure} {
⟦TAB⟧puts [outputChannel] "---- Test setup\
⟦TAB⟧⟦TAB⟧failed:\n$setupMsg"
⟦TAB⟧if {[info exists errorInfo(setup)]} {
⟦TAB⟧    puts [outputChannel] "---- errorInfo(setup): $errorInfo(setup)"
⟦TAB⟧    puts [outputChannel] "---- errorCode(setup): $errorCode(setup)"
⟦TAB⟧}
    }
    if {$scriptFailure} {
⟦TAB⟧if {$scriptCompare} {
⟦TAB⟧    puts [outputChannel] "---- Error testing result: $scriptMatch"
⟦TAB⟧} else {
⟦TAB⟧    puts [outputChannel] "---- Result was:\n$actualAnswer"
⟦TAB⟧    puts [outputChannel] "---- Result should have been\
⟦TAB⟧⟦TAB⟧    ($match matching):\n$result"
⟦TAB⟧}
    }
    if {$codeFailure} {
⟦TAB⟧switch -- $returnCode {
⟦TAB⟧    0 { set msg "Test completed normally" }
⟦TAB⟧    1 { set msg "Test generated error" }
⟦TAB⟧    2 { set msg "Test generated return exception" }
⟦TAB⟧    3 { set msg "Test generated break exception" }
⟦TAB⟧    4 { set msg "Test generated continue exception" }
⟦TAB⟧    default { set msg "Test generated exception" }
⟦TAB⟧}
⟦TAB⟧puts [outputChannel] "---- $msg; Return code was: $returnCode"
⟦TAB⟧puts [outputChannel] "---- Return code should have been\
⟦TAB⟧⟦TAB⟧one of: $returnCodes"
⟦TAB⟧if {[IsVerbose error]} {
⟦TAB⟧    if {[info exists errorInfo(body)] && ([lsearch $returnCodes 1]<0)} {
⟦TAB⟧⟦TAB⟧puts [outputChannel] "---- errorInfo: $errorInfo(body)"
⟦TAB⟧⟦TAB⟧puts [outputChannel] "---- errorCode: $errorCode(body)"
⟦TAB⟧    }
⟦TAB⟧}
    }
    if {$outputFailure} {
⟦TAB⟧if {$outputCompare} {
⟦TAB⟧    puts [outputChannel] "---- Error testing output: $outputMatch"
⟦TAB⟧} else {
⟦TAB⟧    puts [outputChannel] "---- Output was:\n$outData"
⟦TAB⟧    puts [outputChannel] "---- Output should have been\
⟦TAB⟧⟦TAB⟧    ($match matching):\n$output"
⟦TAB⟧}
    }
    if {$errorFailure} {
⟦TAB⟧if {$errorCompare} {
⟦TAB⟧    puts [outputChannel] "---- Error testing errorOutput: $errorMatch"
⟦TAB⟧} else {
⟦TAB⟧    puts [outputChannel] "---- Error output was:\n$errData"
⟦TAB⟧    puts [outputChannel] "---- Error output should have\
⟦TAB⟧⟦TAB⟧    been ($match matching):\n$errorOutput"
⟦TAB⟧}
    }
    if {$cleanupFailure} {
⟦TAB⟧puts [outputChannel] "---- Test cleanup failed:\n$cleanupMsg"
⟦TAB⟧if {[info exists errorInfo(cleanup)]} {
⟦TAB⟧    puts [outputChannel] "---- errorInfo(cleanup): $errorInfo(cleanup)"
⟦TAB⟧    puts [outputChannel] "---- errorCode(cleanup): $errorCode(cleanup)"
⟦TAB⟧}
    }
    if {$coreFailure} {
⟦TAB⟧puts [outputChannel] "---- Core file produced while running\
⟦TAB⟧⟦TAB⟧test!  $coreMsg"
    }
    puts [outputChannel] "==== $name FAILED\n"

    incr testLevel -1
    return
}

# Skipped --
#
# Given a test name and it constraints, returns a boolean indicating
# whether the current configuration says the test should be skipped.
#
# Side Effects:  Maintains tally of total tests seen and tests skipped.
#
proc tcltest::Skipped {name constraints} {
    variable testLevel
    variable numTests
    variable testConstraints

    if {$testLevel == 1} {
⟦TAB⟧incr numTests(Total)
    }
    # skip the test if it's name matches an element of skip
    foreach pattern [skip] {
⟦TAB⟧if {[string match $pattern $name]} {
⟦TAB⟧    if {$testLevel == 1} {
⟦TAB⟧⟦TAB⟧incr numTests(Skipped)
⟦TAB⟧⟦TAB⟧DebugDo 1 {AddToSkippedBecause userSpecifiedSkip}
⟦TAB⟧    }
⟦TAB⟧    return 1
⟦TAB⟧}
    }
    # skip the test if it's name doesn't match any element of match
    set ok 0
    foreach pattern [match] {
⟦TAB⟧if {[string match $pattern $name]} {
⟦TAB⟧    set ok 1
⟦TAB⟧    break
⟦TAB⟧}
    }
    if {!$ok} {
⟦TAB⟧if {$testLevel == 1} {
⟦TAB⟧    incr numTests(Skipped)
⟦TAB⟧    DebugDo 1 {AddToSkippedBecause userSpecifiedNonMatch}
⟦TAB⟧}
⟦TAB⟧return 1
    }
    if {[string equal {} $constraints]} {
⟦TAB⟧# If we're limited to the listed constraints and there aren't
⟦TAB⟧# any listed, then we shouldn't run the test.
⟦TAB⟧if {[limitConstraints]} {
⟦TAB⟧    AddToSkippedBecause userSpecifiedLimitConstraint
⟦TAB⟧    if {$testLevel == 1} {
⟦TAB⟧⟦TAB⟧incr numTests(Skipped)
⟦TAB⟧    }
⟦TAB⟧    return 1
⟦TAB⟧}
    } else {
⟦TAB⟧# "constraints" argument exists;
⟦TAB⟧# make sure that the constraints are satisfied.

⟦TAB⟧set doTest 0
⟦TAB⟧if {[string match {*[$\[]*} $constraints] != 0} {
⟦TAB⟧    # full expression, e.g. {$foo > [info tclversion]}
⟦TAB⟧    catch {set doTest [uplevel #0 expr $constraints]}
⟦TAB⟧} elseif {[regexp {[^.a-zA-Z0-9 \n\r\t]+} $constraints] != 0} {
⟦TAB⟧    # something like {a || b} should be turned into
⟦TAB⟧    # $testConstraints(a) || $testConstraints(b).
⟦TAB⟧    regsub -all {[.\w]+} $constraints {$testConstraints(&)} c
⟦TAB⟧    catch {set doTest [eval expr $c]}
⟦TAB⟧} elseif {![catch {llength $constraints}]} {
⟦TAB⟧    # just simple constraints such as {unixOnly fonts}.
⟦TAB⟧    set doTest 1
⟦TAB⟧    foreach constraint $constraints {
⟦TAB⟧⟦TAB⟧if {(![info exists testConstraints($constraint)]) \
⟦TAB⟧⟦TAB⟧⟦TAB⟧|| (!$testConstraints($constraint))} {
⟦TAB⟧⟦TAB⟧    set doTest 0

⟦TAB⟧⟦TAB⟧    # store the constraint that kept the test from
⟦TAB⟧⟦TAB⟧    # running
⟦TAB⟧⟦TAB⟧    set constraints $constraint
⟦TAB⟧⟦TAB⟧    break
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧
⟦TAB⟧if {$doTest == 0} {
⟦TAB⟧    if {[IsVerbose skip]} {
⟦TAB⟧⟦TAB⟧puts [outputChannel] "++++ $name SKIPPED: $constraints"
⟦TAB⟧    }

⟦TAB⟧    if {$testLevel == 1} {
⟦TAB⟧⟦TAB⟧incr numTests(Skipped)
⟦TAB⟧⟦TAB⟧AddToSkippedBecause $constraints
⟦TAB⟧    }
⟦TAB⟧    return 1
⟦TAB⟧}
    }
    return 0
}

# RunTest --
#
# This is where the body of a test is evaluated.  The combination of
# [RunTest] and [Eval] allows the output and error output of the test
# body to be captured for comparison against the expected values.

proc tcltest::RunTest {name script} {
    DebugPuts 3 "Running $name {$script}"

    # If there is no "memory" command (because memory debugging isn't
    # enabled), then don't attempt to use the command.

    if {[llength [info commands memory]] == 1} {
⟦TAB⟧memory tag $name
    }

    set code [catch {uplevel 1 $script} actualAnswer]

    return [list $actualAnswer $code]
}

#####################################################################

# tcltest::cleanupTestsHook --
#
#⟦TAB⟧This hook allows a harness that builds upon tcltest to specify
#       additional things that should be done at cleanup.
#

if {[llength [info commands tcltest::cleanupTestsHook]] == 0} {
    proc tcltest::cleanupTestsHook {} {}
}

# tcltest::cleanupTests --
#
# Remove files and dirs created using the makeFile and makeDirectory
# commands since the last time this proc was invoked.
#
# Print the names of the files created without the makeFile command
# since the tests were invoked.
#
# Print the number tests (total, passed, failed, and skipped) since the
# tests were invoked.
#
# Restore original environment (as reported by special variable env).
#
# Arguments:
#      calledFromAllFile - if 0, behave as if we are running a single
#      test file within an entire suite of tests.  if we aren't running
#      a single test file, then don't report status.  check for new
#      files created during the test run and report on them.  if 1,
#      report collated status from all the test file runs.
#
# Results:
#      None.
#
# Side Effects:
#      None
#

proc tcltest::cleanupTests {{calledFromAllFile 0}} {
    variable filesMade
    variable filesExisted
    variable createdNewFiles
    variable testSingleFile
    variable numTests
    variable numTestFiles
    variable failFiles
    variable skippedBecause
    variable currentFailure
    variable originalEnv
    variable originalTclPlatform
    variable coreModTime

    FillFilesExisted
    set testFileName [file tail [info script]]

    # Call the cleanup hook
    cleanupTestsHook

    # Remove files and directories created by the makeFile and
    # makeDirectory procedures.  Record the names of files in
    # workingDirectory that were not pre-existing, and associate them
    # with the test file that created them.

    if {!$calledFromAllFile} {
⟦TAB⟧foreach file $filesMade {
⟦TAB⟧    if {[file exists $file]} {
⟦TAB⟧⟦TAB⟧DebugDo 1 {Warn "cleanupTests deleting $file..."}
⟦TAB⟧⟦TAB⟧catch {file delete -force $file}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set currentFiles {}
⟦TAB⟧foreach file [glob -nocomplain \
⟦TAB⟧⟦TAB⟧-directory [temporaryDirectory] *] {
⟦TAB⟧    lappend currentFiles [file tail $file]
⟦TAB⟧}
⟦TAB⟧set newFiles {}
⟦TAB⟧foreach file $currentFiles {
⟦TAB⟧    if {[lsearch -exact $filesExisted $file] == -1} {
⟦TAB⟧⟦TAB⟧lappend newFiles $file
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set filesExisted $currentFiles
⟦TAB⟧if {[llength $newFiles] > 0} {
⟦TAB⟧    set createdNewFiles($testFileName) $newFiles
⟦TAB⟧}
    }

    if {$calledFromAllFile || $testSingleFile} {

⟦TAB⟧# print stats

⟦TAB⟧puts -nonewline [outputChannel] "$testFileName:"
⟦TAB⟧foreach index [list "Total" "Passed" "Skipped" "Failed"] {
⟦TAB⟧    puts -nonewline [outputChannel] \
⟦TAB⟧⟦TAB⟧    "\t$index\t$numTests($index)"
⟦TAB⟧}
⟦TAB⟧puts [outputChannel] ""

⟦TAB⟧# print number test files sourced
⟦TAB⟧# print names of files that ran tests which failed

⟦TAB⟧if {$calledFromAllFile} {
⟦TAB⟧    puts [outputChannel] \
⟦TAB⟧⟦TAB⟧    "Sourced $numTestFiles Testnfo exists originalEnv($index)]} {
⟦TAB⟧⟦TAB⟧lappend newEnv $index
⟦TAB⟧⟦TAB⟧unset ::env($index)
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧if {$::env($index) != $originalEnv($index)} {
⟦TAB⟧⟦TAB⟧    lappend changedEnv $index
⟦TAB⟧⟦TAB⟧    set ::env($index) $originalEnv($index)
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧foreach index [array names originalEnv] {
⟦TAB⟧    if {![info exists ::env($index)]} {
⟦TAB⟧⟦TAB⟧lappend removedEnv $index
⟦TAB⟧⟦TAB⟧set ::env($index) $originalEnv($index)
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧if {[llength $newEnv] > 0} {
⟦TAB⟧    puts [outputChannel] \
⟦TAB⟧⟦TAB⟧    "env array elements created:\t$newEnv"
⟦TAB⟧}
⟦TAB⟧if {[llength $changedEnv] > 0} {
⟦TAB⟧    puts [outputChannel] \
⟦TAB⟧⟦TAB⟧    "env array elements changed:\t$changedEnv"
⟦TAB⟧}
⟦TAB⟧if {[llength $removedEnv] > 0} {
⟦TAB⟧    puts [outputChannel] \
⟦TAB⟧⟦TAB⟧    "env array elements removed:\t$removedEnv"
⟦TAB⟧}

⟦TAB⟧set changedTclPlatform {}
⟦TAB⟧foreach index [array names originalTclPlatform] {
⟦TAB⟧    if {$::tcl_platform($index) \
⟦TAB⟧⟦TAB⟧    != $originalTclPlatform($index)} {
⟦TAB⟧⟦TAB⟧lappend changedTclPlatform $index
⟦TAB⟧⟦TAB⟧set ::tcl_platform($index) $originalTclPlatform($index)
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧if {[llength $changedTclPlatform] > 0} {
⟦TAB⟧    puts [outputChannel] "tcl_platform array elements\
⟦TAB⟧⟦TAB⟧    changed:\t$changedTclPlatform"
⟦TAB⟧}

⟦TAB⟧if {[file exists [file join [workingDirectory] core]]} {
⟦TAB⟧    if {[preserveCore] > 1} {
⟦TAB⟧⟦TAB⟧puts "rename core file (> 1)"
⟦TAB⟧⟦TAB⟧puts [outputChannel] "produced core file! \
⟦TAB⟧⟦TAB⟧⟦TAB⟧Moving file to: \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [temporaryDirectory] core-$testFileName]"
⟦TAB⟧⟦TAB⟧catch {file rename -force \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [workingDirectory] core] \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [temporaryDirectory] core-$testFileName]
⟦TAB⟧⟦TAB⟧} msg
⟦TAB⟧⟦TAB⟧if {[string length $msg] > 0} {
⟦TAB⟧⟦TAB⟧    PrintError "Problem renaming file: $msg"
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧# Print a message if there is a core file and (1) there
⟦TAB⟧⟦TAB⟧# previously wasn't one or (2) the new one is different
⟦TAB⟧⟦TAB⟧# from the old one.

⟦TAB⟧⟦TAB⟧if {[info exists coreModTime]} {
⟦TAB⟧⟦TAB⟧    if {$coreModTime != [file mtime \
⟦TAB⟧⟦TAB⟧⟦TAB⟧    [file join [workingDirectory] core]]} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧puts [outputChannel] "A core file was created!"
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧} else {
⟦TAB⟧⟦TAB⟧    puts [outputChannel] "A core file was created!"
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧     $args
    } else {
⟦TAB⟧# Finding tests only in [testsDirectory] is normal operation.
⟦TAB⟧# This procedure is written to accept multiple directory arguments
⟦TAB⟧# only to satisfy version 1 compatibility.
⟦TAB⟧set matchFiles] {
⟦TAB⟧    set matchFileList [concat $matchFileList \
⟦TAB⟧⟦TAB⟧    [glob -directory $directory -types {b c f p s} \
⟦TAB⟧⟦TAB⟧    -nocomplain -- $match]]
⟦TAB⟧}

⟦TAB⟧# List files in $directory that match patterns to skip.
⟦TAB⟧set skipFileList [list]
⟦TAB⟧foreach skip [skipFiles] {
⟦TAB⟧    set skipFileList [concat $skipFileList \
⟦TAB⟧⟦TAB⟧    [glob -directory $directory -types {b c f p s} \
⟦TAB⟧⟦TAB⟧    -nocomplain -- $skip]]
⟦TAB⟧}

⟦TAB⟧# Add to result list all files in match list and not in skip list
⟦TAB⟧foreach file $matchFileList {
⟦TAB⟧    if {[lsearch -exact $skipFileList $file] == -1} {
⟦TAB⟧⟦TAB⟧lappend matchingFiles $file
⟦TAB⟧    }
⟦TAB⟧}
    }

    if {[llength $matchingFiles] == 0} {
⟦TAB⟧PrintError "No test files remain after applying your match and\
⟦TAB⟧⟦TAB⟧skip patterns!"
    }
    return $matchingFiles
}

# tcltest::GetMatchingDirectories --
#
#⟦TAB⟧Looks at the patterns given to match and skip directories and
#⟦TAB⟧uses them to put together a list of the test directories that we
#⟦TAB⟧should attempt to run.  (Only subdirectories containing an
#⟦TAB⟧"all.tcl" file are put into the list.)
#
# Arguments:
#⟦TAB⟧root directory from which to search
#
# Results:
#⟦TAB⟧The constructed list is returned to the user.  This is used in
#⟦TAB⟧the primary all.tcl file.
#
# Side Effects:
#       None.

proc tcltest::GetMatchingDirectories {rootdir} {

    # Determine the skip list first, to avoid [glob]-ing over subdirectories
    # we're going to throw away anyway.  Be sure we skip the $rootdir if it
    # comes up to avoid infinite loops.
    set skipDirs [list $rootdir]
    foreach pattern [skipDirectories] {
⟦TAB⟧set skipDirs [concat $skipDirs [glob -directory $rootdir -types d \
⟦TAB⟧⟦TAB⟧-nocomplain -- $pattern]]
    }

    # Now step through the matching directories, prune out the skipped ones
    # as you go.
    set matchDirs [list]
    foreach pattern [matchDirectories] {
⟦TAB⟧fo childargv [list]
⟦TAB⟧    foreach opt [Configure] {
⟦TAB⟧⟦TAB⟧if {[string equal $opt -outfile]} {continue}
⟦TAB⟧⟦TAB⟧lappend childargv $opt [Configure $opt]
⟦TAB⟧    }
⟦TAB⟧    set cmd [linsert $childargv 0 | $shell $file]
⟦TAB⟧    if {[catch {
⟦TAB⟧⟦TAB⟧incr numTestFiles
⟦TAB⟧⟦TAB⟧set pipeFd [open $cmd "r"]
⟦TAB⟧⟦TAB⟧while {[gets $pipeFd line] >= 0} {
⟦TAB⟧⟦TAB⟧    if {[regexp [join {
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {^([^:]+):\t}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {Total\t([0-9]+)\t}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {Passed\t([0-9]+)\t}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {Skipped\t([0-9]+)\t}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {Failed\t([0-9]+)}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    } ""] $line null testFile \
⟦TAB⟧⟦TAB⟧⟦TAB⟧    Total Passed Skipped Failed]} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧foreach index {Total Passed Skipped Failed} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧    incr numTests($index) [set $index]
⟦TAB⟧⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧⟦TAB⟧if {$Failed > 0} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧    lappend failFiles $testFile
⟦TAB⟧⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧    } elseif {[regexp [join {
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {^Number of tests skipped }
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {for each constraint:}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {|^\t(\oreach file $testFileFailures {
⟦TAB⟧    puts [outputChannel] "  [file tail $file]\n"
⟦TAB⟧}
    }

    # Checking for subdirectories in which to run tests
    foreach directory [GetMatchingDirectories [testsDirectory]] {
⟦TAB⟧set dir [file tail $directory]
⟦TAB⟧puts [outputChannel] [string repeat ~ 44]
⟦TAB⟧puts [outputChannel] "$dir test began at [eval $timeCmd]\n"
⟦TAB⟧
⟦TAB⟧uplevel 1 [list ::source [file join $directory all.tcl]]
⟦TAB⟧
⟦TAB⟧set endTime [eval $timeCmd]
⟦TAB⟧puts [outputChannel] "\n$dir test ended at $endTime"
⟦TAB⟧puts [outputChannel] ""
⟦TAB⟧puts [outputChannel] [string repeat ~ 44]
    }
    return
}

#####################################################################

# Test utility procs - not used in tcltest, but may be useful for
# testing.

# tcltest::loadTestedCommands --
#
#     Uses the specified script to load the commands to test. Allowed to
#     be empty, as the tested commands could have been compiled into the
#     interpreter.
#
# Arguments
#     none
#
# Results
#     none
#
# Side Effects:
# tion regarding what procs and variables exist.
#
# Arguments:
#⟦TAB⟧none
#
# Results:
#⟦TAB⟧Modifies the variable saveState
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::saveState {} {
    variable saveState
    u extra newlines removed
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::normalizeMsg {msg} {
    regsub "\n$" [string tolower $msg] "" msg
    set msg [string map [list "\n\n" "\n"] $msg]
    return [string map [list "\n\}" "\}"] $msg]
}

# tcltest::makeFile --
#
# Create a new file with the name <name>, and write <contents> to it.
#
# If this file hasn't been created via makeFile since the last time
# cleanupTests was called, add it to the $filesMade list, so it will be
# removed by the next call to cleanupTests.
#
# Arguments:
#⟦TAB⟧contents        content of the new file
#       name            name of the new file
#       directory       directory name for new file
#
# Results:
#⟦TAB⟧absolute path to the file created
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::makeFile {contents name {directory ""}} {
    variable filesMade
    FillFilesExisted

    if {[llength [info level 0]] == 3} {
⟦TAB⟧set directory [temporaryDirectory]
    }

    set fullName [file join $directory $name]

    DebugPuts 3 "[lindex [info level 0] 0]:\
⟦TAB⟧     putting ``$contents'' into $fullName"

    set fd [open $fullName w]
    fconfigure $fd -translation lf
    if {[string equal [string index $contents end] \n]} {
⟦TAB⟧puts -nonewline $fd $contents
    } else {
⟦TAB⟧puts $fd $contents
    }
    close $fd

    if {[lsearch -exact $filesMade $fullName] == -1} {
⟦TAB⟧lappend filesMade $fullName
    }
    return $fullName
}

# tcltest::removeFile --
#
#⟦TAB⟧Removes the named file from the filesystem
#
# Arguments:
#⟦TAB⟧name          file to be removed
#       directory     directory from which to remove file
#
# Results:
#⟦TAB⟧return value from [file delete]
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::removeFile {name {directory ""}} {
    variable filesMade
    FillFilesExisted
    if {[llength [info level 0]] == 2} {
⟦TAB⟧set directory [temporaryDirectory]
    }
    set fullName [file join $directory $name]
    DebugPuts 3 "[lindex [info level 0] 0]: removing $fullName"
    set idx [lsearch -exact $filesMade $fullName]
    set filesMade [lreplace $filesMade $idx $idx]
    if {$idx == -1} {
⟦TAB⟧DebugDo 1 {
⟦TAB⟧    Warn "removeFile removing \"$fullName\":\n  not created by makeFile"
⟦TAB⟧}
    }⟦SPACE⟧
    if {![file isfile $fullName]} {
⟦TAB⟧DebugDo 1 {
⟦TAB⟧    Warn "removeFile removing \"$fullName\":\n  not a file"
⟦TAB⟧}
    }
    return [file delete $fullName]
}

# tcltest::makeDirectory --
#
# Create a new dir with the name <name>.
#
# If this dir hasn't been created via makeDirectory since the last time
# cleanupTests was called, add it to the $directoriesMade list, so it
# will be removed by the next call to cleanupTests.
#
# Arguments:
#       name            name of the new directory
#       directory       directory in which to create new dir
#
# Results:
#⟦TAB⟧absolute path to the directory created
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::makeDirectory {name {directory ""}} {
    variable filesMade
    FillFilesExisted
    if {[llength [info level 0]] == 2} {
⟦TAB⟧set directory [temporaryDirectory]
    }
    set fullName [file join $directory $name]
    DebugPuts 3 "[lindex [info level 0] 0]: creating $fullName"
    file mkdir $fullName
    if {[lsearch -exact $filesMade $fullName] == -1} {
⟦TAB⟧lappend filesMade $fullName
    }
    return $fullName
}

# tcltest::removeDirectory --
#
#⟦TAB⟧Removes a named directory from the file system.
#
# Arguments:
#⟦TAB⟧name          Name of the directory to remove
#       directory     Directory from which to remove
#
# Results:
#⟦TAB⟧return value from [file delete]
#
# Side effects:
#⟦TAB⟧None

proc tcltest::removeDirectory {name {directory ""}} {
    variable filesMade
    FillFilesExisted
    if {[llength [info level 0]] == 2} {
⟦TAB⟧set directory [temporaryDirectory]
    }
    set fullName [file join $directory $name]
    DebugPuts 3 "[lindex [info level 0] 0]: deleting $fullName"
    set idx [lsearch -exact $filesMade $fullName]
    set filesMade [lreplace $filesMade $idx $idx]
    if {$idx == -1} {
⟦TAB⟧DebugDo 1 {
⟦TAB⟧    Warn "removeDirectory removing \"$fullName\":\n  not created\
⟦TAB⟧⟦TAB⟧    by makeDirectory"
⟦TAB⟧}
    }⟦SPACE⟧
    if {![file isdirectory $fullName]} {
⟦TAB⟧DebugDo 1 {
⟦TAB⟧    Warn "removeDirectory removing \"$fullName\":\n  not a directory"
⟦TAB⟧}
    }
    return [file delete -force $fullName]
}

# tcltest::viewFile --
#
#⟦TAB⟧reads the content of a file and returns it
#
# Arguments:
#⟦TAB⟧name of the file to read
#       directory in which file is located
#
# Results:
#⟦TAB⟧content of the named file
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::viewFile {name {directory ""}} {
    FillFilesExisted
    if {[llength [info level 0]] == 2} {
⟦TAB⟧set directory [temporaryDirectory]
    }
    set fullName [file join $directory $name]
    set f [open $fullName]
    set data [read -nonewline $f]
    close $f
    return $data
}

# tcltest::bytestring --
#
# Construct a string that consists of the requested sequence of bytes,
# as opposed to a string of properly formed UTF-8 characters.
# This allows the tester to
# 1. Create denormalized or improperly formed strings to pass to C
#    procedures that are supposed to accept strings with embedded NULL
#    bytes.
# 2. Confirm that a string result has a certain pattern of bytes, for
#    instance to confirm that "\xe0\0" in a Tcl script is stored
#    internally in UTF-8 as the sequence of bytes "\xc3\xa0\xc0\x80".
#
# Generally, it's a bad idea to examine the bytes in a Tcl string or to
# construct improperly formed strings in this manner, because it involves
# exposing that Tcl uses UTF-8 internally.
#
# Arguments:
#⟦TAB⟧string being converted
#
# Results:
#⟦TAB⟧result fom encoding
#
# Side effects:
#⟦TAB⟧None

proc tcltest::bytestring {string} {
    return [encoding convertfrom identity $string]
}

# tcltest::OpenFiles --
#
#⟦TAB⟧used in io tests, uses testchannel
#
# Arguments:
#⟦TAB⟧None.
#
# Results:
#⟦TAB⟧???
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::OpenFiles {} {
    if {[catch {testchannel open} result]} {
⟦TAB⟧return {}
    }
    return $result
}

# tcltest::LeakFiles --
#
#⟦TAB⟧used in io tests, uses testchannel
#
# Arguments:
#⟦TAB⟧None.
#
# Results:
#⟦TAB⟧???
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::LeakFiles {old} {
    if {[catch {testchannel open} new]} {
⟦TAB⟧return {}
    }
    set leak {}
    foreach p $new {
⟦TAB⟧if {[lsearch $old $p] < 0} {
⟦TAB⟧    lappend leak $p
⟦TAB⟧}
    }
    return $leak
}

#
# Internationalization / ISO support procs     -- dl
#

# tcltest::SetIso8859_1_Locale --
#
#⟦TAB⟧used in cmdIL.test, uses testlocale
#
# Arguments:
#⟦TAB⟧None.
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::SetIso8859_1_Locale {} {
    variable previousLocale
    variable isoLocale
    if {[info commands testlocale] != ""} {
⟦TAB⟧set previousLocale [testlocale ctype]
⟦TAB⟧testlocale ctype $isoLocale
    }
    return
}

# tcltest::RestoreLocale --
#
#⟦TAB⟧used in cmdIL.test, uses testlocale
#
# Arguments:
#⟦TAB⟧None.
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::RestoreLocale {} {
    variable previousLocale
    if {[info commands testlocale] != ""} {
⟦TAB⟧testlocale ctype $previousLocale
    }
    return
}

# tcltest::threadReap --
#
#⟦TAB⟧Kill all threads except for the main thread.
#⟦TAB⟧Do nothing if testthread is not defined.
#
# Arguments:
#⟦TAB⟧none.
#
# Results:
#⟦TAB⟧Returns the number of existing threads.
#
# Side Effects:
#       none.
#

proc tcltest::threadReap {} {
    if {[info commands testthread] != {}} {

⟦TAB⟧# testthread built into tcltest

⟦TAB⟧testthread errorproc ThreadNullError
⟦TAB⟧while {[llength [testthread names]] > 1} {
⟦TAB⟧    foreach tid [testthread names] {
⟦TAB⟧⟦TAB⟧if {$tid != [mainThread]} {
⟦TAB⟧⟦TAB⟧    catch {
⟦TAB⟧⟦TAB⟧⟦TAB⟧testthread send -async $tid {testthread exit}
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧    ## Enter a bit a sleep to give the threads enough breathing
⟦TAB⟧    ## room to kill themselves off, otherwise the end up with a
⟦TAB⟧    ## massive queue of repeated events
⟦TAB⟧    after 1
⟦TAB⟧}
⟦TAB⟧testthread errorproc ThreadError
⟦TAB⟧return [llength [testthread names]]
    } elseif {[info commands thread::id] != {}} {
⟦TAB⟧
⟦TAB⟧# Thread extension

⟦TAB⟧thread::errorproc ThreadNullError
⟦TAB⟧while {[llength [thread::names]] > 1} {
⟦TAB⟧    foreach tid [thread::names] {
⟦TAB⟧⟦TAB⟧if {$tid != [mainThread]} {
⟦TAB⟧⟦TAB⟧    catch {thread::send -async $tid {thread::exit}}
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧    ## Enter a bit a sleep to give the threads enough breathing
⟦TAB⟧    ## room to kill themselves off, otherwise the end up with a
⟦TAB⟧    ## massive queue of repeated events
⟦TAB⟧    after 1
⟦TAB⟧}
⟦TAB⟧thread::errorproc ThreadError
⟦TAB⟧return [llength [thread::names]]
    } else {
⟦TAB⟧return 1
    }
    return 0
}

# Initialize the constraints and set up command line arguments
namespace eval tcltest {
    # Define initializers for all the built-in contraint definitions
    DefineConstraintInitializers

    # Set up the constraints in the testConstraints array to be lazily
    # initialized by a registered initializer, or by "false" if no
    # initializer is registered.
    trace variable testConstraints r [namespace code SafeFetch]

    # Only initialize constraints at package load time if an
    # [initConstraintsHook] has been pre-defined.  This is only
    # for compatibility support.  The modern way to add a custom
    # test constraint is to just call the [testConstraint] command
    # straight away, without all this "hook" nonsense.
    if {[string equal [namespace current] \
⟦TAB⟧    [namespace qualifiers [namespace which initConstraintsHook]]]} {
⟦TAB⟧InitConstraints
    } else {
⟦TAB⟧proc initConstraintsHook {} {}
    }

    # Define the standard match commands
    customMatch exact⟦TAB⟧[list string equal]
    customMatch glob⟦TAB⟧[list string match]
    customMatch regexp⟦TAB⟧[list regexp --]

    # If the TCLTEST_OPTIONS environment variable exists, configure
    # tcltest according to the option values it specifies.  This has
    # the effect of resetting tcltest's default configuration.
    proc ConfigureFromEnvironment {} {
⟦TAB⟧upvar #0 env(TCLTEST_OPTIONS) options
⟦TAB⟧if {[catch {llength $options} msg]} {
⟦TAB⟧    Warn "invalid TCLTEST_OPTIONS \"$options\":\n  invalid\
⟦TAB⟧⟦TAB⟧    Tcl list: $msg"
⟦TAB⟧    return
⟦TAB⟧}
⟦TAB⟧if {[llength $::env(TCLTEST_OPTIONS)] % 2} {
⟦TAB⟧    Warn "invalid TCLTEST_OPTIONS: \"$options\":\n  should be\
⟦TAB⟧⟦TAB⟧    -option value ?-option value ...?"
⟦TAB⟧    return
⟦TAB⟧}
⟦TAB⟧if {[catch {eval Configure $::env(TCLTEST_OPTIONS)} msg]} {
⟦TAB⟧    Warn "invalid TCLTEST_OPTIONS: \"$options\":\n  $msg"
⟦TAB⟧    return
⟦TAB⟧}
    }
    if {[info exists ::env(TCLTEST_OPTIONS)]} {
⟦TAB⟧ConfigureFromEnvironment
    }

    proc LoadTimeCmdLineArgParsingRequired {} {
⟦TAB⟧set required false
⟦TAB⟧if {[info exists ::argv] && [lsearch -exact $::argv -help] != -1} {
⟦TAB⟧    # The command line asks for -help, so give it (and exit)
⟦TAB⟧    # right now.  ([configure] does not process -help)
⟦TAB⟧    set required true
⟦TAB⟧}
⟦TAB⟧foreach hook { PrintUsageInfoHook processCmdLineArgsHook
⟦TAB⟧⟦TAB⟧⟦TAB⟧processCmdLineArgsAddFlagsHook } {
⟦TAB⟧    if {[string equal [namespace current] [namespace qualifiers \
⟦TAB⟧⟦TAB⟧    [namespace which $hook]]]} {
⟦TAB⟧⟦TAB⟧set required true
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧proc $hook args {}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $required
    }

    # Only initialize configurable options from the command line arguments
    # at package load time if necessary for backward compatibility.  This
    # lets the tcltest user call [configure] for themselves if they wish.
    # Traces are established for auto-configuration from the command line
    # if any configurable options are accessed before the user calls
    # [configure].
    if {[LoadTimeCmdLineArgParsingRequired]} {
⟦TAB⟧ProcessCmdLineArgs
    } else {
⟦TAB⟧EstablishAutoConfigureTraces
    }

    package provide [namespace tail [namespace current]] $Version
}
```

- real-harness-explicit-source-child, lookup, completion 0, result:

```text-visible
child-provider {8.4.6 8.4 ::} module-code 1 module-result {invalid command name "pwd"} require-code 1 require-result {can't find package tcltest}⟦EOF⟧
```

- real-harness-explicit-source-child, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- real-harness-explicit-source-child: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 14

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
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

    # tests that use threads need to know which is the main thread
    Default mainThread 1
    variable mainThread
    if {[info commands thread::id] != {}} {
⟦TAB⟧set mainThread [thread::id]
    } elseif {[info commands testthread] != {}} {
⟦TAB⟧set mainThread [testthread id]
    }

    # Set workingDirectory to [pwd]. The default output directory for
    # Tcl tests is the working directory.  Whenever this value changes
    # change to that directory.
    variable workingDirectory
    trace variable workingDirectory w \
⟦TAB⟧    [namespace code {cd $workingDirectory ;#}]

    Default workingDirectory [pwd] AcceptAbsolutePath
    proc workingDirectory { {dir ""} } {
⟦TAB⟧variable workingDirectory
⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $workingDirectory
⟦TAB⟧}
⟦TAB⟧set workingDirectory [AcceptAbsolutePath $dir]
    }

    # Set the location of the execuatble
    Default tcltest [info nameofexecutable]
    trace variable tcltest w [namespace code {testConstraint stdio \
⟦TAB⟧    [eval [ConstraintInitializer stdio]] ;#}]

    # save the platform information so it can be restored later
    Default originalTclPlatform [array get ::tcl_platform]

    # If a core file exists, save its modification time.
    if {[file exists [file join [workingDirectory] core]]} {
⟦TAB⟧Default coreModTime \
⟦TAB⟧⟦TAB⟧[file mtime [file join [workingDirectory] core]]
    }

    # stdout and stderr buffers for use when we want to store them
    Default outData {}
    Default errData {}

    # keep track of test level for nested test commands
    variable testLevel 0

    # the variables and procs that existed when saveState was called are
    # stored in a variable of the same name
    Default saveState {}

    # Internationalization support -- used in [SetIso8859_1_Locale] and
    # [RestoreLocale]. Those commands are used in cmdIL.test.

    if {![info exists [namespace current]::isoLocale]} {
⟦TAB⟧variable isoLocale fr
⟦TAB⟧switch -- $::tcl_platform(platform) {
⟦TAB⟧    "unix" {

⟦TAB⟧⟦TAB⟧# Try some 'known' values for some platforms:

⟦TAB⟧⟦TAB⟧switch -exact -- $::tcl_platform(os) {
⟦TAB⟧⟦TAB⟧    "FreeBSD" {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr_FR.ISO_8859-1
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    HP-UX {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr_FR.iso88591
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    Linux -
⟦TAB⟧⟦TAB⟧    IRIX {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    default {

⟦TAB⟧⟦TAB⟧⟦TAB⟧# Works on SunOS 4 and Solaris, and maybe
⟦TAB⟧⟦TAB⟧⟦TAB⟧# others...  Define it to something else on your
⟦TAB⟧⟦TAB⟧⟦TAB⟧# system if you want to test those.

⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale iso_8859_1
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧    "windows" {
⟦TAB⟧⟦TAB⟧set isoLocale French
⟦TAB⟧    }
⟦TAB⟧}
    }

    variable ChannelsWeOpened; array set ChannelsWeOpened {}
    # output goes to stdout by default
    Default outputChannel stdout
    proc outputChannel { {filename ""} } {
⟦TAB⟧variable outputChannel
⟦TAB⟧variable ChannelsWeOpened

⟦TAB⟧# This is very subtle and tricky, so let me try to explain.
⟦TAB⟧# (Hopefully this longer comment will be clear when I come
⟦TAB⟧# back in a few months, unlike its predecessor :) )
⟦TAB⟧#⟦SPACE⟧
⟦TAB⟧# The [outputChannel] command (and underlying variable) have to
⟦TAB⟧# be kept in sync with the [configure -outfile] configuration
⟦TAB⟧# option ( and underlying variable Option(-outfile) ).  This is
⟦TAB⟧# accomplished with a write trace on Option(-outfile) that will
⟦TAB⟧# update [outputChannel] whenver a new value is written.  That
⟦TAB⟧# much is easy.
⟦TAB⟧#
⟦TAB⟧# The trick is that in order to maintain compatibility with
⟦TAB⟧# version 1 of tcltest, we must allow every configuration option
⟦TAB⟧# to get its inital value from command line arguments.  This is
⟦TAB⟧# accomplished by setting initial read traces on all the
⟦TAB⟧# configuration options to parse the command line option the first
⟦TAB⟧# time they are read.  These traces are cancelled whenever the
⟦TAB⟧# program itself calls [configure].
⟦TAB⟧#⟦SPACE⟧
⟦TAB⟧# OK, then so to support tcltest 1 compatibility, it seems we want
⟦TAB⟧# to get the return from [outputFile] to trigger the read traces,
⟦TAB⟧# just in case.
⟦TAB⟧#
⟦TAB⟧# BUT!  A little known feature of Tcl variable traces is that⟦SPACE⟧
⟦TAB⟧# traces are disabled during the handling of other traces.  So,
⟦TAB⟧# if we trigger read traces on Option(-outfile) and that triggers
⟦TAB⟧# command line parsing which turns around and sets an initial
⟦TAB⟧# value for Option(-outfile) -- <whew!> -- the write trace that
⟦TAB⟧# would keep [outputChannel] in sync with that new initial value
⟦TAB⟧# would not fire!
⟦TAB⟧#
⟦TAB⟧# SO, finally, as a workaround, instead of triggering read traces
⟦TAB⟧# by invoking [outputFile], we instead trigger the same set of
⟦TAB⟧# read traces by invoking [debug].  Any command that reads a
⟦TAB⟧# configuration option would do.  [debug] is just a handy one.
⟦TAB⟧# The end result is that we support tcltest 1 compatibility and
⟦TAB⟧# keep outputChannel and -outfile in sync in all cases.
⟦TAB⟧debug

⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $outputChannel
⟦TAB⟧}
⟦TAB⟧if {[info exists ChannelsWeOpened($outputChannel)]} {
⟦TAB⟧    close $outputChannel
⟦TAB⟧    unset ChannelsWeOpened($outputChannel)
⟦TAB⟧}
⟦TAB⟧switch -exact -- $filename {
⟦TAB⟧    stderr -
⟦TAB⟧    stdout {
⟦TAB⟧⟦TAB⟧set outputChannel $filename
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧set outputChannel [open $filename a]
⟦TAB⟧⟦TAB⟧set ChannelsWeOpened($outputChannel) 1

⟦TAB⟧⟦TAB⟧# If we created the file in [temporaryDirectory], then
⟦TAB⟧⟦TAB⟧# [cleanupTests] will delete it, unless we claim it was
⟦TAB⟧⟦TAB⟧# already there.
⟦TAB⟧⟦TAB⟧set outdir [normalizePath [file dirname \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [pwd] $filename]]]
⟦TAB⟧⟦TAB⟧if {[string equal $outdir [temporaryDirectory]]} {
⟦TAB⟧⟦TAB⟧    variable filesExisted
⟦TAB⟧⟦TAB⟧    FillFilesExisted
⟦TAB⟧⟦TAB⟧    set filename [file tail $filename]
⟦TAB⟧⟦TAB⟧    if {[lsearch -exact $filesExisted $filename] == -1} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧lappend filesExisted $filename
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $outputChannel
    }

    # errors go to stderr by default
    Default errorChannel stderr
    proc errorChannel { {filename ""} } {
⟦TAB⟧variable errorChannel
⟦TAB⟧variable ChannelsWeOpened

⟦TAB⟧# This is subtle and tricky.  See the comment above in
⟦TAB⟧# [outputChannel] for a detailed explanation.
⟦TAB⟧debug

⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $errorChannel
⟦TAB⟧}
⟦TAB⟧if {[info exists ChannelsWeOpened($errorChannel)]} {
⟦TAB⟧    close $errorChannel
⟦TAB⟧    unset ChannelsWeOpened($errorChannel)
⟦TAB⟧}
⟦TAB⟧switch -exact -- $filename {
⟦TAB⟧    stderr -
⟦TAB⟧    stdout {
⟦TAB⟧⟦TAB⟧set errorChannel $filename
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧set errorChannel [open $filename a]
⟦TAB⟧⟦TAB⟧set ChannelsWeOpened($errorChannel) 1

⟦TAB⟧⟦TAB⟧# If we created the file in [temporaryDirectory], then
⟦TAB⟧⟦TAB⟧# [cleanupTests] will delete it, unless we claim it was
⟦TAB⟧⟦TAB⟧# already there.
⟦TAB⟧⟦TAB⟧set outdir [normalizePath [file dirname \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [pwd] $filename]]]
⟦TAB⟧⟦TAB⟧if {[string equal $outdir [temporaryDirectory]]} {
⟦TAB⟧⟦TAB⟧    variable filesExisted
⟦TAB⟧⟦TAB⟧    FillFilesExisted
⟦TAB⟧⟦TAB⟧    set filename [file tail $filename]
⟦TAB⟧⟦TAB⟧    if {[lsearch -exact $filesExisted $filename] == -1} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧lappend filesExisted $filename
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $errorChannel
    }

##### Set up the configurable options
    #
    # The configurable options of the package
    variable Option; array set Option {}

    # Usage strings for those options
    variable Usage; array set Usage {}

    # Verification commands for those options
    variable Verify; array set Verify {}

    # Initialize the default values of the configurable options that are
    # historically associated with an exported variable.  If that variable
    # is already set, support compatibility by accepting its pre-set value.
    # Use [trace] to establish ongoing connection between the deprecated
    # exported variable and the modern option kept as a true internal var.
    # Also set up usage string and value testing for the option.
    proc Option {option value usage {verify AcceptAll} {varName {}}} {
⟦TAB⟧variable Option
⟦TAB⟧variable Verify
⟦TAB⟧variable Usage
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧set Usage($option) $usage
⟦TAB⟧set Verify($option) $verify
⟦TAB⟧if {[catch {$verify $value} msg]} {
⟦TAB⟧    return -code error $msg
⟦TAB⟧} else {
⟦TAB⟧    set Option($option) $msg
⟦TAB⟧}
⟦TAB⟧if {[string length $varName]} {
⟦TAB⟧    variable $varName
⟦TAB⟧    if {[info exists $varName]} {
⟦TAB⟧⟦TAB⟧if {[catch {$verify [set $varName]} msg]} {
⟦TAB⟧⟦TAB⟧    return -code error $msg
⟦TAB⟧⟦TAB⟧} else {
⟦TAB⟧⟦TAB⟧    set Option($option) $msg
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧unset $varName
⟦TAB⟧    }
⟦TAB⟧    namespace eval [namespace current] \
⟦TAB⟧    ⟦TAB⟧    [list upvar 0 Option($option) $varName]
⟦TAB⟧    # Workaround for Bug (now Feature Request) 572889.  Grrrr....
⟦TAB⟧    # Track all the variables tied to options
⟦TAB⟧    lappend OptionControlledVariables $varName
⟦TAB⟧    # Later, set auto-configure read traces on all
⟦TAB⟧    # of them, since a single trace on Option does not work.
⟦TAB⟧    proc $varName {{value {}}} [subst -nocommands {
⟦TAB⟧⟦TAB⟧if {[llength [info level 0]] == 2} {
⟦TAB⟧⟦TAB⟧    Configure $option [set value]
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧return [Configure $option]
⟦TAB⟧    }]
⟦TAB⟧}
    }

    proc MatchingOption {option} {
⟦TAB⟧variable Option
⟦TAB⟧set match [array names Option $option*]
⟦TAB⟧switch -- [llength $match] {
⟦TAB⟧    0 {
⟦TAB⟧⟦TAB⟧set sorted [lsort [array names Option]]
⟦TAB⟧⟦TAB⟧set values [join [lrange $sorted 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append values ", or [lindex $sorted end]"
⟦TAB⟧⟦TAB⟧return -code error "unknown option $option: should be\
⟦TAB⟧⟦TAB⟧⟦TAB⟧one of $values"
⟦TAB⟧    }
⟦TAB⟧    1 {
⟦TAB⟧⟦TAB⟧return [lindex $match 0]
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧# Exact match trumps ambiguity
⟦TAB⟧⟦TAB⟧if {[lsearch -exact $match $option] >= 0} {
⟦TAB⟧⟦TAB⟧    return $option
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧set values [join [lrange $match 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append values ", or [lindex $match end]"
⟦TAB⟧⟦TAB⟧return -code error "ambiguous option $option:\
⟦TAB⟧⟦TAB⟧⟦TAB⟧could match $values"
⟦TAB⟧    }
⟦TAB⟧}
    }

    proc EstablishAutoConfigureTraces {} {
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧foreach varName [concat $OptionControlledVariables Option] {
⟦TAB⟧    variable $varName
⟦TAB⟧    trace variable $varName r [namespace code {ProcessCmdLineArgs ;#}]
⟦TAB⟧}
    }

    proc RemoveAutoConfigureTraces {} {
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧foreach varName [concat $OptionControlledVariables Option] {
⟦TAB⟧    variable $varName
⟦TAB⟧    foreach pair [trace vinfo $varName] {
⟦TAB⟧⟦TAB⟧foreach {op cmd} $pair break
⟦TAB⟧⟦TAB⟧if {[string equal r $op]
⟦TAB⟧⟦TAB⟧⟦TAB⟧&& [string match *ProcessCmdLineArgs* $cmd]} {
⟦TAB⟧⟦TAB⟧    trace vdelete $varName $op $cmd
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧# Once the traces are removed, this can become a no-op
⟦TAB⟧proc RemoveAutoConfigureTraces {} {}
    }

    proc Configure args {
⟦TAB⟧variable Option
⟦TAB⟧variable Verify
⟦TAB⟧set n [llength $args]
⟦TAB⟧if {$n == 0} {
⟦TAB⟧    return [lsort [array names Option]]
⟦TAB⟧}
⟦TAB⟧if {$n == 1} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    return $Option($option)
⟦TAB⟧}
⟦TAB⟧while {[llength $args] > 1} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    if {[catch {$Verify($option) [lindex $args 1]} value]} {
⟦TAB⟧⟦TAB⟧return -code error "invalid $option\
⟦TAB⟧⟦TAB⟧⟦TAB⟧value \"[lindex $args 1]\": $value"
⟦TAB⟧    }
⟦TAB⟧    set Option($option) $value
⟦TAB⟧    set args [lrange $args 2 end]
⟦TAB⟧}
⟦TAB⟧if {[llength $args]} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    return -code error "missing value for option $option"
⟦TAB⟧}
    }
    proc configure args {
⟦TAB⟧RemoveAutoConfigureTraces
⟦TAB⟧set code [catch {eval Configure $args} msg]
⟦TAB⟧return -code $code $msg
    }
⟦SPACE⟧⟦SPACE⟧⟦SPACE⟧⟦SPACE⟧
    proc AcceptVerbose { level } {
⟦TAB⟧set level [AcceptList $level]
⟦TAB⟧if {[llength $level] == 1} {
⟦TAB⟧    if {![regexp {^(pass|body|skip|start|error)$} $level]} {
⟦TAB⟧⟦TAB⟧# translate single characters abbreviations to expanded list
⟦TAB⟧⟦TAB⟧set level [string map {p pass b body s skip t start e error} \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[split $level {}]]
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set valid [list]
⟦TAB⟧foreach v $level {
⟦TAB⟧    if {[regexp {^(pass|body|skip|start|error)$} $v]} {
⟦TAB⟧⟦TAB⟧lappend valid $v
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $valid
    }

    proc IsVerbose {level} {
⟦TAB⟧variable Option
⟦TAB⟧return [expr {[lsearch -exact $Option(-verbose) $level] != -1}]
    }

    # Default verbosity is to show bodies of failed tests
    Option -verbose {body error} {
⟦TAB⟧Takes any combination of the values 'p', 's', 'b', 't' and 'e'.
⟦TAB⟧Test suite will display all passed tests if 'p' is specified, all
⟦TAB⟧skipped tests if 's' is specified, the bodies of failed tests if
⟦TAB⟧'b' is specified, and when tests start if 't' is specified.
⟦TAB⟧ErrorInfo is displayed if 'e' is specified.
    } AcceptVerbose verbose

    # Match and skip patterns default to the empty list, except for
    # matchFiles, which defaults to all .test files in the
    # testsDirectory and matchDirectories, which defaults to all
    # directories.
    Option -match * {
⟦TAB⟧Run all tests within the specified files that match one of the
⟦TAB⟧list of glob patterns given.
    } AcceptList match

    Option -skip {} {
⟦TAB⟧Skip all tests within the specified tests (via -match) and files
⟦TAB⟧that match one of the list of glob patterns given.
    } AcceptList skip

    Option -file *.test {
⟦TAB⟧Run tests in all test files that match the glob pattern given.
    } AcceptPattern matchFiles

    # By default, skip files that appear to be SCCS lock files.
    Option -notfile l.*.test {
⟦TAB⟧Skip all test files that match the glob pattern given.
    } AcceptPattern skipFiles

    Option -relateddir * {
⟦TAB⟧Run tests in directories that match the glob pattern given.
    } AcceptPattern matchDirectories

    Option -asidefromdir {} {
⟦TAB⟧Skip tests in directories that match the glob pattern given.
    } AcceptPattern skipDirectories

    # By default, don't save core files
    Option -preservecore 0 {
⟦TAB⟧If 2, save any core files produced during testing in the directory
⟦TAB⟧specified by -tmpdir. If 1, notify the user if core files are
⟦TAB⟧created.
    } AcceptInteger preserveCore

    # debug output doesn't get printed by default; debug level 1 spits
    # up only the tests that were skipped because they didn't match or
    # were specifically skipped.  A debug level of 2 would spit up the
    # tcltest variables and flags provided; a debug level of 3 causes
    # some additional output regarding operations of the test harness.
    # The tcltest package currently implements only up to debug level 3.
    Option -debug 0 {
⟦TAB⟧Internal debug level⟦SPACE⟧
    } AcceptInteger debug

    proc SetSelectedConstraints args {
⟦TAB⟧variable Option
⟦TAB⟧foreach c $Option(-constraints) {
⟦TAB⟧    testConstraint $c 1
⟦TAB⟧}
    }
    Option -constraints {} {
⟦TAB⟧Do not skip the listed constraints listed in -constraints.
    } AcceptList
    trace variable Option(-constraints) w \
⟦TAB⟧    [namespace code {SetSelectedConstraints ;#}]

    # Don't run only the "-constraint" specified tests by default
    proc ClearUnselectedConstraints args {
⟦TAB⟧variable Option
⟦TAB⟧variable testConstraints
⟦TAB⟧if {!$Option(-limitconstraints)} {return}
⟦TAB⟧foreach c [array names testConstraints] {
⟦TAB⟧    if {[lsearch -exact $Option(-constraints) $c] == -1} {
⟦TAB⟧⟦TAB⟧testConstraint $c 0
⟦TAB⟧    }
⟦TAB⟧}
    }
    Option -limitconstraints false {
⟦TAB⟧whether to run only tests with the constraints
    } AcceptBoolean limitConstraints⟦SPACE⟧
    trace variable Option(-limitconstraints) w \
⟦TAB⟧    [namespace code {ClearUnselectedConstraints ;#}]

    # A test application has to know how to load the tested commands
    # into the interpreter.
    Option -load {} {
⟦TAB⟧Specifies the script to load the tested commands.
    } AcceptScript loadScript

    # Default is to run each test file in a separate process
    Option -singleproc 0 {
⟦TAB⟧whether to run all tests in one process
    } AcceptBoolean singleProcess⟦SPACE⟧

    proc AcceptTemporaryDirectory { directory } {
⟦TAB⟧set directory [AcceptAbsolutePath $directory]
⟦TAB⟧if {![file exists $directory]} {
⟦TAB⟧    file mkdir $directory
⟦TAB⟧}
⟦TAB⟧set directory [AcceptDirectory $directory]
⟦TAB⟧if {![file writable $directory]} {
⟦TAB⟧    if {[string equal [workingDirectory] $directory]} {
⟦TAB⟧⟦TAB⟧# Special exception: accept the default value
⟦TAB⟧⟦TAB⟧# even if the directory is not writable
⟦TAB⟧⟦TAB⟧return $directory
⟦TAB⟧    }
⟦TAB⟧    return -code error "\"$directory\" is not writeable"
⟦TAB⟧}
⟦TAB⟧return $directory
    }

    # Directory where files should be created
    Option -tmpdir [workingDirectory] {
⟦TAB⟧Save temporary files in the specified directory.
    } AcceptTemporaryDirectory temporaryDirectory
    trace variable Option(-tmpdir) w \
⟦TAB⟧    [namespace code {normalizePath Option(-tmpdir) ;#}]

    # Tests should not rely on the current working directory.
    # Files that are part of the test suite should be accessed relative
    # to [testsDirectory]
    Option -testdir [workingDirectory] {
⟦TAB⟧Search tests in the specified directory.
    } AcceptDirectory testsDirectory
    trace variable Option(-testdir) w \
⟦TAB⟧    [namespace code {normalizePath Option(-testdir) ;#}]

    proc AcceptLoadFile { file } {
⟦TAB⟧if {[string equal "" $file]} {return $file}
⟦TAB⟧set file [file join [temporaryDirectory] $file]
⟦TAB⟧return [AcceptReadable $file]
    }
    proc ReadLoadScript {args} {
⟦TAB⟧variable Option
⟦TAB⟧if {[string equal "" $Option(-loadfile)]} {return}
⟦TAB⟧set tmp [open $Option(-loadfile) r]
⟦TAB⟧loadScript [read $tmp]
⟦TAB⟧close $tmp
    }
    Option -loadfile {} {
⟦TAB⟧Read the script to load the tested commands from the specified file.
    } AcceptLoadFile loadFile
    trace variable Option(-loadfile) w [namespace code ReadLoadScript]

    proc AcceptOutFile { file } {
⟦TAB⟧if {[string equal stderr $file]} {return $file}
⟦TAB⟧if {[string equal stdout $file]} {return $file}
⟦TAB⟧return [file join [temporaryDirectory] $file]
    }

    # output goes to stdout by default
    Option -outfile stdout {
⟦TAB⟧Send output from test runs to the specified file.
    } AcceptOutFile outputFile
    trace variable Option(-outfile) w \
⟦TAB⟧    [namespace code {outputChannel $Option(-outfile) ;#}]

    # errors go to stderr by default
    Option -errfile stderr {
⟦TAB⟧Send errors from test runs to the specified file.
    } AcceptOutFile errorFile
    trace variable Option(-errfile) w \
⟦TAB⟧    [namespace code {errorChannel $Option(-errfile) ;#}]

}

#####################################################################

# tcltest::Debug* --
#
#     Internal helper procedures to write out debug information
#     dependent on the chosen level. A test shell may overide
#     them, f.e. to redirect the output into a different
#     channel, or even into a GUI.

# tcltest::DebugPuts --
#
#     Prints the specified string if the current debug level is
#     higher than the provided level argument.
#
# Arguments:
#     level   The lowest debug level triggering the output
#     string  The string to print out.
#
# Results:
#     Prints the string. Nothing else is allowed.
#
# Side Effects:
#     None.
#

proc tcltest::DebugPuts {level string} {
    variable debug
    if {$debug >= $level} {
⟦TAB⟧puts $string
    }
    return
}

# tcltest::DebugPArray --
#
#     Prints the contents of the specified array if the current
#       debug level is higher than the provided level argument
#
# Arguments:
#     level           The lowest debug level triggering the output
#     arrayvar        The name of the array to print out.
#
# Results:
#     Prints the contents of the array. Nothing else is allowed.
#
# Side Effects:
#     None.
#

proc tcltest::DebugPArray {level arrayvar} {
    variable debug

    if {$debug >= $level} {
⟦TAB⟧catch {upvar  $arrayvar $arrayvar}
⟦TAB⟧parray $arrayvar
    }
    return
}

# Define our own [parray] in ::tcltest that will inherit use of the [puts]
# defined in ::tcltest.  NOTE: Ought to construct with [info args] and
# [info default], but can't be bothered now.  If [parray] changes, then
# this will need changing too.
auto_load ::parray
proc tcltest::parray {a {pattern *}} [info body ::parray]

# tcltest::DebugDo --
#
#     Executes the script if the current debug level is greater than
#       the provided level argument
#
# Arguments:
#     level   The lowest debug level triggering the execution.
#     script  The tcl script executed upon a debug level high enough.
#
# Results:
#     Arbitrary side effects, dependent on the executed script.
#
# Side Effects:
#     None.
#

proc tcltest::DebugDo {level script} {
    variable debug

    if {$debug >= $level} {
⟦TAB⟧uplevel 1 $script
    }
    return
}

#####################################################################

proc tcltest::Warn {msg} {
    puts [outputChannel] "WARNING: $msg"
}

# tcltest::mainThread
#
#     Accessor command for tcltest variable mainThread.
#
proc tcltest::mainThread { {new ""} } {
    variable mainThread
    if {[llength [info level 0]] == 1} {
⟦TAB⟧return $mainThread
    }
    set mainThread $new
}

# tcltest::testConstraint --
#
#⟦TAB⟧sets a test constraint to a value; to do multiple constraints,
#       call this proc multiple times.  also returns the value of the
#       named constraint if no value was supplied.
#
# Arguments:
#⟦TAB⟧constraint - name of the constraint
#       value - new value for constraint (should be boolean) - if not
#               supplied, this is a query
#
# Results:
#⟦TAB⟧content of tcltest::testConstraints($constraint)
#
# Side effects:
#⟦TAB⟧none

proc tcltest::testConstraint {constraint {value ""}} {
    variable testConstraints
    variable Option
    DebugPuts 3 "entering testConstraint $constraint $value"
    if {[llength [info level 0]] == 2} {
⟦TAB⟧return $testConstraints($constraint)
    }
    # Check for boolean values
    if {[catch {expr {$value && $value}} msg]} {
⟦TAB⟧return -code error $msg
    }
    if {[limitConstraints]⟦SPACE⟧
⟦TAB⟧    && [lsearch -exact $Option(-constraints) $constraint] == -1} {
⟦TAB⟧set value 0
    }
    set testConstraints($constraint) $value
}

# tcltest::interpreter --
#
#⟦TAB⟧the interpreter name stored in tcltest::tcltest
#
# Arguments:
#⟦TAB⟧executable name
#
# Results:
#⟦TAB⟧content of tcltest::tcltest
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::interpreter { {interp ""} } {
    variable tcltest
    if {[llength [info level 0]] == 1} {
⟦TAB⟧return $tcltest
    }
    if {[string equal {} $interp]} {
⟦TAB⟧set tcltest {}
    } else {
⟦TAB⟧set tcltest $interp
    }
}

#####################################################################

# tcltest::AddToSkippedBecause --
#
#⟦TAB⟧Increments the variable used to track how many tests were
#       skipped because of a particular constraint.
#
# Arguments:
#⟦TAB⟧constraint     The name of the constraint to be modified
#
# Results:
#⟦TAB⟧Modifies tcltest::skippedBecause; sets the variable to 1 if
#       didn't previously exist - otherwise, it just increments it.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::AddToSkippedBecause { constraint {value 1}} {
    # add the constraint to the list of constraints that kept tests
    # from running
    variable skippedBecause

    if {[info exists skippedBecause($constraint)]} {
⟦TAB⟧incr skippedBecause($constraint) $value
    } else {
⟦TAB⟧set skippedBecause($constraint) $value
    }
    return
}

# tcltest::PrintError --
#
#⟦TAB⟧Prints errors to tcltest::errorChannel and then flushes that
#       channel, making sure that all messages are < 80 characters per
#       line.
#
# Arguments:
#⟦TAB⟧errorMsg     String containing the error to be printed
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::PrintError {errorMsg} {
    set InitialMessage "Error:  "
    set InitialMsgLen  [string length $InitialMessage]
    puts -nonewline [errorChannel] $InitialMessage

    # Keep track of where the end of the string is.
    set endingIndex [string length $errorMsg]

    if {$endingIndex < (80 - $InitialMsgLen)} {
⟦TAB⟧puts [errorChannel] $errorMsg
    } else {
⟦TAB⟧# Print up to 80 characters on the first line, including the
⟦TAB⟧# InitialMessage.
⟦TAB⟧set beginningIndex [string last " " [string range $errorMsg 0 \
⟦TAB⟧⟦TAB⟧[expr {80 - $InitialMsgLen}]]]
⟦TAB⟧puts [errorChannel] [string range $errorMsg 0 $beginningIndex]

⟦TAB⟧while {![string equal end $beginningIndex]} {
⟦TAB⟧    puts -nonewline [errorChannel] \
⟦TAB⟧⟦TAB⟧    [string repeat " " $InitialMsgLen]
⟦TAB⟧    if {($endingIndex - $beginningIndex)
⟦TAB⟧⟦TAB⟧    < (80 - $InitialMsgLen)} {
⟦TAB⟧⟦TAB⟧puts [errorChannel] [string trim \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[string range $errorMsg $beginningIndex end]]
⟦TAB⟧⟦TAB⟧break
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧set newEndingIndex [expr {[string last " " \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[string range $errorMsg $beginningIndex \
⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧[expr {$beginningIndex
⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧+ (80 - $InitialMsgLen)}]
⟦TAB⟧⟦TAB⟧]] + $beginningIndex}]
⟦TAB⟧⟦TAB⟧if {($newEndingIndex <= 0)
⟦TAB⟧⟦TAB⟧⟦TAB⟧|| ($newEndingIndex <= $beginningIndex)} {
⟦TAB⟧⟦TAB⟧    set newEndingIndex end
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧puts [errorChannel] [string trim \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[string range $errorMsg \
⟦TAB⟧⟦TAB⟧⟦TAB⟧    $beginningIndex $newEndingIndex]]
⟦TAB⟧⟦TAB⟧set beginningIndex $newEndingIndex
⟦TAB⟧    }
⟦TAB⟧}
    }
    flush [errorChannel]
    return
}

# tcltest::SafeFetch --
#
#⟦TAB⟧ The following trace procedure makes it so that we can safely
#        refer to non-existent members of the testConstraints array
#        without causing an error.  Instead, reading a non-existent
#        member will return 0. This is necessary because tests are
#        allowed to use constraint "X" without ensuring that
#        testConstraints("X") is defined.
#
# Arguments:
#⟦TAB⟧n1 - name of the array (testConstraints)
#       n2 - array key value (constraint name)
#       op - operation performed on testConstraints (generally r)
#
# Results:
#⟦TAB⟧none
#
# Side effects:
#⟦TAB⟧sets testConstraints($n2) to 0 if it's referenced but never
#       before used

proc tcltest::SafeFetch {n1 n2 op} {
    variable testConstraints
    DebugPuts 3 "entering SafeFetch $n1 $n2 $op"
    if {[string equal {} $n2]} {return}
    if {![info exists testConstraints($n2)]} {
⟦TAB⟧if {[catch {testConstraint $n2 [eval [ConstraintInitializer $n2]]}]} {
⟦TAB⟧    testConstraint $n2 0
⟦TAB⟧}
    }
}

# tcltest::ConstraintInitializer --
#
#⟦TAB⟧Get or set a script that when evaluated in the tcltest namespace
#⟦TAB⟧will return a boolean value with which to initialize the
#⟦TAB⟧associated constraint.
#
# Arguments:
#⟦TAB⟧constraint - name of the constraint initialized by the script
#⟦TAB⟧script - the initializer script
#
# Results
#⟦TAB⟧boolean value of the constraint - enabled or disabled
#
# Side effects:
#⟦TAB⟧Constraint is initialized for future reference by [test]
proc tcltest::ConstraintInitializer {constraint {script ""}} {
    variable ConstraintInitializer
    DebugPuts 3 "entering ConstraintInitializer $constraint $script"
    if {[llength [info level 0]] == 2} {
⟦TAB⟧return $ConstraintInitializer($constraint)
    }
    # Check for boolean values
    if {![info complete $script]} {
⟦TAB⟧return -code error "ConstraintInitializer must be complete script"
    }
    set ConstraintInitializer($constraint) $script
}

# tcltest::InitConstraints --
#
# Call all registered constraint initializers to force initialization
# of all known constraints.
# See the tcltest man page for the list of built-in constraints defined
# in this procedure.
#
# Arguments:
#⟦TAB⟧none
#
# Results:
#⟦TAB⟧The testConstraints array is reset to have an index for each
#⟦TAB⟧built-in test constraint.
#
# Side Effects:
#       None.
#

proc tcltest::InitConstraints {} {
    variable ConstraintInitializer
    initConstraintsHook
    foreach constraint [array names ConstraintInitializer] {
⟦TAB⟧testConstraint $constraint
    }
}

proc tcltest::DefineConstraintInitializers {} {
    ConstraintInitializer singleTestInterp {singleProcess}

    # All the 'pc' constraints are here for backward compatibility and
    # are not documented.  They have been replaced with equivalent 'win'
    # constraints.

    ConstraintInitializer unixOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) unix}
    ConstraintInitializer macOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) macintosh}
    ConstraintInitializer pcOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) windows}
    ConstraintInitializer winOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) windows}

    ConstraintInitializer unix {testConstraint unixOnly}
    ConstraintInitializer mac {testConstraint macOnly}
    ConstraintInitializer pc {testConstraint pcOnly}
    ConstraintInitializer win {testConstraint winOnly}

    ConstraintInitializer unixOrPc \
⟦TAB⟧    {expr {[testConstraint unix] || [testConst  } else {
⟦TAB⟧ProcessFlags $::argv
    }

    # Spit out everything you know if we're at a debug level 2 or
    # greater
    DebugPuts 2 "Flags passed into tcltest:"
    if {[info exists ::env(TCLTEST_OPTIONS)]} {
⟦TAB⟧DebugPuts 2 \
⟦TAB⟧⟦TAB⟧"    ::env(TCLTEST_OPTIONS): $::env(TCLTEST_OPTIONS)"
    }
    if {[info exists ::argv]} {
⟦TAB⟧DebugPuts 2 "    argv: $::argv"
    }
    DebugPuts    2 "tcltest::debug              = [debug]"
    DebugPuts    2 "tcltest::testsDirectory     = [testsDirectory]"
    DebugPuts    2 "tcltest::workingDirectory   = [workingDirectory]"
    DebugPuts    2 "tcltest::temporaryDirectory = [temporaryDirectory]"
    DebugPuts    2 "tcltest::outputChannel      = [outputChannel]"
    DebugPuts    2 "tcltest::errorChannel       = [errorChannel]"
    DebugPuts    2 "Original environmnding it on to the regular puts.
#
# Arguments:
#⟦TAB⟧same as standard puts
#
# Results:
#⟦TAB⟧none
#
# Side effects:
#       Intercepts puts; data that would otherwise go to stdout, stderr,
#⟦TAB⟧or file channels specified in outputChannel and errorChannel
#⟦TAB⟧does not get sent to the normal puts function.
namespace eval tcltest::Replace {
    namespace export puts
}
proc tcltest::Replace::puts {args} {
    variable [namespace parent]::outData
    variable [namespace parent]::errData
    switch [llength $args] {
⟦TAB⟧1 {
⟦TAB⟧    # Only the string to be printed is specified
⟦TAB⟧    append outData [lindex $args 0]\n
⟦TAB⟧    return
⟦TAB⟧    # return [Puts [lindex $args 0]]
⟦TAB⟧}
⟦TAB⟧2 {
⟦TAB⟧    # Either -nonewline or channelId has been specified
⟦TAB⟧    if {[string equal -nonewline [lindex $args 0]]} {
⟦TAB⟧⟦TAB⟧append outData [lindex $args end]
⟦TAB⟧⟦TAB⟧return
⟦TAB⟧⟦TAB⟧# return [Puts -nonewline [lindex $args end]]
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧set channel [lindex $args 0]
⟦TAB⟧⟦TAB⟧set newline \n
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧3 {
⟦TAB⟧    if {[string equal -nonewline [lindex $args 0]]} {
⟦TAB⟧⟦TAB⟧# Both -nonewline and channelId are specified, unless
⟦TAB⟧⟦TAB⟧# it's an error.  -nonewline is supposed to be argv[0].
⟦TAB⟧⟦TAB⟧set channel [lindex $args 1]
⟦TAB⟧⟦TAB⟧set newline ""
⟦TAB⟧    }
⟦TAB⟧}
    }

    if {[info exists channeed. This attribute is
#⟦TAB⟧⟦TAB⟧⟦TAB⟧optional; default is {}
#   body -⟦TAB⟧        Script to run to carry out the test.  It must
#⟦TAB⟧⟦TAB⟧        return a result that can be checked for
#⟦TAB⟧⟦TAB⟧        correctness.  This attrxpected output sent to stdout.  This attribute
#                       is optional; default is {}.
#   errorOutput -       Expected output sent to stderr.  This attribute
#                       is op  This
#                       attribute is optional; default is {}.
#   cleanup -           Code to run after $script (above).  This
#                       attribute is optional; default is {}.
#   match -             specifies type of matching to do on result,
#                       output, errorOutput; this must be a string
#⟦TAB⟧⟦TAB⟧⟦TAB⟧previously registered by a call to [customMatch].
#⟦TAB⟧⟦TAB⟧⟦TAB⟧The strings exact, glob, and regexp are pre-registered
#⟦TAB⟧⟦TAB⟧⟦TAB⟧by the tcltest package.  Default value is exact.
#
# Arguments:
#   name -⟦TAB⟧⟦TAB⟧Name of test, in the form foo-1.2.
#   description -⟦TAB⟧Short textual description of the test, to
#  ⟦TAB⟧⟦TAB⟧  ⟦TAB⟧help humans understand what it does.
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#       Just about anything is possible depending on the test.
#

proc tcltest::test {name description args} {
    global tcl_platform
    variable testLevel
    variable coreModTime
    DebugPuts 3 "test $name $args"
    DebugDo 1 {
⟦TAB⟧variable TestNames
⟦TAB⟧catch {
⟦TAB⟧    puts "test name '$name' re-usdetermine whether or not to trap output based on whether or not
    # these variables (output & errorOutput) are defined.
    foreach item {constraints setup cleanup body result returnCodes
⟦TAB⟧    match} {
⟦TAB⟧set $item {}
    }

    # Set the default match mode
    set match exact

    # Set the default match values for return codes (0 is the standard
    # expected return value if everything went well; 2 represents
    # 'return' being used in the test script).
    set returnCodes [list 0 2]

    # The old test format can't have a 3rd argument (constraints or
    # script) that starts with '-'.
 Match] $match] == -1} {
⟦TAB⟧    incr testLevel -1
⟦TAB⟧    set sorted [lsort [array names CustomMatch]]
⟦TAB⟧    set values [join [lrange $sorted 0 end-1] ", "]
⟦TAB⟧    append values ", or [lindex $sorted end]"
⟦TAB⟧    return -code error "bad -match value \"$match\":\
⟦TAB⟧⟦TAB⟧    must be $values"
⟦TAB⟧}

⟦TAB⟧# Replace symbolic valies supplied for -returnCodes
⟦TAB⟧foreach {strcode numcode} {ok 0 normal 0 error 1 return 2 break 3 cy.
⟦TAB⟧set result [lindex $args end]
⟦TAB⟧if {[llength $args] == 2} {
⟦TAB⟧    set body [lindex $args 0]
⟦TAB⟧} elseif {[llength $args] == 3} {
⟦TAB⟧    set constraints [lindex $args 0]
⟦TAB⟧    set body [lindex $args 1]
⟦TAB⟧} else {
⟦TAB⟧    incr testLevel -1
⟦TAB⟧    return -code error "wrong # args:\
⟦TAB⟧⟦TAB⟧    should be \"test name desc ?options?\""
⟦TAB⟧}
    }

    if {[Skipped $name $constraints]} {
⟦TAB⟧incr testLevel -1
⟦TAB⟧return
    }

    # Save information about the core file.⟦SPACE⟧⟦SPACE⟧
    if {[preserveCore]} {
⟦TAB⟧if {[file exists [file join [workingDirectory] core]]} {
⟦TAB⟧    set coreModTime [file mtime [file join [workingDirectory] core]]setupFailure [expr {$code != 0}]

    # Only run the test body if the setup was successful
    if {!$setupFailure} {

⟦TAB⟧# Verbose notification of $body start
⟦TAB⟧if {[IsVerbose start]} {
⟦TAB⟧    puts [outputChannel] "---- $name start"
⟦TAB⟧    flush [outputChannel]
⟦TAB⟧}

⟦TAB⟧set command [list [namespace origin RunTest] $name $body]
⟦TAB⟧if {[info exists output] || [info exists errorOutput]} {
⟦TAB⟧    set testResult [uplevel 1 [list [namespace origin Eval] $command 0]]
⟦TAB⟧} else {
⟦TAB⟧    set testResult [uplevel 1 [list [namespace origin Eval] $command 1]]
⟦TAB⟧}
⟦TAB⟧foreach {actualAnswer returnCode} $testResult break
⟦TAB⟧if {$returnCode == 1} {
⟦TAB⟧    set errorInfo(body) $::errorInfo
⟦TAB⟧    set errorCode(body) $::errorCode
⟦TAB⟧}
    }

    # Always run the cleanup script
    set code [catch {uplevel 1 $cleanup} cleanupMsg]
    if {$code == 1} {
⟦TAB⟧set errorInfo(cleanup) $::errorInfo
⟦TAB⟧set errorCode(cleanup) $::errorCode
    }
    set cleanupFailure [expr {$code != 0}]

    set coreFailure 0
    set coreMsg ""
    # check for a core     4 { set msg "Test generated continue exception" }
⟦TAB⟧    default { set msg "Test generated exception" }
⟦TAB⟧}
⟦TAB⟧puts [outputChannel] "---- $msg; Return code was: $returnCode"
⟦TAB⟧puts [outputChannel] "----rInfo(body)"
⟦TAB⟧⟦TAB⟧puts [outputChannel] "---- errorCode: $errorCode(body)"
⟦TAB⟧    }
⟦TAB⟧}
    }
    if {$outputFailure} {
⟦TAB⟧if {$outputCompare} {
⟦TAB⟧    puts [outputChannel] "---- Error testing output: $outputMatch"
⟦TAB⟧} else {
⟦TAB⟧    puts [outputChannel] "---- Output was:\n$outData"
⟦TAB⟧    puts [outputChannel] "---- Output should have been\
⟦TAB⟧⟦TAB⟧    ($match matching):\n$output"
⟦TAB⟧}
    }
    if {$errorFailure} {
⟦TAB⟧if {tput should have\
⟦TAB⟧⟦TAB⟧    been ($match matching):\n$errorOutput"
⟦TAB⟧}
    }
    if {$cleanupFailure} {
⟦TAB⟧puts [outputChannel] "---- Test cleanup failed:\n$cleanupMsg"
⟦TAB⟧if {[info exists errorInfo(cleanup)]} {
⟦TAB⟧    puts [outputChannel] "---- errorInfo(cleanup): $errorInfo(cleanup)"
⟦TAB⟧    puts [outputChannel] "---- errorCode(cleanup): $errorCode(cleanup)"
⟦TAB⟧}
    }
    if {$coreFailure} {
⟦TAB⟧puts [outputChannel] "---- Core file produced while running\
⟦TAB⟧⟦TAB⟧test!  $coreMsg"
    }
    puts [outputChannel] "==== $name FAILED\n"

    incr testLevel -1
    return
}

# Skipped --
#
# Given a test name and it constraints, returns a boolean indicating
# whether the current configuration says the test should be skipped.
#
# Side Effects:  Maintains tally of total tests seen and tests skipped.
#
proc tcltest::Skipped {name constraints} {
    variable testLevel
    variable numTests
    variable testConstraints

    if {$testLevel == 1} {
⟦TAB⟧incr numTests(Total)
    }
    # skip the test if it's name matches an element of skip
    foreach pattern [skip] {
⟦TAB⟧if {[string match $pattern $name]} {
⟦TAB⟧    if {$testLevel == 1} {
⟦TAB⟧⟦TAB⟧incr numTests(Skipped)
⟦TAB⟧⟦TAB⟧DebugDo 1 {AddToSkippedBecause userSpecifiedSkip}
⟦TAB⟧    }
⟦TAB⟧    return 1
⟦TAB⟧}
    }
    # skip the test if it's name doesn't match any element of match
    set ok 0
    foreach pattern [match] {
⟦TAB⟧if {[string match $pattern $name]} {
⟦TAB⟧    set ok 1
⟦TAB⟧    break
⟦TAB⟧}
    }
mTests
    variable numTestFiles
    variable failFiles
    variable skippedBecause
    variable currentFailure
    variable originalEnv
    variable originalTclPlatform
    variable coreModTime

    FillFilesExisted
    set testFileName [file tail [info script]]

    # Call the cleanup hook
    cleanupTestsHook

    # Remove files and directories created by the makeFile and
    # makeDirectory pr$filesMade {
⟦TAB⟧    if {[file exists $file]} {
⟦TAB⟧⟦TAB⟧DebugDo 1 {Warn "cleanupTests deleting $file..."}
⟦TAB⟧⟦TAB⟧catch {file delete -force $file}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set currentFiles {}
⟦TAB⟧foreach file [glob -nocomplain \
⟦TAB⟧⟦TAB⟧-directory [temporaryDirectory] *] {
⟦TAB⟧    lappend currentFiles [file tail $file]
⟦TAB⟧}
⟦TAB⟧set newFiles {}
⟦TAB⟧foreach file $currentFiles {
⟦TAB⟧    if {[lsearch -exact $filesExisted $file] == -1} {
⟦TAB⟧⟦TAB⟧lappend newFiles $file
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set filesExisted $currentFiles
⟦TAB⟧if {[llength $newFiles] > 0} {
⟦TAB⟧    set createdNewFiles($testFileName) $newFiles
⟦TAB⟧}
    }

    if {$calledFromAllFile || $testSingleFile} {

⟦TAB⟧# prir each constraint:"
⟦TAB⟧    foreach constraint [lsort $constraintList] {
⟦TAB⟧⟦TAB⟧puts [outputChannel] \
⟦TAB⟧⟦TAB⟧⟦TAB⟧"\t$skippedBecause($constraint)\t$constraint"
⟦TAB⟧⟦TAB⟧unset skippedBecause($constraint)
⟦TAB⟧    }
⟦TAB⟧}

⟦TAB⟧# report the names of test files in createdNewFiles, and reset
⟦TAB⟧# the array to be empty.

⟦TAB⟧set testFilesThatTurded [lsort [array names createdNewFiles]]
⟦TAB⟧if {[llength $testFilesThatTurded] > 0} {
⟦TAB⟧    puts [outputChannel] "Warning: files left behind:"
⟦TAB⟧    foreach testFile $testFilesThatTurded {
⟦TAB⟧⟦TAB⟧puts [outputChannel] \
⟦TAB⟧⟦TAB⟧⟦TAB⟧"\t$testFile:\t$createdNewFiles($testFile)"
⟦TAB⟧⟦TAB⟧unset createdNewFiles($testFile)
⟦TAB⟧   nteractive mode
⟦TAB⟧# This should be changed to determine if an event
⟦TAB⟧# loop is running, which is the real issue.
⟦TAB⟧# Actually, this doesn't belong here at all.  A package
⟦TAB⟧# really has no business [exit]-ing an application.
⟦TAB⟧if {![catch {package present Tk}] && ![testConstraint interactive]} {
⟦TAB⟧    exit
⟦TAB⟧}
    } else {

⟦TAB⟧# if we're deferring stat-reporting until all files are sourced,
⟦TAB⟧# then add cuatform] {
⟦TAB⟧    if {$::tcl_platform($index) \
⟦TAB⟧⟦TAB⟧    != $originalTclPlatform($index)} {
⟦TAB⟧⟦TAB⟧lappend changedTclPlatform $index
⟦TAB⟧⟦TAB⟧set ::tcl_platform($index) $originalTclPlatform($index)
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧if {[llength $changedTclPlatform] > 0} {
⟦TAB⟧    puts [outputChannel] "tcl_platform array elements\
⟦TAB⟧⟦TAB⟧    changed:\t$changedTclPlatform"
⟦TAB⟧}

⟦TAB⟧if {[file exists [file join [workingDirectory] core]]} {
⟦TAB⟧    if {[preserveCore] > 1} {
⟦TAB⟧⟦TAB⟧puts "rename core file (> 1)"
⟦TAB⟧⟦TAB⟧puts [outputChannel] "produced core file! \
⟦TAB⟧⟦TAB⟧⟦TAB⟧Moving file to: \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [temporaryDirectory] core-$testFileName]"
⟦TAB⟧⟦TAB⟧catch {file rename -force \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [workingDirectory] core] \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [temporaryDirectory] core-$testFileName]
⟦TAB⟧⟦TAB⟧} msg
⟦TAB⟧⟦TAB⟧if {[string length $msg] > 0} {
⟦TAB⟧⟦TAB⟧    PrintError "Problem renaming file: $msg"
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧# Print a message if there is a core file and (1) there
⟦TAB⟧⟦TAB⟧# previously wasn't one or (2) the new one is different
⟦TAB⟧⟦TAB⟧# from the old one.

⟦TAB⟧⟦TAB⟧if {[info exists coreModTime]} {
⟦TAB⟧⟦TAB⟧    if {$coreModTime != [file mtime \
⟦TAB⟧⟦TAB⟧⟦TAB⟧    [file join [workingDirectory] core]]} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧puts [outputChannel] "A core file was created!"
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧} else {
⟦TAB⟧⟦TAB⟧    puts [outputChannel] "A core file was created!"
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
    }
    flush [outputChannel]
    flush [errorChannel]
    return
}

#####################################################################

# Procs that determine which tests/test files to run

# tcltest::GetMatchingFiles
#
#       Looks at the patterns given to match and skip files and uses
#⟦TAB⟧them to put together a list of the tests that will be run.
#
# Arguments:
#       directory to search
#
# Results:
#       The constructed list is returned to the user.  This will
#⟦TAB⟧primarily be used in 'all.tcl' files.  It is used in
#⟦TAB⟧runAllTests.
#
# Side Effects:
#       None

# a lower case version is needed for compatibility with tcltest 1.0
proc tcltest::getMatchingFiles args {eval GetMatchingFiles $args}

proc tcltest::GetMatchingFiles { args } {
    if {[llength $args]} {
⟦TAB⟧set dirLists:
#       None.

proc tcltest::GetMatchingDirectories {rootdir} {

    # Determine the skip list first, to avoid [glob]-ing over subdirectories
    # we're going to throw away anyway.  Be sure we skip the $rootdir if it
    # comes up to avoid infinite loops.
    set skipDirs [list $rootdir]
    foreach pattern [skipDirectories] {
⟦TAB⟧set skipDirs [concat $skipDirs [glob -directory $rootdir -types d \
⟦TAB⟧⟦TAB⟧-nocomplain -- $pattern]]
    }

    # Now step through the matching directories, prune out the skipped ones
    # as you go.
    set matchDirs [list]
    foreach pattern [matchDirectories] {
⟦TAB⟧foreach path [glob -directory $rootdir -types d -nocomplain -- \
⟦TAB⟧⟦TAB⟧$pattern] {
⟦TAB⟧    if {[lsearch -exact $skipDirs $path] == -1} {
⟦TAB⟧⟦TAB⟧set matchDirs [concat $matchDirs [GetMatchingDirectories $path]]
⟦TAB⟧⟦TAB⟧if {[file exists [file join $path all.tcl]]} {
⟦TAB⟧⟦TAB⟧    lappend matchDirs $path
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
    }

    if {[llength $matchDirs] == 0} {
⟦TAB⟧DebugPuts 1 "No test directories remain after applying match\
⟦TAB⟧⟦TAB⟧andoes on
#⟦TAB⟧to source all.tcl files in matching test subdirectories.
#
# Arguments:
#⟦TAB⟧shell being tested
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::runAllTests { {shell ""} } {
    variable testSingleFile
    variable numTestFiles
    variable numTests
    variable failFiles

    FillFilesExisted
    if {[llength [info level 0]] == 1} {
⟦TAB⟧set shell [interpreter]
    }

    set testSingleFile false

    puts [outputChannel] "Tests running in interp:  $shell"
    puts [outputChannel] "Tests located in:  [testsDirectory]"
    puts [outputChannel] "Tests running in:  [workingDirectory]"
    puts [outputChannel] "Temporary files stored in\
⟦TAB⟧    [temporaryDirectory]"

    # [file system] first available in Tcl 8.4
    if {![catch {file system [testsDirectory]} result]
⟦TAB⟧    && ![string equal native [lindex $result 0]]} {
⟦TAB⟧# If we aren't running in the native filesystem, then we must
⟦TAB⟧# run the tests in a single process (via 'source'), because
⟦TAB⟧# trying to run then via a pipe wi}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {Skipped\t([0-9]+)\t}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {Failed\t([0-9]+)}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    } ""] $line null testFile \
⟦TAB⟧⟦TAB⟧⟦TAB⟧    Total Passed Skipped Failed]} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧foreach index {Total Passed Skipped Failed} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧    incr numTests($index) [set $index]
⟦TAB⟧⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧⟦TAB⟧if {$Failed > 0} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧    lappend failFiles $testFile
⟦TAB⟧⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧    } elseif {[regexp [join {
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {^Number of tests skipped }
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {for each constraint:}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {|^\t(\d+)\t(.+)$}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    } ""] $line match skipped constraint]} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧if {[string match \t* $match]} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧    AddToSkippedBecause $constraint $skipped
⟦TAB⟧⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧⟦TAB⟧puts [outputChannel] $line
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧close $pipeFd
⟦TAB⟧    } msg]} {
⟦TAB⟧⟦TAB⟧puts [outputChannel] "Test file error: $msg"
⟦TAB⟧⟦TAB⟧# append the name of the test to a list to be reported
⟦TAB⟧⟦TAB⟧# later
⟦TAB⟧⟦TAB⟧lappend testFileFailures $file
⟦TAB⟧    }
⟦TAB⟧}
    }

    # cleanup
    puts [outputChannel] "\nTests ended at [eval $timeCmd]"
    cleanupTests 1
    if {[info exists testFileFailures]} {
⟦TAB⟧puts [outputChannel] "\nTest files exiting with errors:  \n"
⟦TAB⟧foreach file $testFileFailures {
⟦TAB⟧    puts [outputChannel] "  [file tail $file]\n"
⟦TAB⟧}
    }

    # Checking for subdirectories in which to run tests
    foreach directory [GetMatchingDirectories [testsDirectory]] {
⟦TAB⟧set dir [file tail $directory]
⟦TAB⟧puts [outputChannel] [string repeat ~ 44]
⟦TAB⟧puts [outputChannel] "$dir test began at [eval $timeCmd]\n"
⟦TAB⟧
⟦TAB⟧uplevel 1 [list ::source [file join $directory all.tcl]]
⟦TAB⟧
⟦TAB⟧set endTime [eval $timeCmd]
⟦TAB⟧puts [outputChannel] "\n$dir test ended at $endTime"
⟦TAB⟧puts [outputChannel] ""
⟦TAB⟧puts [outputChannel] [string repeat ~ 44]
    }
    return
}

#####################################################################

# Test utility procs - not used in tcltest, but may be useful for
# testing.

# tcltest::loadTestedCommands --
#
#     Uses the specified script to load the commands to test. Allowed to
#     be empty, as the tested commands could have been compiled into the
#     interpreter.
#
# Arguments
#     none
#
# Results
#     none
#
# Side Effects:
# plevel 1 [list ::set [namespace which -variable saveState]] \
⟦TAB⟧    {[::list [::info procs] [::info vars]]}
    DebugPuts  2 "[lindex [info level 0] 0]: $saveState"
    return
}

# tcltest::restoreState --
#
#⟦TAB⟧Remove procs and variables that didn't exist before the call to
#       [saveState].
#
# Arguments:
#⟦TAB⟧none
#
# Results:
#⟦TAB⟧Removes procs and variables from your environment if they don't
#⟦TAB⟧exist in the saveState variable.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::restoreState {} {
    variable saveState
    foreach p [uplevel 1 {::info procs}] {
⟦TAB⟧if {([lsearch [lindex $saveState 0] $p] < 0)
⟦TAB⟧⟦TAB⟧&& ![string equal [namespace current]::$p \
⟦TAB⟧⟦TAB⟧[uplevel 1 [list ::namespace origin $p]]]} {

⟦TAB⟧    DebugPuts 2 "[lindex [info level 0] 0]: Removing proc $p"
⟦TAB⟧    uplevel 1 [list ::catch [list ::rename $p {}]]
⟦TAB⟧}
    }
    foreach p [uplevel 1 {::info vars}] {
⟦TAB⟧if {[lsearch [lindex $saveState 1] $p] < 0} {
⟦TAB⟧    DebugPuts 2 "[lindex [info level 0] 0]:\
⟦TAB⟧⟦TAB⟧    Removing variable $p"
⟦TAB⟧    uplevel 1 [list ::catch [list ::unset $p]]
⟦TAB⟧}
    }
    return
}

# tcltest::normalizeMsg --
#
#⟦TAB⟧Removes "extra" newlines from a string.
#
# Arguments:
#⟦TAB⟧msg        String to be modified
#
# Results:
#⟦TAB⟧string with extra newlines removed
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::normalizeMsg {msg} {
    regsub "\n$" [string tolower $msg] "" msg
    set msg [string map [list "\n\n" "\n"] $msg]
    return [string map [list "\n\}" "\}"] $msg]
}

# tcltest::makeFile --
#
# Create a new file with the name <name>, and write <contents> to it.
#
# If this file hasn't been created via makeFile since the last time
# cleanupTests was called, add it to the $filesMade list, so it will be
# removed by the next call to cleanupTests.
#
# Arguments:
#⟦TAB⟧contents        content of the new file
#       name            name of the new file
#       directory       directory name for new file
#
# Results:
#⟦TAB⟧absolute path to the file created
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::makeFile {contents name {directory ""}} {
 tory {name {directory ""}} {
    variable filesMade
    FillFilesExisted
    if {[llength [info level 0]] == 2} {
⟦TAB⟧set directory [temporaryDirectory]
    }
    set fullName [file join $directory $name]
    DebugPuts 3 "[lindex [info level 0] 0]: creating $fullName"
    file mkdir $fullName
    if {[lsearch -exact $filesMade $fullName] == -1} {
⟦TAB⟧lappend filesMade $fullName
    }
    return $fullName
}

# tcltest::removeDirectory --
#
#⟦TAB⟧Removes a named directory from the file system.
#
# Arguments:
#⟦TAB⟧name          Name of the directory to remove
#       directory     Directory from which to remove
#
# Results:
#⟦TAB⟧return value from [file delete]
#
# Side effects:
#⟦TAB⟧None

proc tcltest::removeDirectory {name {directory ""}} {
    variable filesMade
    FillFilesExisted
    if {[llength [info level 0]] == 2} {
⟦TAB⟧set directory [temporaryDirectory]
    }
    set fullName [file join $directory $name]
    DebugPuts 3 "[lindex [info level 0] 0]: deleting $fullName"
    set idx [lsearch -exact $filesMade $fullName]
    set filesMade [lreplace $filesMade $idx $idx]
    if {$idx == -1} {
⟦TAB⟧DebugDo 1 {
⟦TAB⟧    Warn "removeDirectory removing \"$fullName\":\n  not created\
⟦TAB⟧⟦TAB⟧    by makeDirectory"
⟦TAB⟧}
    }⟦SPACE⟧
    if {![file isdirectory $fullName]} {
⟦TAB⟧DebugDo 1 {
⟦TAB⟧    Warn "removeDirectory removing \"$fullName\":\n  not a directory"
⟦TAB⟧}
    }
    return [file delete -force $fullName]
}

# tcltest::viewFile --
#
#⟦TAB⟧reads the content of a file and returns it
#
# Arguments:
#⟦TAB⟧name of the file to read
#       directory in which file is located
#
# Results:
#⟦TAB⟧content of the named file
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::viewFile {name {directory ""}} {
    FillFilesExisted
    if {[llength [info level 0]] == 2} {
⟦TAB⟧set directory [temporaryDirectory]
    }
    set fullName [file join $directory $name]
    set f [open $fullName]
    set data [read -nonewline $f]
    close $f
    return $data
}

# tcltest::bytestring --
#
# Construct a string that consists of the requested sequence of bytes,
# as oak
}

#
# Internationalization / ISO support procs     -- dl
#

# tcltest::SetIso8859_1_Locale --
#
#⟦TAB⟧used in cmdIL.test, uses testlocale
#
# Arguments:
#⟦TAB⟧None.
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::SetIso8859_1_Locale {} {
    variable previousLocale
    variable isoLocale
    if {[info commands testlocale] != ""} {
⟦TAB⟧set previousLocale [testlocale ctype]
⟦TAB⟧testlocale ctype fo commands testthread] != {}} {

⟦TAB⟧# testthread built into tcltest

⟦TAB⟧testthread errorproc ThreadNullError
⟦TAB⟧while {[llength [testthread names]] > 1} {
⟦TAB⟧    foreach tid [testthread names] {
⟦TAB⟧⟦TAB⟧if {$tid != [mainThread]} {
⟦TAB⟧⟦TAB⟧    catch {
⟦TAB⟧⟦TAB⟧⟦TAB⟧testthread send -async $tid {testthread exit}
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧    ## Enter a bit a sleep to give the threads enough breathing
⟦TAB⟧    ## room to kill themselves off events
⟦TAB⟧    after 1
⟦TAB⟧}
⟦TAB⟧thread::errorproc ThreadError
⟦TAB⟧return [llength [thread::names]]
    } else {
⟦TAB⟧return 1
    }
    return 0
}

# Initialize the constraints and set up command line arguments
namckage load time if an
    # [initConstraintsHook] has been pre-defined.  This is only
    # for compatibility support.  The modern way to add a custom
    # test constraint is to just call the [testConstraint] command
    # straight away, without all this "hook" nonsense.
    if {[string equal [namespace current] \
⟦TAB⟧    [namespace qualifiers [namespace which initConstraintsHook]]]} {
⟦TAB⟧InitConstraixp⟦TAB⟧[list regexp --]

    # If the TCLTEST_OPTIONS environment variable exists, configure
    # tcltest according to the option values it specifies.  This has
    # the effect of resetting tcltest's default configuration.
    proc ConfigureFromEnvironment {} {
⟦TAB⟧upvar #0 env(TCLTEST_OPTIONS) options
⟦TAB⟧if {[catch {llength $options} msg]} {
⟦TAB⟧    Warn "invalid TCLTEST_OPTIONS \"$options\":\n  invalid\
⟦TAB⟧}
⟦TAB⟧if {[catch {eval Configure $::env(TCLTEST_OPTIONS)} msg]} {
⟦TAB⟧    Warn "invalid TCLTEST_OPTIONS: \"$options\":\n  $msg"
⟦TAB⟧    return
⟦TAB⟧}
    }
    if {[info exists ::env(TCLTEST_OPTIONS)]} {
⟦TAB⟧Configur⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 0, result:

```text-visible
child-provider {8.4.6 8.4 ::} module-code 1 module-result {invalid command name "pwd"} require-code 1 require-result {can't find package tcltest}⟦EOF⟧
```

- real-harness-explicit-source-child, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- real-harness-explicit-source-child: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 15

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
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

    # tests that use threads need to know which is the main thread
    Default mainThread 1
    variable mainThread
    if {[info commands thread::id] != {}} {
⟦TAB⟧set mainThread [thread::id]
    } elseif {[info commands testthread] != {}} {
⟦TAB⟧set mainThread [testthread id]
    }

    # Set workingDirectory to [pwd]. The default output directory for
    # Tcl tests is the working directory.  Whenever this value changes
    # change to that directory.
    variable workingDirectory
    trace variable workingDirectory w \
⟦TAB⟧    [namespace code {cd $workingDirectory ;#}]

    Default workingDirectory [pwd] AcceptAbsolutePath
    proc workingDirectory { {dir ""} } {
⟦TAB⟧variable workingDirectory
⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $workingDirectory
⟦TAB⟧}
⟦TAB⟧set workingDirectory [AcceptAbsolutePath $dir]
    }

    # Set the location of the execuatble
    Default tcltest [info nameofexecutable]
    trace variable tcltest w [namespace code {testConstraint stdio \
⟦TAB⟧    [eval [ConstraintInitializer stdio]] ;#}]

    # save the platform information so it can be restored later
    Default originalTclPlatform [array get ::tcl_platform]

    # If a core file exists, save its modification time.
    if {[file exists [file join [workingDirectory] core]]} {
⟦TAB⟧Default coreModTime \
⟦TAB⟧⟦TAB⟧[file mtime [file join [workingDirectory] core]]
    }

    # stdout and stderr buffers for use when we want to store them
    Default outData {}
    Default errData {}

    # keep track of test level for nested test commands
    variable testLevel 0

    # the variables and procs that existed when saveState was called are
    # stored in a variable of the same name
    Default saveState {}

    # Internationalization support -- used in [SetIso8859_1_Locale] and
    # [RestoreLocale]. Those commands are used in cmdIL.test.

    if {![info exists [namespace current]::isoLocale]} {
⟦TAB⟧variable isoLocale fr
⟦TAB⟧switch -- $::tcl_platform(platform) {
⟦TAB⟧    "unix" {

⟦TAB⟧⟦TAB⟧# Try some 'known' values for some platforms:

⟦TAB⟧⟦TAB⟧switch -exact -- $::tcl_platform(os) {
⟦TAB⟧⟦TAB⟧    "FreeBSD" {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr_FR.ISO_8859-1
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    HP-UX {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr_FR.iso88591
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    Linux -
⟦TAB⟧⟦TAB⟧    IRIX {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    default {

⟦TAB⟧⟦TAB⟧⟦TAB⟧# Works on SunOS 4 and Solaris, and maybe
⟦TAB⟧⟦TAB⟧⟦TAB⟧# others...  Define it to something else on your
⟦TAB⟧⟦TAB⟧⟦TAB⟧# system if you want to test those.

⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale iso_8859_1
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧    "windows" {
⟦TAB⟧⟦TAB⟧set isoLocale French
⟦TAB⟧    }
⟦TAB⟧}
    }

    variable ChannelsWeOpened; array set ChannelsWeOpened {}
    # output goes to stdout by default
    Default outputChannel stdout
    proc outputChannel { {filename ""} } {
⟦TAB⟧variable outputChannel
⟦TAB⟧variable ChannelsWeOpened

⟦TAB⟧# This is very subtle and tricky, so let me try to explain.
⟦TAB⟧# (Hopefully this longer comment will be clear when I come
⟦TAB⟧# back in a few months, unlike its predecessor :) )
⟦TAB⟧#⟦SPACE⟧
⟦TAB⟧# The [outputChannel] command (and underlying variable) have to
⟦TAB⟧# be kept in sync with the [configure -outfile] configuration
⟦TAB⟧# option ( and underlying variable Option(-outfile) ).  This is
⟦TAB⟧# accomplished with a write trace on Option(-outfile) that will
⟦TAB⟧# update [outputChannel] whenver a new value is written.  That
⟦TAB⟧# much is easy.
⟦TAB⟧#
⟦TAB⟧# The trick is that in order to maintain compatibility with
⟦TAB⟧# version 1 of tcltest, we must allow every configuration option
⟦TAB⟧# to get its inital value from command line arguments.  This is
⟦TAB⟧# accomplished by setting initial read traces on all the
⟦TAB⟧# configuration options to parse the command line option the first
⟦TAB⟧# time they are read.  These traces are cancelled whenever the
⟦TAB⟧# program itself calls [configure].
⟦TAB⟧#⟦SPACE⟧
⟦TAB⟧# OK, then so to support tcltest 1 compatibility, it seems we want
⟦TAB⟧# to get the return from [outputFile] to trigger the read traces,
⟦TAB⟧# just in case.
⟦TAB⟧#
⟦TAB⟧# BUT!  A little known feature of Tcl variable traces is that⟦SPACE⟧
⟦TAB⟧# traces are disabled during the handling of other traces.  So,
⟦TAB⟧# if we trigger read traces on Option(-outfile) and that triggers
⟦TAB⟧# command line parsing which turns around and sets an initial
⟦TAB⟧# value for Option(-outfile) -- <whew!> -- the write trace that
⟦TAB⟧# would keep [outputChannel] in sync with that new initial value
⟦TAB⟧# would not fire!
⟦TAB⟧#
⟦TAB⟧# SO, finally, as a workaround, instead of triggering read traces
⟦TAB⟧# by invoking [outputFile], we instead trigger the same set of
⟦TAB⟧# read traces by invoking [debug].  Any command that reads a
⟦TAB⟧# configuration option would do.  [debug] is just a handy one.
⟦TAB⟧# The end result is that we support tcltest 1 compatibility and
⟦TAB⟧# keep outputChannel and -outfile in sync in all cases.
⟦TAB⟧debug

⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $outputChannel
⟦TAB⟧}
⟦TAB⟧if {[info exists ChannelsWeOpened($outputChannel)]} {
⟦TAB⟧    close $outputChannel
⟦TAB⟧    unset ChannelsWeOpened($outputChannel)
⟦TAB⟧}
⟦TAB⟧switch -exact -- $filename {
⟦TAB⟧    stderr -
⟦TAB⟧    stdout {
⟦TAB⟧⟦TAB⟧set outputChannel $filename
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧set outputChannel [open $filename a]
⟦TAB⟧⟦TAB⟧set ChannelsWeOpened($outputChannel) 1

⟦TAB⟧⟦TAB⟧# If we created the file in [temporaryDirectory], then
⟦TAB⟧⟦TAB⟧# [cleanupTests] will delete it, unless we claim it was
⟦TAB⟧⟦TAB⟧# already there.
⟦TAB⟧⟦TAB⟧set outdir [normalizePath [file dirname \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [pwd] $filename]]]
⟦TAB⟧⟦TAB⟧if {[string equal $outdir [temporaryDirectory]]} {
⟦TAB⟧⟦TAB⟧    variable filesExisted
⟦TAB⟧⟦TAB⟧    FillFilesExisted
⟦TAB⟧⟦TAB⟧    set filename [file tail $filename]
⟦TAB⟧⟦TAB⟧    if {[lsearch -exact $filesExisted $filename] == -1} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧lappend filesExisted $filename
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $outputChannel
    }

    # errors go to stderr by default
    Default errorChannel stderr
    proc errorChannel { {filename ""} } {
⟦TAB⟧variable errorChannel
⟦TAB⟧variable ChannelsWeOpened

⟦TAB⟧# This is subtle and tricky.  See the comment above in
⟦TAB⟧# [outputChannel] for a detailed explanation.
⟦TAB⟧debug

⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $errorChannel
⟦TAB⟧}
⟦TAB⟧if {[info exists ChannelsWeOpened($errorChannel)]} {
⟦TAB⟧    close $errorChannel
⟦TAB⟧    unset ChannelsWeOpened($errorChannel)
⟦TAB⟧}
⟦TAB⟧switch -exact -- $filename {
⟦TAB⟧    stderr -
⟦TAB⟧    stdout {
⟦TAB⟧⟦TAB⟧set errorChannel $filename
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧set errorChannel [open $filename a]
⟦TAB⟧⟦TAB⟧set ChannelsWeOpened($errorChannel) 1

⟦TAB⟧⟦TAB⟧# If we created the file in [temporaryDirectory], then
⟦TAB⟧⟦TAB⟧# [cleanupTests] will delete it, unless we claim it was
⟦TAB⟧⟦TAB⟧# already there.
⟦TAB⟧⟦TAB⟧set outdir [normalizePath [file dirname \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [pwd] $filename]]]
⟦TAB⟧⟦TAB⟧if {[string equal $outdir [temporaryDirectory]]} {
⟦TAB⟧⟦TAB⟧    variable filesExisted
⟦TAB⟧⟦TAB⟧    FillFilesExisted
⟦TAB⟧⟦TAB⟧    set filename [file tail $filename]
⟦TAB⟧⟦TAB⟧    if {[lsearch -exact $filesExisted $filename] == -1} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧lappend filesExisted $filename
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $errorChannel
    }

##### Set up the configurable options
    #
    # The configurable options of the package
    variable Option; array set Option {}

    # Usage strings for those options
    variable Usage; array set Usage {}

    # Verification commands for those options
    variable Verify; array set Verify {}

    # Initialize the default values of the configurable options that are
    # historically associated with an exported variable.  If that variable
    # is already set, support compatibility by accepting its pre-set value.
    # Use [trace] to establish ongoing connection between the deprecated
    # exported variable and the modern option kept as a true internal var.
    # Also set up usage string and value testing for the option.
    proc Option {option value usage {verify AcceptAll} {varName {}}} {
⟦TAB⟧variable Option
⟦TAB⟧variable Verify
⟦TAB⟧variable Usage
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧set Usage($option) $usage
⟦TAB⟧set Verify($option) $verify
⟦TAB⟧if {[catch {$verify $value} msg]} {
⟦TAB⟧    return -code error $msg
⟦TAB⟧} else {
⟦TAB⟧    set Option($option) $msg
⟦TAB⟧}
⟦TAB⟧if {[string length $varName]} {
⟦TAB⟧    variable $varName
⟦TAB⟧    if {[info exists $varName]} {
⟦TAB⟧⟦TAB⟧if {[catch {$verify [set $varName]} msg]} {
⟦TAB⟧⟦TAB⟧    return -code error $msg
⟦TAB⟧⟦TAB⟧} else {
⟦TAB⟧⟦TAB⟧    set Option($option) $msg
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧unset $varName
⟦TAB⟧    }
⟦TAB⟧    namespace eval [namespace current] \
⟦TAB⟧    ⟦TAB⟧    [list upvar 0 Option($option) $varName]
⟦TAB⟧    # Workaround for Bug (now Feature Request) 572889.  Grrrr....
⟦TAB⟧    # Track all the variables tied to options
⟦TAB⟧    lappend OptionControlledVariables $varName
⟦TAB⟧    # Later, set auto-configure read traces on all
⟦TAB⟧    # of them, since a single trace on Option does not work.
⟦TAB⟧    proc $varName {{value {}}} [subst -nocommands {
⟦TAB⟧⟦TAB⟧if {[llength [info level 0]] == 2} {
⟦TAB⟧⟦TAB⟧    Configure $option [set value]
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧return [Configure $option]
⟦TAB⟧    }]
⟦TAB⟧}
    }

    proc MatchingOption {option} {
⟦TAB⟧variable Option
⟦TAB⟧set match [array names Option $option*]
⟦TAB⟧switch -- [llength $match] {
⟦TAB⟧    0 {
⟦TAB⟧⟦TAB⟧set sorted [lsort [array names Option]]
⟦TAB⟧⟦TAB⟧set values [join [lrange $sorted 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append values ", or [lindex $sorted end]"
⟦TAB⟧⟦TAB⟧return -code error "unknown option $option: should be\
⟦TAB⟧⟦TAB⟧⟦TAB⟧one of $values"
⟦TAB⟧    }
⟦TAB⟧    1 {
⟦TAB⟧⟦TAB⟧return [lindex $match 0]
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧# Exact match trumps ambiguity
⟦TAB⟧⟦TAB⟧if {[lsearch -exact $match $option] >= 0} {
⟦TAB⟧⟦TAB⟧    return $option
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧set values [join [lrange $match 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append values ", or [lindex $match end]"
⟦TAB⟧⟦TAB⟧return -code error "ambiguous option $option:\
⟦TAB⟧⟦TAB⟧⟦TAB⟧could match $values"
⟦TAB⟧    }
⟦TAB⟧}
    }

    proc EstablishAutoConfigureTraces {} {
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧foreach varName [concat $OptionControlledVariables Option] {
⟦TAB⟧    variable $varName
⟦TAB⟧    trace variable $varName r [namespace code {ProcessCmdLineArgs ;#}]
⟦TAB⟧}
    }

    proc RemoveAutoConfigureTraces {} {
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧foreach varName [concat $OptionControlledVariables Option] {
⟦TAB⟧    variable $varName
⟦TAB⟧    foreach pair [trace vinfo $varName] {
⟦TAB⟧⟦TAB⟧foreach {op cmd} $pair break
⟦TAB⟧⟦TAB⟧if {[string equal r $op]
⟦TAB⟧⟦TAB⟧⟦TAB⟧&& [string match *ProcessCmdLineArgs* $cmd]} {
⟦TAB⟧⟦TAB⟧    trace vdelete $varName $op $cmd
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧# Once the traces are removed, this can become a no-op
⟦TAB⟧proc RemoveAutoConfigureTraces {} {}
    }

    proc Configure args {
⟦TAB⟧variable Option
⟦TAB⟧variable Verify
⟦TAB⟧set n [llength $args]
⟦TAB⟧if {$n == 0} {
⟦TAB⟧    return [lsort [array names Option]]
⟦TAB⟧}
⟦TAB⟧if {$n == 1} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    return $Option($option)
⟦TAB⟧}
⟦TAB⟧while {[llength $args] > 1} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    if {[catch {$Verify($option) [lindex $args 1]} value]} {
⟦TAB⟧⟦TAB⟧return -code error "invalid $option\
⟦TAB⟧⟦TAB⟧⟦TAB⟧value \"[lindex $args 1]\": $value"
⟦TAB⟧    }
⟦TAB⟧    set Option($option) $value
⟦TAB⟧    set args [lrange $args 2 end]
⟦TAB⟧}
⟦TAB⟧if {[llength $args]} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    return -code error "missing value for option $option"
⟦TAB⟧}
    }
    proc configure args {
⟦TAB⟧RemoveAutoConfigureTraces
⟦TAB⟧set code [catch {eval Configure $args} msg]
⟦TAB⟧return -code $code $msg
    }
⟦SPACE⟧⟦SPACE⟧⟦SPACE⟧⟦SPACE⟧
    proc AcceptVerbose { level } {
⟦TAB⟧set level [AcceptList $level]
⟦TAB⟧if {[llength $level] == 1} {
⟦TAB⟧    if {![regexp {^(pass|body|skip|start|error)$} $level]} {
⟦TAB⟧⟦TAB⟧# translate single characters abbreviations to expanded list
⟦TAB⟧⟦TAB⟧set level [string map {p pass b body s skip t start e error} \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[split $level {}]]
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set valid [list]
⟦TAB⟧foreach v $level {
⟦TAB⟧    if {[regexp {^(pass|body|skip|start|error)$} $v]} {
⟦TAB⟧⟦TAB⟧lappend valid $v
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $valid
    }

    proc IsVerbose {level} {
⟦TAB⟧variable Option
⟦TAB⟧return [expr {[lsearch -exact $Option(-verbose) $level] != -1}]
    }

    # Default verbosity is to show bodies of failed tests
    Option -verbose {body error} {
⟦TAB⟧Takes any combination of the values 'p', 's', 'b', 't' and 'e'.
⟦TAB⟧Test suite will display all passed tests if 'p' is specified, all
⟦TAB⟧skipped tests if 's' is specified, the bodies of failed tests if
⟦TAB⟧'b' is specified, and when tests start if 't' is specified.
⟦TAB⟧ErrorInfo is displayed if 'e' is specified.
    } AcceptVerbose verbose

    # Match and skip patterns default to the empty list, except for
    # matchFiles, which defaults to all .test files in the
    # testsDirectory and matchDirectories, which defaults to all
    # directories.
    Option -match * {
⟦TAB⟧Run all tests within the specified files that match one of the
⟦TAB⟧list of glob patterns given.
    } AcceptList match

    Option -skip {} {
⟦TAB⟧Skip all tests within the specified tests (via -match) and files
⟦TAB⟧that match one of the list of glob patterns given.
    } AcceptList skip

    Option -file *.test {
⟦TAB⟧Run tests in all test files that match the glob pattern given.
    } AcceptPattern matchFiles

    # By default, skip files that appear to be SCCS lock files.
    Option -notfile l.*.test {
⟦TAB⟧Skip all test files that match the glob pattern given.
    } AcceptPattern skipFiles

    Option -relateddir * {
⟦TAB⟧Run tests in directories that match the glob pattern given.
    } AcceptPattern matchDirectories

    Option -asidefromdir {} {
⟦TAB⟧Skip tests in directories that match the glob pattern given.
    } AcceptPattern skipDirectories

    # By default, don't save core files
    Option -preservecore 0 {
⟦TAB⟧If 2, save any core files produced during testing in the directory
⟦TAB⟧specified by -tmpdir. If 1, notify the user if core files are
⟦TAB⟧created.
    } AcceptInteger preserveCore

    # debug output doesn't get printed by default; debug level 1 spits
    # up only the tests that were skipped because they didn't match or
    # were specifically skipped.  A debug level of 2 would spit up the
    # tcltest variables and flags provided; a debug level of 3 causes
    # some additional output regarding operations of the test harness.
    # The tcltest package currently implements only up to debug level 3.
    Option -debug 0 {
⟦TAB⟧Internal debug level⟦SPACE⟧
    } AcceptInteger debug

    proc SetSelectedConstraints args {
⟦TAB⟧variable Option
⟦TAB⟧foreach c $Option(-constraints) {
⟦TAB⟧    testConstraint $c 1
⟦TAB⟧}
    }
    Option -constraints {} {
⟦TAB⟧Do not skip the listed constraints listed in -constraints.
    } AcceptList
    trace variable Option(-constraints) w \
⟦TAB⟧    [namespace code {SetSelectedConstraints ;#}]

    # Don't run only the "-constraint" specified tests by default
    proc ClearUnselectedConstraints args {
⟦TAB⟧variable Option
⟦TAB⟧variable testConstraints
⟦TAB⟧if {!$Option(-limitconstraints)} {return}
⟦TAB⟧foreach c [array names testConstraints] {
⟦TAB⟧    if {[lsearch -exact $Option(-constraints) $c] == -1} {
⟦TAB⟧⟦TAB⟧testConstraint $c 0
⟦TAB⟧    }
⟦TAB⟧}
    }
    Option -limitconstraints false {
⟦TAB⟧whether to run only tests with the constraints
    } AcceptBoolean limitConstraints⟦SPACE⟧
    trace variable Option(-limitconstraints) w \
⟦TAB⟧    [namespace code {ClearUnselectedConstraints ;#}]

    # A test application has to know how to load the tested commands
    # into the interpreter.
    Option -load {} {
⟦TAB⟧Specifies the script to load the tested commands.
    } AcceptScript loadScript

    # Default is to run each test file in a separate process
    Option -singleproc 0 {
⟦TAB⟧whether to run all tests in one process
    } AcceptBoolean singleProcess⟦SPACE⟧

    proc AcceptTemporaryDirectory { directory } {
⟦TAB⟧set directory [AcceptAbsolutePath $directory]
⟦TAB⟧if {![file exists $directory]} {
⟦TAB⟧    file mkdir $directory
⟦TAB⟧}
⟦TAB⟧set directory [AcceptDirectory $directory]
⟦TAB⟧if {![file writable $directory]} {
⟦TAB⟧    if {[string equal [workingDirectory] $directory]} {
⟦TAB⟧⟦TAB⟧# Special exception: accept the default value
⟦TAB⟧⟦TAB⟧# even if the directory is not writable
⟦TAB⟧⟦TAB⟧return $directory
⟦TAB⟧    }
⟦TAB⟧    return -code error "\"$directory\" is not writeable"
⟦TAB⟧}
⟦TAB⟧return $directory
    }

    # Directory where files should be created
    Option -tmpdir [workingDirectory] {
⟦TAB⟧Save temporary files in the specified directory.
    } AcceptTemporaryDirectory temporaryDirectory
    trace variable Option(-tmpdir) w \
⟦TAB⟧    [namespace code {normalizePath Option(-tmpdir) ;#}]

    # Tests should not rely on the current working directory.
    # Files that are part of the test suite should be accessed relative
    # to [testsDirectory]
    Option -testdir [workingDirectory] {
⟦TAB⟧Search tests in the specified directory.
    } AcceptDirectory testsDirectory
    trace variable Option(-testdir) w \
⟦TAB⟧    [namespace code {normalizePath Option(-testdir) ;#}]

    proc AcceptLoadFile { file } {
⟦TAB⟧if {[string equal "" $file]} {return $file}
⟦TAB⟧set file [file join [temporaryDirectory] $file]
⟦TAB⟧return [AcceptReadable $file]
    }
    proc ReadLoadScript {args} {
⟦TAB⟧variable Option
⟦TAB⟧if {[string equal "" $Option(-loadfile)]} {return}
⟦TAB⟧set tmp [open $Option(-loadfile) r]
⟦TAB⟧loadScript [read $tmp]
⟦TAB⟧close $tmp
    }
    Option -loadfile {} {
⟦TAB⟧Read the script to load the tested commands from the specified file.
    } AcceptLoadFile loadFile
    trace variable Option(-loadfile) w [namespace code ReadLoadScript]

    proc AcceptOutFile { file } {
⟦TAB⟧if {[string equal stderr $file]} {return $file}
⟦TAB⟧if {[string equal stdout $file]} {return $file}
⟦TAB⟧return [file join [temporaryDirectory] $file]
    }

    # output goes to stdout by default
    Option -outfile stdout {
⟦TAB⟧Send output from test runs to the specified file.
    } AcceptOutFile outputFile
    trace variable Option(-outfile) w \
⟦TAB⟧    [namespace code {outputChannel $Option(-outfile) ;#}]

    # errors go to stderr by default
    Option -errfile stderr {
⟦TAB⟧Send errors from test runs to the specified file.
    } AcceptOutFile errorFile
    trace variable Option(-errfile) w \
⟦TAB⟧    [namespace code {errorChannel $Option(-errfile) ;#}]

}

#####################################################################

# tcltest::Debug* --
#
#     Internal helper procedures to write out debug information
#     dependent on the chosen level. A test shell may overide
#     them, f.e. to redirect the output into a different
#     channel, or even into a GUI.

# tcltest::DebugPuts --
#
#     Prints the specified string if the current debug level is
#     higher than the provided level argument.
#
# Arguments:
#     level   The lowest debug level triggering the output
#     string  The string to print out.
#
# Results:
#     Prints the string. Nothing else is allowed.
#
# Side Effects:
#     None.
#

proc tcltest::DebugPuts {level string} {
    variable debug
    if {$debug >= $level} {
⟦TAB⟧puts $string
    }
    return
}

# tcltest::DebugPArray --
#
#     Prints the contents of the specified array if the current
#       debug level is higher than the provided level argument
#
# Arguments:
#     level           The lowest debug level triggering the output
#     arrayvar        The name of the array to print out.
#
# Results:
#     Prints the contents of the array. Nothing else is allowed.
#
# Side Effects:
#     None.
#

proc tcltest::DebugPArray {level arrayvar} {
    variable debug

    if {$debug >= $level} {
⟦TAB⟧catch {upvar  $arrayvar $arrayvar}
⟦TAB⟧parray $arrayvar
    }
    return
}

# Define our own [parray] in ::tcltest that will inherit use of the [puts]
# defined in ::tcltest.  NOTE: Ought to construct with [info args] and
# [info default], but can't be bothered now.  If [parray] changes, then
# this will need changing too.
auto_load ::parray
proc tcltest::parray {a {pattern *}} [info body ::parray]

# tcltest::DebugDo --
#
#     Executes the script if the current debug level is greater than
#       the provided level argument
#
# Arguments:
#     level   The lowest debug level triggering the execution.
#     scultiple times.  also returns the value of the
#       named constraint if no value was supplied.
#
# Arguments:
#⟦TAB⟧constraint - name of the constraint
#       value - new value for constraint (should b== -1} {
⟦TAB⟧set value 0
    }
    set testConstraints($constraint) $value
}

# tcltest::interpreter --
#
#⟦TAB⟧the interpreter name stored in tcltest::tcltest
#
# Arguments:
#⟦TAB⟧executable name
#
# Results:
# equal {} $interp]} {
⟦TAB⟧set tcltest {}
    } else {
⟦TAB⟧set tcltest $interp
    }
}

#####################################################################

# tcltest::AddToSkippedBecause --
#
#⟦TAB⟧Increments the variable used to track how many tests were
#       skipped because of a particular constraint.
#
# Arguments:
#⟦TAB⟧constraint     The name of the constraint to be modified
#
# Results:
#⟦TAB⟧Modifies tcltest::skippedBecause; sets the variable to 1 if
#       didn't previously exist - otherwise, it just increments it.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::AddToSkippedBecause { constraint {value 1} sure that all messages are < 80 characters per
#       line.
#
# Arguments:
#⟦TAB⟧errorMsg     String containing the error to be printed
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::Prinle {![string equal end $beginningIndex]} {
⟦TAB⟧    puts -nonewline [errorChannel] \
⟦TAB⟧⟦TAB⟧    [string repeat " " $InitialMsgLen]
⟦TAB⟧    if {($endingIndex - $beginningIndex)
⟦TAB⟧⟦TAB⟧    < (80 - $InitialMsgLen)} {
⟦TAB⟧⟦TAB⟧puts [errorChannel] [string trim \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[string range $errorMsg $beginningIndex end]]
⟦TAB⟧⟦TAB⟧break
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧set newEndingIndex [expr {[string last " " \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[string range $errorMsg $beginningIndex \
⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧[expr {$beginningIndex
⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧+ (80 - $InitialMsgLen)}]
⟦TAB⟧⟦TAB⟧]] + $beginningIndex}]
⟦TAB⟧⟦TAB⟧if {($newEndingIndex <= 0)
⟦TAB⟧⟦TAB⟧⟦TAB⟧|| ($newEndingIndex <= $beginningIndex)} {
⟦TAB⟧⟦TAB⟧    set newEndingIndex end
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧puts [errorChannel] [string trim \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[string range $errorMsg \
⟦TAB⟧⟦TAB⟧⟦TAB⟧    $beginningIndex $newEndingIndex]]
⟦TAB⟧⟦TAB⟧set beginningIndex $newEndingIndex
⟦TAB⟧    }
⟦TAB⟧}
    }
    flush [errorChannel]
    return
}

# tcltest::Sah $::tcl_platform(platform)]} {
⟦TAB⟧    set code 0
        }
        if {[string equal windows $::tcl_platform(platform)]} {
⟦TAB⟧    if {[catch {
⟦TAB⟧        set file _tcl_test_remove_me.txt
⟦TAB⟧        makeFile ecok mkdir]] == 0 ||
⟦TAB⟧        [llength [auto_execok fgrep]] == 0 ||
⟦TAB⟧        [llength [auto_execok grep]] == 0 ||
⟦TAB⟧        [llength [auto_execok ps]] == 0
⟦TAB⟧    } {
⟦TAB⟧        set code 0
⟦TAB⟧    }
⟦TAB⟧    removeFile $file
        }
⟦TAB⟧set code
    }

    ConstraintInitializer stdio {
⟦TAB⟧set code 0
⟦TAB⟧if {![catch {set f [open "|[list [interpreter]]" w]}]} {
⟦TAB⟧    if {![catch {puts $f exit}]} {
⟦TAB⟧⟦TAB⟧if {![catch {close
    # on this system.

    ConstraintInitializer socket {
⟦TAB⟧catch {socket} msg
⟦TAB⟧string compare $msg "sockets are not available on this system"
    }

    # Check for internationalization
    ConstraintInitializer hasIsoLocale {
⟦TAB⟧if {[llength [info commands testlocale]] == 0} {
⟦TAB⟧    set code 0
⟦TAB⟧} else {
⟦TAB⟧    set code [string length [SetIso8859_1_Locale]]
⟦TAB⟧    RestoreLocale
⟦TAB⟧}
⟦TAB⟧set code
    }

}
#####################################################################

# Usage and command line arguments processing.

# tcltest::PrintUsageInfo
#
#⟦TAB⟧Prints out the usage information for package tcltest. e]
    PrintUsageInfoHook
}

proc tcltest::Usage { {option ""} } {
    variable Usage
    variable Verify
    if {[llength [info level 0]] == 1} {
⟦TAB⟧set msg "Usage: [file tail [info nameofexecutable]] script "
⟦TAB⟧append msg "?-help? ?flag value? ... \n"
⟦TAB⟧append msg "Available flags (and valid input values) are:"

⟦TAB⟧set max 0
⟦TAB⟧set allOpts [concat -help [Configure]]
⟦TAB⟧foreach opt $allOpts {
⟦TAB⟧    set foo x $length($opt)}
⟦TAB⟧}
⟦TAB⟧set rest [expr {72 - $max}]
⟦TAB⟧foreach opt $allOpts {
⟦TAB⟧    append msg \n$line($opt)
⟦TAB⟧    append msg [string repeat " " [expr {$max - $length($opt)}]]
⟦TAB⟧    set u [string trim $usage($opt)]
⟦TAB⟧    catch {append u "  (default: \[[Configure $opt]])"}
⟦TAB⟧    regsub -all {\s*\n\s*} $u " " u
⟦TAB⟧    while {[string length $u] > $rest} {
⟦TAB⟧⟦TAB⟧set break [string wordstart $u $rest]
⟦TAB⟧⟦TAB⟧if {$break == 0alid list so add it to the result.
            # lappend result [string trim $token]
            append result \{$token\}
            set token {}
        }
    }

    # If the last token has not beeniable, if it matches an element in skip, or if one of the
# elements of "constraints" turns out not to be true.
#
# If testLevel is 1, then this is a top level test, and we record
# pass/fail information; otherwise, this information is not logged and
# is not added to running totals.
#
# Attributes:
#   Only description is a required attribute.  All others are optional.
#   Default values are indicated.
#
#   constraints -⟦TAB⟧A list of one or more keywords, each of which
#⟦TAB⟧⟦TAB⟧⟦TAB⟧must be the name of an element in the array
#⟦TAB⟧⟦TAB⟧⟦TAB⟧"testConstraints".  If any of these elements is
#⟦TAB⟧⟦TAB⟧⟦TAB⟧zero, the test is skippibute is optional;
#                       default is {}
#   result -⟦TAB⟧        Expected result from script.  This attribute is
#                       optional; default is {}.
#   output -            Expected output sent to stdout.  This attribute
#                       is optional; default is {}.
#   errorOutput -       Expected output sent to stderr.  This attribute
#                       is optional; default is {}.
#   returnCodes -       Expected return codes.  This attribute is
#                       optional; default is {0 2}.
#   setup -             Code to run before $script (above).  This
#                       attribute is optional; default is {}.
#   cleanup -           Code to run after $script (above).  This
#                       attribute is optional; default is {}.
#   match -             specifies type of matching to do on result,
#                       output, errorOutput; this must be a string
#⟦TAB⟧⟦TAB⟧⟦TAB⟧previously registered by a call to [customMatch].
#⟦TAB⟧⟦TAB⟧⟦TAB⟧The strings exact, glob, and regexp are pre-registered
#⟦TAB⟧⟦TAB⟧⟦TAB⟧by the tcltest package.  Default value is exact.
#
# Arguments:
#   name -⟦TAB⟧⟦TAB⟧Name of test, in the form foo-1.2.
#   description -⟦TAB⟧Short textual descriptimatchFiles] {
⟦TAB⟧    set matchFileList [concat $matchFileList \
⟦TAB⟧⟦TAB⟧    [glob -directory $directory -types {b c f p s} \
⟦TAB⟧⟦TAB⟧    -nocomplain -- $match]]
⟦TAB⟧}

⟦TAB⟧# List files in $directory that match patterns td to result list all files in match list and not in skip list
⟦TAB⟧foreach file $matchFileList {
⟦TAB⟧    if {[lsearch -exact $skipFileList $file] == -1} {
⟦TAB⟧⟦TAB⟧lappend matchingFiles $file
⟦TAB⟧    }
⟦TAB⟧}
    }

    if {[llength $matchingFiles] == 0} {
⟦TAB⟧PrintError "No test files remain after applying your match and\
⟦TAB⟧⟦TAB⟧skip patterns!"
    }
    return $matchingFiles
}

# tcltest::GetMatchingDirectories --
#
#⟦TAB⟧Lookse are put into the list.)
#
# Arguments:
#⟦TAB⟧root directory from which to search
#
# Results:
#⟦TAB⟧The constructed list is returned to the user.  This is used in
#⟦TAB⟧the primary all.tcl file.
#
# Side Effects:
#       None.

proc tcltest::GetMatchingDirectories {rootdir} {

    # Determine the skip list first, to avoid [glob]-ing over subdirectories
    # we're going to throw away anyway.  Be sure we skip the $rootdir if it
    # comes up to avoid infinite loops.
    set skipDirs [list $rootdir]
    foreach pattern [skipDirectories] {
⟦TAB⟧set skipDirs [concat $skipDirs [glob -directory $rootdir -types d \
⟦TAB⟧⟦TAB⟧-nocomplain -- $pattern]]
    }

    # Now step through the matching directories, prune out the skipped ones
    # as you go.
    set matchDirs [list]
    foreach pattern [matchDirectories] {
⟦TAB⟧foingleFile false

    puts [outputChannel] "Tests running in interp:  $shell"
    puts [outputChannel] "Tests located in:  [testsDirectory]"
    puts [outputChannel] "Tests running in:  [workingDirectory]"
    puts [outputChannel] "Temporary files stored in\
⟦TAB⟧    [temporaryDirectory]"

    # [file system] first available in Tcl 8.4
    if {![catch {file system [testsDirectory]} result]
⟦TAB⟧    && ![string equal native [lindex $result 0]]} {
⟦TAB⟧# If we aren't running in the native filesystem, then we must
⟦TAB⟧# run the tests in a single process (via 'source'), because
⟦TAB⟧# trying to run then via a pipe wi####################################################

# Test utility procs - not used in tcltest, but may be useful for
# testing.

# tcltest::loadTestedCommands --
#
#     Uses the specified script t    none.

proc tcltest::loadTestedCommands {} {
    variable l
    if {[string equal {} [loadScript]]} {
⟦TAB⟧return
    }

    return [uplevel 1 [loadScript]]
}

# tcltest::saveState --
#
#⟦TAB⟧Save information regarding what procs and variables exist.
#
# Arguments:
#⟦TAB⟧none
#
# Results:
#⟦TAB⟧Modifies the variable saveState
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::saveState {} {
    variable saveState
    uplevel 1 [list ::set [namespace which -variable saveState]] \
⟦TAB⟧    {[::list [::info procs] [::info vars]]}
    DebugPuts  2 "[lindex [info level 0] 0]: $saveState"
    return
}

# tcltest::restoreState --
#
#⟦TAB⟧Remove procs and variables that didn't exist before the call to
#       [saveState].
#
# Arguments:
#⟦TAB⟧none
#
# Results:
#⟦TAB⟧Removes procs and variables from your environment if they don't
#⟦TAB⟧exist in the saveState variable.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::restoreState {} {
    variable saveState
    foreach p [uplevel 1 {::info procs}] {
⟦TAB⟧if {([lsearch [lindex $saveState 0] $p] < 0)
⟦TAB⟧⟦TAB⟧&& ![string equal [namespace current]::$p \
⟦TAB⟧⟦TAB⟧[uplevel 1 [list ::namespace origin $p]]]} {

⟦TAB⟧    DebugPuts 2 "[lindex [info level 0] 0]: Removing proc $p"
⟦TAB⟧    uplevel 1 [list ::catch [list ::renamst ::catch [list ::unset $p]]
⟦TAB⟧}
    }
    return
}

# tcltest::normalizeMsg --
#
#⟦TAB⟧Removes "extra" newlines from a string.
#
# Arguments:
#⟦TAB⟧msg        String to be modified
#
# Results:
#⟦TAB⟧string with extra newlines removed
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::normalizeMsg {msg} {
    regsub "\n$" [string tolower $msg] "" msg
    set msg [string map [list "\n\n" "\n"] $msg]
    return [string map [list "\n\}" "\}"] $msg]
}

# tcltest::makeFile --
#
# Create a new file with the name <name>, and write <contents> to it.
#
# If this file hasn't been created via makeFile since the last time
# cDirectory]
    }
    set fullName [file join $directory $name]
    DebugPuts 3 "[lindex [info level 0] 0]: removing $fullName"
    set idx [lsearch -exact $filesMade $fullName]
    set filesMade [lrep         name of the new directory
#       directory       directory in which to create new dir
#
# Results:
#⟦TAB⟧absolute path to the directory created
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::makeDirecsMade $fullName]
    set filesMade [lreplace $filesMade $idx $idx]
    if {$idx == -1} {
⟦TAB⟧DebugDo 1 {
⟦TAB⟧    Warn "removeDirectory removing \"$fullName\":\n  not created\
⟦TAB⟧⟦TAB⟧    by makeDirectory"
⟦TAB⟧}
    e --
#
#⟦TAB⟧reads the content of a file and returns it
#
# Arguments:
#⟦TAB⟧name of the file to read
#       directory in which file is located
#
# Results:
#⟦TAB⟧content of the named file
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::viewFile {name {directory ""}} {
    FillFilesExisted
    if {[llength [info level 0]] == 2} {
⟦TAB⟧set directory [temporaryDirectory]
    }
    set fullName [file join $directory $namepposed to a string of properly formed UTF-8 characters.
# This allows the tester to
# 1. Create denormalized or improperly formed strings to pass to C
#    procedures that are supposed to accept strings with embedded NULL
#    bytes.
# 2. Confirm that a string result has a certain pattern of bytes, for
#    instance to confirm that "\xe0\0" in a Tcl script is stored
#    internally in UTF-8 as the sequence of bytes "\xc3\xa0\xc0\x80".
#
# Generally, it's a bad idea to examine the bytes in a Tcl string or to
# construct improperly formed strings in this manner, because it involves
# exposing that Tcl uses UTF-8 internally.
#
# Arguments:
#⟦TAB⟧string being converted
#
# Results:
#⟦TAB⟧result fom encoding
#
# Side effects:
#⟦TAB⟧None

proc tcltest::bytestring {string} {
    return [encoding convertfrom stchannel open} result]} {
⟦TAB⟧return {}
    }
    return $result
}

# tcltest::LeakFiles --
#
#⟦TAB⟧used in io tests, uses testchannel
#
# Arguments:
#⟦TAB⟧None.
#
# Results:
#⟦TAB⟧???
#
# Side effects:
#⟦TAB⟧None.

pr= [mainThread]} {
⟦TAB⟧⟦TAB⟧    catch {
⟦TAB⟧⟦TAB⟧⟦TAB⟧testthread send -async $tid {testthread exit}
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧    ## Enter a bit a sleep to give the threads enough breathing
⟦TAB⟧    ## room to kill themselves off= {}} {
⟦TAB⟧
⟦TAB⟧# Thread extension

⟦TAB⟧thread::errorproc ThreadNullError
⟦TAB⟧while {[llength [thread::names]] > 1} {
⟦TAB⟧    foreach tid [thread::names] {
⟦TAB⟧⟦TAB⟧if {$tid != [mainThread]} {
⟦TAB⟧⟦TAB⟧    catch {thread::send -async $tid {thread::exit}}
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧    ## Enter a bit a sleep to give the threads enough breathing
⟦TAB⟧    ## room to kill themselves off, otherwise the end up with a
⟦TAB⟧    ## massive queue of repeatedespace eval tcltest {
    # Define initializers for all the built-in contraint definitions
    DefineConstraintInitializers

    # Set up the constraints in the testConstraints array to be lazily
    # initialized by a registered initializer, or by "false" if no
    # initializer is registered.
    trace variable testConstraints r [namespace code SafeFetch]

    # Only initialize constraints at panstraint] command
    # straight away, without all this "hook" nonsense.
    if {[string equal [namespace current] \
⟦TAB⟧    [namespace qualifiers [namespace which initConstraintsHook]]]} {
⟦TAB⟧InitConstraints
    } else {
⟦TAB⟧proc initConstraintsHook {} {}
    }

    # Define the standard match commands
    customMatch exact⟦TAB⟧[list string equal]
    customMatch glob⟦TAB⟧[list string match]
    customMatch regeeFromEnvironment
    }

    proc LoadTimeCmdLineArgParsingRequired {} {
⟦TAB⟧set required false
⟦TAB⟧if {[info exists ::argv] && [lsearch -exact $::argv -help] != -1} {
⟦TAB⟧    # The command line asks for -help, so give it (and exit)
⟦TAB⟧    # right now.  ([configure] does not process -help)
⟦TAB⟧    set required true
⟦TAB⟧}
⟦TAB⟧foreach hook { PrintUsageInfoHook processCmdLineArgsHook
⟦TAB⟧⟦TAB⟧⟦TAB⟧processCmdLineArgsAddFlagsHook } {
⟦TAB⟧    if {[string equal [namespace current] [namespace qualifiers \
⟦TAB⟧⟦TAB⟧    [namespace which $hook]]]} {
⟦TAB⟧⟦TAB⟧set required true
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧proc $hook args {}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $required
    }

    # O⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 0, result:

```text-visible
child-provider {8.4.6 8.4 ::} module-code 1 module-result {invalid command name "pwd"} require-code 1 require-result {can't find package tcltest}⟦EOF⟧
```

- real-harness-explicit-source-child, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- real-harness-explicit-source-child: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 2

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
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

    # tests that use threads need to know which is the main thread
    Default mainThread 1
    variable mainThread
    if {[info commands thread::id] != {}} {
⟦TAB⟧set mainThread [thread::id]
    } elseif {[info commands testthread] != {}} {
⟦TAB⟧set mainThread [testthread id]
    }

    # Set workingDirectory to [pwd]. The default output directory for
    # Tcl tests is the working directory.  Whenever this value changes
    # change to that directory.
    variable workingDirectory
    trace variable workingDirectory w \
⟦TAB⟧    [namespace code {cd $workingDirectory ;#}]

    Default workingDirectory [pwd] AcceptAbsolutePath
    proc workingDirectory { {dir ""} } {
⟦TAB⟧variable workingDirectory
⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $workingDirectory
⟦TAB⟧}
⟦TAB⟧set workingDirectory [AcceptAbsolutePath $dir]
    }

    # Set the location of the execuatble
    Default tcltest [info nameofexecutable]
    trace variable tcltest w [namespace code {testConstraint stdio \
⟦TAB⟧    [eval [ConstraintInitializer stdio]] ;#}]

    # save the platform information so it can be restored later
    Default originalTclPlatform [array get ::tcl_platform]

    # If a core file exists, save its modification time.
    if {[file exists [file join [workingDirectory] core]]} {
⟦TAB⟧Default coreModTime \
⟦TAB⟧⟦TAB⟧[file mtime [file join [workingDirectory] core]]
    }

    # stdout and stderr buffers for use when we want to store them
    Default outData {}
    Default errData {}

    # keep track of test level for nested test commands
    variable testLevel 0

    # the variables and procs that existed when saveState was called are
    # stored in a variable of the same name
    Default saveState {}

    # Internationalization support -- used in [SetIso8859_1_Locale] and
    # [RestoreLocale]. Those commands are used in cmdIL.test.

    if {![info exists [namespace current]::isoLocale]} {
⟦TAB⟧variable isoLocale fr
⟦TAB⟧switch -- $::tcl_platform(platform) {
⟦TAB⟧    "unix" {

⟦TAB⟧⟦TAB⟧# Try some 'known' values for some platforms:

⟦TAB⟧⟦TAB⟧switch -exact -- $::tcl_platform(os) {
⟦TAB⟧⟦TAB⟧    "FreeBSD" {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr_FR.ISO_8859-1
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    HP-UX {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr_FR.iso88591
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    Linux -
⟦TAB⟧⟦TAB⟧    IRIX {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    default {

⟦TAB⟧⟦TAB⟧⟦TAB⟧# Works on SunOS 4 and Solaris, and maybe
⟦TAB⟧⟦TAB⟧⟦TAB⟧# others...  Define it to something else on your
⟦TAB⟧⟦TAB⟧⟦TAB⟧# system if you want to test those.

⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale iso_8859_1
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧    "windows" {
⟦TAB⟧⟦TAB⟧set isoLocale French
⟦TAB⟧    }
⟦TAB⟧}
    }

    variable ChannelsWeOpened; array set ChannelsWeOpened {}
    # output goes to stdout by default
    Default outputChannel stdout
    proc outputChannel { {filename ""} } {
⟦TAB⟧variable outputChannel
⟦TAB⟧variable ChannelsWeOpened

⟦TAB⟧# This is very subtle and tricky, so let me try to explain.
⟦TAB⟧# (Hopefully this longer comment will be clear when I come
⟦TAB⟧# back in a few months, unlike its predecessor :) )
⟦TAB⟧#⟦SPACE⟧
⟦TAB⟧# The [outputChannel] command (and underlying variable) have to
⟦TAB⟧# be kept in sync with the [configure -outfile] configuration
⟦TAB⟧# option ( and underlying variable Option(-outfile) ).  This is
⟦TAB⟧# accomplished with a write trace on Option(-outfile) that will
⟦TAB⟧# update [outputChannel] whenver a new value is written.  That
⟦TAB⟧# much is easy.
⟦TAB⟧#
⟦TAB⟧# The trick is that in order to maintain compatibility with
⟦TAB⟧# version 1 of tcltest, we must allow every configuration option
⟦TAB⟧# to get its inital value from command line arguments.  This is
⟦TAB⟧# accomplished by setting initial read traces on all the
⟦TAB⟧# configuration options to parse the command line option the first
⟦TAB⟧# time they are read.  These traces are cancelled whenever the
⟦TAB⟧# program itself calls [configure].
⟦TAB⟧#⟦SPACE⟧
⟦TAB⟧# OK, then so to support tcltest 1 compatibility, it seems we want
⟦TAB⟧# to get the return from [outputFile] to trigger the read traces,
⟦TAB⟧# just in case.
⟦TAB⟧#
⟦TAB⟧# BUT!  A little known feature of Tcl variable traces is that⟦SPACE⟧
⟦TAB⟧# traces are disabled during the handling of other traces.  So,
⟦TAB⟧# if we trigger read traces on Option(-outfile) and that triggers
⟦TAB⟧# command line parsing which turns around and sets an initial
⟦TAB⟧# value for Option(-outfile) -- <whew!> -- the write trace that
⟦TAB⟧# would keep [outputChannel] in sync with that new initial value
⟦TAB⟧# would not fire!
⟦TAB⟧#
⟦TAB⟧# SO, finally, as a workaround, instead of triggering read traces
⟦TAB⟧# by invoking [outputFile], we instead trigger the same set of
⟦TAB⟧# read traces by invoking [debug].  Any command that reads a
⟦TAB⟧# configuration option would do.  [debug] is just a handy one.
⟦TAB⟧# The end result is that we support tcltest 1 compatibility and
⟦TAB⟧# keep outputChannel and -outfile in sync in all cases.
⟦TAB⟧debug

⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $outputChannel
⟦TAB⟧}
⟦TAB⟧if {[info exists ChannelsWeOpened($outputChannel)]} {
⟦TAB⟧    close $outputChannel
⟦TAB⟧    unset ChannelsWeOpened($outputChannel)
⟦TAB⟧}
⟦TAB⟧switch -exact -- $filename {
⟦TAB⟧    stderr -
⟦TAB⟧    stdout {
⟦TAB⟧⟦TAB⟧set outputChannel $filename
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧set outputChannel [open $filename a]
⟦TAB⟧⟦TAB⟧set ChannelsWeOpened($outputChannel) 1

⟦TAB⟧⟦TAB⟧# If we created the file in [temporaryDirectory], then
⟦TAB⟧⟦TAB⟧# [cleanupTests] will delete it, unless we claim it was
⟦TAB⟧⟦TAB⟧# already there.
⟦TAB⟧⟦TAB⟧set outdir [normalizePath [file dirname \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [pwd] $filename]]]
⟦TAB⟧⟦TAB⟧if {[string equal $outdir [temporaryDirectory]]} {
⟦TAB⟧⟦TAB⟧    variable filesExisted
⟦TAB⟧⟦TAB⟧    FillFilesExisted
⟦TAB⟧⟦TAB⟧    set filename [file tail $filename]
⟦TAB⟧⟦TAB⟧    if {[lsearch -exact $filesExisted $filename] == -1} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧lappend filesExisted $filename
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $outputChannel
    }

    # errors go to stderr by default
    Default errorChannel stderr
    proc errorChannel { {filename ""} } {
⟦TAB⟧variable errorChannel
⟦TAB⟧variable ChannelsWeOpened

⟦TAB⟧# This is subtle and tricky.  See the comment above in
⟦TAB⟧# [outputChannel] for a detailed explanation.
⟦TAB⟧debug

⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $errorChannel
⟦TAB⟧}
⟦TAB⟧if {[info exists ChannelsWeOpened($errorChannel)]} {
⟦TAB⟧    close $errorChannel
⟦TAB⟧    unset ChannelsWeOpened($errorChannel)
⟦TAB⟧}
⟦TAB⟧switch -exact -- $filename {
⟦TAB⟧    stderr -
⟦TAB⟧    stdout {
⟦TAB⟧⟦TAB⟧set errorChannel $filename
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧set errorChannel [open $filename a]
⟦TAB⟧⟦TAB⟧set ChannelsWeOpened($errorChannel) 1

⟦TAB⟧⟦TAB⟧# If we created the file in [temporaryDirectory], then
⟦TAB⟧⟦TAB⟧# [cleanupTests] will delete it, unless we claim it was
⟦TAB⟧⟦TAB⟧# already there.
⟦TAB⟧⟦TAB⟧set outdir [normalizePath [file dirname \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [pwd] $filename]]]
⟦TAB⟧⟦TAB⟧if {[string equal $outdir [temporaryDirectory]]} {
⟦TAB⟧⟦TAB⟧    variable filesExisted
⟦TAB⟧⟦TAB⟧    FillFilesExisted
⟦TAB⟧⟦TAB⟧    set filename [file tail $filename]
⟦TAB⟧⟦TAB⟧    if {[lsearch -exact $filesExisted $filename] == -1} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧lappend filesExisted $filename
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $errorChannel
    }

##### Set up the configurable options
    #
    # The configurable options of the package
    variable Option; array set Option {}

    # Usage strings for those options
    variable Usage; array set Usage {}

    # Verification commands for those options
    variable Verify; array set Verify {}

    # Initialize the default values of the configurable options that are
    # historically associated with an exported variable.  If that variable
    # is already set, support compatibility by accepting its pre-set value.
    # Use [trace] to establish ongoing connection between the deprecated
    # exported variable and the modern option kept as a true internal var.
    # Also set up usage string and value testing for the option.
    proc Option {option value usage {verify AcceptAll} {varName {}}} {
⟦TAB⟧variable Option
⟦TAB⟧variable Verify
⟦TAB⟧variable Usage
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧set Usage($option) $usage
⟦TAB⟧set Verify($option) $verify
⟦TAB⟧if {[catch {$verify $value} msg]} {
⟦TAB⟧    return -code error $msg
⟦TAB⟧} else {
⟦TAB⟧    set Option($option) $msg
⟦TAB⟧}
⟦TAB⟧if {[string length $varName]} {
⟦TAB⟧    variable $varName
⟦TAB⟧    if {[info exists $varName]} {
⟦TAB⟧⟦TAB⟧if {[catch {$verify [set $varName]} msg]} {
⟦TAB⟧⟦TAB⟧    return -code error $msg
⟦TAB⟧⟦TAB⟧} else {
⟦TAB⟧⟦TAB⟧    set Option($option) $msg
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧unset $varName
⟦TAB⟧    }
⟦TAB⟧    namespace eval [namespace current] \
⟦TAB⟧    ⟦TAB⟧    [list upvar 0 Option($option) $varName]
⟦TAB⟧    # Workaround for Bug (now Feature Request) 572889.  Grrrr....
⟦TAB⟧    # Track all the variables tied to options
⟦TAB⟧    lappend OptionControlledVariables $varName
⟦TAB⟧    # Later, set auto-configure read traces on all
⟦TAB⟧    # of them, since a single trace on Option does not work.
⟦TAB⟧    proc $varName {{value {}}} [subst -nocommands {
⟦TAB⟧⟦TAB⟧if {[llength [info level 0]] == 2} {
⟦TAB⟧⟦TAB⟧    Configure $option [set value]
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧return [Configure $option]
⟦TAB⟧    }]
⟦TAB⟧}
    }

    proc MatchingOption {option} {
⟦TAB⟧variable Option
⟦TAB⟧set match [array names Option $option*]
⟦TAB⟧switch -- [llength $match] {
⟦TAB⟧    0 {
⟦TAB⟧⟦TAB⟧set sorted [lsort [array names Option]]
⟦TAB⟧⟦TAB⟧set values [join [lrange $sorted 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append values ", or [lindex $sorted end]"
⟦TAB⟧⟦TAB⟧return -code error "unknown option $option: should be\
⟦TAB⟧⟦TAB⟧⟦TAB⟧one of $values"
⟦TAB⟧    }
⟦TAB⟧    1 {
⟦TAB⟧⟦TAB⟧return [lindex $match 0]
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧# Exact match trumps ambiguity
⟦TAB⟧⟦TAB⟧if {[lsearch -exact $match $option] >= 0} {
⟦TAB⟧⟦TAB⟧    return $option
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧set values [join [lrange $match 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append values ", or [lindex $match end]"
⟦TAB⟧⟦TAB⟧return -code error "ambiguous option $option:\
⟦TAB⟧⟦TAB⟧⟦TAB⟧could match $values"
⟦TAB⟧    }
⟦TAB⟧}
    }

    proc EstablishAutoConfigureTraces {} {
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧foreach varName [concat $OptionControlledVariables Option] {
⟦TAB⟧    variable $varName
⟦TAB⟧    trace variable $varName r [namespace code {ProcessCmdLineArgs ;#}]
⟦TAB⟧}
    }

    proc RemoveAutoConfigureTraces {} {
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧foreach varName [concat $OptionControlledVariables Option] {
⟦TAB⟧    variable $varName
⟦TAB⟧    foreach pair [trace vinfo $varName] {
⟦TAB⟧⟦TAB⟧foreach {op cmd} $pair break
⟦TAB⟧⟦TAB⟧if {[string equal r $op]
⟦TAB⟧⟦TAB⟧⟦TAB⟧&& [string match *ProcessCmdLineArgs* $cmd]} {
⟦TAB⟧⟦TAB⟧    trace vdelete $varName $op $cmd
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧# Once the traces are removed, this can become a no-op
⟦TAB⟧proc RemoveAutoConfigureTraces {} {}
    }

    proc Configure args {
⟦TAB⟧variable Option
⟦TAB⟧variable Verify
⟦TAB⟧set n [llength $args]
⟦TAB⟧if {$n == 0} {
⟦TAB⟧    return [lsort [array names Option]]
⟦TAB⟧}
⟦TAB⟧if {$n == 1} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    return $Option($option)
⟦TAB⟧}
⟦TAB⟧while {[llength $args] > 1} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    if {[catch {$Verify($option) [lindex $args 1]} value]} {
⟦TAB⟧⟦TAB⟧return -code error "invalid $option\
⟦TAB⟧⟦TAB⟧⟦TAB⟧value \"[lindex $args 1]\": $value"
⟦TAB⟧    }
⟦TAB⟧    set Option($option) $value
⟦TAB⟧    set args [lrange $args 2 end]
⟦TAB⟧}
⟦TAB⟧if {[llength $args]} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    return -code error "missing value for option $option"
⟦TAB⟧}
    }
    proc configure args {
⟦TAB⟧RemoveAutoConfigureTraces
⟦TAB⟧set code [catch {eval Configure $args} msg]
⟦TAB⟧return -code $code $msg
    }
⟦SPACE⟧⟦SPACE⟧⟦SPACE⟧⟦SPACE⟧
    proc AcceptVerbose { level } {
⟦TAB⟧set level [AcceptList $level]
⟦TAB⟧if {[llength $level] == 1} {
⟦TAB⟧    if {![regexp {^(pass|body|skip|start|error)$} $level]} {
⟦TAB⟧⟦TAB⟧# translate single characters abbreviations to expanded list
⟦TAB⟧⟦TAB⟧set level [string map {p pass b body s skip t start e error} \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[split $level {}]]
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set valid [list]
⟦TAB⟧foreach v $level {
⟦TAB⟧    if {[regexp {^(pass|body|skip|start|error)$} $v]} {
⟦TAB⟧⟦TAB⟧lappend valid $v
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $valid
    }

    proc IsVerbose {level} {
⟦TAB⟧variable Option
⟦TAB⟧return [expr {[lsearch -exact $Option(-verbose) $level] != -1}]
    }

    # Default verbosity is to show bodies of failed tests
    Option -verbose {body error} {
⟦TAB⟧Takes any combination of the values 'p', 's', 'b', 't' and 'e'.
⟦TAB⟧Test suite will display all passed tests if 'p' is specified, all
⟦TAB⟧skipped tests if 's' is specified, the bodies of failed tests if
⟦TAB⟧'b' is specified, and when tests start if 't' is specified.
⟦TAB⟧ErrorInfo is displayed if 'e' is specified.
    } AcceptVerbose verbose

    # Match and skip patterns default to the empty list, except for
    # matchFiles, which defaults to all .test files in the
    # testsDirectory and matchDirectories, which defaults to all
    # directories.
    Option -match * {
⟦TAB⟧Run all tests within the specified files that match one of the
⟦TAB⟧list of glob patterns given.
    } AcceptList match

    Option -skip {} {
⟦TAB⟧Skip all tests within the specified tests (via -match) and files
⟦TAB⟧that match one of the list of glob patterns given.
    } AcceptList skip

    Option -file *.test {
⟦TAB⟧Run tests in all test files that match the glob pattern given.
    } AcceptPattern matchFiles

    # By default, skip files that appear to be SCCS lock files.
    Option -notfile l.*.test {
⟦TAB⟧Skip all test files that match the glob pattern given.
    } AcceptPattern skipFiles

    Option -relateddir * {
⟦TAB⟧Run tests in directories that match the glob pattern given.
    } AcceptPattern matchDirectories

    Option -asidefromdir {} {
⟦TAB⟧Skip tests in directories that match the glob pattern given.
    } AcceptPattern skipDirectories

    # By default, don't save core files
    Option -preservecore 0 {
⟦TAB⟧If 2, save any core files produced during testing in the directory
⟦TAB⟧specified by -tmpdir. If 1, notify the user if core files are
⟦TAB⟧created.
    } AcceptInteger preserveCore

    # debug output doesn't get printed by default; debug level 1 spits
    # up only the tests that were skipped because they didn't match or
    # were specifically skipped.  A debug level of 2 would spit up the
    # tcltest variables and flags provided; a debug level of 3 causes
    # some additional output regarding operations of the test harness.
    # The tcltest package currently implements only up to debug level 3.
    Option -debug 0 {
⟦TAB⟧Internal debug level⟦SPACE⟧
    } AcceptInteger debug

    proc SetSelectedConstraints args {
⟦TAB⟧variable Option
⟦TAB⟧foreach c $Option(-constraints) {
⟦TAB⟧    testConstraint $c 1
⟦TAB⟧}
    }
    Option -constraints {} {
⟦TAB⟧Do not skip the listed constraints listed in -constraints.
    } AcceptList
    trace variable Option(-constraints) w \
⟦TAB⟧    [namespace code {SetSelectedConstraints ;#}]

    # Don't run only the "-constraint" specified tests by default
    proc ClearUnselectedConstraints args {
⟦TAB⟧variable Option
⟦TAB⟧variable testConstraints
⟦TAB⟧if {!$Option(-limitconstraints)} {return}
⟦TAB⟧foreach c [array names testConstraints] {
⟦TAB⟧    if {[lsearch -exact $Option(-constraints) $c] == -1} {
⟦TAB⟧⟦TAB⟧testConstraint $c 0
⟦TAB⟧    }
⟦TAB⟧}
    }
    Option -limitconstraints false {
⟦TAB⟧whether to run only tests with the constraints
    } AcceptBoolean limitConstraints⟦SPACE⟧
    trace variable Option(-limitconstraints) w \
⟦TAB⟧    [namespace code {ClearUnselectedConstraints ;#}]

    # A test application has to know how to load the tested commands
    # into the interpreter.
    Option -load {} {
⟦TAB⟧Specifies the script to load the tested commands.
    } AcceptScript loadScript

    # Default is to run each test file in a separate process
    Option -singleproc 0 {
⟦TAB⟧whether to run all tests in one process
    } AcceptBoolean singleProcess⟦SPACE⟧

    proc AcceptTemporaryDirectory { directory } {
⟦TAB⟧set directory [AcceptAbsolutePath $directory]
⟦TAB⟧if {![file exists $directory]} {
⟦TAB⟧    file mkdir $directory
⟦TAB⟧}
⟦TAB⟧set directory [AcceptDirectory $directory]
⟦TAB⟧if {![file writable $directory]} {
⟦TAB⟧    if {[string equal [workingDirectory] $directory]} {
⟦TAB⟧⟦TAB⟧# Special exception: accept the default value
⟦TAB⟧⟦TAB⟧# even if the directory is not writable
⟦TAB⟧⟦TAB⟧return $directory
⟦TAB⟧    }
⟦TAB⟧    return -code error "\"$directory\" is not writeable"
⟦TAB⟧}
⟦TAB⟧return $directory
    }

    # Directory where files should be created
    Option -tmpdir [workingDirectory] {
⟦TAB⟧Save temporary files in the specified directory.
    } AcceptTemporaryDirectory temporaryDirectory
    trace variable Option(-tmpdir) w \
⟦TAB⟧    [namespace code {normalizePath Option(-tmpdir) ;#}]

    # Tests should not rely on the current working directory.
    # Files that are part of the test suite should be accessed relative
    # to [testsDirectory]
    Option -testdir [workingDirectory] {
⟦TAB⟧Search tests in the specified directory.
    } AcceptDirectory testsDirectory
    trace variable Option(-testdir) w \
⟦TAB⟧    [namespace code {normalizePath Option(-testdir) ;#}]

    proc AcceptLoadFile { file } {
⟦TAB⟧if {[string equal "" $file]} {return $file}
⟦TAB⟧set file [file join [temporaryDirectory] $file]
⟦TAB⟧return [AcceptReadable $file]
    }
    proc ReadLoadScript {args} {
⟦TAB⟧variable Option
⟦TAB⟧if {[string equal "" $Option(-loadfile)]} {return}
⟦TAB⟧set tmp [open $Option(-loadfile) r]
⟦TAB⟧loadScript [read $tmp]
⟦TAB⟧close $tmp
    }
    Option -loadfile {} {
⟦TAB⟧Read the script to load the tested commands from the specified file.
    } AcceptLoadFile loadFile
    trace variable Option(-loadfile) w [namespace code ReadLoadScript]

    proc AcceptOutFile { file } {
⟦TAB⟧if {[string equal stderr $file]} {return $file}
⟦TAB⟧if {[string equal stdout $file]} {return $file}
⟦TAB⟧return [file join [temporaryDirectory] $file]
    }

    # output goes to stdout by default
    Option -outfile stdout {
⟦TAB⟧Send output from test runs to the specified file.
    } AcceptOutFile outputFile
    trace variable Option(-outfile) w \
⟦TAB⟧    [namespace code {outputChannel $Option(-outfile) ;#}]

    # errors go to stderr by default
    Option -errfile stderr {
⟦TAB⟧Send errors from test runs to the specified file.
    } AcceptOutFile errorFile
    trace variable Option(-errfile) w \
⟦TAB⟧    [namespace code {errorChannel $Option(-errfile) ;#}]

}

#####################################################################

# tcltest::Debug* --
#
#     Internal helper procedures to write out debug information
#     dependent on the chosen level. A test shell may overide
#     them, f.e. to redirect the output into a different
#     channel, or even into a GUI.

# tcltest::DebugPuts --
#
#     Prints the specified string if the current debug level is
#     higher than the provided level argument.
#
# Arguments:
#     level   The lowest debug level triggering the output
#     string  The string to print out.
#
# Results:
#     Prints the string. Nothing else is allowed.
#
# Side Effects:
#     None.
#

proc tcltest::DebugPuts {level string} {
    variable debug
    if {$debug >= $level} {
⟦TAB⟧puts $string
    }
    return
}

# tcltest::DebugPArray --
#
#     Prints the contents of the specified array if the current
#       debug level is higher than the provided level argument
#
# Arguments:
#     level           The lowest debug level triggering the output
#     arrayvar        The name of the array to print out.
#
# Results:
#     Prints the contents of the array. Nothing else is allowed.
#
# Side Effects:
#     None.
#

proc tcltest::DebugPArray {level arrayvar} {
    variable debug

    if {$debug >= $level} {
⟦TAB⟧catch {upvar  $arrayvar $arrayvar}
⟦TAB⟧parray $arrayvar
    }
    return
}

# Define our own [parray] in ::tcltest that will inherit use of the [puts]
# defined in ::tcltest.  NOTE: Ought to construct with [info args] and
# [info default], but can't be bothered now.  If [parray] changes, then
# this will need changing too.
auto_load ::parray
proc tcltest::parray {a {pattern *}} [info body ::parray]

# tcltest::DebugDo --
#
#     Executes the script if the current debug level is greater than
#       the provided level argument
#
# Arguments:
#     level   The lowest debug level triggering the execution.
#     script  The tcl script executed upon a debug level high enough.
#
# Results:
#     Arbitrary side effects, dependent on the executed script.
#
# Side Effects:
#     None.
#

proc tcltest::DebugDo {level script} {
    variable debug

    if {$debug >= $level} {
⟦TAB⟧uplevel 1 $script
    }
    return
}

#####################################################################

proc tcltest::Warn {msg} {
    puts [outputChannel] "WARNING: $msg"
}

# tcltest::mainThread
#
#     Accessor command for tcltest variable mainThread.
#
proc tcltest::mainThread { {new ""} } {
    variable mainThread
    if {[llength [info level 0]] == 1} {
⟦TAB⟧return $mainThread
    }
    set mainThread $new
}

# tcltest::testConstraint --
#
#⟦TAB⟧sets a test constraint to a value; to do multiple constraints,
#       call this proc multiple times.  also returns the value of the
#       named constraint if no value was supplied.
#
# Arguments:
#⟦TAB⟧constraint - name of the constraint
#       value - new value for constraint (should be boolean) - if not
#               supplied, this is a query
#
# Results:
#⟦TAB⟧content of tcltest::testConstraints($constraint)
#
# Side effects:
#⟦TAB⟧none

proc tcltest::testConstraint {constraint {value ""}} {
    variable testConstraints
    variable Option
    DebugPuts 3 "entering testConstraint $constraint $value"
    if {[llength [info level 0]] == 2} {
⟦TAB⟧return $testConstraints($constraint)
    }
    # Check for boolean values
    if {[catch {expr {$value && $value}} msg]} {
⟦TAB⟧return -code error $msg
    }
    if {[limitConstraints]⟦SPACE⟧
⟦TAB⟧    && [lsearch -exact $Option(-constraints) $constraint] == -1} {
⟦TAB⟧set value 0
    }
    set testConstraints($constraint) $value
}

# tcltest::interpreter --
#
#⟦TAB⟧the interpreter name stored in tcltest::tcltest
#
# Arguments:
#⟦TAB⟧executable name
#
# Results:
#⟦TAB⟧content of tcltest::tcltest
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::interpreter { {interp ""} } {
    variable tcltest
    if {[llength [info level 0]] == 1} {
⟦TAB⟧return $tcltest
    }
    if {[stringveFile $file
        }
⟦TAB⟧set code
    }

    ConstraintInitializer stdio {
⟦TAB⟧set code 0
⟦TAB⟧if {![catch {set f [open "|[list [interpreter]]" w]}]} {
⟦TAB⟧    if {![catch {puts $f exit}]} {
⟦TAB⟧⟦TAB⟧if {![catch {close $f}]} {
⟦TAB⟧⟦TAB⟧    set code 1
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set code
    }

    # Deliberately call socket with the wrong number of arguments.  The
    # error message you get will indicate whether sockets are available
    # on this system.

    ConstraintInitializer socket {
⟦TAB⟧catch {socket} msg
⟦TAB⟧string compare $msg "sockets are not available on this system"
    }

    # Check for internationalization
    ConstraintInitializer hasIsoLocale {
⟦TAB⟧if {[llength [info commands testlocale]] == 0} {
⟦TAB⟧    set code 0
⟦TAB⟧} else {
⟦TAB⟧    set code [string length [SetIso8859_1_Locale]]
⟦TAB⟧    RestoreLocale
⟦TAB⟧}
⟦TAB⟧set code
    }

}
#####################################################################

# Usage and command line arguments processing.

# tcltest::PrintUsageInfo
#
#⟦TAB⟧Prints out the usage information for package tcltest.  This can
#⟦TAB⟧be customized with the redefinition of [PrintUsageInfoHook].
#
# Arguments:
#⟦TAB⟧none
#
# Results:
#       none
#
# Side Effects:
#       none
proc tcltest::PrintUsageInfo {} {
    puts [Usage]
    PrintUsageInfoHook
}

proc tcltest::Usage { {option ""} } {
    variable Usage
    variable Verify
    if {[llength [info level 0]] == 1} {
⟦TAB⟧set msg "Usage: [file tail [info nameofexecutable]] script "
⟦TAB⟧append msg "?-help? ?flag value? ... \n"
⟦TAB⟧append msg "Available flags (and valid input values) are:"

⟦TAB⟧set max 0
⟦TAB⟧set allOpts [concat -help [Configure]]
⟦TAB⟧foreach opt $allOpts {
⟦TAB⟧    set foo [Usage $opt]
⟦TAB⟧    foreach [list x type($opt) usage($opt)] $foo break
⟦TAB⟧    set line($opt) "  $opt $type($opt)  "
⟦TAB⟧    set length($opt) [string length $line($opt)]
⟦TAB⟧    if {$length($opt) > $max} {set max $length($opt)}
⟦TAB⟧}
⟦TAB⟧set rest [expr {72 - $max}]
⟦TAB⟧foreach opt $allOpts {
⟦TAB⟧    append msg \n$line($opt)
⟦TAB⟧    append msg [string repeat " " [expr {$max - $length($opt)}]]
⟦TAB⟧    set u [string trim $usage(iable, if it matches an element in skip, or if one of the
# elements of "constraints" turns out not to be true.
#
# If testLevel is 1, then this is a top level test, and we record
# pass/fail information; otherwise, this information is not logged and
# is not added to running totals.
#
# Attributes:
#   Only description is a required attribute.  All others are optional.
#   Default values are indiibute is optional;
#                       default is {}
#   result -⟦TAB⟧        Expected result from script.  This attribute is
#                       optional; default is {}.
#   output -            Expected output sent to stdout.  This attribute
#                       is optional; default is {}.
#   errorOutput -       Expected output sent to stderr.  This attribute
#                       is optional; default is {}.
#   returnCodes -       Expected return codes.  This attribute is
#                       optional; default is {0 2}.
#   setup -             Code to run before $script (above).  This
#                       attribute is optional; default is {}.
#   cleanup -           Code to run after $script (above).  This
#                       attribute is optional; default is {}.
#   match -             specifies type of matching to do on result,
#                       output, errorOutput; this must be a string
#⟦TAB⟧⟦TAB⟧⟦TAB⟧previously registered by a call to [customMatch].
#⟦TAB⟧⟦TAB⟧⟦TAB⟧The strings exact, glob, and regexp are pre-registered
#⟦TAB⟧⟦TAB⟧⟦TAB⟧by the tcltest package.  Default value is exact.
#
# Arguments:
#   name -⟦TAB⟧⟦TAB⟧Name of test, in the form foo-1.2.
#   description -⟦TAB⟧Short textual description of the test, to
#  ⟦TAB⟧⟦TAB⟧  ⟦TAB⟧help humans understand what it does.
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#       Just about anything is possible depending on the test.
#

proc tcltest::test {name description args} {
    global tcl_platform
    variable testLevel
    variable coreModTime
    DebugPuts 3 "test $name $args"
    DebugDo 1 {
⟦TAB⟧variable TestNames
⟦TAB⟧catch {
⟦TAB⟧    puts "test name '$name' re-use Effects:
#      None
#

proc tcltest::cleanupTests {{calledFromAllFile 0}} {
    variable filesMade
    variable filesExisted
    variable createdNewFiles
    variable testSingleFile
    variable numTests
    variable numTestFiles
    variable failFiles
    variable skippedBecause
    variable currentFailure
    variable originalEnv
    variable originalTclPlatform
    variable coreModTime

    FillFilesExisted
    set testFileName [file tail [info script]]

    # Call the cleanup hook
    cleanupTestsHook

    # Remove files and directories created by the makeFile and
    # makeDirectory procedures.  Record the names of files in
    # workingDirectory that were not pre-existing, and associate them
    # with the test file that created them.

    if {!$calledFromAllFile} {
⟦TAB⟧foreach file $filesMade {
⟦TAB⟧    if {[file exists $file]} {
⟦TAB⟧⟦TAB⟧DebugDo 1 {Warn "cleanupTests deleting $file..."}
⟦TAB⟧⟦TAB⟧catch {file delete -force $file}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set currentFiles {}
⟦TAB⟧foreach file [glob -nocomplain \
⟦TAB⟧⟦TAB⟧-les $file
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set filesExisted $currentFiles
⟦TAB⟧if {[llength $newFiles] > 0} {
⟦TAB⟧    set createdNewFiles($testFileName) $newFiles
⟦TAB⟧}
    }

    if {$calledFromAllFile || $testSingleFile} {

⟦TAB⟧# print stats

⟦TAB⟧puts -nonewline [outputChannel] "$testFileName:"
⟦TAB⟧foreach index [list "Total" "Passed" "Skipped" "Failed"] {
⟦TAB⟧    puts -nonewline [outputChannel] \
⟦TAB⟧⟦TAB⟧    "\t$index\t$numTests($index)"
⟦TAB⟧}
⟦TAB⟧puts [outputChannel] ""

⟦TAB⟧# print number test files sourced
⟦TAB⟧# print names of files that ran tests which failed

⟦TAB⟧if {$calledFromAllFile} {
⟦TAB⟧    puts [outputChannel] \
⟦TAB⟧⟦TAB⟧    "Sourced $numTestFiles Test Files."
⟦TAB⟧    set numTestFiles 0
⟦TAB⟧    if {[llength $failFiles] > 0} {
⟦TAB⟧⟦TAB⟧puts [outputChannel] \
⟦TAB⟧⟦TAB⟧⟦TAB⟧"Files with failing tests: $failFiles"
⟦TAB⟧⟦TAB⟧set failFiles {}
⟦TAB⟧    }
⟦TAB⟧}

⟦TAB⟧# if any tests were skipped, print the constraints that kept
⟦TAB⟧# them from running.

⟦TAB⟧set constraintList [array names skippedBecause]
⟦TAB⟧if {[llength $constraintList] > 0} {
⟦TAB⟧    puts [outputChannel] \
⟦TAB⟧⟦TAB⟧    "Number of tests skipped fo if {[catch {
⟦TAB⟧⟦TAB⟧incr numTestFiles
⟦TAB⟧⟦TAB⟧set pipeFd [open $cmd "r"]
⟦TAB⟧⟦TAB⟧while {[gets $pipeFd line] >= 0} {
⟦TAB⟧⟦TAB⟧    if {[regexp [join {
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {^([^:]+):\t}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {Total\t([0-9]+)\t}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {Passed\t([0-9]+)\t($index) [set $index]
⟦TAB⟧⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧⟦TAB⟧if {$Failed > 0} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧    lappend failFiles $testFile
⟦TAB⟧⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧    } elseif {[regexp [join {
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {^Number of tests skipped }
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {for each constraint:}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {|^\t(\
    # cleanup
    puts [outputChannel] "\nTests ended at [eval $timeCmd]"
    cleanupTests 1
    if {[info exists testFileFailures]} {
⟦TAB⟧puts [outputChannel] "\nTest files exiting with errors:  \n"
⟦TAB⟧foreach file $testFileFailures {
⟦TAB⟧    puts [outputChannel] "  [file tail $file]\n"
⟦TAB⟧}
    }

    # Checking for subdirectories in which to run tests
    foreach directory [GetMatchingDirectories [testsDirectory]] {
⟦TAB⟧set dir [file tail $directory]
⟦TAB⟧puts [outputChannel] [string repeat ~ 44]
⟦TAB⟧puts [outputChannel] "$dir test began at [eval $timeCmd]\n"
⟦TAB⟧
⟦TAB⟧uplevel 1 [list ::source [file join $directory all.tcl]]
⟦TAB⟧
⟦TAB⟧set endTime [eval $timeCmd]
⟦TAB⟧puts [outputChannel] "\n$dir test ended at $endTime"
⟦TAB⟧puts [outputChannel] ""
⟦TAB⟧puts [outputChannel] [string repeat ~ 44]
    }
    return
}

#####################################################################

# Test utility procs - not used in tcltest, but may be useful for
# testing.

# tcltest::loadTestedCommands --
#
#     Uses the specified script to load the commands to test. Allowed to
#     be empty, as the tested commands could have been compiled into the
#     interpreter.
#
# Arguments
#     none
#
# Results
#     none
#
# Side Effects:
#     none.

proc tcltest::loadTestedCommands {} {
    variable l
    if {[string equal {} [loadScript]]} {
⟦TAB⟧return
    }

    return [uplevel 1 [loadScript]]
}

# tcltest::saveState --
#
#⟦TAB⟧Save information regarding what procs and variables exist.
#
# Arguments:
#⟦TAB⟧none
#
# Results:
#⟦TAB⟧Modifies the variable saveState
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::saveState {} {
    variable saveState
    u⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 0, result:

```text-visible
child-provider {8.4.6 8.4 ::} module-code 1 module-result {invalid command name "pwd"} require-code 1 require-result {can't find package tcltest}⟦EOF⟧
```

- real-harness-explicit-source-child, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- real-harness-explicit-source-child: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 3

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
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

    # tests that use threads need to know which is the main thread
    Default mainThread 1
    variable mainThread
    if {[info commands thread::id] != {}} {
⟦TAB⟧set mainThread [thread::id]
    } elseif {[info commands testthread] != {}} {
⟦TAB⟧set mainThread [testthread id]
    }

    # Set workingDirectory to [pwd]. The default output directory for
    # Tcl tests is the working directory.  Whenever this value changes
    # change to that directory.
    variable workingDirectory
    trace variable workingDirectory w \
⟦TAB⟧    [namespace code {cd $workingDirectory ;#}]

    Default workingDirectory [pwd] AcceptAbsolutePath
    proc workingDirectory { {dir ""} } {
⟦TAB⟧variable workingDirectory
⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $workingDirectory
⟦TAB⟧}
⟦TAB⟧set workingDirectory [AcceptAbsolutePath $dir]
    }

    # Set the location of the execuatble
    Default tcltest [info nameofexecutable]
    trace variable tcltest w [namespace code {testConstraint stdio \
⟦TAB⟧    [eval [ConstraintInitializer stdio]] ;#}]

    # save the platform information so it can be restored later
    Default originalTclPlatform [array get ::tcl_platform]

    # If a core file exists, save its modification time.
    if {[file exists [file join [workingDirectory] core]]} {
⟦TAB⟧Default coreModTime \
⟦TAB⟧⟦TAB⟧[file mtime [file join [workingDirectory] core]]
    }

    # stdout and stderr buffers for use when we want to store them
    Default outData {}
    Default errData {}

    # keep track of test level for nested test commands
    variable testLevel 0

    # the variables and procs that existed when saveState was called are
    # stored in a variable of the same name
    Default saveState {}

    # Internationalization support -- used in [SetIso8859_1_Locale] and
    # [RestoreLocale]. Those commands are used in cmdIL.test.

    if {![info exists [namespace current]::isoLocale]} {
⟦TAB⟧variable isoLocale fr
⟦TAB⟧switch -- $::tcl_platform(platform) {
⟦TAB⟧    "unix" {

⟦TAB⟧⟦TAB⟧# Try some 'known' values for some platforms:

⟦TAB⟧⟦TAB⟧switch -exact -- $::tcl_platform(os) {
⟦TAB⟧⟦TAB⟧    "FreeBSD" {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr_FR.ISO_8859-1
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    HP-UX {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr_FR.iso88591
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    Linux -
⟦TAB⟧⟦TAB⟧    IRIX {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    default {

⟦TAB⟧⟦TAB⟧⟦TAB⟧# Works on SunOS 4 and Solaris, and maybe
⟦TAB⟧⟦TAB⟧⟦TAB⟧# others...  Define it to something else on your
⟦TAB⟧⟦TAB⟧⟦TAB⟧# system if you want to test those.

⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale iso_8859_1
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧    "windows" {
⟦TAB⟧⟦TAB⟧set isoLocale French
⟦TAB⟧    }
⟦TAB⟧}
    }

    variable ChannelsWeOpened; array set ChannelsWeOpened {}
    # output goes to stdout by default
    Default outputChannel stdout
    proc outputChannel { {filename ""} } {
⟦TAB⟧variable outputChannel
⟦TAB⟧variable ChannelsWeOpened

⟦TAB⟧# This is very subtle and tricky, so let me try to explain.
⟦TAB⟧# (Hopefully this longer comment will be clear when I come
⟦TAB⟧# back in a few months, unlike its predecessor :) )
⟦TAB⟧#⟦SPACE⟧
⟦TAB⟧# The [outputChannel] command (and underlying variable) have to
⟦TAB⟧# be kept in sync with the [configure -outfile] configuration
⟦TAB⟧# option ( and underlying variable Option(-outfile) ).  This is
⟦TAB⟧# accomplished with a write trace on Option(-outfile) that will
⟦TAB⟧# update [outputChannel] whenver a new value is written.  That
⟦TAB⟧# much is easy.
⟦TAB⟧#
⟦TAB⟧# The trick is that in order to maintain compatibility with
⟦TAB⟧# version 1 of tcltest, we must allow every configuration option
⟦TAB⟧# to get its inital value from command line arguments.  This is
⟦TAB⟧# accomplished by setting initial read traces on all the
⟦TAB⟧# configuration options to parse the command line option the first
⟦TAB⟧# time they are read.  These traces are cancelled whenever the
⟦TAB⟧# program itself calls [configure].
⟦TAB⟧#⟦SPACE⟧
⟦TAB⟧# OK, then so to support tcltest 1 compatibility, it seems we want
⟦TAB⟧# to get the return from [outputFile] to trigger the read traces,
⟦TAB⟧# just in case.
⟦TAB⟧#
⟦TAB⟧# BUT!  A little known feature of Tcl variable traces is that⟦SPACE⟧
⟦TAB⟧# traces are disabled during the handling of other traces.  So,
⟦TAB⟧# if we trigger read traces on Option(-outfile) and that triggers
⟦TAB⟧# command line parsing which turns around and sets an initial
⟦TAB⟧# value for Option(-outfile) -- <whew!> -- the write trace that
⟦TAB⟧# would keep [outputChannel] in sync with that new initial value
⟦TAB⟧# would not fire!
⟦TAB⟧#
⟦TAB⟧# SO, finally, as a workaround, instead of triggering read traces
⟦TAB⟧# by invoking [outputFile], we instead trigger the same set of
⟦TAB⟧# read traces by invoking [debug].  Any command that reads a
⟦TAB⟧# configuration option would do.  [debug] is just a handy one.
⟦TAB⟧# The end result is that we support tcltest 1 compatibility and
⟦TAB⟧# keep outputChannel and -outfile in sync in all cases.
⟦TAB⟧debug

⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $outputChannel
⟦TAB⟧}
⟦TAB⟧if {[info exists ChannelsWeOpened($outputChannel)]} {
⟦TAB⟧    close $outputChannel
⟦TAB⟧    unset ChannelsWeOpened($outputChannel)
⟦TAB⟧}
⟦TAB⟧switch -exact -- $filename {
⟦TAB⟧    stderr -
⟦TAB⟧    stdout {
⟦TAB⟧⟦TAB⟧set outputChannel $filename
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧set outputChannel [open $filename a]
⟦TAB⟧⟦TAB⟧set ChannelsWeOpened($outputChannel) 1

⟦TAB⟧⟦TAB⟧# If we created the file in [temporaryDirectory], then
⟦TAB⟧⟦TAB⟧# [cleanupTests] will delete it, unless we claim it was
⟦TAB⟧⟦TAB⟧# already there.
⟦TAB⟧⟦TAB⟧set outdir [normalizePath [file dirname \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [pwd] $filename]]]
⟦TAB⟧⟦TAB⟧if {[string equal $outdir [temporaryDirectory]]} {
⟦TAB⟧⟦TAB⟧    variable filesExisted
⟦TAB⟧⟦TAB⟧    FillFilesExisted
⟦TAB⟧⟦TAB⟧    set filename [file tail $filename]
⟦TAB⟧⟦TAB⟧    if {[lsearch -exact $filesExisted $filename] == -1} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧lappend filesExisted $filename
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $outputChannel
    }

    # errors go to stderr by default
    Default errorChannel stderr
    proc errorChannel { {filename ""} } {
⟦TAB⟧variable errorChannel
⟦TAB⟧variable ChannelsWeOpened

⟦TAB⟧# This is subtle and tricky.  See the comment above in
⟦TAB⟧# [outputChannel] for a detailed explanation.
⟦TAB⟧debug

⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $errorChannel
⟦TAB⟧}
⟦TAB⟧if {[info exists ChannelsWeOpened($errorChannel)]} {
⟦TAB⟧    close $errorChannel
⟦TAB⟧    unset ChannelsWeOpened($errorChannel)
⟦TAB⟧}
⟦TAB⟧switch -exact -- $filename {
⟦TAB⟧    stderr -
⟦TAB⟧    stdout {
⟦TAB⟧⟦TAB⟧set errorChannel $filename
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧set errorChannel [open $filename a]
⟦TAB⟧⟦TAB⟧set ChannelsWeOpened($errorChannel) 1

⟦TAB⟧⟦TAB⟧# If we created the file in [temporaryDirectory], then
⟦TAB⟧⟦TAB⟧# [cleanupTests] will delete it, unless we claim it was
⟦TAB⟧⟦TAB⟧# already there.
⟦TAB⟧⟦TAB⟧set outdir [normalizePath [file dirname \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [pwd] $filename]]]
⟦TAB⟧⟦TAB⟧if {[string equal $outdir [temporaryDirectory]]} {
⟦TAB⟧⟦TAB⟧    variable filesExisted
⟦TAB⟧⟦TAB⟧    FillFilesExisted
⟦TAB⟧⟦TAB⟧    set filename [file tail $filename]
⟦TAB⟧⟦TAB⟧    if {[lsearch -exact $filesExisted $filename] == -1} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧lappend filesExisted $filename
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $errorChannel
    }

##### Set up the configurable options
    #
    # The configurable options of the package
    variable Option; array set Option {}

    # Usage strings for those options
    variable Usage; array set Usage {}

    # Verification commands for those options
    variable Verify; array set Verify {}

    # Initialize the default values of the configurable options that are
    # historically associated with an exported variable.  If that variable
    # is already set, support compatibility by accepting its pre-set value.
    # Use [trace] to establish ongoing connection between the deprecated
    # exported variable and the modern option kept as a true internal var.
    # Also set up usage string and value testing for the option.
    proc Option {option value usage {verify AcceptAll} {varName {}}} {
⟦TAB⟧variable Option
⟦TAB⟧variable Verify
⟦TAB⟧variable Usage
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧set Usage($option) $usage
⟦TAB⟧set Verify($option) $verify
⟦TAB⟧if {[catch {$verify $value} msg]} {
⟦TAB⟧    return -code error $msg
⟦TAB⟧} else {
⟦TAB⟧    set Option($option) $msg
⟦TAB⟧}
⟦TAB⟧if {[string length $varName]} {
⟦TAB⟧    variable $varName
⟦TAB⟧    if {[info exists $varName]} {
⟦TAB⟧⟦TAB⟧if {[catch {$verify [set $varName]} msg]} {
⟦TAB⟧⟦TAB⟧    return -code error $msg
⟦TAB⟧⟦TAB⟧} else {
⟦TAB⟧⟦TAB⟧    set Option($option) $msg
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧unset $varName
⟦TAB⟧    }
⟦TAB⟧    namespace eval [namespace current] \
⟦TAB⟧    ⟦TAB⟧    [list upvar 0 Option($option) $varName]
⟦TAB⟧    # Workaround for Bug (now Feature Request) 572889.  Grrrr....
⟦TAB⟧    # Track all the variables tied to options
⟦TAB⟧    lappend OptionControlledVariables $varName
⟦TAB⟧    # Later, set auto-configure read traces on all
⟦TAB⟧    # of them, since a single trace on Option does not work.
⟦TAB⟧    proc $varName {{value {}}} [subst -nocommands {
⟦TAB⟧⟦TAB⟧if {[llength [info level 0]] == 2} {
⟦TAB⟧⟦TAB⟧    Configure $option [set value]
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧return [Configure $option]
⟦TAB⟧    }]
⟦TAB⟧}
    }

    proc MatchingOption {option} {
⟦TAB⟧variable Option
⟦TAB⟧set match [array names Option $option*]
⟦TAB⟧switch -- [llength $match] {
⟦TAB⟧    0 {
⟦TAB⟧⟦TAB⟧set sorted [lsort [array names Option]]
⟦TAB⟧⟦TAB⟧set values [join [lrange $sorted 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append values ", or [lindex $sorted end]"
⟦TAB⟧⟦TAB⟧return -code error "unknown option $option: should be\
⟦TAB⟧⟦TAB⟧⟦TAB⟧one of $values"
⟦TAB⟧    }
⟦TAB⟧    1 {
⟦TAB⟧⟦TAB⟧return [lindex $match 0]
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧# Exact match trumps ambiguity
⟦TAB⟧⟦TAB⟧if {[lsearch -exact $match $option] >= 0} {
⟦TAB⟧⟦TAB⟧    return $option
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧set values [join [lrange $match 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append values ", or [lindex $match end]"
⟦TAB⟧⟦TAB⟧return -code error "ambiguous option $option:\
⟦TAB⟧⟦TAB⟧⟦TAB⟧could match $values"
⟦TAB⟧    }
⟦TAB⟧}
    }

    proc EstablishAutoConfigureTraces {} {
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧foreach varName [concat $OptionControlledVariables Option] {
⟦TAB⟧    variable $varName
⟦TAB⟧    trace variable $varName r [namespace code {ProcessCmdLineArgs ;#}]
⟦TAB⟧}
    }

    proc RemoveAutoConfigureTraces {} {
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧foreach varName [concat $OptionControlledVariables Option] {
⟦TAB⟧    variable $varName
⟦TAB⟧    foreach pair [trace vinfo $varName] {
⟦TAB⟧⟦TAB⟧foreach {op cmd} $pair break
⟦TAB⟧⟦TAB⟧if {[string equal r $op]
⟦TAB⟧⟦TAB⟧⟦TAB⟧&& [string match *ProcessCmdLineArgs* $cmd]} {
⟦TAB⟧⟦TAB⟧    trace vdelete $varName $op $cmd
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧# Once the traces are removed, this can become a no-op
⟦TAB⟧proc RemoveAutoConfigureTraces {} {}
    }

    proc Configure args {
⟦TAB⟧variable Option
⟦TAB⟧variable Verify
⟦TAB⟧set n [llength $args]
⟦TAB⟧if {$n == 0} {
⟦TAB⟧    return [lsort [array names Option]]
⟦TAB⟧}
⟦TAB⟧if {$n == 1} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    return $Option($option)
⟦TAB⟧}
⟦TAB⟧while {[llength $args] > 1} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    if {[catch {$Verify($option) [lindex $args 1]} value]} {
⟦TAB⟧⟦TAB⟧return -code error "invalid $option\
⟦TAB⟧⟦TAB⟧⟦TAB⟧value \"[lindex $args 1]\": $value"
⟦TAB⟧    }
⟦TAB⟧    set Option($option) $value
⟦TAB⟧    set args [lrange $args 2 end]
⟦TAB⟧}
⟦TAB⟧if {[llength $args]} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    return -code error "missing value for option $option"
⟦TAB⟧}
    }
    proc configure args {
⟦TAB⟧RemoveAutoConfigureTraces
⟦TAB⟧set code [catch {eval Configure $args} msg]
⟦TAB⟧return -code $code $msg
    }
⟦SPACE⟧⟦SPACE⟧⟦SPACE⟧⟦SPACE⟧
    proc AcceptVerbose { level } {
⟦TAB⟧set level [AcceptList $level]
⟦TAB⟧if {[llength $level] == 1} {
⟦TAB⟧    if {![regexp {^(pass|body|skip|start|error)$} $level]} {
⟦TAB⟧⟦TAB⟧# translate single characters abbreviations to expanded list
⟦TAB⟧⟦TAB⟧set level [string map {p pass b body s skip t start e error} \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[split $level {}]]
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set valid [list]
⟦TAB⟧foreach v $level {
⟦TAB⟧    if {[regexp {^(pass|body|skip|start|error)$} $v]} {
⟦TAB⟧⟦TAB⟧lappend valid $v
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $valid
    }

    proc IsVerbose {level} {
⟦TAB⟧variable Option
⟦TAB⟧return [expr {[lsearch -exact $Option(-verbose) $level] != -1}]
    }

    # Default verbosity is to show bodies of failed tests
    Option -verbose {body error} {
⟦TAB⟧Takes any combination of the values 'p', 's', 'b', 't' and 'e'.
⟦TAB⟧Test suite will display all passed tests if 'p' is specified, all
⟦TAB⟧skipped tests if 's' is specified, the bodies of failed tests if
⟦TAB⟧'b' is specified, and when tests start if 't' is specified.
⟦TAB⟧ErrorInfo is displayed if 'e' is specified.
    } AcceptVerbose verbose

    # Match and skip patterns default to the empty list, except for
    # matchFiles, which defaults to all .test files in the
    # testsDirectory and matchDirectories, which defaults to all
    # directories.
    Option -match * {
⟦TAB⟧Run all tests within the specified files that match one of the
⟦TAB⟧list of glob patterns given.
    } AcceptList match

    Option -skip {} {
⟦TAB⟧Skip all tests within the specified tests (via -match) and files
⟦TAB⟧that match one of the list of glob patterns given.
    } AcceptList skip

    Option -file *.test {
⟦TAB⟧Run tests in all test files that match the glob pattern given.
    } AcceptPattern matchFiles

    # By default, skip files that appear to be SCCS lock files.
    Option -notfile l.*.test {
⟦TAB⟧Skip all test files that match the glob pattern given.
    } AcceptPattern skipFiles

    Option -relateddir * {
⟦TAB⟧Run tests in directories that match the glob pattern given.
    } AcceptPattern matchDirectories

    Option -asidefromdir {} {
⟦TAB⟧Skip tests in directories that match the glob pattern given.
    } AcceptPattern skipDirectories

    # By default, don't save core files
    Option -preservecore 0 {
⟦TAB⟧If 2, save any core files produced during testing in the directory
⟦TAB⟧specified by -tmpdir. If 1, notify the user if core files are
⟦TAB⟧created.
    } AcceptInteger preserveCore

    # debug output doesn't get printed by default; debug level 1 spits
    # up only the tests that were skipped because they didn't match or
    # were specifically skipped.  A debug level of 2 would spit up the
    # tcltest variables and flags provided; a debug level of 3 causes
    # some additional output regarding operations of the test harness.
    # The tcltest package currently implements only up to debug level 3.
    Option -debug 0 {
⟦TAB⟧Internal debug level⟦SPACE⟧
    } AcceptInteger debug

    proc SetSelectedConstraints args {
⟦TAB⟧variable Option
⟦TAB⟧foreach c $Option(-constraints) {
⟦TAB⟧    testConstraint $c 1
⟦TAB⟧}
    }
    Option -constraints {} {
⟦TAB⟧Do not skip the listed constraints listed in -constraints.
    } AcceptList
    trace variable Option(-constraints) w \
⟦TAB⟧    [namespace code {SetSelectedConstraints ;#}]

    # Don't run only the "-constraint" specified tests by default
    proc ClearUnselectedConstraints args {
⟦TAB⟧variable Option
⟦TAB⟧variable testConstraints
⟦TAB⟧if {!$Option(-limitconstraints)} {return}
⟦TAB⟧foreach c [array names testConstraints] {
⟦TAB⟧    if {[lsearch -exact $Option(-constraints) $c] == -1} {
⟦TAB⟧⟦TAB⟧testConstraint $c 0
⟦TAB⟧    }
⟦TAB⟧}
    }
    Option -limitconstraints false {
⟦TAB⟧whether to run only tests with the constraints
    } AcceptBoolean limitConstraints⟦SPACE⟧
    trace variable Option(-limitconstraints) w \
⟦TAB⟧    [namespace code {ClearUnselectedConstraints ;#}]

    # A test application has to know how to load the tested commands
    # into the interpreter.
    Option -load {} {
⟦TAB⟧Specifies the script to load the tested commands.
    } AcceptScript loadScript

    # Default is to run each test file in a separate process
    Option -singleproc 0 {
⟦TAB⟧whether to run all tests in one process
    } AcceptBoolean singleProcess⟦SPACE⟧

    proc AcceptTemporaryDirectory { directory } {
⟦TAB⟧set directory [AcceptAbsolutePath $directory]
⟦TAB⟧if {![file exists $directory]} {
⟦TAB⟧    file mkdir $directory
⟦TAB⟧}
⟦TAB⟧set directory [AcceptDirectory $directory]
⟦TAB⟧if {![file writable $directory]} {
⟦TAB⟧    if {[string equal [workingDirectory] $directory]} {
⟦TAB⟧⟦TAB⟧# Special exception: accept the default value
⟦TAB⟧⟦TAB⟧# even if the directory is not writable
⟦TAB⟧⟦TAB⟧return $directory
⟦TAB⟧    }
⟦TAB⟧    return -code error "\"$directory\" is not writeable"
⟦TAB⟧}
⟦TAB⟧return $directory
    }

    # Directory where files should be created
    Option -tmpdir [workingDirectory] {
⟦TAB⟧Save temporary files in the specified directory.
    } AcceptTemporaryDirectory temporaryDirectory
    trace variable Option(-tmpdir) w \
⟦TAB⟧    [namespace code {normalizePath Option(-tmpdir) ;#}]

    # Tests should not rely on the current working directory.
    # Files that are part of the test suite should be accessed relative
    # to [testsDirectory]
    Option -testdir [workingDirectory] {
⟦TAB⟧Search tests in the specified directory.
    } AcceptDirectory testsDirectory
    trace variable Option(-testdir) w \
⟦TAB⟧    [namespace code {normalizePath Option(-testdir) ;#}]

    proc AcceptLoadFile { file } {
⟦TAB⟧if {[string equal "" $file]} {return $file}
⟦TAB⟧set file [file join [temporaryDirectory] $file]
⟦TAB⟧return [AcceptReadable $file]
    }
    proc ReadLoadScript {args} {
⟦TAB⟧variable Option
⟦TAB⟧if {[string equal "" $Option(-loadfile)]} {return}
⟦TAB⟧set tmp [open $Option(-loadfile) r]
⟦TAB⟧loadScript [read $tmp]
⟦TAB⟧close $tmp
    }
    Option -loadfile {} {
⟦TAB⟧Read the script to load the tested commands from the specified file.
    } AcceptLoadFile loadFile
    trace variable Option(-loadfile) w [namespace code ReadLoadScript]

    proc AcceptOutFile { file } {
⟦TAB⟧if {[string equal stderr $file]} {return $file}
⟦TAB⟧if {[string equal stdout $file]} {return $file}
⟦TAB⟧return [file join [temporaryDirectory] $file]
    }

    # output goes to stdout by default
    Option -outfile stdout {
⟦TAB⟧Send output from test runs to the specified file.
    } AcceptOutFile outputFile
    trace variable Option(-outfile) w \
⟦TAB⟧    [namespace code {outputChannel $Option(-outfile) ;#}]

    # errors go to stderr by default
    Option -errfile stderr {
⟦TAB⟧Send errors from test runs to the specified file.
    } AcceptOutFile errorFile
    trace variable Option(-errfile) w \
⟦TAB⟧    [namespace code {errorChannel $Option(-errfile) ;#}]

}

#####################################################################

# tcltest::Debug* --
#
#     Internal helper procedures to write out debug information
#     dependent on the chosen level. A test shell may overide
#     them, f.e. to redirect the output into a different
#     channel, or even into a GUI.

# tcltest::DebugPuts --
#
#     Prints the specified string if the current debug level is
#     higher than the provided level argument.
#
# Arguments:
#     level   The lowest debug level triggering the output
#     string  The string to print out.
#
# Results:
#     Prints the string. Nothing else is allowed.
#
# Side Effects:
#     None.
#

proc tcltest::DebugPuts {level string} {
    variable debug
    if {$debug >= $level} {
⟦TAB⟧puts $string
    }
    return
}

# tcltest::DebugPArray --
#
#     Prints the contents of the specified array if the current
#       debug level is higher than the provided level argument
#
# Arguments:
#     level           The lowest debug level triggering the output
#     arrayvar        The name of the array to print out.
#
# Results:
#     Prints the contents of the array. Nothing else is allowed.
#
# Side Effects:
#     None.
#

proc tcltest::DebugPArray {level arrayvar} {
    variable debug

    if {$debug >= $level} {
⟦TAB⟧catch {upvar  $arrayvar $arrayvar}
⟦TAB⟧parray $arrayvar
    }
    return
}

# Define our own [parray] in ::tcltest that will inherit use of the [puts]
# defined in ::tcltest.  NOTE: Ought to construct with [info args] and
# [info default], but can't be bothered now.  If [parray] changes, then
# this will need changing too.
auto_load ::parray
proc tcltest::parray {a {pattern *}} [info body ::parray]

# tcltest::DebugDo --
#
#     Executes the script if the current debug level is greater than
#       the provided level argument
#
# Arguments:
#     level   The lowest debug level triggering the execution.
#     script  The tcl script executed upon a debug level high enough.
#
# Results:
#     Arbitrary side effects, dependent on the executed script.
#
# Side Effects:
#     None.
#

proc tcltest::DebugDo {level script} {
    variable debug

    if {$debug >= $level} {
⟦TAB⟧uplevel 1 $script
    }
    return
}

#####################################################################

proc tcltest::Warn {msg} {
    puts [outputChannel] "WARNING: $msg"
}

# tcltest::mainThread
#
#     Accessor command for tcltest variable mainThread.
#
proc tcltest::mainThread { {new ""} } {
    variable mainThread
    if {[llength [info level 0]] == 1} {
⟦TAB⟧return $mainThread
    }
    set mainThread $new
}

# tcltest::testConstraint --
#
#⟦TAB⟧sets a test constraint to a value; to do multiple constraints,
#       call this proc multiple times.  also returns the value of the
#       named constraint if no value was supplied.
#
# Arguments:
#⟦TAB⟧constraint - name of the constraint
#       value - new value for constraint (should be boolean) - if not
#               supplied, this is a query
#
# Results:
#⟦TAB⟧content of tcltest::testConstraints($constraint)
#
# Side effects:
#⟦TAB⟧none

proc tcltest::testConstraint {constraint {value ""}} {
    variable testConstraints
    variable Option
    DebugPuts 3 "entering testConstraint $constraint $value"
    if {[llength [info level 0]] == 2} {
⟦TAB⟧return $testConstraints($constraint)
    }
    # Check for boolean values
    if {[catch {expr {$value && $value}} msg]} {
⟦TAB⟧return -code error $msg
    }
    if {[limitConstraints]⟦SPACE⟧
⟦TAB⟧    && [lsearch -exact $Option(-constraints) $constraint] == -1} {
⟦TAB⟧set value 0
    }
    set testConstraints($constraint) $value
}

# tcltest::interpreter --
#
#⟦TAB⟧the interpreter name stored in tcltest::tcltest
#
# Arguments:
#⟦TAB⟧executable name
#
# Results:
#⟦TAB⟧content of tcltest::tcltest
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::interpreter { {interp ""} } {
    variable tcltest
    if {[llength [info level 0]] == 1} {
⟦TAB⟧return $tcltest
    }
    if {[stringle {![string equal end $beginningIndex]} {
⟦TAB⟧    puts -nonewline [errorChannel] \
⟦TAB⟧⟦TAB⟧    [string repeat " " $InitialMsgLen]
⟦TAB⟧    if {($endingIndex - $beginningIndex)
⟦TAB⟧⟦TAB⟧    < (80 - $InitialMsgLen)} {
⟦TAB⟧⟦TAB⟧p⟦TAB⟧[expr {$beginningIndex
⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧+ (80 - $InitialMsgLen)}]
⟦TAB⟧⟦TAB⟧]] + $beginningIndex}]
⟦TAB⟧⟦TAB⟧if {($newEndingIndex <= 0)
⟦TAB⟧⟦TAB⟧⟦TAB⟧|| ($newEndingIndex <= $beginningIndex)} {
⟦TAB⟧⟦TAB⟧    set newEndingIndex end
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧puts [errofeFetch --
#
#⟦TAB⟧ The following trace procedure makes it so that we can safely
#        refer to non-existent members of the testConstraints array
#        without causing an error.  Instead, reading a non-existent
#        member will return 0. This is necessary because tests are
#        allowed to use constraint "X" without ensuring that
#        testConstraints("X") is defined.
#
# Arguments:
#⟦TAB⟧n1 - name of the array (testConstraints)
#       n2 - array key value (constraint name)
#       op - operation performed on testConstraints (generally r)
#
# Results:
#⟦TAB⟧none
#
# Side effects:
#⟦TAB⟧sets testConstraints($n2) to 0 if it's referenced but never
#       before used

proc tcltest::SafeFetch {n1 n2 op} {
    variable testConstraints
    DebugPuts 3 "entering SafeFetch $n1 $n2 $op"
    if {[string equal {} $n2]} {return}
    if {![info exists testConstraints($n2)]} {
⟦TAB⟧if {[catch {testConstraint $n2 [eval [ConstraintInitializer $n2]]}]} {
⟦TAB⟧    testConstraint $n2 0
⟦TAB⟧}
    }
}

# tcltest::Coint - name of the constraint initialized by the script
#⟦TAB⟧script - the initializer script
#
# Results
#⟦TAB⟧boolean value of the constraint - enabled or disabled
#
# Side effects:
#⟦TAB⟧Constraint is initialized for future reference by [test]
proc tcltest::ConstraintInitializer {constraint {script ""}} {
    variable ConstraintInitializer
    DebugPuts 3 "entering ConstraintInitializer $constraint $script"
    if {[llength [info level 0]] == 2} {
⟦TAB⟧return $ConstraintInitializer($constraint)
    }
    # Check for boolean values
    if {![info complete $script]} {
⟦TAB⟧return -code error "ConstraintInitialize{hello} $file
⟦TAB⟧    }]} {
⟦TAB⟧        set code 0
⟦TAB⟧    } elseif {
⟦TAB⟧        [catch {exec cat $file}] ||
⟦TAB⟧        [catch {exec echo hello}] ||
⟦TAB⟧        [catch {exec sh -c echo hello}] ||
⟦TAB⟧        [catch {exep the $rootdir if it
    # comes up to avoid infinite loops.
    set skipDirs [list $rootdir]
    foreach pattern [skipDirectories] {
⟦TAB⟧set skipDirs [concat $skipDirs [glob -directory $rootdir -types d \
⟦TAB⟧⟦TAB⟧-nocomplain -- $pattern]]
    }

    # Now step through the matching directories, prune out the skipped ones
    # as you go.
    set matchDirs [list]
    foreach pattern [matchDirectories] {
⟦TAB⟧foreach path [glob -directory $rootdir -types d -nocomplain -- \
⟦TAB⟧⟦TAB⟧$pattern] {
⟦TAB⟧    if {[lsearch -exact $skipDirs $path] == -1} {
⟦TAB⟧⟦TAB⟧set matchDirs [concat $matchDirs [GetMatchingDirectories $path]]
⟦TAB⟧⟦TAB⟧if  skip patterns!"
    }
    return $matchDirs
}

# tcltest::runAllTests --
#
#⟦TAB⟧prints output and sources test files according to the match and
#⟦TAB⟧skip patterns provided.  after sourcing test files, it goes on
#⟦TAB⟧to source all.tcl files in matching test subdirectories.
#
# Arguments:
#⟦TAB⟧shell being tested
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::runAllTests { {shell ""} } {
    variable testSingleFile
    variable numTestFiles
    variable numTests
    variable failFiles

    FillFilesExisted
    if {[llength [info level 0]] == 1} {
⟦TAB⟧set shell [interpreter]
    }

    set testSingleFile false

    puts [outputChannel] "Tests running in interp:  $shell"
    puts [outputChannel] "Tests located in:  [testsDirectory]"
    puts [outputChannel] "Tests running in:  [workingDirectory]"
    puts [outputChannel] "Temporary files stored in\
⟦TAB⟧    [temporaryDirectory]"

    # [file system] first available in Tcl 8.4
    if {![catch {file system [testsDirectory]} result]
⟦TAB⟧    && ![string equal native [lindex $result 0]]} {
⟦TAB⟧# If we aren't running in the native filesystem, then we must
⟦TAB⟧# run the tests in a single process (via 'source'), because
⟦TAB⟧# trying to run then via a pipe wi⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 0, result:

```text-visible
child-provider {8.4.6 8.4 ::} module-code 1 module-result {invalid command name "pwd"} require-code 1 require-result {can't find package tcltest}⟦EOF⟧
```

- real-harness-explicit-source-child, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- real-harness-explicit-source-child: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 4

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
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

    # tests that use threads need to know which is the main thread
    Default mainThread 1
    variable mainThread
    if {[info commands thread::id] != {}} {
⟦TAB⟧set mainThread [thread::id]
    } elseif {[info commands testthread] != {}} {
⟦TAB⟧set mainThread [testthread id]
    }

    # Set workingDirectory to [pwd]. The default output directory for
    # Tcl tests is the working directory.  Whenever this value changes
    # change to that directory.
    variable workingDirectory
    trace variable workingDirectory w \
⟦TAB⟧    [namespace code {cd $workingDirectory ;#}]

    Default workingDirectory [pwd] AcceptAbsolutePath
    proc workingDirectory { {dir ""} } {
⟦TAB⟧variable workingDirectory
⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $workingDirectory
⟦TAB⟧}
⟦TAB⟧set workingDirectory [AcceptAbsolutePath $dir]
    }

    # Set the location of the execuatble
    Default tcltest [info nameofexecutable]
    trace variable tcltest w [namespace code {testConstraint stdio \
⟦TAB⟧    [eval [ConstraintInitializer stdio]] ;#}]

    # save the platform information so it can be restored later
    Default originalTclPlatform [array get ::tcl_platform]

    # If a core file exists, save its modification time.
    if {[file exists [file join [workingDirectory] core]]} {
⟦TAB⟧Default coreModTime \
⟦TAB⟧⟦TAB⟧[file mtime [file join [workingDirectory] core]]
    }

    # stdout and stderr buffers for use when we want to store them
    Default outData {}
    Default errData {}

    # keep track of test level for nested test commands
    variable testLevel 0

    # the variables and procs that existed when saveState was called are
    # stored in a variable of the same name
    Default saveState {}

    # Internationalization support -- used in [SetIso8859_1_Locale] and
    # [RestoreLocale]. Those commands are used in cmdIL.test.

    if {![info exists [namespace current]::isoLocale]} {
⟦TAB⟧variable isoLocale fr
⟦TAB⟧switch -- $::tcl_platform(platform) {
⟦TAB⟧    "unix" {

⟦TAB⟧⟦TAB⟧# Try some 'known' values for some platforms:

⟦TAB⟧⟦TAB⟧switch -exact -- $::tcl_platform(os) {
⟦TAB⟧⟦TAB⟧    "FreeBSD" {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr_FR.ISO_8859-1
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    HP-UX {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr_FR.iso88591
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    Linux -
⟦TAB⟧⟦TAB⟧    IRIX {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    default {

⟦TAB⟧⟦TAB⟧⟦TAB⟧# Works on SunOS 4 and Solaris, and maybe
⟦TAB⟧⟦TAB⟧⟦TAB⟧# others...  Define it to something else on your
⟦TAB⟧⟦TAB⟧⟦TAB⟧# system if you want to test those.

⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale iso_8859_1
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧    "windows" {
⟦TAB⟧⟦TAB⟧set isoLocale French
⟦TAB⟧    }
⟦TAB⟧}
    }

    variable ChannelsWeOpened; array set ChannelsWeOpened {}
    # output goes to stdout by default
    Default outputChannel stdout
    proc outputChannel { {filename ""} } {
⟦TAB⟧variable outputChannel
⟦TAB⟧variable ChannelsWeOpened

⟦TAB⟧# This is very subtle and tricky, so let me try to explain.
⟦TAB⟧# (Hopefully this longer comment will be clear when I come
⟦TAB⟧# back in a few months, unlike its predecessor :) )
⟦TAB⟧#⟦SPACE⟧
⟦TAB⟧# The [outputChannel] command (and underlying variable) have to
⟦TAB⟧# be kept in sync with the [configure -outfile] configuration
⟦TAB⟧# option ( and underlying variable Option(-outfile) ).  This is
⟦TAB⟧# accomplished with a write trace on Option(-outfile) that will
⟦TAB⟧# update [outputChannel] whenver a new value is written.  That
⟦TAB⟧# much is easy.
⟦TAB⟧#
⟦TAB⟧# The trick is that in order to maintain compatibility with
⟦TAB⟧# version 1 of tcltest, we must allow every configuration option
⟦TAB⟧# to get its inital value from command line arguments.  This is
⟦TAB⟧# accomplished by setting initial read traces on all the
⟦TAB⟧# configuration options to parse the command line option the first
⟦TAB⟧# time they are read.  These traces are cancelled whenever the
⟦TAB⟧# program itself calls [configure].
⟦TAB⟧#⟦SPACE⟧
⟦TAB⟧# OK, then so to support tcltest 1 compatibility, it seems we want
⟦TAB⟧# to get the return from [outputFile] to trigger the read traces,
⟦TAB⟧# just in case.
⟦TAB⟧#
⟦TAB⟧# BUT!  A little known feature of Tcl variable traces is that⟦SPACE⟧
⟦TAB⟧# traces are disabled during the handling of other traces.  So,
⟦TAB⟧# if we trigger read traces on Option(-outfile) and that triggers
⟦TAB⟧# command line parsing which turns around and sets an initial
⟦TAB⟧# value for Option(-outfile) -- <whew!> -- the write trace that
⟦TAB⟧# would keep [outputChannel] in sync with that new initial value
⟦TAB⟧# would not fire!
⟦TAB⟧#
⟦TAB⟧# SO, finally, as a workaround, instead of triggering read traces
⟦TAB⟧# by invoking [outputFile], we instead trigger the same set of
⟦TAB⟧# read traces by invoking [debug].  Any command that reads a
⟦TAB⟧# configuration option would do.  [debug] is just a handy one.
⟦TAB⟧# The end result is that we support tcltest 1 compatibility and
⟦TAB⟧# keep outputChannel and -outfile in sync in all cases.
⟦TAB⟧debug

⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $outputChannel
⟦TAB⟧}
⟦TAB⟧if {[info exists ChannelsWeOpened($outputChannel)]} {
⟦TAB⟧    close $outputChannel
⟦TAB⟧    unset ChannelsWeOpened($outputChannel)
⟦TAB⟧}
⟦TAB⟧switch -exact -- $filename {
⟦TAB⟧    stderr -
⟦TAB⟧    stdout {
⟦TAB⟧⟦TAB⟧set outputChannel $filename
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧set outputChannel [open $filename a]
⟦TAB⟧⟦TAB⟧set ChannelsWeOpened($outputChannel) 1

⟦TAB⟧⟦TAB⟧# If we created the file in [temporaryDirectory], then
⟦TAB⟧⟦TAB⟧# [cleanupTests] will delete it, unless we claim it was
⟦TAB⟧⟦TAB⟧# already there.
⟦TAB⟧⟦TAB⟧set outdir [normalizePath [file dirname \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [pwd] $filename]]]
⟦TAB⟧⟦TAB⟧if {[string equal $outdir [temporaryDirectory]]} {
⟦TAB⟧⟦TAB⟧    variable filesExisted
⟦TAB⟧⟦TAB⟧    FillFilesExisted
⟦TAB⟧⟦TAB⟧    set filename [file tail $filename]
⟦TAB⟧⟦TAB⟧    if {[lsearch -exact $filesExisted $filename] == -1} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧lappend filesExisted $filename
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $outputChannel
    }

    # errors go to stderr by default
    Default errorChannel stderr
    proc errorChannel { {filename ""} } {
⟦TAB⟧variable errorChannel
⟦TAB⟧variable ChannelsWeOpened

⟦TAB⟧# This is subtle and tricky.  See the comment above in
⟦TAB⟧# [outputChannel] for a detailed explanation.
⟦TAB⟧debug

⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $errorChannel
⟦TAB⟧}
⟦TAB⟧if {[info exists ChannelsWeOpened($errorChannel)]} {
⟦TAB⟧    close $errorChannel
⟦TAB⟧    unset ChannelsWeOpened($errorChannel)
⟦TAB⟧}
⟦TAB⟧switch -exact -- $filename {
⟦TAB⟧    stderr -
⟦TAB⟧    stdout {
⟦TAB⟧⟦TAB⟧set errorChannel $filename
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧set errorChannel [open $filename a]
⟦TAB⟧⟦TAB⟧set ChannelsWeOpened($errorChannel) 1

⟦TAB⟧⟦TAB⟧# If we created the file in [temporaryDirectory], then
⟦TAB⟧⟦TAB⟧# [cleanupTests] will delete it, unless we claim it was
⟦TAB⟧⟦TAB⟧# already there.
⟦TAB⟧⟦TAB⟧set outdir [normalizePath [file dirname \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [pwd] $filename]]]
⟦TAB⟧⟦TAB⟧if {[string equal $outdir [temporaryDirectory]]} {
⟦TAB⟧⟦TAB⟧    variable filesExisted
⟦TAB⟧⟦TAB⟧    FillFilesExisted
⟦TAB⟧⟦TAB⟧    set filename [file tail $filename]
⟦TAB⟧⟦TAB⟧    if {[lsearch -exact $filesExisted $filename] == -1} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧lappend filesExisted $filename
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $errorChannel
    }

##### Set up the configurable options
    #
    # The configurable options of the package
    variable Option; array set Option {}

    # Usage strings for those options
    variable Usage; array set Usage {}

    # Verification commands for those options
    variable Verify; array set Verify {}

    # Initialize the default values of the configurable options that are
    # historically associated with an exported variable.  If that variable
    # is already set, support compatibility by accepting its pre-set value.
    # Use [trace] to establish ongoing connection between the deprecated
    # exported variable and the modern option kept as a true internal var.
    # Also set up usage string and value testing for the option.
    proc Option {option value usage {verify AcceptAll} {varName {}}} {
⟦TAB⟧variable Option
⟦TAB⟧variable Verify
⟦TAB⟧variable Usage
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧set Usage($option) $usage
⟦TAB⟧set Verify($option) $verify
⟦TAB⟧if {[catch {$verify $value} msg]} {
⟦TAB⟧    return -code error $msg
⟦TAB⟧} else {
⟦TAB⟧    set Option($option) $msg
⟦TAB⟧}
⟦TAB⟧if {[string length $varName]} {
⟦TAB⟧    variable $varName
⟦TAB⟧    if {[info exists $varName]} {
⟦TAB⟧⟦TAB⟧if {[catch {$verify [set $varName]} msg]} {
⟦TAB⟧⟦TAB⟧    return -code error $msg
⟦TAB⟧⟦TAB⟧} else {
⟦TAB⟧⟦TAB⟧    set Option($option) $msg
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧unset $varName
⟦TAB⟧    }
⟦TAB⟧    namespace eval [namespace current] \
⟦TAB⟧    ⟦TAB⟧    [list upvar 0 Option($option) $varName]
⟦TAB⟧    # Workaround for Bug (now Feature Request) 572889.  Grrrr....
⟦TAB⟧    # Track all the variables tied to options
⟦TAB⟧    lappend OptionControlledVariables $varName
⟦TAB⟧    # Later, set auto-configure read traces on all
⟦TAB⟧    # of them, since a single trace on Option does not work.
⟦TAB⟧    proc $varName {{value {}}} [subst -nocommands {
⟦TAB⟧⟦TAB⟧if {[llength [info level 0]] == 2} {
⟦TAB⟧⟦TAB⟧    Configure $option [set value]
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧return [Configure $option]
⟦TAB⟧    }]
⟦TAB⟧}
    }

    proc MatchingOption {option} {
⟦TAB⟧variable Option
⟦TAB⟧set match [array names Option $option*]
⟦TAB⟧switch -- [llength $match] {
⟦TAB⟧    0 {
⟦TAB⟧⟦TAB⟧set sorted [lsort [array names Option]]
⟦TAB⟧⟦TAB⟧set values [join [lrange $sorted 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append values ", or [lindex $sorted end]"
⟦TAB⟧⟦TAB⟧return -code error "unknown option $option: should be\
⟦TAB⟧⟦TAB⟧⟦TAB⟧one of $values"
⟦TAB⟧    }
⟦TAB⟧    1 {
⟦TAB⟧⟦TAB⟧return [lindex $match 0]
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧# Exact match trumps ambiguity
⟦TAB⟧⟦TAB⟧if {[lsearch -exact $match $option] >= 0} {
⟦TAB⟧⟦TAB⟧    return $option
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧set values [join [lrange $match 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append values ", or [lindex $match end]"
⟦TAB⟧⟦TAB⟧return -code error "ambiguous option $option:\
⟦TAB⟧⟦TAB⟧⟦TAB⟧could match $values"
⟦TAB⟧    }
⟦TAB⟧}
    }

    proc EstablishAutoConfigureTraces {} {
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧foreach varName [concat $OptionControlledVariables Option] {
⟦TAB⟧    variable $varName
⟦TAB⟧    trace variable $varName r [namespace code {ProcessCmdLineArgs ;#}]
⟦TAB⟧}
    }

    proc RemoveAutoConfigureTraces {} {
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧foreach varName [concat $OptionControlledVariables Option] {
⟦TAB⟧    variable $varName
⟦TAB⟧    foreach pair [trace vinfo $varName] {
⟦TAB⟧⟦TAB⟧foreach {op cmd} $pair break
⟦TAB⟧⟦TAB⟧if {[string equal r $op]
⟦TAB⟧⟦TAB⟧⟦TAB⟧&& [string match *ProcessCmdLineArgs* $cmd]} {
⟦TAB⟧⟦TAB⟧    trace vdelete $varName $op $cmd
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧# Once the traces are removed, this can become a no-op
⟦TAB⟧proc RemoveAutoConfigureTraces {} {}
    }

    proc Configure args {
⟦TAB⟧variable Option
⟦TAB⟧variable Verify
⟦TAB⟧set n [llength $args]
⟦TAB⟧if {$n == 0} {
⟦TAB⟧    return [lsort [array names Option]]
⟦TAB⟧}
⟦TAB⟧if {$n == 1} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    return $Option($option)
⟦TAB⟧}
⟦TAB⟧while {[llength $args] > 1} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    if {[catch {$Verify($option) [lindex $args 1]} value]} {
⟦TAB⟧⟦TAB⟧return -code error "invalid $option\
⟦TAB⟧⟦TAB⟧⟦TAB⟧value \"[lindex $args 1]\": $value"
⟦TAB⟧    }
⟦TAB⟧    set Option($option) $value
⟦TAB⟧    set args [lrange $args 2 end]
⟦TAB⟧}
⟦TAB⟧if {[llength $args]} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    return -code error "missing value for option $option"
⟦TAB⟧}
    }
    proc configure args {
⟦TAB⟧RemoveAutoConfigureTraces
⟦TAB⟧set code [catch {eval Configure $args} msg]
⟦TAB⟧return -code $code $msg
    }
⟦SPACE⟧⟦SPACE⟧⟦SPACE⟧⟦SPACE⟧
    proc AcceptVerbose { level } {
⟦TAB⟧set level [AcceptList $level]
⟦TAB⟧if {[llength $level] == 1} {
⟦TAB⟧    if {![regexp {^(pass|body|skip|start|error)$} $level]} {
⟦TAB⟧⟦TAB⟧# translate single characters abbreviations to expanded list
⟦TAB⟧⟦TAB⟧set level [string map {p pass b body s skip t start e error} \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[split $level {}]]
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set valid [list]
⟦TAB⟧foreach v $level {
⟦TAB⟧    if {[regexp {^(pass|body|skip|start|error)$} $v]} {
⟦TAB⟧⟦TAB⟧lappend valid $v
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $valid
    }

    proc IsVerbose {level} {
⟦TAB⟧variable Option
⟦TAB⟧return [expr {[lsearch -exact $Option(-verbose) $level] != -1}]
    }

    # Default verbosity is to show bodies of failed tests
    Option -verbose {body error} {
⟦TAB⟧Takes any combination of the values 'p', 's', 'b', 't' and 'e'.
⟦TAB⟧Test suite will display all passed tests if 'p' is specified, all
⟦TAB⟧skipped tests if 's' is specified, the bodies of failed tests if
⟦TAB⟧'b' is specified, and when tests start if 't' is specified.
⟦TAB⟧ErrorInfo is displayed if 'e' is specified.
    } AcceptVerbose verbose

    # Match and skip patterns default to the empty list, except for
    # matchFiles, which defaults to all .test files in the
    # testsDirectory and matchDirectories, which defaults to all
    # directories.
    Option -match * {
⟦TAB⟧Run all tests within the specified files that match one of the
⟦TAB⟧list of glob patterns given.
    } AcceptList match

    Option -skip {} {
⟦TAB⟧Skip all tests within the specified tests (via -match) and files
⟦TAB⟧that match one of the list of glob patterns given.
    } AcceptList skip

    Option -file *.test {
⟦TAB⟧Run tests in all test files that match the glob pattern given.
    } AcceptPattern matchFiles

    # By default, skip files that appear to be SCCS lock files.
    Option -notfile l.*.test {
⟦TAB⟧Skip all test files that match the glob pattern given.
    } AcceptPattern skipFiles

    Option -relateddir * {
⟦TAB⟧Run tests in directories that match the glob pattern given.
    } AcceptPattern matchDirectories

    Option -asidefromdir {} {
⟦TAB⟧Skip tests in directories that match the glob pattern given.
    } AcceptPattern skipDirectories

    # By default, don't save core files
    Option -preservecore 0 {
⟦TAB⟧If 2, save any core files produced during testing in the directory
⟦TAB⟧specified by -tmpdir. If 1, notify the user if core files are
⟦TAB⟧created.
    } AcceptInteger preserveCore

    # debug output doesn't get printed by default; debug level 1 spits
    # up only the tests that were skipped because they didn't match or
    # were specifically skipped.  A debug level of 2 would spit up the
    # tcltest variables and flags provided; a debug level of 3 causes
    # some additional output regarding operations of the test harness.
    # The tcltest package currently implements only up to debug level 3.
    Option -debug 0 {
⟦TAB⟧Internal debug level⟦SPACE⟧
    } AcceptInteger debug

    proc SetSelectedConstraints args {
⟦TAB⟧variable Option
⟦TAB⟧foreach c $Option(-constraints) {
⟦TAB⟧    testConstraint $c 1
⟦TAB⟧}
    }
    Option -constraints {} {
⟦TAB⟧Do not skip the listed constraints listed in -constraints.
    } AcceptList
    trace variable Option(-constraints) w \
⟦TAB⟧    [namespace code {SetSelectedConstraints ;#}]

    # Don't run only the "-constraint" specified tests by default
    proc ClearUnselectedConstraints args {
⟦TAB⟧variable Option
⟦TAB⟧variable testConstraints
⟦TAB⟧if {!$Option(-limitconstraints)} {return}
⟦TAB⟧foreach c [array names testConstraints] {
⟦TAB⟧    if {[lsearch -exact $Option(-constraints) $c] == -1} {
⟦TAB⟧⟦TAB⟧testConstraint $c 0
⟦TAB⟧    }
⟦TAB⟧}
    }
    Option -limitconstraints false {
⟦TAB⟧whether to run only tests with the constraints
    } AcceptBoolean limitConstraints⟦SPACE⟧
    trace variable Option(-limitconstraints) w \
⟦TAB⟧    [namespace code {ClearUnselectedConstraints ;#}]

    # A test application has to know how to load the tested commands
    # into the interpreter.
    Option -load {} {
⟦TAB⟧Specifies the script to load the tested commands.
    } AcceptScript loadScript

    # Default is to run each test file in a separate process
    Option -singleproc 0 {
⟦TAB⟧whether to run all tests in one process
    } AcceptBoolean singleProcess⟦SPACE⟧

    proc AcceptTemporaryDirectory { directory } {
⟦TAB⟧set directory [AcceptAbsolutePath $directory]
⟦TAB⟧if {![file exists $directory]} {
⟦TAB⟧    file mkdir $directory
⟦TAB⟧}
⟦TAB⟧set directory [AcceptDirectory $directory]
⟦TAB⟧if {![file writable $directory]} {
⟦TAB⟧    if {[string equal [workingDirectory] $directory]} {
⟦TAB⟧⟦TAB⟧# Special exception: accept the default value
⟦TAB⟧⟦TAB⟧# even if the directory is not writable
⟦TAB⟧⟦TAB⟧return $directory
⟦TAB⟧    }
⟦TAB⟧    return -code error "\"$directory\" is not writeable"
⟦TAB⟧}
⟦TAB⟧return $directory
    }

    # Directory where files should be created
    Option -tmpdir [workingDirectory] {
⟦TAB⟧Save temporary files in the specified directory.
    } AcceptTemporaryDirectory temporaryDirectory
    trace variable Option(-tmpdir) w \
⟦TAB⟧    [namespace code {normalizePath Option(-tmpdir) ;#}]

    # Tests should not rely on the current working directory.
    # Files that are part of the test suite should be accessed relative
    # to [testsDirectory]
    Option -testdir [workingDirectory] {
⟦TAB⟧Search tests in the specified directory.
    } AcceptDirectory testsDirectory
    trace variable Option(-testdir) w \
⟦TAB⟧    [namespace code {normalizePath Option(-testdir) ;#}]

    proc AcceptLoadFile { file } {
⟦TAB⟧if {[string equal "" $file]} {return $file}
⟦TAB⟧set file [file join [temporaryDirectory] $file]
⟦TAB⟧return [AcceptReadable $file]
    }
    proc ReadLoadScript {args} {
⟦TAB⟧variable Option
⟦TAB⟧if {[string equal "" $Option(-loadfile)]} {return}
⟦TAB⟧set tmp [open $Option(-loadfile) r]
⟦TAB⟧loadScript [read $tmp]
⟦TAB⟧close $tmp
    }
    Option -loadfile {} {
⟦TAB⟧Read the script to load the tested commands from the specified file.
    } AcceptLoadFile loadFile
    trace variable Option(-loadfile) w [namespace code ReadLoadScript]

    proc AcceptOutFile { file } {
⟦TAB⟧if {[string equal stderr $file]} {return $file}
⟦TAB⟧if {[string equal stdout $file]} {return $file}
⟦TAB⟧return [file join [temporaryDirectory] $file]
    }

    # output goes to stdout by default
    Option -outfile stdout {
⟦TAB⟧Send output from test runs to the specified file.
    } AcceptOutFile outputFile
    trace variable Option(-outfile) w \
⟦TAB⟧    [namespace code {outputChannel $Option(-outfile) ;#}]

    # errors go to stderr by default
    Option -errfile stderr {
⟦TAB⟧Send errors from test runs to the specified file.
    } AcceptOutFile errorFile
    trace variable Option(-errfile) w \
⟦TAB⟧    [namespace code {errorChannel $Option(-errfile) ;#}]

}

#####################################################################

# tcltest::Debug* --
#
#     Internal helper procedures to write out debug information
#     dependent on the chosen level. A test shell may overide
#     them, f.e. to redirect the output into a different
#     channel, or even into a GUI.

# tcltest::DebugPuts --
#
#     Prints the specified string if the current debug level is
#     higher than the provided level argument.
#
# Arguments:
#     level   The lowest debug level triggering the output
#     string  The string to print out.
#
# Results:
#     Prints the string. Nothing else is allowed.
#
# Side Effects:
#     None.
#

proc tcltest::DebugPuts {level string} {
    variable debug
    if {$debug >= $level} {
⟦TAB⟧puts $string
    }
    return
}

# tcltest::DebugPArray --
#
#     Prints the contents of the specified array if the current
#       debug level is higher than the provided level argument
#
# Arguments:
#     level           The lowest debug level triggering the output
#     arrayvar        The name of the array to print out.
#
# Results:
#     Prints the contents of the array. Nothing else is allowed.
#
# Side Effects:
#     None.
#

proc tcltest::DebugPArray {level arrayvar} {
    variable debug

    if {$debug >= $level} {
⟦TAB⟧catch {upvar  $arrayvar $arrayvar}
⟦TAB⟧parray $arrayvar
    }
    return
}

# Define our own [parray] in ::tcltest that will inherit use of the [puts]
# defined in ::tcltest.  NOTE: Ought to construct with [info args] and
# [info default], but can't be bothered now.  If [parray] changes, then
# this will need changing too.
auto_load ::parray
proc tcltest::parray {a {pattern *}} [info body ::parray]

# tcltest::DebugDo --
#
#     Executes the script if the current debug level is greater than
#       the provided level argument
#
# Arguments:
#     level   The lowest debug level triggering the execution.
#     script  The tcl script executed upon a debug level high enough.
#
# Results:
#     Arbitrary side effects, dependent on the executed script.
#
# Side Effects:
#     None.
#

proc tcltest::DebugDo {level script} {
    variable debug

    if {$debug >= $level} {
⟦TAB⟧uplevel 1 $script
    }
    return
}

#####################################################################

proc tcltest::Warn {msg} {
    puts [outputChannel] "WARNING: $msg"
}

# tcltest::mainThread
#
#     Accessor command for tcltest variable mainThread.
#
proc tcltest::mainThread { {new ""} } {
    variable mainThread
    if {[llength [info level 0]] == 1} {
⟦TAB⟧return $mainThread
    }
    set mainThread $new
}

# tcltest::testConstraint --
#
#⟦TAB⟧sets a test constraint to a value; to do multiple constraints,
#       call this proc multiple times.  also returns the value of the
#       named constraint if no value was supplied.
#
# Arguments:
#⟦TAB⟧constraint - name of the constraint
#       value - new value for constraint (should be boolean) - if not
#               supplied, this is a query
#
# Results:
#⟦TAB⟧content of tcltest::testConstraints($constraint)
#
# Side effects:
#⟦TAB⟧none

proc tcltest::testConstraint {constraint {value ""}} {
    variable testConstraints
    variable Option
    DebugPuts 3 "entering testConstraint $constraint $value"
    if {[llength [info level 0]] == 2} {
⟦TAB⟧return $testConstraints($constraint)
    }
    # Check for boolean values
    if {[catch {expr {$value && $value}} msg]} {
⟦TAB⟧return -code error $msg
    }
    if {[limitConstraints]⟦SPACE⟧
⟦TAB⟧    && [lsearch -exact $Option(-constraints) $constraint] == -1} {
⟦TAB⟧set value 0
    }
    set testConstraints($constraint) $value
}

# tcltest::interpreter --
#
#⟦TAB⟧the interpreter name stored in tcltest::tcltest
#
# Arguments:
#⟦TAB⟧executable name
#
# Results:
#⟦TAB⟧content of tcltest::tcltest
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::interpreter { {interp ""} } {
    variable tcltest
    if {[llength [info level 0]] == 1} {
⟦TAB⟧return $tcltest
    }
    if {[string equal {} $interp]} {
⟦TAB⟧set tcltest {}
    } else {
⟦TAB⟧set tcltest $interp
    }
}

#####################################################################

# tcltest::AddToSkippedBecause --
#
#⟦TAB⟧Increments the variable used to track how many tests were
#       skipped because of a particular constraint.
#
# Arguments:
#⟦TAB⟧constraint     The name of the constraint to be modified
#
# Results:
#⟦TAB⟧Modifies tcltest::skippedBecause; sets the variable to 1 if
#       didn't previously exist - otherwise, it just increments it.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::AddToSkippedBecause { constraint {value 1}} {
    # add the constraint to the list of constraints that kept tests
    # from running
    variable skippedBecause

    if {[info exists skippedBecause($constraint)]} {
⟦TAB⟧incr skippedBecause($constraint) $value
    } else {
⟦TAB⟧set skippedBecause($constraint) $value
    }
    return
}

# tcltest::PrintError --
#
#⟦TAB⟧Prints errors to tcltest::errorChannel and then flushes that
#       channel, making sure that all messages are < 80 characters per
#       line.
#
# Arguments:
#⟦TAB⟧errorMsg     String containing the error to be printed
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::PrintError {errorMsg} {
    set InitialMessage "Error:  "
    set InitialMsgLen  [string length $InitialMessage]
    puts -nonewline [errorChannel] $InitialMessage

    # Keep track of where the end of the string is.
    set endingIndex [string length $errorMsg]

    if {$endingIndex < (80 - $InitialMsgLen)} {
⟦TAB⟧puts [errorChannel] $errorMsg
    } else {
⟦TAB⟧# Print up to 80 characters on the first line, including the
⟦TAB⟧# InitialMessage.
⟦TAB⟧set beginningIndex [string last " " [string range $errorMsg 0 \
⟦TAB⟧⟦TAB⟧[expr {80 - $InitialMsgLen}]]]
⟦TAB⟧puts [errorChannel] [string range $errorMsg 0 $beginningIndex]

⟦TAB⟧while {![string equal end $beginningIndex]} {
⟦TAB⟧    puts -nonewline [errorChannel] \
⟦TAB⟧⟦TAB⟧    [string repeat " " $InitialMsgLen]
⟦TAB⟧    if {($endingIndex - $beginningIndex)
⟦TAB⟧⟦TAB⟧    < (80 - $InitialMsgLen)} {
⟦TAB⟧⟦TAB⟧puts [errorChannel] [string trim \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[string range $errorMsg $beginningIndex end]]
⟦TAB⟧⟦TAB⟧break
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧set newEndingIndex [expr {[string last " " \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[string range $errorMsg $beginningIndex \
⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧[expr {$beginningIndex
⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧+ (80 - $InitialMsgLen)}]
⟦TAB⟧⟦TAB⟧]] + $beginningIndex}]
⟦TAB⟧⟦TAB⟧if {($newEndingIndex <= 0)
⟦TAB⟧⟦TAB⟧⟦TAB⟧|| ($newEndingIndex <= $beginningIndex)} {
⟦TAB⟧⟦TAB⟧    set newEndingIndex end
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧puts [errorChannel] [string trim \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[string range $errorMsg \
⟦TAB⟧⟦TAB⟧⟦TAB⟧    $beginningIndex $newEndingIndex]]
⟦TAB⟧⟦TAB⟧set beginningIndex $newEndingIndex
⟦TAB⟧    }
⟦TAB⟧}
    }
    flush [errorChannel]
    return
}

# tcltest::SafeFetch --
#
#⟦TAB⟧ The following trace procedure makes it so that we can safely
#        refer to non-existent members of the testConstraints array
#        without causing an error.  Instead, reading a non-existent
#        member will return 0. This is necessary because tests are
#        allowed to use constraint "X" without ensuring that
#        testConstraints("X") is defined.
#
# Arguments:
#⟦TAB⟧n1 - name of the array (testConstraints)
#       n2 - array key value (constraint name)
#       op - operation performed on testConstraints (generally r)
#
# Results:
#⟦TAB⟧none
#
# Side effects:
#⟦TAB⟧sets testConstraints($n2) to 0 if it's referenced but never
#       before used

proc tcltest::SafeFetch {n1 n2 op} {
    variable testConstraints
    DebugPuts 3 "entering SafeFetch $n1 $n2 $op"
    if {[string equal {} $n2]} {return}
    if {![info exists testConstraints($n2)]} {
⟦TAB⟧if {[catch {testConstraint $n2 [eval [ConstraintInitializer $n2]]}]} {
⟦TAB⟧    testConstraint $n2 0
⟦TAB⟧}
    }
}

# tcltest::ConstraintInitializer --
#
#⟦TAB⟧Get or set a script that when evaluated in the tcltest namespace
#⟦TAB⟧will return a boolean value with which to initialize the
#⟦TAB⟧associated constraint.
#
# Arguments:
#⟦TAB⟧constraint - name of the constraint initialized by the script
#⟦TAB⟧script - the initializer script
#
# Results
#⟦TAB⟧boolean value of the constraint - enabled or disabled
#
# Side effects:
#⟦TAB⟧Constraint is initialized for future reference by [test]
proc tcltest::ConstraintInitializer {constraint {script ""}} {
    variable ConstraintInitializer
    DebugPuts 3 "entering ConstraintInitializer $constraint $script"
    if {[llength [info level 0]] == 2} {
⟦TAB⟧return $ConstraintInitializer($constraint)
    }
    # Check for boolean values
    if {![info complete $script]} {
⟦TAB⟧return -code error "ConstraintInitializer must be complete script"
    }
    set ConstraintInitializer($constraint) $script
}

# tcltest::InitConstraints --
#
# Call all registered constraint initializers to force initialization
# of all known constraints.
# See the tcltest man page for the list of built-in constraints defined
# in this procedure.
#
# Arguments:
#⟦TAB⟧none
#
# Results:
#⟦TAB⟧The testConstraints array is reset to have an index for each
#⟦TAB⟧built-in test constraint.
#
# Side Effects:
#       None.
#

proc tcltest::InitConstraints {} {
    variable ConstraintInitializer
    initConstraintsHook
    foreach constraint [array names ConstraintInitializer] {
⟦TAB⟧testConstraint $constraint
    }
}

proc tcltest::DefineConstraintInitializers {} {
    ConstraintInitializer singleTestInterp {singleProcess}

    # All the 'pc' constraints are here for backward compatibility and
    # are not documented.  They have been replaced with equivalent 'win'
    # constraints.

    ConstraintInitializer unixOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) unix}
    ConstraintInitializer macOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) macintosh}
    ConstraintInitializer pcOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) windows}
    ConstraintInitializer winOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) windows}

    ConstraintInitializer unix {testConstraint unixOnly}
    ConstraintInitializer mac {testConstraint macOnly}
    ConstraintInitializer pc {testConstraint pcOnly}
    ConstraintInitializer win {testConstraint winOnly}

    ConstraintInitializer unixOrPc \
⟦TAB⟧    {expr {[testConstraint unix] || [testConstraint pc]}}
    ConstraintInitializer macOrPc \
⟦TAB⟧    {expr {[testConstraint mac] || [testConstraint pc]}}
    ConstraintInitializer unixOrWin \
⟦TAB⟧    {expr {[testConstraint unix] || [testConstraint win]}}
    ConstraintInitializer macOrWin \
⟦TAB⟧    {expr {[testConstraint mac] || [testConstraint win]}}
    ConstraintInitializer macOrUnix \
⟦TAB⟧    {expr {[testConstraint mac] || [testConstraint unix]}}

    ConstraintInitializer nt {string equal $::tcl_platform(os) "Windows NT"}
    ConstraintInitializer 95 {string equal $::tcl_platform(os) "Windows 95"}
    ConstraintInitializer 98 {string equal $::tcl_platform(os) "Windows 98"}

    # The following Constraints switches are used to mark tests that
    # should work, but have been temporarily disabled on certain
    # platforms because they don't and we haven't gotten around to
    # fixing the underlying problem.

    ConstraintInitializer tempNotPc {expr {![testConstraint pc]}}
    ConstraintInitializer tempNotWin {expr {![testConstraint win]}}
    ConstraintInitializer tempNotMac {expr {![testConstraint mac]}}
    ConstraintInitializer tempNotUnix {expr {![testConstraint unix]}}

    # The following Constraints switches are used to mark tests that
    # crash on certain platforms, so that they can be reactivated again
    # when the underlying problem is fixed.

    ConstraintInitializer pcCrash {expr {![testConstraint pc]}}
    ConstraintInitializer winCrash {expr {![testConstraint win]}}
    ConstraintInitializer macCrash {expr {![testConstraint mac]}}
    ConstraintInitializer unixCrash {expr {![testConstraint unix]}}

    # Skip empty tests

    ConstraintInitializer emptyTest {format 0}

    # By default, tests that expose known bugs are skipped.

    ConstraintInitializer knownBug {format 0}

    # By default, non-portable tests are skipped.

    ConstraintInitializer nonPortable {format 0}

    # Some tests require user interaction.

    ConstraintInitializer userInteraction {format 0}

    # Some tests must be skipped if the interpreter is not in
    # interactive mode

    ConstraintInitializer interactive \
⟦TAB⟧    {expr {[info exists ::tcl_interactive] && $::tcl_interactive}}

    # Some tests can only be run if the installation came from a CD
    # image instead of a web image.  Some tests must be skipped if you
    # are running as root on Unix.  Other tests can only be run if you
    # are running as root on Unix.

    ConstraintInitializer root {expr \
⟦TAB⟧    {[string equal unix $::tcl_platform(platform)]
⟦TAB⟧    && ([string equal root $::tcl_platform(user)]
⟦TAB⟧⟦TAB⟧|| [string equal "" $::tcl_platform(user)])}}
    ConstraintInitializer notRoot {expr {![testConstraint root]}}

    # Set nonBlockFiles constraint: 1 means this platform supports
    # setting files into nonblocking mode.

    ConstraintInitializer nonBlockFiles {
⟦TAB⟧    set code [expr {[catch {set f [open defs r]}]⟦SPACE⟧
⟦TAB⟧⟦TAB⟧    || [catch {fconfigure $f -blocking off}]}]
⟦TAB⟧    catch {close $f}
⟦TAB⟧    set code
    }

    # Set asyncPipeClose constraint: 1 means this platform supports
    # async flush and async close on a pipe.
    #
    # Test for SCO Unix - cannot run async flushing tests because a
    # potential problem with select is apparently interfering.
    # (Mark Diekhans).

    ConstraintInitializer asyncPipeClose {expr {
⟦TAB⟧    !([string equal unix $::tcl_platform(platform)]⟦SPACE⟧
⟦TAB⟧    && ([catch {exec uname -X | fgrep {Release = 3.2v}}] == 0))}}

    # Test to see if we have a broken version of sprintf with respect
    # to the "e" format of floating-point numbers.

    ConstraintInitializer eformat {string equal [format %g 5e-5] 5e-05}

    # Test to see if execed commands such as cat, echo, rm and so forth
    # are present on this machine.

    ConstraintInitializer unixExecs {
⟦TAB⟧set code 1
        if {[string equal macintosh $::tcl_platform(platform)]} {
⟦TAB⟧    set code 0
        }
        if {[string equal windows $::tcl_platform(platform)]} {
⟦TAB⟧    if {[catch {
⟦TAB⟧        set file _tcl_test_remove_me.txt
⟦TAB⟧        makeFile {hello} $file
⟦TAB⟧    }]} {
⟦TAB⟧        set code 0
⟦TAB⟧    } elseif {
⟦TAB⟧        [catch {exec cat $file}] ||
⟦TAB⟧        [catch {exec echo hello}] ||
⟦TAB⟧        [catch {exec sh -c echo hello}] ||
⟦TAB⟧        [catch {exec wc $file}] ||
⟦TAB⟧        [catch {exec sleep 1}] ||
⟦TAB⟧        [catch {exec echo abc > $file}] ||
⟦TAB⟧        [catch {exec chmod 644 $file}] ||
⟦TAB⟧        [catch {exec rm $file}] ||
⟦TAB⟧        [llength [auto_execok mkdir]] == 0 ||
⟦TAB⟧        [llength [auto_execok fgrep]] == 0 ||
⟦TAB⟧        [llength [auto_execok grep]] == 0 ||
⟦TAB⟧        [llength [auto_execok ps]] == 0
⟦TAB⟧    } {
⟦TAB⟧        set code 0
⟦TAB⟧    }
⟦TAB⟧    removeFile $file
        }
⟦TAB⟧set code
    }

    ConstraintInitializer stdio {
⟦TAB⟧set code 0
⟦TAB⟧if {![catch {set f [open "|[list [interpreter]]" w]}]} {
⟦TAB⟧    if {![catch {puts $f exit}]} {
⟦TAB⟧⟦TAB⟧if {![catch {close $f}]} {
⟦TAB⟧⟦TAB⟧    set code 1
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set code
    }

    # Deliberately call socket with the wrong number of arguments.  The
    # error message you get will indicate whether sockets are available
    # on this system.

    ConstraintInitializer socket {
⟦TAB⟧catch {socket} msg
⟦TAB⟧string compare $msg "sockets are not available on this system"
    }

    # Check for internationalization
    ConstraintInitializer hasIsoLocale {
⟦TAB⟧if {[llength [info commands testlocale]] == 0} {
⟦TAB⟧    set code 0
⟦TAB⟧} else {
⟦TAB⟧    set code [string length [SetIso8859_1_Locale]]
⟦TAB⟧    RestoreLocale
⟦TAB⟧}
⟦TAB⟧set code
    }

}
#####################################################################

# Usage and command line arguments processing.

# tcltest::PrintUsageInfo
#
#⟦TAB⟧Prints out the usage information for package tcltest.  This can
#⟦TAB⟧be customized with the redefinition of [PrintUsageInfoHook].
#
# Arguments:
#⟦TAB⟧none
#
# Results:
#       none
#
# Side Effects:
#       none
proc tcltest::PrintUsageInfo {} {
    puts [Usage]
    PrintUsageInfoHook
}

proc tcltest::Usage { {option ""} } {
    variable Usage
    variable Verify
    if {[llength [info level 0]] == 1} {
⟦TAB⟧set msg "Usage: [file tail [info nameofexecutable]] script "
⟦TAB⟧append msg "?-help? ?flag value? ... \n"
⟦TAB⟧append msg "Available flags (and valid input values) are:"

⟦TAB⟧set max 0
⟦TAB⟧set allOpts [concat -help [Configure]]
⟦TAB⟧foreach opt $allOpts {
⟦TAB⟧    set foo [Usage $opt]
⟦TAB⟧    foreach [list x type($opt) usage($opt)] $foo break
⟦TAB⟧    set line($opt) "  $opt $type($opt)  "
⟦TAB⟧    set length($opt) [string length $line($opt)]
⟦TAB⟧    if {$length($opt) > $max} {set max $length($opt)}
⟦TAB⟧}
⟦TAB⟧set rest [expr {72 - $max}]
⟦TAB⟧foreach opt $allOpts {
⟦TAB⟧    append msg \n$line($opt)
⟦TAB⟧    append msg [string repeat " " [expr {$max - $length($opt)}]]
⟦TAB⟧    set u [string trim $usage($opt)]
⟦TAB⟧    catch {append u "  (default: \[[Configure $opt]])"}
⟦TAB⟧    regsub -all {\s*\n\s*} $u " " u
⟦TAB⟧    while {[string length $u] > $rest} {
⟦TAB⟧⟦TAB⟧set break [string wordstart $u $rest]
⟦TAB⟧⟦TAB⟧if {$break == 0} {
⟦TAB⟧⟦TAB⟧    set break [string wordend $u 0]
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧append msg [string range $u 0 [expr {$break - 1}]]
⟦TAB⟧⟦TAB⟧set u [string trim [string range $u $break end]]
⟦TAB⟧⟦TAB⟧append msg \n[string repeat " " $max]
⟦TAB⟧    }
⟦TAB⟧    append msg $u
⟦TAB⟧}
⟦TAB⟧return $msg\n
    } elseif {[string equal -help $option]} {
⟦TAB⟧return [list -help "" "Display this usage information."]
    } else {
⟦TAB⟧set type [lindex [info args $Verify($option)] 0]
⟦TAB⟧return [list $option $type $Usage($option)]
    }
}

# tcltest::ProcessFlags --
#
#⟦TAB⟧process command line arguments supplied in the flagArray - this
#⟦TAB⟧is called by processCmdLineArgs.  Modifies tcltest variables
#⟦TAB⟧according to the content of the flagArray.
#
# Arguments:
#⟦TAB⟧flagArray - array containing name/value pairs of flags
#
# Results:
#⟦TAB⟧sets tcltest variables according to their values as defined by
#       flagArray
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::ProcessFlags {flagArray} {
    # Process -help first
    if {[lsearch -exact $flagArray {-help}] != -1} {
⟦TAB⟧PrintUsageInfo
⟦TAB⟧exit 1
    }

    if {[llength $flagArray] == 0} {
⟦TAB⟧RemoveAutoConfigureTraces
    } else {
⟦TAB⟧set args $flagArray
⟦TAB⟧while {[llength $args]>1 && [catch {eval configure $args} msg]} {

⟦TAB⟧    # Something went wrong parsing $args for tcltest options
⟦TAB⟧    # Check whether the problem is "unknown option"
⟦TAB⟧    if {[regexp {^unknown option (\S+):} $msg -> option]} {
⟦TAB⟧⟦TAB⟧# Could be this is an option the Hook knows about
⟦TAB⟧⟦TAB⟧set moreOptions [processCmdLineArgsAddFlagsHook]
⟦TAB⟧⟦TAB⟧if {[lsearch -exact $moreOptions $option] == -1} {
⟦TAB⟧⟦TAB⟧    # Nope.  Report the error, including additional options,
⟦TAB⟧⟦TAB⟧    # but keep going
⟦TAB⟧⟦TAB⟧    if {[llength $moreOptions]} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧append msg ", "
⟦TAB⟧⟦TAB⟧⟦TAB⟧append msg [join [lrange $moreOptions 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧⟦TAB⟧append msg "or [lindex $moreOptions end]"
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    Warn $msg
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧# error is something other than "unknown option"
⟦TAB⟧⟦TAB⟧# notify user of the error; and exit
⟦TAB⟧⟦TAB⟧puts [errorChannel] $msg
⟦TAB⟧⟦TAB⟧exit 1
⟦TAB⟧    }

⟦TAB⟧    # To recover, find that unknown option and remove up to it.
⟦TAB⟧    # then retry
⟦TAB⟧    while {![string equal [lindex $args 0] $option]} {
⟦TAB⟧⟦TAB⟧set args [lrange $args 2 end]
⟦TAB⟧    }
⟦TAB⟧    set args [lrange $args 2 end]
⟦TAB⟧}
⟦TAB⟧if {[llength $args] == 1} {
⟦TAB⟧    puts [errorChannel] \
⟦TAB⟧⟦TAB⟧    "missing value for option [lindex $args 0]"
⟦TAB⟧    exit 1
⟦TAB⟧}
    }

    # Call the hook
    catch {
        array set flag $flagArray
        processCmdLineArgsHook [array get flag]
    }
    return
}

# tcltest::ProcessCmdLineArgs --
#
#       This procedure must be run after constraint initialization is
#⟦TAB⟧set up (by [DefineConstraintInitializers]) because some constraints
#⟦TAB⟧can be overridden.
#
#       Perform configuration according to the command-line options.
#
# Arguments:
#⟦TAB⟧none
#
# Results:
#⟦TAB⟧Sets the above-named variables in the tcltest namespace.
#
# Side Effects:
#       None.
#

proc tcltest::ProcessCmdLineArgs {} {
    variable originalEnv
    variable testConstraints

    # The "argv" var doesn't exist in some cases, so use {}.
    if {![info exists ::argv]} {
⟦TAB⟧ProcessFlags {}
    } else {
⟦TAB⟧ProcessFlags $::argv
    }

    # Spit out everything you know if we're at a debug level 2 or
    # greater
    DebugPuts 2 "Flags passed into tcltest:"
    if {[info exists ::env(TCLTEST_OPTIONS)]} {
⟦TAB⟧DebugPuts 2 \
⟦TAB⟧⟦TAB⟧"    ::env(TCLTEST_OPTIONS): $::env(TCLTEST_OPTIONS)"
    }
    if {[info exists ::argv]} {
⟦TAB⟧DebugPuts 2 "    argv: $::argv"
    }
    DebugPuts    2 "tcltest::debug              = [debug]"
    DebugPuts    2 "tcltest::testsDirectory     = [testsDirectory]"
    DebugPuts    2 "tcltest::workingDirectory   = [workingDirectory]"
    DebugPuts    2 "tcltest::temporaryDirectory = [temporaryDirectory]"
    DebugPuts    2 "tcltest::outputChannel      = [outputChannel]"
    DebugPuts    2 "tcltest::errorChannel       = [errorChannel]"
    DebugPuts    2 "Original environment (tcltest::originalEnv):"
    DebugPArray  2 originalEnv
    DebugPuts    2 "Constraints:"
    DebugPArray  2 testConstraints
}

#####################################################################

# Code to run the tests goes here.

# tcltest::TestPuts --
#
#⟦TAB⟧Used to redefine puts in test environment.  Stores whatever goes
#⟦TAB⟧out on stdout in tcltest::outData and stderr in errData before
#⟦TAB⟧sending it on to the regular puts.
#
# Arguments:
#⟦TAB⟧same as standard puts
#
# Results:
#⟦TAB⟧none
#
# Side effects:
#       Intercepts puts; data that would otherwise go to stdout, stderr,
#⟦TAB⟧or file channels specified in outputChannel and errorChannel
#⟦TAB⟧does not get sent to the normal puts function.
namespace eval tcltest::Replace {
    namespace export puts
}
proc tcltest::Replace::puts {args} {
    variable [namespace parent]::outData
    variable [namespace parent]::errData
    switch [llength $args] {
⟦TAB⟧1 {
⟦TAB⟧    # Only the string to be printed is specified
⟦TAB⟧    append outData [lindex $args 0]\n
⟦TAB⟧    return
⟦TAB⟧    # return [Puts [lindex $args 0]]
⟦TAB⟧}
⟦TAB⟧2 {
⟦TAB⟧    # Either -nonewline or channelId has been specified
⟦TAB⟧    if {[string equal -nonewline [lindex $args 0]]} {
⟦TAB⟧⟦TAB⟧append outData [lindex $args end]
⟦TAB⟧⟦TAB⟧return
⟦TAB⟧⟦TAB⟧# return [Puts -nonewline [lindex $args end]]
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧set channel [lindex $args 0]
⟦TAB⟧⟦TAB⟧set newline \n
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧3 {
⟦TAB⟧    if {[string equal -nonewline [lindex $args 0]]} {
⟦TAB⟧⟦TAB⟧# Both -nonewline and channelId are specified, unless
⟦TAB⟧⟦TAB⟧# it's an error.  -nonewline is supposed to be argv[0].
⟦TAB⟧⟦TAB⟧set channel [lindex $args 1]
⟦TAB⟧⟦TAB⟧set newline ""
⟦TAB⟧    }
⟦TAB⟧}
    }

    if {[info exists channel]} {
⟦TAB⟧if {[string equal $channel [[namespace parent]::outputChannel]]
⟦TAB⟧⟦TAB⟧|| [string equal $channel stdout]} {
⟦TAB⟧    append outData [lindex $args end]$newline
⟦TAB⟧    return
⟦TAB⟧} elseif {[string equal $channel [[namespace parent]::errorChannel]]
⟦TAB⟧⟦TAB⟧|| [string equal $channel stderr]} {
⟦TAB⟧    append errData [lindex $args end]$newline
⟦TAB⟧    return
⟦TAB⟧}
    }

    # If we haven't returned by now, we don't know how to handle the
    # input.  Let puts handle it.
    return [eval Puts $args]
}

# tcltest::Eval --
#
#⟦TAB⟧Evaluate the script in the test environment.  If ignoreOutput is
#       false, store data sent to stderr and stdout in outData and
#       errData.  Otherwise, ignore this output altogether.
#
# Arguments:
#⟦TAB⟧script             Script to evaluate
#       ?ignoreOutput?     Indicates whether or not to ignore output
#⟦TAB⟧⟦TAB⟧⟦TAB⟧   sent to stdout & stderr
#
# Results:
#⟦TAB⟧result from running the script
#
# Side effects:
#⟦TAB⟧Empties the contents of outData and errData before running a
#⟦TAB⟧test if ignoreOutput is set to 0.

proc tcltest::Eval {script {ignoreOutput 1}} {
    variable outData
    variable errData
    DebugPuts 3 "[lindex [info level 0] 0] called"
    if {!$ignoreOutput} {
⟦TAB⟧set outData {}
⟦TAB⟧set errData {}
⟦TAB⟧rename ::puts [namespace current]::Replace::Puts
⟦TAB⟧namespace eval :: \
⟦TAB⟧⟦TAB⟧[list namespace import [namespace origin Replace::puts]]
⟦TAB⟧namespace import Replace::puts
    }
    set result [uplevel 1 $script]
    if {!$ignoreOutput} {
⟦TAB⟧namespace forget puts
⟦TAB⟧namespace eval :: namespace forget puts
⟦TAB⟧rename [namespace current]::Replace::Puts ::puts
    }
    return $result
}

# tcltest::CompareStrings --
#
#⟦TAB⟧compares the expected answer to the actual answer, depending on
#⟦TAB⟧the mode provided.  Mode determines whether a regexp, exact,
#⟦TAB⟧glob or custom comparison is done.
#
# Arguments:
#⟦TAB⟧actual - string containing the actual result
#       expected - pattern to be matched against
#       mode - type of comparison to be done
#
# Results:
#⟦TAB⟧result of the match
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::CompareStrings {actual expected mode} {
    variable CustomMatch
    if {![info exists CustomMatch($mode)]} {
        return -code error "No matching command registered for `-match $mode'"
    }
    set match [namespace eval :: $CustomMatch($mode) [list $expected $actual]]
    if {[catch {expr {$match && $match}} result]} {
⟦TAB⟧return -code error "Invalid result from `-match $mode' command: $result"
    }
    return $match
}

# tcltest::customMatch --
#
#⟦TAB⟧registers a command to be called when a particular type of
#⟦TAB⟧matching is required.
#
# Arguments:
#⟦TAB⟧nickname - Keyword for the type of matching
#⟦TAB⟧cmd - Incomplete command that implements that type of matching
#⟦TAB⟧⟦TAB⟧when completed with expected string and actual string
#⟦TAB⟧⟦TAB⟧and then evaluated.
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#⟦TAB⟧Sets the variable tcltest::CustomMatch

proc tcltest::customMatch {mode script} {
    variable CustomMatch
    if {![info complete $script]} {
⟦TAB⟧return -code error \
⟦TAB⟧⟦TAB⟧"invalid customMatch script; can't evaluate after completion"
    }
    set CustomMatch($mode) $script
}

# tcltest::SubstArguments list
#
# This helper function takes in a list of words, then perform a
# substitution on the list as though each word in the list is a separate
# argument to the Tcl function.  For example, if this function is
# invoked as:
#
#      SubstArguments {$a {$a}}
#
# Then it is as though the function is invoked as:
#
#      SubstArguments $a {$a}
#
# This code is adapted from Paul Duffin's function "SplitIntoWords".
# The original function can be found  on:
#
#      http://purl.org/thecliff/tcl/wiki/858.html
#
# Results:
#     a list containing the result of the substitution
#
# Exceptions:
#     An error may occur if the list containing unbalanced quote or
#     unknown variable.
#
# Side Effects:
#     None.
#

proc tcltest::SubstArguments {argList} {

    # We need to split the argList up into tokens but cannot use list
    # operations as they throw away some significant quoting, and
    # [split] ignores braces as it should.  Therefore what we do is
    # gradually build up a string out of whitespace seperated strings.
    # We cannot use [split] to split the argList into whitespace
    # separated strings as it throws away the whitespace which maybe
    # important so we have to do it all by hand.

    set result {}
    set token ""

    while {[string length $argList]} {
        # Look for the next word containing a quote: " { }
        if {[regexp -indices {[^ \t\n]*[\"\{\}]+[^ \t\n]*} \
⟦TAB⟧⟦TAB⟧$argList all]} {
            # Get the text leading up to this word, but not including
⟦TAB⟧    # this word, from the argList.
            set text [string range $argList 0 \
⟦TAB⟧⟦TAB⟧    [expr {[lindex $all 0] - 1}]]
            # Get the word with the quote
            set word [string range $argList \
                    [lindex $all 0] [lindex $all 1]]

            # Remove all text up to and including the word from the
            # argList.
            set argList [string range $argList \
                    [expr {[lindex $all 1] + 1}] end]
        } else {
            # Take everything up to the end of the argList.
            set text $argList
            set word {}
            set argList {}
        }

        if {$token != {}} {
            # If we saw a word with quote before, then there is a
            # multi-word token starting with that word.  In this case,
            # add the text and the current word to this token.
            append token $text $word
        } else {
            # Add the text to the result.  There is no need to parse
            # the text because it couldn't be a part of any multi-word
            # token.  Then start a new multi-word token with the word
            # because we need to pass this token to the Tcl parser to
            # check for balancing quotes
            append result $text
            set token $word
        }

        if { [catch {llength $token} length] == 0 && $length == 1} {
            # The token is a valid list so add it to the result.
            # lappend result [string trim $token]
            append result \{$token\}
            set token {}
        }
    }

    # If the last token has not been added to the list then there
    # is a problem.
    if { [string length $token] } {
        error "incomplete token \"$token\""
    }

    return $result
}

# tcltest::test --
#
# This procedure runs a test and prints an error message if the test
# fails.  If verbose has been set, it also prints a message even if the
# test succeeds.  The test will be skipped if it doesn't match the
# match variable, if it matches an element in skip, or if one of the
# elements of "constraints" turns out not to be true.
#
# If testLevel is 1, then this is a top level test, and we record
# pass/fail information; otherwise, this information is not logged and
# is not added to running totals.
#
# Attributes:
#   Only description is a required attribute.  All others are optional.
#   Default values are indicated.
#
#   constraints -⟦TAB⟧A list of one or more keywords, each of which
#⟦TAB⟧⟦TAB⟧⟦TAB⟧must be the name of an element in the array
#⟦TAB⟧⟦TAB⟧⟦TAB⟧"testConstraints".  If any of these elements is
#⟦TAB⟧⟦TAB⟧⟦TAB⟧zero, the test is skipped. This attribute is
#⟦TAB⟧⟦TAB⟧⟦TAB⟧optional; default is {}
#   body -⟦TAB⟧        Script to run to carry out the test.  It must
#⟦TAB⟧⟦TAB⟧        return a result that can be checked for
#⟦TAB⟧⟦TAB⟧        correctness.  This attribute is optional;
#                       default is {}
#   result -⟦TAB⟧        Expected result from script.  This attribute is
#                       optional; default is {}.
#   output -            Expected output sent to stdout.  This attribute
#                       is optional; default is {}.
#   errorOutput -       Expected output sent to stderr.  This attribute
#                       is optional; default is {}.
#   returnCodes -       Expected return codes.  This attribute is
#                       optional; default is {0 2}.
#   setup -             Code to run before $script (above).  This
#                       attribute is optional; default is {}.
#   cleanup -           Code to run after $script (above).  This
#                       attribute is optional; default is {}.
#   match -             specifies type of matching to do on result,
#                       output, errorOutput; this must be a string
#⟦TAB⟧⟦TAB⟧⟦TAB⟧previously registered by a call to [customMatch].
#⟦TAB⟧⟦TAB⟧⟦TAB⟧The strings exact, glob, and regexp are pre-registered
#⟦TAB⟧⟦TAB⟧⟦TAB⟧by the tcltest package.  Default value is exact.
#
# Arguments:
#   name -⟦TAB⟧⟦TAB⟧Name of test, in the form foo-1.2.
#   description -⟦TAB⟧Short textual description of the test, to
#  ⟦TAB⟧⟦TAB⟧  ⟦TAB⟧help humans understand what it does.
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#       Just about anything is possible depending on the test.
#

proc tcltest::test {name description args} {
    global tcl_platform
    variable testLevel
    variable coreModTime
    DebugPuts 3 "test $name $args"
    DebugDo 1 {
⟦TAB⟧variable TestNames
⟦TAB⟧catch {
⟦TAB⟧    puts "test name '$name' re-used; prior use in $TestNames($name)"
⟦TAB⟧}
⟦TAB⟧set TestNames($name) [info script]
    }

    FillFilesExisted
    incr testLevel

    # Pre-define everything to null except output and errorOutput.  We
    # determine whether or not to trap output based on whether or not
    # these variables (output & errorOutput) are defined.
    foreach item {constraints setup cleanup body result returnCodes
⟦TAB⟧    match} {
⟦TAB⟧set $item {}
    }

    # Set the default match mode
    set match exact

    # Set the default match values for return codes (0 is the standard
    # expected return value if everything went well; 2 represents
    # 'return' being used in the test script).
    set returnCodes [list 0 2]

    # The old test format can't have a 3rd argument (constraints or
    # script) that starts with '-'.
⟦SPACE⟧
    # Save information about the core file.⟦SPACE⟧⟦SPACE⟧
    if {[preserveCore]} {
⟦TAB⟧if {[file exists [file join [workingDirectory] core]]} {
⟦TAB⟧    set coreModTime [file mtime [file join [workingDirectory] core]]
⟦TAB⟧}
    }

    # First, run the setup script
    set code [catch {uplevel 1 $setup} setupMsg]
    if {$code == 1} {
⟦TAB⟧set errorInfo(setup) $::errorInfo
⟦TAB⟧set errorCode(setup) $::errorCode
    }
    set setupFailure [expr {$code != 0}]

    # Only run the test body if the setup was successful
    if {!$setupFailure} {

⟦TAB⟧# Verbose notification of $body start
⟦TAB⟧if {[IsVerbose start]} {
⟦TAB⟧    puts [outputChannel] "---- $name start"
⟦TAB⟧    flush [outputChannel]
⟦TAB⟧}

⟦TAB⟧set command [list [namespace origin RunTest] $name $body]
⟦TAB⟧if {[info exists output] || [info exists errorOutput]} {
⟦TAB⟧    set testResult [uplevel 1 [list [namespace origin Eval] $command 0]]
⟦TAB⟧} else {
⟦TAB⟧    set testResult [uplevel 1 [list [namespace origin Eval] $command 1]]
⟦TAB⟧}
⟦TAB⟧foreach {actualAnswer returnCode} $testResult break
⟦TAB⟧if {$returnCode == 1} {
⟦TAB⟧    set errorInfo(body) $::errorInfo
⟦TAB⟧    set errorCode(body) $::errorCode
⟦TAB⟧}
    }

    # Always run the cleanup script
    set code [catch {uplevel 1 $cleanup} cleanupMsg]
    if {$code == 1} {
⟦TAB⟧set errorInfo(cleanup) $::errorInfo
⟦TAB⟧set errorCode(cleanup) $::errorCode
    }
    set cleanupFailure [expr {$code != 0}]

    set coreFailure 0
    set coreMsg ""
    # check for a core file first - if one was created by the test,
    # then the test failed
    if {[preserveCore]} {
⟦TAB⟧if {[file exists [file join [workingDirectory] core]]} {
⟦TAB⟧    # There's only a test failure if there is a core file
⟦TAB⟧    # and (1) there previously wasn't one or (2) the new
⟦TAB⟧    # one is different from the old one.
⟦TAB⟧    if {[info exists coreModTime]} {
⟦TAB⟧⟦TAB⟧if {$coreModTime != [file mtime \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [workingDirectory] core]]} {
⟦TAB⟧⟦TAB⟧    set coreFailure 1
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧set coreFailure 1
⟦TAB⟧    }
⟦TAB⟧
⟦TAB⟧    if {([preserveCore] > 1) && ($coreFailure)} {
⟦TAB⟧⟦TAB⟧append coreMsg "\nMoving file to:\
⟦TAB⟧⟦TAB⟧    [fi  puts [outputChannel] "\n==== $name\
⟦TAB⟧    [string trim $description] FAILED"
    if {[string length $body]} {
⟦TAB⟧puts [outputChannel] "==== Contents of test case:"
⟦TAB⟧puts [outputChannel] $body
    }
   s [outputChannel] "---- errorCode(setup): $errorCode(setup)"
⟦TAB⟧}
    }
    if {$scriptFailure} {
⟦TAB⟧if {$scriptCompare} {
⟦TAB⟧    puts [outputChannel] "---- Error testing result: $scriptMatch"
⟦TAB⟧} else {
⟦TAB⟧    puts [outputChannel] "---- Result was:\n$actualAnswer"
⟦TAB⟧    puts [outputChannel] "---- Result should have been\
⟦TAB⟧⟦TAB⟧    ($match matching):\n$result"
⟦TAB⟧}
    }
    if {$codeFailure} {
⟦TAB⟧switch -- $returnCode {
⟦TAB⟧    0 { set msg "Test completed normally" }
⟦TAB⟧    1 { set msg "Test generated error" }
⟦TAB⟧    2 { set msg "Test generated return exception" }
⟦TAB⟧    3 { set msg "Test generated break exception" }
⟦TAB⟧    4 { set msg "Test generated continue exception" }
⟦TAB⟧    default { set msg "Test generated exception" }
⟦TAB⟧}
⟦TAB⟧puts [outputChannel] "---- $msg; Return code was: $returnCode"
⟦TAB⟧puts [outputChannel] "---- Return code should have been\
⟦TAB⟧⟦TAB⟧one of: $returnCodes"
⟦TAB⟧if {[IsVerbose error]} {
⟦TAB⟧    if {[info exists errorInfo(body)] && ([lsearch $returnCodes 1]<0)} {
⟦TAB⟧⟦TAB⟧puts [outputChannel] "---- errorInfo: $errorInfo(body)"
⟦TAB⟧⟦TAB⟧puts [outputChannel] "---- errorCode: $errorCode(body)"
⟦TAB⟧    }
⟦TAB⟧}
    }
    if {$outputFailure} {
⟦TAB⟧if {$outputCompare} {
⟦TAB⟧    puts [outputChannel] "---- Error testing output: $outputMatch"
⟦TAB⟧} else {
⟦TAB⟧    puts [outputChannel] "---- Output was:\n$outData"
⟦TAB⟧    puts [outputChannel] "---- Output should have been\
⟦TAB⟧⟦TAB⟧    ($match matching):\n$output"
⟦TAB⟧}
    }
    if {$errorFailure} {
⟦TAB⟧if {$errorCompare} {
⟦TAB⟧    puts [outputChannel] "---- Error testing errorOutput: $errorMatch"
⟦TAB⟧} else {
⟦TAB⟧    puts [outputChannel] "---- Error output was:\n$errData"
⟦TAB⟧    puts [outputChannel] "---- Error output should have\
⟦TAB⟧⟦TAB⟧    been ($match matching):\n$errorOutput"
⟦TAB⟧}
    }
    if {$cleanupFailure} {
⟦TAB⟧puts [outputChannel] "---- Test cleanup failed:\n$cleanupMsg"
⟦TAB⟧if {[info exists errorInfo(cleanup)]}straints] != 0} {
⟦TAB⟧    # full expression, e.g. {$foo > [info tclversion]}
⟦TAB⟧    catch {set doTest [uplevel #0 expr $constraints]}
⟦TAB⟧} elseif {[regexp {[^.a-zA-Z0-9 \n\r\t]+} $constraints] != 0} {
⟦TAB⟧    # something like {a || b} should be turned into
⟦TAB⟧    # $testConstraints(a) || $testConstraints(b).
⟦TAB⟧    regsub -all {[.\w]+} $constraints {$testConstraints(&)} c
⟦TAB⟧    catch {set doTest [eval expr $c]}
⟦TAB⟧} elseif {![catch {llength $constraints}]} {
⟦TAB⟧    # just simple constraints such as {unixOnly fonts}.
⟦TAB⟧    set doTest 1
⟦TAB⟧    foreach constraint $constraints {
⟦TAB⟧⟦TAB⟧if {(![info exists testConstraints($constraint)]) \
⟦TAB⟧⟦TAB⟧⟦TAB⟧|| (!$testConstraints($constraint))} {
⟦TAB⟧⟦TAB⟧    set doTest 0

⟦TAB⟧⟦TAB⟧    # store the constraint that kept the test from
⟦TAB⟧⟦TAB⟧    # running
⟦TAB⟧⟦TAB⟧    set constraints $constraint
⟦TAB⟧⟦TAB⟧    break
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧
⟦TAB⟧if {$doTest == 0} {
⟦TAB⟧    if {[IsVerbose skip]} {
⟦TAB⟧⟦TAB⟧puts [outputChannel] "++++ $name SKIPPED: $constraints"
⟦TAB⟧    }

⟦TAB⟧    if {$testLevel == 1} {
⟦TAB⟧⟦TAB⟧incr numTests(Skipped)
⟦TAB⟧⟦TAB⟧AddToSkippedBecause $constraints
⟦TAB⟧    }
⟦TAB⟧    return 1
⟦TAB⟧}
    }
    return 0
}

# RunTest --
#
# This is where the body of a test is evaluated.  The combination of
# [RunTest] and [Eval] allows the output and error output of the test
# body to be captured for comparison against the expected values.

proc tcltest::RunTest {name script} {
    DebugPuts 3 "Running $name {$script}"

    # If there is no "memory" command (because memory debugging isn't
    # enabled), then don't attempt to use the command.

    if {[llength [info commands memory]] == 1} {
⟦TAB⟧memory tag $name
    }

    set code [catch {uplevel 1 $script} actualAnswer]

    return [list $actualAnswer $code]
}

#####################################################################

# tcltest::cleanupTestsHook --
#
#⟦TAB⟧This hook allows a harness that builds upon tcltest to specify
#       additional things that should be done at cleanup.
#

if {[llength [info commands tcltest::cleanupTestsHook]] == 0} {
    proc tcltest::cleanupTestsHook {} {}
}

# tcltemTests
    variable numTestFiles
    variable failFiles
    variable skippedBecause
    variable currentFailure
    variable originalEnv
    variable originalTclPlatform
    variable coreModTime

    FillFilesExisted
    set testFileName [file tail [info script]]

    # Call the cleanup hook
    cleanupTestsHook

    # Remove files and directories created by the makeFile and
    # makeDirectory procedures.  Record the names of files in
    # workingDirectory that were not pre-existing, and associate them
    # with the test file that created them.

    if {!$calledFromAllFile} {
⟦TAB⟧foreach file $filesMade {
⟦TAB⟧    if {[file exists $file]} {
⟦TAB⟧⟦TAB⟧DebugDo 1 {Warn "cleanupTests deleting $file..."}
⟦TAB⟧⟦TAB⟧catch {file delete -force $file}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set currentFiles {}
⟦TAB⟧foreach file [glob -nocomplain \
⟦TAB⟧⟦TAB⟧-directory [temporaryDirectory] *] {
⟦TAB⟧    lappend currentFiles [file tail $file]
⟦TAB⟧}
⟦TAB⟧set newFiles {}
⟦TAB⟧foreach file $currentFiles {
⟦TAB⟧    if {[lsearch -exact $filesExisted $file] == -1} {
⟦TAB⟧⟦TAB⟧lappend newFiles $file
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set filesExisted $currentFiles
⟦TAB⟧if {[llength $newFiles] > 0} {
⟦TAB⟧    set createdNewFiles($testFileName) $newFiles
⟦TAB⟧}
    }

    if {$calledFromAllFile || $testSingleFile} {

⟦TAB⟧# print stats

⟦TAB⟧puts -nonewline [outputChannel] "$testFileName:"
⟦TAB⟧foreach index [list "Total" "Passed" "Skipped" "Failed"] {
⟦TAB⟧    puts -nonewline [outputChannel] \
⟦TAB⟧⟦TAB⟧    "\t$index\t$numTests($index)"
⟦TAB⟧}
⟦TAB⟧puts [outputChannel] ""

⟦TAB⟧# print number test files sourced
⟦TAB⟧# print names of files that ran tests which failed

⟦TAB⟧if {$calledFromAllFile} {
⟦TAB⟧    puts [outputChannel] \
⟦TAB⟧⟦TAB⟧    "Sourced $numTestFiles Test####################################################

# Test utility procs - not used in tcltest, but may be useful for
# testing.

# tcltest::loadTestedCommands --
#
#     Uses the specified script to load the commands to test. Allowed to
#     be empty, as the tested commands could have been compiled into the
#     interpreter.
#
# Arguments
#     none
#
# Results
#     none
#
# Side Effects:
# e --
#
#⟦TAB⟧Remove procs and variables that didn't exist before the call to
#       [saveState].
#
# Arguments:
#⟦TAB⟧none
#
# Results:
#⟦TAB⟧Removes procs and variables from your environment if they don't
#⟦TAB⟧exist in the saveState variable.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::restoreState {} {
    variable saveState
    foreach p [uplevel 1 {::info procs}] {
⟦TAB⟧if {([lsearch [lindex $saveState 0] $p] < 0)st ::catch [list ::unset $p]]
⟦TAB⟧}
    }
    return
}

# tcltest::normalizeMsg --
#
#⟦TAB⟧Removes "extra" newlines from a string.
#
# Arguments:
#⟦TAB⟧msg        String to be modified
#
# Results:
#⟦TAB⟧string with extra newlines removed
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::normalizeMsg {msg} {
    regsub "\n$" [string tolower $msg] "" msg
    set msg [string map [list "\n\n" "\n"] $msg]
    return [string map [list "\n\}" "\}"] $msg]
}

# tcltest::makeFile --
#
# Create a new file with the name <name>, and write <contents> to it.
#
# If this file hasn't been created via makeFile since the last time
# cleanupTests was called, add it to the $filesMade list, so it will be
# removed by the next call to cleanupTests.
#
# Arguments:
#⟦TAB⟧contents        content of the new file
#       name            name of the new file
#       directory       directory name for new file
#
# Results:
#⟦TAB⟧absolute path to the file created
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::makeFile {contents name {directory ""}} {
    variable filesMade
    FillFilesExisted

    if {[llength [info level 0]] == 3} {
⟦TAB⟧set directory [temporaryDirectory]
    }

    set fullName [file join $directory $name]

    DebugPuts 3 "[lindex [info level 0] 0]:\
⟦TAB⟧     putting ``$contents'' into $fullName"

    set fd [open $fullName w]
    fconfigure $fd -translation lf
    if {[string equal [string index $contents end] \n]} {
⟦TAB⟧puts -nonewline $fd $contents
    } else {
⟦TAB⟧puts $fd $contents
    }
    close $fd

    if {[lsearch -exact $filesMade $fullName] == -1} {
⟦TAB⟧lappend filesMade $fullName
    }
    return $fullName
}

# tcltest::removeFile --
#
#⟦TAB⟧Removes the named file from the filesystem
#
# Arguments:
#⟦TAB⟧name          file to be removed
#       directory     directory from which to remove file
#
# Results:
#⟦TAB⟧return value from [file delete]
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::removeFile {name {directory ""}} {
    variable filesMade
    FillFilesExisted
    if {[llength [info level 0]] == 2} {
⟦TAB⟧set directory [temporaryDirectory]
    }
    set fullName [file join $directory $name]
    DebugPuts 3 "[lindex [info level 0] 0]: removing $fullName"
    set idx [lsearch -exact $filesMade $fullName]
    set filesMade [lreplace $filesMade $idx $idx]
    if {$idx == -1} {
⟦TAB⟧DebugDo 1 {
⟦TAB⟧    Warn "removeFile removing \"$fullName\":\n  not created by makeFile"
⟦TAB⟧}
    }⟦SPACE⟧
    if {![file isfile $fullName]} {
⟦TAB⟧DebugDo 1 {
⟦TAB⟧    Warn "removeFile removing \"$fullName\":\n  not a file"
⟦TAB⟧}
    }
    return [file delete $fullName]
}

# tcltest::makeDirectory --
#
# Create a new dir with the name <name>.
#
# If this dir hasn't been created via makeDirectory since the last time
# cleanupTests was called, add it to the $directoriesMade list, so it
# will be removed by the next call to cleanupTests.
#
# Arguments:
#       name            name of the new directory
#       directory       directory in which to create new dir
#
# Results:
#⟦TAB⟧absolute path to the directory created
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::makeDirectory {name {directory ""}} {
    variable filesMade
    FillFilesExisted
    if {[llength [info level 0]] == 2} {
⟦TAB⟧set directory [temporaryDirectory]
    }
    set fullName [file join $directory $name]
    DebugPuts 3 "[lindex [info level 0] 0]: creating $fullName"
    file mkdir $fullName
    if {[lsearch -exact $filesMade $fullName] == -1} {
⟦TAB⟧lappend filesMade $fullName
    }
    return $fullName
}

# tcltest::removeDirectory --
#
#⟦TAB⟧Removes a named directory from the file system.
#
# Arguments:
#⟦TAB⟧name          Name of the directory to remove
#       directory     Directory from which to remove
#
# Results:
#⟦TAB⟧return value from [file delete]
#
# Side effects:
#⟦TAB⟧None

proc tcltest::removeDirectory {name {directory ""}} {
    variable filesMade
    FillFilesExisted
    if {[llength [info levsMade $fullName]
    set filesMade [lreplace $filesMade $idx $idx]
    if {$idx == -1} {
⟦TAB⟧DebugDo 1 {
⟦TAB⟧    Warn "removeDirectory removing \"$fullName\":\n  not created\
⟦TAB⟧⟦TAB⟧    by makeDirectory"
⟦TAB⟧}
    e --
#
#⟦TAB⟧reads the content of a file and returns it
#
# Arguments:
#⟦TAB⟧name of the file to read
#       directory in which file is located
#
# Results:
#⟦TAB⟧content of the named file
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::viewFile {name {directory ""}} {
    FillFilesExisted
    if {[llength [info level 0]] == 2} {
⟦TAB⟧set directory [temporaryDirectory]
    }
    set fullName [file join $directory $name]
    set f [open $fullName]
    set data [read -nonewline $f]
    close $f
    return $data
}

# tcltest::bytestring --
#
# Construct a string that consists of the requested sequence of bytes,
# as opposed to a string of properly formed UTF-8 characters.
# This allows the tester to
# 1. Create denormalized or improperly formed strings to pass to C
#    procedures that are supposed to accept strings with embedded NULL
#    bytes.
# 2. Confirm that a string result has a certain pattern of bytes, for
#    instance to confirm that "\xe0\0" in a Tcl script is stored
#    internally in UTF-8 as the sequence of bytes "\xc3\xa0\xc0\x80".
#
# Generally, it's a bad idea to examine the bytes in a Tcl string or to
# construct improperly formed strings in this manner, because it involves
# exposing that Tcl uses UTF-8 internally.
#
# Arguments:
#⟦TAB⟧string being converted
#
# Results:
#⟦TAB⟧result fom encoding
#
# Side effects:
#⟦TAB⟧None

proc tcltest::bytestring {string} {
    return [encoding convertfrom identity $string]
}

# tcltest::OpenFiles --
#
#⟦TAB⟧used in io tests, uses testchannel
#
# Arguments:
#⟦TAB⟧None.
#
# Results:
#⟦TAB⟧???
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::OpenFiles {} {
    if {[catch {testchannel open} result]} {
⟦TAB⟧return {}
    }
    return $result
}

# tcltest::LeakFiles --
#
#⟦TAB⟧used in io tests, uses testchannel
#
# Arguments:
#⟦TAB⟧None.
#
# Results:
#⟦TAB⟧???
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::LeakFiles {old} {
    if {[catch {testchannel open} new]} {
⟦TAB⟧return {}
    }
    set leak {}
    foreach p $new {
⟦TAB⟧if {[lsearch $old $p] < 0} {
⟦TAB⟧    lappend leak $p
⟦TAB⟧}
    }
    return $leak
}

#
# Internationalization / ISO support procs     -- dl
#

# tcltest::SetIso8859_1_Locale --
#
#⟦TAB⟧used in cmdIL.test, uses testlocale
#
# Arguments:
#⟦TAB⟧None.
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::SetIso8859_1_Locale {} {
    variable previousLocale
    variable isoLocale
    if {[info commands testlocale] != ""} {
⟦TAB⟧set previousLocale [testlocale ctype]
⟦TAB⟧testlocale ctype $isoLocale
    }
    return
}

# tcltest::RestoreLocale --
#
#⟦TAB⟧used in cmdIL.test, uses testlocale
#
# Arguments:
#⟦TAB⟧None.
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::RestoreLocale {} {
    variable previousLocale
    if {[info commands testlocale] != ""} {
⟦TAB⟧testlocale ctype $previousLocale
    }
    return
}

# tcltest::threadReap --
#
#⟦TAB⟧Kill all threads except for the main thread.
#⟦TAB⟧Do nothing if testthread is not defined.
#
# Arguments:
#⟦TAB⟧none.
#
# Results:
#⟦TAB⟧Returns the number of existing threads.
#
# Side Effects:
#       none.
#

proc tcltest::threadReap {} {
    if {[info commands testthread] != {}} {

⟦TAB⟧# testthread built into tcltest

⟦TAB⟧testthread errorproc ThreadNullError
⟦TAB⟧while {[llength [testthread names]] > 1} {
⟦TAB⟧    foreach tid [testthread names] {
⟦TAB⟧⟦TAB⟧if {$tid != [mainThread]} {
⟦TAB⟧⟦TAB⟧    catch {
⟦TAB⟧⟦TAB⟧⟦TAB⟧testthread send -async $tid {testthread exit}
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧    ## Enter a bit a sleep to give the threads enough breathing
⟦TAB⟧    ## room to kill themselves off, otherwise the end up with a
⟦TAB⟧    ## massive queue of repeated events
⟦TAB⟧    after 1
⟦TAB⟧}
⟦TAB⟧testthread errorproc ThreadError
⟦TAB⟧return [llength [testthread names]]
    } elseif {[info commands thread::id] !nstraint] command
    # straight away, without all this "hook" nonsense.
    if {[string equal [namespace current] \
⟦TAB⟧    [namespace qualifiers [namespace which initConstraintsHook]]]} {
⟦TAB⟧InitConstraints
    } else {
⟦TAB⟧proc initConstraintsHook {} {}
    }

    # Define the standard match commands
    customMatch exact⟦TAB⟧[list string equal]
    customMatch glob⟦TAB⟧[list string match]
    customMatch regexp⟦TAB⟧[list regexp --]

    # If the TCLTEST_OPTIONS environment variable exists, configure
    # tcltest according to the option values it specifies.  This has
    # the effect of resetting tcltest's default configuration.
    proc ConfigureFromEnvironment {} {
⟦TAB⟧upvar #0 env(TCLTEST_OPTIONS) options
⟦TAB⟧if {[catch {llength $options} msg]} {
⟦TAB⟧    Warn "invalid TCLTEST_OPTIONS \"$options\":\n  invalid\
⟦TAB⟧⟦TAB⟧    Tcl list: $msg"
⟦TAB⟧    return
⟦TAB⟧}
⟦TAB⟧if {[llength $::env(TCLTEST_OPTIONS)] % 2} {
⟦TAB⟧    Warn "invalid TCLTEST_OPTIONS: \"$options\":\n  should be\
⟦TAB⟧⟦TAB⟧    -option value ?-option value ...?"
⟦TAB⟧    return
⟦TAB⟧}
⟦TAB⟧if {[catch {eval Configure $::env(TCLTEST_OPTIONS)} msg]} {
⟦TAB⟧    Warn "invalid TCLTEST_OPTIONS: \"$options\":\n  $msg"
⟦TAB⟧    return
⟦TAB⟧}
    }
    if {[info exists ::env(TCLTEST_OPTIONS)]} {
⟦TAB⟧ConfigureFromEnvironment
    }

    proc LoadTimeCmdLineArgParsingRequired {} {
⟦TAB⟧set required false
⟦TAB⟧if {[info exists ::argv] && [lsearch -exact $::argv -help] != -1} {
⟦TAB⟧    # The command line asks for -help, so give it (and exit)
⟦TAB⟧    # right now.  ([configure] does not process -help)
⟦TAB⟧    set required true
⟦TAB⟧}
⟦TAB⟧foreach hook { PrintUsageInfoHook processCmdLineArgsHook
⟦TAB⟧⟦TAB⟧⟦TAB⟧processCmdLineArgsAddFlagsHook } {
⟦TAB⟧    if {[string equal [namespace current] [namespace qualifiers \
⟦TAB⟧⟦TAB⟧    [namespace which $hook]]]} {
⟦TAB⟧⟦TAB⟧set required true
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧proc $hook args {}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $required
    }

    # O⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 0, result:

```text-visible
child-provider {8.4.6 8.4 ::} module-code 1 module-result {invalid command name "pwd"} require-code 1 require-result {can't find package tcltest}⟦EOF⟧
```

- real-harness-explicit-source-child, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- real-harness-explicit-source-child: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 5

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
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

    # tests that use threads need to know which is the main thread
    Default mainThread 1
    variable mainThread
    if {[info commands thread::id] != {}} {
⟦TAB⟧set mainThread [thread::id]
    } elseif {[info commands testthread] != {}} {
⟦TAB⟧set mainThread [testthread id]
    }

    # Set workingDirectory to [pwd]. The default output directory for
    # Tcl tests is the working directory.  Whenever this value changes
    # change to that directory.
    variable workingDirectory
    trace variable workingDirectory w \
⟦TAB⟧    [namespace code {cd $workingDirectory ;#}]

    Default workingDirectory [pwd] AcceptAbsolutePath
    proc workingDirectory { {dir ""} } {
⟦TAB⟧variable workingDirectory
⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $workingDirectory
⟦TAB⟧}
⟦TAB⟧set workingDirectory [AcceptAbsolutePath $dir]
    }

    # Set the location of the execuatble
    Default tcltest [info nameofexecutable]
    trace variable tcltest w [namespace code {testConstraint stdio \
⟦TAB⟧    [eval [ConstraintInitializer stdio]] ;#}]

    # save the platform information so it can be restored later
    Default originalTclPlatform [array get ::tcl_platform]

    # If a core file exists, save its modification time.
    if {[file exists [file join [workingDirectory] core]]} {
⟦TAB⟧Default coreModTime \
⟦TAB⟧⟦TAB⟧[file mtime [file join [workingDirectory] core]]
    }

    # stdout and stderr buffers for use when we want to store them
    Default outData {}
    Default errData {}

    # keep track of test level for nested test commands
    variable testLevel 0

    # the variables and procs that existed when saveState was called are
    # stored in a variable of the same name
    Default saveState {}

    # Internationalization support -- used in [SetIso8859_1_Locale] and
    # [RestoreLocale]. Those commands are used in cmdIL.test.

    if {![info exists [namespace current]::isoLocale]} {
⟦TAB⟧variable isoLocale fr
⟦TAB⟧switch -- $::tcl_platform(platform) {
⟦TAB⟧    "unix" {

⟦TAB⟧⟦TAB⟧# Try some 'known' values for some platforms:

⟦TAB⟧⟦TAB⟧switch -exact -- $::tcl_platform(os) {
⟦TAB⟧⟦TAB⟧    "FreeBSD" {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr_FR.ISO_8859-1
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    HP-UX {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr_FR.iso88591
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    Linux -
⟦TAB⟧⟦TAB⟧    IRIX {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    default {

⟦TAB⟧⟦TAB⟧⟦TAB⟧# Works on SunOS 4 and Solaris, and maybe
⟦TAB⟧⟦TAB⟧⟦TAB⟧# others...  Define it to something else on your
⟦TAB⟧⟦TAB⟧⟦TAB⟧# system if you want to test those.

⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale iso_8859_1
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧    "windows" {
⟦TAB⟧⟦TAB⟧set isoLocale French
⟦TAB⟧    }
⟦TAB⟧}
    }

    variable ChannelsWeOpened; array set ChannelsWeOpened {}
    # output goes to stdout by default
    Default outputChannel stdout
    proc outputChannel { {filename ""} } {
⟦TAB⟧variable outputChannel
⟦TAB⟧variable ChannelsWeOpened

⟦TAB⟧# This is very subtle and tricky, so let me try to explain.
⟦TAB⟧# (Hopefully this longer comment will be clear when I come
⟦TAB⟧# back in a few months, unlike its predecessor :) )
⟦TAB⟧#⟦SPACE⟧
⟦TAB⟧# The [outputChannel] command (and underlying variable) have to
⟦TAB⟧# be kept in sync with the [configure -outfile] configuration
⟦TAB⟧# option ( and underlying variable Option(-outfile) ).  This is
⟦TAB⟧# accomplished with a write trace on Option(-outfile) that will
⟦TAB⟧# update [outputChannel] whenver a new value is written.  That
⟦TAB⟧# much is easy.
⟦TAB⟧#
⟦TAB⟧# The trick is that in order to maintain compatibility with
⟦TAB⟧# version 1 of tcltest, we must allow every configuration option
⟦TAB⟧# to get its inital value from command line arguments.  This is
⟦TAB⟧# accomplished by setting initial read traces on all the
⟦TAB⟧# configuration options to parse the command line option the first
⟦TAB⟧# time they are read.  These traces are cancelled whenever the
⟦TAB⟧# program itself calls [configure].
⟦TAB⟧#⟦SPACE⟧
⟦TAB⟧# OK, then so to support tcltest 1 compatibility, it seems we want
⟦TAB⟧# to get the return from [outputFile] to trigger the read traces,
⟦TAB⟧# just in case.
⟦TAB⟧#
⟦TAB⟧# BUT!  A little known feature of Tcl variable traces is that⟦SPACE⟧
⟦TAB⟧# traces are disabled during the handling of other traces.  So,
⟦TAB⟧# if we trigger read traces on Option(-outfile) and that triggers
⟦TAB⟧# command line parsing which turns around and sets an initial
⟦TAB⟧# value for Option(-outfile) -- <whew!> -- the write trace that
⟦TAB⟧# would keep [outputChannel] in sync with that new initial value
⟦TAB⟧# would not fire!
⟦TAB⟧#
⟦TAB⟧# SO, finally, as a workaround, instead of triggering read traces
⟦TAB⟧# by invoking [outputFile], we instead trigger the same set of
⟦TAB⟧# read traces by invoking [debug].  Any command that reads a
⟦TAB⟧# configuration option would do.  [debug] is just a handy one.
⟦TAB⟧# The end result is that we support tcltest 1 compatibility and
⟦TAB⟧# keep outputChannel and -outfile in sync in all cases.
⟦TAB⟧debug

⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $outputChannel
⟦TAB⟧}
⟦TAB⟧if {[info exists ChannelsWeOpened($outputChannel)]} {
⟦TAB⟧    close $outputChannel
⟦TAB⟧    unset ChannelsWeOpened($outputChannel)
⟦TAB⟧}
⟦TAB⟧switch -exact -- $filename {
⟦TAB⟧    stderr -
⟦TAB⟧    stdout {
⟦TAB⟧⟦TAB⟧set outputChannel $filename
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧set outputChannel [open $filename a]
⟦TAB⟧⟦TAB⟧set ChannelsWeOpened($outputChannel) 1

⟦TAB⟧⟦TAB⟧# If we created the file in [temporaryDirectory], then
⟦TAB⟧⟦TAB⟧# [cleanupTests] will delete it, unless we claim it was
⟦TAB⟧⟦TAB⟧# already there.
⟦TAB⟧⟦TAB⟧set outdir [normalizePath [file dirname \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [pwd] $filename]]]
⟦TAB⟧⟦TAB⟧if {[string equal $outdir [temporaryDirectory]]} {
⟦TAB⟧⟦TAB⟧    variable filesExisted
⟦TAB⟧⟦TAB⟧    FillFilesExisted
⟦TAB⟧⟦TAB⟧    set filename [file tail $filename]
⟦TAB⟧⟦TAB⟧    if {[lsearch -exact $filesExisted $filename] == -1} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧lappend filesExisted $filename
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $outputChannel
    }

    # errors go to stderr by default
    Default errorChannel stderr
    proc errorChannel { {filename ""} } {
⟦TAB⟧variable errorChannel
⟦TAB⟧variable ChannelsWeOpened

⟦TAB⟧# This is subtle and tricky.  See the comment above in
⟦TAB⟧# [outputChannel] for a detailed explanation.
⟦TAB⟧debug

⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $errorChannel
⟦TAB⟧}
⟦TAB⟧if {[info exists ChannelsWeOpened($errorChannel)]} {
⟦TAB⟧    close $errorChannel
⟦TAB⟧    unset ChannelsWeOpened($errorChannel)
⟦TAB⟧}
⟦TAB⟧switch -exact -- $filename {
⟦TAB⟧    stderr -
⟦TAB⟧    stdout {
⟦TAB⟧⟦TAB⟧set errorChannel $filename
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧set errorChannel [open $filename a]
⟦TAB⟧⟦TAB⟧set ChannelsWeOpened($errorChannel) 1

⟦TAB⟧⟦TAB⟧# If we created the file in [temporaryDirectory], then
⟦TAB⟧⟦TAB⟧# [cleanupTests] will delete it, unless we claim it was
⟦TAB⟧⟦TAB⟧# already there.
⟦TAB⟧⟦TAB⟧set outdir [normalizePath [file dirname \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [pwd] $filename]]]
⟦TAB⟧⟦TAB⟧if {[string equal $outdir [temporaryDirectory]]} {
⟦TAB⟧⟦TAB⟧    variable filesExisted
⟦TAB⟧⟦TAB⟧    FillFilesExisted
⟦TAB⟧⟦TAB⟧    set filename [file tail $filename]
⟦TAB⟧⟦TAB⟧    if {[lsearch -exact $filesExisted $filename] == -1} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧lappend filesExisted $filename
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $errorChannel
    }

##### Set up the configurable options
    #
    # The configurable options of the package
    variable Option; array set Option {}

    # Usage strings for those options
    variable Usage; array set Usage {}

    # Verification commands for those options
    variable Verify; array set Verify {}

    # Initialize the default values of the configurable options that are
    # historically associated with an exported variable.  If that variable
    # is already set, support compatibility by accepting its pre-set value.
    # Use [trace] to establish ongoing connection between the deprecated
    # exported variable and the modern option kept as a true internal var.
    # Also set up usage string and value testing for the option.
    proc Option {option value usage {verify AcceptAll} {varName {}}} {
⟦TAB⟧variable Option
⟦TAB⟧variable Verify
⟦TAB⟧variable Usage
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧set Usage($option) $usage
⟦TAB⟧set Verify($option) $verify
⟦TAB⟧if {[catch {$verify $value} msg]} {
⟦TAB⟧    return -code error $msg
⟦TAB⟧} else {
⟦TAB⟧    set Option($option) $msg
⟦TAB⟧}
⟦TAB⟧if {[string length $varName]} {
⟦TAB⟧    variable $varName
⟦TAB⟧    if {[info exists $varName]} {
⟦TAB⟧⟦TAB⟧if {[catch {$verify [set $varName]} msg]} {
⟦TAB⟧⟦TAB⟧    return -code error $msg
⟦TAB⟧⟦TAB⟧} else {
⟦TAB⟧⟦TAB⟧    set Option($option) $msg
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧unset $varName
⟦TAB⟧    }
⟦TAB⟧    namespace eval [namespace current] \
⟦TAB⟧    ⟦TAB⟧    [list upvar 0 Option($option) $varName]
⟦TAB⟧    # Workaround for Bug (now Feature Request) 572889.  Grrrr....
⟦TAB⟧    # Track all the variables tied to options
⟦TAB⟧    lappend OptionControlledVariables $varName
⟦TAB⟧    # Later, set auto-configure read traces on all
⟦TAB⟧    # of them, since a single trace on Option does not work.
⟦TAB⟧    proc $varName {{value {}}} [subst -nocommands {
⟦TAB⟧⟦TAB⟧if {[llength [info level 0]] == 2} {
⟦TAB⟧⟦TAB⟧    Configure $option [set value]
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧return [Configure $option]
⟦TAB⟧    }]
⟦TAB⟧}
    }

    proc MatchingOption {option} {
⟦TAB⟧variable Option
⟦TAB⟧set match [array names Option $option*]
⟦TAB⟧switch -- [llength $match] {
⟦TAB⟧    0 {
⟦TAB⟧⟦TAB⟧set sorted [lsort [array names Option]]
⟦TAB⟧⟦TAB⟧set values [join [lrange $sorted 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append values ", or [lindex $sorted end]"
⟦TAB⟧⟦TAB⟧return -code error "unknown option $option: should be\
⟦TAB⟧⟦TAB⟧⟦TAB⟧one of $values"
⟦TAB⟧    }
⟦TAB⟧    1 {
⟦TAB⟧⟦TAB⟧return [lindex $match 0]
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧# Exact match trumps ambiguity
⟦TAB⟧⟦TAB⟧if {[lsearch -exact $match $option] >= 0} {
⟦TAB⟧⟦TAB⟧    return $option
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧set values [join [lrange $match 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append values ", or [lindex $match end]"
⟦TAB⟧⟦TAB⟧return -code error "ambiguous option $option:\
⟦TAB⟧⟦TAB⟧⟦TAB⟧could match $values"
⟦TAB⟧    }
⟦TAB⟧}
    }

    proc EstablishAutoConfigureTraces {} {
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧foreach varName [concat $OptionControlledVariables Option] {
⟦TAB⟧    variable $varName
⟦TAB⟧    trace variable $varName r [namespace code {ProcessCmdLineArgs ;#}]
⟦TAB⟧}
    }

    proc RemoveAutoConfigureTraces {} {
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧foreach varName [concat $OptionControlledVariables Option] {
⟦TAB⟧    variable $varName
⟦TAB⟧    foreach pair [trace vinfo $varName] {
⟦TAB⟧⟦TAB⟧foreach {op cmd} $pair break
⟦TAB⟧⟦TAB⟧if {[string equal r $op]
⟦TAB⟧⟦TAB⟧⟦TAB⟧&& [string match *ProcessCmdLineArgs* $cmd]} {
⟦TAB⟧⟦TAB⟧    trace vdelete $varName $op $cmd
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧# Once the traces are removed, this can become a no-op
⟦TAB⟧proc RemoveAutoConfigureTraces {} {}
    }

    proc Configure args {
⟦TAB⟧variable Option
⟦TAB⟧variable Verify
⟦TAB⟧set n [llength $args]
⟦TAB⟧if {$n == 0} {
⟦TAB⟧    return [lsort [array names Option]]
⟦TAB⟧}
⟦TAB⟧if {$n == 1} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    return $Option($option)
⟦TAB⟧}
⟦TAB⟧while {[llength $args] > 1} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    if {[catch {$Verify($option) [lindex $args 1]} value]} {
⟦TAB⟧⟦TAB⟧return -code error "invalid $option\
⟦TAB⟧⟦TAB⟧⟦TAB⟧value \"[lindex $args 1]\": $value"
⟦TAB⟧    }
⟦TAB⟧    set Option($option) $value
⟦TAB⟧    set args [lrange $args 2 end]
⟦TAB⟧}
⟦TAB⟧if {[llength $args]} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    return -code error "missing value for option $option"
⟦TAB⟧}
    }
    proc configure args {
⟦TAB⟧RemoveAutoConfigureTraces
⟦TAB⟧set code [catch {eval Configure $args} msg]
⟦TAB⟧return -code $code $msg
    }
⟦SPACE⟧⟦SPACE⟧⟦SPACE⟧⟦SPACE⟧
    proc AcceptVerbose { level } {
⟦TAB⟧set level [AcceptList $level]
⟦TAB⟧if {[llength $level] == 1} {
⟦TAB⟧    if {![regexp {^(pass|body|skip|start|error)$} $level]} {
⟦TAB⟧⟦TAB⟧# translate single characters abbreviations to expanded list
⟦TAB⟧⟦TAB⟧set level [string map {p pass b body s skip t start e error} \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[split $level {}]]
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set valid [list]
⟦TAB⟧foreach v $level {
⟦TAB⟧    if {[regexp {^(pass|body|skip|start|error)$} $v]} {
⟦TAB⟧⟦TAB⟧lappend valid $v
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $valid
    }

    proc IsVerbose {level} {
⟦TAB⟧variable Option
⟦TAB⟧return [expr {[lsearch -exact $Option(-verbose) $level] != -1}]
    }

    # Default verbosity is to show bodies of failed tests
    Option -verbose {body error} {
⟦TAB⟧Takes any combination of the values 'p', 's', 'b', 't' and 'e'.
⟦TAB⟧Test suite will display all passed tests if 'p' is specified, all
⟦TAB⟧skipped tests if 's' is specified, the bodies of failed tests if
⟦TAB⟧'b' is specified, and when tests start if 't' is specified.
⟦TAB⟧ErrorInfo is displayed if 'e' is specified.
    } AcceptVerbose verbose

    # Match and skip patterns default to the empty list, except for
    # matchFiles, which defaults to all .test files in the
    # testsDirectory and matchDirectories, which defaults to all
    # directories.
    Option -match * {
⟦TAB⟧Run all tests within the specified files that match one of the
⟦TAB⟧list of glob patterns given.
    } AcceptList match

    Option -skip {} {
⟦TAB⟧Skip all tests within the specified tests (via -match) and files
⟦TAB⟧that match one of the list of glob patterns given.
    } AcceptList skip

    Option -file *.test {
⟦TAB⟧Run tests in all test files that match the glob pattern given.
    } AcceptPattern matchFiles

    # By default, skip files that appear to be SCCS lock files.
    Option -notfile l.*.test {
⟦TAB⟧Skip all test files that match the glob pattern given.
    } AcceptPattern skipFiles

    Option -relateddir * {
⟦TAB⟧Run tests in directories that match the glob pattern given.
    } AcceptPattern matchDirectories

    Option -asidefromdir {} {
⟦TAB⟧Skip tests in directories that match the glob pattern given.
    } AcceptPattern skipDirectories

    # By default, don't save core files
    Option -preservecore 0 {
⟦TAB⟧If 2, save any core files produced during testing in the directory
⟦TAB⟧specified by -tmpdir. If 1, notify the user if core files are
⟦TAB⟧created.
    } AcceptInteger preserveCore

    # debug output doesn't get printed by default; debug level 1 spits
    # up only the tests that were skipped because they didn't match or
    # were specifically skipped.  A debug level of 2 would spit up the
    # tcltest variables and flags provided; a debug level of 3 causes
    # some additional output regarding operations of the test harness.
    # The tcltest package currently implements only up to debug level 3.
    Option -debug 0 {
⟦TAB⟧Internal debug level⟦SPACE⟧
    } AcceptInteger debug

    proc SetSelectedConstraints args {
⟦TAB⟧variable Option
⟦TAB⟧foreach c $Option(-constraints) {
⟦TAB⟧    testConstraint $c 1
⟦TAB⟧}
    }
    Option -constraints {} {
⟦TAB⟧Do not skip the listed constraints listed in -constraints.
    } AcceptList
    trace variable Option(-constraints) w \
⟦TAB⟧    [namespace code {SetSelectedConstraints ;#}]

    # Don't run only the "-constraint" specified tests by default
    proc ClearUnselectedConstraints args {
⟦TAB⟧variable Option
⟦TAB⟧variable testConstraints
⟦TAB⟧if {!$Option(-limitconstraints)} {return}
⟦TAB⟧foreach c [array names testConstraints] {
⟦TAB⟧    if {[lsearch -exact $Option(-constraints) $c] == -1} {
⟦TAB⟧⟦TAB⟧testConstraint $c 0
⟦TAB⟧    }
⟦TAB⟧}
    }
    Option -limitconstraints false {
⟦TAB⟧whether to run only tests with the constraints
    } AcceptBoolean limitConstraints⟦SPACE⟧
    trace variable Option(-limitconstraints) w \
⟦TAB⟧    [namespace code {ClearUnselectedConstraints ;#}]

    # A test application has to know how to load the tested commands
    # into the interpreter.
    Option -load {} {
⟦TAB⟧Specifies the script to load the tested commands.
    } AcceptScript loadScript

    # Default is to run each test file in a separate process
    Option -singleproc 0 {
⟦TAB⟧whether to run all tests in one process
    } AcceptBoolean singleProcess⟦SPACE⟧

    proc AcceptTemporaryDirectory { directory } {
⟦TAB⟧set directory [AcceptAbsolutePath $directory]
⟦TAB⟧if {![file exists $directory]} {
⟦TAB⟧    file mkdir $directory
⟦TAB⟧}
⟦TAB⟧set directory [AcceptDirectory $directory]
⟦TAB⟧if {![file writable $directory]} {
⟦TAB⟧    if {[string equal [workingDirectory] $directed for future reference by [test]
proc tcltest::ConstraintInitializer {constraint {script ""}} {
    variable ConstraintInitializer
    DebugPuts 3 "entering ConstraintInitializer $constraint $script"
    if {[llength [info level 0]] == 2} {
⟦TAB⟧return $ConstraintInitializer($constraint)
    }
    # Check for boolean values
    if {![info complete $script]} {
⟦TAB⟧return -code error "ConstraintInitializeown constraints.
# See the tcltest man page for the list of built-in constraints defined
# in this procedure.
#
# Arguments:
#⟦TAB⟧none
#
# Results:
#⟦TAB⟧The testConstraints array is reset to have an index for each
#⟦TAB⟧built-in test constraint.
#
# Side Effects:
#       None.
#

proc tcltest::InitConstraints {} {
    variable ConstraintInitializer
    initConstraintsHook
    foreach constraint [array names ConstraintInitializer] {
⟦TAB⟧testConstraint $constraint
    }
}

proc tcltest::DefineConstraintInitializers {} {
    ConstraintInitializer singleTestInterp {singleProcess}

    # All the 'pc' constraints are here for backward compatibility and
    # are not documented.  They have been replaced with equivalent 'win'
    # constraints.

    ConstraintInitializer unixOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) unix}
    ConstraintInitializer macOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) macintosh}
    ConstraintInitializer pcOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) windows}
    ConstraintInitializer winOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) windows}

    ConstraintInitializer unix {testConstraint unixOnly}
    ConstraintInitializer mac {testConstraint macOnly}
    ConstraintInitializer pc {testConstraint pcOnly}
    ConstraintInitializer win {testConstraint winOnly}

    ConstraintInitializer unixOrPc \
⟦TAB⟧    {expr {[testConstraint unix] || [testConstraint pc]}}
    ConstraintInitializer macOrPc \
⟦TAB⟧    {expr {[testConstraint mac] || [testConstraint pc]}}
    ConstraintInitializer unixOrWin \
⟦TAB⟧    {expr {[testConstraint unix] || [testConstraint win Files."
⟦TAB⟧    set numTestFiles 0
⟦TAB⟧    if {[llength $failFiles] > 0} {
⟦TAB⟧⟦TAB⟧puts [outputChannel] \
⟦TAB⟧⟦TAB⟧⟦TAB⟧"Files with failing tests: $failFiles"
⟦TAB⟧⟦TAB⟧set failFiles {}
⟦TAB⟧    }
⟦TAB⟧}

⟦TAB⟧# if any tests were skipped, pri the names of test files in createdNewFiles, and reset
⟦TAB⟧# the array to be empty.

⟦TAB⟧set testFilesThatTurded [lsort [array names createdNewFiles]]
⟦TAB⟧if {[llength $testFilesThatTurded] > 0} {
⟦TAB⟧    puts [outputChannel] "Warning: files left behind:"
⟦TAB⟧    foreach testFile $testFilesThatTurded {
⟦TAB⟧⟦TAB⟧puts [outputChannel] \
⟦TAB⟧⟦TAB⟧⟦TAB⟧"\t$testFile:\t$createdNewFiles($testFile)"
⟦TAB⟧⟦TAB⟧unset createdNewFiles($testFile)
⟦TAB⟧    }
⟦TAB⟧}

⟦TAB⟧# reset filesMade, filesExisted, and numTests

⟦TAB⟧set filesMade {}
⟦TAB⟧foreach index [list "Total" "Passed" "Skipped" "Failed"] {
⟦TAB⟧    set numTests($index) 0
⟦TAB⟧}

⟦TAB⟧# exit only if running Tk in non-interactive mode
⟦TAB⟧# This should be changed to determine if an event
⟦TAB⟧# loop is running, which is the real issue.
⟦TAB⟧# Actually, this doesn't belong here at all.  A package
⟦TAB⟧# really has no business [exit]-ing an application.
⟦TAB⟧if {![catch {package present Tk}] && ![testConstraint interactive]} {
⟦TAB⟧    exit
⟦TAB⟧}
    } else {

⟦TAB⟧# if we're deferring stat-reporting until all files are sourced,
⟦TAB⟧# then add current file to failFile list if any tests in this
⟦TAB⟧# file failed

⟦TAB⟧if {$currentFailure \
⟦TAB⟧⟦TAB⟧&& ([lsearch -exact $failFiles $testFileName] == -1)} {
⟦TAB⟧    lappend failFiles $testFileName
⟦TAB⟧}
⟦TAB⟧set currentFailure false

⟦TAB⟧# restore the environment to the state it was in before this package
⟦TAB⟧# was loaded

⟦TAB⟧set newEnv {}
⟦TAB⟧set changedEnv {}
⟦TAB⟧set removedEnv {}
⟦TAB⟧foreach index [array names ::env] {
⟦TAB⟧    if {![info exists originalEnv($index)]} {
⟦TAB⟧⟦TAB⟧lappend newEnv $index
⟦TAB⟧⟦TAB⟧unset ::env($index)
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧if {$::env($index) != $originalEnv($index)} {
⟦TAB⟧⟦TAB⟧    lappend changedEnv $index
⟦TAB⟧⟦TAB⟧    set ::env($index) $ {[llength $newEnv] > 0} {
⟦TAB⟧    puts [outputChannel] \
⟦TAB⟧⟦TAB⟧    "env array elements created:\t$newEnv"
⟦TAB⟧}
⟦TAB⟧if {[llength $changedEnv] > 0} {
⟦TAB⟧    puts [outputChannel] \
⟦TAB⟧⟦TAB⟧    "env array elements changed:\ {
    variable previousLocale
    if {[info commands testlocale] != ""} {
⟦TAB⟧testlocale ctype $previousLocale
    }
    return
}

# tcltest::threadReap --
#
#⟦TAB⟧Kill all threads except for the main thread.
#⟦TAB⟧Do nothing if testthread is not defined.
#
# Arguments:
#⟦TAB⟧none.
#
# Results:
#⟦TAB⟧Returns the number of existing threads.
#
# Side Effects:
#       none.
#

proc tcltest::threadReap {} {
    if {[in= [mainThread]} {
⟦TAB⟧⟦TAB⟧    catch {
⟦TAB⟧⟦TAB⟧⟦TAB⟧testthread send -async $tid {testthread exit}
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧    ## Enter a bit a sleep to give the threads enough breathing
⟦TAB⟧    ## room to kill themselves off, otherwise the end up with a
⟦TAB⟧    ## massive queue of repeated events
⟦TAB⟧    after 1
⟦TAB⟧}
⟦TAB⟧testthread errorproc ThreadError
⟦TAB⟧return [llength [testthread names]]
    } elseif {[info commands thread::id] != {}} {
⟦TAB⟧
⟦TAB⟧# Thread extension

⟦TAB⟧thread::errorproc ThreadNullError
⟦TAB⟧while {[llength [thread::names]] > 1} {
⟦TAB⟧    foreach tid [thread::names] {
⟦TAB⟧⟦TAB⟧if {$tid != [mainThread]} {
⟦TAB⟧⟦TAB⟧    catch {thread::send -async $tid {thread::exit}}
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧    ## Enter a bit a sleep to give the threads enough breathing
⟦TAB⟧    ## room to kill themselves off, otherwise the end up with a
⟦TAB⟧    ## massive queue of repeated events
⟦TAB⟧    after 1
⟦TAB⟧}
⟦TAB⟧thread::errorproc ThreadError
⟦TAB⟧return [llength [thread::names]]
    } else {
⟦TAB⟧return 1
    }
    return 0
}

# Initialize the constraints and set up command line arguments
namespace eval tcltest {
    # Define initializers for all the built-in contraint definitions
    DefineConstraintInitializers

    # Set up the constraints in the testConstraints array to be lazily
    # initialized by a registered initializer, or by "false" if no
    # initializer is registered.
    trace variable testConstraints r [namespace code SafeFetch]

    # Only initialize constraints at package load time if an
    # [initConstraintsHook] has been pre-defined.  This is only
    # for compatibility support.  The modern way to add a custom
    # test constraint is to just call the [testCo⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 0, result:

```text-visible
child-provider {8.4.6 8.4 ::} module-code 1 module-result {invalid command name "pwd"} require-code 1 require-result {can't find package tcltest}⟦EOF⟧
```

- real-harness-explicit-source-child, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- real-harness-explicit-source-child: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 6

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
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

    # tests that use threads need to know which is the main thread
    Default mainThread 1
    variable mainThread
    if {[info commands thread::id] != {}} {
⟦TAB⟧set mainThread [thread::id]
    } elseif {[info commands testthread] != {}} {
⟦TAB⟧set mainThread [testthread id]
    }

    # Set workingDirectory to [pwd]. The default output directory for
    # Tcl tests is the working directory.  Whenever this value changes
    # change to that directory.
    variable workingDirectory
    trace variable workingDirectory w \
⟦TAB⟧    [namespace code {cd $workingDirectory ;#}]

    Default workingDirectory [pwd] AcceptAbsolutePath
    proc workingDirectory { {dir ""} } {
⟦TAB⟧variable workingDirectory
⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $workingDirectory
⟦TAB⟧}
⟦TAB⟧set workingDirectory [AcceptAbsolutePath $dir]
    }

    # Set the location of the execuatble
    Default tcltest [info nameofexecutable]
    trace variable tcltest w [namespace code {testConstraint stdio \
⟦TAB⟧    [eval [ConstraintInitializer stdio]] ;#}]

    # save the platform information so it can be restored later
    Default originalTclPlatform [array get ::tcl_platform]

    # If a core file exists, save its modification time.
    if {[file exists [file join [workingDirectory] core]]} {
⟦TAB⟧Default coreModTime \
⟦TAB⟧⟦TAB⟧[file mtime [file join [workingDirectory] core]]
    }

    # stdout and stderr buffers for use when we want to store them
    Default outData {}
    Default errData {}

    # keep track of test level for nested test commands
    variable testLevel 0

    # the variables and procs that existed when saveState was called are
    # stored in a variable of the same name
    Default saveState {}

    # Internationalization support -- used in [SetIso8859_1_Locale] and
    # [RestoreLocale]. Those commands are used in cmdIL.test.

    if {![info exists [namespace current]::isoLocale]} {
⟦TAB⟧variable isoLocale fr
⟦TAB⟧switch -- $::tcl_platform(platform) {
⟦TAB⟧    "unix" {

⟦TAB⟧⟦TAB⟧# Try some 'known' values for some platforms:

⟦TAB⟧⟦TAB⟧switch -exact -- $::tcl_platform(os) {
⟦TAB⟧⟦TAB⟧    "FreeBSD" {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr_FR.ISO_8859-1
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    HP-UX {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr_FR.iso88591
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    Linux -
⟦TAB⟧⟦TAB⟧    IRIX {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    default {

⟦TAB⟧⟦TAB⟧⟦TAB⟧# Works on SunOS 4 and Solaris, and maybe
⟦TAB⟧⟦TAB⟧⟦TAB⟧# others...  Define it to something else on your
⟦TAB⟧⟦TAB⟧⟦TAB⟧# system if you want to test those.

⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale iso_8859_1
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧    "windows" {
⟦TAB⟧⟦TAB⟧set isoLocale French
⟦TAB⟧    }
⟦TAB⟧}
    }

    variable ChannelsWeOpened; array set ChannelsWeOpened {}
    # output goes to stdout by default
    Default outputChannel stdout
    proc outputChannel { {filename ""} } {
⟦TAB⟧variable outputChannel
⟦TAB⟧variable ChannelsWeOpened

⟦TAB⟧# This is very subtle and tricky, so let me try to explain.
⟦TAB⟧# (Hopefully this longer comment will be clear when I come
⟦TAB⟧# back in a few months, unlike its predecessor :) )
⟦TAB⟧#⟦SPACE⟧
⟦TAB⟧# The [outputChannel] command (and underlying variable) have to
⟦TAB⟧# be kept in sync with the [configure -outfile] configuration
⟦TAB⟧# option ( and underlying variable Option(-outfile) ).  This is
⟦TAB⟧# accomplished with a write trace on Option(-outfile) that will
⟦TAB⟧# update [outputChannel] whenver a new value is written.  That
⟦TAB⟧# much is easy.
⟦TAB⟧#
⟦TAB⟧# The trick is that in order to maintain compatibility with
⟦TAB⟧# version 1 of tcltest, we must allow every configuration option
⟦TAB⟧# to get its inital value from command line arguments.  This is
⟦TAB⟧# accomplished by setting initial read traces on all the
⟦TAB⟧# configuration options to parse the command line option the first
⟦TAB⟧# time they are read.  These traces are cancelled whenever the
⟦TAB⟧# program itself calls [configure].
⟦TAB⟧#⟦SPACE⟧
⟦TAB⟧# OK, then so to support tcltest 1 compatibility, it seems we want
⟦TAB⟧# to get the return from [outputFile] to trigger the read traces,
⟦TAB⟧# just in case.
⟦TAB⟧#
⟦TAB⟧# BUT!  A little known feature of Tcl variable traces is that⟦SPACE⟧
⟦TAB⟧# traces are disabled during the handling of other traces.  So,
⟦TAB⟧# if we trigger read traces on Option(-outfile) and that triggers
⟦TAB⟧# command line parsing which turns around and sets an initial
⟦TAB⟧# value for Option(-outfile) -- <whew!> -- the write trace that
⟦TAB⟧# would keep [outputChannel] in sync with that new initial value
⟦TAB⟧# would not fire!
⟦TAB⟧#
⟦TAB⟧# SO, finally, as a workaround, instead of triggering read traces
⟦TAB⟧# by invoking [outputFile], we instead trigger the same set of
⟦TAB⟧# read traces by invoking [debug].  Any command that reads a
⟦TAB⟧# configuration option would do.  [debug] is just a handy one.
⟦TAB⟧# The end result is that we support tcltest 1 compatibility and
⟦TAB⟧# keep outputChannel and -outfile in sync in all cases.
⟦TAB⟧debug

⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $outputChannel
⟦TAB⟧}
⟦TAB⟧if {[info exists ChannelsWeOpened($outputChannel)]} {
⟦TAB⟧    close $outputChannel
⟦TAB⟧    unset ChannelsWeOpened($outputChannel)
⟦TAB⟧}
⟦TAB⟧switch -exact -- $filename {
⟦TAB⟧    stderr -
⟦TAB⟧    stdout {
⟦TAB⟧⟦TAB⟧set outputChannel $filename
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧set outputChannel [open $filename a]
⟦TAB⟧⟦TAB⟧set ChannelsWeOpened($outputChannel) 1

⟦TAB⟧⟦TAB⟧# If we created the file in [temporaryDirectory], then
⟦TAB⟧⟦TAB⟧# [cleanupTests] will delete it, unless we claim it was
⟦TAB⟧⟦TAB⟧# already there.
⟦TAB⟧⟦TAB⟧set outdir [normalizePath [file dirname \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [pwd] $filename]]]
⟦TAB⟧⟦TAB⟧if {[string equal $outdir [temporaryDirectory]]} {
⟦TAB⟧⟦TAB⟧    variable filesExisted
⟦TAB⟧⟦TAB⟧    FillFilesExisted
⟦TAB⟧⟦TAB⟧    set filename [file tail $filename]
⟦TAB⟧⟦TAB⟧    if {[lsearch -exact $filesExisted $filename] == -1} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧lappend filesExisted $filename
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $outputChannel
    }

    # errors go to stderr by default
    Default errorChannel stderr
    proc errorChannel { {filename ""} } {
⟦TAB⟧variable errorChannel
⟦TAB⟧variable ChannelsWeOpened

⟦TAB⟧# This is subtle and tricky.  See the comment above in
⟦TAB⟧# [outputChannel] for a detailed explanation.
⟦TAB⟧debug

⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $errorChannel
⟦TAB⟧}
⟦TAB⟧if {[info exists ChannelsWeOpened($errorChannel)]} {
⟦TAB⟧    close $errorChannel
⟦TAB⟧    unset ChannelsWeOpened($errorChannel)
⟦TAB⟧}
⟦TAB⟧switch -exact -- $filename {
⟦TAB⟧    stderr -
⟦TAB⟧    stdout {
⟦TAB⟧⟦TAB⟧set errorChannel $filename
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧set errorChannel [open $filename a]
⟦TAB⟧⟦TAB⟧set ChannelsWeOpened($errorChannel) 1

⟦TAB⟧⟦TAB⟧# If we created the file in [temporaryDirectory], then
⟦TAB⟧⟦TAB⟧# [cleanupTests] will delete it, unless we claim it was
⟦TAB⟧⟦TAB⟧# already there.
⟦TAB⟧⟦TAB⟧set outdir [normalizePath [file dirname \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [pwd] $filename]]]
⟦TAB⟧⟦TAB⟧if {[string equal $outdir [temporaryDirectory]]} {
⟦TAB⟧⟦TAB⟧    variable filesExisted
⟦TAB⟧⟦TAB⟧    FillFilesExisted
⟦TAB⟧⟦TAB⟧    set filename [file tail $filename]
⟦TAB⟧⟦TAB⟧    if {[lsearch -exact $filesExisted $filename] == -1} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧lappend filesExisted $filename
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $errorChannel
    }

##### Set up the configurable options
    #
    # The configurable options of the package
    variable Option; array set Option {}

    # Usage strings for those options
    variable Usage; array set Usage {}

    # Verification commands for those options
    variable Verify; array set Verify {}

    # Initialize the default values of the configurable options that are
    # historically associated with an exported variable.  If that variable
    # is already set, support compatibility by accepting its pre-set value.
    # Use [trace] to establish ongoing connection between the deprecated
    # exported variable and the modern option kept as a true internal var.
    # Also set up usage string and value testing for the option.
    proc Option {option value usage {verify AcceptAll} {varName {}}} {
⟦TAB⟧variable Option
⟦TAB⟧variable Verify
⟦TAB⟧variable Usage
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧set Usage($option) $usage
⟦TAB⟧set Verify($option) $verify
⟦TAB⟧if {[catch {$verify $value} msg]} {
⟦TAB⟧    return -code error $msg
⟦TAB⟧} else {
⟦TAB⟧    set Option($option) $msg
⟦TAB⟧}
⟦TAB⟧if {[string length $varName]} {
⟦TAB⟧    variable $varName
⟦TAB⟧    if {[info exists $varName]} {
⟦TAB⟧⟦TAB⟧if {[catch {$verify [set $varName]} msg]} {
⟦TAB⟧⟦TAB⟧    return -code error $msg
⟦TAB⟧⟦TAB⟧} else {
⟦TAB⟧⟦TAB⟧    set Option($option) $msg
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧unset $varName
⟦TAB⟧    }
⟦TAB⟧    namespace eval [namespace current] \
⟦TAB⟧    ⟦TAB⟧    [list upvar 0 Option($option) $varName]
⟦TAB⟧    # Workaround for Bug (now Feature Request) 572889.  Grrrr....
⟦TAB⟧    # Track all the variables tied to options
⟦TAB⟧    lappend OptionControlledVariables $varName
⟦TAB⟧    # Later, set auto-configure read traces on all
⟦TAB⟧    # of them, since a single trace on Option does not work.
⟦TAB⟧    proc $varName {{value {}}} [subst -nocommands {
⟦TAB⟧⟦TAB⟧if {[llength [info level 0]] == 2} {
⟦TAB⟧⟦TAB⟧    Configure $option [set value]
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧return [Configure $option]
⟦TAB⟧    }]
⟦TAB⟧}
    }

    proc MatchingOption {option} {
⟦TAB⟧variable Option
⟦TAB⟧set match [array names Option $option*]
⟦TAB⟧switch -- [llength $match] {
⟦TAB⟧    0 {
⟦TAB⟧⟦TAB⟧set sorted [lsort [array names Option]]
⟦TAB⟧⟦TAB⟧set values [join [lrange $sorted 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append values ", or [lindex $sorted end]"
⟦TAB⟧⟦TAB⟧return -code error "unknown option $option: should be\
⟦TAB⟧⟦TAB⟧⟦TAB⟧one of $values"
⟦TAB⟧    }
⟦TAB⟧    1 {
⟦TAB⟧⟦TAB⟧return [lindex $match 0]
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧# Exact match trumps ambiguity
⟦TAB⟧⟦TAB⟧if {[lsearch -exact $match $option] >= 0} {
⟦TAB⟧⟦TAB⟧    return $option
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧set values [join [lrange $match 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append values ", or [lindex $match end]"
⟦TAB⟧⟦TAB⟧return -code error "ambiguous option $option:\
⟦TAB⟧⟦TAB⟧⟦TAB⟧could match $values"
⟦TAB⟧    }
⟦TAB⟧}
    }

    proc EstablishAutoConfigureTraces {} {
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧foreach varName [concat $OptionControlledVariables Option] {
⟦TAB⟧    variable $varName
⟦TAB⟧    trace variable $varName r [namespace code {ProcessCmdLineArgs ;#}]
⟦TAB⟧}
    }

    proc RemoveAutoConfigureTraces {} {
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧foreach varName [concat $OptionControlledVariables Option] {
⟦TAB⟧    variable $varName
⟦TAB⟧    foreach pair [trace vinfo $varName] {
⟦TAB⟧⟦TAB⟧foreach {op cmd} $pair break
⟦TAB⟧⟦TAB⟧if {[string equal r $op]
⟦TAB⟧⟦TAB⟧⟦TAB⟧&& [string match *ProcessCmdLineArgs* $cmd]} {
⟦TAB⟧⟦TAB⟧    trace vdelete $varName $op $cmd
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧# Once the traces are removed, this can become a no-op
⟦TAB⟧proc RemoveAutoConfigureTraces {} {}
    }

    proc Configure args {
⟦TAB⟧variable Option
⟦TAB⟧variable Verify
⟦TAB⟧set n [llength $args]
⟦TAB⟧if {$n == 0} {
⟦TAB⟧    return [lsort [array names Option]]
⟦TAB⟧}
⟦TAB⟧if {$n == 1} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    return $Option($option)
⟦TAB⟧}
⟦TAB⟧while {[llength $args] > 1} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    if {[catch {$Verify($option) [lindex $args 1]} value]} {
⟦TAB⟧⟦TAB⟧return -code error "invalid $option\
⟦TAB⟧⟦TAB⟧⟦TAB⟧value \"[lindex $args 1]\": $value"
⟦TAB⟧    }
⟦TAB⟧    set Option($option) $value
⟦TAB⟧    set args [lrange $args 2 end]
⟦TAB⟧}
⟦TAB⟧if {[llength $args]} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    return -code error "missing value for option $option"
⟦TAB⟧}
    }
    proc configure args {
⟦TAB⟧RemoveAutoConfigureTraces
⟦TAB⟧set code [catch {eval Configure $args} msg]
⟦TAB⟧return -code $code $msg
    }
⟦SPACE⟧⟦SPACE⟧⟦SPACE⟧⟦SPACE⟧
    proc AcceptVerbose { level } {
⟦TAB⟧set level [AcceptList $level]
⟦TAB⟧if {[llength $level] == 1} {
⟦TAB⟧    if {![regexp {^(pass|body|skip|start|error)$} $level]} {
⟦TAB⟧⟦TAB⟧# translate single characters abbreviations to expanded list
⟦TAB⟧⟦TAB⟧set level [string map {p pass b body s skip t start e error} \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[split $level {}]]
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set valid [list]
⟦TAB⟧foreach v $level {
⟦TAB⟧    if {[regexp {^(pass|body|skip|start|error)$} $v]} {
⟦TAB⟧⟦TAB⟧lappend valid $v
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $valid
    }

    proc IsVerbose {level} {
⟦TAB⟧variable Option
⟦TAB⟧return [expr {[lsearch -exact $Option(-verbose) $level] != -1}]
    }

    # Default verbosity is to show bodies of failed tests
    Option -verbose {body error} {
⟦TAB⟧Takes any combination of the values 'p', 's', 'b', 't' and 'e'.
⟦TAB⟧Test suite will display all passed tests if 'p' is specified, all
⟦TAB⟧skipped tests if 's' is specified, the bodies of failed tests if
⟦TAB⟧'b' is specified, and when tests start if 't' is specified.
⟦TAB⟧ErrorInfo is displayed if 'e' is specified.
    } AcceptVerbose verbose

    # Match and skip patterns default to the empty list, except for
    # matchFiles, which defaults to all .test files in the
    # testsDirectory and matchDirectories, which defaults to all
    # directories.
    Option -match * {
⟦TAB⟧Run all tests within the specified files that match one of the
⟦TAB⟧list of glob patterns given.
    } AcceptList match

    Option -skip {} {
⟦TAB⟧Skip all tests within the specified tests (via -match) and files
⟦TAB⟧that match one of the list of glob patterns given.
    } AcceptList skip

    Option -file *.test {
⟦TAB⟧Run tests in all test files that match the glob pattern given.
    } AcceptPattern matchFiles

    # By default, skip files that appear to be SCCS lock files.
    Option -notfile l.*.test {
⟦TAB⟧Skip all test files that match the glob pattern given.
    } AcceptPattern skipFiles

    Option -relateddir * {
⟦TAB⟧Run tests in directories that match the glob pattern given.
    } AcceptPattern matchDirectories

    Option -asidefromdir {} {
⟦TAB⟧Skip tests in directories that match the glob pattern given.
    } AcceptPattern skipDirectories

    # By default, don't save core files
    Option -preservecore 0 {
⟦TAB⟧If 2, save any core files produced during testing in the directory
⟦TAB⟧specified by -tmpdir. If 1, notify the user if core files are
⟦TAB⟧created.
    } AcceptInteger preserveCore

    # debug output doesn't get printed by default; debug level 1 spits
    # up only the tests that were skipped because they didn't match or
    # were specifically skipped.  A debug level of 2 would spit up the
    # tcltest variables and flags provided; a debug level of 3 causes
    # some additional output regarding operations of the test harness.
    # The tcltest package currently implements only up to debug level 3.
    Option -debug 0 {
⟦TAB⟧Internal debug level⟦SPACE⟧
    } AcceptInteger debug

    proc SetSelectedConstraints args {
⟦TAB⟧variable Option
⟦TAB⟧foreach c $Option(-constraints) {
⟦TAB⟧    testConstraint $c 1
⟦TAB⟧}
    }
    Option -constraints {} {
⟦TAB⟧Do not skip the listed constraints listed in -constraints.
    } AcceptList
    trace variable Option(-constraints) w \
⟦TAB⟧    [namespace code {SetSelectedConstraints ;#}]

    # Don't run only the "-constraint" specified tests by default
    proc ClearUnselectedConstraints args {
⟦TAB⟧variable Option
⟦TAB⟧variable testConstraints
⟦TAB⟧if {!$Option(-limitconstraints)} {return}
⟦TAB⟧foreach c [array names testConstraints] {
⟦TAB⟧    if {[lsearch -exact $Option(-constraints) $c] == -1} {
⟦TAB⟧⟦TAB⟧testConstraint $c 0
⟦TAB⟧    }
⟦TAB⟧}
    }
    Option -limitconstraints false {
⟦TAB⟧whether to run only tests with the constraints
    } AcceptBoolean limitConstraints⟦SPACE⟧
    trace variable Option(-limitconstraints) w \
⟦TAB⟧    [namespace code {ClearUnselectedConstraints ;#}]

    # A test application has to know how to load the tested commands
    # into the interpreter.
    Option -load {} {
⟦TAB⟧Specifies the script to load the tested commands.
    } AcceptScript loadScript

    # Default is to run each test file in a separate process
    Option -singleproc 0 {
⟦TAB⟧whether to run all tests in one process
    } AcceptBoolean singleProcess⟦SPACE⟧

    proc AcceptTemporaryDirectory { directory } {
⟦TAB⟧set directory [AcceptAbsolutePath $directory]
⟦TAB⟧if {![file exists $directory]} {
⟦TAB⟧    file mkdir $directory
⟦TAB⟧}
⟦TAB⟧set directory [AcceptDirectory $directory]
⟦TAB⟧if {![file writable $directory]} {
⟦TAB⟧    if {[string equal [workingDirectory] $directws}
    ConstraintInitializer winOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) windows}

    ConstraintInitializer unix {testConstraint unixOnly}
    ConstraintInitializer mac {testConstraint maraint pc]}}
    ConstraintInitializer macOrPc \
⟦TAB⟧    {expr {[testConstraint mac] || [testConstraint pc]}}
    ConstraintInitializer unixOrWin \
⟦TAB⟧    {expr {[testConstraint unix] || [testConstraint win]}}
    ConstraintInitializer macOrWin \
⟦TAB⟧    {expr {[testConstraint mac] || [testConstraint win]}}
    ConstraintInitializer macOrUnix \
⟦TAB⟧    {expr {[testConstraint mac] || [testConstraint unix]}}

    ConstraintInitializer nt {string equal $::tcl_platform(os) "Windows NT"}
    ConstraintInitializer 95 {string equal $::tcl_platform(os) "Windows 95"}
    ConstraintInitializer 98 {string equal $::tand we haven't gotten around to
    # fixing the underlying problem.

    ConstraintInitializer tempNotPc {expr {![testConstraint pc]}}
    ConstraintInitializer tempNotWin {expr {![testConstraint win]}}
    ConstraintInitializer tempNotMac {expr {![testConstraint mac]}}
    ConstraintInitializer tempNotUnix {expr {![testConstraint unix]}}

    # The following Constraints switches are used to mark tests that
    # crash on certain platforms, so that they can be reactivated again
    # when the underlying problem is fixed.

    ConstraintInitializer pcCrash {expr {![testConstraint pc]}}
    ConSkip empty tests

    ConstraintInitializer emptyTest {format 0}

    # By default, tests that expose known bugs are skipped.

    ConstraintInitializer knownBug {format 0}

    # By default, non-portable tests are skipped.

    ConstraintInitializer nonPortable {format 0}

    # Some tests require user interaction.

    ConstraintInitializer userInteraction {format 0}

    # Some tests must be skipped if the interpreter is not in
    # interactive mode

    ConstraintInitializer interactive \
⟦TAB⟧    {expr {[info exists ::tcl_interactive] && $::tcl_interactive}}

    # Some tests can only be runel [[namespace parent]::errorChannel]]
⟦TAB⟧⟦TAB⟧|| [string equal $channel stderr]} {
⟦TAB⟧    append errData [lindex $args end]$newline
⟦TAB⟧    return
⟦TAB⟧}
    }

    # If we haven't returned by now, we don't know honot to ignore output
#⟦TAB⟧⟦TAB⟧⟦TAB⟧   sent to stdout & stderr
#
# Results:
#⟦TAB⟧result from running the script
#
# Side effects:
#⟦TAB⟧Empties the contents of outData and errData before running a
#⟦TAB⟧test if ignoreOutput is set to 0.

proc tcltest::Eval {script {ignoreOutput 1}} {
    variable outData
    variable errData
    DebugPuts 3 "[lindex [info level 0] 0] called"
    if {!$ignoreOutput} {
⟦TAB⟧set outData {}
⟦TAB⟧set errData {}
⟦TAB⟧rename ::puts [namespace current]::Replace::Puts
⟦TAB⟧namespace eval :: \
⟦TAB⟧⟦TAB⟧[list namespace import [namespace origin Replace::puts]]
⟦TAB⟧namespace import Replace::puts
    }
    set result [uplevel 1 $script]
    if {!$ignoreOutput} {
⟦TAB⟧namespace forget puts
⟦TAB⟧namespace eval :: namespace forget puts
⟦TAB⟧rename [namespace current]::Replace::Puts ::puts
    }
    return $result
}

# tcltest::CompareStrings --
#
#⟦TAB⟧compares the expected answer to the actual answer, depending on
#⟦TAB⟧the mode provided.  Mode determines whether a regexp, exact,
#⟦TAB⟧glob or custom comparison is done.
#
# Arguments:
#⟦TAB⟧actual - string containing the actual result
#       expected - pattern to be matched against
#       mode - type of comparison to be done
#
# Results:
#⟦TAB⟧result of the match
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::CompareStrings {actual expected mode} {
    variable CustomMatch
    if {![info exists CustomMatch($mode)]} {
        return -code error "No matching command registered for `-match $mode'"
    }
    set match [namespace eval :: $CustomMatch($mode) [list $expected $actual]]
    if {[catch {expr {$match && $match}} result]} {
⟦TAB⟧return -code error "Invalid result from `-match $mode' command: $result"
    }
    return $match
}

# tcltest::customMatch --
#
#⟦TAB⟧registers a command to be called when a particular type of
#⟦TAB⟧matching is required.
#
# Arguments:
#⟦TAB⟧nickname - Keyword for the ty58.html
#
# Results:
#     a list containing the result of the substitution
#
# Exceptions:
#     An error may occur if the list containing unbalanced quote or
#     unknown variable.
#
# Side Effects # Look for the next word containing a quote: " { }
        if {[regexp -indices {[^ \t\n]*[\"\{\}]+[^ \t\n]*} \
⟦TAB⟧⟦TAB⟧$argList all]} {
            # Get the text leading up to this word, but not includinargList \
                    [lindex $all 0] [lindex $all 1]]

            # Remove all text up to and including the word from the
            # argList.
            set argList [string range $argList \
                    [expr {[lindex $all 1] + 1}] end]
        } else {
            # Take everything up to the end of the argList.
            set text $argList
            set word {}
               # add the text and the current word to this token.
            append token $text $word
        } else {
            # Add the text to the result.  There is no need to parse
            # the text because it couldn't be a part of any multi-word
            # token.  Then start a new multi-word token with the word
            # because we need to pass this token to the Tcl parser to
            # check for balancing quotes
            append result $text
            set token $word
        }

        if { [catch {llength $token} length] == 0 && $length == 1} {
            # The token is a valid list so add it to the result.
            # lappend result [string trim $token]
            append result \{$token\}
            set token {}
        }
    }

    # If the last token has not been added to the list then there
    # is a problem.
    if { [string length $token] } {
        error "incomplete token \"$token\""
    }

    return $result
}

# tcltest::test --
#
# This procedure runs a test and prints an error message if the test
# fails.  If verbose has been set, it also prints a message even if the
# test succeeds.  The test will be skipped if it doesn't match the
# match varuts [outputChannel] ""

⟦TAB⟧# print number test files sourced
⟦TAB⟧# print names of files that ran tests which failed

⟦TAB⟧if {$calledFromAllFile} {
⟦TAB⟧    puts [outputChannel] \
⟦TAB⟧⟦TAB⟧    "Sourced $numTestFiles Test Files."
⟦TAB⟧    set numTestFiles 0
⟦TAB⟧    if {[llength $failFiles] > 0} {
⟦TAB⟧⟦TAB⟧puts [outputChannel] \
⟦TAB⟧⟦TAB⟧⟦TAB⟧"Files with failing tests: $failFiles"
⟦TAB⟧⟦TAB⟧set failFiles {}
⟦TAB⟧    }
⟦TAB⟧}

⟦TAB⟧# if any tests were skipped, prir each constraint:"
⟦TAB⟧    foreach constraint [lsort $constraintList] {
⟦TAB⟧⟦TAB⟧puts [outputChannel] \
⟦TAB⟧⟦TAB⟧⟦TAB⟧"\t$skippedBecause($constraint)\t$constraint"
⟦TAB⟧⟦TAB⟧unset skippedBecause($constraint)
⟦TAB⟧    }
⟦TAB⟧}

⟦TAB⟧# report the names of test files in createdNewFiles, and reset
⟦TAB⟧# the array to be empty.

⟦TAB⟧set testFilesThatTurded [lsort [array names createdNewFiles]]
⟦TAB⟧if {[llength $testFilesThatTurded] > 0} {
⟦TAB⟧    puts [outputChannel] "Warning: files left behind:"
⟦TAB⟧    foreach testFile $testFilesThatTurded {
⟦TAB⟧⟦TAB⟧puts [outputChannel] \
⟦TAB⟧⟦TAB⟧⟦TAB⟧"\t$testFile:\t$createdNewFiles($testFile)"
⟦TAB⟧⟦TAB⟧unset createdNewFiles($testFile)
⟦TAB⟧    }
⟦TAB⟧}

⟦TAB⟧# reset filesMade, filesExisted, and numTests

⟦TAB⟧set filesMade {}
⟦TAB⟧foreach index [list "Total" "Passed" "Skipped" "Failed"] {
⟦TAB⟧    set numTests($index) 0
⟦TAB⟧}

⟦TAB⟧# exit only if running Tk in non-interactive mode
⟦TAB⟧# This should be changed to determine if an event
⟦TAB⟧# loop is running, which is the real issue.
⟦TAB⟧# Actually, this doesn't belong here at all.  A package
⟦TAB⟧# really has no business [exit]-ing an application.
⟦TAB⟧if {![catch {package present Tk}] && ![testConstraint interactive]} {
⟦TAB⟧    exit
⟦TAB⟧}
    } else {

⟦TAB⟧# if we're deferring stat-reporting until all files are sourced,
⟦TAB⟧# then add current file to failFile list if any tests in this
⟦TAB⟧# file failed

⟦TAB⟧if {$currentFailure \
⟦TAB⟧⟦TAB⟧&& ([lsearch -exact $failFiles $testFileName] == -1)} {
⟦TAB⟧    lappend failFiles $testFileName
⟦TAB⟧}
⟦TAB⟧set currentFanfo exists originalEnv($index)]} {
⟦TAB⟧⟦TAB⟧lappend newEnv $index
⟦TAB⟧⟦TAB⟧unset ::env($index)
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧if {$::env($index) != $originalEnv($index)} {
⟦TAB⟧⟦TAB⟧    lappend changedEnv $index
⟦TAB⟧⟦TAB⟧    set ::env($index) $s:
#       None.

proc tcltest::GetMatchingDirectories {rootdir} {

    # Determine the skip list first, to avoid [glob]-ing over subdirectories
    # we're going to throw away anyway.  Be sure we skip the $rootdir if it
    # comes up to avoid infinite loops.
    set skipDirs [list $rootdir]
    foreach pattern [skipDirectories] {
⟦TAB⟧set skipDirs [concat $skipDirs [glob -directory $rootdir -types dreach path [glob -directory $rootdir -types d -nocomplain -- \
⟦TAB⟧⟦TAB⟧$pattern] {
⟦TAB⟧    if {[lsearch -exact $skipDirs $path] == -1} {
⟦TAB⟧⟦TAB⟧set matchDirs [concat $matchDirs [GetMatchingDirectories $path]]
⟦TAB⟧⟦TAB⟧if {[file exists [file join $path all.tcl]]} {
⟦TAB⟧⟦TAB⟧    lappend matchDirs $path
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
    }

    if {[llength $matchDirs] == 0} {
⟦TAB⟧DebugPuts 1 "No test directories remain after applying match\
⟦TAB⟧⟦TAB⟧and skip patterns!"
    }
    return $matchDirs
}

# tcltest::runAllTests --
#
#⟦TAB⟧prints output and sources test files according to the match and
#⟦TAB⟧skip patterns provided.  after sourcing test files, it giable testSingleFile
    variable numTestFiles
    variable numTests
    variable failFiles

    FillFilesExisted
    if {[llength [info level 0]] == 1} {
⟦TAB⟧set shell [interpreter]
    }

    set testSingleFile false

    puts [outputChannel] "Tests running in interp:  $shell"
    puts [outputChannel] "Tests located in:  [testsDirectory]"
    puts [outputChannel] "Tests running in:  [workingDirectory]"
    puts [outputChannel] "Temporary files stored in\
⟦TAB⟧    [temporaryDirectory]"

    # [file system] first available in Tcl 8.4
    if {![catch {file system [testsDirectory]} result]
⟦TAB⟧    && ![string equal native [lindex $result 0]]} {
⟦TAB⟧# If we aren't running in the native filesystem, then we must
⟦TAB⟧# run the tests in a single process (via 'source'), because
⟦TAB⟧# trying to run then via a pipe wiatch:  [match]"

    if {[llength [skipFiles]] > 0} {
⟦TAB⟧puts [outputChannel] \
⟦TAB⟧⟦TAB⟧"Skipping test files that match:  [skipFiles]"
    }
    if {[llength [matchFiles]] > 0} {
⟦TAB⟧puts [outputChannel] \
⟦TAB⟧⟦TAB⟧"On}
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧close $pipeFd
⟦TAB⟧    } msg]} {
⟦TAB⟧⟦TAB⟧puts [outputChannel] "Test file error: $msg"
⟦TAB⟧⟦TAB⟧# append the name of the test to a list to be reported
⟦TAB⟧⟦TAB⟧# later
⟦TAB⟧⟦TAB⟧lappend testFileFailures $file
⟦TAB⟧    }
⟦TAB⟧}
    }

    # cleanup
    puts [outputChannel] "\nTests ended at [eval $timeCmd]"
    cleanupTests 1
    if {[info exists testFileFailures]} {
⟦TAB⟧puts [outputChannel] "\nTest files exiting with errors:  \n"
⟦TAB⟧fDirectory]] {
⟦TAB⟧set dir [file tail $directory]
⟦TAB⟧puts [outputChannel] [string repeat ~ 44]
⟦TAB⟧puts [outputChannel] "$dir test began at [eval $timeCmd]\n"
⟦TAB⟧
⟦TAB⟧uplevel 1 [list ::source [file join $directory all.tcl]]
⟦TAB⟧
⟦TAB⟧set endTime [eval $timeCmd]
⟦TAB⟧puts [outputChannel] "\n$dir test ended at $endTime"
⟦TAB⟧puts [outputChannel] ""
⟦TAB⟧puts [outputChannel] [string repeat ~ 44]
    }
    return
}

#####################################################################

# Test utility procs - not used in tcltest, but may be useful for
# testing.

# tcltest::loadTestedCommands --
#
#     Uses the specified script to load the commands to test. Allowed to
#     be empty, as the tested commands could have been compiled into the
#     interpreter.
#
# Arguments
#     none
#
# Results
#     none
#
# Side Effects:
#     none.

proc tcltest::loadTestedCommands {} {
    variable l
    if {[string equal {} [loadScript]]} {
⟦TAB⟧return
    }

    return [uplevel 1 [loadScript]]
}

# tcltest::saveState --
#
#⟦TAB⟧Save informaplevel 1 [list ::set [namespace which -variable saveState]] \
⟦TAB⟧    {[::list [::info procs] [::info vars]]}
    DebugPuts  2 "[lindex [info level 0] 0]: $saveState"
    return
}

# tcltest::restoreState --
#
#⟦TAB⟧Remove procs and variables that didn't exist before the call to
#       [saveState].
#
# Arguments:
#⟦TAB⟧none
#
# Results:
#⟦TAB⟧Removes procs and variables from your environment if they don't
#⟦TAB⟧exist in the saveState variable.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::restoreState {} {
    variable saveState
    foreach p [uplevel 1 {::info procs}] {
⟦TAB⟧if {([lsearch [lindex $saveState 0] $p] < 0)moveFile --
#
#⟦TAB⟧Removes the named file from the filesystem
#
# Arguments:
#⟦TAB⟧name          file to be removed
#       directory     directory from which to remove file
#
# Results:
#⟦TAB⟧return value from [file delete]
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::removeFile {name {directory ""}} {
    variable filesMade
    FillFilesExisted
    if {[llength [info level 0]] == 2} {
⟦TAB⟧set directory [temporarylace $filesMade $idx $idx]
    if {$idx == -1} {
⟦TAB⟧DebugDo 1 {
⟦TAB⟧    Warn "removeFile removing \"$fullName\":\n  not created by makeFile"
⟦TAB⟧}
    }⟦SPACE⟧
    if {![file isfile $fullName]} {
⟦TAB⟧DebugDo 1 {
⟦TAB⟧    Warn "removeFile removing \"$fullName\":\n  not a file"
⟦TAB⟧}
    }
    return [file delete $fullName]
}

# tcltest::makeDirectory --
#
# Create a new dir with the name <name>.
#
# If this dir hasn't been created via makeDirectory since the last time
# cleanupTests was called, add it to the $directoriesMade list, so it
# will be removed by the next call to cleanupTests.
#
# Arguments:
#       name            name of the new directory
#       directory       directory in which to create new dir
#
# Results:
#⟦TAB⟧absolute path to the directory created
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::makeDirectory {name {directory ""}} {
    variable filesMade
    FillFilesExisted
    if {[llength [info level 0]] == 2} {
⟦TAB⟧set directory [temporaryDirectory]
    }
    set fullName [file join $directory $name]
    DebugPuts 3 "[lindex [info level 0] 0]: creating $fullName"
    file mkdir $fullName
    if {[lsearch -exact $filesMade $fullName] == -1} {
⟦TAB⟧lappend filesMade $fullName
    }
    return $fullNamsMade $fullName]
    set filesMade [lreplace $filesMade $idx $idx]
    if {$idx == -1} {
⟦TAB⟧DebugDo 1 {
⟦TAB⟧    Warn "removeDirectory removing \"$fullName\":\n  not created\
⟦TAB⟧⟦TAB⟧    by makeDirectory"
⟦TAB⟧}
    e --
#
#⟦TAB⟧reads the content of a file and returns it
#
# Arguments:
#⟦TAB⟧name of the file to read
#       directory in which file is located
#
# Results:
#⟦TAB⟧content of the named file
#
# Side effects:
#⟦TAB⟧No$isoLocale
    }
    return
}

# tcltest::RestoreLocale --
#
#⟦TAB⟧used in cmdIL.test, uses testlocale
#
# Arguments:
#⟦TAB⟧None.
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::RestoreLocale {}fo commands testthread] != {}} {

⟦TAB⟧# testthread built into tcltest

⟦TAB⟧testthread errorproc ThreadNullError
⟦TAB⟧while {[llength [testthread names]] > 1} {
⟦TAB⟧    foreach tid [testthread names] {
⟦TAB⟧⟦TAB⟧if {$tid != [mainThread]} {
⟦TAB⟧⟦TAB⟧    catch {
⟦TAB⟧⟦TAB⟧⟦TAB⟧testthread send -async $tid {testthread exit}
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧    ## Enter a bit a sleep to give the threads enough breathing
⟦TAB⟧    ## room to kill themselves off, otherwise the end up with a
⟦TAB⟧    ## massive queue of repeated events
⟦TAB⟧    after 1
⟦TAB⟧}
⟦TAB⟧testthread errorproc ThreadError
⟦TAB⟧return [llength [testthread names]]
    } elseif {[info commands thread::id] !sync $tid {thread::exit}}
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧    ## Enter a bit a sleep to give the threads enough breathing
⟦TAB⟧    ## room to kill themselves off, otherwise the end up with a
⟦TAB⟧    ## massive queue of repeated events
⟦TAB⟧    after 1
⟦TAB⟧}
⟦TAB⟧thread::errorproc ThreadError
⟦TAB⟧return [llength [thread::names]]
    } else {
⟦TAB⟧return 1
    }
    return 0
}

# Initialize the constraints and set up command line arguments
namespace eval tcltest {
    # Define initializers for all the built-in contraint definitions
    DefineConstraintInitializers

    # Set up the constraints in the testConstraints array to be lazily
    # initialized by a registered initializer, or by "false" if no
    # initializer is registered.
    trace variable testConstraints r [namespace code SafeFetch]

    # Only initialize constraints at panstraint] command
    # straight away, without all this "hook" nonsense.
    if {[string equal [namespace current] \
⟦TAB⟧    [namespace qualifiers [namespace which initConstraintsHook]]]} {
⟦TAB⟧InitConstraints
    } else {
⟦TAB⟧proc initConstraintsHook {} {}
    }

    # Define the standard match commands
    customMatch exact⟦TAB⟧[list string equal]
    customMatch glob⟦TAB⟧[list string match]
    customMatch rege so give it (and exit)
⟦TAB⟧    # right now.  ([configure] does not process -help)
⟦TAB⟧    set required true
⟦TAB⟧}
⟦TAB⟧foreach hook { PrintUsageInfoHook processCmdLineArgsHook
⟦TAB⟧⟦TAB⟧⟦TAB⟧processCmdLineArgsAddFlagsHook } {nly initialize configurable options from the command line arguments
    # at package load time if necessary for backward compatibility.  This
    # lets the tcltest user call [configure] for themselveneArgParsingRequired]} {
⟦TAB⟧ProcessCmdLineArgs
    } else {
⟦TAB⟧EstablishAutoConfigureTraces
    }

    package provide [namespace tail [namespace current]] $Version
}
```

- real-harness-explicit-source-child, lookup, completion 0, result:

```text-visible
child-provider {8.4.6 8.4 ::} module-code 1 module-result {invalid command name "pwd"} require-code 1 require-result {can't find package tcltest}⟦EOF⟧
```

- real-harness-explicit-source-child, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- real-harness-explicit-source-child: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 7

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
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

    # tests that use threads need to know which is the main thread
    Default mainThread 1
    variable mainThread
    if {[info commands thread::id] != {}} {
⟦TAB⟧set mainThread [thread::id]
    } elseif {[info commands testthread] != {}} {
⟦TAB⟧set mainThread [testthread id]
    }

    # Set workingDirectory to [pwd]. The default output directory for
    # Tcl tests is the working directory.  Whenever this value changes
    # change to that directory.
    variable workingDirectory
    trace variable workingDirectory w \
⟦TAB⟧    [namespace code {cd $workingDirectory ;#}]

    Default workingDirectory [pwd] AcceptAbsolutePath
    proc workingDirectory { {dir ""} } {
⟦TAB⟧variable workingDirectory
⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $workingDirectory
⟦TAB⟧}
⟦TAB⟧set workingDirectory [AcceptAbsolutePath $dir]
    }

    # Set the location of the execuatble
    Default tcltest [info nameofexecutable]
    trace variable tcltest w [namespace code {testConstraint stdio \
⟦TAB⟧    [eval [ConstraintInitializer stdio]] ;#}]

    # save the platform information so it can be restored later
    Default originalTclPlatform [array get ::tcl_platform]

    # If a core file exists, save its modification time.
    if {[file exists [file join [workingDirectory] core]]} {
⟦TAB⟧Default coreModTime \
⟦TAB⟧⟦TAB⟧[file mtime [file join [workingDirectory] core]]
    }

    # stdout and stderr buffers for use when we want to store them
    Default outData {}
    Default errData {}

    # keep track of test level for nested test commands
    variable testLevel 0

    # the variables and procs that existed when saveState was called are
    # stored in a variable of the same name
    Default saveState {}

    # Internationalization support -- used in [SetIso8859_1_Locale] and
    # [RestoreLocale]. Those commands are used in cmdIL.test.

    if {![info exists [namespace current]::isoLocale]} {
⟦TAB⟧variable isoLocale fr
⟦TAB⟧switch -- $::tcl_platform(platform) {
⟦TAB⟧    "unix" {

⟦TAB⟧⟦TAB⟧# Try some 'known' values for some platforms:

⟦TAB⟧⟦TAB⟧switch -exact -- $::tcl_platform(os) {
⟦TAB⟧⟦TAB⟧    "FreeBSD" {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr_FR.ISO_8859-1
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    HP-UX {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr_FR.iso88591
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    Linux -
⟦TAB⟧⟦TAB⟧    IRIX {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    default {

⟦TAB⟧⟦TAB⟧⟦TAB⟧# Works on SunOS 4 and Solaris, and maybe
⟦TAB⟧⟦TAB⟧⟦TAB⟧# others...  Define it to something else on your
⟦TAB⟧⟦TAB⟧⟦TAB⟧# system if you want to test those.

⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale iso_8859_1
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧    "windows" {
⟦TAB⟧⟦TAB⟧set isoLocale French
⟦TAB⟧    }
⟦TAB⟧}
    }

    variable ChannelsWeOpened; array set ChannelsWeOpened {}
    # output goes to stdout by default
    Default outputChannel stdout
    proc outputChannel { {filename ""} } {
⟦TAB⟧variable outputChannel
⟦TAB⟧variable ChannelsWeOpened

⟦TAB⟧# This is very subtle and tricky, so let me try to explain.
⟦TAB⟧# (Hopefully this longer comment will be clear when I come
⟦TAB⟧# back in a few months, unlike its predecessor :) )
⟦TAB⟧#⟦SPACE⟧
⟦TAB⟧# The [outputChannel] command (and underlying variable) have to
⟦TAB⟧# be kept in sync with the [configure -outfile] configuration
⟦TAB⟧# option ( and underlying variable Option(-outfile) ).  This is
⟦TAB⟧# accomplished with a write trace on Option(-outfile) that will
⟦TAB⟧# update [outputChannel] whenver a new value is written.  That
⟦TAB⟧# much is easy.
⟦TAB⟧#
⟦TAB⟧# The trick is that in order to maintain compatibility with
⟦TAB⟧# version 1 of tcltest, we must allow every configuration option
⟦TAB⟧# to get its inital value from command line arguments.  This is
⟦TAB⟧# accomplished by setting initial read traces on all the
⟦TAB⟧# configuration options to parse the command line option the first
⟦TAB⟧# time they are read.  These traces are cancelled whenever the
⟦TAB⟧# program itself calls [configure].
⟦TAB⟧#⟦SPACE⟧
⟦TAB⟧# OK, then so to support tcltest 1 compatibility, it seems we want
⟦TAB⟧# to get the return from [outputFile] to trigger the read traces,
⟦TAB⟧# just in case.
⟦TAB⟧#
⟦TAB⟧# BUT!  A little known feature of Tcl variable traces is that⟦SPACE⟧
⟦TAB⟧# traces are disabled during the handling of other traces.  So,
⟦TAB⟧# if we trigger read traces on Option(-outfile) and that triggers
⟦TAB⟧# command line parsing which turns around and sets an initial
⟦TAB⟧# value for Option(-outfile) -- <whew!> -- the write trace that
⟦TAB⟧# would keep [outputChannel] in sync with that new initial value
⟦TAB⟧# would not fire!
⟦TAB⟧#
⟦TAB⟧# SO, finally, as a workaround, instead of triggering read traces
⟦TAB⟧# by invoking [outputFile], we instead trigger the same set of
⟦TAB⟧# read traces by invoking [debug].  Any command that reads a
⟦TAB⟧# configuration option would do.  [debug] is just a handy one.
⟦TAB⟧# The end result is that we support tcltest 1 compatibility and
⟦TAB⟧# keep outputChannel and -outfile in sync in all cases.
⟦TAB⟧debug

⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $outputChannel
⟦TAB⟧}
⟦TAB⟧if {[info exists ChannelsWeOpened($outputChannel)]} {
⟦TAB⟧    close $outputChannel
⟦TAB⟧    unset ChannelsWeOpened($outputChannel)
⟦TAB⟧}
⟦TAB⟧switch -exact -- $filename {
⟦TAB⟧    stderr -
⟦TAB⟧    stdout {
⟦TAB⟧⟦TAB⟧set outputChannel $filename
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧set outputChannel [open $filename a]
⟦TAB⟧⟦TAB⟧set ChannelsWeOpened($outputChannel) 1

⟦TAB⟧⟦TAB⟧# If we created the file in [temporaryDirectory], then
⟦TAB⟧⟦TAB⟧# [cleanupTests] will delete it, unless we claim it was
⟦TAB⟧⟦TAB⟧# already there.
⟦TAB⟧⟦TAB⟧set outdir [normalizePath [file dirname \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [pwd] $filename]]]
⟦TAB⟧⟦TAB⟧if {[string equal $outdir [temporaryDirectory]]} {
⟦TAB⟧⟦TAB⟧    variable filesExisted
⟦TAB⟧⟦TAB⟧    FillFilesExisted
⟦TAB⟧⟦TAB⟧    set filename [file tail $filename]
⟦TAB⟧⟦TAB⟧    if {[lsearch -exact $filesExisted $filename] == -1} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧lappend filesExisted $filename
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $outputChannel
    }

    # errors go to stderr by default
    Default errorChannel stderr
    proc errorChannel { {filename ""} } {
⟦TAB⟧variable errorChannel
⟦TAB⟧variable ChannelsWeOpened

⟦TAB⟧# This is subtle and tricky.  See the comment above in
⟦TAB⟧# [outputChannel] for a detailed explanation.
⟦TAB⟧debug

⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $errorChannel
⟦TAB⟧}
⟦TAB⟧if {[info exists ChannelsWeOpened($errorChannel)]} {
⟦TAB⟧    close $errorChannel
⟦TAB⟧    unset ChannelsWeOpened($errorChannel)
⟦TAB⟧}
⟦TAB⟧switch -exact -- $filename {
⟦TAB⟧    stderr -
⟦TAB⟧    stdout {
⟦TAB⟧⟦TAB⟧set errorChannel $filename
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧set errorChannel [open $filename a]
⟦TAB⟧⟦TAB⟧set ChannelsWeOpened($errorChannel) 1

⟦TAB⟧⟦TAB⟧# If we created the file in [temporaryDirectory], then
⟦TAB⟧⟦TAB⟧# [cleanupTests] will delete it, unless we claim it was
⟦TAB⟧⟦TAB⟧# already there.
⟦TAB⟧⟦TAB⟧set outdir [normalizePath [file dirname \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [pwd] $filename]]]
⟦TAB⟧⟦TAB⟧if {[string equal $outdir [temporaryDirectory]]} {
⟦TAB⟧⟦TAB⟧    variable filesExisted
⟦TAB⟧⟦TAB⟧    FillFilesExisted
⟦TAB⟧⟦TAB⟧    set filename [file tail $filename]
⟦TAB⟧⟦TAB⟧    if {[lsearch -exact $filesExisted $filename] == -1} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧lappend filesExisted $filename
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $errorChannel
    }

##### Set up the configurable options
    #
    # The configurable options of the package
    variable Option; array set Option {}

    # Usage strings for those options
    variable Usage; array set Usage {}

    # Verification commands for those options
    variable Verify; array set Verify {}

    # Initialize the default values of the configurable options that are
    # historically associated with an exported variable.  If that variable
    # is already set, support compatibility by accepting its pre-set value.
    # Use [trace] to establish ongoing connection between the deprecated
    # exported variable and the modern option kept as a true internal var.
    # Also set up usage string and value testing for the option.
    proc Option {option value usage {verify AcceptAll} {varName {}}} {
⟦TAB⟧variable Option
⟦TAB⟧variable Verify
⟦TAB⟧variable Usage
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧set Usage($option) $usage
⟦TAB⟧set Verify($option) $verify
⟦TAB⟧if {[catch {$verify $value} msg]} {
⟦TAB⟧    return -code error $msg
⟦TAB⟧} else {
⟦TAB⟧    set Option($option) $msg
⟦TAB⟧}
⟦TAB⟧if {[string length $varName]} {
⟦TAB⟧    variable $varName
⟦TAB⟧    if {[info exists $varName]} {
⟦TAB⟧⟦TAB⟧if {[catch {$verify [set $varName]} msg]} {
⟦TAB⟧⟦TAB⟧    return -code error $msg
⟦TAB⟧⟦TAB⟧} else {
⟦TAB⟧⟦TAB⟧    set Option($option) $msg
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧unset $varName
⟦TAB⟧    }
⟦TAB⟧    namespace eval [namespace current] \
⟦TAB⟧    ⟦TAB⟧    [list upvar 0 Option($option) $varName]
⟦TAB⟧    # Workaround for Bug (now Feature Request) 572889.  Grrrr....
⟦TAB⟧    # Track all the variables tied to options
⟦TAB⟧    lappend OptionControlledVariables $varName
⟦TAB⟧    # Later, set auto-configure read traces on all
⟦TAB⟧    # of them, since a single trace on Option does not work.
⟦TAB⟧    proc $varName {{value {}}} [subst -nocommands {
⟦TAB⟧⟦TAB⟧if {[llength [info level 0]] == 2} {
⟦TAB⟧⟦TAB⟧    Configure $option [set value]
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧return [Configure $option]
⟦TAB⟧    }]
⟦TAB⟧}
    }

    proc MatchingOption {option} {
⟦TAB⟧variable Option
⟦TAB⟧set match [array names Option $option*]
⟦TAB⟧switch -- [llength $match] {
⟦TAB⟧    0 {
⟦TAB⟧⟦TAB⟧set sorted [lsort [array names Option]]
⟦TAB⟧⟦TAB⟧set values [join [lrange $sorted 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append values ", or [lindex $sorted end]"
⟦TAB⟧⟦TAB⟧return -code error "unknown option $option: should be\
⟦TAB⟧⟦TAB⟧⟦TAB⟧one of $values"
⟦TAB⟧    }
⟦TAB⟧    1 {
⟦TAB⟧⟦TAB⟧return [lindex $match 0]
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧# Exact match trumps ambiguity
⟦TAB⟧⟦TAB⟧if {[lsearch -exact $match $option] >= 0} {
⟦TAB⟧⟦TAB⟧    return $option
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧set values [join [lrange $match 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append values ", or [lindex $match end]"
⟦TAB⟧⟦TAB⟧return -code error "ambiguous option $option:\
⟦TAB⟧⟦TAB⟧⟦TAB⟧could match $values"
⟦TAB⟧    }
⟦TAB⟧}
    }

    proc EstablishAutoConfigureTraces {} {
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧foreach varName [concat $OptionControlledVariables Option] {
⟦TAB⟧    variable $varName
⟦TAB⟧    trace variable $varName r [namespace code {ProcessCmdLineArgs ;#}]
⟦TAB⟧}
    }

    proc RemoveAutoConfigureTraces {} {
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧foreach varName [concat $OptionControlledVariables Option] {
⟦TAB⟧    variable $varName
⟦TAB⟧    foreach pair [trace vinfo $varName] {
⟦TAB⟧⟦TAB⟧foreach {op cmd} $pair break
⟦TAB⟧⟦TAB⟧if {[string equal r $op]
⟦TAB⟧⟦TAB⟧⟦TAB⟧&& [string match *ProcessCmdLineArgs* $cmd]} {
⟦TAB⟧⟦TAB⟧    trace vdelete $varName $op $cmd
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧# Once the traces are removed, this can become a no-op
⟦TAB⟧proc RemoveAutoConfigureTraces {} {}
    }

    proc Configure args {
⟦TAB⟧variable Option
⟦TAB⟧variable Verify
⟦TAB⟧set n [llength $args]
⟦TAB⟧if {$n == 0} {
⟦TAB⟧    return [lsort [array names Option]]
⟦TAB⟧}
⟦TAB⟧if {$n == 1} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    return $Option($option)
⟦TAB⟧}
⟦TAB⟧while {[llength $args] > 1} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    if {[catch {$Verify($option) [lindex $args 1]} value]} {
⟦TAB⟧⟦TAB⟧return -code error "invalid $option\
⟦TAB⟧⟦TAB⟧⟦TAB⟧value \"[lindex $args 1]\": $value"
⟦TAB⟧    }
⟦TAB⟧    set Option($option) $value
⟦TAB⟧    set args [lrange $args 2 end]
⟦TAB⟧}
⟦TAB⟧if {[llength $args]} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    return -code error "missing value for option $option"
⟦TAB⟧}
    }
    proc configure args {
⟦TAB⟧RemoveAutoConfigureTraces
⟦TAB⟧set code [catch {eval Configure $args} msg]
⟦TAB⟧return -code $code $msg
    }
⟦SPACE⟧⟦SPACE⟧⟦SPACE⟧⟦SPACE⟧
    proc AcceptVerbose { level } {
⟦TAB⟧set level [AcceptList $level]
⟦TAB⟧if {[llength $level] == 1} {
⟦TAB⟧    if {![regexp {^(pass|body|skip|start|error)$} $level]} {
⟦TAB⟧⟦TAB⟧# translate single characters abbreviations to expanded list
⟦TAB⟧⟦TAB⟧set level [string map {p pass b body s skip t start e error} \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[split $level {}]]
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set valid [list]
⟦TAB⟧foreach v $level {
⟦TAB⟧    if {[regexp {^(pass|body|skip|start|error)$} $v]} {
⟦TAB⟧⟦TAB⟧lappend valid $v
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $valid
    }

    proc IsVerbose {level} {
⟦TAB⟧variable Option
⟦TAB⟧return [expr {[lsearch -exact $Option(-verbose) $level] != -1}]
    }

    # Default verbosity is to show bodies of failed tests
    Option -verbose {body error} {
⟦TAB⟧Takes any combination of the values 'p', 's', 'b', 't' and 'e'.
⟦TAB⟧Test suite will display all passed tests if 'p' is specified, all
⟦TAB⟧skipped tests if 's' is specified, the bodies of failed tests if
⟦TAB⟧'b' is specified, and when tests start if 't' is specified.
⟦TAB⟧ErrorInfo is displayed if 'e' is specified.
    } AcceptVerbose verbose

    # Match and skip patterns default to the empty list, except for
    # matchFiles, which defaults to all .test files in the
    # testsDirectory and matchDirectories, which defaults to all
    # directories.
    Option -match * {
⟦TAB⟧Run all tests within the specified files that match one of the
⟦TAB⟧list of glob patterns given.
    } AcceptList match

    Option -skip {} {
⟦TAB⟧Skip all tests within the specified tests (via -match) and files
⟦TAB⟧that match one of the list of glob patterns given.
    } AcceptList skip

    Option -file *.test {
⟦TAB⟧Run tests in all test files that match the glob pattern given.
    } AcceptPattern matchFiles

    # By default, skip files that appear to be SCCS lock files.
    Option -notfile l.*.test {
⟦TAB⟧Skip all test files that match the glob pattern given.
    } AcceptPattern skipFiles

    Option -relateddir * {
⟦TAB⟧Run tests in directories that match the glob pattern given.
    } AcceptPattern matchDirectories

    Option -asidefromdir {} {
⟦TAB⟧Skip tests in directories that match the glob pattern given.
    } AcceptPattern skipDirectories

    # By default, don't save core files
    Option -preservecore 0 {
⟦TAB⟧If 2, save any core files produced during testing in the directory
⟦TAB⟧specified by -tmpdir. If 1, notify the user if core files are
⟦TAB⟧created.
    } AcceptInteger preserveCore

    # debug output doesn't get printed by default; debug level 1 spits
    # up only the tests that were skipped because they didn't match or
    # were specifically skipped.  A debug level of 2 would spit up the
    # tcltest variables and flags provided; a debug level of 3 causes
    # some additional output regarding operations of the test harness.
    # The tcltest package currently implements only up to debug level 3.
    Option -debug 0 {
⟦TAB⟧Internal debug level⟦SPACE⟧
    } AcceptInteger debug

    proc SetSelectedConstraints args {
⟦TAB⟧variable Option
⟦TAB⟧foreach c $Option(-constraints) {
⟦TAB⟧    testConstraint $c 1
⟦TAB⟧}
    }
    Option -constraints {} {
⟦TAB⟧Do not skip the listed constraints listed in -constraints.
    } AcceptList
    trace variable Option(-constraints) w \
⟦TAB⟧    [namespace code {SetSelectedConstraints ;#}]

    # Don't run only the "-constraint" specified tests by default
    proc ClearUnselectedConstraints args {
⟦TAB⟧variable Option
⟦TAB⟧variable testConstraints
⟦TAB⟧if {!$Option(-limitconstraints)} {return}
⟦TAB⟧foreach c [array names testConstraints] {
⟦TAB⟧    if {[lsearch -exact $Option(-constraints) $c] == -1} {
⟦TAB⟧⟦TAB⟧testConstraint $c 0
⟦TAB⟧    }
⟦TAB⟧}
    }
    Option -limitconstraints false {
⟦TAB⟧whether to run only tests with the constraints
    } AcceptBoolean limitConstraints⟦SPACE⟧
    trace variable Option(-limitconstraints) w \
⟦TAB⟧    [namespace code {ClearUnselectedConstraints ;#}]

    # A test application has to know how to load the tested commands
    # into the interpreter.
    Option -load {} {
⟦TAB⟧Specifies the script to load the tested commands.
    } AcceptScript loadScript

    # Default is to run each test file in a separate process
    Option -singleproc 0 {
⟦TAB⟧whether to run all tests in one process
    } AcceptBoolean singleProcess⟦SPACE⟧

    proc AcceptTemporaryDirectory { directory } {
⟦TAB⟧set directory [AcceptAbsolutePath $directory]
⟦TAB⟧if {![file exists $directory]} {
⟦TAB⟧    file mkdir $directory
⟦TAB⟧}
⟦TAB⟧set directory [AcceptDirectory $directory]
⟦TAB⟧if {![file writable $directory]} {
⟦TAB⟧    if {[string equal [workingDirectory] $directory]} {
⟦TAB⟧⟦TAB⟧# Special exception: accept the default value
⟦TAB⟧⟦TAB⟧# even if the directory is not writable
⟦TAB⟧⟦TAB⟧return $directory
⟦TAB⟧    }
⟦TAB⟧    return -code error "\"$directory\" is not writeable"
⟦TAB⟧}
⟦TAB⟧return $directory
    }

    # Directory where files should be created
    Option -tmpdir [workingDirectory] {
⟦TAB⟧Save temporary files in the specified directory.
    } AcceptTemporaryDirectory temporaryDirectory
    trace variable Option(-tmpdir) w \
⟦TAB⟧    [namespace code {normalizePath Option(-tmpdir) ;#}]

    # Tests should not rely on the current working directory.
    # Files that are part of the test suite should be accessed relative
    # to [testsDirectory]
    Option -testdir [workingDirectory] {
⟦TAB⟧Search tests in the specified directory.
    } AcceptDirectory testsDirectory
    trace variable Option(-testdir) w \
⟦TAB⟧    [namespace code {normalizePath Option(-testdir) ;#}]

    proc AcceptLoadFile { file } {
⟦TAB⟧if {[string equal "" $file]} {return $file}
⟦TAB⟧set file [file join [temporaryDirectory] $file]
⟦TAB⟧return [AcceptReadable $file]
    }
    proc ReadLoadScript {args} {
⟦TAB⟧variable Option
⟦TAB⟧if {[string equal "" $Option(-loadfile)]} {return}
⟦TAB⟧set tmp [open $Option(-loadfile) r]
⟦TAB⟧loadScript [read $tmp]
⟦TAB⟧close $tmp
    }
    Option -loadfile {} {
⟦TAB⟧Read the script to load the tested commands from the specified file.
    } AcceptLoadFile loadFile
    trace variable Option(-loadfile) w [namespace code ReadLoadScript]

    proc AcceptOutFile { file } {
⟦TAB⟧if {[string equal stderr $file]} {return $file}
⟦TAB⟧if {[string equal stdout $file]} {return $file}
⟦TAB⟧return [file join [temporaryDirectory] $file]
    }

    # output goes to stdout by default
    Option -outfile stdout {
⟦TAB⟧Send output from test runs to the specified file.
    } AcceptOutFile outputFile
    trace variable Option(-outfile) w \
⟦TAB⟧    [namespace code {outputChannel $Option(-outfile) ;#}]

    # errors go to stderr by default
    Option -errfile stderr {
⟦TAB⟧Send errors from test runs to the specified file.
    } AcceptOutFile errorFile
    trace variable Option(-errfile) w \
⟦TAB⟧    [namespace code {errorChannel $Option(-errfile) ;#}]

}

#####################################################################

# tcltest::Debug* --
#
#     Internal helper procedures to write out debug information
#     dependent on the chosen level. A test shell may overide
#     them, f.e. to redirect the output into a different
#     channel, or even into a GUI.

# tcltest::DebugPuts --
#
#     Prints the specified string if the current debug level is
#     higher than the provided level argument.
#
# Arguments:
#     level   The lowest debug level triggering the output
#     string  The string to print out.
#
# Results:
#     Prints the string. Nothing else is allowed.
#
# Side Effects:
#     None.
#

proc tcltest::DebugPuts {level string} {
    variable debug
    if {$debug >= $level} {
⟦TAB⟧puts $string
    }
    return
}

# tcltest::DebugPArray --
#
#     Prints the contents of the specified array if the current
#       debug level is higher than the provided level argument
#
# Arguments:
#     level           The lowest debug level triggering the output
#     arrayvar        The name of the array to print out.
#
# Results:
#     Prints the contents of the array. Nothing else is allowed.
#
# Side Effects:
#     None.
#

proc tcltest::DebugPArray {level arrayvar} {
    variable debug

    if {$debug >= $level} {
⟦TAB⟧catch {upvar  $arrayvar $arrayvar}
⟦TAB⟧parray $arrayvar
    }
    return
}

# Define our own [parray] in ::tcltest that will inherit use of the [puts]
# defined in ::tcltest.  NOTE: Ought to construct with [info args] and
# [info default], but can't be bothered now.  If [parray] changes, then
# this will need changing too.
auto_load ::parray
proc tcltest::parray {a {pattern *}} [info body ::parray]

# tcltest::DebugDo --
#
#     Executes the script if the current debug level is greater than
#       the provided level argument
#
# Arguments:
#     level   The lowest debug level triggering the execution.
#     sc""}} {
    variable testConstraints
    variable Option
    DebugPuts 3 "entering testConstraint $constraint $value"
    if {[llength [info level 0]] == 2} {
⟦TAB⟧return $testConstraints($constraint)
     equal {} $interp]} {
⟦TAB⟧set tcltest {}
    } else {
⟦TAB⟧set tcltest $interp
    }
}

#####################################################################

# tcltest::AddToSkippedBecause --
#
#⟦TAB⟧Increments the variable used to track how many tests were
#       skipped because of a particular constraint.
#
# Arguments:
#⟦TAB⟧constraint     The name of the constraint to be modified
#
# Results:
#⟦TAB⟧Modifies tc} {
    # add the constraint to the list of constraints that kept tests
    # from running
    variable skippedBecause

    if {[info exists skippedBecause($constraint)]} {
⟦TAB⟧incr skippedBecause($constraint) $value
    } else {
⟦TAB⟧set skippedBecause($constraint) $value
    }
    return
}

# tcltest::PrintError --
#
#⟦TAB⟧Prints errors to tcltest::errorChannel and then flushes that
#       channel, makingtError {errorMsg} {
    set InitialMessage "Error:  "
    set InitialMsgLen  [string length $InitialMessage]
    puts -nonewline [errorChannel] $InitialMessage

    # Keep track of where the end of the string is.
    set endingIndex [string length $errorMsg]

    if {$endingIndex < (80 - $InitialMsgLen)} {
⟦TAB⟧puts [errorChannel] $errorMsg
    } else {
⟦TAB⟧# Print up to 80 characters on the first line, including the
⟦TAB⟧# InitialMessage.
⟦TAB⟧set beginningIndex [string last " " [string range $errorMsg 0 \
⟦TAB⟧⟦TAB⟧[expr {80 - $InitialMsgLen}]]]
⟦TAB⟧puts [errorChannel] [string range $errorMsg 0 $beginningIndex]

⟦TAB⟧while {![string equal end $beginningIndex]} {
⟦TAB⟧    puts -nonewline [errorChannel] \
⟦TAB⟧⟦TAB⟧    [string repeat " " $InitialMsgLen]
⟦TAB⟧    if {($endingIndex - $beginningIndex)
⟦TAB⟧⟦TAB⟧    < (80 - $InitialMsgLen)} {
⟦TAB⟧⟦TAB⟧puts [errorChannel] [string trim \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[string range $errorMsg $beginningIndex end]]
⟦TAB⟧⟦TAB⟧break
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧set newEndingIndex [expr {[string last " " \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[string range $errorMsg $beginningIndex \
⟦TAB⟧⟦TAB⟧⟦TAB⟧tring equal {} $n2]} {return}
    if {![info exists testConstraints($n2)]} {
⟦TAB⟧if {[catch {testConstraint $n2 [eval [ConstraintInitializer $n2]]}]} {
⟦TAB⟧    testConstraint $n2 0
⟦TAB⟧}
    }
}

# tcltest::ConstraintInitializer --
#
#⟦TAB⟧Get or set a script that when evaluated in the tcltest namespace
#⟦TAB⟧will return a boolean value with which to initialize the
#⟦TAB⟧associated constraint.
#
# Arguments:
#⟦TAB⟧constraint - name of the constraint initialized by the script
#⟦TAB⟧script - the initializer script
#
# Results
#⟦TAB⟧boolean value of the constraint - enabled or disabled
#
# Side effects:
#⟦TAB⟧Constraint is initialized for future reference by [test]
proc tcltest::ConstraintInitializer {constraint {script ""}} {
    variable ConstraintInitializer
    DebugPuts 3 "entering ConstraintInitializer $constraint $script"r must be complete script"
    }
    set ConstraintInitializer($constraint) $script
}

# tcltest::InitConstraints --
#
# Call all registered constraint initializers to force initialization
# of all known constraints.
# See the tcltest man page for the list of built-in constraints defined
# in this procedure.
#
# Arguments:
#⟦TAB⟧none
#
# Results:
#⟦TAB⟧The testConstraints array is reset to have an index for each
#⟦TAB⟧built-in test constraint.
#
# Side Effects:
#       None.
#

proc tcltest::InitConstraints {} {
    variable ConstraintInitializer
    initConstraintsHook
    foreach constraint [array names ConstraintInitializer] {
⟦TAB⟧testConstraint $constraint
    }
}

proc tcltest::DefineConstraintInitializers {} {
    ConstraintInitializer singleTestInterp {singleProcess}

    # All the 'pc' constraints are here for backward compatibility and
    # are not documented.  They have been replaced with equivalent 'win'
    # constraints.

    ConstraintInitializer unixOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) unix}
    ConstraintInitializer macOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) macintosh}
    ConstraintInitializer pcOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) windo $f}]} {
⟦TAB⟧⟦TAB⟧    set code 1
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set code
    }

    # Deliberately call socket with the wrong number of arguments.  The
    # error message you get will indicate whether sockets are availabletInitializer hasIsoLocale {
⟦TAB⟧if {[llength [info commands testlocale]] == 0} {
⟦TAB⟧    set code 0
⟦TAB⟧} else {
⟦TAB⟧    set code [string length [SetIso8859_1_Locale]]
⟦TAB⟧    RestoreLocale
⟦TAB⟧}
⟦TAB⟧set code
    }

}
#####################################################################

# Usage and command line arguments processing.

# tcltest::PrintUsageInfo
#
#⟦TAB⟧Prints out the usage information for package tcltest.  This can
#⟦TAB⟧be customized with the redefinition of [PrintUsageInfoHook].
#
# Arguments:
#⟦TAB⟧none
#
# Results:
#       none
#
# Side Effects:
#       none
proc tcltest::PrintUsageInfo {} {
    puts [Usage]
    PrintUsageInfoHook
}

proc tcltest::Usage { {option ""} } {
    variable Usage
    variable Verify
    if {[llength [info level 0]] == 1} {
⟦TAB⟧set msg "Usage: [file tail [info nameofexecutable]] [Usage $opt]
⟦TAB⟧    foreach [list x type($opt) usage($opt)] $foo break
⟦TAB⟧    set line($opt) "  $opt $type($opt)  "
⟦TAB⟧    set length($opt) [string length $line($opt)]
⟦TAB⟧    if {$length($opt) > $max} {set max $length($opt)}
⟦TAB⟧}
⟦TAB⟧set rest [expr {72 - $max}]
⟦TAB⟧foreach opt $allOpts {
⟦TAB⟧    append msg \n$line($opt)
⟦TAB⟧    append msg [string repeat " " [expr {$max - $length($opt)}]]
⟦TAB⟧    set u [string trim $usage($opt)]
⟦TAB⟧    catch {append u "  (default: \[[Configure $opt]])"}
⟦TAB⟧    regsub -all {\s*\n\s*} $u " " u
⟦TAB⟧    while {[string length $u] > $rest} {
⟦TAB⟧⟦TAB⟧set break [string wordstart $u $rest]
⟦TAB⟧⟦TAB⟧if {$break == 0} {
⟦TAB⟧⟦TAB⟧    set break [string wordend $u 0]
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧append msg [string range $u 0 [expr {$break - 1}]]
⟦TAB⟧⟦TAB⟧set u [string trim [string range $u $break end]]
⟦TAB⟧⟦TAB⟧append msg \n[string repeat " " $max]
⟦TAB⟧    }
⟦TAB⟧    append msg $u
⟦TAB⟧}
⟦TAB⟧return $msg\n
    } elseif {[string equal -help $option]} {
⟦TAB⟧return [list -help "" "Display this usage information."]
    } else {
⟦TAB⟧set type [lindex [info args $Verify($option)] 0]ent (tcltest::originalEnv):"
    DebugPArray  2 originalEnv
    DebugPuts    2 "Constraints:"
    DebugPArray  2 testConstraints
}

#####################################################################

# Code to run the tests goes here.

# tcltest::TestPuts --
#
#⟦TAB⟧Used to redefine puts in test environment.  Stores whatever goes
#⟦TAB⟧out on stdout in tcltest::outData and stderr in errData before
#⟦TAB⟧sending it on to the regular puts.
#
# Arguments:
#⟦TAB⟧same as standard puts
#
# Results:
#⟦TAB⟧none
#
# Side effects:
#       Intercepts puts; data that would otherwise go to stdout, stderr,
#⟦TAB⟧or file channels specified in outputChannel and errorChannel
#⟦TAB⟧does not get sent to the normal puts function.
namespace eval tcltest::Replace {
    namespace export puts
}
proc tcltest::Replace::puts {args} {
    variable [namespace parent]::outData
    variable [namespace parent]::errData
    switch [llength $args] {
⟦TAB⟧1 {
⟦TAB⟧    # Only the string to be printed is specified
⟦TAB⟧    append outData [lindex $args 0]\n
⟦TAB⟧    return
⟦TAB⟧    # return [Puts [lindex $args 0]]
⟦TAB⟧}
⟦TAB⟧2 {
⟦TAB⟧    # Either -nonewline or channelId has been specified
⟦TAB⟧    if {[string equal -nonewline [lindex $args 0]]} {
⟦TAB⟧⟦TAB⟧append outData [lindex $args end]
⟦TAB⟧⟦TAB⟧return
⟦TAB⟧⟦TAB⟧# return [Puts -nonewline [lindex $args end]]
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧set channel [lindex $args 0]
⟦TAB⟧⟦TAB⟧set newline \n
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧3 {
⟦TAB⟧    if {[string equal -nonewline [lindex $args 0]]} {
⟦TAB⟧⟦TAB⟧# Both -nonewline and channelId are specified, unless
⟦TAB⟧⟦TAB⟧# it's an error.  -nonewline is supposed to be argv[0].
⟦TAB⟧⟦TAB⟧set channel [lindex $args 1]
⟦TAB⟧⟦TAB⟧set newline ""
⟦TAB⟧    }
⟦TAB⟧}
    }

    if {[info exists channel]} {
⟦TAB⟧if {[string equal $channel [[namespace parent]::outputChannel]]
⟦TAB⟧⟦TAB⟧|| [string equal $channel stdout]} {
⟦TAB⟧    append outData [lindex $args end]$newline
⟦TAB⟧    return
⟦TAB⟧} elseif {[string equal $channel [[namespace parent]::errorChannel]]
⟦TAB⟧⟦TAB⟧|| [string equal $channel stderr]} {
⟦TAB⟧    append errData [lindex $args end]$newline
⟦TAB⟧    return
⟦TAB⟧}
    }

    # If we haven't returned by now, we don't know ho   # separated strings as it throws away the whitespace which maybe
    # important so we have to do it all by hand.

    set result {}
    set token ""

    while {[string length $argList]} {
        # Look for the next word containing a quote: " { }
        if {[regexp -indices {[^ \t\n]*[\"\{\}]+[^ \t\n]*} \
⟦TAB⟧⟦TAB⟧$argList all]} {
            # Get the text leading up to this word, but not includinargList \
                    [lindex $all 0] [lindex $all 1]]

            # Remove all text up to and including the word from the
            # argList.
            set argList [string range $argList \
                    [expr {[lindex $all 1] + 1}] end]
        } else {
            # Take everything up to the end of the argList.
            set text $argList
            set word {}
            set argList {}
        }

        if {$token != {}} {
            # If we saw a word with quote before, then there is a
            # multi-word token starting with that word.  In this case,
            # add the text and the current word to this token.
            append token $text $word
        } else {
            # Add the text to the result.  There is no need to parse
            # the text because it couldn't be a part of any multi-word
            # token.  Then start a new multi-word token with the word
            # because we need to pass this token to the Tcl parser to
            # check for balancing quotes
            append result $text
            set token $word
        }

        if { [catch {llength $token} length] == 0 && $length == 1} {
            # The token is a valid list so add it to the result.
            # lappend result [string trim $token]
            append result \{$token\}
            set token {}
        }
    }

    # If the last token has not been added to the list then there
    # is a problem.
    if { [string length $token] } {
        error "incomplete token \"$token\""
    }

    return $result
}

# tcltest::test --
#
# This procedure ru  This
#                       attribute is optional; default is {}.
#   cleanup -           Code to run after $script (above).  This
#                       attribute is optional; default is {}.
#   match -             specifies type of matching to do on result,
#                       output, errorOutput; this must be a string
#⟦TAB⟧⟦TAB⟧⟦TAB⟧previously registered by a call to [customMatch].
#⟦TAB⟧⟦TAB⟧⟦TAB⟧The strings exact, glob, and regexp are pre-registered
#⟦TAB⟧⟦TAB⟧⟦TAB⟧by the tcltest package.  Default value is exact.
#
# Arguments:
#   name -⟦TAB⟧⟦TAB⟧Name of test, in the form foo-1.2.
#   description -⟦TAB⟧Short textual description of the test, to
#  ⟦TAB⟧⟦TAB⟧  ⟦TAB⟧help humans understand what it does.
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#       Just about anything is possible depending on the test.
#

proc tcltest::test {name description args} {
    global tcl_platform
    variable testLevel
    variable coreModTime
    DebugPuts 3 "test $name $args"
    DebugDo 1 {
⟦TAB⟧variable TestNames
⟦TAB⟧catch {
⟦TAB⟧    puts "test name '$name' re-used; prior use in $TestNames($name)"
⟦TAB⟧}
⟦TAB⟧set TestNames($name) [info script]
    }

    FillFilesExisted
    incr testLevel

    # Pre-define everything to null except output and errorOutput.  We
    # determine whether or not to trap output based on whether or not
    # these variables (output & errorOutput) are defined.
    foreach item {constraints setup cleanup body result returnCodes
⟦TAB⟧    matchl; 2 represents
    # 'return' being used in the test script).
    set returnCodes [list 0 2]

    # The old test format can't have a 3rd argument (constraints or
    # script) that starts with '-'.
    if {[string match -* [lindex $args 0]]
⟦TAB⟧    || ([llength $args] <= 1)} {
⟦TAB⟧if {[llength $args] == 1} {
⟦TAB⟧    set list [SubstArguments [lindex $args 0]]
⟦TAB⟧    foreach {element value} $list {
⟦TAB⟧⟦TAB⟧set testMatch] $match] == -1} {
⟦TAB⟧    incr testLevel -1
⟦TAB⟧    set sorted [lsort [array names CustomMatch]]
⟦TAB⟧    set values [join [lrange $sorted 0 end-1] ", "]
⟦TAB⟧    append values ", or [lindex $sorted end]"
⟦TAB⟧⟦SPACE⟧⟦SPACE⟧
⟦TAB⟧}
    }

    # First, run the setup script
    set code [catch {uplevel 1 $setup} setupMsg]
    if {$code == 1} {
⟦TAB⟧set errorInfo(setup) $::errorInfo
⟦TAB⟧set errorCode(setup) $::errorCode
    }
    set setupFailure [expr {$code != 0}]

    # Only run the test body if the setup was successful
    if {!$setupFailure} {

⟦TAB⟧# Verbose notification of $body start
⟦TAB⟧if {[IsVerbose start]} {
⟦TAB⟧    puts [outputevel 1 [list [namespace origin Eval] $command 0]]
⟦TAB⟧} else {
⟦TAB⟧    set testResult [uplevel 1 [list [namespace origin Eval] $command 1]]
⟦TAB⟧}
⟦TAB⟧foreach {actualAnswer returnCode} $testResult break
⟦TAB⟧if {$returnCode == 1} {
⟦TAB⟧    set errorInfo(body) $::errorInfo
⟦TAB⟧    set errorCode(body) $::errorCode
⟦TAB⟧}
    }

    # Always run the cleanup script
    set code [catch {uplevel 1 $cleanup} cleanupMsg]
    if {$code == 1} {
⟦TAB⟧set errorInfo(cleanup) $::errorInfo
⟦TAB⟧set errorCode(cleanup) $::errorCode
    }
    set cleanupFailure [expr {$code != 0}]

    set coreFailure 0
    set coreMsg ""
    # check for a core file first - if one was created by the test,
    # then the test failed
    if {[preserveCore]} {
⟦TAB⟧if {[file exists [file join [workingDirectory] core]]} {
⟦TAB⟧    # There's only a test failure if there is a core file
⟦TAB⟧    # and (1) there previously wasn't one or (2) the new
⟦TAB⟧    # one is different from the old one.
⟦TAB⟧    if {[info exists coreModTime]} {
⟦TAB⟧⟦TAB⟧if {$coreModTime != [file mtime \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [workingDirectory] core]]} {
⟦TAB⟧⟦TAB⟧    set coreFailure 1
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧set coreFailure 1
⟦TAB⟧    }
⟦TAB⟧
⟦TAB⟧    if {([preserveCore] > 1) && ($coreFailure)} {
⟦TAB⟧⟦TAB⟧append coreMsg "\nMoving file to:\
⟦TAB⟧⟦TAB⟧    [file join [temporaryDirectory] core-$name]"
⟦TAB⟧⟦TAB⟧catch {file rename -force \
⟦TAB⟧⟦TAB⟧    [file join [workingDirectory] core] \
⟦TAB⟧⟦TAB⟧    [file join [temporaryDirectory] core-$name]
⟦TAB⟧⟦TAB⟧} msg
⟦TAB⟧⟦TAB⟧if {[string length $msg] > 0} {
⟦TAB⟧⟦TAB⟧    append coreMsg "\nError:\
⟦TAB⟧⟦TAB⟧⟦TAB⟧Problem renaming core file: $msg"
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
    }

    # check if the return code matched the expected return code
    set codeFailure 0
    if {!$setupFmpare [catch {
⟦TAB⟧CompareStrings $actualAnswer $result $match
    } scriptMatch]] == 0} {
⟦TAB⟧set scriptFailure [expr {!$scriptMatch}]
    } else {
⟦TAB⟧set scriptFailure 1
    }

    # if we didn't experiencetestLevel == 1} {
⟦TAB⟧    incr numTests(Passed)
⟦TAB⟧    if {[IsVerbose pass]} {
⟦TAB⟧⟦TAB⟧puts [outputChannel] "++++ $name PASSED"
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧incr testLevel -1
⟦TAB⟧return
    }

    # We know the test failed, tally it  puts [outputChannel] "\n==== $name\
⟦TAB⟧    [string trim $description] FAILED"
    if {[string length $body]} {
⟦TAB⟧puts [outputChannel] "==== Contents of test case:"
⟦TAB⟧puts [outputChannel] $body
    }
    if {$setupFailure} {
⟦TAB⟧puts [outputChannel] "---- Test setup\
⟦TAB⟧⟦TAB⟧failed:\n$setupMsg"
⟦TAB⟧if {[info exists errorInfo(setup)]} {
⟦TAB⟧    puts [outputChannel] "---- errorInfo(setup): $errorInfo(setup)"
⟦TAB⟧    puts [outputChannel] "---- errorCode(setup): $errorCode(setup)"
⟦TAB⟧}
    }
    if {$scriptFailure} {
⟦TAB⟧if {$scriptCompare} {
⟦TAB⟧    puts [outputChannel] "---- Error testing result: $scriptMatch"
⟦TAB⟧} else {
⟦TAB⟧    puts [outputChannel] "---- Result was:\n$actualAnswer"
⟦TAB⟧    puts [outputChannel] "---- Result should have been\
⟦TAB⟧⟦TAB⟧    ($match matching):\n$result"
⟦TAB⟧}
    }
    if {$codeFailure} {
⟦TAB⟧switch -- $return    4 { set msg "Test generated continue exception" }
⟦TAB⟧    default { set msg "Test generated exception" }
⟦TAB⟧}
⟦TAB⟧puts [outputChannel] "---- $msg; Return code was: $returnCode"
⟦TAB⟧puts [outputChannel] "---- Return code should have been\
⟦TAB⟧⟦TAB⟧one of: $returnCodes"
⟦TAB⟧if {[IsVerbose error]} {
⟦TAB⟧    if {[info exists errorInfo(body)] && ([lsearch $returnCodes 1]<0)} {
⟦TAB⟧⟦TAB⟧puts [outputChannel] "---- errorInfo: $errorInfo(body)"
⟦TAB⟧⟦TAB⟧puts [outputChannel] "---- errorCode: $errorCode(body)"
⟦TAB⟧    }
⟦TAB⟧}
    }
    if {$outputFailure} {
⟦TAB⟧if {$outputCompare} {
⟦TAB⟧    puts [outputChannel] "---- Error testing output: $outputMatch"
⟦TAB⟧} else {
⟦TAB⟧    puts [outputChannel] "---- Output was:\n$outData"
⟦TAB⟧    puts [outputChannel] "---- Output should have been\
⟦TAB⟧⟦TAB⟧    ($match matching):\n$output"
⟦TAB⟧}
    }
    if {$errorFailure} {
⟦TAB⟧if {nel] \
⟦TAB⟧⟦TAB⟧"Test files run in separate interpreters"
    }
    if {[llength [skip]] > 0} {
⟦TAB⟧puts [outputChannel] "Skipping tests that match:  [skip]"
    }
    puts [outputChannel] "Running tests that mly running test files that match:  [matchFiles]"
    }

    set timeCmd {clock format [clock seconds]}
    puts [outputChannel] "Tests began at [eval $timeCmd]"

    # Run each of the specified tests
ce $file]
⟦TAB⟧} else {
⟦TAB⟧    # Pass along our configuration to the child processes.
⟦TAB⟧    # EXCEPT for the -outfile, because the parent process
⟦TAB⟧    # needs to read and process output of children.
⟦TAB⟧    set childargv [list]
⟦TAB⟧    foreach opt [Configure] {
⟦TAB⟧⟦TAB⟧if {[string equal $opt -outfile]} {continue}
⟦TAB⟧⟦TAB⟧lappend childargv $opt [Configure $opt]
⟦TAB⟧    }
⟦TAB⟧    set cmd [linsert $childargv 0 | $shell $file]
⟦TAB⟧    if {[catch {
⟦TAB⟧⟦TAB⟧incr numTestFiles
⟦TAB⟧⟦TAB⟧set pipeFd [open $cmd "r"]
⟦TAB⟧⟦TAB⟧while {[gets $pipeFd line] >= 0} {
⟦TAB⟧⟦TAB⟧    if {[regexp [join {
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {^([^:]+):\t}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {Total\t([0-9]+)\t}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {Passed\t([0-9]+)\t}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {Skipped\t([0-9]+)\t}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {Failed\t([0-9]+)}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    } ""] $line null testFile \
⟦TAB⟧⟦TAB⟧⟦TAB⟧    Total Passed Skipped Failed]} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧foreach index {Total Passed Skipped Failed} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧    incr numTests($index) [set $index]
⟦TAB⟧⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧⟦TAB⟧if {$Failed > 0} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧    lappend failFiles $testFile
⟦TAB⟧⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧    } elseif {[regexp [join {
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {^Number of tests skipped }
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {for each constraint:}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    {|^\t(\d+)\t(.+)$}
⟦TAB⟧⟦TAB⟧⟦TAB⟧    } ""] $line match skipped constraint]} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧if {[string match \t* $match]} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧    AddToSkippedBecause $constraint $skipped
⟦TAB⟧⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧⟦TAB⟧puts [outputChannel] $line
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧close $pipeFd
⟦TAB⟧    } msg]} {
⟦TAB⟧⟦TAB⟧puts [outputChannel] "Test file error: $msg"
⟦TAB⟧⟦TAB⟧# append the name of the test to a list to be reported
⟦TAB⟧⟦TAB⟧# later
⟦TAB⟧⟦TAB⟧lappend testFileFailures $file
⟦TAB⟧    }
⟦TAB⟧}
    }

    # cleanup
    puts [outputChannel] "\nTests ended at [eval $timeCmd]"
    cleanupTests 1
    if {[info exists testFileFailures]} {
⟦TAB⟧puts [outputChannel] "\nTest files exiting with errors:  \n"
⟦TAB⟧f⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 0, result:

```text-visible
child-provider {8.4.6 8.4 ::} module-code 1 module-result {invalid command name "pwd"} require-code 1 require-result {can't find package tcltest}⟦EOF⟧
```

- real-harness-explicit-source-child, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- real-harness-explicit-source-child: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 8

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
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

    # tests that use threads need to know which is the main thread
    Default mainThread 1
    variable mainThread
    if {[info commands thread::id] != {}} {
⟦TAB⟧set mainThread [thread::id]
    } elseif {[info commands testthread] != {}} {
⟦TAB⟧set mainThread [testthread id]
    }

    # Set workingDirectory to [pwd]. The default output directory for
    # Tcl tests is the working directory.  Whenever this value changes
    # change to that directory.
    variable workingDirectory
    trace variable workingDirectory w \
⟦TAB⟧    [namespace code {cd $workingDirectory ;#}]

    Default workingDirectory [pwd] AcceptAbsolutePath
    proc workingDirectory { {dir ""} } {
⟦TAB⟧variable workingDirectory
⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $workingDirectory
⟦TAB⟧}
⟦TAB⟧set workingDirectory [AcceptAbsolutePath $dir]
    }

    # Set the location of the execuatble
    Default tcltest [info nameofexecutable]
    trace variable tcltest w [namespace code {testConstraint stdio \
⟦TAB⟧    [eval [ConstraintInitializer stdio]] ;#}]

    # save the platform information so it can be restored later
    Default originalTclPlatform [array get ::tcl_platform]

    # If a core file exists, save its modification time.
    if {[file exists [file join [workingDirectory] core]]} {
⟦TAB⟧Default coreModTime \
⟦TAB⟧⟦TAB⟧[file mtime [file join [workingDirectory] core]]
    }

    # stdout and stderr buffers for use when we want to store them
    Default outData {}
    Default errData {}

    # keep track of test level for nested test commands
    variable testLevel 0

    # the variables and procs that existed when saveState was called are
    # stored in a variable of the same name
    Default saveState {}

    # Internationalization support -- used in [SetIso8859_1_Locale] and
    # [RestoreLocale]. Those commands are used in cmdIL.test.

    if {![info exists [namespace current]::isoLocale]} {
⟦TAB⟧variable isoLocale fr
⟦TAB⟧switch -- $::tcl_platform(platform) {
⟦TAB⟧    "unix" {

⟦TAB⟧⟦TAB⟧# Try some 'known' values for some platforms:

⟦TAB⟧⟦TAB⟧switch -exact -- $::tcl_platform(os) {
⟦TAB⟧⟦TAB⟧    "FreeBSD" {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr_FR.ISO_8859-1
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    HP-UX {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr_FR.iso88591
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    Linux -
⟦TAB⟧⟦TAB⟧    IRIX {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    default {

⟦TAB⟧⟦TAB⟧⟦TAB⟧# Works on SunOS 4 and Solaris, and maybe
⟦TAB⟧⟦TAB⟧⟦TAB⟧# others...  Define it to something else on your
⟦TAB⟧⟦TAB⟧⟦TAB⟧# system if you want to test those.

⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale iso_8859_1
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧    "windows" {
⟦TAB⟧⟦TAB⟧set isoLocale French
⟦TAB⟧    }
⟦TAB⟧}
    }

    variable ChannelsWeOpened; array set ChannelsWeOpened {}
    # output goes to stdout by default
    Default outputChannel stdout
    proc outputChannel { {filename ""} } {
⟦TAB⟧variable outputChannel
⟦TAB⟧variable ChannelsWeOpened

⟦TAB⟧# This is very subtle and tricky, so let me try to explain.
⟦TAB⟧# (Hopefully this longer comment will be clear when I come
⟦TAB⟧# back in a few months, unlike its predecessor :) )
⟦TAB⟧#⟦SPACE⟧
⟦TAB⟧# The [outputChannel] command (and underlying variable) have to
⟦TAB⟧# be kept in sync with the [configure -outfile] configuration
⟦TAB⟧# option ( and underlying variable Option(-outfile) ).  This is
⟦TAB⟧# accomplished with a write trace on Option(-outfile) that will
⟦TAB⟧# update [outputChannel] whenver a new value is written.  That
⟦TAB⟧# much is easy.
⟦TAB⟧#
⟦TAB⟧# The trick is that in order to maintain compatibility with
⟦TAB⟧# version 1 of tcltest, we must allow every configuration option
⟦TAB⟧# to get its inital value from command line arguments.  This is
⟦TAB⟧# accomplished by setting initial read traces on all the
⟦TAB⟧# configuration options to parse the command line option the first
⟦TAB⟧# time they are read.  These traces are cancelled whenever the
⟦TAB⟧# program itself calls [configure].
⟦TAB⟧#⟦SPACE⟧
⟦TAB⟧# OK, then so to support tcltest 1 compatibility, it seems we want
⟦TAB⟧# to get the return from [outputFile] to trigger the read traces,
⟦TAB⟧# just in case.
⟦TAB⟧#
⟦TAB⟧# BUT!  A little known feature of Tcl variable traces is that⟦SPACE⟧
⟦TAB⟧# traces are disabled during the handling of other traces.  So,
⟦TAB⟧# if we trigger read traces on Option(-outfile) and that triggers
⟦TAB⟧# command line parsing which turns around and sets an initial
⟦TAB⟧# value for Option(-outfile) -- <whew!> -- the write trace that
⟦TAB⟧# would keep [outputChannel] in sync with that new initial value
⟦TAB⟧# would not fire!
⟦TAB⟧#
⟦TAB⟧# SO, finally, as a workaround, instead of triggering read traces
⟦TAB⟧# by invoking [outputFile], we instead trigger the same set of
⟦TAB⟧# read traces by invoking [debug].  Any command that reads a
⟦TAB⟧# configuration option would do.  [debug] is just a handy one.
⟦TAB⟧# The end result is that we support tcltest 1 compatibility and
⟦TAB⟧# keep outputChannel and -outfile in sync in all cases.
⟦TAB⟧debug

⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $outputChannel
⟦TAB⟧}
⟦TAB⟧if {[info exists ChannelsWeOpened($outputChannel)]} {
⟦TAB⟧    close $outputChannel
⟦TAB⟧    unset ChannelsWeOpened($outputChannel)
⟦TAB⟧}
⟦TAB⟧switch -exact -- $filename {
⟦TAB⟧    stderr -
⟦TAB⟧    stdout {
⟦TAB⟧⟦TAB⟧set outputChannel $filename
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧set outputChannel [open $filename a]
⟦TAB⟧⟦TAB⟧set ChannelsWeOpened($outputChannel) 1

⟦TAB⟧⟦TAB⟧# If we created the file in [temporaryDirectory], then
⟦TAB⟧⟦TAB⟧# [cleanupTests] will delete it, unless we claim it was
⟦TAB⟧⟦TAB⟧# already there.
⟦TAB⟧⟦TAB⟧set outdir [normalizePath [file dirname \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [pwd] $filename]]]
⟦TAB⟧⟦TAB⟧if {[string equal $outdir [temporaryDirectory]]} {
⟦TAB⟧⟦TAB⟧    variable filesExisted
⟦TAB⟧⟦TAB⟧    FillFilesExisted
⟦TAB⟧⟦TAB⟧    set filename [file tail $filename]
⟦TAB⟧⟦TAB⟧    if {[lsearch -exact $filesExisted $filename] == -1} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧lappend filesExisted $filename
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $outputChannel
    }

    # errors go to stderr by default
    Default errorChannel stderr
    proc errorChannel { {filename ""} } {
⟦TAB⟧variable errorChannel
⟦TAB⟧variable ChannelsWeOpened

⟦TAB⟧# This is subtle and tricky.  See the comment above in
⟦TAB⟧# [outputChannel] for a detailed explanation.
⟦TAB⟧debug

⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $errorChannel
⟦TAB⟧}
⟦TAB⟧if {[info exists ChannelsWeOpened($errorChannel)]} {
⟦TAB⟧    close $errorChannel
⟦TAB⟧    unset ChannelsWeOpened($errorChannel)
⟦TAB⟧}
⟦TAB⟧switch -exact -- $filename {
⟦TAB⟧    stderr -
⟦TAB⟧    stdout {
⟦TAB⟧⟦TAB⟧set errorChannel $filename
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧set errorChannel [open $filename a]
⟦TAB⟧⟦TAB⟧set ChannelsWeOpened($errorChannel) 1

⟦TAB⟧⟦TAB⟧# If we created the file in [temporaryDirectory], then
⟦TAB⟧⟦TAB⟧# [cleanupTests] will delete it, unless we claim it was
⟦TAB⟧⟦TAB⟧# already there.
⟦TAB⟧⟦TAB⟧set outdir [normalizePath [file dirname \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [pwd] $filename]]]
⟦TAB⟧⟦TAB⟧if {[string equal $outdir [temporaryDirectory]]} {
⟦TAB⟧⟦TAB⟧    variable filesExisted
⟦TAB⟧⟦TAB⟧    FillFilesExisted
⟦TAB⟧⟦TAB⟧    set filename [file tail $filename]
⟦TAB⟧⟦TAB⟧    if {[lsearch -exact $filesExisted $filename] == -1} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧lappend filesExisted $filename
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $errorChannel
    }

##### Set up the configurable options
    #
    # The configurable options of the package
    variable Option; array set Option {}

    # Usage strings for those options
    variable Usage; array set Usage {}

    # Verification commands for those options
    variable Verify; array set Verify {}

    # Initialize the default values of the configurable options that are
    # historically associated with an exported variable.  If that variable
    # is already set, support compatibility by accepting its pre-set value.
    # Use [trace] to establish ongoing connection between the deprecated
    # exported variable and the modern option kept as a true internal var.
    # Also set up usage string and value testing for the option.
    proc Option {option value usage {verify AcceptAll} {varName {}}} {
⟦TAB⟧variable Option
⟦TAB⟧variable Verify
⟦TAB⟧variable Usage
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧set Usage($option) $usage
⟦TAB⟧set Verify($option) $verify
⟦TAB⟧if {[catch {$verify $value} msg]} {
⟦TAB⟧    return -code error $msg
⟦TAB⟧} else {
⟦TAB⟧    set Option($option) $msg
⟦TAB⟧}
⟦TAB⟧if {[string length $varName]} {
⟦TAB⟧    variable $varName
⟦TAB⟧    if {[info exists $varName]} {
⟦TAB⟧⟦TAB⟧if {[catch {$verify [set $varName]} msg]} {
⟦TAB⟧⟦TAB⟧    return -code error $msg
⟦TAB⟧⟦TAB⟧} else {
⟦TAB⟧⟦TAB⟧    set Option($option) $msg
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧unset $varName
⟦TAB⟧    }
⟦TAB⟧    namespace eval [namespace current] \
⟦TAB⟧    ⟦TAB⟧    [list upvar 0 Option($option) $varName]
⟦TAB⟧    # Workaround for Bug (now Feature Request) 572889.  Grrrr....
⟦TAB⟧    # Track all the variables tied to options
⟦TAB⟧    lappend OptionControlledVariables $varName
⟦TAB⟧    # Later, set auto-configure read traces on all
⟦TAB⟧    # of them, since a single trace on Option does not work.
⟦TAB⟧    proc $varName {{value {}}} [subst -nocommands {
⟦TAB⟧⟦TAB⟧if {[llength [info level 0]] == 2} {
⟦TAB⟧⟦TAB⟧    Configure $option [set value]
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧return [Configure $option]
⟦TAB⟧    }]
⟦TAB⟧}
    }

    proc MatchingOption {option} {
⟦TAB⟧variable Option
⟦TAB⟧set match [array names Option $option*]
⟦TAB⟧switch -- [llength $match] {
⟦TAB⟧    0 {
⟦TAB⟧⟦TAB⟧set sorted [lsort [array names Option]]
⟦TAB⟧⟦TAB⟧set values [join [lrange $sorted 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append values ", or [lindex $sorted end]"
⟦TAB⟧⟦TAB⟧return -code error "unknown option $option: should be\
⟦TAB⟧⟦TAB⟧⟦TAB⟧one of $values"
⟦TAB⟧    }
⟦TAB⟧    1 {
⟦TAB⟧⟦TAB⟧return [lindex $match 0]
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧# Exact match trumps ambiguity
⟦TAB⟧⟦TAB⟧if {[lsearch -exact $match $option] >= 0} {
⟦TAB⟧⟦TAB⟧    return $option
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧set values [join [lrange $match 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append values ", or [lindex $match end]"
⟦TAB⟧⟦TAB⟧return -code error "ambiguous option $option:\
⟦TAB⟧⟦TAB⟧⟦TAB⟧could match $values"
⟦TAB⟧    }
⟦TAB⟧}
    }

    proc EstablishAutoConfigureTraces {} {
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧foreach varName [concat $OptionControlledVariables Option] {
⟦TAB⟧    variable $varName
⟦TAB⟧    trace variable $varName r [namespace code {ProcessCmdLineArgs ;#}]
fromdir {} {
⟦TAB⟧Skip tests in directories that match the glob pattern given.
    } AcceptPattern skipDirectories

    # By default, don't save core files
    Option -preservecore 0 {
⟦TAB⟧If 2, save any corfault; debug level 1 spits
    # up only the tests that were skipped because they didn't match or
    # were specifically skipped.  A debug level of 2 would spit up the
    # tcltest variables and flags provided; a debug level of 3 causes
    # some additional output regarding operations of the test harness.
    # The tcltest package currently implements only up to debug level 3.
    Option -debug 0 {
⟦TAB⟧Internal debug level⟦SPACE⟧
    } AcceptInteger debug

    proc SetSelectedConstraints args {
⟦TAB⟧variable Option
⟦TAB⟧foreach c $Option(-constraints) {
⟦TAB⟧    testConstraint $c 1
⟦TAB⟧}
    }
    Option -constraints {} {
⟦TAB⟧Do not skip the listed constraints listed in -constraints.
    } AcceptList
    trace variable Option(-constraints) w \
⟦TAB⟧    [namespace code {SetSelectedConstraints ;#}]

    # Don't run only the "-constraint" specified tests by default
    proc ClearUnselectedConstraints args {
⟦TAB⟧variable Option
⟦TAB⟧variable testConstraints
⟦TAB⟧if {!$Option(-limitconstraints)} {return}
⟦TAB⟧foreach c [array names testConstraints] {
⟦TAB⟧    if {[lsearch -exact $Option(-constraints) $c] == -1} {
⟦TAB⟧⟦TAB⟧testConstraint $c 0
⟦TAB⟧    }
⟦TAB⟧}
    }
    Option -limitconstraints false {
⟦TAB⟧whether to run only tests with the constraints
    } AcceptBoolean limitConstraints⟦SPACE⟧
    trace variable Option(-limitconstraints) w \
⟦TAB⟧    [namespace code {ClearUnselectedConstraints ;#}]

    # A test application has to know how to load the tested commands
    # into the interpreter.
    Option -load {} {
⟦TAB⟧Specifies the script to load the tested commands.
    } AcceptScript loadScript

    # Default is to run each test file in a separate prp]
⟦TAB⟧close $tmp
    }
    Option -loadfile {} {
⟦TAB⟧Read the script to load the tested commands from the specified file.
    } AcceptLoadFile loadFile
    trace variable Option(-loadfile) w [namespace code
    trace variable Option(-errfile) w \
⟦TAB⟧    [namespace code {errorChannel $Option(-errfile) ;#}]

}

#####################################################################

# tcltest::Debug* --
#
#     Internal helper procedures to write out debug information
#     dependent on the chosen level. A test shell may overide
#     them, f.e. to redirect the output into a different
#     channel, or even into a GUI.

# tcltest::DebugPuts --
#
#     Prints the specified string if the current debug level is
#     higher than the provided level argument.
#
# Arguments:
#     level   The lowest debug level triggering the output
#     string  The string to print out.
#
# Results:
#     Prints the string. Nothing else is allowed.
#
# Side Effects:
#     None.
#

proc tcltest::DebugPuts {level string} {
    variable debug
    if {$debug >= $level} {
⟦TAB⟧puts $string
    }
    return
}

# tcltest::DebugPArray --
#
#     Prints the contents of the specified array if the current
#       debug level is and
# [info default], but can't be bothered now.  If [parray] changes, then
# this will need changing too.
auto_load ::parray
proc tcltest::parray {a {pattern *}} [info body ::parray]

# tcltest::DebugDo --
#
#     Executes the script if the current debug level is greater than
#       the provided level argument
#
# Arguments:
#     level   The lowest debug level triggering the execution.
#     script  The tcl script executed upon a debug level high enough.
#
# Results:
#     Arbitrary side effects, dependent on the executed script.
#
# Side Effects:
#     None.
#

proc tcltest::DebugDo {level script} {
    variable debug

    if {$debug >= $level} {
⟦TAB⟧uplevel 1 $script
    }
    return
}

#####################################################################

proc tcltest::Warn {msg} {
    == -1} {
⟦TAB⟧set value 0
    }
    set testConstraints($constraint) $value
}

# tcltest::interpreter --
#
#⟦TAB⟧the interpreter name stored in tcltest::tcltest
#
# Arguments:
#⟦TAB⟧executable name
#
# Results:
#set moreOptions [processCmdLineArgsAddFlagsHook]
⟦TAB⟧⟦TAB⟧if {[lsearch -exact $moreOptions $option] == -1} {
⟦TAB⟧⟦TAB⟧    # Nope.  Report the error, including additional options,
⟦TAB⟧⟦TAB⟧    # but keep going
⟦TAB⟧⟦TAB⟧    if {[llength $moreOptions]} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧append msg ", "
⟦TAB⟧⟦TAB⟧⟦TAB⟧append msg [join [lrange $moreOptions 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧⟦TAB⟧append msg "or [lindex $moreOptions end]"
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    Warn $msg
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧# error is something other than "unknown option"
⟦TAB⟧⟦TAB⟧# notify user of the error; and exit
⟦TAB⟧⟦TAB⟧puts [errorChannel] $msg
⟦TAB⟧⟦TAB⟧exit 1
⟦TAB⟧    }

⟦TAB⟧    # To recover, find that unknown option and remove up to it.
⟦TAB⟧    # then retry
⟦TAB⟧    while {![string equal [lindex $args 0] $option]} {
⟦TAB⟧⟦TAB⟧set args [lrange $args 2 end]
⟦TAB⟧    }
⟦TAB⟧    set args [lrange $args 2 end]
⟦TAB⟧}
⟦TAB⟧if {[llength $args] == 1} {
⟦TAB⟧    puts [errorChannel] \
⟦TAB⟧⟦TAB⟧    "missing value for option [lindex $args 0]"
⟦TAB⟧    exit 1
⟦TAB⟧}
    }

    # Call the hook
    catch {
        array set flag $flagArray
        processCmdLineArgsHook [array get flag]
    }
    return
}

# tcltest::ProcessCmdLineArgs --
#
#       This procedure must be run after constraint initialization is
#⟦TAB⟧set up (by [DefineConstraintInitializers]) because some constraints
#⟦TAB⟧can be overridden.
#
#       Perform configuration according to the command-line options.
#
# Arguments:
#⟦TAB⟧none
#
# Results:
#⟦TAB⟧Sets the above-named variables in the tcltest namespace.
#
# Side Effects:
#       None.
#

proc tcltest::ProcessCmdLineArgs {} {
    variable originalEnv
    variable testConstraints

    # The "argv" var doesn't exist in some cases, so use {}.
    if {![info exists ::argv]} {
⟦TAB⟧ProcessFlags {}
    } else {
⟦TAB⟧ProcessFlags $::argv
    }

    # Spit out everything you know if we're at a debug level 2 or
    # greater
    DebugPuts 2 "Flags passed into tcltest:"
    if {[info exists ::env(TCLTEST_'t evaluate after completion"
    }
    set CustomMatch($mode) $script
}

# tcltest::SubstArguments list
#
# This helper function takes in a list of words, then perform a
# substitution on the list asnvoked as:
#
#      SubstArguments $a {$a}
#
# This code is adapted from Paul Duffin's function "SplitIntoWords".
# The original function can be found  on:
#
#      http://purl.org/thecliff/tcl/wiki/858.html
#
# Results:
#     a list containing the result of the substitution
#
# Exceptions:
#     An error may occur if the list containing unbalanced quote or
#     unknown variable.
#
# Side Effects # Look for the next word containing a quote: " { }
        if {[regexp -indices {[^ \t\n]*[\"\{\}]+[^ \t\n]*} \
⟦TAB⟧⟦TAB⟧$argList all]} {
            # Get the text leading up to this word, but not including
⟦TAB⟧    # this word, from the argList.
            set text [string range $argList 0 \
⟦TAB⟧⟦TAB⟧    [expr {[lindex $all 0] - 1}]]
            # Get the word with the quote
            set word [string range $argList \
                    [lindex $all 0] [lindex $all 1]]

            # Remove all text up to and including the word from the
            # argList.
            set argList [string range $argList \
                    [expr {[lindex $all 1] + 1}] end]
        } else {
            # Take everything up to the end of the argList.
            set text $argList
            set word {}
            set argList {}
        }

        if {$token != {}} {
            # If we saw a word with quote before, then there is a
            # multi-word token starting with that word.  In this case,
            # add the text and the current word to this token.
            append token $text $word
        } else {
            # Add the text to the result.  There is no need to parse
            # the text because it couldn't be a part of any multi-word
            # token.  Then start a new multi-word token with the word
            # because we need to pass this token to the Tcl parser to
           ed. This attribute is
#⟦TAB⟧⟦TAB⟧⟦TAB⟧optional; default is {}
#   body -⟦TAB⟧        Script to run to carry out the test.  It must
#⟦TAB⟧⟦TAB⟧        return a result that can be checked for
#⟦TAB⟧⟦TAB⟧        correctness.  This attribute is optional;
#                       default is {}
#   result -⟦TAB⟧        Expected result from script.  This attribute is
#                       optional; default is {}.
#   output -            Expected output sent to stdout.  This attribute
#                       is optional; default is {}.
#   errorOutput -       Expected output sent to stderr.  This attribute
#                       is optional; default is {}.
#   returnCodes -       Expected return codes.  This attribute is
#                       optional; default is {0 2}.
#   setup -             Code to run before $script (above).  This
#                       attribute is optional; default is {}.
#   cleanup -           Code to run after $script (above).  This
#                       attribute is optional; default is {}.
#   match -             specifies type of matching to do on result,
#                       output, errorOutput; this must be a string
#⟦TAB⟧⟦TAB⟧⟦TAB⟧previously registered by a call to [customMatch].
#⟦TAB⟧⟦TAB⟧⟦TAB⟧The strings exact, glob, and regexp are pre-registered
#⟦TAB⟧⟦TAB⟧⟦TAB⟧by the tcltest package.  Default value is exact.
#
# Arguments:
#   name -⟦TAB⟧⟦TAB⟧Name of test, in the form foo-1.2.
#   description -⟦TAB⟧Short textual description of the test, to
#  ⟦TAB⟧⟦TAB⟧  ⟦TAB⟧help humans understand what it does.
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#       Just about anything is possible depending on the test.
#

proc tcltest::test {name description args} {
    global tcl_platform
    variable testLevel
    variable coreModTime
    DebugPuts 3 "test $name $args"
    DebugDo 1 {
⟦TAB⟧variable TestNames
⟦TAB⟧catch {
⟦TAB⟧    puts "test name '$name' re-used; prior use in $TestNames($name)"
⟦TAB⟧}
⟦TAB⟧set TestNames($name) [info script]
    }

    FillFilesExisted
    incr testLevel

    # Pre-define everything to null except output and errorOutput.  We
    # Channel] "---- $name start"
⟦TAB⟧    flush [outputChannel]
⟦TAB⟧}

⟦TAB⟧set command [list [namespace origin RunTest] $name $body]
⟦TAB⟧if {[info exists output] || [info exists errorOutput]} {
⟦TAB⟧    set testResult [uplevel 1 [list [namespace origin Eval] $command 0]]
⟦TAB⟧} else {
⟦TAB⟧    set testResult [uplevel 1 [list [namespace origin Eval] $command 1]]
⟦TAB⟧}
⟦TAB⟧foreach {actualAnswer returnCode} $testResult break
⟦TAB⟧if {$returnCode == 1} {
⟦TAB⟧    set errorInfo(body) $::errorInfo
⟦TAB⟧    set errorCode(body) $::errorCode
⟦TAB⟧}
    }

    # Always run the cleanup script
    set code [catch {uplevel 1 $cleanup} cleanupMsg]
    if {$code == 1} {
⟦TAB⟧set errorInfo(cleanup) $::errorInfo
⟦TAB⟧set errorCode(cleanup) $::errorCode
    }
    set cleanupFailure [expr {$code != 0}]

    set coreFailure 0
    set coreMsg ""
    # check for a core file first - if one was created by the test,
    # then the test failed
    if {[preserveCore]} {
⟦TAB⟧if {[file exists [file join [workingDirectory] core]]} {
⟦TAB⟧    # There's only a test failure if there is a core file
⟦TAB⟧    # and (1) there previously wasn't one or (2) the new
⟦TAB⟧    # one is different from the old one.
⟦TAB⟧    if {[info exists coreModTime]} {
⟦TAB⟧⟦TAB⟧if {$coreModTime != [file mtime \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [workingDirectory] core]]} {
⟦TAB⟧⟦TAB⟧    set coreFailure 1
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧set coreFailure 1
⟦TAB⟧    }
⟦TAB⟧
⟦TAB⟧    if {([preserveCore] > 1) && ($coreFailure)} {
⟦TAB⟧⟦TAB⟧append coreMsg "\nMoving file to:\
⟦TAB⟧⟦TAB⟧    [file join [temporaryDirectory] core-$name]"
⟦TAB⟧⟦TAB⟧catch {file rename -force \
⟦TAB⟧⟦TAB⟧    [file join [workingDirectory] core] \
⟦TAB⟧⟦TAB⟧    [file join [temporaryDirectory] core-$name]
⟦TAB⟧⟦TAB⟧} msg
⟦TAB⟧⟦TAB⟧if {[string length $msg] > 0} {
⟦TAB⟧⟦TAB⟧    append coreMsg "\nError:\
⟦TAB⟧⟦TAB⟧⟦TAB⟧Problem renaming core file: $msg"
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
    }

    # check if the return code matched the expected return code
    set codeFailure 0
    if {!$setupFailure && [lsearch -exact $returnCodes $returnCode] == -1} {
⟦TAB⟧set codeFailure 1
    }

    # If expected output/error strings exist, we have to compare
    # them.  If the comparison fails, then so dimpare [catch {
⟦TAB⟧CompareStrings $actualAnswer $result $match
    } scriptMatch]] == 0} {
⟦TAB⟧set scriptFailure [expr {!$scriptMatch}]
    } else {
⟦TAB⟧set scriptFailure 1
    }

    # if we didn't experience any failures, then we passed
    variable numTests
    if {!($setupFailure || $cleanupFailure || $coreFailure
⟦TAB⟧    || $outputFailure || $errorFailure || $codeFailure
⟦TAB⟧    || $scriptFailure)} {
⟦TAB⟧if {$testLevel == 1} {
⟦TAB⟧    incr numTests(Passed)
⟦TAB⟧    if {[IsVerbose pass]} {
⟦TAB⟧⟦TAB⟧puts [outputChannel] "++++ $name PASSED"
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧incr testLevel -1
⟦TAB⟧return
    }

    # We know the test failed, tally it...
    if {$testLevel == 1} {
⟦TAB⟧incr numTests(Failed)
    }

    # ... then report according to the type of failure
    variable currentFailure true
    if {![IsVerbose body]} {
⟦TAB⟧set body ""
    }⟦TAB⟧
    puts [outputChannel] "\n==== $name\
⟦TAB⟧    [string trim $description] FAILED"
    if {[string length $body]} {
⟦TAB⟧puts [outputChannel] "==== Contents of test case:"
⟦TAB⟧puts [outputChannel] $body
    }
    if {$setupFailure} {
⟦TAB⟧puts [outputChannel] "---- Test setup\
⟦TAB⟧⟦TAB⟧failed:\n$setupMsg"
⟦TAB⟧if {[info exists errorInfo(setup)]} {
⟦TAB⟧    puts [outputChannel] "---- errorInfo(setup): $errorInfo(setup)"
⟦TAB⟧    puts [outputChannel] "---- errorCode(setup): $errorCode(setup)"
⟦TAB⟧}
    }
    if {$scriptFailure} {
⟦TAB⟧if {$scriptCompare} {
⟦TAB⟧    puts [outputChannel] "---- Error testing result: $scriptMatch"
⟦TAB⟧} else {
⟦TAB⟧    puts [outputChannel] "---- Result was:\n$actualAnswer"
⟦TAB⟧    puts [outputChannel] "---- Result should have been\
⟦TAB⟧⟦TAB⟧    ($match matching):\n$result"
⟦TAB⟧}
    }
    if {$codeFailure} {
⟦TAB⟧switch -- $returnCode {
⟦TAB⟧    0 { set msg "Test completed normally" }
⟦TAB⟧    1 { set msg "Test generated error" }
⟦TAB⟧    2 { set msg "Test generated return exception" }
⟦TAB⟧    3 { set msg "Test generated break exception" }
⟦TAB⟧    4 { set msg "Test generated continue exception" }
⟦TAB⟧    default { set msg "Test generated exception" }
⟦TAB⟧}
⟦TAB⟧puts [outputChannel] "---- $msg; Return code was: $returnCode"
⟦TAB⟧puts [outputChannel] "----rInfo(body)"
⟦TAB⟧⟦TAB⟧puts [outputChannel] "---- errorCode: $errorCode(body)"
⟦TAB⟧    }
⟦TAB⟧}
    }
    if {$outputFailure} {
⟦TAB⟧if {$outputCompare} {
⟦TAB⟧    puts [outputChannel] "---- Error testing output: $outputMat$errorCompare} {
⟦TAB⟧    puts [outputChannel] "---- Error testing errorOutput: $errorMatch"
⟦TAB⟧} else {
⟦TAB⟧    puts [outputChannel] "---- Error output was:\n$errData"
⟦TAB⟧    puts [outputChannel] "---- Error output should have\
⟦TAB⟧⟦TAB⟧    been ($match matching):\n$errorOutput"
⟦TAB⟧}
    }
    if {$cleanupFailure} {
⟦TAB⟧puts [outputChannel] "---- Test cleanup failed:\n$cleanupMsg"
⟦TAB⟧if {[info exists errorInfo(cleanup)]} {
⟦TAB⟧    puts [outputChannel] "---- errorInfo(cleanup): $errorInfo(cleanup)"
⟦TAB⟧    puts [outputChannel] "---- errorCode(cleanup): $errorCode(cleanup)"
⟦TAB⟧}
    }
    if {$coreFailure} {
⟦TAB⟧puts [outputChannel] "---- Core file produced while running\
⟦TAB⟧⟦TAB⟧test!  $coreMsg"
    }
    puts [outputChannel] "==== $name FAILED\n"

    incr testLevel -1
    return
}

# Skipped --
#
# Given a test name and it constraints, returns a boolean indicating
# whether the current configuration says the test should be skipped.
#
# Side Effects:  Maintains tally of total tests seen and tests skipped.
#
proc tcltest::Skipped {name constraints} {
    variable testLevel
    variable numTests
    variable testConstraints

    if {$testLevel == 1} {
⟦TAB⟧incr numTests(Total)
    }
    # skip the test if it's name matches an element of skip
    foreach pattern [skip] {
⟦TAB⟧if {[string match $pattern $name]} {
⟦TAB⟧    if {$testLevel == 1} {
⟦TAB⟧⟦TAB⟧incr numTests(Skipped)
⟦TAB⟧⟦TAB⟧DebugDo 1 {AddToSkippedBecause userSpecifiedSkip}
⟦TAB⟧    }
⟦TAB⟧    return 1
⟦TAB⟧}
    }
    # skip the test if it's name doesn't match any element of match
    set ok 0
    foreach pattern [match] {
⟦TAB⟧if {[string match $pattern $name]} {
⟦TAB⟧    set ok 1
⟦TAB⟧    break
⟦TAB⟧}
    }
    if {!$ok} {
⟦TAB⟧if {$testLevel == 1} {
⟦TAB⟧    incr numTests(Skipped)
⟦TAB⟧    DebugDo 1 {AddToSkippedBecause userSpecifiedNonMatch}
⟦TAB⟧}
⟦TAB⟧return 1
    }
    if {[string equal {} $constraints]} {
⟦TAB⟧# If we're mTests
    variable numTestFiles
    variable failFiles
    variable skippedBecause
    variable currentFailure
    variable originalEnv
    variable originalTclPlatform
    variable coreModTime

    ocedures.  Record the names of files in
    # workingDirectory that were not pre-existing, and associate them
    # with the test file that created them.

    if {!$calledFromAllFile} {
⟦TAB⟧foreach file $filesMade {
⟦TAB⟧    if {[file exists $file]} {
⟦TAB⟧⟦TAB⟧DebugDo 1 {Warn "cleanupTests deleting $file..."}
⟦TAB⟧⟦TAB⟧catch {file delete -force $file}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set currentFiles {}
⟦TAB⟧foreach file [glob -nocomplain \
⟦TAB⟧⟦TAB⟧-directory [temporaryDirectory] *] {
⟦TAB⟧    lappend currentFiles [file tail $file]
⟦TAB⟧}
⟦TAB⟧set newFiles {}
⟦TAB⟧foreach file $currentFiles {
⟦TAB⟧    if {[lsearch -exact $filesExisted $file] == -1} {
⟦TAB⟧⟦TAB⟧lappend newFiles $file
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set filesExisted $currentFiles
⟦TAB⟧if {[llength $newFiles] > 0} {
⟦TAB⟧    set createdNewFiles($testFileName) $newFiles
⟦TAB⟧}
    }

    if {$calledFromAllFile || $testSingleFile} {

⟦TAB⟧# print stats

⟦TAB⟧puts -nonewline [outputChannel] "$testFileName:"
⟦TAB⟧foreach index [list "Total" "Passed" "Skipped" "Failed"] {
⟦TAB⟧    puts -nonewline [outputChannel] \
⟦TAB⟧⟦TAB⟧    "\t$index\t$numTests($index)"
⟦TAB⟧}
⟦TAB⟧puts [outputChannel] ""

⟦TAB⟧# print number test files sourced
⟦TAB⟧# print names of files that ran tests which failed

⟦TAB⟧if {$calledFromAllFile} {
⟦TAB⟧    puts [outputChannel] \
⟦TAB⟧⟦TAB⟧    "Sourced $numTestFiles Test Files."
⟦TAB⟧    set numTestFiles 0
⟦TAB⟧    if {[llength $failFiles] > 0} {
⟦TAB⟧⟦TAB⟧puts [outputChannel] \
⟦TAB⟧⟦TAB⟧⟦TAB⟧"Files with failing tests: $failFiles"
⟦TAB⟧⟦TAB⟧set failFiles {}
⟦TAB⟧    }
⟦TAB⟧}

⟦TAB⟧# if any tests were skipped, print the constraints that kept
⟦TAB⟧# them from running.

⟦TAB⟧set constraintList [array names skippedBecause]
⟦TAB⟧if {[llength $constraintList] > 0} {
⟦TAB⟧    puts [outputChannel] \
⟦TAB⟧⟦TAB⟧    "Number of tests skipped for each constraint:"
⟦TAB⟧    foreach constraint [lsort $constraintList] {
⟦TAB⟧⟦TAB⟧puts [outputChannel] \
⟦TAB⟧⟦TAB⟧⟦TAB⟧"\t$skippedBecause($constraint)\t$constraint"
⟦TAB⟧⟦TAB⟧unset skippedBecause($constraint)
⟦TAB⟧    }
⟦TAB⟧}

⟦TAB⟧# report }
⟦TAB⟧}

⟦TAB⟧# reset filesMade, filesExisted, and numTests

⟦TAB⟧set filesMade {}
⟦TAB⟧foreach index [list "Total" "Passed" "Skipped" "Failed"] {
⟦TAB⟧    set numTests($index) 0
⟦TAB⟧}

⟦TAB⟧# exit only if running Tk in non-irrent file to failFile list if any tests in this
⟦TAB⟧# file failed

⟦TAB⟧if {$currentFailure \
⟦TAB⟧⟦TAB⟧&& ([lsearch -exact $failFiles $testFileName] == -1)} {
⟦TAB⟧    lappend failFiles $testFileName
⟦TAB⟧}
⟦TAB⟧set currentFailure false

⟦TAB⟧# restore the environment to the state it was in before this package
⟦TAB⟧# was loaded

⟦TAB⟧set newEnv {}
⟦TAB⟧set changedEnv {}
⟦TAB⟧set removedEnv {}
⟦TAB⟧foreach index [array names ::env] {
⟦TAB⟧    if {![info exists originalEnv($index)]} {
⟦TAB⟧⟦TAB⟧lappend newEnv $index
⟦TAB⟧⟦TAB⟧unset ::env($index)
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧if {$::env($index) != $originalEnv($index)} {
⟦TAB⟧⟦TAB⟧    lappend changedEnv $index
⟦TAB⟧⟦TAB⟧    set ::env($index) $originalEnv($index)
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧foreach index [array names originalEnv] {
⟦TAB⟧    if {![info exists ::env($index)]} {
⟦TAB⟧⟦TAB⟧lappend removedEnv $index
⟦TAB⟧⟦TAB⟧set ::env($index) $originalEnv($index)
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧if {[llength $newEnv] > 0} {
⟦TAB⟧    puts [outputChannel] \
⟦TAB⟧⟦TAB⟧    "env array elements created:\t$newEnv"
⟦TAB⟧}
⟦TAB⟧if {[llength $changedEnv] > 0} {
⟦TAB⟧    puts [outputChannel] \
⟦TAB⟧⟦TAB⟧    "env array elements changed:\t$changedEnv"
⟦TAB⟧}
⟦TAB⟧if {[llength $removedEnv] > 0} {
⟦TAB⟧    puts [outputChannel] \
⟦TAB⟧⟦TAB⟧    "env array elements removed:\t$removedEnv"
⟦TAB⟧}

⟦TAB⟧set changedTclPlatform {}
⟦TAB⟧foreach index [array names originalTclPlatform] {
⟦TAB⟧    if {$::tcl_platform($index) \
⟦TAB⟧⟦TAB⟧    != $originalTclPlatform($index)} {
⟦TAB⟧⟦TAB⟧lappend changedTclPlatform $index
⟦TAB⟧⟦TAB⟧set ::tcl_platform($index) $originalTclPlatform($index)
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧if {[llength $changedTclPlatform] > 0} {
⟦TAB⟧    puts [outputChannel] "tcl_platform array elements\
⟦TAB⟧⟦TAB⟧    changed:\t$changedTclPlatform"
⟦TAB⟧}

⟦TAB⟧if {[file exists [file join [workingDirectory] core]]} {
⟦TAB⟧    if {[preserveCore] > 1} {
⟦TAB⟧⟦TAB⟧puts "rename core file (> 1)"
⟦TAB⟧⟦TAB⟧puts [outputChannel] "produced core file! \
⟦TAB⟧⟦TAB⟧⟦TAB⟧Moving file to: \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [temporaryDirectory] core-$testFileName]"
⟦TAB⟧⟦TAB⟧catch {file rename -forcereach path [glob -directory $rootdir -types d -nocomplain -- \
⟦TAB⟧⟦TAB⟧$pattern] {
⟦TAB⟧    if {[lsearch -exact $skipDirs $path] == -1} {
⟦TAB⟧⟦TAB⟧set matchDirs [concat $matchDirs [GetMatchingDirectories $path]]
⟦TAB⟧⟦TAB⟧if {[file exists [file join $path all.tcl]]} {
⟦TAB⟧⟦TAB⟧    lappend matchDirs $path
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
    }

    if {[llength $matchDirs] == 0} {
⟦TAB⟧DebugPuts 1 "No test directories remain after applying match\
⟦TAB⟧⟦TAB⟧and skip patterns!"
    }
    return $matchDirs
}

# tcltest::runAllTests --
#
#⟦TAB⟧prints output and sources test files according to the match and
#⟦TAB⟧skip patterns provided.  after sourcing test files, it goes on
#⟦TAB⟧to source all.tcl files in matching test subdirectories.
#
# Arguments:
#⟦TAB⟧shell being tested
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::runAllTests { {shell ""} } {
    variable testSingleFile
    variable numTestFiles
    variable numTests
    variable failFiles

    FillFilesExisted
    if {[llength [info level 0]] == 1} {
⟦TAB⟧set shell [interpreter]
    }

    set testSingleFile false

    puts [outputChannel] "Tests running in interp:  $shell"
    puts [outputChannel] "Tests located in:  [testsDirectory]"
    puts [outputChannel] "Tests running in:  [workingDirectory]"
    puts [outputChannel] "Temporary files stored in\
⟦TAB⟧    [temporaryDirectory]"

    # [file system] first available in Tcl 8.4
    if {![catch {file system [testsDirectory]} result]
⟦TAB⟧    && ![string equal native [lindex $result 0]]} {
⟦TAB⟧# If we aren't running in the native filesystem, then we must
⟦TAB⟧# run the tests in a single process (via 'source'), because
⟦TAB⟧# trying to run then via a pipe will fail since the files don't
⟦TAB⟧# really exist.
⟦TAB⟧singleProcess 1
    }

    if {[singleProcess]} {
⟦TAB⟧puts [outputChannel] \
⟦TAB⟧⟦TAB⟧"Test files sourced into current interpreter"
    } else {
⟦TAB⟧puts [outputChannel] \
⟦TAB⟧⟦TAB⟧"Test files run in separate interpreters"
    }
    if {[llength [skip]] > 0} {
⟦TAB⟧puts [outputChannel] "Skipping tests that match:  [skip]"
    }
    puts [outputChannel] "Running tests that msync $tid {thread::exit}}
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧    ## Enter a bit a sleep to give the threads enough breathing
⟦TAB⟧    ## room to kill themselves off, otherwise the end up with a
⟦TAB⟧    ## massive queue of repeated events
⟦TAB⟧    after 1
⟦TAB⟧}
⟦TAB⟧thread::errorproc ThreadError
⟦TAB⟧return [llength [thread::names]]
    } else {
⟦TAB⟧return 1
    }
    return 0
}

# Initialize the constraints and set up command line arguments
namespace eval tcltest {
    # Define initializers for all the built-in contraint definitions
    DefineConstraintInitializers

    # Set up the constraints in the testConstraints array to be lazily
    # initialized by a registered initializer, or by "false" if no
    # initializer is registered.
    trace variable testConstraints r [namespace code SafeFetch]

    # Only initialize constraints at package load time if an
    # [initConstraintsHook] has been pre-defined.  This is only
    # for compatibility support.  The modern way to add a custom
    # test constraint is to just call the [testConstraint] command
    # straight away, without all this "hook" nonsense.
    if {[string equal [namespace current] \
⟦TAB⟧    [namespace qualifiers [namespace which initConstraintsHook]]]} {
⟦TAB⟧InitConstraints
    } else {
⟦TAB⟧proc initConstraintsHook {} {}
    }

    # Define the standard match commands
    customMatch exact⟦TAB⟧[list string equal]
    customMatch glob⟦TAB⟧[list string match]
    customMatch regexp⟦TAB⟧[list regexp --]

    # If the TCLTEST_OPTIONS environment variable exists, configure
    # tcltest according to the option values it specifies.  This has
    # the effect of resetting tcltest's default configuration.
    proc ConfigureFromEnvironment {} {
⟦TAB⟧upvar #0 env(TCLTEST_OPTIONS) options
⟦TAB⟧if {[catch {llength $options} msg]} {
⟦TAB⟧    Warn "invalid TCLTEST_OPTIONS \"$options\":\n  invalid\
⟦TAB⟧⟦TAB⟧    Tcl list: $msg"
⟦TAB⟧    return
⟦TAB⟧}
⟦TAB⟧if {[llength $::env(TCLTEST_OPTIONS)] % 2} {
⟦TAB⟧    Warn "invalid TCLTEST_OPTIONS: \"$options\":\n  should be\
⟦TAB⟧⟦TAB⟧    -option value ?-option value ...?"
⟦TAB⟧    return
⟦TAB⟧⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 0, result:

```text-visible
child-provider {8.4.6 8.4 ::} module-code 1 module-result {invalid command name "pwd"} require-code 1 require-result {can't find package tcltest}⟦EOF⟧
```

- real-harness-explicit-source-child, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- real-harness-explicit-source-child: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 9

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
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

    # tests that use threads need to know which is the main thread
    Default mainThread 1
    variable mainThread
    if {[info commands thread::id] != {}} {
⟦TAB⟧set mainThread [thread::id]
    } elseif {[info commands testthread] != {}} {
⟦TAB⟧set mainThread [testthread id]
    }

    # Set workingDirectory to [pwd]. The default output directory for
    # Tcl tests is the working directory.  Whenever this value changes
    # change to that directory.
    variable workingDirectory
    trace variable workingDirectory w \
⟦TAB⟧    [namespace code {cd $workingDirectory ;#}]

    Default workingDirectory [pwd] AcceptAbsolutePath
    proc workingDirectory { {dir ""} } {
⟦TAB⟧variable workingDirectory
⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $workingDirectory
⟦TAB⟧}
⟦TAB⟧set workingDirectory [AcceptAbsolutePath $dir]
    }

    # Set the location of the execuatble
    Default tcltest [info nameofexecutable]
    trace variable tcltest w [namespace code {testConstraint stdio \
⟦TAB⟧    [eval [ConstraintInitializer stdio]] ;#}]

    # save the platform information so it can be restored later
    Default originalTclPlatform [array get ::tcl_platform]

    # If a core file exists, save its modification time.
    if {[file exists [file join [workingDirectory] core]]} {
⟦TAB⟧Default coreModTime \
⟦TAB⟧⟦TAB⟧[file mtime [file join [workingDirectory] core]]
    }

    # stdout and stderr buffers for use when we want to store them
    Default outData {}
    Default errData {}

    # keep track of test level for nested test commands
    variable testLevel 0

    # the variables and procs that existed when saveState was called are
    # stored in a variable of the same name
    Default saveState {}

    # Internationalization support -- used in [SetIso8859_1_Locale] and
    # [RestoreLocale]. Those commands are used in cmdIL.test.

    if {![info exists [namespace current]::isoLocale]} {
⟦TAB⟧variable isoLocale fr
⟦TAB⟧switch -- $::tcl_platform(platform) {
⟦TAB⟧    "unix" {

⟦TAB⟧⟦TAB⟧# Try some 'known' values for some platforms:

⟦TAB⟧⟦TAB⟧switch -exact -- $::tcl_platform(os) {
⟦TAB⟧⟦TAB⟧    "FreeBSD" {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr_FR.ISO_8859-1
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    HP-UX {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr_FR.iso88591
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    Linux -
⟦TAB⟧⟦TAB⟧    IRIX {
⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale fr
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    default {

⟦TAB⟧⟦TAB⟧⟦TAB⟧# Works on SunOS 4 and Solaris, and maybe
⟦TAB⟧⟦TAB⟧⟦TAB⟧# others...  Define it to something else on your
⟦TAB⟧⟦TAB⟧⟦TAB⟧# system if you want to test those.

⟦TAB⟧⟦TAB⟧⟦TAB⟧set isoLocale iso_8859_1
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧    "windows" {
⟦TAB⟧⟦TAB⟧set isoLocale French
⟦TAB⟧    }
⟦TAB⟧}
    }

    variable ChannelsWeOpened; array set ChannelsWeOpened {}
    # output goes to stdout by default
    Default outputChannel stdout
    proc outputChannel { {filename ""} } {
⟦TAB⟧variable outputChannel
⟦TAB⟧variable ChannelsWeOpened

⟦TAB⟧# This is very subtle and tricky, so let me try to explain.
⟦TAB⟧# (Hopefully this longer comment will be clear when I come
⟦TAB⟧# back in a few months, unlike its predecessor :) )
⟦TAB⟧#⟦SPACE⟧
⟦TAB⟧# The [outputChannel] command (and underlying variable) have to
⟦TAB⟧# be kept in sync with the [configure -outfile] configuration
⟦TAB⟧# option ( and underlying variable Option(-outfile) ).  This is
⟦TAB⟧# accomplished with a write trace on Option(-outfile) that will
⟦TAB⟧# update [outputChannel] whenver a new value is written.  That
⟦TAB⟧# much is easy.
⟦TAB⟧#
⟦TAB⟧# The trick is that in order to maintain compatibility with
⟦TAB⟧# version 1 of tcltest, we must allow every configuration option
⟦TAB⟧# to get its inital value from command line arguments.  This is
⟦TAB⟧# accomplished by setting initial read traces on all the
⟦TAB⟧# configuration options to parse the command line option the first
⟦TAB⟧# time they are read.  These traces are cancelled whenever the
⟦TAB⟧# program itself calls [configure].
⟦TAB⟧#⟦SPACE⟧
⟦TAB⟧# OK, then so to support tcltest 1 compatibility, it seems we want
⟦TAB⟧# to get the return from [outputFile] to trigger the read traces,
⟦TAB⟧# just in case.
⟦TAB⟧#
⟦TAB⟧# BUT!  A little known feature of Tcl variable traces is that⟦SPACE⟧
⟦TAB⟧# traces are disabled during the handling of other traces.  So,
⟦TAB⟧# if we trigger read traces on Option(-outfile) and that triggers
⟦TAB⟧# command line parsing which turns around and sets an initial
⟦TAB⟧# value for Option(-outfile) -- <whew!> -- the write trace that
⟦TAB⟧# would keep [outputChannel] in sync with that new initial value
⟦TAB⟧# would not fire!
⟦TAB⟧#
⟦TAB⟧# SO, finally, as a workaround, instead of triggering read traces
⟦TAB⟧# by invoking [outputFile], we instead trigger the same set of
⟦TAB⟧# read traces by invoking [debug].  Any command that reads a
⟦TAB⟧# configuration option would do.  [debug] is just a handy one.
⟦TAB⟧# The end result is that we support tcltest 1 compatibility and
⟦TAB⟧# keep outputChannel and -outfile in sync in all cases.
⟦TAB⟧debug

⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $outputChannel
⟦TAB⟧}
⟦TAB⟧if {[info exists ChannelsWeOpened($outputChannel)]} {
⟦TAB⟧    close $outputChannel
⟦TAB⟧    unset ChannelsWeOpened($outputChannel)
⟦TAB⟧}
⟦TAB⟧switch -exact -- $filename {
⟦TAB⟧    stderr -
⟦TAB⟧    stdout {
⟦TAB⟧⟦TAB⟧set outputChannel $filename
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧set outputChannel [open $filename a]
⟦TAB⟧⟦TAB⟧set ChannelsWeOpened($outputChannel) 1

⟦TAB⟧⟦TAB⟧# If we created the file in [temporaryDirectory], then
⟦TAB⟧⟦TAB⟧# [cleanupTests] will delete it, unless we claim it was
⟦TAB⟧⟦TAB⟧# already there.
⟦TAB⟧⟦TAB⟧set outdir [normalizePath [file dirname \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [pwd] $filename]]]
⟦TAB⟧⟦TAB⟧if {[string equal $outdir [temporaryDirectory]]} {
⟦TAB⟧⟦TAB⟧    variable filesExisted
⟦TAB⟧⟦TAB⟧    FillFilesExisted
⟦TAB⟧⟦TAB⟧    set filename [file tail $filename]
⟦TAB⟧⟦TAB⟧    if {[lsearch -exact $filesExisted $filename] == -1} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧lappend filesExisted $filename
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $outputChannel
    }

    # errors go to stderr by default
    Default errorChannel stderr
    proc errorChannel { {filename ""} } {
⟦TAB⟧variable errorChannel
⟦TAB⟧variable ChannelsWeOpened

⟦TAB⟧# This is subtle and tricky.  See the comment above in
⟦TAB⟧# [outputChannel] for a detailed explanation.
⟦TAB⟧debug

⟦TAB⟧if {[llength [info level 0]] == 1} {
⟦TAB⟧    return $errorChannel
⟦TAB⟧}
⟦TAB⟧if {[info exists ChannelsWeOpened($errorChannel)]} {
⟦TAB⟧    close $errorChannel
⟦TAB⟧    unset ChannelsWeOpened($errorChannel)
⟦TAB⟧}
⟦TAB⟧switch -exact -- $filename {
⟦TAB⟧    stderr -
⟦TAB⟧    stdout {
⟦TAB⟧⟦TAB⟧set errorChannel $filename
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧set errorChannel [open $filename a]
⟦TAB⟧⟦TAB⟧set ChannelsWeOpened($errorChannel) 1

⟦TAB⟧⟦TAB⟧# If we created the file in [temporaryDirectory], then
⟦TAB⟧⟦TAB⟧# [cleanupTests] will delete it, unless we claim it was
⟦TAB⟧⟦TAB⟧# already there.
⟦TAB⟧⟦TAB⟧set outdir [normalizePath [file dirname \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[file join [pwd] $filename]]]
⟦TAB⟧⟦TAB⟧if {[string equal $outdir [temporaryDirectory]]} {
⟦TAB⟧⟦TAB⟧    variable filesExisted
⟦TAB⟧⟦TAB⟧    FillFilesExisted
⟦TAB⟧⟦TAB⟧    set filename [file tail $filename]
⟦TAB⟧⟦TAB⟧    if {[lsearch -exact $filesExisted $filename] == -1} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧lappend filesExisted $filename
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $errorChannel
    }

##### Set up the configurable options
    #
    # The configurable options of the package
    variable Option; array set Option {}

    # Usage strings for those options
    variable Usage; array set Usage {}

    # Verification commands for those options
    variable Verify; array set Verify {}

    # Initialize the default values of the configurable options that are
    # historically associated with an exported variable.  If that variable
    # is already set, support compatibility by accepting its pre-set value.
    # Use [trace] to establish ongoing connection between the deprecated
    # exported variable and the modern option kept as a true internal var.
    # Also set up usage string and value testing for the option.
    proc Option {option value usage {verify AcceptAll} {varName {}}} {
⟦TAB⟧variable Option
⟦TAB⟧variable Verify
⟦TAB⟧variable Usage
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧set Usage($option) $usage
⟦TAB⟧set Verify($option) $verify
⟦TAB⟧if {[catch {$verify $value} msg]} {
⟦TAB⟧    return -code error $msg
⟦TAB⟧} else {
⟦TAB⟧    set Option($option) $msg
⟦TAB⟧}
⟦TAB⟧if {[string length $varName]} {
⟦TAB⟧    variable $varName
⟦TAB⟧    if {[info exists $varName]} {
⟦TAB⟧⟦TAB⟧if {[catch {$verify [set $varName]} msg]} {
⟦TAB⟧⟦TAB⟧    return -code error $msg
⟦TAB⟧⟦TAB⟧} else {
⟦TAB⟧⟦TAB⟧    set Option($option) $msg
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧unset $varName
⟦TAB⟧    }
⟦TAB⟧    namespace eval [namespace current] \
⟦TAB⟧    ⟦TAB⟧    [list upvar 0 Option($option) $varName]
⟦TAB⟧    # Workaround for Bug (now Feature Request) 572889.  Grrrr....
⟦TAB⟧    # Track all the variables tied to options
⟦TAB⟧    lappend OptionControlledVariables $varName
⟦TAB⟧    # Later, set auto-configure read traces on all
⟦TAB⟧    # of them, since a single trace on Option does not work.
⟦TAB⟧    proc $varName {{value {}}} [subst -nocommands {
⟦TAB⟧⟦TAB⟧if {[llength [info level 0]] == 2} {
⟦TAB⟧⟦TAB⟧    Configure $option [set value]
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧return [Configure $option]
⟦TAB⟧    }]
⟦TAB⟧}
    }

    proc MatchingOption {option} {
⟦TAB⟧variable Option
⟦TAB⟧set match [array names Option $option*]
⟦TAB⟧switch -- [llength $match] {
⟦TAB⟧    0 {
⟦TAB⟧⟦TAB⟧set sorted [lsort [array names Option]]
⟦TAB⟧⟦TAB⟧set values [join [lrange $sorted 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append values ", or [lindex $sorted end]"
⟦TAB⟧⟦TAB⟧return -code error "unknown option $option: should be\
⟦TAB⟧⟦TAB⟧⟦TAB⟧one of $values"
⟦TAB⟧    }
⟦TAB⟧    1 {
⟦TAB⟧⟦TAB⟧return [lindex $match 0]
⟦TAB⟧    }
⟦TAB⟧    default {
⟦TAB⟧⟦TAB⟧# Exact match trumps ambiguity
⟦TAB⟧⟦TAB⟧if {[lsearch -exact $match $option] >= 0} {
⟦TAB⟧⟦TAB⟧    return $option
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧set values [join [lrange $match 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append values ", or [lindex $match end]"
⟦TAB⟧⟦TAB⟧return -code error "ambiguous option $option:\
⟦TAB⟧⟦TAB⟧⟦TAB⟧could match $values"
⟦TAB⟧    }
⟦TAB⟧}
    }

    proc EstablishAutoConfigureTraces {} {
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧foreach varName [concat $OptionControlledVariables Option] {
⟦TAB⟧    variable $varName
⟦TAB⟧    trace variable $varName r [namespace code {ProcessCmdLineArgs ;#}]
⟦TAB⟧}
    }

    proc RemoveAutoConfigureTraces {} {
⟦TAB⟧variable OptionControlledVariables
⟦TAB⟧foreach varName [concat $OptionControlledVariables Option] {
⟦TAB⟧    variable $varName
⟦TAB⟧    foreach pair [trace vinfo $varName] {
⟦TAB⟧⟦TAB⟧foreach {op cmd} $pair break
⟦TAB⟧⟦TAB⟧if {[string equal r $op]
⟦TAB⟧⟦TAB⟧⟦TAB⟧&& [string match *ProcessCmdLineArgs* $cmd]} {
⟦TAB⟧⟦TAB⟧    trace vdelete $varName $op $cmd
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧# Once the traces are removed, this can become a no-op
⟦TAB⟧proc RemoveAutoConfigureTraces {} {}
    }

    proc Configure args {
⟦TAB⟧variable Option
⟦TAB⟧variable Verify
⟦TAB⟧set n [llength $args]
⟦TAB⟧if {$n == 0} {
⟦TAB⟧    return [lsort [array names Option]]
⟦TAB⟧}
⟦TAB⟧if {$n == 1} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    return $Option($option)
⟦TAB⟧}
⟦TAB⟧while {[llength $args] > 1} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    if {[catch {$Verify($option) [lindex $args 1]} value]} {
⟦TAB⟧⟦TAB⟧return -code error "invalid $option\
⟦TAB⟧⟦TAB⟧⟦TAB⟧value \"[lindex $args 1]\": $value"
⟦TAB⟧    }
⟦TAB⟧    set Option($option) $value
⟦TAB⟧    set args [lrange $args 2 end]
⟦TAB⟧}
⟦TAB⟧if {[llength $args]} {
⟦TAB⟧    if {[catch {MatchingOption [lindex $args 0]} option]} {
⟦TAB⟧⟦TAB⟧return -code error $option
⟦TAB⟧    }
⟦TAB⟧    return -code error "missing value for option $option"
⟦TAB⟧}
    }
    proc configure args {
⟦TAB⟧RemoveAutoConfigureTraces
⟦TAB⟧set code [catch {eval Configure $args} msg]
⟦TAB⟧return -code $code $msg
    }
⟦SPACE⟧⟦SPACE⟧⟦SPACE⟧⟦SPACE⟧
    proc AcceptVerbose { level } {
⟦TAB⟧set level [AcceptList $level]
⟦TAB⟧if {[llength $level] == 1} {
⟦TAB⟧    if {![regexp {^(pass|body|skip|start|error)$} $level]} {
⟦TAB⟧⟦TAB⟧# translate single characters abbreviations to expanded list
⟦TAB⟧⟦TAB⟧set level [string map {p pass b body s skip t start e error} \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[split $level {}]]
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set valid [list]
⟦TAB⟧foreach v $level {
⟦TAB⟧    if {[regexp {^(pass|body|skip|start|error)$} $v]} {
⟦TAB⟧⟦TAB⟧lappend valid $v
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $valid
    }

    proc IsVerbose {level} {
⟦TAB⟧variable Option
⟦TAB⟧return [expr {[lsearch -exact $Option(-verbose) $level] != -1}]
    }

    # Default verbosity is to show bodies of failed tests
    Option -verbose {body error} {
⟦TAB⟧Takes any combination of the values 'p', 's', 'b', 't' and 'e'.
⟦TAB⟧Test suite will display all passed tests if 'p' is specified, all
⟦TAB⟧skipped tests if 's' is specified, the bodies of failed tests if
⟦TAB⟧'b' is specified, and when tests start if 't' is specified.
⟦TAB⟧ErrorInfo is displayed if 'e' is specified.
    } AcceptVerbose verbose

    # Match and skip patterns default to the empty list, except for
    # matchFiles, which defaults to all .test files in the
    # testsDirectory and matchDirectories, which defaults to all
    # directories.
    Option -match * {
⟦TAB⟧Run all tests within the specified files that match one of the
⟦TAB⟧list of glob patterns given.
    } AcceptList match

    Option -skip {} {
⟦TAB⟧Skip all tests within the specified tests (via -match) and files
⟦TAB⟧that match one of the list of glob patterns given.
    } AcceptList skip

    Option -file *.test {
⟦TAB⟧Run tests in all test files that match the glob pattern given.
    } AcceptPattern matchFiles

    # By default, skip files that appear to be SCCS lock files.
    Option -notfile l.*.test {
⟦TAB⟧Skip all test files that match the glob pattern given.
    } AcceptPattern skipFiles

    Option -relateddir * {
⟦TAB⟧Run tests in directories that match the glob pattern given.
    } AcceptPattern matchDirectories

    Option -asidefromdir {} {
⟦TAB⟧Skip tests in directories that match the glob pattern given.
    } AcceptPattern skipDirectories

    # By default, don't save core files
    Option -preservecore 0 {
⟦TAB⟧If 2, save any core files produced during testing in the directory
⟦TAB⟧specified by -tmpdir. If 1, notify the user if core files are
⟦TAB⟧created.
    } AcceptInteger preserveCore

    # debug output doesn't get printed by default; debug level 1 spits
    # up only the tests that were skipped because they didn't match or
    # were specifically skipped.  A debug level of 2 would spit up the
    # tcltest variables and flags provided; a debug level of 3 causes
    # some additional output regarding operations of the test harness.
    # The tcltest package currently implements only up to debug level 3.
    Option -debug 0 {
⟦TAB⟧Internal debug level⟦SPACE⟧
    } AcceptInteger debug

    proc SetSelectedConstraints args {
⟦TAB⟧variable Option
⟦TAB⟧foreach c $Option(-constraints) {
⟦TAB⟧    testConstraint $c 1
⟦TAB⟧}
    }
    Option -constraints {} {
⟦TAB⟧Do not skip the listed constraints listed in -constraints.
    } AcceptList
    trace variable Option(-constraints) w \
⟦TAB⟧    [namespace code {SetSelectedConstraints ;#}]

    # Don't run only the "-constraint" specified tests by default
    proc ClearUnselectedConstraints args {
⟦TAB⟧variable Option
⟦TAB⟧variable testConstraints
⟦TAB⟧if {!$Option(-limitconstraints)} {return}
⟦TAB⟧foreach c [array names testConstraints] {
⟦TAB⟧    if {[lsearch -exact $Option(-constraints) $c] == -1} {
⟦TAB⟧⟦TAB⟧testConstraint $c 0
⟦TAB⟧    }
⟦TAB⟧}
    }
    Option -limitconstraints false {
⟦TAB⟧whether to run only tests with the constraints
    } AcceptBoolean limitConstraints⟦SPACE⟧
    trace variable Option(-limitconstraints) w \
⟦TAB⟧    [namespace code {ClearUnselectedConstraints ;#}]

    # A test application has to know how to load the tested commands
    # into the interpreter.
    Option -load {} {
⟦TAB⟧Specifies the script to load the tested commands.
    } AcceptScript loadScript

    # Default is to run each test file in a separate process
    Option -singleproc 0 {
⟦TAB⟧whether to run all tests in one process
    } AcceptBoolean singleProcess⟦SPACE⟧

    proc AcceptTemporaryDirectory { directory } {
⟦TAB⟧set directory [AcceptAbsolutePath $directory]
⟦TAB⟧if {![file exists $directory]} {
⟦TAB⟧    file mkdir $directory
⟦TAB⟧}
⟦TAB⟧set directory [AcceptDirectory $directory]
⟦TAB⟧if {![file writable $directory]} {
⟦TAB⟧    if {[string equal [workingDirectory] $directory]} {
⟦TAB⟧⟦TAB⟧# Special exception: accept the default value
⟦TAB⟧⟦TAB⟧# even if the directory is not writable
⟦TAB⟧⟦TAB⟧return $directory
⟦TAB⟧    }
⟦TAB⟧    return -code error "\"$directory\" is not writeable"
⟦TAB⟧}
⟦TAB⟧return $directory
    }

    # Directory where files should be created
    Option -tmpdir [workingDirectory] {
⟦TAB⟧Save temporary files in the specified directory.
    } AcceptTemporaryDirectory temporaryDirectory
    trace variable Option(-tmpdir) w \
⟦TAB⟧    [namespace code {normalizePath Option(-tmpdir) ;#}]

    # Tests should not rely on the current working directory.
    # Files that are part of the test suite should be accessed relative
    # to [testsDirectory]
    Option -testdir [workingDirectory] {
⟦TAB⟧Search tests in the specified directory.
    } AcceptDirectory testsDirectory
    trace variable Option(-testdir) w \
⟦TAB⟧    [namespace code {normalizePath Option(-testdir) ;#}]

    proc AcceptLoadFile { file } {
⟦TAB⟧if {[string equal "" $file]} {return $file}
⟦TAB⟧set file [file join [temporaryDirectory] $file]
⟦TAB⟧return [AcceptReadable $file]
    }
    proc ReadLoadScript {args} {
⟦TAB⟧variable Option
⟦TAB⟧if {[string equal "" $Option(-loadfile)]} {return}
⟦TAB⟧set tmp [open $Option(-loadfile) r]
⟦TAB⟧loadScript [read $tmp]
⟦TAB⟧close $tmp
    }
    Option -loadfile {} {
⟦TAB⟧Read the script to load the tested commands from the specified file.
    } AcceptLoadFile loadFile
    trace variable Option(-loadfile) w [namespace code ReadLoadScript]

    proc AcceptOutFile { file } {
⟦TAB⟧if {[string equal stderr $file]} {return $file}
⟦TAB⟧if {[string equal stdout $file]} {return $file}
⟦TAB⟧return [file join [temporaryDirectory] $file]
    }

    # output goes to stdout by default
    Option -outfile stdout {
⟦TAB⟧Send output from test runs to the specified file.
    } AcceptOutFile outputFile
    trace variable Option(-outfile) w \
⟦TAB⟧    [namespace code {outputChannel $Option(-outfile) ;#}]

    # errors go to stderr by default
    Option -errfile stderr {
⟦TAB⟧Send errors from test runs to the specified file.
    } AcceptOutFile errorFile
    trace variable Option(-errfile) w \
⟦TAB⟧    [namespace code {errorChannel $Option(-errfile) ;#}]

}

#####################################################################

# tcltest::Debug* --
#
#     Internal helper procedures to write out debug information
#     dependent on the chosen level. A test shell may overide
#     them, f.e. to redirect the output into a different
#     channel, or even into a GUI.

# tcltest::DebugPuts --
#
#     Prints the specified string if the current debug level is
#     higher than the provided level argument.
#
# Arguments:
#     level   The lowest debug level triggering the output
#     string  The string to print out.
#
# Results:
#     Prints the string. Nothing else is allowed.
#
# Side Effects:
#     None.
#

proc tcltest::DebugPuts {level string} {
    variable debug
    if {$debug >= $level} {
⟦TAB⟧puts $string
    }
    return
}

# tcltest::DebugPArray --
#
#     Prints the contents of the specified array if the current
#       debug level is higher than the provided level argument
#
# Arguments:
#     level           The lowest debug level triggering the output
#     arrayvar        The name of the array to print out.
#
# Results:
#     Prints the contents of the array. Nothing else is allowed.
#
# Side Effects:
#     None.
#

proc tcltest::DebugPArray {level arrayvar} {
    variable debug

    if {$debug >= $level} {
⟦TAB⟧catch {upvar  $arrayvar $arrayvar}
⟦TAB⟧parray $arrayvar
    }
    return
}

# Define our own [parray] in ::tcltest that will inherit use of the [puts]
# defined in ::tcltest.  NOTE: Ought to construct with [info args] and
# [info default], but can't be bothered now.  If [parray] changes, then
# this will need changing too.
auto_load ::parray
proc tcltest::parray {a {pattern *}} [info body ::parray]

# tcltest::DebugDo --
#
#     Executes the script if the current debug level is greater than
#       the provided level argument
#
# Arguments:
#     level   The lowest debug level triggering the execution.
#     script  The tcl script executed upon a debug level high enough.
#
# Results:
#     Arbitrary side effects, dependent on the executed script.
#
# Side Effects:
#     None.
#

proc tcltest::DebugDo {level script} {
    variable debug

    if {$debug >= $level} {
⟦TAB⟧uplevel 1 $script
    }
    return
}

#####################################################################

proc tcltest::Warn {msg} {
    puts [outputChannel] "WARNING: $msg"
}

# tcltest::mainThread
#
#     Accessor command for tcltest variable mainThread.
#
proc tcltest::mainThread { {new ""} } {
    variable mainThread
    if {[llength [info level 0]] == 1} {
⟦TAB⟧return $mainThread
    }
    set mainThread $new
}

# tcltest::testConstraint --
#
#⟦TAB⟧sets a test constraint to a value; to do multiple constraints,
#       call this proc multiple times.  also returns the value of the
#       named constraint if no value was supplied.
#
# Arguments:
#⟦TAB⟧constraint - name of the constraint
#       value - new value for constraint (should be boolean) - if not
#               supplied, this is a query
#
# Results:
#⟦TAB⟧content of tcltest::testConstraints($constraint)
#
# Side effects:
#⟦TAB⟧none

proc tcltest::testConstraint {constraint {value ""}} {
    variable testConstraints
    variable Option
    DebugPuts 3 "entering testConstraint $constraint $value"
    if {[llength [info level 0]] == 2} {
⟦TAB⟧return $testConstraints($constraint)
    }
    # Check for boolean values
    if {[catch {expr {$value && $value}} msg]} {
⟦TAB⟧return -code error $msg
    }
    if {[limitConstraints]⟦SPACE⟧
⟦TAB⟧    && [lsearch -exact $Option(-constraints) $constraint] == -1} {
⟦TAB⟧set value 0
    }
    set testConstraints($constraint) $value
}

# tcltest::interpreter --
#
#⟦TAB⟧the interpreter name stored in tcltest::tcltest
#
# Arguments:
#⟦TAB⟧executable name
#
# Results:
#⟦TAB⟧content of tcltest::tcltest
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::interpreter { {interp ""} } {
    variable tcltest
    if {[llength [info level 0]] == 1} {
⟦TAB⟧return $tcltest
    }
    if {[string equal {} $interp]} {
⟦TAB⟧set tcltest {}
    } else {
⟦TAB⟧set tcltest $interp
    }
}

#####################################################################

# tcltest::AddToSkippedBecause --
#
#⟦TAB⟧Increments the variable used to track how many tests were
#       skipped because of a particular constraint.
#
# Arguments:
#⟦TAB⟧constraint     The name of the constraint to be modified
#
# Results:
#⟦TAB⟧Modifies tcltest::skippedBecause; sets the variable to 1 if
#       didn't previously exist - otherwise, it just increments it.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::AddToSkippedBecause { constraint {value 1}} {
    # add the constraint to the list of constraints that kept tests
    # from running
    variable skippedBecause

    if {[info exists skippedBecause($constraint)]} {
⟦TAB⟧incr skippedBecause($constraint) $value
    } else {
⟦TAB⟧set skippedBecause($constraint) $value
    }
    return
}

# tcltest::PrintError --
#
#⟦TAB⟧Prints errors to tcltest::errorChannel and then flushes that
#       channel, making sure that all messages are < 80 characters per
#       line.
#
# Arguments:
#⟦TAB⟧errorMsg     String containing the error to be printed
#
# Results:
#⟦TAB⟧None.
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::PrintError {errorMsg} {
    set InitialMessage "Error:  "
    set InitialMsgLen  [string length $InitialMessage]
    puts -nonewline [errorChannel] $InitialMessage

    # Keep track of where the end of the string is.
    set endingIndex [string length $errorMsg]

    if {$endingIndex < (80 - $InitialMsgLen)} {
⟦TAB⟧puts [errorChannel] $errorMsg
    } else {
⟦TAB⟧# Print up to 80 characters on the first line, including the
⟦TAB⟧# InitialMessage.
⟦TAB⟧set beginningIndex [string last " " [string range $errorMsg 0 \
⟦TAB⟧⟦TAB⟧[expr {80 - $InitialMsgLen}]]]
⟦TAB⟧puts [errorChannel] [string range $errorMsg 0 $beginningIndex]

⟦TAB⟧while {![string equal end $beginningIndex]} {
⟦TAB⟧    puts -nonewline [errorChannel] \
⟦TAB⟧⟦TAB⟧    [string repeat " " $InitialMsgLen]
⟦TAB⟧    if {($endingIndex - $beginningIndex)
⟦TAB⟧⟦TAB⟧    < (80 - $InitialMsgLen)} {
⟦TAB⟧⟦TAB⟧puts [errorChannel] [string trim \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[string range $errorMsg $beginningIndex end]]
⟦TAB⟧⟦TAB⟧break
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧set newEndingIndex [expr {[string last " " \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[string range $errorMsg $beginningIndex \
⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧[expr {$beginningIndex
⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧⟦TAB⟧+ (80 - $InitialMsgLen)}]
⟦TAB⟧⟦TAB⟧]] + $beginningIndex}]
⟦TAB⟧⟦TAB⟧if {($newEndingIndex <= 0)
⟦TAB⟧⟦TAB⟧⟦TAB⟧|| ($newEndingIndex <= $beginningIndex)} {
⟦TAB⟧⟦TAB⟧    set newEndingIndex end
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧puts [errorChannel] [string trim \
⟦TAB⟧⟦TAB⟧⟦TAB⟧[string range $errorMsg \
⟦TAB⟧⟦TAB⟧⟦TAB⟧    $beginningIndex $newEndingIndex]]
⟦TAB⟧⟦TAB⟧set beginningIndex $newEndingIndex
⟦TAB⟧    }
⟦TAB⟧}
    }
    flush [errorChannel]
    return
}

# tcltest::SafeFetch --
#
#⟦TAB⟧ The following trace procedure makes it so that we can safely
#        refer to non-existent members of the testConstraints array
#        without causing an error.  Instead, reading a non-existent
#        member will return 0. This is necessary because tests are
#        allowed to use constraint "X" without ensuring that
#        testConstraints("X") is defined.
#
# Arguments:
#⟦TAB⟧n1 - name of the array (testConstraints)
#       n2 - array key value (constraint name)
#       op - operation performed on testConstraints (generally r)
#
# Results:
#⟦TAB⟧none
#
# Side effects:
#⟦TAB⟧sets testConstraints($n2) to 0 if it's referenced but never
#       before used

proc tcltest::SafeFetch {n1 n2 op} {
    variable testConstraints
    DebugPuts 3 "entering SafeFetch $n1 $n2 $op"
    if {[string equal {} $n2]} {return}
    if {![info exists testConstraints($n2)]} {
⟦TAB⟧if {[catch {testConstraint $n2 [eval [ConstraintInitializer $n2]]}]} {
⟦TAB⟧    testConstraint $n2 0
⟦TAB⟧}
    }
}

# tcltest::ConstraintInitializer --
#
#⟦TAB⟧Get or set a script that when evaluated in the tcltest namespace
#⟦TAB⟧will return a boolean value with which to initialize the
#⟦TAB⟧associated constraint.
#
# Arguments:
#⟦TAB⟧constraint - name of the constraint initialized by the script
#⟦TAB⟧script - the initializer script
#
# Results
#⟦TAB⟧boolean value of the constraint - enabled or disabled
#
# Side effects:
#⟦TAB⟧Constraint is initialized for future reference by [test]
proc tcltest::ConstraintInitializer {constraint {script ""}} {
    variable ConstraintInitializer
    DebugPuts 3 "entering ConstraintInitializer $constraint $script"
    if {[llength [info level 0]] == 2} {
⟦TAB⟧return $ConstraintInitializer($constraint)
    }
    # Check for boolean values
    if {![info complete $script]} {
⟦TAB⟧return -code error "ConstraintInitializer must be complete script"
    }
    set ConstraintInitializer($constraint) $script
}

# tcltest::InitConstraints --
#
# Call all registered constraint initializers to force initialization
# of all known constraints.
# See the tcltest man page for the list of built-in constraints defined
# in this procedure.
#
# Arguments:
#⟦TAB⟧none
#
# Results:
#⟦TAB⟧The testConstraints array is reset to have an index for each
#⟦TAB⟧built-in test constraint.
#
# Side Effects:
#       None.
#

proc tcltest::InitConstraints {} {
    variable ConstraintInitializer
    initConstraintsHook
    foreach constraint [array names ConstraintInitializer] {
⟦TAB⟧testConstraint $constraint
    }
}

proc tcltest::DefineConstraintInitializers {} {
    ConstraintInitializer singleTestInterp {singleProcess}

    # All the 'pc' constraints are here for backward compatibility and
    # are not documented.  They have been replaced with equivalent 'win'
    # constraints.

    ConstraintInitializer unixOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) unix}
    ConstraintInitializer macOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) macintosh}
    ConstraintInitializer pcOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) windows}
    ConstraintInitializer winOnly \
⟦TAB⟧    {string equal $::tcl_platform(platform) windows}

    ConstraintInitializer unix {testConstraint unixOnly}
    ConstraintInitializer mac {testConstraint macOnly}
    ConstraintInitializer pc {testConstraint pcOnly}
    ConstraintInitializer win {testConstraint winOnly}

    ConstraintInitializer unixOrPc \
⟦TAB⟧    {expr {[testConstraint unix] || [testConstraint pc]}}
    ConstraintInitializer macOrPc \
⟦TAB⟧    {expr {[testConstraint mac] || [testConstraint pc]}}
    ConstraintInitializer unixOrWin \
⟦TAB⟧    {expr {[testConstraint unix] || [testConstraint win]}}
    ConstraintInitializer macOrWin \
⟦TAB⟧    {expr {[testConstraint mac] || [testConstraint win]}}
    ConstraintInitializer macOrUnix \
⟦TAB⟧    {expr {[testConstraint mac] || [testConstraint unix]}}

    ConstraintInitializer nt {string equal $::tcl_platform(os) "Windows NT"}
    ConstraintInitializer 95 {string equal $::tcl_platform(os) "Windows 95"}
    ConstraintInitializer 98 {string equal $::tcl_platform(os) "Windows 98"}

    # The following Constraints switches are used to mark tests that
    # should work, but have been temporarily disabled on certain
    # platforms because they don't and we haven't gotten around to
    # fixing the underlying problem.

    ConstraintInitializer tempNotPc {expr {![testConstraint pc]}}
    ConstraintInitializer tempNotWin {expr {![testConstraint win]}}
    ConstraintInitializer tempNotMac {expr {![testConstraint mac]}}
    ConstraintInitializer tempNotUnix {expr {![testConstraint unix]}}

    # The following Constraints switches are used to mark tests that
    # crash on certain platforms, so that they can be reactivated again
    # when the underlying problem is fixed.

    ConstraintInitializer pcCrash {expr {![testConstraint pc]}}
    ConstraintInitializer winCrash {expr {![testConstraint win]}}
    ConstraintInitializer macCrash {expr {![testConstraint mac]}}
    ConstraintInitializer unixCrash {expr {![testConstraint unix]}}

    # Skip empty tests

    ConstraintInitializer emptyTest {format 0}

    # By default, tests that expose known bugs are skipped.

    ConstraintInitializer knownBug {format 0}

    # By default, non-portable tests are skipped.

    ConstraintInitializer nonPortable {format 0}

    # Some tests require user interaction.

    ConstraintInitializer userInteraction {format 0}

    # Some tests must be skipped if the interpreter is not in
    # interactive mode

    ConstraintInitializer interactive \
⟦TAB⟧    {expr {[info exists ::tcl_interactive] && $::tcl_interactive}}

    # Some tests can only be run if the installation came from a CD
    # image instead of a web image.  Some tests must be skipped if you
    # are running as root on Unix.  Other tests can only be run if you
    # are running as root on Unix.

    ConstraintInitializer root {expr \
⟦TAB⟧    {[string equal unix $::tcl_platform(platform)]
⟦TAB⟧    && ([string equal root $::tcl_platform(user)]
⟦TAB⟧⟦TAB⟧|| [string equal "" $::tcl_platform(user)])}}
    ConstraintInitializer notRoot {expr {![testConstraint root]}}

    # Set nonBlockFiles constraint: 1 means this platform supports
    # setting files into nonblocking mode.

    ConstraintInitializer nonBlockFiles {
⟦TAB⟧    set code [expr {[catch {set f [open defs r]}]⟦SPACE⟧
⟦TAB⟧⟦TAB⟧    || [catch {fconfigure $f -blocking off}]}]
⟦TAB⟧    catch {close $f}
⟦TAB⟧    set code
    }

    # Set asyncPipeClose constraint: 1 means this platform supports
    # async flush and async close on a pipe.
    #
    # Test for SCO Unix - cannot run async flushing tests because a
    # potential problem with select is apparently interfering.
    # (Mark Diekhans).

    ConstraintInitializer asyncPipeClose {expr {
⟦TAB⟧    !([string equal unix $::tcl_platform(platform)]⟦SPACE⟧
⟦TAB⟧    && ([catch {exec uname -X | fgrep {Release = 3.2v}}] == 0))}}

    # Test to see if we have a broken version of sprintf with respect
    # to the "e" format of floating-point numbers.

    ConstraintInitializer eformat {string equal [format %g 5e-5] 5e-05}

    # Test to see if execed commands such as cat, echo, rm and so forth
    # are present on this machine.

    ConstraintInitializer unixExecs {
⟦TAB⟧set code 1
        if {[string equal macintosh $::tcl_platform(platform)]} {
⟦TAB⟧    set code 0
        }
        if {[string equal windows $::tcl_platform(platform)]} {
⟦TAB⟧    if {[catch {
⟦TAB⟧        set file _tcl_test_remove_me.txt
⟦TAB⟧        makeFile {hello} $file
⟦TAB⟧    }]} {
⟦TAB⟧        set code 0
⟦TAB⟧    } elseif {
⟦TAB⟧        [catch {exec cat $file}] ||
⟦TAB⟧        [catch {exec echo hello}] ||
⟦TAB⟧        [catch {exec sh -c echo hello}] ||
⟦TAB⟧        [catch {exec wc $file}] ||
⟦TAB⟧        [catch {exec sleep 1}] ||
⟦TAB⟧        [catch {exec echo abc > $file}] ||
⟦TAB⟧        [catch {exec chmod 644 $file}] ||
⟦TAB⟧        [catch {exec rm $file}] ||
⟦TAB⟧        [llength [auto_execok mkdir]] == 0 ||
⟦TAB⟧        [llength [auto_execok fgrep]] == 0 ||
⟦TAB⟧        [llength [auto_execok grep]] == 0 ||
⟦TAB⟧        [llength [auto_execok ps]] == 0
⟦TAB⟧    } {
⟦TAB⟧        set code 0
⟦TAB⟧    }
⟦TAB⟧    removeFile $file
        }
⟦TAB⟧set code
    }

    ConstraintInitializer stdio {
⟦TAB⟧set code 0
⟦TAB⟧if {![catch {set f [open "|[list [interpreter]]" w]}]} {
⟦TAB⟧    if {![catch {puts $f exit}]} {
⟦TAB⟧⟦TAB⟧if {![catch {close $f}]} {
⟦TAB⟧⟦TAB⟧    set code 1
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧set code
    }

    # Deliberately call socket with the wrong number of arguments.  The
    # error message you get will indicate whether sockets are available
    # on this system.

    ConstraintInitializer socket {
⟦TAB⟧catch {socket} msg
⟦TAB⟧string compare $msg "sockets are not available on this system"
    }

    # Check for internationalization
    ConstraintInitializer hasIsoLocale {
⟦TAB⟧if {[llength [info commands testlocale]] == 0} {
⟦TAB⟧    set code 0
⟦TAB⟧} else {
⟦TAB⟧    set code [string length [SetIso8859_1_Locale]]
⟦TAB⟧    RestoreLocale
⟦TAB⟧}
⟦TAB⟧set code
    }

}
#####################################################################

# Usage and command line arguments processing.

# tcltest::PrintUsageInfo
#
#⟦TAB⟧Prints out the usage information for package tcltest.  This can
#⟦TAB⟧be customized with the redefinition of [PrintUsageInfoHook].
#
# Arguments:
#⟦TAB⟧none
#
# Results:
#       none
#
# Side Effects:
#       none
proc tcltest::PrintUsageInfo {} {
    puts [Usage]
    PrintUsageInfoHook
}

proc tcltest::Usage { {option ""} } {
    variable Usage
    variable Verify
    if {[llength [info level 0]] == 1} {
⟦TAB⟧set msg "Usage: [file tail [info nameofexecutable]] script "
⟦TAB⟧append msg "?-help? ?flag value? ... \n"
⟦TAB⟧append msg "Available flags (and valid input values) are:"

⟦TAB⟧set max 0
⟦TAB⟧set allOpts [concat -help [Configure]]
⟦TAB⟧foreach opt $allOpts {
⟦TAB⟧    set foo [Usage $opt]
⟦TAB⟧    foreach [list x type($opt) usage($opt)] $foo break
⟦TAB⟧    set line($opt) "  $opt $type($opt)  "
⟦TAB⟧    set length($opt) [string length $line($opt)]
⟦TAB⟧    if {$length($opt) > $max} {set max $length($opt)}
⟦TAB⟧}
⟦TAB⟧set rest [expr {72 - $max}]
⟦TAB⟧foreach opt $allOpts {
⟦TAB⟧    append msg \n$line($opt)
⟦TAB⟧    append msg [string repeat " " [expr {$max - $length($opt)}]]
⟦TAB⟧    set u [string trim $usage($opt)]
⟦TAB⟧    catch {append u "  (default: \[[Configure $opt]])"}
⟦TAB⟧    regsub -all {\s*\n\s*} $u " " u
⟦TAB⟧    while {[string length $u] > $rest} {
⟦TAB⟧⟦TAB⟧set break [string wordstart $u $rest]
⟦TAB⟧⟦TAB⟧if {$break == 0} {
⟦TAB⟧⟦TAB⟧    set break [string wordend $u 0]
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧⟦TAB⟧append msg [string range $u 0 [expr {$break - 1}]]
⟦TAB⟧⟦TAB⟧set u [string trim [string range $u $break end]]
⟦TAB⟧⟦TAB⟧append msg \n[string repeat " " $max]
⟦TAB⟧    }
⟦TAB⟧    append msg $u
⟦TAB⟧}
⟦TAB⟧return $msg\n
    } elseif {[string equal -help $option]} {
⟦TAB⟧return [list -help "" "Display this usage information."]
    } else {
⟦TAB⟧set type [lindex [info args $Verify($option)] 0]
⟦TAB⟧return [list $option $type $Usage($option)]
    }
}

# tcltest::ProcessFlags --
#
#⟦TAB⟧process command line arguments supplied in the flagArray - this
#⟦TAB⟧is called by processCmdLineArgs.  Modifies tcltest variables
#⟦TAB⟧according to the content of the flagArray.
#
# Arguments:
#⟦TAB⟧flagArray - array containing name/value pairs of flags
#
# Results:
#⟦TAB⟧sets tcltest variables according to their values as defined by
#       flagArray
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::ProcessFlags {flagArray} {
    # Process -help first
    if {[lsearch -exact $flagArray {-help}] != -1} {
⟦TAB⟧PrintUsageInfo
⟦TAB⟧exit 1
    }

    if {[llength $flagArray] == 0} {
⟦TAB⟧RemoveAutoConfigureTraces
    } else {
⟦TAB⟧set args $flagArray
⟦TAB⟧while {[llength $args]>1 && [catch {eval configure $args} msg]} {

⟦TAB⟧    # Something went wrong parsing $args for tcltest options
⟦TAB⟧    # Check whether the problem is "unknown option"
⟦TAB⟧    if {[regexp {^unknown option (\S+):} $msg -> option]} {
⟦TAB⟧⟦TAB⟧# Could be this is an option the Hook knows about
⟦TAB⟧⟦TAB⟧set moreOptions [processCmdLineArgsAddFlagsHook]
⟦TAB⟧⟦TAB⟧if {[lsearch -exact $moreOptions $option] == -1} {
⟦TAB⟧⟦TAB⟧    # Nope.  Report the error, including additional options,
⟦TAB⟧⟦TAB⟧    # but keep going
⟦TAB⟧⟦TAB⟧    if {[llength $moreOptions]} {
⟦TAB⟧⟦TAB⟧⟦TAB⟧append msg ", "
⟦TAB⟧⟦TAB⟧⟦TAB⟧append msg [join [lrange $moreOptions 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧⟦TAB⟧append msg "or [lindex $moreOptions end]"
⟦TAB⟧⟦TAB⟧    }
⟦TAB⟧⟦TAB⟧    Warn $msg
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧# error is something other than "unknown option"
⟦TAB⟧⟦TAB⟧# notify user of the error; and exit
⟦TAB⟧⟦TAB⟧puts [errorChannel] $msg
⟦TAB⟧⟦TAB⟧exit 1
⟦TAB⟧    }

⟦TAB⟧    # To recover, find that unknown option and remove up to it.
⟦TAB⟧    # then retry
⟦TAB⟧    while {![string equal [lindex $args 0] $option]} {
⟦TAB⟧⟦TAB⟧set args [lrange $args 2 end]
⟦TAB⟧    }
⟦TAB⟧    set args [lrange $args 2 end]
⟦TAB⟧}
⟦TAB⟧if {[llength $args] == 1} {
⟦TAB⟧    puts [errorChannel] \
⟦TAB⟧⟦TAB⟧    "missing value for option [lindex $args 0]"
⟦TAB⟧    exit 1
⟦TAB⟧}
    }

    # Call the hook
    catch {
        array set flag $flagArray
        processCmdLineArgsHook [array get flag]
    }
    return
}

# tcltest::ProcessCmdLineArgs --
#
#       This procedure must be run after constraint initialization is
#⟦TAB⟧set up (by [DefineConstraintInitializers]) because some constraints
#⟦TAB⟧can be overridden.
#
#       Perform configuration according to the command-line options.
#
# Arguments:
#⟦TAB⟧none
#
# Results:
#⟦TAB⟧Sets the above-named variables in the tcltest namespace.
#
# Side Effects:
#       None.
#

proc tcltest::ProcessCmdLineArgs {} {
    variable originalEnv
    variable testConstraints

    # The "argv" var doesn't exist in some cases, so use {}.
    if {![info exists ::argv]} {
⟦TAB⟧ProcessFlags {}
    } else {
⟦TAB⟧ProcessFlags $::argv
    }

    # Spit out everything you know if we're at a debug level 2 or
    # greater
    DebugPuts 2 "Flags passed into tcltest:"
    if {[info exists ::env(TCLTEST_⟦TAB⟧Sets the variable tcltest::CustomMatch

proc tcltest::customMatch {mode script} {
    variable CustomMatch
    if {![info complete $script]} {
⟦TAB⟧return -code error \
⟦TAB⟧⟦TAB⟧"invalid customMatch script; can't evaluate after completion"
    }
    set CustomMatch($mode) $script
}

# tcltest::SubstArguments list
#
# This helper function takes in a list of words, then perform a
# substitution on the list as though each word in the list is a separate
# argument to the Tcl function.  For example, if this function is
# invoked as:
#
#      SubstArguments {$a {$a}}
#
# Then it is as though the function is invoked as:
#
#      SubstArguments $a {$a}
#
# This code is adapted from Paul Duffin's function "SplitIntoWords".
# The original function can be found  on:
#
#      http://purl.org/thecliff/tcl/wiki/858.html
#
# Results:
#     a list containing the result of the substitution
#
# Exceptions:
#     An error may occur if the list containing unbalanced quote or
#     unknown variable.
#
# Side Effects:
#     None.
#

proc tcltest::SubstArguments {argList} {

    # We need to split the argList up into tokens but cannot use list
    # operations as they throw away some significant quoting, and
    # [split] ignores braces as it should.  Therefore what we do is
    # gradually build up a string out of whitespace seperated strings.
    # We cannot use [split] to split the argList into whitespace
    # separated strings as it throws away the whitespace which maybe
    # important so we have to do it all by hand.

    set result {}
    set token ""

    while {[string length $argList]} {
        # Look for the next word containing a quote: " { }
        if {[regexp -indices {[^ \t\n]*[\"\{\}]+[^ \t\n]*} \
⟦TAB⟧⟦TAB⟧$argList all]} {
            # Get the text leading up to this word, but not including
⟦TAB⟧    # this word, from the argList.
            set text [string range $argList 0 \
⟦TAB⟧⟦TAB⟧    [expr {[lindex $all 0] - 1}]]
            # Get the word with the quote
            set word [string range $Attributes(-$item) [uplevel 1 \
⟦TAB⟧⟦TAB⟧⟦TAB⟧    ::concat $testAttributes(-$item)]
⟦TAB⟧⟦TAB⟧}
⟦TAB⟧    }
⟦TAB⟧} else {
⟦TAB⟧    array set testAttributes $args
⟦TAB⟧}

⟦TAB⟧set validFlags {-setup -cleanup -body -result -returnCodes \
⟦TAB⟧⟦TAB⟧-match -output -errorOutput -constraints}

⟦TAB⟧foreach flag [array names testAttributes] {
⟦TAB⟧    if {[lsearch -exact $validFlags $flag] == -1} {
⟦TAB⟧⟦TAB⟧incr testLevel -1
⟦TAB⟧⟦TAB⟧set sorted [lsort $validFlags]
⟦TAB⟧⟦TAB⟧set options [join [lrange $sorted 0 end-1] ", "]
⟦TAB⟧⟦TAB⟧append options ", or [lindex $sorted end]"
⟦TAB⟧⟦TAB⟧return -code error "bad option \"$flag\": must be $options"
⟦TAB⟧    }
⟦TAB⟧}

⟦TAB⟧# store whatever the user gave us
⟦TAB⟧foreach item [array names testAttributes] {
⟦TAB⟧    set [string trimleft $item "-"] $testAttributes($item)
⟦TAB⟧}

⟦TAB⟧# Check the values supplied for -match
⟦TAB⟧variable CustomMatch
⟦TAB⟧if {[lsearch [array names CustomMatch] $match] == -1} {
⟦TAB⟧    incr testLevel -1
⟦TAB⟧    set sorted [lsort [array names CustomMatch]]
⟦TAB⟧    set values [join [lrange $sorted 0 end-1] ", "]
⟦TAB⟧    append values ", or [lindex $sorted end]"
⟦TAB⟧    return -code error "bad -match value \"$match\":\
⟦TAB⟧⟦TAB⟧    must be $values"
⟦TAB⟧}

⟦TAB⟧# Replace symbolic valies supplied for -returnCodes
⟦TAB⟧foreach {strcode numcode} {ok 0 normal 0 error 1 return 2 break 3 continue 4} {
⟦TAB⟧    set returnCodes [string map -nocase [list $strcode $numcode] $returnCodes]
⟦TAB⟧}
    } else {
⟦TAB⟧# This is parsing for the old test command format; it is here
⟦TAB⟧# for backward compatibility.
⟦TAB⟧set result [lindex $args end]
⟦TAB⟧if {[llength $args] == 2} {
⟦TAB⟧    set body [lindex $args 0]
⟦TAB⟧} elseif {[llength $args] == 3} {
⟦TAB⟧    set constraints [lindex $args 0]
⟦TAB⟧    set body [lindex $args 1]
⟦TAB⟧} else {
⟦TAB⟧    incr testLevel -1
⟦TAB⟧    return -code error "wrong # args:\
⟦TAB⟧⟦TAB⟧    should be \"test name desc ?options?\""
⟦TAB⟧}
    }

    if {[Skipped $name $constraints]} {
⟦TAB⟧incr testLevel -1
⟦TAB⟧return
    }

    # Save information about the core file.⟦SPACE⟧⟦SPACE⟧
    if {[preserveCore]} {
⟦TAB⟧if {[file exists [file join [workingDirectory] core]]} {
⟦TAB⟧    set coreModTime [file mtime [file join [workingDirectory] core]]f the test
# body to be captured for comparison against the expected values.

proc tcltest::RunTest {name script} {
    DebugPuts 3 "Running $name {$script}"

    # If there is no "memory" command (because memory debugging isn't
    # enabled), then don't attempt to use the command.

    if {[llength [info commands memory]] == 1} {
⟦TAB⟧memory tag $name
    }

    set code [catch {uplevel 1 $script} actualAnswer]

    return [list $actualAnswer $code]
}

#####################################################################

# tcltest::cleanupTestsHook --
#
#⟦TAB⟧This hook allows a harness that builds upon tcltest to specify
#       additional things that should be done at cleanup.
#

if {[llength [info commands tcltest::cleanupTestsHook]] == 0} {
    proc tcltest::cleanupTestsHook {} {}
}

# tcltest::cleanupTests --
#
# Remove files and dirs created using the makeFile and makeDirectory
# commands since the last time this proc was invoked.
#
# Print the names of the files created without the makeFile command
# since the tests were invoked.
#
# Print the number tests (total, passed, failed, and skipped) since the
# tests were invoked.
#
# Restore original environment (as reported by special variable env).
#
# Arguments:
#      calledFromAllFile - if 0, behave as if we are running a single
#      test file within an entire suite of tests.  if we aren't running
#      a single test file, then don't report status.  check for new
#      files created during the test run and report on them.  if 1,
#      report collated status from all the test file runs.
#
# Results:
#      None.
#
# Side Effects:
#      None
#

proc tcltest::cleanupTests {{calledFromAllFile 0}} {
    variable filesMade
    variable filesExisted
    variable createdNewFiles
    variable testSingleFile
    variable numTests
    variable numTestFiles
    variable failFiles
    variable skippedBecause
    variable currentFailure
    variable originalEnv
    variable originalTclPlatform
    variable coreModTime

    ersion is needed for compatibility with tcltest 1.0
proc tcltest::getMatchingFiles args {eval GetMatchingFiles $args}

proc tcltest::GetMatchingFiles { args } {
    if {[llength $args]} {
⟦TAB⟧set dirList $args
    } else {
⟦TAB⟧# Finding tests only in [testsDirectory] is normal operation.
⟦TAB⟧# This procedure is written to accept multiple directory arguments
⟦TAB⟧# only to satisfy version 1 compatibility.
⟦TAB⟧set dirList [list [testsDirectory]]
    }

    set matchingFiles [list]
    foreach directory $dirList {

⟦TAB⟧# List files in $directory that match patterns to run.
⟦TAB⟧set matchFileList [list]
⟦TAB⟧foreach match [matchFiles] {
⟦TAB⟧    set matchFileList [concat $matchFileList \
⟦TAB⟧⟦TAB⟧    [glob -directory $directory -types {b c f p s} \
⟦TAB⟧⟦TAB⟧    -nocomplain -- $match]]
⟦TAB⟧}

⟦TAB⟧# List files in $directory that match patterns to skip.
⟦TAB⟧set skipFileList [list]
⟦TAB⟧foreach skip [skipFiles] {
⟦TAB⟧    set skipFileList [concat $skipFileList \
⟦TAB⟧⟦TAB⟧    [glob -directory $directory -types {b c f p s} \
⟦TAB⟧⟦TAB⟧    -nocomplain -- $skip]]
⟦TAB⟧}

⟦TAB⟧# Add to result list all files in match list and not in skip list
⟦TAB⟧foreach file $matchFileList {
⟦TAB⟧    if {[lsearch -exact $skipFileList $file] == -1} {
⟦TAB⟧⟦TAB⟧lappend matchingFiles $file
⟦TAB⟧    }
⟦TAB⟧}
    }

    if {[llength $matchingFiles] == 0} {
⟦TAB⟧PrintError "No test files remain after applying your match and\
⟦TAB⟧⟦TAB⟧skip patterns!"
    }
    return $matchingFiles
}

# tcltest::GetMatchingDirectories --
#
#⟦TAB⟧Looks at the patterns given to match and skip directories and
#⟦TAB⟧uses them to put together a list of the test directories that we
#⟦TAB⟧should attempt to run.  (Only subdirectories containing an
#⟦TAB⟧"all.tcl" file are put into the list.)
#
# Arguments:
#⟦TAB⟧root directory from which to search
#
# Results:
#⟦TAB⟧The constructed list is returned to the user.  This is used in
#⟦TAB⟧the primary all.tcl file.
#
# Side Effects:
#       None.

proc tcltest::GetMatchingDirectories {rootdir} {

    # Determine the skip list first, to avoid [glob]-ing over subdirectories
    # we're going to throw away anyway.  Be sure we ski[file delete]
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::removeFile {name {directory ""}} {
    variable filesMade
    FillFilesExisted
    if {[llength [info level 0]] == 2} {
⟦TAB⟧set directory [temporaryDirectory]
    }
    set fullName [file join $directory $name]
    DebugPuts 3 "[lindex [info level 0] 0]: removing $fullName"
    set idx [lsearch -exact $filesMade $fullName]
    set filesMade [lreplace $filesMade $idx $idx]
    if {$idx == -1} {
⟦TAB⟧DebugDo 1 {
⟦TAB⟧    Warn "removeFile removing \"$fullName\":\n  not created by makeFile"
⟦TAB⟧}
    }⟦SPACE⟧
    if {![file isfile $fullName]} {
⟦TAB⟧DebugDo 1 {
⟦TAB⟧    Warn "removeFile removing \"$fullName\":\n  not a file"
⟦TAB⟧}
    }
    return [file delete $fullName]
}

# tcltest::makeDirectory --
#
# Create a new dir with the name <name>.
#
# If this dir hasn't been created via makeDirectory since the last time
# cleanupTests was called, add it to the $directoriesMade list, so it
# will be removed by the next call to cleanupTests.
#
# Arguments:
#       name            name of the new directory
#       directory       directory in which to create new dir
#
# Results:
#⟦TAB⟧absolute path to the directory created
#
# Side effects:
#⟦TAB⟧None.

proc tcltest::makeDirectory {name {directory ""}} {
    variable filesMade
    FillFilesExisted
    if {[llength [info level 0]] == 2} {
⟦TAB⟧set directory [temporaryDirectory]
    }
    set fullName [file join $directory $name]
    DebugPuts 3 "[lindex [info level 0] 0]: creating $fullName"
    file mkdir $fullName
    if {[lsearch -exact $filesMade $fullName] == -1} {
⟦TAB⟧lappend filesMade $fullName
    }
    return $fullName
}

# tcltest::removeDirectory --
#
#⟦TAB⟧Removes a named directory from the file system.
#
# Arguments:
#⟦TAB⟧name          Name of the directory to remove
#       directory     Directory from which to remove
#
# Results:
#⟦TAB⟧return value from [file delete]
#
# Side effects:
#⟦TAB⟧None

proc tcltest::removeDirectory {name {directory ""}} {
    variable filesMade
    FillFilesExisted
    if {[llength [info leveFromEnvironment
    }

    proc LoadTimeCmdLineArgParsingRequired {} {
⟦TAB⟧set required false
⟦TAB⟧if {[info exists ::argv] && [lsearch -exact $::argv -help] != -1} {
⟦TAB⟧    # The command line asks for -help, so give it (and exit)
⟦TAB⟧    # right now.  ([configure] does not process -help)
⟦TAB⟧    set required true
⟦TAB⟧}
⟦TAB⟧foreach hook { PrintUsageInfoHook processCmdLineArgsHook
⟦TAB⟧⟦TAB⟧⟦TAB⟧processCmdLineArgsAddFlagsHook } {
⟦TAB⟧    if {[string equal [namespace current] [namespace qualifiers \
⟦TAB⟧⟦TAB⟧    [namespace which $hook]]]} {
⟦TAB⟧⟦TAB⟧set required true
⟦TAB⟧    } else {
⟦TAB⟧⟦TAB⟧proc $hook args {}
⟦TAB⟧    }
⟦TAB⟧}
⟦TAB⟧return $required
    }

    # Only initialize configurable options from the command line arguments
    # at package load time if necessary for backward compatibility.  This
    # lets the tcltest user call [configure] for themselves if they wish.
    # Traces are established for auto-configuration from the command line
    # if any configurable options are accessed before the user calls
    # [configure].
    if {[LoadTimeCmdLineArgParsingRequired]} {
⟦TAB⟧ProcessCmdLineArgs
    } else {
⟦TAB⟧EstablishAutoConfigureTraces
    }

    package provide [namespace tail [namespace current]] $Version
}
```

- real-harness-explicit-source-child, lookup, completion 0, result:

```text-visible
child-provider {8.4.6 8.4 ::} module-code 1 module-result {invalid command name "pwd"} require-code 1 require-result {can't find package tcltest}⟦EOF⟧
```

- real-harness-explicit-source-child, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- real-harness-explicit-source-child: intended operation reached.
