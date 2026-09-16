use super::pool::ConstantPool;
use super::ClassFile;
use crate::error::{JavixError, Result};

const CLASS_MAGIC: u32 = 0xCAFEBABE;

pub struct ClassFileBuilder<'a> {
    pub(super) cursor: &'a [u8],
    pub(super) constant_pool: ConstantPool,
}

impl<'a> ClassFileBuilder<'a> {
    pub fn new(cursor: &'a [u8]) -> Self {
        ClassFileBuilder {
            cursor,
            constant_pool: ConstantPool::new(Vec::new()),
        }
    }

    pub fn parse(mut self) -> Result<ClassFile> {
        let magic = self.read_u32()?;
        if magic != CLASS_MAGIC {
            return Err(JavixError::InvalidMagic(magic));
        }

        let minor_version = self.read_u16()?;
        let major_version = self.read_u16()?;

        self.constant_pool = self.parse_constant_pool()?;

        Ok(ClassFile {
            minor_version,
            major_version,
            access_flags: self.read_u16()?,
            this_class: self.read_u16()?,
            super_class: self.read_u16()?,
            interfaces: self.parse_interfaces()?,
            fields: self.parse_members()?,
            methods: self.parse_members()?,
            attributes: self.parse_attributes()?,
            constant_pool: self.constant_pool,
        })
    }
}
