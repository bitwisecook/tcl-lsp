# cmd_pkg.tcl -- `tclpkg pkg` subcommand handlers.

namespace eval ::tclpkg::cmd_pkg {

    proc dispatch {args_list} {
        if {[llength $args_list] < 1} {
            usage
            exit 2
        }
        set subcmd [lindex $args_list 0]
        set rest [lrange $args_list 1 end]
        switch -- $subcmd {
            init     { cmd_init $rest }
            add      { cmd_add $rest }
            remove   { cmd_remove $rest }
            install  { cmd_install $rest }
            list     { cmd_list $rest }
            tree     { cmd_tree $rest }
            verify   { cmd_verify $rest }
            info     { cmd_info $rest }
            search   { cmd_search $rest }
            freeze   { cmd_freeze $rest }
            sync     { cmd_sync $rest }
            outdated { cmd_outdated $rest }
            why      { cmd_why $rest }
            vendor   { cmd_vendor $rest }
            run      { cmd_run $rest }
            --help - -h - help { usage; exit 0 }
            default {
                puts stderr "error: unknown pkg subcommand '$subcmd'"
                usage
                exit 2
            }
        }
    }

    proc usage {} {
        puts stderr "Usage: tclpkg pkg <subcommand> \[options\]"
        puts stderr ""
        puts stderr "Subcommands:"
        puts stderr "  init      Create a new tclpkg.tcl manifest"
        puts stderr "  add       Add a dependency to the manifest"
        puts stderr "  remove    Remove a dependency from the manifest"
        puts stderr "  install   Resolve, fetch, and lock packages"
        puts stderr "  list      List installed packages"
        puts stderr "  tree      Show dependency tree"
        puts stderr "  verify    Check integrity hashes"
        puts stderr "  info      Show details for a package"
        puts stderr "  search    Search the package registry"
        puts stderr "  freeze    Output locked versions as manifest directives"
        puts stderr "  sync      Install from lockfile only (alias for install --frozen)"
        puts stderr "  outdated  Show packages with newer versions available"
        puts stderr "  why       Explain why a package is in the graph"
        puts stderr "  vendor    Copy packages into the project tree"
        puts stderr "  run       Run the manifest entry point"
    }

    proc _find_manifest {} {
        set dir [pwd]
        for {set i 0} {$i < 20} {incr i} {
            set path [file join $dir tclpkg.tcl]
            if {[file isfile $path]} { return $path }
            set parent [file dirname $dir]
            if {$parent eq $dir} break
            set dir $parent
        }
        return [file join [pwd] tclpkg.tcl]
    }

    proc cmd_init {args_list} {
        set name [file tail [pwd]]
        set version "0.1.0"
        set license "MIT"
        set tcl ">=8.6"
        foreach {k v} $args_list {
            switch -- $k {
                --name    { set name $v }
                --version { set version $v }
                --license { set license $v }
                --tcl     { set tcl $v }
            }
        }
        set path [file join [pwd] tclpkg.tcl]
        if {[file exists $path] && "--force" ni $args_list} {
            puts stderr "error: $path already exists (use --force to overwrite)"
            exit 1
        }
        set content "package     $name\nversion     $version\nlicense     $license\ntcl         $tcl\n"
        set fd [open [file join $path] w]
        fconfigure $fd -encoding utf-8 -translation lf
        puts -nonewline $fd $content
        close $fd
        puts [::tclpkg::ui::ok "wrote $path"]
    }

    proc cmd_add {args_list} {
        if {[llength $args_list] < 1} {
            puts stderr "error: usage: tclpkg pkg add <package> \[version\] \[--dev\] \[--source URL\]"
            exit 2
        }
        set pkg_name [lindex $args_list 0]
        set min_ver "0.0.1"
        if {[llength $args_list] >= 2 && ![string match "--*" [lindex $args_list 1]]} {
            set min_ver [lindex $args_list 1]
        }
        set is_dev [expr {"--dev" in $args_list}]
        set source ""
        set idx [lsearch -exact $args_list "--source"]
        if {$idx >= 0} { set source [lindex $args_list $idx+1] }

        set mpath [_find_manifest]
        if {![file isfile $mpath]} {
            puts stderr "error: manifest not found: $mpath"
            exit 1
        }
        set fd [open [file join $mpath] a]
        fconfigure $fd -encoding utf-8 -translation lf
        set directive [expr {$is_dev ? "dev-require" : "require"}]
        set line "$directive $pkg_name $min_ver"
        if {$source ne ""} { set line "$line -source $source" }
        puts $fd $line
        close $fd
        puts [::tclpkg::ui::ok "added $pkg_name $min_ver to $mpath"]
        puts [::tclpkg::ui::dim "  run 'tclpkg pkg install' to resolve and lock"]
    }

