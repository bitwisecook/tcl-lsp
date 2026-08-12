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

//! Offline security-posture findings (the report's Security tab).
//!
//! A small, documented, high-confidence rule table — data-driven the same way
//! [`crate::forensics`]'s ATT&CK checklist is: each rule inspects whatever
//! this source actually carries and reports a stable-id [`Finding`], never
//! hardcoded per-object logic in a consumer. Detection is entirely passive:
//! nothing here authenticates to a device or makes a network request, and
//! [`crate::crypt`] (the only cryptography involved) never calls the
//! platform `crypt(3)`.
//!
//! # What is inspected
//!
//! * The parsed `bigip.conf`/SCF text (via [`tcl_bigip::parser::parse_bigip_conf`]
//!   and the same low-level block/property helpers `certs.rs`/`secrets.rs`
//!   use) — `auth user` (credential + shell + role), `auth password-policy`,
//!   and `sys snmp` communities.
//! * [`tcl_bigip::secrets::collect_secrets`] — every credential-bearing field
//!   BIG-IP normally stores under the unit master key, flagging any that
//!   are *not* (a plaintext `$M$`-less value).
//! * The UCS file inventory (`files`, when the source is a UCS rather than a
//!   bare config — see [`crate::forensics`] for the same input), scanned for
//!   `/etc/shadow` credential material and for private-key files that carry
//!   no passphrase-protection marker.
//!
//! # What is *not* inspected (documented limitation)
//!
//! F5's per-service management hardening — `sys sshd` (remote root / shell
//! restrictions) and `sys httpd` (management GUI TLS/HTTP settings) — is not
//! yet covered: the generated BIG-IP object model
//! ([`tcl_bigip::model::gen`]) currently keeps both as untyped
//! [`tcl_bigip::model::BigipMinimalObject`]s (identity + description only,
//! no properties preserved), so a rule has no fields to read. Extending
//! those `@generated … do not edit` files by hand was judged higher-risk
//! than shipping without the checks; a follow-up should give the two kinds
//! typed fields, then extend [`RULES`].
//!
//! A source that is only a partial `bigip.conf` (no `auth password-policy`
//! block, no UCS filestore, no `/etc/shadow`) naturally yields
//! [`Status::CouldNotInspect`] / [`Status::NotApplicable`] findings for the
//! rules that need that material — this is the intended behaviour, not a
//! bug: the report tells you what it *couldn't* check, rather than staying
//! silent about it.

use serde_json::{Map, Value as J, json};
use tcl_bigip::model::ModelObject;
use tcl_bigip::parser::driver::parse_bigip_conf;
use tcl_bigip::parser::helpers::{extract_blocks, parse_keyed_block_entries, parse_properties};
use tcl_bigip::secrets::collect_secrets as collect_raw_secrets;

use crate::crypt::{self, Verdict};

/// Severity, most to least urgent — drives the summary counts and the
/// report's ordering/colouring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl Severity {
    fn as_str(self) -> &'static str {
        match self {
            Severity::Critical => "critical",
            Severity::High => "high",
            Severity::Medium => "medium",
            Severity::Low => "low",
            Severity::Info => "info",
        }
    }
}

/// A finding's disposition — the three-way (plus "confirmed") split the
/// issue calls for: *confirmed* (acted on), *clear* (checked, not an issue),
/// *not applicable* (nothing here to check), *could not inspect* (present,
/// but not in a form this generator can verify).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Confirmed,
    Clear,
    NotApplicable,
    CouldNotInspect,
}

impl Status {
    fn as_str(self) -> &'static str {
        match self {
            Status::Confirmed => "confirmed",
            Status::Clear => "clear",
            Status::NotApplicable => "not_applicable",
            Status::CouldNotInspect => "could_not_inspect",
        }
    }
}

/// One security finding. `evidence` and `source` are always human-readable
/// pointers (an object path, a count, a field name) — never a password,
/// hash, salt, master key, private key, or other decrypted secret value.
struct Finding {
    /// Stable rule id, e.g. `"BIGIP-SEC-001"` — see [`RULES`].
    rule_id: &'static str,
    severity: Severity,
    status: Status,
    title: &'static str,
    evidence: String,
    remediation: &'static str,
    source: String,
    /// An authoritative F5 reference, when one is confidently known.
    reference: Option<&'static str>,
}

impl Finding {
    fn to_json(&self) -> J {
        json!({
            "id": self.rule_id,
            "severity": self.severity.as_str(),
            "status": self.status.as_str(),
            "title": self.title,
            "evidence": self.evidence,
            "remediation": self.remediation,
            "source": self.source,
            "reference": self.reference,
        })
    }
}

/// Everything a rule might need, gathered once per device.
struct Ctx<'a> {
    source: &'a str,
    auth_users: Vec<AuthUserFacts>,
    password_policy: Option<tcl_bigip::model::BigipAuthPasswordPolicy>,
    files: &'a [J],
}

