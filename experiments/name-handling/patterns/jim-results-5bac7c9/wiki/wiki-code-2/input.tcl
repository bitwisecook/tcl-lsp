source [binary format H* 2f776f726b73706163652f74636c2d6c73702f746d702f6a696d74636c2d6f7261636c652f74657374732f74657374696e672e74636c]
set setup {}
set question [binary format H* 0a20202020202020206e616d657370616365206576616c206e207b0a2020202020202020202020207661726961626c652078206e616d6573706163650a20202020202020202020202070726f63206275696c64207b7d207b7365742078206c6f63616c3b206e616d65737061636520636f6465207b73657420787d7d0a20202020202020207d0a20202020202020206576616c205b6e3a3a6275696c645d0a20202020]
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
