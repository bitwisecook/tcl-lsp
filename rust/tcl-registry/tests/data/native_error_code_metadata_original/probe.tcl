# Counted ASCII source: double quotes inside braced error text are literal.
proc emit_metadata {case code result} {
    binary scan $result H* result_hex
    set available [info exists ::errorCode]
    set code_hex {}
    if {$available} {
        binary scan $::errorCode H* code_hex
    }
    puts [list CASE $case code $code result_hex $result_hex errorCode_available $available errorCode_hex $code_hex]
}
set version_code [catch {info patchlevel} version_result]
binary scan $version_result H* version_hex
puts [list VERSION_QUERY code $version_code result_hex $version_hex]
set ::errorCode SEEDED_ARBITRARY
set code [catch {error {wrong # args: should be "synthetic value"}} result]
emit_metadata arbitrary-message $code $result
set ::errorCode SEEDED_GENUINE
set code [catch {set} result]
emit_metadata genuine-set-wrongargs $code $result
set ::errorCode SEEDED_EXPLICIT
set code [catch {error {wrong # args: should be "synthetic value"} {} {USER CODE}} result]
emit_metadata explicit-error-code $code $result
puts CLOSED_CASES_3
