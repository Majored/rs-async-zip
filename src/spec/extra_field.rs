// Copyright Cognite AS, 2023

use crate::error::{Result as ZipResult, ZipError};
use crate::spec::header::{ExtraField, HeaderId, UnknownExtraField, Zip64ExtendedInformationExtraField};

impl From<u16> for HeaderId {
    fn from(value: u16) -> Self {
        match value {
            0x0001 => Self::Zip64ExtendedInformationExtraField,
            other => Self::Other(other),
        }
    }
}

impl From<HeaderId> for u16 {
    fn from(value: HeaderId) -> Self {
        match value {
            HeaderId::Zip64ExtendedInformationExtraField => 0x0001,
            HeaderId::Other(other) => other,
        }
    }
}

pub(crate) trait ExtraFieldAsBytes {
    fn as_bytes(&self) -> Vec<u8>;

    fn count_bytes(&self) -> usize;
}

impl ExtraFieldAsBytes for &[ExtraField] {
    fn as_bytes(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        for field in self.iter() {
            buffer.append(&mut field.as_bytes());
        }
        buffer
    }

    fn count_bytes(&self) -> usize {
        self.iter().map(|field| field.count_bytes()).sum()
    }
}

impl ExtraFieldAsBytes for ExtraField {
    fn as_bytes(&self) -> Vec<u8> {
        match self {
            ExtraField::Zip64ExtendedInformationExtraField(field) => field.as_bytes(),
            ExtraField::UnknownExtraField(field) => field.as_bytes(),
        }
    }

    fn count_bytes(&self) -> usize {
        match self {
            ExtraField::Zip64ExtendedInformationExtraField(field) => field.count_bytes(),
            ExtraField::UnknownExtraField(field) => field.count_bytes(),
        }
    }
}

impl ExtraFieldAsBytes for UnknownExtraField {
    fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let header_id: u16 = self.header_id.into();
        bytes.append(&mut header_id.to_le_bytes().to_vec());
        bytes.append(&mut self.data_size.to_le_bytes().to_vec());
        bytes.append(&mut self.content.clone());

        bytes
    }

    fn count_bytes(&self) -> usize {
        4 + self.content.len()
    }
}

impl ExtraFieldAsBytes for Zip64ExtendedInformationExtraField {
    fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let header_id: u16 = self.header_id.into();
        bytes.append(&mut header_id.to_le_bytes().to_vec());
        bytes.append(&mut self.data_size.to_le_bytes().to_vec());
        bytes.append(&mut self.uncompressed_size.to_le_bytes().to_vec());
        bytes.append(&mut self.compressed_size.to_le_bytes().to_vec());
        if let Some(relative_header_offset) = &self.relative_header_offset {
            bytes.append(&mut relative_header_offset.to_le_bytes().to_vec());
        }
        if let Some(disk_start_number) = &self.disk_start_number {
            bytes.append(&mut disk_start_number.to_le_bytes().to_vec());
        }

        bytes
    }

    fn count_bytes(&self) -> usize {
        20 + self.relative_header_offset.map(|_| 8).unwrap_or_default()
            + self.disk_start_number.map(|_| 8).unwrap_or_default()
    }
}

/// Parse a zip64 extra field from bytes.
/// The content of "data" should exclude the header.
fn zip64_extended_information_field_from_bytes(
    header_id: HeaderId,
    data_size: u16,
    data: &[u8],
) -> ZipResult<Zip64ExtendedInformationExtraField> {
    // First ensure that the data is sufficient to populate compressed & uncompressed size.
    if data.len() < 16 {
        return Err(ZipError::Zip64ExtendedFieldIncomplete);
    }
    let uncompressed_size = u64::from_le_bytes(data[0..8].try_into().unwrap());
    let compressed_size = u64::from_le_bytes(data[8..16].try_into().unwrap());
    let relative_header_offset =
        if data.len() >= 24 { Some(u64::from_le_bytes(data[16..24].try_into().unwrap())) } else { None };
    let disk_start_number =
        if data.len() >= 32 { Some(u64::from_le_bytes(data[24..32].try_into().unwrap())) } else { None };

    Ok(Zip64ExtendedInformationExtraField {
        header_id,
        data_size,
        uncompressed_size,
        compressed_size,
        relative_header_offset,
        disk_start_number,
    })
}

pub(crate) fn extra_field_from_bytes(header_id: HeaderId, data_size: u16, data: &[u8]) -> ZipResult<ExtraField> {
    match header_id {
        HeaderId::Zip64ExtendedInformationExtraField => Ok(ExtraField::Zip64ExtendedInformationExtraField(
            zip64_extended_information_field_from_bytes(header_id, data_size, data)?,
        )),
        header_id @ HeaderId::Other(_) => {
            Ok(ExtraField::UnknownExtraField(UnknownExtraField { header_id, data_size, content: data.to_vec() }))
        }
    }
}
