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

//! The pure iRules functions: TMM's own commands, which no runtime here
//! registers, written once for the registry's direct routes and for the iRule
//! test simulator, which registers each as a host command ([`call`]).
//!
//! Each answers only what F5's published reference states for it
//! (`clouddocs.f5.com/api/irules/<command>.html`, its examples the vectors the
//! tests pin) and is [`Unmodelled`] for every input outside that: the route
//! declines it, and the simulator falls back to its generic stub. A byte
//! function reads a word of ASCII text a byte per character, the one reading
//! every conversion TMM might make agrees on.

/// A function's arguments are outside the behaviour the reference states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unmodelled;

type Answer<T> = Result<T, Unmodelled>;

const UNMODELLED: Unmodelled = Unmodelled;

/// What a function answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Output {
    /// Text.
    Text(String),
    /// An integer.
    Int(i64),
    /// Bytes: TMM's byte array.
    Bytes(Vec<u8>),
}

/// The commands [`call`] runs, by their iRules names.
pub const COMMANDS: &[&str] = &[
    "b64encode",
    "b64decode",
    "crc32",
    "md5",
    "sha1",
    "sha256",
    "sha384",
    "sha512",
    "findstr",
    "getfield",
    "substr",
    "domain",
    "URI::basename",
    "URI::path",
    "URI::query",
    "URI::host",
    "URI::port",
    "URI::protocol",
    "URI::decode",
    "URI::encode",
    "URI::compare",
    "IP::addr",
];

/// The bytes of a word of ASCII text, a byte per character.
fn ascii_bytes(word: &str) -> Answer<&[u8]> {
    ascii(word).map(str::as_bytes)
}

/// The pure function `command` over the words after it.
///
/// # Errors
///
/// [`Unmodelled`] for a command [`COMMANDS`] does not list, a shape the
/// command does not take, or arguments outside the published behaviour.
pub fn call(command: &str, words: &[&str]) -> Answer<Output> {
    let hashed = |kind: Digest| match words {
        [word] => Ok(Output::Bytes(digest(kind, ascii_bytes(word)?))),
        _ => Err(UNMODELLED),
    };
    let flag = |same: bool| Output::Int(i64::from(same));
    match (command, words) {
        ("b64encode", [word]) => Ok(Output::Text(b64encode(ascii_bytes(word)?))),
        ("b64decode", [word]) => b64decode(ascii_bytes(word)?).map(Output::Bytes),
        ("crc32", [word]) => Ok(Output::Int(crc32(ascii_bytes(word)?))),
        ("md5", _) => hashed(Digest::Md5),
        ("sha1", _) => hashed(Digest::Sha1),
        ("sha256", _) => hashed(Digest::Sha256),
        ("sha384", _) => hashed(Digest::Sha384),
        ("sha512", _) => hashed(Digest::Sha512),
        ("findstr", [text, search, rest @ ..]) if rest.len() <= 2 => {
            findstr(text, search, rest.first().copied(), rest.get(1).copied()).map(Output::Text)
        }
        ("getfield", [text, split, field]) => getfield(text, split, field).map(Output::Text),
        ("substr", [text, skip, rest @ ..]) if rest.len() <= 1 => {
            substr(text, skip, rest.first().copied()).map(Output::Text)
        }
        ("domain", [text, count]) => domain(text, count).map(Output::Text),
        ("URI::basename", [uri]) => uri_basename(uri).map(Output::Text),
        ("URI::path", [uri, rest @ ..]) if rest.len() <= 2 => {
            uri_path(uri, rest.first().copied(), rest.get(1).copied()).map(Output::Text)
        }
        ("URI::query", [uri, rest @ ..]) if rest.len() <= 1 => {
            uri_query(uri, rest.first().copied()).map(Output::Text)
        }
        ("URI::host", [uri]) => uri_host(uri).map(Output::Text),
        ("URI::port", [uri]) => uri_port(uri).map(|port| Output::Int(i64::from(port))),
        ("URI::protocol", [uri]) => uri_protocol(uri).map(Output::Text),
        ("URI::decode", [uri]) => uri_decode(uri).map(Output::Text),
        ("URI::encode", [uri]) => uri_encode(uri).map(Output::Text),
        ("URI::compare", [first, second]) => uri_compare(first, second).map(flag),
        ("IP::addr", [first, "equals", second]) => ip_addr_equals(first, second).map(flag),
        _ => Err(UNMODELLED),
    }
}

