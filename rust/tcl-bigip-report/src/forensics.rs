//! UCS forensic inventory: shape the file inventory extracted from a UCS into
//! the report's Forensics view, and run a few content-driven heuristics that
//! turn the static "UCS forensic checklist" (see the F5 Sites tab) into live
//! findings against the archive under review.
//!
//! Input is the per-device file inventory the entry points (wasm / CLI) pull
//! out of the archive with [`tcl_bigip_io::list_ucs_members`] +
//! [`tcl_bigip_io::read_ucs_member`] — each a JSON object
//! `{path, size, sha256, isText, content?}`. `content` is present for the small
//! forensic-allowlisted text files (dotfiles, `passwd`, `authorized_keys`,
//! syslog config); it stays local to the page (nothing is uploaded) and the UI
//! masks the sensitive ones by default, exactly like the Secrets tab.
//!
//! Output is `{files: [...enriched...], checklist: [...findings...]}`.

use serde_json::{Map, Value as J, json};

/// Stock TMOS / Linux accounts present on a clean BIG-IP. A `passwd` entry
/// outside this set with a real login shell — or *any* non-`root` UID 0 — is
/// worth a human look (T1136 Create Account / T1078 Valid Accounts).
const STOCK_USERS: &[&str] = &[
    "root",
    "bin",
    "daemon",
    "adm",
    "lp",
    "sync",
    "shutdown",
    "halt",
    "mail",
    "operator",
    "games",
    "ftp",
    "nobody",
    "admin",
    "apache",
    "sshd",
    "tmshnobody",
    "f5_remoteuser",
    "mysql",
    "named",
    "ntp",
    "dbus",
    "rpc",
    "rpcuser",
    "nfsnobody",
    "nscd",
    "vcsa",
    "pcap",
    "oprofile",
    "syslog",
    "postfix",
    "tomcat",
    "restnoded",
    "tmsh",
];

/// Login shells that mean the account can actually be used interactively (so an
/// unexpected user with one of these is more interesting than a service stub).
const LOGIN_SHELLS: &[&str] = &["/bin/bash", "/bin/sh", "/bin/tmsh", "/usr/bin/tmsh"];

fn str_field<'a>(o: &'a Map<String, J>, k: &str) -> &'a str {
    o.get(k).and_then(J::as_str).unwrap_or("")
}

/// Broad category for grouping / colouring a member in the UI.
fn categorise(path: &str) -> &'static str {
    if path.contains("/.ssh/") || path.starts_with(".ssh/") {
        "ssh"
    } else if path == "etc/passwd"
        || path == "etc/shadow"
        || path == "etc/group"
        || path == "etc/gshadow"
    {
        "accounts"
    } else if path.starts_with("etc/openldap/")
        || path.starts_with("etc/pam.d/")
        || path.starts_with("etc/security/")
        || path == "etc/nsswitch.conf"
        || path == "etc/krb5.conf"
        || path == "etc/ldap.conf"
    {
        "auth"
    } else if path.starts_with("etc/syslog-ng/") {
        "logging"
    } else if path.starts_with("etc/cron") {
        "cron"
    } else if (path.starts_with("home/") || path.starts_with("root/"))
        && path.rsplit('/').next().is_some_and(|b| b.starts_with('.'))
    {
        "dotfile"
    } else {
        "other"
    }
}

/// Whether a member's content is sensitive enough to mask in the UI by default
/// (password hashes, private keys). Metadata (path/size/hash) is always shown.
fn is_sensitive(path: &str) -> bool {
    path == "etc/shadow"
        || path == "etc/gshadow"
        || path.ends_with("/id_rsa")
        || path.ends_with("/id_dsa")
        || path.ends_with("/id_ecdsa")
        || path.ends_with("/id_ed25519")
}

/// Count the real key lines in an `authorized_keys` body (non-blank,
/// non-comment).
fn count_authorized_keys(content: &str) -> usize {
    content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .count()
}

/// Parse `etc/passwd` content into `(user, uid, shell)` rows.
fn parse_passwd(content: &str) -> Vec<(String, String, String)> {
    content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let f: Vec<&str> = line.split(':').collect();
            // name:passwd:uid:gid:gecos:home:shell
            if f.len() < 7 {
                return None;
            }
            Some((f[0].to_string(), f[2].to_string(), f[6].to_string()))
        })
        .collect()
}

