# cas.tcl -- content-addressable store and integrity hashing.
#
# Bit-compatible with tclpkg/cas.py: same canonicalisation, same hash
# format (sha256-<base64url-no-pad>), same CAS directory layout.

package require sha256

namespace eval ::tclpkg::cas {

    variable ignored_names {.git .hg .svn .fossil .DS_Store Thumbs.db}

    # Compute the canonical SHA-256 integrity hash of a directory tree.
    proc integrity_of_tree {root} {
        variable ignored_names
        set extra [_load_tclpkgignore $root]
        set all_ignored [concat $ignored_names $extra]
        set files [_walk_tree $root $all_ignored]
        set ctx [::sha2::SHA256Init]
        foreach rel_path $files {
            set abs [file join $root $rel_path]
            if {![file isfile $abs]} continue
            set content [_read_binary $abs]
            set mode [_file_mode $abs]
            set size [string length $content]
            set file_hash [::sha2::sha256 -hex $content]
            ::sha2::SHA256Update $ctx "${rel_path}\n"
            ::sha2::SHA256Update $ctx "[format %o $mode]\n"
            ::sha2::SHA256Update $ctx "${size}\n"
            ::sha2::SHA256Update $ctx "${file_hash}\n"
            ::sha2::SHA256Update $ctx $content
        }
        set digest [::sha2::SHA256Final $ctx]
        set b64 [_base64url_encode $digest]
        return "sha256-$b64"
    }

    proc verify_integrity {root expected} {
        set actual [integrity_of_tree $root]
        return [expr {$actual eq $expected}]
    }

    proc _load_tclpkgignore {root} {
        set path [file join $root .tclpkgignore]
        if {![file isfile $path]} { return {} }
        set fd [open [file join $path] r]
        set text [read $fd]
        close $fd
        set patterns {}
        foreach line [split $text "\n"] {
            set stripped [string trim $line]
            if {$stripped ne "" && ![string match "#*" $stripped]} {
                lappend patterns $stripped
            }
        }
        return $patterns
    }

    proc _walk_tree {root ignored} {
        set result {}
        _walk_recurse $root "" $ignored result
        return [lsort $result]
    }

    proc _walk_recurse {root prefix ignored result_var} {
        upvar 1 $result_var result
        set entries [lsort [glob -nocomplain -directory [file join $root $prefix] *]]
        foreach entry $entries {
            set name [file tail $entry]
            if {$name in $ignored} continue
            set rel [expr {$prefix eq "" ? $name : "$prefix/$name"}]
            if {[file isdirectory $entry]} {
                _walk_recurse $root $rel $ignored result
            } else {
                lappend result $rel
            }
        }
    }

    proc _read_binary {path} {
        set fd [open [file join $path] rb]
        set data [read $fd]
        close $fd
        return $data
    }

    proc _file_mode {path} {
        # Approximate: return 0644 for regular, 0755 for executable.
        if {[file executable $path]} {
            return 0755
        }
        return 0644
    }

    proc _base64url_encode {binary_data} {
        package require base64
        set b64 [::base64::encode -maxlen 0 $binary_data]
        # Convert to base64url: + -> -, / -> _, strip trailing =.
        set b64url [string map {+ - / _} $b64]
        set b64url [string trimright $b64url "="]
        return $b64url
    }

    # CAS store operations.

    proc cache_dir {} {
        if {[info exists ::env(XDG_CACHE_HOME)] && $::env(XDG_CACHE_HOME) ne ""} {
            return [file join $::env(XDG_CACHE_HOME) tcl-lsp]
        }
        if {$::tcl_platform(os) eq "Darwin"} {
            return [file join [file normalize ~] Library Caches tcl-lsp]
        }
        return [file join [file normalize ~] .cache tcl-lsp]
    }

    proc cas_base {} {
        return [file join [cache_dir] tclpkg cas sha256]
    }

    proc _entry_dir {integrity} {
        if {![string match "sha256-*" $integrity]} {
            error "unsupported integrity format: $integrity"
        }
        set b64 [string range $integrity 7 end]
        # Re-pad for base64 decoding.
        set pad [expr {(4 - [string length $b64] % 4) % 4}]
        append b64 [string repeat "=" $pad]
        set b64std [string map {- + _ /} $b64]
        package require base64
        set raw [::base64::decode $b64std]
        binary scan $raw H* hex
        set shard [string range $hex 0 1]
        return [file join [cas_base] $shard $hex]
    }

    proc has {integrity} {
        set tree [file join [_entry_dir $integrity] tree]
        return [file isdirectory $tree]
    }

    proc tree_path {integrity} {
        set tree [file join [_entry_dir $integrity] tree]
        if {![file isdirectory $tree]} {
            error "CAS entry missing for $integrity (run 'tclpkg pkg install' to populate)"
        }
        return $tree
    }

    proc store {source_dir args} {
        set name ""
        set version ""
        set expected ""
        foreach {k v} $args {
            switch -- $k {
                -name     { set name $v }
                -version  { set version $v }
                -expected { set expected $v }
            }
        }
        set actual [integrity_of_tree $source_dir]
        if {$expected ne "" && $actual ne $expected} {
            error "integrity mismatch for ${name}@${version}: expected $expected, got $actual"
        }
        set entry [_entry_dir $actual]
        set tree [file join $entry tree]
        if {[file isdirectory $tree]} {
            return $actual
        }
        file mkdir $entry
        if {$name ne ""} {
            set fd [open [file join $entry package.txt] w]
            puts $fd "$name $version"
            close $fd
        }
        file copy -- $source_dir $tree
        return $actual
    }

    proc materialise {integrity dest args} {
        set use_symlink 1
        foreach {k v} $args {
            if {$k eq "-symlink"} { set use_symlink $v }
        }
        set tree [tree_path $integrity]
        if {[file exists $dest]} {
            file delete -force -- $dest
        }
        file mkdir [file dirname $dest]
        if {$use_symlink} {
            if {![catch {file link -symbolic $dest $tree}]} {
                return
            }
        }
        file copy -- $tree $dest
    }
}