/// `b64encode`: the bytes as base64, padded, on one line.
#[must_use]
pub fn b64encode(bytes: &[u8]) -> String {
    let encoded = crate::binary::base64_encode(bytes, 0, b"");
    String::from_utf8(encoded).unwrap_or_default()
}

/// `b64decode`: the bytes a canonical base64 text spells — its length a
/// multiple of four, its alphabet the standard one, its padding only at the
/// end — which is every text [`b64encode`] gives. TMM raises `conversion
/// error` on text it cannot decode, but the reference does not say which
/// texts those are, so anything else is unmodelled.
///
/// # Errors
///
/// [`Unmodelled`] for a text that is not canonical base64.
pub fn b64decode(text: &[u8]) -> Answer<Vec<u8>> {
    let decoded = crate::binary::base64_decode(text, true).map_err(|_| UNMODELLED)?;
    if crate::binary::base64_encode(&decoded, 0, b"") == text {
        Ok(decoded)
    } else {
        Err(UNMODELLED)
    }
}

/// `crc32`: the CRC-32 of the bytes (polynomial `0x04c11db7`, reflected,
/// initialised and complemented with `0xffffffff`, as zlib's), which TMM
/// returns as a 64-bit integer sign-extended from 32 bits: a checksum whose
/// high bit is set is negative (the reference's masking idiom, `expr
/// {0xffffffff & [crc32 $msg]}`, recovers zlib's unsigned value).
#[must_use]
pub fn crc32(bytes: &[u8]) -> i64 {
    let mut crc = 0xffff_ffff_u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let low = crc & 1;
            crc >>= 1;
            if low != 0 {
                crc ^= 0xedb8_8320;
            }
        }
    }
    i64::from(i32::from_ne_bytes((!crc).to_ne_bytes()))
}

/// The message digests `md5`, `sha1`, `sha256`, `sha384` and `sha512` take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Digest {
    /// `md5`: RFC 1321's 16 bytes.
    Md5,
    /// `sha1`: FIPS 180-4's 20 bytes.
    Sha1,
    /// `sha256`: FIPS 180-4's 32 bytes.
    Sha256,
    /// `sha384`: FIPS 180-4's 48 bytes.
    Sha384,
    /// `sha512`: FIPS 180-4's 64 bytes.
    Sha512,
}

/// The raw digest of the bytes, which TMM returns as a byte array (its
/// examples read it with `binary scan … w1`).
#[must_use]
pub fn digest(kind: Digest, bytes: &[u8]) -> Vec<u8> {
    use md5::Digest as _;
    match kind {
        Digest::Md5 => md5::Md5::digest(bytes).to_vec(),
        Digest::Sha1 => sha1::Sha1::digest(bytes).to_vec(),
        Digest::Sha256 => sha2::Sha256::digest(bytes).to_vec(),
        Digest::Sha384 => sha2::Sha384::digest(bytes).to_vec(),
        Digest::Sha512 => sha2::Sha512::digest(bytes).to_vec(),
    }
}

/// A count word the reference reads as a number: decimal digits alone, with no
/// sign, no leading zero and no space. TMM's integer reading of anything else
/// is not stated.
fn count(word: &str) -> Answer<usize> {
    let canonical = !word.is_empty()
        && word.bytes().all(|byte| byte.is_ascii_digit())
        && (word == "0" || !word.starts_with('0'));
    if canonical {
        word.parse().map_err(|_| UNMODELLED)
    } else {
        Err(UNMODELLED)
    }
}

/// The ASCII text the string functions read, a byte per character: TMM's
/// character model for anything else is not stated.
fn ascii(text: &str) -> Answer<&str> {
    if text.is_ascii() {
        Ok(text)
    } else {
        Err(UNMODELLED)
    }
}

/// How a terminator word reads: a count of characters, or a string to stop
/// at. A word of digits is a count.
enum Terminator<'a> {
    Count(usize),
    Text(&'a str),
}

