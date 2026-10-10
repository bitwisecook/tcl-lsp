// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original-object native frame selection, separate from static level advice.

use super::{NativeFrameLevelCache, NativeFrameLevelProtocol};
use tcl_dialect::TclVersion;
use tcl_syntax::scalar_getter::{
    NativeScalarGetterError, NativeScalarGetterFailure, NativeScalarGetterKind,
    NativeScalarGetterProtocol, NativeScalarGetterValue,
};

/// Original object operations required by the selected native frame recipe.
/// Host refusals stay outside native guest failures; probes still commit every
/// reached original cache transition before returning an unsuccessful outcome.
pub trait NativeFrameLevelObject {
    /// Missing host capability or authentic object storage.
    type Error;
    /// Materialize the original string under its independently selected engine.
    ///
    /// # Errors
    /// Returns the object's actual storage or updater refusal.
    fn native_frame_string(&mut self) -> Result<Vec<u8>, Self::Error>;
    /// Run an error-neutral getter on the original shared object.
    ///
    /// # Errors
    /// Returns an unavailable native getter or its dependency refusal.
    fn native_frame_probe(
        &mut self,
        kind: NativeScalarGetterKind,
    ) -> Result<Result<i64, NativeScalarGetterFailure>, Self::Error>;
    /// Inspect the original integer cache, including exact big magnitudes.
    fn native_frame_is_integer(&self) -> bool;
    /// Inspect only native machine-int/wide caches, excluding bignums.
    fn native_frame_is_machine_integer(&self) -> bool;
    /// Inspect the original release-authenticated frame cache.
    ///
    /// # Errors
    /// Rejects an unauthenticated or foreign cache origin.
    fn native_frame_cache(&self) -> Result<Option<NativeFrameLevelCache>, Self::Error>;
    /// Install the reached frame cache while preserving original resident bytes.
    ///
    /// # Errors
    /// Rejects missing resident storage or a foreign native cache recipe.
    fn native_frame_set_cache(&mut self, cache: NativeFrameLevelCache) -> Result<(), Self::Error>;
}

/// Successful original-object level interpretation and retained frame target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeFrameLevelResolution {
    /// The first operand is consumed as the level word.
    pub explicit: bool,
    /// Existing physical call-frame level selected by the recipe.
    pub target: usize,
}

pub use tcl_syntax::native_frame_error::NativeFrameLevelFailure;

impl NativeFrameLevelProtocol {
    /// C9's single pure script operand probes list length before frame conversion.
    #[must_use]
    pub fn probes_single_script_list_first(self) -> bool {
        self.tcl_version()
            .is_some_and(|version| version >= TclVersion::V9_0)
    }

    /// Resolve a command's optional leading level. Jim's command-level probe
    /// tests the materialized first byte before its original Long getter;
    /// forced parity levels use [`Self::resolve_object`] directly.
    ///
    /// # Errors
    /// Preserves the same host/native failure distinction as object resolution.
    pub fn resolve_leading_object<O: NativeFrameLevelObject>(
        self,
        current: usize,
        original: &mut O,
    ) -> Result<Result<NativeFrameLevelResolution, NativeFrameLevelFailure>, O::Error> {
        if self.is_jim084() {
            let name = original.native_frame_string()?;
            if !name
                .first()
                .is_some_and(|byte| byte.is_ascii_digit() || *byte == b'#')
            {
                return Ok(self.finish(current, Some((true, 1)), false, b"1"));
            }
        }
        self.resolve_object(current, original)
    }

    /// Select a frame through the actual original-object conversion schedule.
    /// Static lexical overrides and semantic numeric labels do not authorize
    /// an original cache, primitive getter or frame-reference publication.
    ///
    /// # Errors
    /// The outer error preserves unavailable host operations. The inner error
    /// is a reached native guest failure after its original cache effects.
    pub fn resolve_object<O: NativeFrameLevelObject>(
        self,
        current: usize,
        original: &mut O,
    ) -> Result<Result<NativeFrameLevelResolution, NativeFrameLevelFailure>, O::Error> {
        self.resolve_object_with_requirement(current, original, false)
    }

