// SPDX-License-Identifier: AGPL-3.0-or-later
//! Genuine native namespace-name primary storage and checked conversion.

use super::{IntRep, Value};
use tcl_runtime_api::native_namespace_name::NativeNamespaceNameCache;
use tcl_syntax::value::ValueError;

impl Value {
    /// Inspect the original physical primary without reading or generating strings.
    #[must_use]
    pub fn native_namespace_name_cache(&self) -> Option<NativeNamespaceNameCache> {
        match &*self.0.intrep.borrow() {
            IntRep::NativeNamespaceName(cache) => Some(cache.clone()),
            _ => None,
        }
    }

    /// Install a checked actual-engine descriptor on this same original object.
    /// Namespace-world validation precedes this physical conversion.
    ///
    /// # Errors
    /// Refuses a foreign issuer or missing resident storage for a no-updater primary.
    pub fn install_native_namespace_name_cache(
        &self,
        cache: NativeNamespaceNameCache,
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<(), ValueError> {
        let protocol = dialect.native_namespace_name_protocol().ok_or(
            ValueError::CommandProtocolUnavailable("native namespace-name object issuer"),
        )?;
        if protocol.recipe().version() != cache.version()
            || (!protocol.recipe().has_string_updater() && self.resident_string_bytes().is_none())
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "native namespace-name cache origin or storage",
            ));
        }
        *self.0.intrep.borrow_mut() = IntRep::NativeNamespaceName(cache);
        Ok(())
    }

    /// Recover a genuine retained descriptor under its independently selected issuer.
    /// The issuing engine must validate its private interpreter receipt first.
    ///
    /// # Errors
    /// Refuses a foreign native release or an absent string for a no-updater primary.
    pub fn from_native_namespace_name_cache(
        cache: NativeNamespaceNameCache,
        dialect: tcl_registry::InvocationDialect,
        resident: Option<(
            std::rc::Rc<[u8]>,
            tcl_syntax::native_string::NativeStringStorageIdentity,
        )>,
    ) -> Result<Self, ValueError> {
        let mut value = Self::from_native_string_bytes(b"".as_slice());
        if let Some((bytes, storage)) = resident {
            value = value.with_resident_string_bytes_and_storage(bytes, storage)?;
        } else {
            *value.0.string.borrow_mut() = None;
        }
        value.install_native_namespace_name_cache(cache, dialect)?;
        Ok(value)
    }

    /// Construct an explicitly authored namespace result; this grants no
    /// native namespace cache or physical engine authority.
    ///
    /// # Errors
    /// Refuses a provider that is not independently validated for logical F5.
    pub(crate) fn authored_namespace_result(
        bytes: &[u8],
        producer: tcl_syntax::native_namespace_name::NativeNamespaceObjectProducer,
        provider: tcl_syntax::naming::NamePolicyProtocol,
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<Self, ValueError> {
        let recipe = dialect.authored_namespace_result_recipe(provider).ok_or(
            ValueError::CommandProtocolUnavailable("authored namespace result provider"),
        )?;
        let primary = recipe
            .result_primary(
                producer,
                tcl_syntax::native_namespace_name::NativeNamespaceLifecycle::Live,
            )
            .ok_or(ValueError::CommandProtocolUnavailable(
                "authored namespace result purpose",
            ))?;
        let value = if primary
            == tcl_syntax::native_namespace_name::NativeNamespaceCurrentPrimary::String
        {
            Self::from_raw_parts(
                Some(tcl_syntax::raw_string::RawString::from_bytes(bytes)),
                IntRep::NativeString {
                    protocol: tcl_syntax::native_string::NativeStringProtocol::C(recipe.version()),
                    num_chars: None,
                    unicode: None,
                },
            )
        } else {
            Self::from_native_string_bytes(bytes)
        };
        value
            .0
            .string_storage
            .set(tcl_syntax::native_string::NativeStringStorageIdentity::Allocated);
        Ok(value)
    }

    /// Retire only this original namespace primary, retaining its resident string.
    pub fn retire_native_namespace_name_cache(&self) {
        if matches!(*self.0.intrep.borrow(), IntRep::NativeNamespaceName(_)) {
            *self.0.intrep.borrow_mut() = IntRep::Str;
        }
    }
}