    proc cmd_remove {args_list} {
        if {[llength $args_list] < 1} {
            puts stderr "error: usage: tclpkg pkg remove <package>"
            exit 2
        }
        set pkg_name [lindex $args_list 0]
        set mpath [_find_manifest]
        if {![file isfile $mpath]} {
            puts stderr "error: manifest not found: $mpath"
            exit 1
        }
        set fd [open [file join $mpath] r]
        set text [read $fd]
        close $fd
        set new_lines {}
        set removed 0
        foreach line [split $text "\n"] {
            set trimmed [string trimleft $line]
            if {[string match "require $pkg_name *" $trimmed] ||
                [string match "require $pkg_name" $trimmed] ||
                [string match "dev-require $pkg_name *" $trimmed] ||
                [string match "dev-require $pkg_name" $trimmed]} {
                incr removed
            } else {
                lappend new_lines $line
            }
        }
        if {$removed == 0} {
            puts stderr "error: package '$pkg_name' not found in manifest"
            exit 1
        }
        set fd [open [file join $mpath] w]
        fconfigure $fd -encoding utf-8 -translation lf
        puts -nonewline $fd [join $new_lines "\n"]
        close $fd
        puts [::tclpkg::ui::ok "removed $pkg_name from $mpath"]
    }

    proc cmd_install {args_list} {
        set mpath [_find_manifest]
        set manifest [::tclpkg::manifest::load_file $mpath]
        set direct {}
        foreach req [dict get $manifest requires] {
            lappend direct [dict create name [dict get $req name] minimum [dict get $req minimum]]
        }
        set dev_direct {}
        foreach req [dict get $manifest dev_requires] {
            lappend dev_direct [dict create name [dict get $req name] minimum [dict get $req minimum]]
        }
        set replaces {}
        foreach r [dict get $manifest replaces] {
            lappend replaces [dict create name [dict get $r name] version [dict get $r version]]
        }
        set excludes {}
        foreach e [dict get $manifest excludes] {
            lappend excludes [dict create name [dict get $e name] version [dict get $e version]]
        }

        set include_dev [expr {"--no-dev" ni $args_list}]
        set resolved [::tclpkg::resolver::resolve $direct $dev_direct \
            -replaces $replaces -excludes $excludes -include_dev $include_dev]

        # Build lockfile.
        set lf [::tclpkg::lockfile::new_lockfile [dict get $manifest name] [dict get $manifest tcl_constraint]]
        set pkgs {}
        foreach rp $resolved {
            lappend pkgs [dict create \
                name [dict get $rp name] \
                version [dict get $rp version] \
                source [dict create type tarball url "" subdir "" rev null] \
                integrity "" size 0 \
                requires {} provides {} license "" \
                dev [dict get $rp dev]]
        }
        dict set lf packages $pkgs

        set lockfile_path [file join [file dirname $mpath] tclpkg.lock]
        ::tclpkg::lockfile::write $lf $lockfile_path

        foreach pkg $pkgs {
            set dev_tag ""
            if {[dict get $pkg dev]} { set dev_tag " (dev)" }
            puts [::tclpkg::ui::ok "[format %-20s [dict get $pkg name]] [dict get $pkg version]$dev_tag"]
        }
        puts [::tclpkg::ui::ok "wrote $lockfile_path"]
    }

    proc cmd_list {args_list} {
        set mpath [_find_manifest]
        set lockfile_path [file join [file dirname $mpath] tclpkg.lock]
        set lf [::tclpkg::lockfile::read_ $lockfile_path]
        puts [format "%-20s %-12s %-8s %-8s" NAME VERSION KIND DEV]
        foreach pkg [lsort -command _by_name [dict get $lf packages]] {
            set kind [expr {[llength [dict get $pkg requires]] > 0 ? "trans" : "direct"}]
            set dev [expr {[dict get $pkg dev] ? "dev" : ""}]
            puts [format "%-20s %-12s %-8s %-8s" [dict get $pkg name] [dict get $pkg version] $kind $dev]
        }
    }

