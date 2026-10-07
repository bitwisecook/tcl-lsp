proc R2286_RUN_TOKEN_hex {value} {
    binary scan $value H* encoded
    return $encoded
}
proc R2286_RUN_TOKEN_watch {name index operation} {
    upvar 1 R2286_RUN_TOKEN_callbacks callbacks
    lappend callbacks [list $operation [R2286_RUN_TOKEN_hex $name] [R2286_RUN_TOKEN_hex $index]]
    if {$operation eq "read" || $operation eq "r"} {
        uplevel 1 {unset R2286_RUN_TOKEN_value; set R2286_RUN_TOKEN_value NEW}
    }
}
when HTTP_REQUEST {
    if {[HTTP::uri] eq "/r2286-RUN_TOKEN-backend"} {
        return
    }
    set R2286_RUN_TOKEN_rows {}
    foreach R2286_RUN_TOKEN_syntax {modern legacy} {
        catch {unset R2286_RUN_TOKEN_value}
        set R2286_RUN_TOKEN_callbacks {}
        set R2286_RUN_TOKEN_value OLD
        if {$R2286_RUN_TOKEN_syntax eq "modern"} {
            set R2286_RUN_TOKEN_install [catch {trace add variable R2286_RUN_TOKEN_value {read unset} R2286_RUN_TOKEN_watch} R2286_RUN_TOKEN_install_result]
        } else {
            set R2286_RUN_TOKEN_install [catch {trace variable R2286_RUN_TOKEN_value ru R2286_RUN_TOKEN_watch} R2286_RUN_TOKEN_install_result]
        }
        set R2286_RUN_TOKEN_code [catch {
            set R2286_RUN_TOKEN_head lappend
            set R2286_RUN_TOKEN_answer [$R2286_RUN_TOKEN_head R2286_RUN_TOKEN_value EXTRA]
            list $R2286_RUN_TOKEN_answer [set R2286_RUN_TOKEN_value]
        } R2286_RUN_TOKEN_result]
        lappend R2286_RUN_TOKEN_rows [list syntax $R2286_RUN_TOKEN_syntax install $R2286_RUN_TOKEN_install install_hex [R2286_RUN_TOKEN_hex $R2286_RUN_TOKEN_install_result] operation $R2286_RUN_TOKEN_code result_hex [R2286_RUN_TOKEN_hex $R2286_RUN_TOKEN_result] callbacks $R2286_RUN_TOKEN_callbacks]
        if {$R2286_RUN_TOKEN_syntax eq "modern"} {
            catch {trace remove variable R2286_RUN_TOKEN_value {read unset} R2286_RUN_TOKEN_watch}
        } else {
            catch {trace vdelete R2286_RUN_TOKEN_value ru R2286_RUN_TOKEN_watch}
        }
        catch {unset R2286_RUN_TOKEN_value}
    }
    log local0.notice [list RESOLUTION2286_VARIABLE_LIFECYCLE RUN_TOKEN tmm [TMM::cmp_unit] rows $R2286_RUN_TOKEN_rows]
    HTTP::respond 200 content [list RUN_TOKEN $R2286_RUN_TOKEN_rows]
}
