// SPDX-License-Identifier: AGPL-3.0-or-later
//! Genuine C91 property-name primary; fields own original accessor headers.
use super::{IntRep, NativeObjectLifetimeLease, Value};
#[derive(Clone)]
pub(super) struct NativePropertyName {
    pub(super) reader: Value,
    pub(super) writer: Value,
}
impl Value {
    pub(crate) fn native_property_name_is_cached(&self) -> bool {
        matches!(&*self.0.intrep.borrow(), IntRep::NativePropertyName(_))
    }
    #[cfg(test)]
    pub(crate) fn native_property_accessor(
        &self,
        recipe: tcl_registry::native_property_lookup::NativePropertyLookupProtocol,
        writable: bool,
    ) -> Result<NativeObjectLifetimeLease, tcl_syntax::value::ValueError> {
        self.native_property_accessor_using(
            recipe,
            writable,
            &mut (),
            &mut |_| {
                self.native_string_bytes(recipe.strings())
                    .map_err(tcl_syntax::raw_string::NativeStringAccessError::Unavailable)
                    .map_err(Into::into)
            },
            &mut |_| Ok(()),
        )
    }
    pub(crate) fn native_property_accessor_checked<C>(
        &self,
        recipe: tcl_registry::native_property_lookup::NativePropertyLookupProtocol,
        writable: bool,
        context: &mut C,
        string: &mut impl FnMut(&mut C) -> Result<super::Rc<[u8]>, tcl_cmd_core::CmdError>,
        check: &mut impl FnMut(&mut C) -> Result<(), tcl_cmd_core::CmdError>,
    ) -> Result<NativeObjectLifetimeLease, tcl_cmd_core::CmdError> {
        self.native_property_accessor_using(recipe, writable, context, string, check)
    }
    fn native_property_accessor_using<C, E: From<tcl_syntax::value::ValueError>>(
        &self,
        recipe: tcl_registry::native_property_lookup::NativePropertyLookupProtocol,
        writable: bool,
        context: &mut C,
        string: &mut impl FnMut(&mut C) -> Result<super::Rc<[u8]>, E>,
        check: &mut impl FnMut(&mut C) -> Result<(), E>,
    ) -> Result<NativeObjectLifetimeLease, E> {
        check(context)?;
        self.check_native_header()?;
        let cached = if let IntRep::NativePropertyName(cache) = &*self.0.intrep.borrow() {
            Some(
                if writable {
                    &cache.writer
                } else {
                    &cache.reader
                }
                .native_lifetime_lease(),
            )
        } else {
            None
        };
        if let Some(cached) = cached {
            check(context)?;
            cached.value().check_native_header()?;
            return Ok(cached);
        }
        let reached = string(context);
        check(context)?;
        let bytes = reached?;
        let (reader, writer) = recipe.member_accessor_names(&bytes);
        let reader = Value::new_native_string_bytes(reader);
        let writer = Value::new_native_string_bytes(writer);
        let materialization = recipe.materialization();
        reader.retain_native_string_representation(materialization)?;
        writer.retain_native_string_representation(materialization)?;
        let lease = if writable { &writer } else { &reader }.native_lifetime_lease();
        let retired = std::mem::replace(&mut *self.0.intrep.borrow_mut(), IntRep::Str);
        drop(retired);
        check(context)?;
        self.check_native_header()?;
        *self.0.intrep.borrow_mut() =
            IntRep::NativePropertyName(NativePropertyName { reader, writer });
        Ok(lease)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_property_name_duplicate_reuses_and_releases_accessor_children() {
        // Native proof: naming.property.original-accessor-name-cache-children

        let dialect = tcl_registry::InvocationDialect::of_profile(
            crate::environment::profile_for_dialect("tcl9.1"),
        );
        let recipe = dialect.native_property_lookup_protocol().unwrap();
        let original = Value::new_native_string_bytes(b"-y\xff\0tail".as_slice());
        let reader = original.native_property_accessor(recipe, false).unwrap();
        let writer = original.native_property_accessor(recipe, true).unwrap();
        assert_eq!(
            reader
                .value()
                .native_string_bytes(recipe.strings())
                .unwrap()
                .as_ref(),
            b"<ReadProp-y\xff>"
        );
        assert_eq!(
            writer
                .value()
                .native_string_bytes(recipe.strings())
                .unwrap()
                .as_ref(),
            b"<WriteProp-y\xff>"
        );
        assert_eq!(reader.value().native_object_reference_count(), 1);
        assert_eq!(writer.value().native_object_reference_count(), 1);
        let copy = original.duplicate_native_object_in(recipe.strings());
        assert_eq!(
            copy.native_property_accessor(recipe, false)
                .unwrap()
                .value()
                .native_object_identity(),
            reader.value().native_object_identity()
        );
        assert_eq!(reader.value().native_object_reference_count(), 2);
        assert_eq!(writer.value().native_object_reference_count(), 2);
        drop(copy);
        assert_eq!(reader.value().native_object_reference_count(), 1);
        assert_eq!(writer.value().native_object_reference_count(), 1);
        original.invalidate_native_string_for_test();
        assert_eq!(
            original
                .native_property_accessor(recipe, false)
                .unwrap()
                .value()
                .native_object_identity(),
            reader.value().native_object_identity()
        );
        assert!(original.native_string_bytes(recipe.strings()).is_err());
        assert!(
            original
                .native_string_bytes(tcl_syntax::native_string::NativeStringProtocol::Jim084)
                .is_err()
        );
    }
}
