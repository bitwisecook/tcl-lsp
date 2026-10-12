source [binary format H* 2f776f726b73706163652f74636c2d6c73702f746d702f6a696d74636c2d6f7261636c652f74657374732f74657374696e672e74636c]
set setup {}
set question [binary format H* 0a20202020202020206e616d657370616365206576616c206f6e65207b7661726961626c652078206f6e653b2070726f632074616b65207b7d207b7661726961626c6520783b2072657475726e2024787d7d0a20202020202020206e616d657370616365206576616c2074776f207b7661726961626c6520782074776f7d0a202020202020202072656e616d65206f6e653a3a74616b652074776f3a3a74616b650a202020202020202074776f3a3a74616b650a20202020]
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
