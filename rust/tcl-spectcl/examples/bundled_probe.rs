//! TEMPORARY verification probe (NOT for commit).
//!
//! Prints what the *running executable* discovers as its bundled pack
//! directory and whether a given EDA command resolves for a dialect. Built in
//! release (`debug_assertions` off) it exercises the real shipped path — the
//! `specs/` directory beside the executable — with the source-checkout
//! fallback compiled out entirely.
//!
//! Usage: `bundled_probe <dialect> <command>...`

fn main() {
    let mut args = std::env::args().skip(1);
    let dialect = args.next().unwrap_or_else(|| "xilinx-eda-tcl".to_owned());
    let commands: Vec<String> = args.collect();

    println!("debug_assertions       : {}", cfg!(debug_assertions));
    println!(
        "current_exe            : {}",
        std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "<unknown>".to_owned())
    );
    println!(
        "bundled_dir()          : {}",
        tcl_spectcl::discovery::bundled_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "<none>".to_owned())
    );

    let packs = tcl_spectcl::bundled::packs();
    let mut names: Vec<&str> = packs.packs.iter().map(|p| p.name.as_str()).collect();
    names.sort_unstable();
    println!("bundled packs          : {names:?}");
    println!("bundled pack notices   : {}", packs.notices.len());

    let registry = tcl_spectcl::bundled::registry_for_dialect(&dialect);
    for command in &commands {
        println!(
            "{dialect} :: {command:<20} => {}",
            if registry.get(command).is_some() {
                "KNOWN"
            } else {
                "unknown"
            }
        );
    }
}
