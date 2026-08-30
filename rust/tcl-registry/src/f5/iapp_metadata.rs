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

//! F7: iApp target and execution policy are **action-local data**, not
//! properties of one global `f5-iapps` environment.
//!
//! The `sys application template` schema carries three facts the single
//! environment cannot preserve, and the registry's own object schema
//! already lists all three (`crate::bigip::data::sys`):
//!
//! - `requires-bigip-version-min` / `requires-bigip-version-max` — the
//!   template's own BIG-IP compatibility interval, which is *positive
//!   source evidence already present in the artefact being analysed*. A
//!   workspace-wide BIG-IP default must not override a narrower template
//!   range.
//! - `role-acl` — which roles may run each action.
//! - `run-as` — the account the implementation runs as; **omitted means
//!   the calling user**, which is an unknown principal at analysis time,
//!   not a default administrator.
//!
//! Two implementation scripts on the same appliance, reporting the same
//! Tcl release, can therefore have different target ranges and different
//! principals. This module parses the metadata into a typed overlay; the
//! version interval lands on
//! [`tcl_dialect::model::VersionAxisId::big_ip`], so intersecting it with
//! the configured targets is an ordinary set operation on one axis and
//! intersecting it with a *Tcl* version is a typed error.
//!
//! The appliance security settings F7 also names (`systemauth.disablebash`
//! and friends, F5 Bug ID 589374) are deliberately **not** here: they are
//! live system policy, they were recorded by the probe run only as
//! `systemauth.disablebash false` on one host
//! (`scripts/dev/bigip-probes/results/10-context-parity.txt`, E4.1), and
//! they belong in a separate live-policy overlay rather than in a document
//! overlay parsed from template source.

use tcl_dialect::model::{HalfOpenRange, Version, VersionAxisId, VersionSet, VersionSetError};

/// The account an action's implementation runs as (F7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunAsPrincipal {
    /// `run-as` was omitted: the implementation runs as **the calling
    /// user**, who is unknown at analysis time. Authorisation-sensitive
    /// analysis widens; it must not assume an administrator surface.
    CallingUser,
    /// `run-as` named a fixed account.
    Account(String),
    /// `run-as` was present but its value could not be read statically.
    /// Widens exactly like [`Self::CallingUser`].
    Unknown,
}

impl RunAsPrincipal {
    /// Whether authorisation-sensitive analysis must widen because the
    /// principal is not statically known (F7: *"Unknown or dynamic
    /// principals/policy must widen authorisation-sensitive analysis; they
    /// must not silently inherit an administrator command surface"*).
    #[must_use]
    pub const fn widens_authorisation(&self) -> bool {
        matches!(self, Self::CallingUser | Self::Unknown)
    }
}

/// A malformed piece of action metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IAppMetadataError {
    /// A declared BIG-IP bound is not a well-formed version.
    InvalidBigIpBound {
        /// The property that carried it.
        property: &'static str,
        /// The rejected spelling.
        spelling: String,
    },
    /// The declared interval admits no BIG-IP release at all
    /// (`min` above `max`).
    EmptyBigIpInterval {
        /// The declared minimum.
        min: String,
        /// The declared maximum.
        max: String,
    },
}

impl std::fmt::Display for IAppMetadataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidBigIpBound { property, spelling } => {
                write!(f, "{property}: `{spelling}` is not a BIG-IP version")
            }
            Self::EmptyBigIpInterval { min, max } => {
                write!(f, "requires-bigip-version {min}..{max} admits no release")
            }
        }
    }
}

impl std::error::Error for IAppMetadataError {}

/// The typed document overlay of one iApp template action (F7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IAppActionOverlay {
    /// The declared compatibility interval on the **BIG-IP** axis.
    ///
    /// Unbounded when the template declares neither bound. The maximum is
    /// read as inclusive of the declared point: `requires-bigip-version-max
    /// 13.1.0` admits `13.1.0` itself. Whether it is also meant to admit
    /// later components of the same train (`13.1.0.5`) is **not
    /// measured**, so the narrow reading is taken and both declared
    /// spellings are kept verbatim in [`Self::declared_min`] /
    /// [`Self::declared_max`] for a future ruling to widen.
    pub bigip_range: VersionSet,
    /// `requires-bigip-version-min` exactly as the template spelled it.
    pub declared_min: Option<String>,
    /// `requires-bigip-version-max` exactly as the template spelled it.
    pub declared_max: Option<String>,
    /// The roles `role-acl` admits. `None` = the property was absent;
    /// an empty vector = it was present as `none`, admitting no role.
    pub role_acl: Option<Vec<String>>,
    /// The principal the implementation runs as.
    pub run_as: RunAsPrincipal,
}

impl IAppActionOverlay {
    /// An overlay declaring nothing: unbounded targets, no role
    /// restriction, and the calling user as principal — which still widens
    /// authorisation analysis.
    #[must_use]
    pub fn unconstrained() -> Self {
        Self {
            bigip_range: VersionSet::from_ranges(
                VersionAxisId::big_ip(),
                vec![HalfOpenRange::Span {
                    min: Version::parse("0").expect("0 is a version"),
                    max: None,
                }],
            ),
            declared_min: None,
            declared_max: None,
            role_acl: None,
            run_as: RunAsPrincipal::CallingUser,
        }
    }

