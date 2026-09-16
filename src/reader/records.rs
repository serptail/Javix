use super::attribute::AttributeInfo;
use super::cursor::ClassFileBuilder;
use super::pool::*;
use super::MemberInfo;
use crate::error::{JavixError, Result};

impl<'a> ClassFileBuilder<'a> {
    pub(super) fn parse_constant_pool(&mut self) -> Result<ConstantPool> {
        let entry_count = self.read_u16()?;
        let mut entries = Vec::with_capacity(entry_count as usize);
        let mut index = 1;

        while index < entry_count {
            let takes_two_slots;

            let entry = match self.read_u8()? {
                1 => {
                    let length = self.read_u16()? as usize;
                    let bytes = self.read_bytes(length)?;
                    takes_two_slots = false;
                    CPIndexType::Utf8(decode_modified_utf8(&bytes)?)
                }
                3 => {
                    takes_two_slots = false;
                    CPIndexType::Integer(self.read_u32()? as i32)
                }
                4 => {
                    takes_two_slots = false;
                    CPIndexType::Float(f32::from_bits(self.read_u32()?))
                }
                5 => {
                    takes_two_slots = true;
                    CPIndexType::Long(self.read_u64()? as i64)
                }
                6 => {
                    takes_two_slots = true;
                    CPIndexType::Double(f64::from_bits(self.read_u64()?))
                }
                7 => {
                    takes_two_slots = false;
                    CPIndexType::Class(self.read_u16()?)
                }
                8 => {
                    takes_two_slots = false;
                    CPIndexType::String(self.read_u16()?)
                }
                9 => {
                    takes_two_slots = false;
                    CPIndexType::FieldRef {
                        class_index: self.read_u16()?,
                        name_and_type_index: self.read_u16()?,
                    }
                }
                10 => {
                    takes_two_slots = false;
                    CPIndexType::MethodRef {
                        class_index: self.read_u16()?,
                        name_and_type_index: self.read_u16()?,
                    }
                }
                11 => {
                    takes_two_slots = false;
                    CPIndexType::InterfaceMethodRef {
                        class_index: self.read_u16()?,
                        name_and_type_index: self.read_u16()?,
                    }
                }
                12 => {
                    takes_two_slots = false;
                    CPIndexType::NameAndType {
                        name_index: self.read_u16()?,
                        descriptor_index: self.read_u16()?,
                    }
                }
                15 => {
                    takes_two_slots = false;
                    CPIndexType::MethodHandle {
                        reference_kind: self.read_u8()?,
                        reference_index: self.read_u16()?,
                    }
                }
                16 => {
                    takes_two_slots = false;
                    CPIndexType::MethodType {
                        descriptor_index: self.read_u16()?,
                    }
                }
                17 | 18 => {
                    takes_two_slots = false;
                    CPIndexType::InvokeDynamic {
                        bootstrap_method_attr_index: self.read_u16()?,
                        name_and_type_index: self.read_u16()?,
                    }
                }
                19 | 20 => {
                    takes_two_slots = false;
                    CPIndexType::Class(self.read_u16()?)
                }
                tag => return Err(JavixError::UnknownConstantTag(tag)),
            };

            entries.push(entry);
            index += 1;

            if takes_two_slots {
                entries.push(CPIndexType::Unusable);
                index += 1;
            }
        }

        Ok(ConstantPool::new(entries))
    }

    pub(super) fn parse_interfaces(&mut self) -> Result<Vec<u16>> {
        let count = self.read_u16()?;
        let mut interfaces = Vec::with_capacity(count as usize);
        for _ in 0..count {
            interfaces.push(self.read_u16()?);
        }
        Ok(interfaces)
    }

    pub(super) fn parse_members<T: MemberInfo>(&mut self) -> Result<Vec<T>> {
        let count = self.read_u16()?;
        let mut members = Vec::with_capacity(count as usize);
        for _ in 0..count {
            members.push(self.parse_member()?);
        }
        Ok(members)
    }

    fn parse_member<T: MemberInfo>(&mut self) -> Result<T> {
        let access_flags = self.read_u16()?;
        let name_index = self.read_u16()?;
        let descriptor_index = self.read_u16()?;
        let attributes = self.parse_attributes()?;

        Ok(T::new(access_flags, name_index, descriptor_index, attributes))
    }

    pub(super) fn parse_attributes(&mut self) -> Result<Vec<AttributeInfo>> {
        let count = self.read_u16()?;
        let mut attributes = Vec::with_capacity(count as usize);
        for _ in 0..count {
            attributes.push(self.parse_attribute()?);
        }
        Ok(attributes)
    }

    fn parse_attribute(&mut self) -> Result<AttributeInfo> {
        let name_index = self.read_u16()?;
        let length = self.read_u32()?;
        let info = self.read_bytes(length as usize)?;

        let name = match self.constant_pool.entry_at(name_index) {
            CPIndexType::Utf8(name) => name,
            _ => format!("#{}", name_index),
        };

        Ok(AttributeInfo { name, info })
    }
}

fn decode_modified_utf8(bytes: &[u8]) -> Result<String> {
    let mut out = String::with_capacity(bytes.len());
    let mut pos = 0;
    let mut high_surrogate: Option<u32> = None;

    while pos < bytes.len() {
        let first = bytes[pos];

        let (code, width) = if first < 0x80 {
            (first as u32, 1)
        } else if first & 0xE0 == 0xC0 {
            if pos + 1 >= bytes.len() {
                return Err(JavixError::InvalidUtf8);
            }
            let second = bytes[pos + 1] as u32;
            ((((first & 0x1F) as u32) << 6) | (second & 0x3F), 2)
        } else if first & 0xF0 == 0xE0 {
            if pos + 2 >= bytes.len() {
                return Err(JavixError::InvalidUtf8);
            }
            let second = bytes[pos + 1] as u32;
            let third = bytes[pos + 2] as u32;
            (
                (((first & 0x0F) as u32) << 12) | ((second & 0x3F) << 6) | (third & 0x3F),
                3,
            )
        } else {
            return Err(JavixError::InvalidUtf8);
        };

        pos += width;

        if (0xD800..0xDC00).contains(&code) {
            if high_surrogate.is_some() {
                return Err(JavixError::InvalidUtf8);
            }
            high_surrogate = Some(code);
            continue;
        }

        if (0xDC00..0xE000).contains(&code) {
            let high = high_surrogate.take().ok_or(JavixError::InvalidUtf8)?;
            let combined = 0x10000 + ((high - 0xD800) << 10) + (code - 0xDC00);
            out.push(char::from_u32(combined).ok_or(JavixError::InvalidUtf8)?);
            continue;
        }

        if high_surrogate.is_some() {
            return Err(JavixError::InvalidUtf8);
        }

        out.push(char::from_u32(code).ok_or(JavixError::InvalidUtf8)?);
    }

    if high_surrogate.is_some() {
        return Err(JavixError::InvalidUtf8);
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::decode_modified_utf8;

    #[test]
    fn plain_ascii() {
        assert_eq!(decode_modified_utf8(b"main").unwrap(), "main");
    }

    #[test]
    fn encoded_nul() {
        assert_eq!(decode_modified_utf8(&[0xC0, 0x80]).unwrap(), "\u{0}");
    }

    #[test]
    fn surrogate_pair() {
        let bytes = [0xED, 0xA0, 0xB4, 0xED, 0xB4, 0x9E];
        assert_eq!(decode_modified_utf8(&bytes).unwrap(), "\u{1D11E}");
    }
}
