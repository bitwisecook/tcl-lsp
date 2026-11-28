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

//! Declared implementations: asserted reference bodies and a Wasm extension on the extension host.

use super::*;

/// A pack's `vendor::double`, backed by a Tcl body of `body`'s text that its
/// author asserts may be evaluated, for a workspace the load trusts.
fn reference_pack(command_rows: &str, body: &str) -> tcl_spectcl::pack::PackSet {
    asserting_pack(command_rows, body, " -evaluate")
}

/// The same, with `flag` after the body: the author's assertion, or nothing.
fn asserting_pack(command_rows: &str, body: &str, flag: &str) -> tcl_spectcl::pack::PackSet {
    let source = format!(
        "speclib vendor 2.0 {{\n    command vendor::double {{\n        arity 1\n        \
         {command_rows}\n        runtime_backing tcl-body {{-pack-text {{{body}}}{flag}}}\n    }}\n}}\n"
    );
    tcl_spectcl::pack::load_in_memory(vec![(
        tcl_spectcl::discovery::PackFile {
            tier: tcl_spectcl::discovery::Tier::Workspace,
            path: std::path::PathBuf::from("vendor.tclspec"),
            origin: tcl_spectcl::discovery::Origin::Setting,
            dependency_tier: None,
        },
        source,
    )])
}

/// A command a pack backs with a Tcl body its author asserts may be evaluated
/// (`-evaluate`) is, when the sandbox can express the body, a declared
/// implementation the driver runs as it runs any other: the route is the
/// implementation route, its capability reads one exact operand for each of the
/// body's parameters, and the pack's hook list holds the body the host will run.
/// The same command with a body that reaches for the frame (`upvar`) derives
/// nothing and says why; so does one whose declared arity is not the body's
/// parameters, and one whose author stated its evaluation.
#[test]
fn a_reference_body_is_a_declared_implementation_when_the_sandbox_can_express_it() {
    crate::install_surfaces();
    use tcl_registry::pack_hooks::HookFamily;

    let derived = reference_pack("", "proc vendor::double {x} {expr {$x * 2}}");
    assert!(
        !derived
            .notices
            .iter()
            .any(|notice| notice.message.contains("`-evaluate` asks")),
        "{:?}",
        derived.notices
    );
    let command = &derived.packs[0].commands[0];
    let SemanticsDeclaration::Declared(declared) = command.spec.semantics else {
        panic!(
            "the body derives a declaration: {:?}",
            command.spec.semantics
        );
    };
    let EvalRoute::Implementation(capability) = declared.route() else {
        panic!("the implementation route: {:?}", declared.route());
    };
    assert_eq!(capability.identity.id, "vendor::double.reference");
    assert_eq!(
        capability.inputs,
        &[DeclaredInput::Operand {
            index: 0,
            exactness: Exactness::Exact
        }]
    );
    assert_eq!(capability.host, HostKind::BoundedTcl);
    assert_eq!(capability.completion, CompletionSupport::NormalOnly);
    let [hook] = &command.hooks[..] else {
        panic!("one hook body: {:?}", command.hooks);
    };
    assert_eq!(hook.family, HookFamily::Evaluate);
    let tcl_spectcl::loader::HookSource::Body { params, body, .. } = &hook.source else {
        panic!("a Tcl body: {:?}", hook.source);
    };
    assert_eq!(params, &["x".to_owned()]);
    assert_eq!(body, "fold [\nexpr {$x * 2}\n]");

    // The negative: a body that reaches for the caller's frame derives nothing,
    // the command keeps the declaration it had, and the load says which command
    // the sandbox would not take.
    let upvar = reference_pack("", "proc vendor::double {x} {upvar 1 $x y; expr {$y * 2}}");
    let command = &upvar.packs[0].commands[0];
    assert!(
        matches!(command.spec.semantics, SemanticsDeclaration::Inherited),
        "{:?}",
        command.spec.semantics
    );
    assert!(command.hooks.is_empty(), "{:?}", command.hooks);
    assert!(
        upvar
            .notices
            .iter()
            .any(|notice| notice.message.contains("`upvar`")
                && notice.context == "command vendor::double"),
        "{:?}",
        upvar.notices
    );

    // A body for another arity than the command declares would answer a call the
    // declaration admits without being given its arguments.
    let source = "speclib vendor 2.0 {\n    command vendor::double {\n        arity 1..2\n        \
                  runtime_backing tcl-body {-pack-text {proc vendor::double {x} {expr {$x * 2}}} -evaluate}\n    }\n}\n";
    let wide = tcl_spectcl::pack::load_in_memory(vec![(
        tcl_spectcl::discovery::PackFile {
            tier: tcl_spectcl::discovery::Tier::Workspace,
            path: std::path::PathBuf::from("vendor.tclspec"),
            origin: tcl_spectcl::discovery::Origin::Setting,
            dependency_tier: None,
        },
        source.to_owned(),
    )]);
    assert_eq!(wide.packs[0].commands[0].hooks.len(), 0);
    assert!(
        wide.notices
            .iter()
            .any(|notice| notice.message.contains("another arity")),
        "{:?}",
        wide.notices
    );

    // What the author states, stands: `semantics none` abstains for the command,
    // and no body is derived under it.
    let declined = reference_pack("semantics none", "proc vendor::double {x} {expr {$x * 2}}");
    let command = &declined.packs[0].commands[0];
    assert!(matches!(
        command.spec.semantics,
        SemanticsDeclaration::Declined
    ));
    assert!(command.hooks.is_empty(), "{:?}", command.hooks);
}