    /// Resolve an operand that the command's genuine argc already requires as
    /// a level. A non-level value cannot become an omitted caller/default here.
    /// Original conversion/cache order remains the same as the leading probe.
    ///
    /// # Errors
    /// Host/native storage refusals remain outside reached guest failures.
    pub fn resolve_required_object<O: NativeFrameLevelObject>(
        self,
        current: usize,
        original: &mut O,
    ) -> Result<Result<NativeFrameLevelResolution, NativeFrameLevelFailure>, O::Error> {
        // naming.variable.original-upvar-and-exists-completion-and-name-windows
        // docs/design/analysis/name-resolution-proofs/variable.original-upvar-and-exists-completion-and-name-windows.md
        self.resolve_object_with_requirement(current, original, true)
    }

    fn resolve_object_with_requirement<O: NativeFrameLevelObject>(
        self,
        current: usize,
        original: &mut O,
        required: bool,
    ) -> Result<Result<NativeFrameLevelResolution, NativeFrameLevelFailure>, O::Error> {
        if self.is_jim084() {
            return self.resolve_jim_object(current, original);
        }
        let version = self.tcl_version().expect("selected C frame recipe");
        if self.probes_integer_first() {
            self.resolve_c_integer_first(current, original, version, required)
        } else {
            self.resolve_c_string_first(current, original, version, required)
        }
    }

    fn resolve_jim_object<O: NativeFrameLevelObject>(
        self,
        current: usize,
        original: &mut O,
    ) -> Result<Result<NativeFrameLevelResolution, NativeFrameLevelFailure>, O::Error> {
        let name = original.native_frame_string()?;
        let prefix = c_prefix(&name);
        let selector = if prefix.first() == Some(&b'#') {
            let getter = NativeScalarGetterProtocol::for_point(
                tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
            )
            .expect("selected pinned Jim recipe");
            getter
                .jim_absolute_frame_probe(&prefix[1..])
                .expect("selected Jim string stage")
                .ok()
                .map(|level| (false, level))
        } else {
            original
                .native_frame_probe(NativeScalarGetterKind::Wide)?
                .ok()
                .map(|level| (true, level))
        };
        Ok(self.finish(current, selector, true, prefix))
    }

    fn resolve_c_integer_first<O: NativeFrameLevelObject>(
        self,
        current: usize,
        original: &mut O,
        version: TclVersion,
        required: bool,
    ) -> Result<Result<NativeFrameLevelResolution, NativeFrameLevelFailure>, O::Error> {
        let integer = original.native_frame_probe(NativeScalarGetterKind::Int)?;
        if let Ok(level) = integer {
            if version >= TclVersion::V9_0 {
                let wide = original.native_frame_probe(NativeScalarGetterKind::Wide)?;
                let distance = wide
                    .ok()
                    .filter(|wide| (0..=i64::from(i32::MAX)).contains(wide));
                if let Some(distance) = distance {
                    return self.finish_original(current, Some((true, distance)), true, original);
                }
                return self.finish_original(current, None, true, original);
            }
            if level >= 0 {
                return self.finish_original(current, Some((true, level)), true, original);
            }
        }
        if let Some(NativeFrameLevelCache::Absolute(level)) = original.native_frame_cache()? {
            return self.finish_original(current, Some((false, i64::from(level))), true, original);
        }
        let name = original.native_frame_string()?;
        let prefix = c_prefix(&name);
        if prefix.first() == Some(&b'#') {
            return self.absolute_suffix(current, original, prefix, version);
        }
        let is_level = if version >= TclVersion::V9_0 {
            original.native_frame_is_integer()
        } else {
            prefix.first().is_some_and(u8::is_ascii_digit)
        };
        Ok(self.finish(
            current,
            (!is_level && !required).then_some((true, 1)),
            required,
            if is_level || required { prefix } else { b"1" },
        ))
    }

