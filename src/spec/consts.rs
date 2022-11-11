// Copyright (c) 2022 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

pub const SIGNATURE_LENGTH: usize = 4;

// Local file header constants
// 
// https://github.com/Majored/rs-async-zip/blob/main/SPECIFICATION.md#437
pub const LFH_SIGNATURE: u32 = 0x4034b50;
pub const LFH_LENGTH: usize = 26;

// Central directory header constants
// 
// https://github.com/Majored/rs-async-zip/blob/main/SPECIFICATION.md#4312
pub const CDH_SIGNATURE: u32 = 0x2014b50;
pub const CDH_LENGTH: usize = 42;

// End of central directory record constants
// 
// https://github.com/Majored/rs-async-zip/blob/main/SPECIFICATION.md#4316
pub const EOCDR_SIGNATURE: u32 = 0x6054b50;
pub const EOCDR_LENGTH: usize = 18;

// https://github.com/Majored/rs-async-zip/blob/main/SPECIFICATION.md#439
pub const DATA_DESCRIPTOR_SIGNATURE: u32 = 0x8074b50;