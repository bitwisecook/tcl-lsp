namespace eval ::a {
    proc helper {} { return 1 }
    proc run {} { helper }
}
namespace eval ::b {
    proc helper {} { return 2 }
}
