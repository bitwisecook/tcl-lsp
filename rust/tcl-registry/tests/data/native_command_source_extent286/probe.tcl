# Original ASCII source. Every literal space and tab in the specimen bodies is significant.
proc emit_hex {tag code value} {
    binary scan $value H* hex
    puts [list $tag $code $hex]
}
puts [list META [info patchlevel]]
foreach {tag script} {
    EOF_SPACE { error marker   }
    SEMICOLON_SPACE { error marker   ; set unreachable 1}
    NEWLINE_SPACE { error marker   
set unreachable 1}
    QUOTED_SPACE { error "marker"   }
    BRACED_SPACE { error {marker}   }
    TAB_SPACE {	error marker	 	}
    CONTINUATION_SPACE { error marker \
   }
} {
    binary scan $script H* source_hex
    puts [list SOURCE $tag $source_hex]
    set code [catch {eval $script} result]
    emit_hex RESULT_$tag $code $result
    set info_code [catch {set ::errorInfo} error_info]
    emit_hex ERRORINFO_$tag $info_code $error_info
}
proc source_error {} { error proc_marker   }
set code [catch {source_error} result]
emit_hex PROC_RESULT $code $result
set info_code [catch {set ::errorInfo} error_info]
emit_hex PROC_ERRORINFO $info_code $error_info
proc source_frame_target {} {
    set description [info frame -1]
    foreach {key value} $description {
        if {[string equal $key cmd]} {
            return $value
        }
    }
    error {no cmd field}
}
proc source_frame_space {} { source_frame_target   }
proc source_frame_semicolon {} { source_frame_target   ; return done }
set code [catch {info frame} frame_availability]
emit_hex INFO_FRAME_AVAILABILITY $code $frame_availability
if {$code == 0} {
    set code [catch {source_frame_space} value]
    emit_hex FRAME_EOF_SPACE $code $value
    proc source_frame_target {} {
        set description [info frame -1]
        foreach {key value} $description {
            if {[string equal $key cmd]} {
                set ::semicolon_frame $value
            }
        }
    }
    set code [catch {source_frame_semicolon} value]
    emit_hex FRAME_SEMICOLON_RESULT $code $value
    set code [catch {set ::semicolon_frame} value]
    emit_hex FRAME_SEMICOLON_CMD $code $value
}
