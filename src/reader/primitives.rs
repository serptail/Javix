use super::cursor::ClassFileBuilder;
use crate::error::{JavixError, Result};

use std::io::Read;

impl<'a> ClassFileBuilder<'a> {
    pub(super) fn read_u8(&mut self) -> Result<u8> {
        let mut byte = [0u8; 1];
        self.cursor
            .read_exact(&mut byte)
            .map_err(|_| JavixError::UnexpectedEof)?;
        Ok(byte[0])
    }

    pub(super) fn read_u16(&mut self) -> Result<u16> {
        let mut bytes = [0u8; 2];
        self.cursor
            .read_exact(&mut bytes)
            .map_err(|_| JavixError::UnexpectedEof)?;
        Ok(u16::from_be_bytes(bytes))
    }

    pub(super) fn read_u32(&mut self) -> Result<u32> {
        let mut bytes = [0u8; 4];
        self.cursor
            .read_exact(&mut bytes)
            .map_err(|_| JavixError::UnexpectedEof)?;
        Ok(u32::from_be_bytes(bytes))
    }

    pub(super) fn read_u64(&mut self) -> Result<u64> {
        let mut bytes = [0u8; 8];
        self.cursor
            .read_exact(&mut bytes)
            .map_err(|_| JavixError::UnexpectedEof)?;
        Ok(u64::from_be_bytes(bytes))
    }

    pub(super) fn read_bytes(&mut self, count: usize) -> Result<Vec<u8>> {
        let mut bytes = vec![0u8; count];
        self.cursor
            .read_exact(&mut bytes)
            .map_err(|_| JavixError::UnexpectedEof)?;
        Ok(bytes)
    }
}
