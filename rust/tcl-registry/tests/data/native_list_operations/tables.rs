pub const LIST_TABLES: &[(&str,&str,&str)] = &[
    ("tcl8.4", include_str!("8.4.20.txt"), include_str!("../native_list_assignment_order/8.4.20.txt")),
    ("tcl8.5", include_str!("8.5.19.txt"), include_str!("../native_list_assignment_order/8.5.19.txt")),
    ("tcl8.6", include_str!("8.6.18.txt"), include_str!("../native_list_assignment_order/8.6.18.txt")),
    ("tcl9.0", include_str!("9.0.4.txt"), include_str!("../native_list_assignment_order/9.0.4.txt")),
    ("tcl9.1", include_str!("9.1.0.txt"), include_str!("../native_list_assignment_order/9.1.0.txt")),
    ("jim", include_str!("jim.txt"), include_str!("../native_list_assignment_order/jim.txt")),
];