impl<'a> Terminator<'a> {
    /// A word of digits is a count. A word that starts like a number — a
    /// digit, a sign, a space — and is not one is neither form the reference
    /// states, since whether TMM reads its leading digits is not said.
    fn of(word: &'a str) -> Answer<Self> {
        if !word.is_empty() && word.bytes().all(|byte| byte.is_ascii_digit()) {
            return count(word).map(Self::Count);
        }
        match word.bytes().next() {
            None => Err(UNMODELLED),
            Some(first) if first.is_ascii_digit() || b"+- \t".contains(&first) => Err(UNMODELLED),
            Some(_) => Ok(Self::Text(word)),
        }
    }
}

/// `substr string skip_count ?terminator?`: from character `skip_count` (0 is
/// the first) to the end, to `terminator` characters on, or to the first
/// `terminator` string after it, which is the end where it does not occur.
/// A count of 0 answered the whole rest in some releases and not in others
/// (the reference's note on 11.5.4 and 11.6.0), so it is unmodelled.
///
/// # Errors
///
/// [`Unmodelled`] outside the published behaviour.
pub fn substr(text: &str, skip: &str, terminator: Option<&str>) -> Answer<String> {
    let text = ascii(text)?;
    let start = count(skip)?;
    if start > text.len() {
        return Err(UNMODELLED);
    }
    let rest = &text[start..];
    let taken = match terminator.map(Terminator::of).transpose()? {
        None => rest,
        Some(Terminator::Count(0)) => return Err(UNMODELLED),
        Some(Terminator::Count(length)) => &rest[..length.min(rest.len())],
        Some(Terminator::Text(stop)) => {
            let stop = ascii(stop)?;
            rest.find(stop).map_or(rest, |at| &rest[..at])
        }
    };
    Ok(taken.to_owned())
}

/// `findstr string search_string ?skip_count ?terminator??`: the text from
/// `skip_count` characters after the start of the first `search_string`, to
/// the end, to `terminator` characters on, or to the first `terminator` string
/// after the start. The reference states neither a search string that does not
/// occur nor a terminator string that does not, so both are unmodelled.
///
/// # Errors
///
/// [`Unmodelled`] outside the published behaviour.
pub fn findstr(
    text: &str,
    search: &str,
    skip: Option<&str>,
    terminator: Option<&str>,
) -> Answer<String> {
    let text = ascii(text)?;
    let search = ascii(search)?;
    if search.is_empty() {
        return Err(UNMODELLED);
    }
    let found = text.find(search).ok_or(UNMODELLED)?;
    let start = found + skip.map_or(Ok(0), count)?;
    if start > text.len() {
        return Err(UNMODELLED);
    }
    let rest = &text[start..];
    let taken = match terminator.map(Terminator::of).transpose()? {
        None => rest,
        Some(Terminator::Count(0)) => return Err(UNMODELLED),
        Some(Terminator::Count(length)) => &rest[..length.min(rest.len())],
        Some(Terminator::Text(stop)) => {
            let stop = ascii(stop)?;
            &rest[..rest.find(stop).ok_or(UNMODELLED)?]
        }
    };
    Ok(taken.to_owned())
}

/// `getfield string split field_number`: the `field_number`th (from 1) of the
/// fields `split` separates. The reference states neither an empty field
/// (separators side by side, or at an end) nor a field past the last, so a
/// string with an empty field up to the one asked for, and a number past the
/// last field, are unmodelled.
///
/// # Errors
///
/// [`Unmodelled`] outside the published behaviour.
pub fn getfield(text: &str, split: &str, field: &str) -> Answer<String> {
    let text = ascii(text)?;
    let split = ascii(split)?;
    let field = count(field)?;
    if split.is_empty() || field == 0 {
        return Err(UNMODELLED);
    }
    let fields: Vec<&str> = text.split(split).collect();
    let wanted = fields.get(field - 1).ok_or(UNMODELLED)?;
    if fields[..field].iter().any(|part| part.is_empty()) {
        return Err(UNMODELLED);
    }
    Ok((*wanted).to_owned())
}

/// `domain string count`: the last `count` labels of a dotted name. A name
/// with an empty label, a count of 0 and a count past the labels the name has
/// are unmodelled: the reference shows none of them.
///
/// # Errors
///
/// [`Unmodelled`] outside the published behaviour.
pub fn domain(text: &str, wanted: &str) -> Answer<String> {
    let text = ascii(text)?;
    let wanted = count(wanted)?;
    let labels: Vec<&str> = text.split('.').collect();
    if wanted == 0 || wanted > labels.len() || labels.iter().any(|label| label.is_empty()) {
        return Err(UNMODELLED);
    }
    Ok(labels[labels.len() - wanted..].join("."))
}

