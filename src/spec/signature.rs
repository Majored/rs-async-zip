// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

// https://github.com/Majored/rs-async-zip/blob/main/src/spec/APPNOTE.md#437
pub const LOCAL_FILE_HEADER: u32 = 0x4034b50;

// https://github.com/Majored/rs-async-zip/blob/main/src/spec/APPNOTE.md#4312
pub const CENTRAL_DIRECTORY_FILE_HEADER: u32 = 0x2014b50;

// https://github.com/Majored/rs-async-zip/blob/main/src/spec/APPNOTE.md#439
pub const DATA_DESCRIPTOR: u32 = 0x8074b50;

// https://github.com/Majored/rs-async-zip/blob/main/src/spec/APPNOTE.md#4316
pub const END_OF_CENTRAL_DIRECTORY: u32 = 0x6054b50;
