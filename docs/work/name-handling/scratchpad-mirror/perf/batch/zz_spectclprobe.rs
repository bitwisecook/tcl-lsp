//! Temporary perf probe (perf agent). Delete when done.
use std::sync::atomic::Ordering;
fn load(old: bool) -> (u128, Vec<String>) {
    tcl_lexer::ZZ_OLD.store(old, Ordering::Relaxed);
    let t = std::time::Instant::now();
    let set = tcl_spectcl::bundled::load_embedded();
    let ms = t.elapsed().as_millis();
    tcl_lexer::ZZ_OLD.store(false, Ordering::Relaxed);
    let mut lines: Vec<String> = format!("{set:#?}").lines().map(str::to_owned).collect();
    lines.sort();
    (ms, lines)
}
fn main() {
    let compare = std::env::var_os("ZZ_COMPARE").is_some();
    let (new_ms, new_lines) = load(false);
    println!("new load_embedded ms={new_ms} lines={}", new_lines.len());
    if compare {
        let (old_ms, old_lines) = load(true);
        println!("old load_embedded ms={old_ms} lines={} equal_sorted_lines={}", old_lines.len(), old_lines == new_lines);
    }
}
