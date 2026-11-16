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

//! Which specs carry which route: the sweep against the pinned set, route labels and the capability in a route identity.

use super::*;

pub(super) fn route_stamps(
    reg: &CommandRegistry,
) -> BTreeSet<(String, &'static str, &'static str)> {
    let mut stamps: BTreeSet<(String, &'static str, &'static str)> = BTreeSet::new();
    for name in reg.command_names() {
        for spec in reg.specs(name) {
            if let Some(route) = resolve_semantics(spec, None, None).route() {
                stamps.insert((spec.name.to_owned(), route_label(route), route_owner(route)));
            }
            for sub in spec.subcommands {
                if let SemanticsDeclaration::Declared(semantics) = sub.semantics {
                    stamps.insert((
                        format!("{} {}", spec.name, sub.name),
                        route_label(semantics.route()),
                        route_owner(semantics.route()),
                    ));
                }
            }
        }
    }
    stamps
}

/// The pinned-set gate: the specs carrying each route, over every loadable
/// dialect and the shipped packs. A route cannot appear, vanish, or move
/// without this list changing beside it.
#[test]
fn route_stamps_match_the_pinned_set() {
    let actual = route_stamps(&full_registry());
    let expected = pinned_route_stamps();
    let missing: Vec<_> = expected.difference(&actual).collect();
    let extra: Vec<_> = actual.difference(&expected).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "route stamps drifted from the pinned set\nmissing: {missing:?}\nextra: {extra:?}"
    );
}

/// Who implements a direct route, for the pinned set; `-` for any other
/// family.
fn route_owner(route: EvalRoute) -> &'static str {
    match route {
        EvalRoute::Direct { id } => match id.owner() {
            EvaluatorOwner::Registry => "registry",
        },
        EvalRoute::Expression { .. } | EvalRoute::Implementation(_) | EvalRoute::None { .. } => "-",
    }
}

fn route_label(route: EvalRoute) -> &'static str {
    match route {
        EvalRoute::Direct { id } => match id {
            NativeEvalId::CellIncrement => "direct:cell-increment",
            NativeEvalId::CellAppend => "direct:cell-append",
            NativeEvalId::CellListAppend => "direct:cell-list-append",
            NativeEvalId::CellWrite => "direct:cell-write",
            NativeEvalId::ConstWrite => "direct:const-write",
            NativeEvalId::DictSet => "direct:dict-set",
            NativeEvalId::DictUnset => "direct:dict-unset",
            NativeEvalId::DictIncr => "direct:dict-incr",
            NativeEvalId::DictAppend => "direct:dict-append",
            NativeEvalId::DictListAppend => "direct:dict-lappend",
            NativeEvalId::StringRange => "direct:string-range",
            NativeEvalId::ListOfArgs => "direct:list-of-args",
            NativeEvalId::FormatTemplate => "direct:format-template",
            NativeEvalId::ListLength => "direct:list-length",
            NativeEvalId::StringLength => "direct:string-length",
            NativeEvalId::RegexpMatch => "direct:regexp-match",
            NativeEvalId::RegsubSubstitute => "direct:regsub-substitute",
            NativeEvalId::ScanFormat => "direct:scan-format",
            NativeEvalId::BinaryScan => "direct:binary-scan",
            NativeEvalId::ListAssign => "direct:list-assign",
            NativeEvalId::ArraySet => "direct:array-set",
            NativeEvalId::BinaryFormat => "direct:binary-format",
            NativeEvalId::VariableUnset => "direct:variable-unset",
            NativeEvalId::ErrorRaise => "direct:error-raise",
            NativeEvalId::ReturnComplete => "direct:return-complete",
            NativeEvalId::BreakComplete => "direct:break-complete",
            NativeEvalId::ContinueComplete => "direct:continue-complete",
            NativeEvalId::CatchProtected => "direct:catch-protected",
            NativeEvalId::ParameterDefault => "direct:parameter-default",
            NativeEvalId::ListSet => "direct:list-set",
            NativeEvalId::ListEdit => "direct:list-edit",
            NativeEvalId::ListPop => "direct:list-pop",
            NativeEvalId::Base64Encode => "direct:base64-encode",
            NativeEvalId::Base64Decode => "direct:base64-decode",
            NativeEvalId::Crc32Checksum => "direct:crc32-checksum",
            NativeEvalId::Md5Digest => "direct:md5-digest",
            NativeEvalId::Sha1Digest => "direct:sha1-digest",
            NativeEvalId::Sha256Digest => "direct:sha256-digest",
            NativeEvalId::Sha384Digest => "direct:sha384-digest",
            NativeEvalId::Sha512Digest => "direct:sha512-digest",
            NativeEvalId::FindString => "direct:find-string",
            NativeEvalId::StringField => "direct:string-field",
            NativeEvalId::Substring => "direct:substring",
            NativeEvalId::DomainLabels => "direct:domain-labels",
            NativeEvalId::UriBasename => "direct:uri-basename",
            NativeEvalId::UriPath => "direct:uri-path",
            NativeEvalId::UriQuery => "direct:uri-query",
            NativeEvalId::UriHost => "direct:uri-host",
            NativeEvalId::UriPort => "direct:uri-port",
            NativeEvalId::UriProtocol => "direct:uri-protocol",
            NativeEvalId::UriDecode => "direct:uri-decode",
            NativeEvalId::UriEncode => "direct:uri-encode",
            NativeEvalId::UriCompare => "direct:uri-compare",
            NativeEvalId::IpAddrEquals => "direct:ip-addr-equals",
            NativeEvalId::PathJoin => "direct:path-join",
            NativeEvalId::PathDirname => "direct:path-dirname",
            NativeEvalId::PathTail => "direct:path-tail",
            NativeEvalId::PathExtension => "direct:path-extension",
            NativeEvalId::PathRootname => "direct:path-rootname",
            NativeEvalId::PathSplit => "direct:path-split",
            NativeEvalId::ListSplit => "direct:list-split",
            NativeEvalId::StringFirst => "direct:string-first",
            NativeEvalId::StringMatch => "direct:string-match",
            NativeEvalId::Base32Encode => "direct:base32-encode",
            NativeEvalId::Base32Decode => "direct:base32-decode",
            NativeEvalId::Base32HexEncode => "direct:base32-hex-encode",
            NativeEvalId::Base32HexDecode => "direct:base32-hex-decode",
        },
        EvalRoute::Expression { .. } => "expression:tcl.expr",
        EvalRoute::Implementation(_) => "implementation",
        EvalRoute::None { reason } => match reason {
            NoRouteReason::Unauthored => "none:unauthored",
            NoRouteReason::Declared => "none:declared",
            NoRouteReason::FormUnsupported => "none:form-unsupported",
            NoRouteReason::Callback => "none:callback",
            NoRouteReason::Platform => "none:platform",
        },
    }
}

