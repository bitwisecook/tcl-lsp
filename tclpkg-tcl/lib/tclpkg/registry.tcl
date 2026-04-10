# registry.tcl — tcltk-pkgs registry client with TTL cache.

namespace eval ::tclpkg::registry {

    variable default_url "https://raw.githubusercontent.com/tcltk-pkgs/registry/main/packages.json"
    variable ttl_seconds [expr {24 * 3600}]

    proc registry_dir {} {
        return [file join [::tclpkg::cas::cache_dir] tclpkg registry]
    }

    # Fetch the registry, using cache when fresh.
    proc fetch {args} {
        set url $::tclpkg::registry::default_url
        set offline 0
        set force 0
        foreach {k v} $args {
            switch -- $k {
                -url     { set url $v }
                -offline { set offline $v }
                -force   { set force $v }
            }
        }

        set dir [registry_dir]
        set cached [file join $dir packages.json]
        set etag_path [file join $dir packages.json.etag]
        set fetched_path [file join $dir fetched_at]

        if {$offline} {
            return [_load_cached $cached]
        }

        if {!$force && [_is_fresh $fetched_path] && [file isfile $cached]} {
            return [_load_cached $cached]
        }

        # Network fetch.
        package require http
        catch { package require tls; ::http::register https 443 ::tls::socket }

        set headers {}
        if {[file isfile $etag_path]} {
            set fd [open $etag_path r]
            set old_etag [string trim [read $fd]]
            close $fd
            if {$old_etag ne ""} {
                lappend headers If-None-Match $old_etag
            }
        }

        set code [catch {
            set token [::http::geturl $url -timeout 30000 -headers $headers]
        } err]

        if {$code} {
            if {[file isfile $cached]} {
                return [_load_cached $cached]
            }
            error "cannot reach registry at $url: $err"
        }

        set status [::http::ncode $token]
        if {$status == 304} {
            ::http::cleanup $token
            file mkdir $dir
            _write_file $fetched_path [clock format [clock seconds] -format "%Y-%m-%dT%H:%M:%SZ" -gmt 1]
            return [_load_cached $cached]
        }

        if {$status != 200} {
            ::http::cleanup $token
            if {[file isfile $cached]} {
                return [_load_cached $cached]
            }
            error "registry fetch failed: HTTP $status"
        }

        set body [::http::data $token]
        set new_etag ""
        catch { set new_etag [dict get [::http::meta $token] ETag] }
        ::http::cleanup $token

        file mkdir $dir
        _write_file $cached $body
        _write_file $etag_path $new_etag
        _write_file $fetched_path [clock format [clock seconds] -format "%Y-%m-%dT%H:%M:%SZ" -gmt 1]

        return [_parse $body]
    }

    proc _is_fresh {fetched_path} {
        variable ttl_seconds
        if {![file isfile $fetched_path]} { return 0 }
        set fd [open $fetched_path r]
        set ts [string trim [read $fd]]
        close $fd
        if {[catch {clock scan $ts -format "%Y-%m-%dT%H:%M:%SZ" -gmt 1} epoch]} {
            return 0
        }
        return [expr {[clock seconds] - $epoch < $ttl_seconds}]
    }

    proc _load_cached {path} {
        if {![file isfile $path]} {
            error "no cached registry (run with network access first)"
        }
        set fd [open $path r]
        fconfigure $fd -encoding utf-8
        set text [read $fd]
        close $fd
        return [_parse $text]
    }

    proc _parse {text} {
        package require json
        return [::json::json2dict $text]
    }

    proc _write_file {path content} {
        set fd [open $path w]
        fconfigure $fd -encoding utf-8 -translation lf
        puts -nonewline $fd $content
        close $fd
    }

    # Search cached entries by name/description/tag.
    proc search {query args} {
        set entries [fetch {*}$args]
        set q [string tolower $query]
        set results {}
        foreach entry $entries {
            set name [dict get $entry name]
            set desc ""
            catch { set desc [dict get $entry description] }
            set tags {}
            catch { set tags [dict get $entry tags] }
            set match 0
            if {[string match "*$q*" [string tolower $name]]} { set match 1 }
            if {[string match "*$q*" [string tolower $desc]]} { set match 1 }
            foreach tag $tags {
                if {[string match "*$q*" [string tolower $tag]]} { set match 1 }
            }
            if {$match} { lappend results $entry }
        }
        return $results
    }

    # Look up a single entry by exact name.
    proc lookup {name args} {
        set entries [fetch {*}$args]
        foreach entry $entries {
            if {[dict get $entry name] eq $name} {
                return $entry
            }
        }
        return ""
    }
}
