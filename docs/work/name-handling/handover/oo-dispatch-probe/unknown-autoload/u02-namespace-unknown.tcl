# namespace unknown: per-namespace handler, default values, and fallback to the global unknown.
namespace eval ns {
  namespace unknown [list ::ns::handler]
  proc handler {args} { return "ns handler: $args" }
  proc try_it {} { return [missing 1 2] }
}
puts [ns::try_it]
puts "global default: [namespace unknown]"
puts "ns setting: [namespace eval ns {namespace unknown}]"
namespace eval other { proc go {} { return [catch {missing3} m]:$m } }
puts "other ns: [other::go]"
namespace eval ns { namespace unknown {} }
puts "reset: [catch {ns::try_it} m] $m"
