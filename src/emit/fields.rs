use super::constants as cp;
use super::descriptor;
use super::methods::find_attribute;
use super::{ClassFile, ConstantPool, FieldInfo};

const ACC_PUBLIC: u16 = 0x0001;
const ACC_PRIVATE: u16 = 0x0002;
const ACC_PROTECTED: u16 = 0x0004;
const ACC_STATIC: u16 = 0x0008;
const ACC_FINAL: u16 = 0x0010;
const ACC_VOLATILE: u16 = 0x0040;
const ACC_TRANSIENT: u16 = 0x0080;
const ACC_SYNTHETIC: u16 = 0x1000;

fn build_field_signature(field: &FieldInfo, pool: &ConstantPool) -> String {
    let flags = field.access_flags;
    let mut sig = String::new();

    if flags & ACC_PUBLIC != 0 {
        sig.push_str("public ");
    } else if flags & ACC_PRIVATE != 0 {
        sig.push_str("private ");
    } else if flags & ACC_PROTECTED != 0 {
        sig.push_str("protected ");
    }

    if flags & ACC_SYNTHETIC != 0 {
        sig.push_str("/* synthetic */ ");
    }
    if flags & ACC_STATIC != 0 {
        sig.push_str("static ");
    }
    if flags & ACC_FINAL != 0 {
        sig.push_str("final ");
    }
    if flags & ACC_VOLATILE != 0 {
        sig.push_str("volatile ");
    }
    if flags & ACC_TRANSIENT != 0 {
        sig.push_str("transient ");
    }

    let descriptor_text = cp::utf8(pool, field.descriptor_index).unwrap_or_default();
    match descriptor::parse_field(&descriptor_text) {
        Some(ty) => sig.push_str(&ty.to_string()),
        None => sig.push_str(&format!("/* unreadable descriptor {} */", descriptor_text)),
    }

    sig.push(' ');
    sig.push_str(&cp::utf8(pool, field.name_index).unwrap_or_default());

    if let Some(bytes) = find_attribute(&field.attributes, "ConstantValue") {
        if bytes.len() >= 2 {
            let index = u16::from_be_bytes([bytes[0], bytes[1]]);
            if let Ok(value) = cp::constant(pool, index) {
                sig.push_str(&format!(" = {}", value));
            }
        }
    }

    sig
}

pub fn render_fields(class: &ClassFile) -> String {
    let mut rendered = String::new();

    for field in &class.fields {
        rendered.push('\t');
        rendered.push_str(&build_field_signature(field, &class.constant_pool));
        rendered.push_str(";\n");
    }

    rendered
}
