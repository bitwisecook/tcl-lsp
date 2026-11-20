# Run the BIG-IP rule checker's own event/profile/licence constraints offline.
#
# Usage: tclsh irule-oracle.tcl <extracted-checker-dir> <out/oracle.tsv>
#
# Sources the checker Tcl extracted by extract-tclcheck.py into a plain tclsh,
# stubs the C-side hooks (save_profile_dep, save_proxy_dep, feature_enabled)
# to record what they are asked, runs the checker's own init, and then for
# every event x command evaluates exactly what the loader evaluates for a
# bare command: the command and namespace constraints with the JET/legacy
# two-stage downgrade (tclCheck::invoke2stageConstraints), the constraints of
# the grammar form a zero-argument call matches, and the implicit
# namespace-to-event check. Writes oracle.tsv (event, command, verdict,
# profile requirement, licences, proxy dependency) and, beside it,
# events.tsv, hierarchy.tsv, cat_events.tsv, nsmap.tsv and disabled.tsv.
#
# On 21.1.0.1 it agrees with the appliance's loader on 147,079 of 147,098
# measured cells. Outputs are derived from F5 source: keep them outside
# this repository. The checker writes three debug name lists into the
# working directory when sourced; run it from a scratch directory.
set D [lindex $argv 0]
set ::missing [dict create]
proc unknown {args} { dict incr ::missing [lindex $args 0]; return "" }
foreach f {message_tcl checker_tcl coreTcl_tcl context_tcl jet_checker_tcl jet_misc_tcl jet_tcl jet_event_tcl jet_bigip_checker_tcl jet_bigip_constraint_checker_tcl jet_bigip_cmd_checker_tcl f5Cmds_tcl userproc_tcl} {
    uplevel #0 [list source $D/$f.tcl]
}
# C-side hooks: record what the checker asks of them
set ::pdeps {}; set ::proxy {}; set ::lic {}
proc save_profile_dep {args} { lappend ::pdeps $args; return }
proc save_proxy_dep {args} { lappend ::proxy $args; return }
proc feature_enabled {f} { lappend ::lic $f; return "" }
tclCheck::init
BIGIP::init
set tclCheck::scanning 0
set tclCheck::eventContextCheckingEnabled 1
set tclCheck::currentScope SCOPE_WHEN

