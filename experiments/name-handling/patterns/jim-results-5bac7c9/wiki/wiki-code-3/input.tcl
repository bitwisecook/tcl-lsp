source [binary format H* 2f776f726b73706163652f74636c2d6c73702f746d702f6a696d74636c2d6f7261636c652f74657374732f74657374696e672e74636c]
set setup {}
set question [binary format H* 0a20202020202020206e616d657370616365206576616c206e207b70726f632074616b65207b7d207b72657475726e206f6c647d3b20736574203a3a707265666978205b6e616d65737061636520636f64652074616b655d7d0a202020202020202072656e616d65206e3a3a74616b65206e3a3a73617665640a202020202020202070726f63206e3a3a74616b65207b7d207b72657475726e207265706c6163656d656e747d0a20202020202020206576616c20247072656669780a20202020]
set setup_code [catch $setup setup_result setup_options]
set question_code not-run
set question_result {}
set question_options {}
if {!$setup_code} {
    set question_code [catch $question question_result question_options]
}
foreach field {setup_code setup_result setup_options question_code question_result question_options} {
    binary scan [set $field] H* bytes
    puts [list FIELD $field $bytes]
}
puts [list JIMTEST [array get ::testinfo]]
