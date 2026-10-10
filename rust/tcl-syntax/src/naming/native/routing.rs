// SPDX-License-Identifier: AGPL-3.0-or-later
//! Command qualifier routing retains its purpose independently of display paths.

use super::{
    NameProjectionUnavailable, NativeNameContext, NativeNameProjection, NativeNameProtocol,
    NativeNamePurpose, NativeNameQualification, TclVersion, ends_with_separator,
    qualifier_segments, written_command_tail,
};
use tcl_core_types::{ByteCommandSlot, ByteNamespacePath, NameBytes};

/// The physical namespace token from which selected qualifiers must be walked.
/// Equal constructed paths do not make the caller's retained token global.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeCommandNamespaceRoute {
    /// Start at the interpreter's actual global namespace token.
    Root,
    /// Start at the exact current namespace token supplied by the consumer.
    Context,
    /// Start at the actual parent; a detached parent uses the current token,
    /// matching the C qualifier resolver's null-context rule.
    ContextParent,
}

/// Pure command geometry with the operation's retained qualifier routing.
/// Consumers supply real tokens and choose creation, lifecycle and fallback
/// behaviour. This record grants no live namespace or command identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeCommandSlotProjection {
    slot: ByteCommandSlot,
    namespace_route: NativeCommandNamespaceRoute,
    qualifiers: ByteNamespacePath,
    protocol: NativeNameProtocol,
    purpose: NativeNamePurpose,
}

impl NativeCommandSlotProjection {
    /// Constructed byte geometry, suitable for source-only comparisons.
    #[must_use]
    pub const fn slot(&self) -> &ByteCommandSlot {
        &self.slot
    }
    /// Actual-token anchor selected by this operation's naming recipe.
    #[must_use]
    pub const fn namespace_route(&self) -> NativeCommandNamespaceRoute {
        self.namespace_route
    }
    /// Exact child keys to walk below the selected token anchor.
    #[must_use]
    pub fn qualifiers(&self) -> &[NameBytes] {
        self.qualifiers.as_segments()
    }
    /// Selected engine recipe; this is not a native execution receipt.
    #[must_use]
    pub const fn protocol(&self) -> NativeNameProtocol {
        self.protocol
    }
    /// Operation selecting the geometry and anchor.
    #[must_use]
    pub const fn purpose(&self) -> NativeNamePurpose {
        self.purpose
    }
    /// Discard token routing when a consumer requires only pure geometry.
    #[must_use]
    pub fn into_slot(self) -> ByteCommandSlot {
        self.slot
    }
}