/// The subset of `auth user` facts a rule needs, plus the `role` value this
/// generator recovers itself (the generated model's `partition_access` field
/// carries only partition names — see the module doc's "not yet inspected"
/// note on why this one small extra scan lives here instead of in the typed
/// model).
struct AuthUserFacts {
    name: String,
    full_path: String,
    encrypted_password: String,
    shell: String,
    has_admin_role: bool,
}

impl Ctx<'_> {
    fn build<'a>(source: &'a str, files: &'a [J]) -> Ctx<'a> {
        let cfg = parse_bigip_conf(source, "Common");
        let blocks = extract_blocks(source);

        let mut auth_users = Vec::new();
        let mut password_policy = None;
        for placed in &cfg.objects {
            match &placed.object {
                ModelObject::AuthUser(u) => {
                    let has_admin_role = blocks
                        .iter()
                        .find(|b| {
                            b.header == format!("auth user {}", placed.full_path)
                                || b.header == format!("auth user \"{}\"", placed.full_path)
                        })
                        .is_some_and(|b| {
                            b.body.contains("role admin") || b.body.contains("role administrator")
                        });
                    auth_users.push(AuthUserFacts {
                        name: u.name.clone(),
                        full_path: if placed.full_path.is_empty() {
                            u.name.clone()
                        } else {
                            placed.full_path.clone()
                        },
                        encrypted_password: u.encrypted_password.clone(),
                        shell: u.shell.clone(),
                        has_admin_role,
                    });
                }
                ModelObject::AuthPasswordPolicy(p) => {
                    password_policy = Some(p.clone());
                }
                _ => {}
            }
        }

        Ctx {
            source,
            auth_users,
            password_policy,
            files,
        }
    }
}

/// `(username, factory default password)` pairs to check. A rule table in
/// its own right — extend here, not with a new branch in
/// [`check_default_credentials`].
const DEFAULT_CREDENTIALS: &[(&str, &str)] = &[("root", "default"), ("admin", "admin")];

/// `/etc/shadow`-style `name:hash:…` line → `(name, hash)`.
fn parse_shadow_line(line: &str) -> Option<(&str, &str)> {
    let mut fields = line.splitn(3, ':');
    let name = fields.next()?;
    let hash = fields.next()?;
    if name.is_empty() {
        return None;
    }
    Some((name, hash))
}

/// Every `(source_label, hash)` this UCS/config carries for `username`: the
/// `auth user` config entry (if the name matches) plus any `/etc/shadow`
/// entry in the file inventory.
fn credential_sources(ctx: &Ctx, username: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for u in &ctx.auth_users {
        if u.name == username && !u.encrypted_password.is_empty() {
            out.push((
                format!("auth user {}", u.full_path),
                u.encrypted_password.clone(),
            ));
        }
    }
    for f in ctx.files {
        let Some(o) = f.as_object() else { continue };
        if o.get("path").and_then(J::as_str) != Some("etc/shadow") {
            continue;
        }
        let Some(content) = o.get("content").and_then(J::as_str) else {
            continue;
        };
        for line in content.lines() {
            if let Some((name, hash)) = parse_shadow_line(line)
                && name == username
            {
                out.push(("etc/shadow".to_owned(), hash.to_owned()));
            }
        }
    }
    out
}

/// **BIGIP-SEC-001** — confirmed factory/default account credentials
/// (`root`/`default`, `admin`/`admin`). See [`crypt`] for how a stored hash
/// is verified against the known candidate without ever touching platform
/// `crypt(3)` or logging the stored value.
fn check_default_credentials(id: &'static str, ctx: &Ctx) -> Vec<Finding> {
    let mut out = Vec::new();
    for (username, candidate) in DEFAULT_CREDENTIALS {
        let sources = credential_sources(ctx, username);
        if sources.is_empty() {
            out.push(Finding {
                rule_id: id,
                severity: Severity::Info,
                status: Status::NotApplicable,
                title: "Default credential check has no material to inspect",
                evidence: format!(
                    "no `{username}` account (an `auth user` stanza or `/etc/shadow` entry) is present in this source"
                ),
                remediation: "Supply a full config export or UCS if this device does carry a local `root`/`admin` account.",
                source: username.to_string(),
                reference: None,
            });
            continue;
        }
        for (source, hash) in sources {
            let (severity, status, title, evidence, remediation): (
                Severity,
                Status,
                &'static str,
                String,
                &'static str,
            ) = match crypt::verify(&hash, candidate) {
                Verdict::Match => (
                    Severity::Critical,
                    Status::Confirmed,
                    "Default account credential detected",
                    format!(
                        "the stored `{username}` password verifies against the factory default"
                    ),
                    "Change the credential immediately and review the account's access and login history.",
                ),
                Verdict::NoMatch => (
                    Severity::Info,
                    Status::Clear,
                    "Default credential check passed",
                    format!(
                        "the stored `{username}` password does not verify against the factory default"
                    ),
                    "No action required.",
                ),
                Verdict::NotApplicable => (
                    Severity::Info,
                    Status::NotApplicable,
                    "Account has no usable password",
                    format!(
                        "`{username}` is locked or has no password set (cannot be checked against a default)"
                    ),
                    "No action required.",
                ),
                Verdict::Unsupported => (
                    Severity::Medium,
                    Status::CouldNotInspect,
                    "Stored credential is in an unsupported format",
                    format!(
                        "`{username}`'s stored password is not in a supported hash scheme (SHA-512/256-crypt or MD5-crypt) — it was not compared"
                    ),
                    "Confirm the credential manually, or re-export with a supported hashing scheme so it can be checked automatically.",
                ),
            };
            out.push(Finding {
                rule_id: id,
                severity,
                status,
                title,
                evidence,
                remediation,
                source,
                reference: None,
            });
        }
    }
    out
}

