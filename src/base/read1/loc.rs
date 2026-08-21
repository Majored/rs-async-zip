// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use futures_lite::AsyncBufRead;
use futures_lite::AsyncBufReadExt;
use futures_lite::AsyncReadExt;
use futures_lite::AsyncSeek;
use futures_lite::io::SeekFrom;
use futures_lite::AsyncSeekExt;

use crate::error::Result;
use crate::error::ZipError;
use crate::spec::headers1::EOCDRH;
use crate::spec::headers1::Signature;
use crate::spec::KnownSize;

const EOCDR_FIXED_SIZE: u64 = (Signature::SIZE + EOCDRH::SIZE) as u64;
const EOCDR_FURTHEST_BACK: u64 = EOCDR_FIXED_SIZE + u16::MAX as u64;
const EOCDR_COMM_LENGTH_OFFSET: u64 = EOCDR_FIXED_SIZE - 2;

pub(crate) struct Method0<R> {
    reader: R,
}

impl<R: AsyncBufRead + AsyncSeek + Unpin> Method0<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    /// Locates the EOCDR and returns the offset at which it begins.
    pub async fn locate(&mut self, eor: u64) -> Result<u64> {
        if eor < EOCDR_FIXED_SIZE {
            return Err(ZipError::UnableToLocateEOCDR);
        }

        let mut offset = eor.saturating_sub(EOCDR_FURTHEST_BACK);
        self.reader.seek(SeekFrom::Start(offset)).await?;

        let signature = u32::from(Signature::EOCDRH).to_le_bytes();
        let mut matcher = Matcher { signature, matched: 0, candidates: Vec::new() };
        
        loop {
            let buffer = self.reader.fill_buf().await?;

            if buffer.is_empty() {
                break;
            }

            matcher.next_buffer(buffer, offset);

            let consumed = buffer.len();
            self.reader.consume(consumed);
            offset += consumed as u64;
        }

        // A signature is only a candidate - the same four bytes can appear within a file comment
        // (or within stored file data, if the archive declares a comment which swallows it). The
        // last candidate is the EOCDR unless a comment hides a later one, so where there's any
        // ambiguity, we take the last whose declared comment length runs exactly to EOF.
        if matcher.candidates.len() > 1 {
            for candidate in matcher.candidates.iter().rev() {
                if self.comment_reaches_eof(*candidate, eor).await? {
                    return Ok(*candidate + Signature::SIZE as u64);
                }
            }
        }

        // No candidate agreeing with EOF means a malformed archive whichever one we hand back, so
        // we hand back the last and let the record's parsing and validation describe how.
        matcher.candidates.last().copied().ok_or(ZipError::UnableToLocateEOCDR)
    }

    /// Returns whether the file comment declared by the EOCDR candidate at `offset` ends at EOF.
    async fn comment_reaches_eof(&mut self, offset: u64, length: u64) -> Result<bool> {
        self.reader.seek(SeekFrom::Start(offset + EOCDR_COMM_LENGTH_OFFSET)).await?;

        let mut buffer = [0; 2];
        self.reader.read_exact(&mut buffer).await?;

        Ok(offset + EOCDR_FIXED_SIZE + u16::from_le_bytes(buffer) as u64 == length)
    }
}

struct Matcher {
    signature: [u8; 4],
    matched: usize,
    candidates: Vec<u64>,
}

impl Matcher {
    /// Handles a buffer of bytes, updating the match state and recording any candidates found.
    pub fn next_buffer(&mut self, buffer: &[u8], starting: u64) {
        for (index, byte) in buffer.iter().enumerate() {
            if self.next_byte(*byte) {
                self.candidates.push(starting + index as u64 + 1 - Signature::SIZE as u64);
            }
        }
    }

    /// Returns whether the curent byte resulted in a full match.
    pub fn next_byte(&mut self, byte: u8) -> bool {
        if byte == self.signature[self.matched] {
            self.matched += 1;

            if self.matched == self.signature.len() {
                self.matched = 0;
                return true;
            }

            return false;
        }

        // Restart the match, since it could still match the first byte of the signature.
        if byte == self.signature[0] {
            self.matched = 1;
            return false;
        }

        self.matched = 0;
        false
    }
}