    fn resolve_c_string_first<O: NativeFrameLevelObject>(
        self,
        current: usize,
        original: &mut O,
        version: TclVersion,
        required: bool,
    ) -> Result<Result<NativeFrameLevelResolution, NativeFrameLevelFailure>, O::Error> {
        let name = original.native_frame_string()?;
        let prefix = c_prefix(&name);
        if version == TclVersion::V8_5 {
            if let Some(cache) = original.native_frame_cache()? {
                let selector = match cache {
                    NativeFrameLevelCache::Relative(level) => (true, i64::from(level)),
                    NativeFrameLevelCache::Absolute(level) => (false, i64::from(level)),
                };
                return Ok(self.finish(current, Some(selector), true, prefix));
            }
            if original.native_frame_is_machine_integer() {
                let level = original
                    .native_frame_probe(NativeScalarGetterKind::Int)?
                    .ok();
                return Ok(self.finish(current, level.map(|level| (true, level)), true, prefix));
            }
        }
        if prefix.first() == Some(&b'#') {
            return self.absolute_suffix(current, original, prefix, version);
        }
        if prefix.first().is_some_and(u8::is_ascii_digit) {
            match temporary_int(version, prefix) {
                Ok(level) => {
                    if version == TclVersion::V8_5 {
                        original.native_frame_set_cache(NativeFrameLevelCache::Relative(level))?;
                    }
                    return Ok(self.finish(current, Some((true, i64::from(level))), true, prefix));
                }
                Err(record) => return Ok(Err(NativeFrameLevelFailure::Primitive(record))),
            }
        }
        Ok(self.finish(current, (!required).then_some((true, 1)), required, prefix))
    }

    fn absolute_suffix<O: NativeFrameLevelObject>(
        self,
        current: usize,
        original: &mut O,
        name: &[u8],
        version: TclVersion,
    ) -> Result<Result<NativeFrameLevelResolution, NativeFrameLevelFailure>, O::Error> {
        match temporary_int(version, &name[1..]) {
            Ok(level)
                if level >= 0
                    && !(version >= TclVersion::V9_0
                        && level > 0
                        && name.get(1) == Some(&b'-')) =>
            {
                if version >= TclVersion::V8_5 {
                    original.native_frame_set_cache(NativeFrameLevelCache::Absolute(level))?;
                }
                Ok(self.finish(current, Some((false, i64::from(level))), true, name))
            }
            Err(record) if version == TclVersion::V8_4 => {
                Ok(Err(NativeFrameLevelFailure::Primitive(record)))
            }
            _ => Ok(self.finish(current, None, true, name)),
        }
    }

    fn finish_original<O: NativeFrameLevelObject>(
        self,
        current: usize,
        selector: Option<(bool, i64)>,
        explicit: bool,
        original: &mut O,
    ) -> Result<Result<NativeFrameLevelResolution, NativeFrameLevelFailure>, O::Error> {
        if let Some(target) = target(current, selector) {
            return Ok(Ok(NativeFrameLevelResolution { explicit, target }));
        }
        let name = original.native_frame_string()?;
        Ok(self.finish(current, None, explicit, c_prefix(&name)))
    }

    fn finish(
        self,
        current: usize,
        selector: Option<(bool, i64)>,
        explicit: bool,
        name: &[u8],
    ) -> Result<NativeFrameLevelResolution, NativeFrameLevelFailure> {
        target(current, selector)
            .map(|target| NativeFrameLevelResolution { explicit, target })
            .ok_or_else(|| NativeFrameLevelFailure::BadLevel {
                name: name.to_vec(),
                string_result: self.bad_level_string_result(),
                lookup_code: self
                    .tcl_version()
                    .is_some_and(|version| version >= TclVersion::V8_6),
            })
    }
}

fn target(current: usize, selector: Option<(bool, i64)>) -> Option<usize> {
    let (relative, level) = selector?;
    let current = i64::try_from(current).ok()?;
    let target = if relative {
        current.checked_sub(level)?
    } else {
        level
    };
    (0..=current)
        .contains(&target)
        .then(|| usize::try_from(target).ok())
        .flatten()
}

fn temporary_int(version: TclVersion, bytes: &[u8]) -> Result<i32, Box<NativeScalarGetterError>> {
    let protocol = NativeScalarGetterProtocol::for_tcl_version(version);
    let conversion = protocol
        .fresh_conversion(NativeScalarGetterKind::Int, bytes)
        .expect("selected C temporary Int stage");
    let (_, _, outcome) = conversion.into_parts();
    match outcome {
        Ok(NativeScalarGetterValue::Wide(level)) => {
            Ok(i32::try_from(level).expect("native Int result is signed32"))
        }
        Err(failure) => Err(Box::new(
            protocol
                .failure_presentation(NativeScalarGetterKind::Int, failure, bytes)
                .expect("selected C Int failure"),
        )),
        _ => unreachable!("native Int getter result"),
    }
}
fn c_prefix(bytes: &[u8]) -> &[u8] {
    bytes.split(|byte| *byte == 0).next().unwrap_or_default()
}
