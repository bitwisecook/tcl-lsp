source [binary format H* 2f776f726b73706163652f74636c2d6c73702f746d702f6a696d74636c2d6f7261636c652f74657374732f74657374696e672e74636c]

set background_errors {}
proc bgerror {message} {
    lappend ::background_errors [list $message $::errorCode $::errorInfo]
}
set setup [binary format H* 696e7465727020616c696173207b7d203a3a6d616b65507265666978207b7d203a3a6c6973740a6f6f3a3a636c617373206372656174652043207b0a202020206d6574686f64207469636b207b66697865647d207b736574203a3a6f62736572766564205b6c697374207469636b202466697865645d7d0a202020206d6574686f64207365747570207b7d207b0a2020202020202020736574206362205b6e616d65737061636520636f6465205b3a3a6d616b65507265666978206d79207469636b207b6120627d5d5d0a202020202020202061667465722069646c65202463620a202020207d0a7d]
set question [binary format H* 4320637265617465206f626a6563743b206f626a6563742073657475703b207570646174653b20736574203a3a6f62736572766564]
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
if {$setup_code && [info exists ::errorCode]} {
    set setup_error_code $::errorCode
    set setup_error_info $::errorInfo
}
set question_code not-run
set question_result {}
if {!$setup_code} {
    if {$modern} {
        set question_code [catch $question question_result question_options]
    } else {
        set question_code [catch $question question_result]
    }
    if {$question_code && [info exists ::errorCode]} {
        set question_error_code $::errorCode
        set question_error_info $::errorInfo
    }
}
set observed_exists [info exists ::observed]
set observed {}
if {$observed_exists} {set observed $::observed}
foreach field {setup_code setup_result setup_options setup_error_code setup_error_info question_code question_result question_options question_error_code question_error_info background_errors observed_exists observed} {
    binary scan [set $field] H* bytes
    puts [list FIELD $field $bytes]
}
