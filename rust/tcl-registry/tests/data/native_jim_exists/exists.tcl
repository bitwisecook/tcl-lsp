set exists_rows {}
proc exists_observe {label body} {
    set code [catch {uplevel 1 $body} value]
    lappend ::exists_rows [list $label $code $value]
}
proc exists_fixture {} {return fixture}
alias exists_alias exists_fixture
set exists_value present
set {} empty
set exists_dict [dict create key value]
exists_observe var-present {exists exists_value}
exists_observe var-missing {exists missing_exists_value}
exists_observe var-option {exists -v exists_value}
exists_observe var-empty {exists {}}
exists_observe dict-present {exists exists_dict(key)}
exists_observe dict-missing {exists exists_dict(missing)}
exists_observe dict-root-missing {exists missing_exists_dict(key)}
exists_observe command-proc {exists -command exists_fixture}
exists_observe command-alias {exists -command exists_alias}
exists_observe command-native {exists -command set}
exists_observe command-missing {exists -command missing_exists_command}
exists_observe proc-proc {exists -p exists_fixture}
exists_observe proc-alias {exists -proc exists_alias}
exists_observe proc-native {exists -proc set}
exists_observe alias-alias {exists -a exists_alias}
exists_observe alias-proc {exists -alias exists_fixture}
exists_observe channel-missing {exists -channel missing_exists_command}
exists_observe channel-proc {exists -channel exists_fixture}
exists_observe option-ambiguous {exists -c set}
exists_observe option-bare {exists - set}
exists_observe option-unknown {exists -missing set}
exists_observe option-nul {exists "-command\x00suffix" set}
exists_observe missing-arguments {exists}
exists_observe excess-arguments {exists -command set extra}
namespace eval exists_ns {
    proc local {} {return local}
    exists_observe namespace-local {exists -command local}
    exists_observe namespace-fallback {exists -command exists_fixture}
}
proc exists_link {variable} {upvar 1 $variable local; list [exists local] [exists -var local]}
exists_observe linked-present {exists_link exists_value}
exists_observe linked-missing {exists_link missing_exists_value}
set exists_results [join $exists_rows "\n"]
