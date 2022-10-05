use crate::spec::attribute::AttributeCompatibility;
use crate::spec::entry::{Entry, EntryBuilder};

pub trait EntryExt: Sized {
    fn unix_permission(&self) -> Option<u16>;
}

impl EntryExt for Entry {
    fn unix_permission(&self) -> Option<u16> {
        match self.attribute_compatibility {
            AttributeCompatibility::Unix => Some(((self.external_file_attribute) >> 16) as u16),
            _ => None,
        }
    }
}

pub trait EntryBuilderExt {
    fn unix_permission(self, mode: u16) -> Self;
}

impl EntryBuilderExt for EntryBuilder {
    fn unix_permission(mut self, mode: u16) -> Self {
        self.attribute_compatibility = Some(AttributeCompatibility::Unix);
        
        let attribute_mode_only = (mode as u32) << 16;
        if let Some(attribute) = self.external_file_attribute.as_mut() {
            // Zero out the upper sixteen bits and replace them with the provided mode.
            *attribute = (*attribute & 0xFFFF) | attribute_mode_only;
        } else {
            self.external_file_attribute = Some(attribute_mode_only);
        }

        self
    }
}