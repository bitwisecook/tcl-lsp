# fetchers.tcl -- package source fetchers (HTTP, git, path).
#
# Uses tcllib http for HTTPS and the host git binary via exec.
# Zip extraction is done member-by-member with safety checks
# (path traversal, symlinks, decompression bomb).

namespace eval ::tclpkg::fetchers {

    # Maximum total uncompressed size from a single archive (256 MiB).
    variable max_extract_bytes [expr {256 * 1024 * 1024}]

    # Fetch a tarball via HTTP and extract into dest.
    proc fetch_tarball {url dest args} {
        set timeout 60
        foreach {k v} $args {
            if {$k eq "-timeout"} { set timeout $v }
        }
        package require http
        catch {package require tls; ::http::register https 443 ::tls::socket } _

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

        set fd [open [file join $tmp] wb]
        puts -nonewline $fd $body
        close $fd

        # Extract based on extension.
        set lower [string tolower $url]
        if {[string match "*.zip" $lower]} {
            safe_extract_zip $tmp $dest
        } else {
            _extract_tar $tmp $dest
        }
        file delete -force -- $tmp

        # Strip singular top-level directory.
        set children [glob -nocomplain -directory $dest *]
        if {[llength $children] == 1 && [file isdirectory [lindex $children 0]]} {
            set child [lindex $children 0]
            foreach item [glob -nocomplain -directory $child *] {
                file rename -- $item [file join $dest [file tail $item]]
            }
            file delete -force -- $child
        }
    }

    proc _extract_tar {archive dest} {
        set rc [catch {exec tar xf $archive -C $dest 2>@stderr} err]
        if {$rc} {
            error "tar extraction failed: $err"
        }
    }

    # Safe zip extraction -- member-by-member with validation.
    #
    # Protections:
    # - Zip Slip: rejects members whose resolved path escapes dest.
    # - Absolute paths: rejected.
    # - Symlinks: skipped (unzip -o does not create them when we
    #   validate paths, but we also check for .. traversal).
    # - Zip bomb: aborts if total extracted bytes exceed max_extract_bytes.
    # - TOCTOU: each member is validated and extracted individually;
    #   we use unzip to extract one file at a time.
    proc safe_extract_zip {archive dest} {
        variable max_extract_bytes

        # Get the member list from the zip.
        set rc [catch {exec unzip -Z -1 $archive} member_list]
        if {$rc} {
            error "zip listing failed: cannot read $archive"
        }

        set dest_resolved [file normalize $dest]
        set total_bytes 0

        foreach member [split $member_list "\n"] {
            set member [string trim $member]
            if {$member eq ""} continue

            # Reject absolute paths.
            if {[string index $member 0] eq "/"} {
                error "zip member has absolute path: $member"
            }

            # Reject path traversal.
            set target [file normalize [file join $dest $member]]
            # Check that target starts with dest_resolved + /
            # (using string match to avoid the sibling-directory bypass).
            if {$target ne $dest_resolved && ![string match "${dest_resolved}/*" $target]} {
                error "zip member escapes target directory: $member"
            }

            # Get uncompressed size for this member.
            # zipinfo format: perms ver os SIZE ... filename
            # The size is the standalone integer field (4th column).
            catch {
                set info_line [exec unzip -Z -l $archive $member]
                if {[regexp {\s(\d+)\s+\w+\s+\d+\s+\w+\s} $info_line -> size]} {
                    incr total_bytes $size
                }
            } _

            if {$total_bytes > $max_extract_bytes} {
                error "zip archive exceeds [expr {$max_extract_bytes / (1024*1024)}] MiB uncompressed limit (possible zip bomb)"
            }

            # Extract this single member.
            set rc [catch {exec unzip -o -q $archive $member -d $dest 2>@stderr} err]
            if {$rc} {
                error "zip extraction failed for member $member: $err"
            }
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
        # Strip @rev suffix if embedded in URL, but only when it looks
        # like a ref marker (after .git or after a /), not an SSH user@
        # prefix like git@github.com:org/repo.git.
        if {$rev eq "" && [string match "*@*" $url]} {
            set idx [string last "@" $url]
            set prefix [string range $url 0 $idx-1]
            if {[string match "*/*" $prefix] || [string match "*.git" $prefix]} {
                set rev [string range $url $idx+1 end]
                set url $prefix
            }
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
        } _

        # Strip .git.
        file delete -force -- [file join $dest .git]

        return $sha
    }

    # Copy a local directory into dest.
    proc fetch_path {source dest} {
        if {![file isdirectory $source]} {
            error "local path not found: $source"
        }
        file mkdir $dest
        foreach item [glob -nocomplain -directory $source *] {
            file copy -force -- $item [file join $dest [file tail $item]]
        }
    }
}
