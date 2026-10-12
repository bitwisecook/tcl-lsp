namespace eval AliasHolder {
    namespace eval q {}
    proc inspect {} {
        namespace delete ::AliasHolder
        set relative_code [catch {interp alias {} q::relative {} list RELATIVE} relative_result]
        set rooted_code [catch {interp alias {} ::AliasHolder::q::rooted {} list ROOTED} rooted_result]
        set local_code [catch {q::relative} local_result]
        set root_code [catch {::AliasHolder::q::rooted} root_result]
        list $relative_code $relative_result $rooted_code $rooted_result $local_code $local_result $root_code $root_result [namespace exists ::AliasHolder]
    }
}
set first [AliasHolder::inspect]
list $first [namespace exists ::AliasHolder] [info commands ::AliasHolder::q::*]
