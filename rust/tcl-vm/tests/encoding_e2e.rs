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

//! `encoding convertfrom` / `encoding convertto` run end-to-end through the
//! VM and compared, release by release, with the reference `tclsh`.

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_dialect::TclVersion;

#[derive(Clone, Default)]
struct Capture(Rc<RefCell<Vec<u8>>>);
impl Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// What `src` prints when the VM runs it as `version`.
fn vm_output(src: &str, version: TclVersion) -> String {
    let profile = tcl_registry::model::ingress::resolve_environment(version.dialect_name())
        .analyser_profile();
    let capture = Capture::default();
    let mut vm = tcl_vm::test_support::native_vm_for_profile(profile, Box::new(capture.clone()));
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
    let completion = vm.try_eval_source(src).expect("original source provider");
    assert!(
        completion.code.is_ok(),
        "Tcl {}: {}",
        version.version_string(),
        completion.result.to_str()
    );
    String::from_utf8_lossy(&capture.0.borrow()).into_owned()
}

/// Every line is ASCII: a string is shown as its length and its UTF-8 bytes,
/// so neither the locale nor the release's `string index` model is involved.
/// Tcl 8.4 has no `binary encode`, hence `binary scan`.
///
/// Each conversion is caught in place rather than run through an `uplevel`
/// helper, and the euro sign comes from `format` rather than a `\u` escape:
/// under an ISO 8859-1 locale tclsh 8.4 otherwise reads the euro sign back as
/// its three UTF-8 bytes once an earlier `encoding` call ran that way.
const WITNESS: &str = r#"
set euro [format %c 8364]
proc hex {s} { binary scan $s H* h; return $h }
proc show {label failed r} {
    if {$failed} {
        puts "$label error: $r"
    } else {
        puts "$label [string length $r] [hex [encoding convertto utf-8 $r]]"
    }
}
proc bytes {label failed r} {
    if {$failed} {
        puts "$label error: $r"
    } else {
        puts "$label [hex $r]"
    }
}
show euro [catch {encoding convertfrom utf-8 [binary format H* e282ac]} r] $r
show clef [catch {encoding convertfrom utf-8 "\xf0\x9d\x84\x9e"} r] $r
show cafe [catch {encoding convertfrom utf-8 [binary format H* 636166c3a9]} r] $r
show nul [catch {encoding convertfrom utf-8 [binary format H* 00]} r] $r
show latin1 [catch {encoding convertfrom iso8859-1 [binary format H* 41e9ff]} r] $r
show ascii [catch {encoding convertfrom ascii [binary format H* 417f]} r] $r
show unicode [catch {encoding convertfrom unicode [encoding convertto unicode "A$euro"]} r] $r
show stray [catch {encoding convertfrom utf-8 [binary format H* ff41]} r] $r
show truncated [catch {encoding convertfrom utf-8 [binary format H* 41e282]} r] $r
show modified-nul [catch {encoding convertfrom utf-8 [binary format H* c080]} r] $r
show overlong [catch {encoding convertfrom utf-8 [binary format H* c1bf]} r] $r
show overlong3 [catch {encoding convertfrom utf-8 [binary format H* e08080]} r] $r
show cut-clef [catch {encoding convertfrom utf-8 [binary format H* 41f09d8441]} r] $r
show beyond [catch {encoding convertfrom utf-8 [binary format H* f4908080]} r] $r
show unknown [catch {encoding convertfrom bogus x} r] $r
show arity [catch {encoding convertfrom} r] $r
bytes to-utf8 [catch {encoding convertto utf-8 "caf\xe9 $euro"} r] $r
bytes to-nul [catch {encoding convertto utf-8 "\0"} r] $r
bytes to-unicode [catch {encoding convertto unicode "A$euro"} r] $r
bytes to-latin1 [catch {encoding convertto iso8859-1 "${euro}A"} r] $r
bytes to-ascii [catch {encoding convertto ascii "AB\x7e\x7f"} r] $r
bytes to-unknown [catch {encoding convertto bogus x} r] $r
if {[package vsatisfies [info tclversion] 9]} {
    show tcl8-stray [catch {encoding convertfrom -profile tcl8 utf-8 [binary format H* 41f09d8441]} r] $r
    show replace-stray [catch {encoding convertfrom -profile replace utf-8 [binary format H* 41f09d8441]} r] $r
    show replace-nul [catch {encoding convertfrom -profile replace utf-8 [binary format H* c080]} r] $r
    show tcl8-ascii [catch {encoding convertfrom -prof tcl8 ascii [binary format H* 4180e9]} r] $r
    show cesu [catch {encoding convertfrom utf-8 [binary format H* eda0bdedb880]} r] $r
    show replace-odd [catch {encoding convertfrom -profile replace unicode [binary format H* 410042]} r] $r
    catch {encoding convertfrom utf-8 [binary format H* 41ff]} r
    puts "strict-code $::errorCode"
    show failindex [catch {encoding convertfrom -failindex at utf-8 [binary format H* 4142ff43]} r] $r
    puts "failindex-at $at"
    show clean [catch {encoding convertfrom -failindex at utf-8 [binary format H* 41]} r] $r
    puts "clean-at $at"
    show wide-data [catch {encoding convertfrom utf-8 "A$euro"} r] $r
    puts "wide-data-code $::errorCode"
    show bad-profile [catch {encoding convertfrom -profile bogus utf-8 x} r] $r
    show bad-option [catch {encoding convertfrom -bogus x utf-8 x} r] $r
    bytes to-strict [catch {encoding convertto iso8859-1 "\0\xe9$euro"} r] $r
    puts "to-strict-code $::errorCode"
    bytes to-failindex [catch {encoding convertto -failindex at ascii "\0\0\xe9"} r] $r
    puts "to-failindex-at $at"
    bytes to-replace [catch {encoding convertto -profile replace ascii "A\xe9"} r] $r
}
"#;

/// The reported decodes, pinned per release so the VM is checked even where
/// no reference interpreter is installed (tclsh 8.4.20, 8.6.18, 9.0.4,
/// 9.1.0; 8.5 shares 8.4's three-byte reader).
fn pinned(version: TclVersion) -> [&'static str; 2] {
    let clef = match version {
        TclVersion::V8_4 | TclVersion::V8_5 => "clef 4 c3b0c29dc284c29e",
        TclVersion::V8_6 => "clef 2 f09d849e",
        TclVersion::V9_0 | TclVersion::V9_1 => "clef 1 f09d849e",
    };
    ["euro 1 e282ac", clef]
}

#[test]
fn encoding_conversions_match_tclsh() {
    for version in TclVersion::ALL {
        let release = version.version_string();
        let vm = vm_output(WITNESS, version);
        for line in pinned(version) {
            assert!(
                vm.lines().any(|printed| printed == line),
                "Tcl {release}: missing `{line}` in\n{vm}"
            );
        }
        let Some(tclsh) = tcl_test_support::witness_tclsh(version) else {
            continue;
        };
        let outcome = tcl_test_support::run_script(&tclsh.path, WITNESS.as_bytes())
            .unwrap_or_else(|error| panic!("tclsh{release}: {error}"));
        assert!(
            outcome.success(),
            "tclsh{release}: {}",
            String::from_utf8_lossy(&outcome.stderr)
        );
        let expected = String::from_utf8_lossy(&outcome.stdout);
        assert_eq!(vm, expected, "Tcl {release}");
    }
}