/// The negative the derivation turns on: a body its author did not assert is not
/// run, however plainly the sandbox could, and nothing is said of it.
#[test]
fn a_body_nobody_asserted_is_not_derived_from_however_plain_it_is() {
    crate::install_surfaces();
    use tcl_registry::value_transfer::SemanticsDeclaration;

    for body in [
        "proc vendor::double {x} {expr {$x * 2}}",
        "proc vendor::double {x} {upvar 1 $x y; expr {$y * 2}}",
    ] {
        let silent = asserting_pack("", body, "");
        let command = &silent.packs[0].commands[0];
        assert!(
            matches!(command.spec.semantics, SemanticsDeclaration::Inherited),
            "{body}: {:?}",
            command.spec.semantics
        );
        assert!(command.hooks.is_empty(), "{body}: {:?}", command.hooks);
        assert!(
            !silent
                .notices
                .iter()
                .any(|notice| notice.message.contains("`-evaluate` asks")),
            "{body}: {:?}",
            silent.notices
        );
    }
}

/// A host that loads any artefact as one command and answers its words
/// joined, for an artefact it has loaded.
struct Joiner {
    loaded: std::cell::RefCell<std::collections::HashSet<u64>>,
    loads: std::cell::Cell<u32>,
}

impl tcl_registry::extension_host::ExtensionHost for Joiner {
    fn load(
        &self,
        artefact: &[u8],
        prefix: &str,
    ) -> Result<tcl_registry::extension_host::LoadedExtension, DeclineReason> {
        self.loads.set(self.loads.get() + 1);
        let hash = tcl_registry::extension_host::artefact_hash(artefact);
        self.loaded.borrow_mut().insert(hash);
        Ok(tcl_registry::extension_host::LoadedExtension {
            hash,
            prefix: prefix.to_owned(),
            commands: vec!["pkga_join".to_owned()],
        })
    }

    fn evaluate(
        &self,
        hash: u64,
        words: &[String],
        _budget: &ImplementationBudget,
    ) -> Result<String, DeclineReason> {
        if self.loaded.borrow().contains(&hash) {
            Ok(words.join(":"))
        } else {
            Err(DeclineReason::Transient)
        }
    }
}