/// Community names widely shipped as SNMP defaults across vendors — a device
/// still using one of these is trivially guessable.
const DEFAULT_SNMP_COMMUNITIES: &[&str] = &["public", "private"];

/// **BIGIP-SEC-002** — default or weak SNMP community strings.
///
/// The generated `sys snmp` model only carries each community *stanza's*
/// name (e.g. `comm-public`), not its `community-name` value — so this rule
/// re-derives the real community string from the config text directly with
/// the same generic block/property helpers `certs.rs`/`secrets.rs` use,
/// rather than guessing from the stanza label.
fn check_snmp_communities(id: &'static str, ctx: &Ctx) -> Vec<Finding> {
    let Some(block) = extract_blocks(ctx.source)
        .into_iter()
        .find(|b| b.header == "sys snmp")
    else {
        return vec![Finding {
            rule_id: id,
            severity: Severity::Info,
            status: Status::Clear,
            title: "SNMP is not configured",
            evidence: "no `sys snmp` stanza in this source".to_owned(),
            remediation: "No action required.",
            source: "sys snmp".to_owned(),
            reference: None,
        }];
    };

    let communities_raw = parse_properties(&block.body)
        .into_iter()
        .find(|(k, _)| k == "communities")
        .map(|(_, v)| v)
        .unwrap_or_default();
    let entries = parse_keyed_block_entries(&communities_raw);
    if entries.is_empty() {
        return vec![Finding {
            rule_id: id,
            severity: Severity::Info,
            status: Status::Clear,
            title: "No SNMP community strings configured",
            evidence: "`sys snmp` is present but defines no communities".to_owned(),
            remediation: "No action required.",
            source: "sys snmp".to_owned(),
            reference: None,
        }];
    }

    let mut out = Vec::new();
    let mut default_count = 0usize;
    for (name, body) in entries {
        let props = parse_properties(&body);
        let community_name = props
            .iter()
            .find(|(k, _)| k == "community-name")
            .map(|(_, v)| tcl_bigip::parser::helpers::unquote(v).to_owned())
            .unwrap_or_default();
        let access = props
            .iter()
            .find(|(k, _)| k == "access")
            .map_or("ro", |(_, v)| v.as_str());
        let is_default = DEFAULT_SNMP_COMMUNITIES
            .iter()
            .any(|d| d.eq_ignore_ascii_case(&community_name));
        if !is_default {
            continue;
        }
        default_count += 1;
        let rw = access.eq_ignore_ascii_case("rw");
        out.push(Finding {
            rule_id: id,
            severity: if rw { Severity::High } else { Severity::Medium },
            status: Status::Confirmed,
            title: if rw {
                "Default SNMP community with read-write access"
            } else {
                "Default SNMP community string"
            },
            evidence: format!("community `{name}` uses the default string \"{community_name}\" with {access} access"),
            remediation: "Replace the community string with a unique, hard-to-guess value, and prefer SNMPv3 with authentication/privacy over v1/v2c communities.",
            source: format!("sys snmp communities {name}"),
            reference: None,
        });
    }
    if default_count == 0 {
        out.push(Finding {
            rule_id: id,
            severity: Severity::Info,
            status: Status::Clear,
            title: "No default SNMP community strings found",
            evidence: "community strings are configured; none use a well-known default name"
                .to_owned(),
            remediation: "No action required.",
            source: "sys snmp".to_owned(),
            reference: None,
        });
    }
    out
}

/// A conservative baseline minimum password length; below this a policy is
/// flagged even when enforcement is nominally on. Not an authoritative F5
/// recommendation — a device's own compliance baseline may set this higher.
const WEAK_MIN_LENGTH: u32 = 8;

