//! `f5 pull` — fetch one object from a live device as an SCF stanza.
//!
//! Credential resolution + arg shaping is parity-tested offline; the live GET
//! is implemented but untested here. With `--json` the raw iControl JSON is
//! emitted (via the `json.dumps(indent=2)`-compatible serializer); otherwise
//! the object is rendered to an SCF stanza and passed through `render_config`.

use super::emit::render_config;
use super::remote::auth::{ResolveOptions, resolve_credentials};
use super::remote::json_compat::dumps_indent2;
use super::remote::object_io::{object_to_scf_stanza, pull_object};

/// Parameters for [`run_pull`].
#[allow(clippy::struct_excessive_bools)]
pub struct PullArgs<'a> {
    pub kind: &'a str,
    pub full_path: &'a str,
    pub host: Option<&'a str>,
    pub user: Option<&'a str>,
    pub password: Option<&'a str>,
    pub port: Option<u16>,
    pub no_prompt: bool,
    pub insecure: bool,
    pub json: bool,
    pub timeout: f64,
    pub format: &'a str,
    pub transaction: bool,
}

/// Run `f5 pull`, returning the process exit code (0 / 2).
#[must_use]
pub fn run_pull(args: &PullArgs) -> u8 {
    let creds = match resolve_credentials(&ResolveOptions {
        host: args.host,
        user: args.user,
        password: args.password,
        port: args.port,
        ssh_port: None,
        interactive: !args.no_prompt,
    }) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };

    let obj = match pull_object(
        &creds,
        args.kind,
        args.full_path,
        args.insecure,
        args.timeout,
    ) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("error: pull failed: {e}");
            return 2;
        }
    };

    if args.json {
        println!("{}", dumps_indent2(&obj));
    } else {
        let scf = object_to_scf_stanza(args.kind, &obj);
        print!(
            "{}",
            render_config(&scf, args.format, "create", args.transaction, "")
        );
    }
    0
}