    proc cmd_tree {args_list} {
        set mpath [_find_manifest]
        set lockfile_path [file join [file dirname $mpath] tclpkg.lock]
        set lf [::tclpkg::lockfile::read_ $lockfile_path]
        puts [dict get $lf name]
        set pkgs [lsort -command _by_name [dict get $lf packages]]
        set n [llength $pkgs]
        for {set i 0} {$i < $n} {incr i} {
            set pkg [lindex $pkgs $i]
            set connector [expr {$i == $n - 1 ? "\u2514\u2500\u2500 " : "\u251c\u2500\u2500 "}]
            set dev_tag [expr {[dict get $pkg dev] ? " \[dev\]" : ""}]
            puts "$connector[dict get $pkg name] [dict get $pkg version]$dev_tag"
        }
    }

    proc cmd_verify {args_list} {
        set mpath [_find_manifest]
        set lockfile_path [file join [file dirname $mpath] tclpkg.lock]
        set lf [::tclpkg::lockfile::read_ $lockfile_path]
        set failures 0
        foreach pkg [dict get $lf packages] {
            set integrity [dict get $pkg integrity]
            if {$integrity ne ""} {
                puts [::tclpkg::ui::ok "[format %-20s [dict get $pkg name]] [format %-12s [dict get $pkg version]] [string range $integrity 0 29]\u2026"]
            } else {
                puts [::tclpkg::ui::warn "[format %-20s [dict get $pkg name]] [format %-12s [dict get $pkg version]] no integrity hash"]
                incr failures
            }
        }
        if {$failures > 0} {
            puts stderr "\n$failures package(s) have no integrity hash \u2014 run 'tclpkg pkg install' to populate."
            exit 1
        }
    }

    proc cmd_info {args_list} {
        if {[llength $args_list] < 1} {
            puts stderr "error: usage: tclpkg pkg info <package>"
            exit 2
        }
        set pkg_name [lindex $args_list 0]
        set mpath [_find_manifest]
        set lockfile_path [file join [file dirname $mpath] tclpkg.lock]
        set lf [::tclpkg::lockfile::read_ $lockfile_path]
        set entry [::tclpkg::lockfile::lookup $lf $pkg_name]
        if {$entry eq ""} {
            puts stderr "error: package '$pkg_name' not found in lockfile"
            exit 1
        }
        puts "Name:      [dict get $entry name]"
        puts "Version:   [dict get $entry version]"
        set src [dict get $entry source]
        puts "Source:    [dict get $src type] [dict get $src url]"
        set integrity [dict get $entry integrity]
        puts "Integrity: [expr {$integrity ne "" ? $integrity : "(not computed)"}]"
        set lic ""
        catch {set lic [dict get $entry license] } _
        puts "Licence:   [expr {$lic ne "" ? $lic : "(unknown)"}]"
        puts "Dev:       [expr {[dict get $entry dev] ? "yes" : "no"}]"
    }

    proc cmd_search {args_list} {
        if {[llength $args_list] < 1} {
            puts stderr "error: usage: tclpkg pkg search <query>"
            exit 2
        }
        set query [lindex $args_list 0]
        set offline [expr {"--offline" in $args_list}]
        set results [::tclpkg::registry::search $query -offline $offline]
        if {[llength $results] == 0} {
            puts "No matches found."
            return
        }
        foreach entry $results {
            set desc ""
            catch {set desc [dict get $entry description] } _
            puts [format "  %-20s  %s" [dict get $entry name] $desc]
        }
    }

    proc cmd_freeze {args_list} {
        set mpath [_find_manifest]
        set lockfile_path [file join [file dirname $mpath] tclpkg.lock]
        set lf [::tclpkg::lockfile::read_ $lockfile_path]
        foreach pkg [lsort -command _by_name [dict get $lf packages]] {
            set directive [expr {[dict get $pkg dev] ? "dev-require" : "require"}]
            set source_suffix ""
            set src [dict get $pkg source]
            set url [dict get $src url]
            if {$url ne ""} { set source_suffix " -source $url" }
            puts "$directive [dict get $pkg name] [dict get $pkg version]$source_suffix"
        }
    }

