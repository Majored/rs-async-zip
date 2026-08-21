// Copyright (c) 2023 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A base runtime-agnostic implementation using `futures`'s IO types.
//! 
//! Please see our [`read1`] (formerly [`read`]) and [`write`] modules for more information. We'd
//! advise familiarising yourself with the high-level concepts and terminology used throughout this
//! crate as described below. We use this terminology to describe common ZIP concepts and structures,
//! and our terminology might differ from other ZIP libraries you've used.
//! 
//! ## High-level concepts
//! A ZIP "archive" is the containing/wrapping file which contains zero or more actual "file"s (entries).
//! 
//! Each file is described in two places in the archive: as a "local file" sat immediately before its own data,
//! and as a "central directory record" within the "central directory" - an index of every file which sits at
//! the end of the archive, terminated by the "end of central directory record".
//! 
//! In short:  
//! ARCHIVE - the overarching/outer ZIP "file"  
//! FILE - a specific file in a ZIP archive  
//! LOCAL FILE - the description of a file which precedes its data  
//! CENTRAL DIRECTORY - the index of all files, at the end of the archive  
//! 
//! ## Abbreviations
//! We use abbreviations to shorten accesses to common fields and for readability of our source code:
//! 
//! LF - Local File  
//! LFH - Local File Header  
//! 
//! CDR - Central Directory Record  
//! CDRH - Central Directory Record Header  
//! 
//! EOCDR - End of Central Directory Record  
//! EOCDRH - End of Central Directory Record Header  
//! CEOCDR - Combined End of Central Directory Record   

pub mod read;
pub mod write;
pub mod read1;
