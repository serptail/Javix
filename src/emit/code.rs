use std::collections::HashMap;

use super::constants as cp;
use super::descriptor;
use super::expr::kind::Type;
use super::methods::{find_attribute, MethodRenderer};
use super::ConstantPool;
use crate::reader::method::MethodInfo;

type Res<T> = Result<T, String>;

pub(super) struct CodeAttribute {
    pub code: Vec<u8>,
    pub has_handlers: bool,
    pub attributes: Vec<(String, Vec<u8>)>,
}

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, count: usize) -> Res<&'a [u8]> {
        let end = self.pos + count;
        if end > self.bytes.len() {
            return Err(format!(
                "Code attribute ends early: wanted {} bytes at offset {}",
                count, self.pos
            ));
        }
        let slice = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    fn u16(&mut self) -> Res<u16> {
        let bytes = self.take(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn u32(&mut self) -> Res<u32> {
        let bytes = self.take(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }
}

pub(super) fn parse_code(bytes: &[u8], pool: &ConstantPool) -> Res<CodeAttribute> {
    let mut cursor = Cursor { bytes, pos: 0 };

    let _max_stack = cursor.u16()?;
    let _max_locals = cursor.u16()?;

    let code_length = cursor.u32()? as usize;
    let code = cursor.take(code_length)?.to_vec();

    let handler_count = cursor.u16()? as usize;
    cursor.take(handler_count * 8)?;

    let attribute_count = cursor.u16()? as usize;
    let mut attributes = Vec::with_capacity(attribute_count);

    for _ in 0..attribute_count {
        let name_index = cursor.u16()?;
        let length = cursor.u32()? as usize;
        let info = cursor.take(length)?.to_vec();
        let name = cp::utf8(pool, name_index).unwrap_or_else(|_| format!("#{}", name_index));
        attributes.push((name, info));
    }

    Ok(CodeAttribute {
        code,
        has_handlers: handler_count > 0,
        attributes,
    })
}

pub(super) fn local_variable_names(
    info: &MethodInfo,
    pool: &ConstantPool,
) -> HashMap<u16, (String, Type)> {
    let mut names = HashMap::new();

    let attribute = match find_attribute(&info.attributes, "Code") {
        Some(bytes) => bytes,
        None => return names,
    };

    let parsed = match parse_code(attribute, pool) {
        Ok(parsed) => parsed,
        Err(_) => return names,
    };

    let table = match parsed
        .attributes
        .iter()
        .find(|(name, _)| name == "LocalVariableTable")
    {
        Some((_, bytes)) => bytes,
        None => return names,
    };

    let mut cursor = Cursor {
        bytes: table,
        pos: 0,
    };

    let count = match cursor.u16() {
        Ok(count) => count,
        Err(_) => return names,
    };

    for _ in 0..count {
        let entry = match (
            cursor.u16(),
            cursor.u16(),
            cursor.u16(),
            cursor.u16(),
            cursor.u16(),
        ) {
            (Ok(_), Ok(_), Ok(name_index), Ok(descriptor_index), Ok(slot)) => {
                (name_index, descriptor_index, slot)
            }
            _ => break,
        };

        let (name_index, descriptor_index, slot) = entry;

        let name = match cp::utf8(pool, name_index) {
            Ok(name) => name,
            Err(_) => continue,
        };
        let ty = match cp::utf8(pool, descriptor_index)
            .ok()
            .and_then(|text| descriptor::parse_field(&text))
        {
            Some(ty) => ty,
            None => continue,
        };

        names.entry(slot).or_insert((name, ty));
    }

    names
}

impl<'a> MethodRenderer<'a> {
    pub(super) fn render_body(&mut self) -> String {
        let attribute = match find_attribute(&self.info.attributes, "Code") {
            Some(bytes) => bytes,
            None => return "// no Code attribute\n".to_string(),
        };

        let parsed = match parse_code(attribute, self.pool) {
            Ok(parsed) => parsed,
            Err(reason) => return format!("// {}\n", reason),
        };

        let tagged = match super::decode::decode_with_offsets(&parsed.code) {
            Ok(tagged) => tagged,
            Err(reason) => return format!("// could not decode bytecode: {}\n", reason),
        };

        let mut out = String::new();

        if super::show_cfg() {
            let name = cp::utf8(self.pool, self.info.name_index)
                .unwrap_or_else(|_| "method".to_string());
            out.push_str(&super::cfg::render_comment(&name, &tagged));
        }

        if parsed.has_handlers {
            out.push_str(&super::expr::structure::listing(
                &tagged,
                "method has exception handlers (try/catch is not reconstructed yet)",
            ));
            return out;
        }

        out.push_str(&super::expr::structure::emit_body(
            &tagged, self.pool, &self.ctx,
        ));

        out
    }
}