/// The capability is part of the route (`value-evaluation.md` § *The
/// capability declaration*): two declared implementations that differ in any
/// one field — the pack, the id, the body's content hash, the target axes,
/// an input, a dependency, the budget — are two routes, so nothing keyed by
/// the route can serve one's answer for the other. The same declaration
/// twice is one route.
#[test]
fn the_capability_is_part_of_the_route_identity() {
    static INPUTS: [DeclaredInput; 1] = [DeclaredInput::Operand {
        index: 0,
        exactness: Exactness::Exact,
    }];
    static OTHER_INPUTS: [DeclaredInput; 1] = [DeclaredInput::IncomingTarget { index: 0 }];
    static DEPENDS: [ContextDependency; 2] = [
        ContextDependency::TclProfile,
        ContextDependency::ImplementationIdentity,
    ];
    static FEWER_DEPENDS: [ContextDependency; 1] = [ContextDependency::TclProfile];
    let base = EvaluatorCapability {
        identity: ImplementationIdentity {
            pack: "tenant",
            id: "tenant.label.v1",
            content_hash: 1,
        },
        host: HostKind::BoundedTcl,
        target: Needs::NONE,
        inputs: &INPUTS,
        depends: &DEPENDS,
        budget: ImplementationBudget {
            commands: Some(2000),
            wall_clock_ms: Some(20),
            value_bytes: Some(65536),
        },
        completion: CompletionSupport::NormalOnly,
    };
    let variants = [
        EvaluatorCapability {
            identity: ImplementationIdentity {
                pack: "other",
                ..base.identity
            },
            ..base
        },
        EvaluatorCapability {
            identity: ImplementationIdentity {
                id: "tenant.label.v2",
                ..base.identity
            },
            ..base
        },
        EvaluatorCapability {
            identity: ImplementationIdentity {
                content_hash: 2,
                ..base.identity
            },
            ..base
        },
        EvaluatorCapability {
            target: Needs::NUMERAL_GRAMMAR,
            ..base
        },
        EvaluatorCapability {
            inputs: &OTHER_INPUTS,
            ..base
        },
        EvaluatorCapability {
            depends: &FEWER_DEPENDS,
            ..base
        },
        EvaluatorCapability {
            budget: ImplementationBudget {
                commands: Some(1000),
                ..base.budget
            },
            ..base
        },
    ];
    let route = EvalRoute::Implementation(base);
    assert_eq!(route, EvalRoute::Implementation(base));
    assert_eq!(route.family(), "implementation");
    assert!(route.is_enabled());
    let mut routes = BTreeSet::new();
    routes.insert(format!("{route:?}"));
    let mut hashed = std::collections::HashSet::from([route]);
    for variant in variants {
        let other = EvalRoute::Implementation(variant);
        assert_ne!(other, route, "{variant:?}");
        assert!(hashed.insert(other), "{variant:?}");
        assert!(routes.insert(format!("{other:?}")), "{variant:?}");
    }
    assert_eq!(hashed.len(), variants.len() + 1);
}
