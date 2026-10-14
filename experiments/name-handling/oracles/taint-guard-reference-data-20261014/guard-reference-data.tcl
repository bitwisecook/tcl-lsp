set {$literal} SAFE
set {} SAFE
set arr(key) SAFE
namespace eval qualified {}
set qualified::literal SAFE
puts [list SIGIL [string match SAFE ${$literal}] EMPTY [string match SAFE ${}] LITERAL [string match SAFE {$literal}] ELEMENT [string match SAFE $arr(key)] QUALIFIED [string first SAFE ${qualified::literal}]]
set {a{b}c} SAFE
set [format {a%cb} 123] OTHER
set status [catch {eval [format {string match SAFE %s} {${a{b}c}}]} value]
puts [list CLOSER $status $value]
