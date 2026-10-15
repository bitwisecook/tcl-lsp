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

//! Authentic pinned Jim original-object lookup policy.

/// Selected pinned namespace procedure family; no callable or entry authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeJimNamespaceHelperKind {
    /// Original `namespace which` distribution procedure.
    Which,
    /// Original `namespace code` distribution procedure.
    Code,
    /// Original `namespace origin` distribution procedure.
    Origin,
    /// Original `namespace inscope` distribution procedure.
    Inscope,
    /// Original namespace deletion distribution procedure.
    Delete,
    /// Original namespace variable-link distribution procedure.
    Upvar,
    /// Original private namespace-aware inventory distribution procedure.
    Info,
}
impl NativeJimNamespaceHelperKind {
    /// Pinned source families this owner currently supplies.
    pub const ALL: [Self; 7] = [
        Self::Which,
        Self::Code,
        Self::Origin,
        Self::Inscope,
        Self::Info,
        Self::Delete,
        Self::Upvar,
    ];
}

/// Authored pinned distribution procedure source, not a published callable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeJimNamespaceHelper {
    kind: NativeJimNamespaceHelperKind,
    public_subcommand: bool,
    minimum_arguments: usize,
    maximum_arguments: usize,
    usage: &'static str,
    name: &'static str,
    parameters: &'static str,
    body: &'static str,
}
impl NativeJimNamespaceHelper {
    /// Original selected source family, independent of physical helper publication.
    #[must_use]
    pub const fn kind(self) -> NativeJimNamespaceHelperKind {
        self.kind
    }
    /// `Jim_ParseSubCmd` validates the public operands before helper-head lookup.
    #[must_use]
    pub const fn accepts_public_arguments(self, count: usize) -> bool {
        self.public_subcommand && count >= self.minimum_arguments && count <= self.maximum_arguments
    }
    /// Exact public-subcommand argument synopsis.
    #[must_use]
    pub const fn usage(self) -> &'static str {
        self.usage
    }
    /// Original compound registration spelling and fresh public fallback head.
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.name
    }
    /// Actual original procedure formal syntax.
    #[must_use]
    pub const fn parameters(self) -> &'static str {
        self.parameters
    }
    /// Original helper body; actual procedure/frame publication is backend-owned.
    #[must_use]
    pub const fn body(self) -> &'static str {
        self.body
    }
}

