set { with space } 7
set {$literal} 7
set {[literal]} 7
set {} 7
set café 7
set {a{b}c} 7
set arr(key) 7
foreach reference [list {${ with space }} {${$literal}} {${[literal]}} {${}} {$café} {${a{b}c}} {$arr(key)} {${arr(key)}}] {
    set nested "\[expr \{$reference\}\] + 1"
    set direct "$reference + 1"
    set nested_code [catch {expr $nested} nested_value]
    set direct_code [catch {expr $direct} direct_value]
    if {$nested_code == 0 && $direct_code == 0} {
        puts [list $reference $nested_code $direct_code $nested_value $direct_value]
    } else {
        puts [list $reference $nested_code $direct_code ERROR ERROR]
    }
}
