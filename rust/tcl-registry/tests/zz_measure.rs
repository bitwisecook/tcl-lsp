fn rss_kb() -> u64 {
    let s = std::fs::read_to_string("/proc/self/statm").unwrap();
    let pages: u64 = s.split_whitespace().nth(1).unwrap().parse().unwrap();
    pages * 4
}

#[test]
fn overlay_generation_cost() {
    let profile = tcl_dialect::DialectProfile::by_name("tcl9.0");
    // warm the base
    let _ = tcl_registry::cache::registry_for_profile(profile);
    let before = rss_kb();
    const N: u64 = 20;
    for i in 1..=N {
        let _ = tcl_registry::cache::registry_for_profile_with_overlay(profile, i, |_| {});
    }
    let after = rss_kb();
    eprintln!(
        "RSS {before} -> {after} kB for {N} overlay generations = {} kB each",
        (after - before) / N
    );
}
