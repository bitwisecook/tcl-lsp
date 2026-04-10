# fetchers.tcl — package source fetchers (HTTP, git, path).
#
# Uses tcllib http for HTTPS and the host git binary via exec.

namespace eval ::tclpkg::fetchers {

    # Fetch a tarball via HTTP and extract into dest.
    proc fetch_tarball {url dest args} {
        set timeout 60
        foreach {k v} $args {
            if {$k eq "-timeout"} { set timeout $v }
        }
        package require http
        catch { package require tls; ::http::register https 443 ::tls::socket }

        file mkdir $dest
        set tmp [file join $dest ".download.tmp"]

        set token [::http::geturl $url -timeout [expr {$timeout * 1000}]]
        set status [::http::status $token]
        set code [::http::ncode $token]
        if {$status ne "ok" || $code != 200} {
            set err "HTTP $code fetching $url"
            ::http::cleanup $token
            error $err
        }
        set body [::http::data $token]
        ::http::cleanup $token

        set fd [open $tmp wb]
        puts -nonewline $fd $body
        close $fd

        # Extract based on extension.
        set lower [string tolower $url]
        if {[string match "*.zip" $lower]} {
            _extract_zip $tmp $dest
        } else {
            _extract_tar $tmp $dest
        }
        file delete -force $tmp

        # Strip singular top-level directory.
        set children [glob -nocomplain -directory $dest *]
        if {[llength $children] == 1 && [file isdirectory [lindex $children 0]]} {
            set child [lindex $children 0]
            foreach item [glob -nocomplain -directory $child *] {
                file rename $item [file join $dest [file tail $item]]
            }
            file delete -force $child
        }
    }

    proc _extract_tar {archive dest} {
        # Use system tar; Tcl's built-in zlib + tar module is spotty.
        set rc [catch {exec tar xf $archive -C $dest 2>@stderr} err]
        if {$rc} {
            error "tar extraction failed: $err"
        }
    }

    proc _extract_zip {archive dest} {
        set rc [catch {exec unzip -o -q $archive -d $dest 2>@stderr} err]
        if {$rc} {
            error "zip extraction failed: $err"
        }
    }

    # Clone a git repository and return the resolved commit SHA.
    proc fetch_git {url dest args} {
        set rev ""
        set timeout 120
        foreach {k v} $args {
            switch -- $k {
                -rev     { set rev $v }
                -timeout { set timeout $v }
            }
        }
        # Strip git+ prefix.
        if {[string match "git+*" $url]} {
            set url [string range $url 4 end]
        }
        # Strip @rev suffix if embedded.
        if {$rev eq "" && [string match "*@*" $url]} {
            set idx [string last "@" $url]
            set rev [string range $url $idx+1 end]
            set url [string range $url 0 $idx-1]
        }

        file mkdir $dest
        set cmd [list git clone --depth 1]
        if {$rev ne ""} {
            lappend cmd --branch $rev
        }
        lappend cmd $url $dest

        set rc [catch {exec {*}$cmd 2>@stderr} err]
        if {$rc} {
            error "git clone failed: $err"
        }

        # Get SHA.
        set sha ""
        catch {
            set sha [string trim [exec git -C $dest rev-parse HEAD]]
        }

        # Strip .git.
        file delete -force [file join $dest .git]

        return $sha
    }

    # Copy a local directory into dest.
    proc fetch_path {source dest} {
        if {![file isdirectory $source]} {
            error "local path not found: $source"
        }
        file mkdir $dest
        foreach item [glob -nocomplain -directory $source *] {
            file copy -force $item [file join $dest [file tail $item]]
        }
    }
}
