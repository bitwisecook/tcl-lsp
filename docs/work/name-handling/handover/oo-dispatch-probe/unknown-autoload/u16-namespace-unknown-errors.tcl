# A namespace unknown handler that itself fails; handler with extra words.
namespace eval ns1 { namespace unknown {::ns1::h fixed} ; proc h {pre args} { return "h $pre $args" } ; proc go {} { return [m a] } }
puts [ns1::go]
namespace eval ns2 { namespace unknown ::ns2::h ; proc h {args} { error "handler failed" } ; proc go {} { return [m a] } }
puts "fail: [catch {ns2::go} m] $m"
namespace eval ns3 { namespace unknown ::ns3::nohandler ; proc go {} { return [m a] } }
puts "missing handler: [catch {ns3::go} m] $m"