/// **BIGIP-SEC-003** — disabled or weak password-policy enforcement.
fn check_password_policy(id: &'static str, ctx: &Ctx) -> Vec<Finding> {
    let Some(p) = &ctx.password_policy else {
        return vec![Finding {
            rule_id: id,
            severity: Severity::Medium,
            status: Status::CouldNotInspect,
            title: "Password policy could not be inspected",
            evidence: "no `auth password-policy` stanza is present in this source".to_owned(),
            remediation: "Supply a full config export or UCS to check password-complexity enforcement.",
            source: "auth password-policy".to_owned(),
            reference: None,
        }];
    };

    if p.policy_enforcement.eq_ignore_ascii_case("disabled") {
        return vec![Finding {
            rule_id: id,
            severity: Severity::High,
            status: Status::Confirmed,
            title: "Password complexity enforcement is disabled",
            evidence: "`auth password-policy` has `policy-enforcement disabled` — any password length or complexity is accepted".to_owned(),
            remediation: "Enable password-policy enforcement and set a minimum length/complexity that meets your organisation's baseline.",
            source: "auth password-policy".to_owned(),
            reference: None,
        }];
    }

    let min_len: Option<u32> = p.minimum_length.trim().parse().ok();
    match min_len {
        Some(n) if n < WEAK_MIN_LENGTH => vec![Finding {
            rule_id: id,
            severity: Severity::Medium,
            status: Status::Confirmed,
            title: "Password minimum length is weak",
            evidence: format!("`auth password-policy` requires only {n} character(s)"),
            remediation: "Raise `minimum-length` to meet your organisation's baseline (commonly 14+ for administrative accounts).",
            source: "auth password-policy".to_owned(),
            reference: None,
        }],
        Some(n) => vec![Finding {
            rule_id: id,
            severity: Severity::Info,
            status: Status::Clear,
            title: "Password policy enforcement is enabled",
            evidence: format!("`auth password-policy` enforces a minimum length of {n} character(s)"),
            remediation: "No action required.",
            source: "auth password-policy".to_owned(),
            reference: None,
        }],
        None => vec![Finding {
            rule_id: id,
            severity: Severity::Low,
            status: Status::CouldNotInspect,
            title: "Password minimum length could not be read",
            evidence: "`auth password-policy` enforcement is on, but `minimum-length` is unset or unparseable".to_owned(),
            remediation: "Set an explicit `minimum-length` on the password policy.",
            source: "auth password-policy".to_owned(),
            reference: None,
        }],
    }
}

/// **BIGIP-SEC-004** — plaintext secrets: a credential-bearing field
/// ([`tcl_bigip::secrets::ENCRYPTED_SECRET_KEYS`]) that BIG-IP would normally
/// store under the unit master key, found in clear text instead.
fn check_plaintext_secrets(id: &'static str, ctx: &Ctx) -> Vec<Finding> {
    let plaintext: Vec<_> = collect_raw_secrets(ctx.source)
        .into_iter()
        .filter(|s| !s.encrypted)
        .collect();
    if plaintext.is_empty() {
        return vec![Finding {
            rule_id: id,
            severity: Severity::Info,
            status: Status::Clear,
            title: "No plaintext credential-bearing fields found",
            evidence: "every secret-bearing field in this source is stored under the unit master key (or none are present)".to_owned(),
            remediation: "No action required.",
            source: "config".to_owned(),
            reference: None,
        }];
    }
    plaintext
        .into_iter()
        .map(|s| {
            let high = s.field == "passphrase";
            Finding {
                rule_id: id,
                severity: if high { Severity::High } else { Severity::Medium },
                status: Status::Confirmed,
                title: "Credential-bearing field stored in clear text",
                evidence: format!(
                    "`{}` on `{}` is not encrypted under the unit master key",
                    s.field, s.object
                ),
                remediation: "Re-apply the value through `tmsh`/the GUI so BIG-IP re-encrypts it under the unit master key (`f5mku`), or rotate it if this config was hand-authored and never applied to a device.",
                source: s.object.clone(),
                reference: None,
            }
        })
        .collect()
}

/// PEM markers that indicate a private key IS passphrase-protected — the
/// value BIG-IP writes for an encrypted `PKCS#1`/legacy key (`Proc-Type: 4,
/// ENCRYPTED`) or a `PKCS#8` `EncryptedPrivateKeyInfo` block.
const ENCRYPTED_KEY_MARKERS: &[&str] = &["Proc-Type: 4,ENCRYPTED", "BEGIN ENCRYPTED PRIVATE KEY"];

