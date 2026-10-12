source [binary format H* 2f776f726b73706163652f74636c2d6c73702f746d702f6a696d74636c2d6f7261636c652f74657374732f74657374696e672e74636c]
set setup {}
set question [binary format H* 0a202020202020202070726f632074616b65207b7d207b72657475726e206d61696e7d0a2020202020202020696e74657270206372656174652068656c7065720a2020202020202020696e74657270206576616c2068656c706572207b70726f632074616b65207b7d207b72657475726e2068656c7065727d7d0a2020202020202020696e7465727020616c696173207b7d20616c6961732068656c7065722074616b650a2020202020202020616c6961730a20202020]
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