impl NativeNameProtocol {
    /// First C lookup candidate, including its original qualifier anchor.
    /// Namespace-path and global fallback traversal remain consumer operations.
    ///
    /// # Errors
    /// Refuses Jim's distinct flat command-table lookup model.
    pub fn command_lookup_projection(
        self,
        context: NativeNameContext<'_>,
        original: &[u8],
    ) -> Result<NativeCommandSlotProjection, NameProjectionUnavailable> {
        if self.is_jim084() {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        command_slot_projection(&self.command_lookup_input(context, original)?)
    }

    /// Procedure publication retains its release-selected registration anchor.
    /// C8.4/8.5 reparses a non-global holder's constructed C API name at root;
    /// later C recipes retain the original holder token for relative names.
    ///
    /// # Errors
    /// Refuses a missing actual Jim namespace object.
    pub fn command_publication_projection(
        self,
        context: NativeNameContext<'_>,
        original: &[u8],
    ) -> Result<NativeCommandSlotProjection, NameProjectionUnavailable> {
        let projected =
            command_slot_projection(&self.command_publication_input(context, original)?)?;
        if matches!(self, Self::C(version) if version <= TclVersion::V8_5)
            && !projected.slot.namespace.is_root()
        {
            let mut full_name = Vec::from(b"::".as_slice());
            for component in projected.slot.namespace.as_segments() {
                full_name.extend_from_slice(component.as_bytes());
                full_name.extend_from_slice(b"::");
            }
            full_name.extend_from_slice(projected.slot.simple.as_bytes());
            let mut registered =
                self.command_c_api_publication_projection(NativeNameContext::root(), &full_name)?;
            registered.purpose = NativeNamePurpose::CommandPublication;
            return Ok(registered);
        }
        Ok(projected)
    }

    /// Ensemble publication retains its own registration purpose. `parent`
    /// describes the actual parent token, not a parent reconstructed from a
    /// reporting path; a detached parent is supplied as `None`.
    ///
    /// # Errors
    /// Refuses C8.4 and a missing actual Jim namespace object.
    pub fn ensemble_publication_projection(
        self,
        context: NativeNameContext<'_>,
        explicit: Option<&[u8]>,
        parent: Option<&ByteNamespacePath>,
    ) -> Result<NativeCommandSlotProjection, NameProjectionUnavailable> {
        if matches!(self, Self::C(version) if version < TclVersion::V8_5) {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        if let Some(original) = explicit {
            let mut selected = if self == Self::C(TclVersion::V8_5) {
                self.command_publication_projection(context, original)?
            } else {
                command_slot_projection(&self.command_input(
                    context,
                    original,
                    NativeNamePurpose::EnsemblePublication,
                )?)?
            };
            selected.purpose = NativeNamePurpose::EnsemblePublication;
            return Ok(selected);
        }
        if self.is_jim084() {
            let original = context
                .jim_namespace_object
                .ok_or(NameProjectionUnavailable::MissingJimNamespaceObject)?;
            return Ok(NativeCommandSlotProjection {
                slot: ByteCommandSlot::new(ByteNamespacePath::root(), NameBytes::from(original)),
                namespace_route: NativeCommandNamespaceRoute::Root,
                qualifiers: ByteNamespacePath::root(),
                protocol: self,
                purpose: NativeNamePurpose::EnsemblePublication,
            });
        }
        if self == Self::C(TclVersion::V8_5) {
            let mut written = b"::".to_vec();
            for (index, component) in context.namespace.as_segments().iter().enumerate() {
                if index != 0 {
                    written.extend_from_slice(b"::");
                }
                written.extend_from_slice(component.as_bytes());
            }
            let mut selected =
                self.command_c_api_publication_projection(NativeNameContext::root(), &written)?;
            selected.purpose = NativeNamePurpose::EnsemblePublication;
            return Ok(selected);
        }
        let simple = context
            .namespace
            .as_segments()
            .last()
            .cloned()
            .unwrap_or_default();
        Ok(NativeCommandSlotProjection {
            slot: ByteCommandSlot::new(parent.unwrap_or(context.namespace).clone(), simple),
            namespace_route: NativeCommandNamespaceRoute::ContextParent,
            qualifiers: ByteNamespacePath::root(),
            protocol: self,
            purpose: NativeNamePurpose::EnsemblePublication,
        })
    }

    /// C API registration keeps unqualified names global and relative
    /// qualified names below the exact current token.
    ///
    /// # Errors
    /// Refuses engines without this C registration purpose.
    pub fn command_c_api_publication_projection(
        self,
        context: NativeNameContext<'_>,
        original: &[u8],
    ) -> Result<NativeCommandSlotProjection, NameProjectionUnavailable> {
        command_slot_projection(&self.command_c_api_publication_input(context, original)?)
    }

    /// Alias registration uses its own global/unqualified and current/relative
    /// rule; Jim retains a flat comparison key at the global table.
    ///
    /// # Errors
    /// Refuses an unavailable alias publication purpose.
    pub fn alias_publication_projection(
        self,
        context: NativeNameContext<'_>,
        original: &[u8],
    ) -> Result<NativeCommandSlotProjection, NameProjectionUnavailable> {
        command_slot_projection(&self.alias_publication_input(context, original)?)
    }

    /// Original child-alias registration uses its own constructor extent.
    /// The selected slot is still materialised in the actual child's realm.
    ///
    /// # Errors
    /// Refuses an unavailable registration purpose.
    pub fn child_alias_publication_projection(
        self,
        context: NativeNameContext<'_>,
        original: &[u8],
    ) -> Result<NativeCommandSlotProjection, NameProjectionUnavailable> {
        command_slot_projection(&self.child_alias_publication_input(context, original)?)
    }

    /// Rename destination geometry without procedure-publication reparsing.
    ///
    /// # Errors
    /// Refuses a missing actual Jim namespace object.
    pub fn rename_destination_projection(
        self,
        context: NativeNameContext<'_>,
        original: &[u8],
    ) -> Result<NativeCommandSlotProjection, NameProjectionUnavailable> {
        command_slot_projection(&self.rename_destination_input(context, original)?)
    }

    /// Coroutine publication walks existing qualifiers from its selected token.
    ///
    /// # Errors
    /// Refuses engines without audited C coroutine publication.
    pub fn coroutine_publication_projection(
        self,
        context: NativeNameContext<'_>,
        original: &[u8],
    ) -> Result<NativeCommandSlotProjection, NameProjectionUnavailable> {
        if !matches!(self, Self::C(version) if version >= TclVersion::V8_6) {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        command_slot_projection(&self.command_input(
            context,
            original,
            NativeNamePurpose::CoroutinePublication,
        )?)
    }

    /// `TclOO` object publication walks qualifiers from its exact selected token.
    ///
    /// # Errors
    /// Refuses engines without audited `TclOO` object publication.
    pub fn oo_object_publication_projection(
        self,
        context: NativeNameContext<'_>,
        original: &[u8],
    ) -> Result<NativeCommandSlotProjection, NameProjectionUnavailable> {
        if !matches!(self, Self::C(version) if version >= TclVersion::V8_6) {
            return Err(NameProjectionUnavailable::PurposeNotModelled);
        }
        command_slot_projection(&self.command_input(
            context,
            original,
            NativeNamePurpose::OoObjectPublication,
        )?)
    }
}

pub(super) fn command_slot_projection(
    input: &NativeNameProjection<'_>,
) -> Result<NativeCommandSlotProjection, NameProjectionUnavailable> {
    if let Some(key) = input.jim_flat_key() {
        return Ok(NativeCommandSlotProjection {
            slot: ByteCommandSlot::new(ByteNamespacePath::root(), NameBytes::from(key)),
            namespace_route: NativeCommandNamespaceRoute::Root,
            qualifiers: ByteNamespacePath::root(),
            protocol: input.protocol,
            purpose: input.purpose,
        });
    }
    let context = input
        .context
        .ok_or(NameProjectionUnavailable::PurposeNotModelled)?;
    let global_unqualified = input.qualification == NativeNameQualification::Unqualified
        && matches!(
            input.purpose,
            NativeNamePurpose::CommandCApiPublication
                | NativeNamePurpose::AliasPublication
                | NativeNamePurpose::ChildAliasPublication
        );
    let namespace_route =
        if input.qualification == NativeNameQualification::Absolute || global_unqualified {
            NativeCommandNamespaceRoute::Root
        } else {
            NativeCommandNamespaceRoute::Context
        };
    let segments = qualifier_segments(input.selected());
    let count = if ends_with_separator(input.selected()) {
        segments.len()
    } else {
        segments.len().saturating_sub(1)
    };
    let qualifiers = ByteNamespacePath::from_segments(segments[..count].iter().copied());
    let mut namespace = match namespace_route {
        NativeCommandNamespaceRoute::Root => ByteNamespacePath::root(),
        NativeCommandNamespaceRoute::Context | NativeCommandNamespaceRoute::ContextParent => {
            context.namespace.clone()
        }
    };
    for segment in qualifiers.as_segments() {
        namespace.push(segment.clone());
    }
    Ok(NativeCommandSlotProjection {
        slot: ByteCommandSlot::new(
            namespace,
            NameBytes::from(written_command_tail(input.selected())),
        ),
        namespace_route,
        qualifiers,
        protocol: input.protocol,
        purpose: input.purpose,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensemble_default_parent_and_explicit_current_routing_remain_distinct() {
        // Native proof: naming.ensemble.original-command-holder-routing
        // docs/design/analysis/name-resolution-proofs/ensemble-original-command-holder-routing.md
        // These assertions test recipe geometry only; Runtime and VM source
        // controls compare the separate public native completion columns.
        let path = ByteNamespacePath::from_segments([b"P".as_slice(), b"N"]);
        let parent = ByteNamespacePath::from_segments([b"P".as_slice()]);
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            let recipe = NativeNameProtocol::C(version);
            let context = NativeNameContext::new(&path);
            let live = recipe
                .ensemble_publication_projection(context, None, Some(&parent))
                .unwrap();
            assert_eq!(
                live.namespace_route(),
                NativeCommandNamespaceRoute::ContextParent
            );
            assert_eq!(live.slot().namespace, parent);
            assert_eq!(live.slot().simple.as_bytes(), b"N");
            let detached = recipe
                .ensemble_publication_projection(context, None, None)
                .unwrap();
            let explicit = recipe
                .ensemble_publication_projection(context, Some(b"N"), None)
                .unwrap();
            assert_eq!(detached.slot(), explicit.slot());
            assert_eq!(
                detached.namespace_route(),
                NativeCommandNamespaceRoute::ContextParent
            );
            assert_eq!(
                explicit.namespace_route(),
                NativeCommandNamespaceRoute::Context
            );
            assert_eq!(detached.slot().namespace, path);
            assert_eq!(detached.purpose(), NativeNamePurpose::EnsemblePublication);
            assert_eq!(explicit.purpose(), NativeNamePurpose::EnsemblePublication);
            let rooted = recipe
                .ensemble_publication_projection(context, Some(b"::P::N"), None)
                .unwrap();
            assert_eq!(rooted.slot(), live.slot());
            assert_eq!(rooted.namespace_route(), NativeCommandNamespaceRoute::Root);
        }
    }

    #[test]
    fn equal_constructed_paths_retain_distinct_root_and_current_routes() {
        let path = ByteNamespacePath::from_segments([b"N".as_slice()]);
        let context = NativeNameContext::new(&path);
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let recipe = NativeNameProtocol::C(version);
            let current = recipe.command_lookup_projection(context, b"p").unwrap();
            let rooted = recipe
                .command_lookup_projection(context, b"::N::p")
                .unwrap();
            assert_eq!(current.slot(), rooted.slot());
            assert_eq!(
                current.namespace_route(),
                NativeCommandNamespaceRoute::Context
            );
            assert_eq!(rooted.namespace_route(), NativeCommandNamespaceRoute::Root);
            assert!(current.qualifiers().is_empty());
            assert_eq!(rooted.qualifiers(), path.as_segments());
            let published = recipe
                .command_publication_projection(context, b"p")
                .unwrap();
            assert_eq!(published.purpose(), NativeNamePurpose::CommandPublication);
            assert_eq!(
                published.namespace_route(),
                if version <= TclVersion::V8_5 {
                    NativeCommandNamespaceRoute::Root
                } else {
                    NativeCommandNamespaceRoute::Context
                }
            );
            assert_eq!(published.slot(), current.slot());
        }
    }

    #[test]
    fn purpose_and_selected_extent_control_alias_and_c_api_routes() {
        let path = ByteNamespacePath::from_segments([b"N".as_slice()]);
        let context = NativeNameContext::new(&path);
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let recipe = NativeNameProtocol::C(version);
            for source in [b"plain".as_slice(), b"plain\0q::hidden"] {
                let alias = recipe
                    .alias_publication_projection(context, source)
                    .unwrap();
                let api = recipe
                    .command_c_api_publication_projection(context, source)
                    .unwrap();
                assert_eq!(alias.namespace_route(), NativeCommandNamespaceRoute::Root);
                assert_eq!(api.namespace_route(), NativeCommandNamespaceRoute::Root);
                assert!(alias.qualifiers().is_empty());
                assert_eq!(alias.slot(), api.slot());
                assert_eq!(alias.slot().simple.as_bytes(), b"plain");
            }
            let relative = recipe
                .alias_publication_projection(context, b"q::a")
                .unwrap();
            assert_eq!(
                relative.namespace_route(),
                NativeCommandNamespaceRoute::Context
            );
            assert_eq!(relative.qualifiers(), &[NameBytes::from(b"q".as_slice())]);
        }
        let jim = NativeNameProtocol::Jim084
            .alias_publication_projection(context, b"::::a\0z")
            .unwrap();
        assert_eq!(jim.namespace_route(), NativeCommandNamespaceRoute::Root);
        assert!(jim.qualifiers().is_empty());
        assert_eq!(jim.slot().simple.as_bytes(), b"a\0z");
    }
}
