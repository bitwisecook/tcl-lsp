    #[test]
    fn literal_flows_to_regexp_in_a_large_multi_proc_file() {
        use std::fmt::Write as _;
        let mut s = String::new();
        s.push_str("namespace eval ::bench {\n    variable counter 0\n}\n\n");
        for i in 0..600 {
            let _ = write!(
                s,
                "# proc number {i}\n\
                 proc ::bench::step{i} {{a b}} {{\n\
                 \x20   set v{i} [expr {{$a + $b}}]\n\
                 \x20   set msg \"step {i} = $v{i}\"\n\
                 \x20   if {{$v{i} > 10}} {{\n\
                 \x20       set v{i} [expr {{$v{i} + 1}}]\n\
                 \x20   }}\n\
                 \x20   return $v{i}\n\
                 }}\n\n"
            );
        }
        s.push_str(
            "proc ::bench::regex_check {} {\n    set my_re \".*abc\"\n    regexp $my_re $s\n}\n",
        );
        let got = spans_text(&s);
        assert_eq!(got, vec!["\".*abc\"".to_owned()]);
    }