    /// The template's interval intersected with the configured BIG-IP
    /// targets — F7's *"a workspace-wide BIG-IP default must not override
    /// a narrower template range"*, as one set operation.
    ///
    /// # Errors
    /// [`VersionSetError::AxisMismatch`] when `targets` is not on the
    /// BIG-IP axis. That is the whole point of the typed axis: passing a
    /// Tcl core target set here fails instead of silently comparing two
    /// unrelated trains.
    pub fn effective_targets(&self, targets: &VersionSet) -> Result<VersionSet, VersionSetError> {
        self.bigip_range.intersect(targets)
    }

    /// Whether `role` may run this action. `None` when the template
    /// declares no `role-acl` and the question is therefore open.
    #[must_use]
    pub fn permits_role(&self, role: &str) -> Option<bool> {
        self.role_acl
            .as_ref()
            .map(|roles| roles.iter().any(|declared| declared == role))
    }

    /// Whether authorisation-sensitive analysis must widen for this
    /// action.
    #[must_use]
    pub const fn widens_authorisation(&self) -> bool {
        self.run_as.widens_authorisation()
    }
}

/// Parse `sys application template` action metadata into a typed overlay.
///
/// `properties` are `(name, value)` pairs as the tmsh object layer read
/// them — this module deliberately does not parse configuration syntax;
/// it turns already-read properties into typed facts. Unrecognised
/// properties are ignored: an action carries many more than these four.
///
/// A `run-as` value that is empty or `none` reads as
/// [`RunAsPrincipal::Unknown`] rather than as an account name.
///
/// # Errors
/// [`IAppMetadataError`] when a declared BIG-IP bound is not a version, or
/// when the declared interval is empty.
pub fn parse_iapp_action_metadata<'a, I>(
    properties: I,
) -> Result<IAppActionOverlay, IAppMetadataError>
where
    I: IntoIterator<Item = (&'a str, &'a str)>,
{
    let mut overlay = IAppActionOverlay::unconstrained();
    let mut min = None;
    let mut max = None;
    for (name, value) in properties {
        match name {
            "requires-bigip-version-min" => min = Some(value.to_owned()),
            "requires-bigip-version-max" => max = Some(value.to_owned()),
            "role-acl" => {
                overlay.role_acl = Some(if value.trim().is_empty() || value.trim() == "none" {
                    Vec::new()
                } else {
                    value
                        .split_whitespace()
                        .filter(|word| !matches!(*word, "{" | "}"))
                        .map(str::to_owned)
                        .collect()
                });
            }
            "run-as" => {
                let trimmed = value.trim();
                overlay.run_as = if trimmed.is_empty() || trimmed == "none" {
                    RunAsPrincipal::Unknown
                } else {
                    RunAsPrincipal::Account(trimmed.to_owned())
                };
            }
            _ => {}
        }
    }

    let parse_bound = |property: &'static str, spelling: &Option<String>| {
        spelling
            .as_ref()
            .map(|text| {
                Version::parse(text).map_err(|_| IAppMetadataError::InvalidBigIpBound {
                    property,
                    spelling: text.clone(),
                })
            })
            .transpose()
    };
    let lower = parse_bound("requires-bigip-version-min", &min)?;
    let upper = parse_bound("requires-bigip-version-max", &max)?;

    let floor = lower.unwrap_or_else(|| Version::parse("0").expect("0 is a version"));
    let ranges = match upper {
        // `[floor, ceiling)` unioned with the ceiling point itself, so the
        // declared maximum is admitted; `from_ranges` normalises the two
        // into one interval.
        Some(ceiling) => {
            if floor > ceiling {
                return Err(IAppMetadataError::EmptyBigIpInterval {
                    min: min.unwrap_or_default(),
                    max: max.unwrap_or_default(),
                });
            }
            vec![
                HalfOpenRange::Span {
                    min: floor,
                    max: Some(ceiling.clone()),
                },
                HalfOpenRange::Exact(ceiling),
            ]
        }
        None => vec![HalfOpenRange::Span {
            min: floor,
            max: None,
        }],
    };
    overlay.bigip_range = VersionSet::from_ranges(VersionAxisId::big_ip(), ranges);
    overlay.declared_min = min;
    overlay.declared_max = max;
    Ok(overlay)
}

#[cfg(test)]
mod tests {

    use super::*;

    fn version(text: &str) -> Version {
        Version::parse(text).expect("test version")
    }