/// A URI split as the `URI::` functions read it: an absolute one
/// (`scheme://authority/path?query`) or a path from the root
/// (`/path?query`). A fragment, a `%` escape, a scheme the reference shows no
/// example of, an authority with user information or an IPv6 literal, an
/// empty path segment or a dot segment are each a reading the reference does
/// not state, and unmodelled; so is a URI of any other shape.
struct Uri<'a> {
    scheme: Option<&'a str>,
    host: &'a str,
    port: Option<u16>,
    /// The directories of the path: every segment but the last.
    directories: Vec<&'a str>,
    /// The last segment of the path.
    basename: &'a str,
}

/// The schemes the reference shows a default port for.
const KNOWN_SCHEMES: &[(&str, u16)] = &[("http", 80), ("https", 443), ("ftp", 21), ("sip", 5060)];

impl<'a> Uri<'a> {
    fn parse(text: &'a str) -> Answer<Self> {
        let text = ascii(text)?;
        if text.contains(['#', '%']) {
            return Err(UNMODELLED);
        }
        let (scheme, rest) = match text.split_once("://") {
            Some((scheme, rest)) if is_scheme(scheme) => (Some(scheme), rest),
            _ if text.starts_with('/') => (None, text),
            _ => return Err(UNMODELLED),
        };
        let (host, port, rest) = if scheme.is_some() {
            let (authority, path) = rest
                .find(['/', '?'])
                .map_or((rest, ""), |at| rest.split_at(at));
            let (host, port) = match authority.split_once(':') {
                Some((host, port)) => (host, Some(count(port)?)),
                None => (authority, None),
            };
            if host.is_empty() || host.contains(['@', '[', ']']) {
                return Err(UNMODELLED);
            }
            let port = port
                .map(|port| u16::try_from(port).map_err(|_| UNMODELLED))
                .transpose()?;
            (host, port, path)
        } else {
            ("", None, rest)
        };
        // The query plays no part in the path.
        let path = rest.split_once('?').map_or(rest, |(path, _)| path);
        let segments: Vec<&str> = path.split('/').collect();
        let (basename, directories) = match segments.split_last() {
            // A path from the root splits into an empty first segment.
            Some((last, ["", middle @ ..])) => (*last, middle.to_vec()),
            // An absolute URI with no path, before its query or not.
            Some((&"", [])) => ("", Vec::new()),
            _ => return Err(UNMODELLED),
        };
        if directories
            .iter()
            .any(|segment| segment.is_empty() || segment.starts_with('.'))
            || matches!(basename, "." | "..")
        {
            return Err(UNMODELLED);
        }
        Ok(Self {
            scheme,
            host,
            port,
            directories,
            basename,
        })
    }

    /// The path's directories as the reference spells them, `/` around each.
    fn directory_path(directories: &[&str]) -> String {
        let mut path = String::from("/");
        for directory in directories {
            path.push_str(directory);
            path.push('/');
        }
        path
    }
}

/// Whether `word` is a scheme: a letter, then letters and digits.
fn is_scheme(word: &str) -> bool {
    let mut bytes = word.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
        && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
}

/// `URI::basename uri`: the last segment of the path, without the query
/// (`/main/index.jsp?user=test&login=check` → `index.jsp`).
///
/// # Errors
///
/// [`Unmodelled`] outside the published behaviour.
pub fn uri_basename(uri: &str) -> Answer<String> {
    let parsed = Uri::parse(uri)?;
    if parsed.basename.is_empty() {
        return Err(UNMODELLED);
    }
    Ok(parsed.basename.to_owned())
}