# constraint scripts attached to the grammar form that a bare (zero-argument)
# call matches: a cmd_noop_word, or an alternative that can match nothing
proc zero_form_constraints {node} {
    set node [string trim $node]
    set head [lindex $node 0]
    switch -- $head {
        cmd_noop_word { return [lindex $node 1] }
        cmd_alternative_word {
            foreach alt [lindex $node 1] {
                set c [zero_form_constraints $alt]
                if {$c ne "NOMATCH"} { return [concat $c [lindex $node 2]] }
            }
            return NOMATCH
        }
        cmd_optional_word - cmd_zero_more_word { return [lindex $node 3] }
        cmd_sequence_word {
            set acc {}
            foreach item [lindex $node 1] {
                set c [zero_form_constraints $item]
                if {$c eq "NOMATCH"} { return NOMATCH }
                set acc [concat $acc $c]
            }
            return [concat $acc [lindex $node 2]]
        }
        default { return NOMATCH }
    }
}
proc grammar_of {cmd} {
    if {![info exists ::BIGIP::bigip_rule_syntax($cmd)]} { return "" }
    foreach part $::BIGIP::bigip_rule_syntax($cmd) {
        set part [string trim $part]
        if {[lindex $part 0] eq "jetCheck::run_cmd_checker"} { return [lindex $part 1] }
    }
    return ""
}
proc probe {ev cmd} {
    set ::pdeps {}; set ::proxy {}; set ::lic {}
    set ::message::collectedResults {}
    tclCheck::resetExecutedConstraints
    set tclCheck::currentEvent $ev
    set e0 [tclCheck::getErrorCount]; set w0 [tclCheck::getWarningCount]
    set rc [catch {tclCheck::invoke2stageConstraints $cmd} err]
    # form-level constraints for a bare call, with the loader's two-stage rule
    set g [grammar_of $cmd]
    if {$g ne ""} {
        set zc [zero_form_constraints $g]
        if {$zc ne "NOMATCH" && [llength $zc]} {
            set bk $::message::collectedResults
            set ::message::collectedResults {}
            foreach c $zc { catch {uplevel #0 [concat $c [list $cmd]]} }
            set formerr $::message::collectedResults
            set ::message::collectedResults $bk
            if {$formerr ne {}} {
                # the legacy checker's command constraints decide warn or error
                set ok 1
                if {[info exists ::BIGIP::old_ltmCommandConstraints($cmd)]} {
                    set bk2 $::message::collectedResults; set ::message::collectedResults {}
                    tclCheck::resetExecutedConstraints
                    catch {tclCheck::invokeConstraints ::BIGIP::old_ltmCommandConstraints ::BIGIP::old_ltmNamespaceConstraints $cmd}
                    if {[regexp {badEventContext} $::message::collectedResults]} { set ok 0 }
                    set ::message::collectedResults $bk2
                } else { set ok 0 }
                if {$ok} { tclCheck::logError warnOldCheckerFlaw {} $formerr } else { foreach m $formerr { lappend ::message::collectedResults $m } }
            }
        }
    }
    # the loader's fallback when no explicit event constraint ran for the command
    set implicit 0
    if {![tclCheck::constraintExecuted $cmd COMMAND_EVENT] && [::checkImplicitEventConstraints $cmd] != 0} {
        set implicit 1
        tclCheck::logError badEventContext {} $ev
    }
    set de [expr {[tclCheck::getErrorCount] - $e0}]; set dw [expr {[tclCheck::getWarningCount] - $w0}]
    return [list implicit $implicit rc $rc err [string range $err 0 100] errors $de warnings $dw msgs $::message::collectedResults profiles $::pdeps proxy $::proxy licence $::lic]
}
proc verdict {r} {
    set v valid
    foreach m [dict get $r msgs] {
        set mid [lindex $m 0]
        if {$mid eq "badEventContext"} { return invalid }
        if {$mid eq "badRuleInitContext"} { set v invalid-warn }
        if {$mid eq "warnOldCheckerFlaw" && [regexp {badEventContext|badRuleInitContext} [lindex $m 1]]} { set v invalid-warn }
    }
    return $v
}
proc clean {s} { regsub -all {\s+} $s " " }
set out [open [lindex $argv 1] w]
set events [lsort [array names ::BIGIP::event_table]]
set cmds [lsort [array names ::BIGIP::bigip_rule_syntax]]
foreach ev $events {
    foreach cmd $cmds {
        set r [probe $ev $cmd]
        set prof {}
        foreach p [dict get $r profiles] { lappend prof [clean [lindex $p 0]] }
        puts $out [join [list $ev $cmd [verdict $r] [join [lsort -unique $prof] " ; "] [join [lsort -unique [dict get $r licence]] " "] [clean [dict get $r proxy]]] "\t"]
    }
}
close $out
set dir [file dirname [lindex $argv 1]]
set f [open $dir/events.tsv w]
foreach ev $events {
    set cats {}; set profs {}
    foreach entry $::BIGIP::event_table($ev) {
        switch -- [lindex $entry 0] { event_categories { set cats [lindex $entry 1] } profiles { set profs [lindex $entry 1] } }
    }
    puts $f "$ev\t[clean $cats]\t[clean $profs]"
}
close $f
set f [open $dir/hierarchy.tsv w]
foreach c [lsort [array names ::BIGIP::ltmEventCategoryHierarchy]] { puts $f "$c\t[clean $::BIGIP::ltmEventCategoryHierarchy($c)]" }
close $f
set f [open $dir/cat_events.tsv w]
foreach c [lsort [array names ::BIGIP::ltmCategoryToEvents]] { puts $f "$c\t[clean $::BIGIP::ltmCategoryToEvents($c)]" }
close $f
set f [open $dir/nsmap.tsv w]
foreach n [lsort [array names ::BIGIP::ltmNamespaceToEvents]] { puts $f "$n\t[clean $::BIGIP::ltmNamespaceToEvents($n)]" }
close $f
set f [open $dir/disabled.tsv w]
puts $f [clean $::BIGIP::disabled_cmd_list]
close $f
puts "events [llength $events] commands [llength $cmds] missing: [dict keys $::missing]"