/// Build the Forensics model for one device from its file inventory.
///
/// `files` is the per-device inventory (JSON objects with `path`, `size`,
/// `sha256`, `isText` and optionally `content`). Returns
/// `{files: [...], checklist: [...]}`; both empty when the inventory is empty
/// (e.g. a `bigip.conf`-only source with no archive behind it).
#[must_use]
pub fn collect_forensics(files: &[J]) -> J {
    let mut out_files: Vec<J> = Vec::with_capacity(files.len());
    // Checklist accumulators.
    let mut ak_paths: Vec<String> = Vec::new();
    let mut ak_keys = 0usize;
    let mut passwd_added: Vec<String> = Vec::new();
    let mut passwd_uid0: Vec<String> = Vec::new();
    let mut passwd_seen = false;
    let mut dotfiles: Vec<String> = Vec::new();
    let mut logging_seen = false;

    for f in files {
        let Some(o) = f.as_object() else { continue };
        let path = str_field(o, "path").to_string();
        if path.is_empty() {
            continue;
        }
        let category = categorise(&path);
        let sensitive = is_sensitive(&path);
        let content = o.get("content").and_then(J::as_str);

        // Per-file heuristics.
        if path.ends_with("authorized_keys")
            && let Some(c) = content
        {
            let n = count_authorized_keys(c);
            if n > 0 {
                ak_keys += n;
                ak_paths.push(path.clone());
            }
        }
        if path == "etc/passwd" {
            passwd_seen = true;
            if let Some(c) = content {
                for (user, uid, shell) in parse_passwd(c) {
                    if uid == "0" && user != "root" {
                        passwd_uid0.push(user.clone());
                    }
                    let stock = STOCK_USERS.contains(&user.as_str());
                    let interactive = LOGIN_SHELLS.contains(&shell.as_str());
                    if !stock && interactive {
                        passwd_added.push(user);
                    }
                }
            }
        }
        if category == "dotfile" {
            dotfiles.push(path.clone());
        }
        if category == "logging" {
            logging_seen = true;
        }

        out_files.push(json!({
            "path": path,
            "size": o.get("size").cloned().unwrap_or(J::from(0)),
            "sha256": str_field(o, "sha256"),
            "isText": o.get("isText").and_then(J::as_bool).unwrap_or(false),
            "category": category,
            "sensitive": sensitive,
            // Content is echoed through only for non-sensitive text files; the
            // UI masks even these behind a reveal, and never embeds the raw
            // bytes of shadow / private keys.
            "content": if sensitive { J::Null } else { content.map_or(J::Null, |c| J::String(c.to_owned())) },
        }));
    }

    // ---- Checklist findings (most severe first is applied in the UI) -------
    let mut checklist: Vec<J> = Vec::new();

    // SSH authorized_keys — key-based persistence.
    let ak_present = out_files.iter().any(|f| {
        f.get("path")
            .and_then(J::as_str)
            .is_some_and(|p| p.ends_with("authorized_keys"))
    });
    checklist.push(json!({
        "id": "ssh-authorized-keys",
        "label": "SSH authorized_keys",
        "attack": "T1098.004",
        "verdict": if ak_keys > 0 { "alert" } else if ak_present { "clear" } else { "absent" },
        "detail": if ak_keys > 0 {
            format!("{ak_keys} key(s) present across {} file(s) — verify every key is expected", ak_paths.len())
        } else if ak_present {
            "authorized_keys present but empty".to_string()
        } else {
            "no authorized_keys in the archive".to_string()
        },
        "evidence": ak_paths,
    }));

    // Local accounts — added users / rogue UID 0.
    let mut acct_evidence = passwd_uid0.clone();
    acct_evidence.extend(passwd_added.iter().cloned());
    let acct_verdict = if !passwd_uid0.is_empty() {
        "alert"
    } else if !passwd_added.is_empty() {
        "warn"
    } else if passwd_seen {
        "clear"
    } else {
        "absent"
    };
    checklist.push(json!({
        "id": "local-accounts",
        "label": "Local accounts (passwd)",
        "attack": "T1136 / T1078",
        "verdict": acct_verdict,
        "detail": if !passwd_uid0.is_empty() {
            format!("non-root UID 0 account(s): {}", passwd_uid0.join(", "))
        } else if !passwd_added.is_empty() {
            format!("unrecognised interactive account(s): {}", passwd_added.join(", "))
        } else if passwd_seen {
            "only stock accounts with login shells".to_string()
        } else {
            "etc/passwd not in the archive".to_string()
        },
        "evidence": acct_evidence,
    }));

    // Shell dotfiles — login-hook persistence surface.
    checklist.push(json!({
        "id": "shell-dotfiles",
        "label": "Shell dotfiles",
        "attack": "T1546.004",
        "verdict": if dotfiles.is_empty() { "absent" } else { "info" },
        "detail": if dotfiles.is_empty() {
            "no user dotfiles in the archive".to_string()
        } else {
            format!("{} dotfile(s) present — review for login-time execution", dotfiles.len())
        },
        "evidence": dotfiles,
    }));

    // Logging config — evasion surface.
    checklist.push(json!({
        "id": "logging-config",
        "label": "syslog-ng config",
        "attack": "T1562.006",
        "verdict": if logging_seen { "info" } else { "absent" },
        "detail": if logging_seen {
            "syslog-ng config present — confirm remote logging is intact".to_string()
        } else {
            "no syslog-ng config in the archive".to_string()
        },
        "evidence": J::Array(vec![]),
    }));

    json!({ "files": out_files, "checklist": checklist })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str, content: Option<&str>) -> J {
        let bytes = content.map_or(&b""[..], str::as_bytes);
        json!({
            "path": path,
            "size": bytes.len(),
            "sha256": "0".repeat(64),
            "isText": content.is_some(),
            "content": content,
        })
    }

    fn verdict<'a>(f: &'a J, id: &str) -> &'a str {
        f["checklist"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == id)
            .unwrap_or_else(|| panic!("checklist item {id}"))["verdict"]
            .as_str()
            .unwrap()
    }

    #[test]
    fn authorized_keys_flagged_when_non_empty() {
        let flagged = collect_forensics(&[file(
            "root/.ssh/authorized_keys",
            Some("# comment\nssh-rsa AAAAB3Nz attacker@host\n"),
        )]);
        assert_eq!(verdict(&flagged, "ssh-authorized-keys"), "alert");

        let empty = collect_forensics(&[file("root/.ssh/authorized_keys", Some("# only a comment\n"))]);
        assert_eq!(verdict(&empty, "ssh-authorized-keys"), "clear");

        let absent = collect_forensics(&[file("etc/motd", Some("hi"))]);
        assert_eq!(verdict(&absent, "ssh-authorized-keys"), "absent");
    }

    #[test]
    fn passwd_added_and_uid0_accounts() {
        // A rogue non-root UID 0 account → alert.
        let uid0 = collect_forensics(&[file(
            "etc/passwd",
            Some("root:x:0:0::/root:/bin/bash\nbackdoor:x:0:0::/root:/bin/bash\n"),
        )]);
        assert_eq!(verdict(&uid0, "local-accounts"), "alert");

        // An unrecognised interactive account (non-zero uid) → warn.
        let added = collect_forensics(&[file(
            "etc/passwd",
            Some("root:x:0:0::/root:/bin/bash\neviluser:x:1200:1200::/home/eviluser:/bin/bash\n"),
        )]);
        assert_eq!(verdict(&added, "local-accounts"), "warn");

        // Only stock accounts → clear.
        let clean = collect_forensics(&[file(
            "etc/passwd",
            Some("root:x:0:0::/root:/bin/bash\nnobody:x:99:99::/:/sbin/nologin\n"),
        )]);
        assert_eq!(verdict(&clean, "local-accounts"), "clear");
    }

    #[test]
    fn sensitive_content_is_not_embedded() {
        let f = collect_forensics(&[file("etc/shadow", Some("root:$6$abc$def:19000:0:99999:7:::\n"))]);
        let shadow = &f["files"][0];
        assert_eq!(shadow["category"], "accounts");
        assert_eq!(shadow["sensitive"], J::Bool(true));
        assert_eq!(shadow["content"], J::Null, "shadow content must not be embedded");
        // But its metadata (hash/size) is still there for diffing.
        assert_eq!(shadow["sha256"].as_str().unwrap().len(), 64);
    }

    #[test]
    fn dotfiles_and_logging_surface_as_info() {
        let f = collect_forensics(&[
            file("home/admin/.bashrc", Some("export PATH=$PATH\n")),
            file("etc/syslog-ng/syslog-ng.conf", Some("destination d_remote {};\n")),
        ]);
        assert_eq!(verdict(&f, "shell-dotfiles"), "info");
        assert_eq!(verdict(&f, "logging-config"), "info");
        // Non-sensitive text content is echoed for display.
        let bashrc = f["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["path"] == "home/admin/.bashrc")
            .unwrap();
        assert!(bashrc["content"].as_str().unwrap().contains("PATH"));
    }
}