/// `URI::path uri ?depth | start ?end??`: the directories of the path between
/// `/` (`/path/to/file.ext?param=value` → `/path/to/`), their count for the
/// word `depth`, or those from `start` (from 1) to `end`, which stops at the
/// last. A start past the last directory, a start of 0 (which TMM raises on)
/// and an end before the start are unmodelled.
///
/// # Errors
///
/// [`Unmodelled`] outside the published behaviour.
pub fn uri_path(uri: &str, from: Option<&str>, to: Option<&str>) -> Answer<String> {
    let parsed = Uri::parse(uri)?;
    let directories = &parsed.directories;
    if from == Some("depth") && to.is_none() {
        return Ok(directories.len().to_string());
    }
    let Some(from) = from else {
        return Ok(Uri::directory_path(directories));
    };
    let first = count(from)?;
    if first == 0 || first > directories.len() {
        return Err(UNMODELLED);
    }
    let last = match to {
        None => directories.len(),
        Some(to) => {
            let last = count(to)?;
            if last < first {
                return Err(UNMODELLED);
            }
            last.min(directories.len())
        }
    };
    Ok(Uri::directory_path(&directories[first - 1..last]))
}

/// `URI::query uri ?name?`: the text after the first `?`, empty where there is
/// none, or the value of the parameter `name` in it, empty where it is absent
/// (`URI::query "param1=val1&param2=val2" param1` is empty: a query starts at
/// `?`). A parameter given twice, or without `=`, is unmodelled, and so is a
/// value with a `+` the reference does not say it decodes.
///
/// # Errors
///
/// [`Unmodelled`] outside the published behaviour.
pub fn uri_query(uri: &str, name: Option<&str>) -> Answer<String> {
    let text = ascii(uri)?;
    if text.contains(['#', '%']) {
        return Err(UNMODELLED);
    }
    let query = text.split_once('?').map_or("", |(_, query)| query);
    let Some(name) = name else {
        return Ok(query.to_owned());
    };
    let name = ascii(name)?;
    if name.is_empty() || name.contains(['=', '&']) {
        return Err(UNMODELLED);
    }
    let mut found: Option<&str> = None;
    for parameter in query.split('&').filter(|parameter| !parameter.is_empty()) {
        let (key, value) = parameter.split_once('=').ok_or(UNMODELLED)?;
        // Whether a name matches another case is not stated.
        if key != name && key.eq_ignore_ascii_case(name) {
            return Err(UNMODELLED);
        }
        if key == name {
            if found.is_some() || value.contains('+') {
                return Err(UNMODELLED);
            }
            found = Some(value);
        }
    }
    Ok(found.unwrap_or_default().to_owned())
}

/// `URI::host uri`: the host of an absolute URI, without its port, and empty
/// for a path from the root.
///
/// # Errors
///
/// [`Unmodelled`] outside the published behaviour.
pub fn uri_host(uri: &str) -> Answer<String> {
    Uri::parse(uri).map(|parsed| parsed.host.to_owned())
}

/// `URI::port uri`: the port an absolute URI names, or its scheme's default —
/// 80, 443, 21 and 5060 for `http`, `https`, `ftp` and `sip` — and 80 for a
/// path from the root. TMM answers 0 for a scheme it does not know, but which
/// it knows is not stated beyond the four, so any other scheme without a port
/// is unmodelled.
///
/// # Errors
///
/// [`Unmodelled`] outside the published behaviour.
pub fn uri_port(uri: &str) -> Answer<u16> {
    let parsed = Uri::parse(uri)?;
    if let Some(port) = parsed.port {
        return Ok(port);
    }
    match parsed.scheme {
        None => Ok(80),
        Some(scheme) => KNOWN_SCHEMES
            .iter()
            .find(|(known, _)| *known == scheme)
            .map(|(_, port)| *port)
            .ok_or(UNMODELLED),
    }
}

/// `URI::protocol uri`: the scheme before `://`, and empty for a path from
/// the root.
///
/// # Errors
///
/// [`Unmodelled`] outside the published behaviour.
pub fn uri_protocol(uri: &str) -> Answer<String> {
    Uri::parse(uri).map(|parsed| parsed.scheme.unwrap_or_default().to_owned())
}

/// The characters `URI::encode` writes as they are, and those it escapes as
/// `%` and two hex digits, in the reference's example: the hex letters of
/// `[` and `]` are lower case in one edition of the reference and absent from
/// the other, and the treatment of every character the example does not show
/// is not stated.
fn encode_char(byte: u8) -> Answer<Option<u8>> {
    match byte {
        b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'(' | b')' | b'*' => Ok(None),
        b' ' | b'&' | b'@' | b'#' => Ok(Some(byte)),
        _ => Err(UNMODELLED),
    }
}