/// **BIGIP-SEC-005** — plaintext (non-passphrase-protected) private key
/// material in the UCS filestore. Never reads or reports the key content
/// itself — only whether a passphrase marker is present.
fn check_exposed_private_keys(id: &'static str, ctx: &Ctx) -> Vec<Finding> {
    if ctx.files.is_empty() {
        return vec![Finding {
            rule_id: id,
            severity: Severity::Info,
            status: Status::NotApplicable,
            title: "Private-key material could not be inspected",
            evidence: "no UCS filestore was supplied (a bare config export carries no key files)"
                .to_owned(),
            remediation: "Supply a UCS backup to check for unprotected private-key material.",
            source: "UCS filestore".to_owned(),
            reference: None,
        }];
    }

    let mut out = Vec::new();
    let mut key_files = 0usize;
    for f in ctx.files {
        let Some(o) = f.as_object() else { continue };
        let path = o.get("path").and_then(J::as_str).unwrap_or("");
        let Some(content) = o.get("content").and_then(J::as_str) else {
            continue;
        };
        if !content.contains("PRIVATE KEY-----") {
            continue;
        }
        key_files += 1;
        let protected = ENCRYPTED_KEY_MARKERS.iter().any(|m| content.contains(m));
        if !protected {
            out.push(Finding {
                rule_id: id,
                severity: Severity::High,
                status: Status::Confirmed,
                title: "Unencrypted private key in the archive",
                evidence: format!("`{path}` carries private-key material with no passphrase-protection marker"),
                remediation: "Re-key with a passphrase-protected private key, and restrict who can pull UCS backups from this device.",
                source: path.to_owned(),
                reference: None,
            });
        }
    }
    if out.is_empty() {
        let evidence = if key_files == 0 {
            "no private-key files were found in the archive".to_owned()
        } else {
            format!("{key_files} private-key file(s) found, all passphrase-protected")
        };
        out.push(Finding {
            rule_id: id,
            severity: Severity::Info,
            status: Status::Clear,
            title: "No unprotected private-key material found",
            evidence,
            remediation: "No action required.",
            source: "UCS filestore".to_owned(),
            reference: None,
        });
    }
    out
}

/// **BIGIP-SEC-006** — a local account with full bash shell access but no
/// administrator role. Informational: the built-in `admin` account
/// shipping with `shell bash` is normal and is not flagged; this looks for
/// the less-common case of a lesser-privileged account also carrying full
/// OS shell access.
fn check_shell_access(id: &'static str, ctx: &Ctx) -> Vec<Finding> {
    let flagged: Vec<_> = ctx
        .auth_users
        .iter()
        .filter(|u| u.shell == "bash" && !u.has_admin_role)
        .collect();
    if flagged.is_empty() {
        return vec![Finding {
            rule_id: id,
            severity: Severity::Info,
            status: Status::Clear,
            title: "No non-administrative account has shell access",
            evidence: "every `auth user` with `shell bash` also carries an admin role".to_owned(),
            remediation: "No action required.",
            source: "auth user".to_owned(),
            reference: None,
        }];
    }
    flagged
        .into_iter()
        .map(|u| Finding {
            rule_id: id,
            severity: Severity::Low,
            status: Status::Confirmed,
            title: "Non-administrative account has full shell access",
            evidence: format!("`{}` has `shell bash` without an admin role", u.name),
            remediation: "Set `shell tmsh` unless this account genuinely needs OS-level access, and review why it was granted.",
            source: u.full_path.clone(),
            reference: None,
        })
        .collect()
}

