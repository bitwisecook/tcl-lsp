# Authored simulator roster projection; regenerate with cargo xtask gen-irule-test-data.
namespace eval ::itest::cmd {
    variable _mock_names
    array set _mock_names {
        ::NS::inner::cmd ::itest::cmd::ns_cmd
        HTTP::header ::itest::cmd::http_header
        NS::cmd ::itest::cmd::ns_cmd
        NS::inner::cmd ::itest::cmd::ns_cmd
        a:b ::itest::cmd::cmd_a:b
        traffic-group ::itest::cmd::cmd_traffic_group
    }
}
