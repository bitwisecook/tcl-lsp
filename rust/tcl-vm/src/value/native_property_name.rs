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
    pub(crate) fn native_property_accessor(
        &self,
        recipe: tcl_registry::native_property_lookup::NativePropertyLookupProtocol,
        writable: bool,
    ) -> Result<NativeObjectLifetimeLease, tcl_syntax::value::ValueError> {
        self.check_native_header()?;
        if let IntRep::NativePropertyName(cache) = &*self.0.intrep.borrow() {
            return Ok(if writable {
                &cache.writer
            } else {
                &cache.reader
            }
            .native_lifetime_lease());
        }
        let bytes = self
            .native_string_bytes(recipe.strings())
            .map_err(tcl_syntax::raw_string::NativeStringAccessError::Unavailable)?;
        let (reader, writer) = recipe.accessor_names(&bytes);
        let reader = Value::new_native_string_bytes(reader);
        let writer = Value::new_native_string_bytes(writer);
        let materialization = recipe.materialization();
        reader.retain_native_string_representation(materialization)?;
        writer.retain_native_string_representation(materialization)?;
        let lease = if writable { &writer } else { &reader }.native_lifetime_lease();
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
