#include "common.apl"
define choice yesno_choice {
}
section basic {
    string addr required
    string port
    choice mode display "large"
    password secret required
}
section ssl {
    yesno enable
    table members {
        string member_addr
        string member_port required
    }
}
table standalone {
    string col1
    editchoice col2
}
