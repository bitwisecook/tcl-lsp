//! Temporary perf probe (perf agent). Delete when done.
use std::time::Instant;
use tcl_compiler::analyser::Analyser;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let kind = args.get(1).map(String::as_str).unwrap_or("calls");
    let ns: Vec<usize> = args
        .get(2)
        .map(|s| s.split(',').map(|x| x.parse().unwrap()).collect())
        .unwrap_or_else(|| vec![10, 20, 40]);
    let dump = std::env::var_os("ZZ_DUMP");
    for n in ns {
        let src = match kind {
            "calls" => {
                let mut s = String::from("proc p {a} {return $a}\n");
                for _ in 0..n {
                    s.push_str("p 1\n");
                }
                s
            }
            "procs" => {
                let mut s = String::new();
                for i in 0..n {
                    s.push_str(&format!("proc capped_{i} {{}} {{}}\n"));
                }
                s
            }
            "sets" => {
                let mut s = String::new();
                for i in 0..n {
                    s.push_str(&format!("set v{i} {i}\n"));
                }
                s
            }
            "file" => std::fs::read_to_string(&args[3]).unwrap(),
            _ => panic!("kind"),
        };
        let t = Instant::now();
        let r = Analyser::new().analyse(&src, "tcl8.6");
        let el = t.elapsed();
        println!("{kind} n={n} ms={}", el.as_millis());
        if let Some(d) = &dump {
            std::fs::write(
                format!("{}/{kind}-{n}.txt", d.to_string_lossy()),
                format!("{r:#?}"),
            )
            .unwrap();
        }
    }
}