/// `URI::encode uri`: the reference's percent-encoding.
///
/// # Errors
///
/// [`Unmodelled`] for a character the reference's example does
/// not show.
pub fn uri_encode(text: &str) -> Answer<String> {
    let mut out = String::with_capacity(text.len());
    for byte in ascii(text)?.bytes() {
        match encode_char(byte)? {
            None => out.push(char::from(byte)),
            Some(escaped) => {
                const HEX: &[u8; 16] = b"0123456789abcdef";
                out.push('%');
                out.push(char::from(HEX[usize::from(escaped >> 4)]));
                out.push(char::from(HEX[usize::from(escaped & 0xf)]));
            }
        }
    }
    Ok(out)
}

/// `URI::decode uri`: one pass of percent-decoding, either case of hex digit
/// (RFC 3986 § 2.1, which the reference cites). A `+`, a `%` not followed by
/// two hex digits, and an escape of a byte above 0x7f — whose character TMM
/// would give is not stated — are unmodelled.
///
/// # Errors
///
/// [`Unmodelled`] outside the published behaviour.
pub fn uri_decode(text: &str) -> Answer<String> {
    let bytes = ascii(text)?.as_bytes();
    let mut out = String::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'+' => return Err(UNMODELLED),
            b'%' => {
                let hex = bytes.get(at + 1..at + 3).ok_or(UNMODELLED)?;
                let hex = std::str::from_utf8(hex).map_err(|_| UNMODELLED)?;
                if !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                    return Err(UNMODELLED);
                }
                let byte = u8::from_str_radix(hex, 16).map_err(|_| UNMODELLED)?;
                if !byte.is_ascii() {
                    return Err(UNMODELLED);
                }
                out.push(char::from(byte));
                at += 3;
            }
            byte => {
                out.push(char::from(byte));
                at += 1;
            }
        }
    }
    Ok(out)
}

/// `URI::compare uri1 uri2`, under RFC 2616 § 3.2.3's equivalence: two equal
/// texts are the same URI however it normalises, and two paths from the root
/// with no `%` escape, query or fragment that differ are two, since the rule
/// leaves such a path as it is. Any other pair is unmodelled.
///
/// # Errors
///
/// [`Unmodelled`] outside the published behaviour.
pub fn uri_compare(first: &str, second: &str) -> Answer<bool> {
    if first == second {
        return Ok(true);
    }
    let plain = |uri: &str| {
        uri.starts_with('/')
            && uri.is_ascii()
            && !uri.contains(['%', '?', '#'])
            && !uri.split('/').any(|segment| segment.starts_with('.'))
    };
    if plain(first) && plain(second) {
        return Ok(false);
    }
    Err(UNMODELLED)
}

/// An IPv4 address with an optional prefix length, as `IP::addr` reads one: a
/// dotted quad of decimal octets with no leading zero, and `/N` with `N` from
/// 0 to 32. A dotted mask, a route domain (`%N`) and IPv6 are unmodelled.
fn ipv4(text: &str) -> Answer<(u32, Option<u32>)> {
    let (address, prefix) = match text.split_once('/') {
        Some((address, prefix)) => {
            let prefix = count(prefix)?;
            if prefix > 32 {
                return Err(UNMODELLED);
            }
            (
                address,
                Some(u32::try_from(prefix).map_err(|_| UNMODELLED)?),
            )
        }
        None => (text, None),
    };
    let octets: Vec<&str> = address.split('.').collect();
    let [a, b, c, d] = octets.as_slice() else {
        return Err(UNMODELLED);
    };
    let mut value = 0u32;
    for octet in [a, b, c, d] {
        let octet = count(octet)?;
        let octet = u8::try_from(octet).map_err(|_| UNMODELLED)?;
        value = (value << 8) | u32::from(octet);
    }
    Ok((value, prefix))
}