/// A workspace pack declaring `pkga_join` on the extension host, loaded from a
/// scratch directory beside the artefact `artefact`.
fn extension_pack(artefact: &[u8]) -> tcl_spectcl::PackSet {
    let dir = std::env::temp_dir().join(format!("tcl-lsp-wasm-extension-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    std::fs::write(dir.join("pkga.wasm"), artefact).expect("the artefact");
    let pack = dir.join("pkga.tclspec");
    std::fs::write(
        &pack,
        "speclib pkga 2.2 {\n\
         \x20   command pkga_join {\n\
         \x20       arity 2\n\
         \x20       evaluate -implementation pkga.join.v1 -host wasm_extension {\n\
         \x20           extension pkga.wasm Pkga\n\
         \x20           inputs {arg 0 exact arg 1 exact}\n\
         \x20       }\n\
         \x20   }\n\
         }\n",
    )
    .expect("the pack");
    let packs = tcl_spectcl::pack::load(&[tcl_spectcl::PackFile {
        tier: tcl_spectcl::Tier::Workspace,
        path: pack,
        origin: tcl_spectcl::discovery::Origin::DotDir,
        dependency_tier: None,
    }]);
    std::fs::remove_dir_all(&dir).ok();
    packs
}

/// A `-host wasm_extension` implementation runs its extension's command on
/// the thread's extension host. The pack load reads the artefact beside the
/// pack, and the identity names the pack and carries the artefact's content
/// hash, so the memo key does; the first evaluation loads the artefact on
/// the host, which then runs the command over the declared inputs, and the
/// next reuses the load; a worker with no host installed declines
/// `Transient`, a state of the worker, never a verdict on the inputs.
#[test]
fn a_wasm_extension_implementation_runs_on_the_extension_host() {
    crate::install_surfaces();
    use tcl_registry::extension_host;
    use tcl_registry::value_transfer::{LiteralInputs, evaluate_literal};

    let artefact = b"\0asm\x01\0\0\0pkga";
    let packs = extension_pack(artefact);
    assert!(packs.notices.is_empty(), "{:#?}", packs.notices);
    let command = packs
        .packs
        .iter()
        .flat_map(|pack| &pack.commands)
        .find(|command| command.spec.name == "pkga_join")
        .expect("the command");
    let SemanticsDeclaration::Declared(semantics) = command.spec.semantics else {
        panic!("a declaration: {:?}", command.spec.semantics);
    };
    let EvalRoute::Implementation(capability) = semantics.route() else {
        panic!("an implementation: {:?}", semantics.route());
    };
    assert_eq!(capability.host, HostKind::WasmExtension);
    assert_eq!(capability.identity.pack, "pkga");
    assert_eq!(
        capability.identity.content_hash,
        extension_host::artefact_hash(artefact)
    );

    let release = Some(tcl_dialect::TclVersion::V9_0);
    let run = || evaluate_literal(semantics, "pkga_join", None, &["a", "b"], release);
    extension_host::clear_extension_host();
    let profile = tcl_dialect::DialectProfile::find("tcl9.0");
    let unhosted = semantics.evaluate(
        &LiteralInputs::new("pkga_join", None, &["a", "b"], profile),
        &mut Budget::evaluation(),
    );
    assert!(
        matches!(unhosted, EvalAnswer::Declined(DeclineReason::Transient)),
        "{unhosted:?}"
    );
    assert_eq!(run(), None);

    let host = std::rc::Rc::new(Joiner {
        loaded: std::cell::RefCell::default(),
        loads: std::cell::Cell::new(0),
    });
    extension_host::install_extension_host(host.clone());
    assert_eq!(run().as_deref(), Some("pkga_join:a:b"));
    assert_eq!(run().as_deref(), Some("pkga_join:a:b"));
    assert_eq!(host.loads.get(), 1, "the artefact loads once on the host");
    extension_host::clear_extension_host();
}
