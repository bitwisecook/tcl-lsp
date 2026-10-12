source [binary format H* 2f776f726b73706163652f74636c2d6c73702f746d702f6a696d74636c2d6f7261636c652f74657374732f74657374696e672e74636c]

set background_errors {}
proc bgerror {message} {
    set code {}
    set detail {}
    if {[info exists ::errorCode]} {set code $::errorCode}
    if {[info exists ::errorInfo]} {set detail $::errorInfo}
    lappend ::background_errors [list $message $code $detail]
}
set setup [binary format H* 6e616d657370616365206576616c203a3a777269746572207b0a2020202070726f632061737369676e207b6e616d657d207b7570766172203120246e616d652063656c6c3b207365742063656c6c206368616e6765647d0a2020202070726f6320666f7277617264207b7072656669787d207b75706c6576656c203120247072656669787d0a2020202070726f632063616c6c6572207b7d207b0a2020202020202020736574207b6120627d206f726967696e616c0a2020202020202020666f7277617264205b6e616d65737061636520636f6465205b6c697374203a3a6469737061746368207b6120627d5d5d0a20202020202020206c697374205b736574207b6120627d5d205b6e616d657370616365206576616c203a3a777269746572207b696e666f20657869737473207b6120627d7d5d0a202020207d0a7d0a696e7465727020616c696173207b7d203a3a6469737061746368207b7d203a3a7772697465723a3a61737369676e]
set question [binary format H* 3a3a7772697465723a3a63616c6c6572]
set modern 1
set setup_options {}
set question_options {}
set setup_error_code {}
set setup_error_info {}
set question_error_code {}
set question_error_info {}
if {$modern} {
    set setup_code [catch $setup setup_result setup_options]
} else {
    set setup_code [catch $setup setup_result]
}
if {$setup_code && [info exists ::errorCode]} {set setup_error_code $::errorCode}
if {$setup_code && [info exists ::errorInfo]} {set setup_error_info $::errorInfo}
set question_code not-run
set question_result {}
if {!$setup_code} {
    if {$modern} {
        set question_code [catch $question question_result question_options]
    } else {
        set question_code [catch $question question_result]
    }
    if {$question_code && [info exists ::errorCode]} {set question_error_code $::errorCode}
    if {$question_code && [info exists ::errorInfo]} {set question_error_info $::errorInfo}
}
foreach field {setup_code setup_result setup_options setup_error_code setup_error_info question_code question_result question_options question_error_code question_error_info background_errors} {
    binary scan [set $field] H* bytes
    puts [list FIELD $field $bytes]
}