    /// The declared interval is action-local evidence on the BIG-IP axis,
    /// and it narrows the workspace targets rather than the other way
    /// round (F7).
    #[test]
    fn a_template_range_narrows_the_configured_targets() {
        let overlay = parse_iapp_action_metadata([
            ("requires-bigip-version-min", "13.1.0"),
            ("requires-bigip-version-max", "17.1.0"),
            ("description", "ignored"),
        ])
        .expect("well-formed metadata");
        assert_eq!(overlay.declared_min.as_deref(), Some("13.1.0"));
        assert_eq!(overlay.declared_max.as_deref(), Some("17.1.0"));
        assert!(overlay.bigip_range.contains(&version("13.1.0")));
        assert!(overlay.bigip_range.contains(&version("15.1.5")));
        assert!(
            overlay.bigip_range.contains(&version("17.1.0")),
            "the declared maximum is admitted"
        );
        assert!(!overlay.bigip_range.contains(&version("12.1.0")));
        assert!(!overlay.bigip_range.contains(&version("21.1.0.1")));

        let workspace = VersionSet::from_requirements(VersionAxisId::big_ip(), &["15.0-"])
            .expect("workspace targets");
        let effective = overlay.effective_targets(&workspace).expect("same axis");
        assert!(effective.contains(&version("15.1.5")));
        assert!(
            !effective.contains(&version("13.1.0")),
            "outside the workspace targets"
        );
        assert!(
            !effective.contains(&version("21.1.0.1")),
            "outside the template's declared range"
        );
    }

    /// The axis is typed: a Tcl target set cannot be intersected with a
    /// BIG-IP interval, however similar the numbers look (F6/I2).
    #[test]
    fn a_tcl_target_set_cannot_be_intersected_with_a_bigip_interval() {
        let overlay = parse_iapp_action_metadata([("requires-bigip-version-min", "13.1.0")])
            .expect("well-formed metadata");
        let tcl = VersionSet::from_requirements(
            VersionAxisId::core(tcl_dialect::model::family::Family::Tcl),
            &["8.5"],
        )
        .expect("core targets");
        assert!(matches!(
            overlay.effective_targets(&tcl),
            Err(VersionSetError::AxisMismatch { .. })
        ));
    }

    /// An omitted `run-as` means the calling user — an unknown principal,
    /// which widens rather than inheriting an administrator surface.
    #[test]
    fn an_omitted_principal_widens_authorisation() {
        let omitted = parse_iapp_action_metadata([("role-acl", "{ admin manager }")])
            .expect("well-formed metadata");
        assert_eq!(omitted.run_as, RunAsPrincipal::CallingUser);
        assert!(omitted.widens_authorisation());
        assert_eq!(omitted.permits_role("admin"), Some(true));
        assert_eq!(omitted.permits_role("guest"), Some(false));

        let named = parse_iapp_action_metadata([("run-as", "admin")]).expect("well-formed");
        assert_eq!(named.run_as, RunAsPrincipal::Account("admin".to_owned()));
        assert!(!named.widens_authorisation());
        assert_eq!(named.permits_role("admin"), None, "no role-acl declared");

        let none_acl =
            parse_iapp_action_metadata([("role-acl", "none"), ("run-as", "none")]).expect("ok");
        assert_eq!(none_acl.role_acl, Some(Vec::new()));
        assert_eq!(none_acl.permits_role("admin"), Some(false));
        assert_eq!(none_acl.run_as, RunAsPrincipal::Unknown);
        assert!(none_acl.widens_authorisation());
    }

    /// Malformed and impossible declarations are typed errors, not silent
    /// unbounded ranges.
    #[test]
    fn malformed_bounds_are_rejected() {
        assert_eq!(
            parse_iapp_action_metadata([("requires-bigip-version-min", "13.1.0-hotfix")]),
            Err(IAppMetadataError::InvalidBigIpBound {
                property: "requires-bigip-version-min",
                spelling: "13.1.0-hotfix".to_owned(),
            })
        );
        assert_eq!(
            parse_iapp_action_metadata([
                ("requires-bigip-version-min", "17.1.0"),
                ("requires-bigip-version-max", "13.1.0"),
            ]),
            Err(IAppMetadataError::EmptyBigIpInterval {
                min: "17.1.0".to_owned(),
                max: "13.1.0".to_owned(),
            })
        );
        let unbounded = IAppActionOverlay::unconstrained();
        assert!(unbounded.bigip_range.contains(&version("21.1.0.1")));
        assert!(unbounded.bigip_range.contains(&version("11.5.0")));
        assert_eq!(unbounded.permits_role("admin"), None);
        assert!(unbounded.widens_authorisation());
    }

    /// The three properties this overlay reads are the ones the shipping
    /// BIG-IP object schema declares for `sys application template` — the
    /// overlay and the schema cannot drift apart silently.
    #[test]
    fn the_properties_exist_in_the_bigip_object_schema() {
        let registry = crate::bigip::BigipRegistry::build();
        let spec = registry
            .get("sys_application_template")
            .expect("the template object kind");
        let mut names = std::collections::HashSet::new();
        let mut stack: Vec<&crate::bigip::BigipPropertySpec> = spec.properties.iter().collect();
        while let Some(property) = stack.pop() {
            names.insert(property.name);
            stack.extend(property.block.iter());
        }
        for property in [
            "requires-bigip-version-min",
            "requires-bigip-version-max",
            "role-acl",
            "run-as",
        ] {
            assert!(names.contains(property), "{property} missing from schema");
        }
    }
}
