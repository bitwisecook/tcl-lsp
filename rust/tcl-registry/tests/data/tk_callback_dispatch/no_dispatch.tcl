# Run under each release's own wish (Tk 8.4.20-9.1.0) with xvfb-run; the outputs
# beside this script are tk<version>.txt captures of that run.
# With a timer, an idle callback and a widget binding pending, run one command
# and report whether any pending callback ran before it returned.
wm geometry . 300x300+0+0; update
frame .host -width 40 -height 40; pack .host; update
proc hit {args} { lappend ::log $args }
set n 0
proc case {name cmd} {
    set ::log {}
    after 0 [list hit timer]
    after idle [list hit idle]
    bind .host <Configure> [list hit configure]
    if {[catch {uplevel #0 $cmd} err]} { puts "$name: error [string range $err 0 60]"; update; return }
    set seen $::log
    bind .host <Configure> {}
    update
    puts "$name: [expr {[llength $seen] ? "dispatches $seen" : "none"}]"
}
foreach w {button canvas checkbutton entry frame label labelframe listbox menu menubutton message panedwindow radiobutton scale scrollbar spinbox text toplevel} {
    case ctor.$w [list $w .c_$w]
}
foreach w {button combobox entry frame label labelframe notebook progressbar scale scrollbar separator sizegrip treeview spinbox checkbutton radiobutton menubutton panedwindow} {
    case ctor.ttk::$w [list ttk::$w .t_$w]
}
case pack {pack [label .pk -text x]}
case pack.forget {pack forget .pk}
case grid {pack [frame .gf]; grid [label .gf.gd -text x]}
case place {place [label .pl -text x] -x 0 -y 0}
case bind.set {bind .host <<X>> {puts x}}
case bind.query {bind .host}
case bindtags {bindtags .host}
case bell {bell}
case clipboard {clipboard clear; clipboard append x}
case event.add {event add <<Y>> <Key-y>}
case event.info {event info}
case focus.set {focus .host}
case focus.query {focus}
case font.create {font create f[incr ::n] -size 10}
case font.measure {font measure TkDefaultFont abc}
case image.create {image create photo}
case option.add {option add *foo bar}
case raise {raise .host}
case lower {lower .host}
case selection.clear {selection clear}
case winfo.exists {winfo exists .host}
case winfo.geometry {winfo geometry .host}
case wm.title {wm title . t}
case wm.geometry {wm geometry . 310x310}
case tk.scaling {tk scaling}
case tk.appname {tk appname}
case ttk.style.configure {ttk::style configure TButton -padding 1}
case configure.widget {.host configure -width 41}
case cget.widget {.host cget -width}
case entry.insert.validate-none {entry .ei; .ei insert 0 abc}
case text.insert {text .ti; .ti insert end abc}
case canvas.create {canvas .ci; .ci create line 0 0 1 1}
case listbox.insert {listbox .li; .li insert end a}
case grab.release {grab release .host}
puts "TK [package require Tk]"
exit
