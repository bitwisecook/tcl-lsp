namespace eval HTTP {proc path {} {return /A}; proc method {} {return GET}}
namespace eval IP {proc client_addr {} {return 192.0.2.1}}
foreach script [list {[HTTP::path]} {[{HTTP::path}]} {["HTTP::path"]} {[HTTP\x3a\x3apath]} {[string tolower [HTTP::path]]} {[string {tolower} [HTTP::path]]} {[string tolower {[HTTP::path]}]} {[HTTP::path extra]} {[HTTP::path; HTTP::method]} {[HTTP::path]suffix}] {
  set code [catch {subst $script} result]
  puts [list $script $code $result]
}
