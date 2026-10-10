puts [list version [info patchlevel]]
proc r2286_observe {label script} {
    set status [catch {uplevel #0 $script} result]
    if {$status} {set code $::errorCode} else {set code {}}
    puts [list observation $label status $status result $result errorCode $code]
}
r2286_observe create {interp create s}
r2286_observe original-parent-command {info commands s}
r2286_observe move {rename s moved}
r2286_observe old-parent-command {info commands s}
r2286_observe moved-parent-command {info commands moved}
r2286_observe old-parent-call {s eval {set value old}}
r2286_observe moved-parent-value {moved eval {set value moved}}
r2286_observe original-child-path-value {interp eval s {set value}}
r2286_observe original-child-path-exists {interp exists s}
r2286_observe moved-child-path-exists {interp exists moved}
r2286_observe move-again {rename moved moved_again}
r2286_observe first-moved-parent-command {info commands moved}
r2286_observe second-moved-parent-value {moved_again eval {set value}}
r2286_observe original-child-path-after-second-move {interp eval s {set value}}
r2286_observe delete-original-child-path {interp delete s}
r2286_observe second-moved-parent-after-child-delete {info commands moved_again}
r2286_observe second-moved-call-after-child-delete {moved_again eval {set value}}
r2286_observe create-delete-by-parent {interp create d}
r2286_observe move-delete-by-parent {rename d held}
r2286_observe delete-moved-parent {rename held {}}
r2286_observe child-after-parent-delete {interp exists d}
r2286_observe child-call-after-parent-delete {interp eval d {set value}}
r2286_observe create-qualified-parent {interp create q}
r2286_observe create-parent-namespace {namespace eval N {}}
r2286_observe move-qualified-parent {rename q ::N::held}
r2286_observe qualified-parent-value {::N::held eval {set value qualified}}
r2286_observe original-qualified-child-path-value {interp eval q {set value}}
r2286_observe delete-qualified-child-path {interp delete q}
r2286_observe qualified-parent-after-child-delete {info commands ::N::held}