impl crate::InvocationDialect {
    /// Select one pinned namespace distribution procedure source.
    /// This descriptor grants no installed slot, header or Native compiler entry.
    #[must_use]
    pub fn native_jim_namespace_helper(
        self,
        kind: NativeJimNamespaceHelperKind,
    ) -> Option<NativeJimNamespaceHelper> {
        self.native_jim_lookup_protocol()?;
        Some(match kind {
            NativeJimNamespaceHelperKind::Which => NativeJimNamespaceHelper {
                kind,
                public_subcommand: true,
                minimum_arguments: 1,
                maximum_arguments: 2,
                usage: "?-command|-variable? name",
                name: "namespace which",
                parameters: "{type -command} name",
                body: r#"
	set nsname ::[uplevel 1 [list ::namespace canon $name]]
	if {$type eq "-variable"} {
		return $nsname
	}
	if {$type eq "-command"} {
		if {[exists -command $nsname]} {
			return $nsname
		} elseif {[exists -command ::$name]} {
			return ::$name
		}
		return ""
	}
	return -code error {wrong # args: should be "namespace which ?-command? ?-variable? name"}
"#,
            },
            NativeJimNamespaceHelperKind::Code => NativeJimNamespaceHelper {
                kind,
                public_subcommand: true,
                minimum_arguments: 1,
                maximum_arguments: 1,
                usage: "arg",
                name: "namespace code",
                parameters: "arg",
                body: r#"
	if {[string first "::namespace inscope " $arg] == 0} {
		# Already scoped
		return $arg
	}
	list ::namespace inscope [uplevel 1 ::namespace current] $arg
"#,
            },
            NativeJimNamespaceHelperKind::Origin => NativeJimNamespaceHelper {
                kind,
                public_subcommand: true,
                minimum_arguments: 1,
                maximum_arguments: 1,
                usage: "name",
                name: "namespace origin",
                parameters: "name",
                body: r#"
	set nscanon [uplevel 1 [list ::namespace canon $name]]
	if {[exists -alias $nscanon]} {
		tailcall {namespace origin} [info alias $nscanon]
	}
	if {[exists -command $nscanon]} {
		return ::$nscanon
	}
	if {[exists -command $name]} {
		return ::$name
	}

	return -code error "invalid command name \"$name\""
"#,
            },
            NativeJimNamespaceHelperKind::Inscope => NativeJimNamespaceHelper {
                kind,
                public_subcommand: true,
                minimum_arguments: 2,
                maximum_arguments: usize::MAX,
                usage: "name arg ?arg ...?",
                name: "namespace inscope",
                parameters: "name arg args",
                body: "\n\ttailcall namespace eval $name $arg $args\n",
            },
            NativeJimNamespaceHelperKind::Delete => NativeJimNamespaceHelper {
                kind,
                public_subcommand: true,
                minimum_arguments: 0,
                maximum_arguments: usize::MAX,
                usage: "?name ...?",
                name: "namespace delete",
                parameters: "args",
                body: r#"
	foreach name $args {
		if {$name ni {:: ""}} {
			set name [uplevel 1 [list ::namespace canon $name]]
			foreach i [info commands ${name}::*] { rename $i "" }
			uplevel #0 [list unset {*}[info globals ${name}::*]]
		}
	}
"#,
            },
            NativeJimNamespaceHelperKind::Upvar => NativeJimNamespaceHelper {
                kind,
                public_subcommand: true,
                minimum_arguments: 1,
                maximum_arguments: usize::MAX,
                usage: "ns ?arg ...?",
                name: "namespace upvar",
                parameters: "ns args",
                body: r#"
	set nscanon ::[uplevel 1 [list ::namespace canon $ns]]
	set script [list upvar 0]
	foreach {other local} $args {
		lappend script ${nscanon}::$other $local
	}
	tailcall {*}$script
"#,
            },
            NativeJimNamespaceHelperKind::Info => NativeJimNamespaceHelper {
                kind,
                public_subcommand: false,
                minimum_arguments: 0,
                maximum_arguments: 0,
                usage: "cmd ?pattern?",
                name: "namespace info",
                parameters: "cmd {pattern *}",
                body: r#"
	set current [uplevel 1 ::namespace canon]
	# Now we may need to strip $pattern
	if {[string first :: $pattern] == 0} {
		set global 1
		set prefix ::
	} else {
		set global 0
		set clen [string length $current]
		incr clen 2
	}
	set fqp [namespace canon $current $pattern]
	switch -glob -- $cmd {
		co* - p* {
			if {$global} {
				set result [info $cmd $fqp]
			} else {
				# Add commands in the current namespace
				set r {}
				foreach c [info $cmd $fqp] {
					dict set r [string range $c $clen end] 1
				}
				if {[string match co* $cmd]} {
					# Now in the global namespace
					foreach c [info -nons commands $pattern] {
						dict set r $c 1
					}
				}
				set result [dict keys $r]
			}
		}
		ch* {
			set result [info channels $pattern]
		}
		v* {
			#puts "uplevel #0 info gvars $fqp"
			set result [uplevel #0 info -nons vars $fqp]
		}
		g* {
			set result [info globals $fqp]
		}
		l* {
			set result [uplevel 1 info -nons locals $pattern]
		}
	}
	if {$global} {
		set result [lmap p $result { string cat $prefix $p }]
	}
	return $result
"#,
            },
        })
    }

    /// Actual pinned Jim lookup-cache policy; compatibility versions and C
    /// command-name caches cannot supply this original-object protocol.
    #[must_use]
    pub fn native_jim_lookup_protocol(
        self,
    ) -> Option<tcl_syntax::native_jim_lookup::NativeJimLookupProtocol> {
        (self.native_name_protocol()? == tcl_syntax::naming::NativeNameProtocol::Jim084)
            .then_some(tcl_syntax::native_jim_lookup::NativeJimLookupProtocol::jim084())
    }
}

/// Original object operations for Jim's global namespace-origin helper.
/// Canonicalisation retains the caller namespace; all getters use the actual
/// root namespace. Implementations retain their own provider and currency gates.
pub trait NativeJimNamespaceOriginBackend {
    /// Retained original header, independently of its reporting bytes.
    type Original: Clone;
    /// The adapter's original failure and host-refusal transport.
    type Error;
    /// Construct or retain the canonical operand in the actual caller namespace.
    fn canonical(&mut self, original: &Self::Original) -> Result<Self::Original, Self::Error>;
    /// Query the root alias flag and return the same original prefix object.
    fn alias_prefix(
        &mut self,
        canonical: &Self::Original,
    ) -> Result<Option<Self::Original>, Self::Error>;
    /// Query the original operand through the root command owner.
    fn command_exists(&mut self, original: &Self::Original) -> Result<bool, Self::Error>;
    /// Report the winning operand with Jim's counted root prefix.
    fn report(&mut self, original: &Self::Original) -> Result<Vec<u8>, Self::Error>;
    /// Compare actual header identity without comparing name strings.
    fn same_original(&self, left: &Self::Original, right: &Self::Original) -> bool;
    /// Refuse an unbounded alias cycle without supplying a guest result.
    fn alias_cycle_unavailable(&mut self) -> Self::Error;
}

/// A missing origin retains the actual recursive operand for its guest error.
pub enum NativeJimNamespaceOriginOutcome<V> {
    /// Counted report of the winning original operand.
    Found(Vec<u8>),
    /// Actual recursive operand used to report a missing command.
    Missing(V),
}

/// Execute the pinned helper's alias/canonical/original query order through
/// existing original-object getters. An alias prefix remains ONE name object.
///
/// # Errors
/// Preserves adapter refusals; a repeated actual prefix header refuses rather
/// than inventing a guest result for an unbounded original alias cycle.
pub fn native_jim_namespace_origin<B: NativeJimNamespaceOriginBackend>(
    backend: &mut B,
    original: &B::Original,
) -> Result<NativeJimNamespaceOriginOutcome<B::Original>, B::Error> {
    let mut original = original.clone();
    let mut prefixes = Vec::new();
    loop {
        let canonical = backend.canonical(&original)?;
        if let Some(prefix) = backend.alias_prefix(&canonical)? {
            if prefixes
                .iter()
                .any(|seen| backend.same_original(seen, &prefix))
            {
                return Err(backend.alias_cycle_unavailable());
            }
            prefixes.push(prefix.clone());
            original = prefix;
            continue;
        }
        if backend.command_exists(&canonical)? {
            return backend
                .report(&canonical)
                .map(NativeJimNamespaceOriginOutcome::Found);
        }
        if backend.command_exists(&original)? {
            return backend
                .report(&original)
                .map(NativeJimNamespaceOriginOutcome::Found);
        }
        return Ok(NativeJimNamespaceOriginOutcome::Missing(original));
    }
}

#[cfg(test)]
mod namespace_helper_source_tests {
    #[test]
    fn which_helper_descriptor_keeps_the_original_pinned_declaration() {
        let environment = crate::model::ingress::resolve_known_environment("jim").unwrap();
        let helper = crate::InvocationDialect::of_profile(environment.unit_profile())
            .native_jim_namespace_helper(super::NativeJimNamespaceHelperKind::Which)
            .unwrap();
        let source = include_str!("../tests/data/native_jim_namespace_which/nshelper.tcl");
        let declaration = format!(
            "proc {{{}}} {{{}}} {{{}}}",
            helper.name(),
            helper.parameters(),
            helper.body()
        );
        assert!(source.contains(&declaration));
        for version in tcl_dialect::TclVersion::ALL {
            let environment =
                crate::model::ingress::resolve_known_environment(version.dialect_name()).unwrap();
            assert!(
                crate::InvocationDialect::of_profile(environment.unit_profile())
                    .native_jim_namespace_helper(super::NativeJimNamespaceHelperKind::Which)
                    .is_none()
            );
        }
    }
    #[test]
    fn code_helper_descriptor_keeps_original_source_and_public_arity() {
        let environment = crate::model::ingress::resolve_known_environment("jim").unwrap();
        let dialect = crate::InvocationDialect::of_profile(environment.unit_profile());
        let helper = dialect
            .native_jim_namespace_helper(super::NativeJimNamespaceHelperKind::Code)
            .unwrap();
        let source = include_str!("../tests/data/native_jim_namespace_which/nshelper.tcl");
        assert!(source.contains(&format!(
            "proc {{{}}} {{{}}} {{{}}}",
            helper.name(),
            helper.parameters(),
            helper.body()
        )));
        assert!(!helper.accepts_public_arguments(0));
        assert!(helper.accepts_public_arguments(1));
        assert!(!helper.accepts_public_arguments(2));
        assert_eq!(helper.usage(), "arg");
        for version in tcl_dialect::TclVersion::ALL {
            let environment =
                crate::model::ingress::resolve_known_environment(version.dialect_name()).unwrap();
            assert!(
                crate::InvocationDialect::of_profile(environment.unit_profile())
                    .native_jim_namespace_helper(super::NativeJimNamespaceHelperKind::Code)
                    .is_none()
            );
        }
    }
    #[test]
    fn origin_helper_descriptor_keeps_original_source_and_public_arity() {
        let environment = crate::model::ingress::resolve_known_environment("jim").unwrap();
        let dialect = crate::InvocationDialect::of_profile(environment.unit_profile());
        let helper = dialect
            .native_jim_namespace_helper(super::NativeJimNamespaceHelperKind::Origin)
            .unwrap();
        let source = include_str!("../tests/data/native_jim_namespace_which/nshelper.tcl");
        assert!(source.contains(&format!(
            "proc {{{}}} {{{}}} {{{}}}",
            helper.name(),
            helper.parameters(),
            helper.body()
        )));
        assert!(!helper.accepts_public_arguments(0));
        assert!(helper.accepts_public_arguments(1));
        assert!(!helper.accepts_public_arguments(2));
        assert_eq!(helper.usage(), "name");
        for version in tcl_dialect::TclVersion::ALL {
            let environment =
                crate::model::ingress::resolve_known_environment(version.dialect_name()).unwrap();
            assert!(
                crate::InvocationDialect::of_profile(environment.unit_profile())
                    .native_jim_namespace_helper(super::NativeJimNamespaceHelperKind::Origin)
                    .is_none()
            );
        }
    }
    #[test]
    fn inscope_helper_descriptor_keeps_original_source_and_public_arity() {
        let environment = crate::model::ingress::resolve_known_environment("jim").unwrap();
        let dialect = crate::InvocationDialect::of_profile(environment.unit_profile());
        let helper = dialect
            .native_jim_namespace_helper(super::NativeJimNamespaceHelperKind::Inscope)
            .unwrap();
        let source = include_str!("../tests/data/native_jim_namespace_which/nshelper.tcl");
        assert!(source.contains(&format!(
            "proc {{{}}} {{{}}} {{{}}}",
            helper.name(),
            helper.parameters(),
            helper.body()
        )));
        assert!(!helper.accepts_public_arguments(0));
        assert!(!helper.accepts_public_arguments(1));
        assert!(helper.accepts_public_arguments(2));
        assert!(helper.accepts_public_arguments(3));
        assert!(helper.accepts_public_arguments(usize::MAX));
        assert_eq!(helper.usage(), "name arg ?arg ...?");
        for version in tcl_dialect::TclVersion::ALL {
            let environment =
                crate::model::ingress::resolve_known_environment(version.dialect_name()).unwrap();
            assert!(
                crate::InvocationDialect::of_profile(environment.unit_profile())
                    .native_jim_namespace_helper(super::NativeJimNamespaceHelperKind::Inscope)
                    .is_none()
            );
        }
    }
    #[test]
    fn info_helper_descriptor_keeps_original_private_procedure_without_public_dispatch() {
        let environment = crate::model::ingress::resolve_known_environment("jim").unwrap();
        let dialect = crate::InvocationDialect::of_profile(environment.unit_profile());
        let helper = dialect
            .native_jim_namespace_helper(super::NativeJimNamespaceHelperKind::Info)
            .unwrap();
        let source = include_str!("../tests/data/native_jim_namespace_which/nshelper.tcl");
        assert!(source.contains(&format!(
            "proc {{{}}} {{{}}} {{{}}}",
            helper.name(),
            helper.parameters(),
            helper.body()
        )));
        for count in [0, 1, 2, 3, usize::MAX] {
            assert!(!helper.accepts_public_arguments(count));
        }
        assert!(super::NativeJimNamespaceHelperKind::ALL.contains(&helper.kind()));
        for version in tcl_dialect::TclVersion::ALL {
            let environment =
                crate::model::ingress::resolve_known_environment(version.dialect_name()).unwrap();
            assert!(
                crate::InvocationDialect::of_profile(environment.unit_profile())
                    .native_jim_namespace_helper(super::NativeJimNamespaceHelperKind::Info)
                    .is_none()
            );
        }
    }
    #[test]
    fn delete_helper_descriptor_keeps_original_variadic_procedure_and_public_arity() {
        let environment = crate::model::ingress::resolve_known_environment("jim").unwrap();
        let dialect = crate::InvocationDialect::of_profile(environment.unit_profile());
        let helper = dialect
            .native_jim_namespace_helper(super::NativeJimNamespaceHelperKind::Delete)
            .unwrap();
        let source = include_str!("../tests/data/native_jim_namespace_which/nshelper.tcl");
        assert!(source.contains(&format!(
            "proc {{{}}} {{{}}} {{{}}}",
            helper.name(),
            helper.parameters(),
            helper.body()
        )));
        assert_eq!(helper.usage(), "?name ...?");
        for count in [0, 1, 2, usize::MAX] {
            assert!(helper.accepts_public_arguments(count));
        }
        assert!(super::NativeJimNamespaceHelperKind::ALL.contains(&helper.kind()));
        for version in tcl_dialect::TclVersion::ALL {
            let environment =
                crate::model::ingress::resolve_known_environment(version.dialect_name()).unwrap();
            assert!(
                crate::InvocationDialect::of_profile(environment.unit_profile())
                    .native_jim_namespace_helper(super::NativeJimNamespaceHelperKind::Delete)
                    .is_none()
            );
        }
    }
    #[test]
    fn upvar_helper_descriptor_keeps_original_variadic_procedure_and_public_arity() {
        let environment = crate::model::ingress::resolve_known_environment("jim").unwrap();
        let dialect = crate::InvocationDialect::of_profile(environment.unit_profile());
        let helper = dialect
            .native_jim_namespace_helper(super::NativeJimNamespaceHelperKind::Upvar)
            .unwrap();
        let source = include_str!("../tests/data/native_jim_namespace_which/nshelper.tcl");
        assert!(source.contains(&format!(
            "proc {{{}}} {{{}}} {{{}}}",
            helper.name(),
            helper.parameters(),
            helper.body()
        )));
        assert!(!helper.accepts_public_arguments(0));
        for count in [1, 2, 3, usize::MAX] {
            assert!(helper.accepts_public_arguments(count));
        }
        assert_eq!(helper.usage(), "ns ?arg ...?");
        assert!(super::NativeJimNamespaceHelperKind::ALL.contains(&helper.kind()));
        for version in tcl_dialect::TclVersion::ALL {
            let environment =
                crate::model::ingress::resolve_known_environment(version.dialect_name()).unwrap();
            assert!(
                crate::InvocationDialect::of_profile(environment.unit_profile())
                    .native_jim_namespace_helper(super::NativeJimNamespaceHelperKind::Upvar)
                    .is_none()
            );
        }
    }
}