    proc cmd_sync {args_list} {
        set mpath [_find_manifest]
        set lockfile_path [file join [file dirname $mpath] tclpkg.lock]
        set lf [::tclpkg::lockfile::read_ $lockfile_path]
        foreach pkg [lsort -command _by_name [dict get $lf packages]] {
            puts [::tclpkg::ui::ok "[format %-20s [dict get $pkg name]] [dict get $pkg version]"]
        }
        puts [::tclpkg::ui::ok "synced from $lockfile_path ([llength [dict get $lf packages]] packages)"]
    }

    proc cmd_outdated {args_list} {
        set mpath [_find_manifest]
        set lockfile_path [file join [file dirname $mpath] tclpkg.lock]
        set lf [::tclpkg::lockfile::read_ $lockfile_path]
        puts [format "%-20s %-12s %-12s" NAME CURRENT LATEST]
        foreach pkg [lsort -command _by_name [dict get $lf packages]] {
            puts [format "%-20s %-12s %-12s" [dict get $pkg name] [dict get $pkg version] [dict get $pkg version]]
        }
        puts [::tclpkg::ui::dim "\n  (registry version lookup not yet wired)"]
    }

    proc cmd_why {args_list} {
        if {[llength $args_list] < 1} {
            puts stderr "error: usage: tclpkg pkg why <package>"
            exit 2
        }
        set pkg_name [lindex $args_list 0]
        set mpath [_find_manifest]
        set lockfile_path [file join [file dirname $mpath] tclpkg.lock]
        set lf [::tclpkg::lockfile::read_ $lockfile_path]
        set entry [::tclpkg::lockfile::lookup $lf $pkg_name]
        if {$entry eq ""} {
            puts stderr "error: package '$pkg_name' not found in lockfile"
            exit 1
        }
        puts "$pkg_name [dict get $entry version]"
        set dependents {}
        foreach other [dict get $lf packages] {
            foreach req [dict get $other requires] {
                if {[string match "*$pkg_name*" $req]} {
                    lappend dependents [dict get $other name]
                }
            }
        }
        if {[llength $dependents] > 0} {
            foreach dep $dependents {
                puts "\u2514\u2500\u2500 required by $dep"
            }
        } elseif {[dict get $entry dev]} {
            puts "\u2514\u2500\u2500 direct dev-require dependency"
        } else {
            puts "\u2514\u2500\u2500 direct dependency"
        }
    }

    proc cmd_vendor {args_list} {
        set vendor_dir "vendor"
        set idx [lsearch -exact $args_list "--dir"]
        if {$idx >= 0} { set vendor_dir [lindex $args_list $idx+1] }
        set mpath [_find_manifest]
        set lockfile_path [file join [file dirname $mpath] tclpkg.lock]
        set lf [::tclpkg::lockfile::read_ $lockfile_path]
        file mkdir $vendor_dir
        set vendored 0
        foreach pkg [dict get $lf packages] {
            set integrity [dict get $pkg integrity]
            if {$integrity ne "" && [::tclpkg::cas::has $integrity]} {
                set dest [file join $vendor_dir "[dict get $pkg name]-[dict get $pkg version]"]
                ::tclpkg::cas::materialise $integrity $dest -symlink 0
                puts [::tclpkg::ui::ok "vendored [dict get $pkg name]"]
                incr vendored
            }
        }
        if {$vendored == 0} {
            puts [::tclpkg::ui::dim "  no packages with integrity hashes to vendor"]
        } else {
            puts [::tclpkg::ui::ok "wrote $vendored package(s) to $vendor_dir/"]
        }
    }

    proc cmd_run {args_list} {
        set mpath [_find_manifest]
        set manifest [::tclpkg::manifest::load_file $mpath]
        set entry_file [dict get $manifest entry]
        if {$entry_file eq ""} {
            puts stderr "error: no 'entry' directive in manifest"
            exit 1
        }
        set entry_path [file join [file dirname $mpath] $entry_file]
        if {![file isfile $entry_path]} {
            puts stderr "error: entry file not found: $entry_path"
            exit 1
        }
        set tclsh [::tclpkg::venv::find_tclsh]
        set lib_dir [file join [file dirname $mpath] lib]
        if {[file isdirectory $lib_dir]} {
            set ::env(TCLLIBPATH) "$lib_dir [expr {[info exists ::env(TCLLIBPATH)] ? $::env(TCLLIBPATH) : ""}]"
        }
        exec -- $tclsh $entry_path {*}$args_list >@ stdout 2>@ stderr
    }

    proc _by_name {a b} {
        return [string compare [dict get $a name] [dict get $b name]]
    }
}
