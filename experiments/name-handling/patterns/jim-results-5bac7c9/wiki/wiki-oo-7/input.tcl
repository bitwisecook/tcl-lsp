source [binary format H* 2f776f726b73706163652f74636c2d6c73702f746d702f6a696d74636c2d6f7261636c652f74657374732f74657374696e672e74636c]
set setup {}
set question [binary format H* 0a20202020202020206f6f3a3a636c617373206372656174652043207b6d6574686f64202e73656e64207b7d207b72657475726e206f6c647d3b206578706f7274202e73656e647d0a2020202020202020736574206f626a205b43206e65775d0a2020202020202020736574206265666f7265205b246f626a202e73656e645d0a20202020202020206f6f3a3a646566696e652043206d6574686f64202e73656e64207b7d207b72657475726e207265706c6163656d656e747d0a20202020202020206c69737420246265666f7265205b6361746368207b246f626a202e73656e647d5d0a20202020]
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