/// One entry in the rule table: a stable id plus the function that produces
/// this rule's findings from the gathered [`Ctx`]. Extend the report's
/// Security coverage by adding a row here — never by adding a new
/// `match`/`if` branch to [`collect_security`] itself.
struct SecurityRule {
    id: &'static str,
    check: fn(&'static str, &Ctx) -> Vec<Finding>,
}

const RULES: &[SecurityRule] = &[
    SecurityRule {
        id: "BIGIP-SEC-001",
        check: check_default_credentials,
    },
    SecurityRule {
        id: "BIGIP-SEC-002",
        check: check_snmp_communities,
    },
    SecurityRule {
        id: "BIGIP-SEC-003",
        check: check_password_policy,
    },
    SecurityRule {
        id: "BIGIP-SEC-004",
        check: check_plaintext_secrets,
    },
    SecurityRule {
        id: "BIGIP-SEC-005",
        check: check_exposed_private_keys,
    },
    SecurityRule {
        id: "BIGIP-SEC-006",
        check: check_shell_access,
    },
];

/// Run every [`RULES`] entry against `source` (this device's SCF/config
/// text) and `files` (its UCS file inventory, empty for a bare config —
/// see [`crate::forensics::collect_forensics`] for the same shape), and
/// return `{findings: [...], counts: {critical, high, medium, low, info},
/// actionable}` for the report's Security tab.
///
/// `actionable` counts findings whose `status` is `confirmed` or
/// `could_not_inspect` — the ones worth a human's attention; `clear` and
/// `not_applicable` findings are still returned (for the "we checked, no
/// forgotten defaults were found" states the issue's acceptance criteria
/// asks the report to distinguish) but never inflate the review count.
#[must_use]
pub fn collect_security(source: &str, files: &[J]) -> J {
    let ctx = Ctx::build(source, files);
    let mut findings = Vec::new();
    for rule in RULES {
        findings.extend((rule.check)(rule.id, &ctx));
    }

    let mut counts: Map<String, J> = Map::new();
    for sev in [
        Severity::Critical,
        Severity::High,
        Severity::Medium,
        Severity::Low,
        Severity::Info,
    ] {
        let n = findings.iter().filter(|f| f.severity == sev).count();
        counts.insert(sev.as_str().into(), J::from(n));
    }
    let actionable = findings
        .iter()
        .filter(|f| matches!(f.status, Status::Confirmed | Status::CouldNotInspect))
        .count();

    json!({
        "findings": findings.iter().map(Finding::to_json).collect::<Vec<_>>(),
        "counts": counts,
        "actionable": actionable,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sec(source: &str) -> J {
        collect_security(source, &[])
    }
    fn sec_with_files(source: &str, files: &[J]) -> J {
        collect_security(source, files)
    }

    fn findings_for<'a>(model: &'a J, rule_id: &str) -> Vec<&'a J> {
        model["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["id"] == rule_id)
            .collect()
    }

    fn statuses<'a>(findings: &[&'a J]) -> Vec<&'a str> {
        findings
            .iter()
            .map(|f| f["status"].as_str().unwrap())
            .collect()
    }

    // ---- BIGIP-SEC-001: default credentials ----------------------------

    #[test]
    fn true_positive_default_admin_password_is_confirmed() {
        // admin/admin, SHA-512-crypt.
        let m = sec(
            "auth user /Common/admin {\n    encrypted-password $6$abcsalt12$xVTHU6Ifw7m21v5IXNpEQM1G/HDajebt/qt8a3FrnxzBmgXWpecsAYQNalE3Oaotb83HDNkXt3gc4TbJMjplv1\n}\n",
        );
        let f = findings_for(&m, "BIGIP-SEC-001");
        assert!(
            f.iter()
                .any(|x| x["status"] == "confirmed" && x["severity"] == "critical"),
            "{f:?}"
        );
    }

    #[test]
    fn false_positive_guard_non_default_admin_password_is_clear() {
        // A real (non-default) password must never be reported as confirmed.
        let m = sec(
            "auth user /Common/admin {\n    encrypted-password $6$abcsalt12$VE9mg4.EMW/6CgwAVZO84sWEJUut.3QT0a8eCMuNdhzUkUSRRprlIxSXEdBnv7bEzDyJu76xurILc4Ccw9Yn61\n}\n",
        );
        let f = findings_for(&m, "BIGIP-SEC-001");
        assert!(
            f.iter()
                .any(|x| x["source"] == "auth user /Common/admin" && x["status"] == "clear"),
            "{f:?}"
        );
        assert!(
            !f.iter().any(|x| x["status"] == "confirmed"),
            "a non-default password must never be confirmed: {f:?}"
        );
    }

    #[test]
    fn true_negative_no_admin_user_present_is_not_applicable() {
        let m = sec("ltm pool /Common/p { }\n");
        let f = findings_for(&m, "BIGIP-SEC-001");
        assert_eq!(statuses(&f), vec!["not_applicable", "not_applicable"]);
    }

    #[test]
    fn false_negative_now_caught_root_default_via_shadow_file() {
        // `root` never appears as an `auth user` stanza — it's an OS account
        // only visible via the UCS `/etc/shadow`. Before UCS-inventory support
        // this would be a false negative (silently unchecked); now it's caught.
        let files = vec![json!({
            "path": "etc/shadow",
            "content": "root:$6$abcsalt12$xVTHU6Ifw7m21v5IXNpEQM1G/HDajebt/qt8a3FrnxzBmgXWpecsAYQNalE3Oaotb83HDNkXt3gc4TbJMjplv1:19000:0:99999:7:::\n",
        })];
        let m = sec_with_files("ltm pool /Common/p { }\n", &files);
        let f = findings_for(&m, "BIGIP-SEC-001");
        // Not "root/default" — this shadow line's hash is the admin/admin
        // vector reused for a different user, so it must NOT verify against
        // "default"; assert instead that the source is inspected at all
        // (status is confirmed/clear, not not_applicable) now that a shadow
        // entry exists.
        let root = f
            .iter()
            .find(|x| x["source"] == "etc/shadow")
            .expect("root shadow entry inspected");
        assert_ne!(root["status"], "not_applicable");
    }

    #[test]
    fn shadow_root_default_password_confirmed() {
        let files = vec![json!({
            "path": "etc/shadow",
            "content": "root:$6$rounds=5000$saltstring$6p43fZYI.QjsRRE84rwofulMnyqp504VY9sFYK14/NXu6XfSJmmWs.6LU23LEbtyE.wFHWhcL9ahBHbNgfPCc1:19000:0:99999:7:::\n",
        })];
        let m = sec_with_files("ltm pool /Common/p { }\n", &files);
        let f = findings_for(&m, "BIGIP-SEC-001");
        assert!(
            f.iter()
                .any(|x| x["source"] == "etc/shadow" && x["status"] == "confirmed"),
            "{f:?}"
        );
    }

    #[test]
    fn malformed_and_unsupported_hash_formats_fail_closed() {
        // Classic DES-crypt (no `$` prefix) — must never be confirmed/clear,
        // only "could not inspect". `root` has no credential source at all in
        // this source, so it stays "not applicable"; only the `admin` result
        // (this fixture's actual subject) matters here.
        let m = sec("auth user /Common/admin {\n    encrypted-password xOAFZqRz5RduI\n}\n");
        let f = findings_for(&m, "BIGIP-SEC-001");
        let admin = f
            .iter()
            .find(|x| x["source"] == "auth user /Common/admin")
            .expect("admin credential inspected");
        assert_eq!(admin["status"], "could_not_inspect");
        assert!(
            !f.iter()
                .any(|x| x["status"] == "confirmed" || x["status"] == "clear"),
            "an unsupported format must never be confirmed or clear: {f:?}"
        );
    }

    #[test]
    fn locked_account_is_not_applicable_not_confirmed() {
        let m = sec("auth user /Common/admin {\n    encrypted-password !!\n}\n");
        let f = findings_for(&m, "BIGIP-SEC-001");
        let admin = f
            .iter()
            .find(|x| x["source"] == "auth user /Common/admin")
            .expect("admin credential inspected");
        assert_eq!(admin["status"], "not_applicable");
        assert!(!f.iter().any(|x| x["status"] == "confirmed"));
    }

    #[test]
    fn missing_credential_material_is_not_applicable() {
        let m = sec("auth user /Common/opsUser {\n    shell tmsh\n}\n");
        let f = findings_for(&m, "BIGIP-SEC-001");
        // Neither root nor admin present at all.
        assert_eq!(statuses(&f), vec!["not_applicable", "not_applicable"]);
    }

    #[test]
    fn no_secret_values_leak_into_the_model() {
        let m = sec(
            "auth user /Common/admin {\n    encrypted-password $6$abcsalt12$xVTHU6Ifw7m21v5IXNpEQM1G/HDajebt/qt8a3FrnxzBmgXWpecsAYQNalE3Oaotb83HDNkXt3gc4TbJMjplv1\n}\n",
        );
        let dump = serde_json::to_string(&m).unwrap();
        assert!(
            !dump.contains("$6$"),
            "stored hash must never appear in the model: {dump}"
        );
        assert!(
            !dump.contains("abcsalt12"),
            "salt must never appear in the model: {dump}"
        );
    }

    // ---- BIGIP-SEC-002: SNMP communities --------------------------------

    #[test]
    fn true_positive_default_snmp_community_rw_is_high() {
        let m = sec(
            "sys snmp {\n    communities {\n        c1 {\n            community-name public\n            access rw\n        }\n    }\n}\n",
        );
        let f = findings_for(&m, "BIGIP-SEC-002");
        assert!(
            f.iter()
                .any(|x| x["status"] == "confirmed" && x["severity"] == "high"),
            "{f:?}"
        );
    }

    #[test]
    fn true_positive_default_snmp_community_ro_is_medium() {
        let m = sec(
            "sys snmp {\n    communities {\n        c1 {\n            community-name private\n            access ro\n        }\n    }\n}\n",
        );
        let f = findings_for(&m, "BIGIP-SEC-002");
        assert!(
            f.iter()
                .any(|x| x["status"] == "confirmed" && x["severity"] == "medium"),
            "{f:?}"
        );
    }

    #[test]
    fn false_positive_guard_custom_community_name_is_clear() {
        // A stanza *label* that looks default (e.g. "comm-public") must not
        // trip the rule when the actual `community-name` value is custom.
        let m = sec(
            "sys snmp {\n    communities {\n        comm-public {\n            community-name a-real-secret-string\n            access ro\n        }\n    }\n}\n",
        );
        let f = findings_for(&m, "BIGIP-SEC-002");
        assert_eq!(statuses(&f), vec!["clear"]);
        let dump = serde_json::to_string(&f).unwrap();
        assert!(
            !dump.contains("a-real-secret-string"),
            "custom community string must not be echoed: {dump}"
        );
    }

    #[test]
    fn true_negative_no_snmp_configured() {
        let m = sec("ltm pool /Common/p { }\n");
        let f = findings_for(&m, "BIGIP-SEC-002");
        assert_eq!(statuses(&f), vec!["clear"]);
    }

    // ---- BIGIP-SEC-003: password policy ---------------------------------

    #[test]
    fn true_positive_password_policy_disabled() {
        let m = sec("auth password-policy {\n    policy-enforcement disabled\n}\n");
        let f = findings_for(&m, "BIGIP-SEC-003");
        assert!(
            f.iter()
                .any(|x| x["status"] == "confirmed" && x["severity"] == "high"),
            "{f:?}"
        );
    }

    #[test]
    fn true_positive_min_length_weak() {
        let m = sec(
            "auth password-policy {\n    policy-enforcement enabled\n    minimum-length 6\n}\n",
        );
        let f = findings_for(&m, "BIGIP-SEC-003");
        assert!(f.iter().any(|x| x["status"] == "confirmed"), "{f:?}");
    }

    #[test]
    fn false_positive_guard_strong_policy_is_clear() {
        let m = sec(
            "auth password-policy {\n    policy-enforcement enabled\n    minimum-length 14\n}\n",
        );
        let f = findings_for(&m, "BIGIP-SEC-003");
        assert_eq!(statuses(&f), vec!["clear"]);
    }

    #[test]
    fn missing_password_policy_could_not_inspect() {
        let m = sec("ltm pool /Common/p { }\n");
        let f = findings_for(&m, "BIGIP-SEC-003");
        assert_eq!(statuses(&f), vec!["could_not_inspect"]);
    }

    // ---- BIGIP-SEC-004: plaintext secrets --------------------------------

    #[test]
    fn true_positive_plaintext_secret_detected() {
        let m = sec("ltm monitor https /Common/mon {\n    password clearpw\n}\n");
        let f = findings_for(&m, "BIGIP-SEC-004");
        assert!(f.iter().any(|x| x["status"] == "confirmed"), "{f:?}");
        let dump = serde_json::to_string(&f).unwrap();
        assert!(
            !dump.contains("clearpw"),
            "the plaintext value must never be echoed: {dump}"
        );
    }

    #[test]
    fn false_positive_guard_encrypted_secret_is_clear() {
        let m = sec(
            "auth radius-server /Common/r {\n    secret $M$mn$9uYx+bTjLD8YSGZAUEfFwHvpSDwZsL25kZxdn5NZmCI=\n}\n",
        );
        let f = findings_for(&m, "BIGIP-SEC-004");
        assert_eq!(statuses(&f), vec!["clear"]);
    }

    // ---- BIGIP-SEC-005: exposed private keys -----------------------------

    #[test]
    fn true_positive_unprotected_private_key() {
        let files = vec![json!({
            "path": "config/ssl/ssl.key/x.key",
            "content": "-----BEGIN RSA PRIVATE KEY-----\nMII...\n-----END RSA PRIVATE KEY-----\n",
        })];
        let m = sec_with_files("ltm pool /Common/p { }\n", &files);
        let f = findings_for(&m, "BIGIP-SEC-005");
        assert!(
            f.iter()
                .any(|x| x["status"] == "confirmed" && x["severity"] == "high"),
            "{f:?}"
        );
    }

    #[test]
    fn false_positive_guard_passphrase_protected_key_is_clear() {
        let files = vec![json!({
            "path": "config/ssl/ssl.key/x.key",
            "content": "-----BEGIN RSA PRIVATE KEY-----\nProc-Type: 4,ENCRYPTED\nDEK-Info: AES-256-CBC,ABCDEF\n\nMII...\n-----END RSA PRIVATE KEY-----\n",
        })];
        let m = sec_with_files("ltm pool /Common/p { }\n", &files);
        let f = findings_for(&m, "BIGIP-SEC-005");
        assert_eq!(statuses(&f), vec!["clear"]);
    }

    #[test]
    fn no_ucs_files_is_not_applicable() {
        let m = sec("ltm pool /Common/p { }\n");
        let f = findings_for(&m, "BIGIP-SEC-005");
        assert_eq!(statuses(&f), vec!["not_applicable"]);
    }

    // ---- BIGIP-SEC-006: shell access --------------------------------------

    #[test]
    fn true_positive_non_admin_bash_shell_flagged() {
        let m = sec(
            "auth user /Common/opsUser {\n    partition-access {\n        Common {\n            role manager\n        }\n    }\n    shell bash\n}\n",
        );
        let f = findings_for(&m, "BIGIP-SEC-006");
        assert!(f.iter().any(|x| x["status"] == "confirmed"), "{f:?}");
    }

    #[test]
    fn false_positive_guard_admin_bash_shell_not_flagged() {
        let m = sec(
            "auth user /Common/admin {\n    partition-access {\n        all-partitions {\n            role admin\n        }\n    }\n    shell bash\n}\n",
        );
        let f = findings_for(&m, "BIGIP-SEC-006");
        assert_eq!(statuses(&f), vec!["clear"]);
    }

    #[test]
    fn false_positive_guard_tmsh_shell_not_flagged() {
        let m = sec(
            "auth user /Common/opsUser {\n    partition-access {\n        Common {\n            role manager\n        }\n    }\n    shell tmsh\n}\n",
        );
        let f = findings_for(&m, "BIGIP-SEC-006");
        assert_eq!(statuses(&f), vec!["clear"]);
    }

    // ---- overall model shape ----------------------------------------------

    #[test]
    fn counts_reflect_severities() {
        let m = sec(
            "auth user /Common/admin {\n    encrypted-password $6$abcsalt12$xVTHU6Ifw7m21v5IXNpEQM1G/HDajebt/qt8a3FrnxzBmgXWpecsAYQNalE3Oaotb83HDNkXt3gc4TbJMjplv1\n}\n",
        );
        assert!(m["counts"]["critical"].as_u64().unwrap() >= 1);
        assert!(m["actionable"].as_u64().unwrap() >= 1);
    }

    #[test]
    fn model_is_json_serialisable() {
        let m = sec(
            "auth user /Common/admin {\n    encrypted-password $6$abc$def\n}\nsys snmp {\n    communities {\n        c { community-name public }\n    }\n}\nauth password-policy {\n    policy-enforcement disabled\n}\n",
        );
        serde_json::to_string(&m).expect("model serialises");
    }
}