/// `IP::addr addr1[/mask] equals addr2[/mask]`: whether the two addresses fall
/// in one network under the prefix one side gives, or are the same address
/// when neither does (`10.42.2.0/24 equals 10.42.2.1` is true). Prefixes on
/// both sides are unmodelled: the reference does not say which applies.
///
/// # Errors
///
/// [`Unmodelled`] outside the published behaviour.
pub fn ip_addr_equals(first: &str, second: &str) -> Answer<bool> {
    let (first, first_prefix) = ipv4(first)?;
    let (second, second_prefix) = ipv4(second)?;
    let prefix = match (first_prefix, second_prefix) {
        (Some(_), Some(_)) => return Err(UNMODELLED),
        (Some(prefix), None) | (None, Some(prefix)) => prefix,
        (None, None) => 32,
    };
    let mask = u32::MAX.checked_shl(32 - prefix).unwrap_or(0);
    Ok(first & mask == second & mask)
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::*;

    fn text(command: &str, words: &[&str]) -> Option<String> {
        match call(command, words) {
            Ok(Output::Text(text)) => Some(text),
            Ok(Output::Int(value)) => Some(value.to_string()),
            Ok(Output::Bytes(bytes)) => Some(bytes.iter().fold(String::new(), |mut hex, byte| {
                let _ = write!(hex, "{byte:02x}");
                hex
            })),
            Err(Unmodelled) => None,
        }
    }

    /// Each function answers inside what the reference states and is
    /// unmodelled at its edges: a byte function's non-ASCII word, a
    /// non-canonical base64 text, a terminator count of 0, a search string
    /// or a field that is not there, a scheme with no default port the
    /// reference lists, two prefixes, a shape the command does not take.
    #[test]
    fn each_function_answers_only_what_the_reference_states() {
        assert_eq!(text("b64encode", &["abc"]).as_deref(), Some("YWJj"));
        assert_eq!(text("b64encode", &["caf\u{e9}"]), None);
        assert_eq!(text("b64encode", &["a", "b"]), None);
        assert_eq!(text("b64decode", &["YWJj"]).as_deref(), Some("616263"));
        assert_eq!(text("b64decode", &["YWJ"]), None);
        assert_eq!(text("b64decode", &["YWJk="]), None);
        assert_eq!(text("b64decode", &["YW Jj"]), None);
        assert_eq!(text("crc32", &["123456789"]).as_deref(), Some("-873187034"));
        assert_eq!(text("substr", &["abcdef", "1", "0"]), None);
        assert_eq!(text("substr", &["abcdef", "1", "-2"]), None);
        assert_eq!(text("substr", &["abcdef", "9"]), None);
        assert_eq!(text("findstr", &["abc", "x"]), None);
        assert_eq!(text("findstr", &["abc", "b", "0", "q"]), None);
        assert_eq!(text("findstr", &["abcabc", "b"]).as_deref(), Some("bcabc"));
        assert_eq!(text("getfield", &["a::b", ":", "2"]), None);
        assert_eq!(text("getfield", &["a:b", ":", "3"]), None);
        assert_eq!(text("domain", &["a..b", "1"]), None);
        assert_eq!(text("domain", &["a.b", "2"]).as_deref(), Some("a.b"));
        assert_eq!(text("domain", &["a.b", "5"]), None);
        assert_eq!(text("URI::port", &["myproto://example.com/"]), None);
        assert_eq!(text("URI::host", &["http://user@example.com/"]), None);
        assert_eq!(text("URI::basename", &["/a/../b"]), None);
        assert_eq!(text("URI::query", &["/p?a=1&a=2", "a"]), None);
        assert_eq!(text("URI::query", &["/p?A=1", "a"]), None);
        assert_eq!(text("URI::decode", &["a+b"]), None);
        assert_eq!(text("URI::decode", &["%e9"]), None);
        assert_eq!(text("URI::decode", &["%2"]), None);
        assert_eq!(text("URI::encode", &["a[b]"]), None);
        assert_eq!(text("URI::compare", &["/a", "/b?x"]), None);
        assert_eq!(
            text("IP::addr", &["10.0.0.1/8", "equals", "10.0.0.2/8"]),
            None
        );
        assert_eq!(text("IP::addr", &["10.0.0.01", "equals", "10.0.0.1"]), None);
        assert_eq!(text("IP::addr", &["10.0.0.0", "mask", "255.0.0.0"]), None);
        assert_eq!(
            text("IP::addr", &["10.1.2.3", "equals", "10.0.0.0/8"]).as_deref(),
            Some("1")
        );
        assert_eq!(text("htonl", &["1"]), None);
    }
}
