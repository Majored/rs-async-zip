// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A set of primitive types used to read and write ZIP archives. 

pub(crate) mod attribute;
pub(crate) mod compression;
pub(crate) mod consts;
pub(crate) mod extra_field;
pub(crate) mod header;
pub(crate) mod parse;
pub(crate) mod version;
pub mod headers1;
pub mod constructs;
pub mod extra;

pub use compression::Compression;

/// An accessor trait for types that have a fixed size in bytes, such as headers.
/// 
/// We cannot use `std::mem::size_of::<T>()` because the types are not packed.
pub trait KnownSize {
    const SIZE: usize;
}
