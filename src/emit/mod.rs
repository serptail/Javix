mod blocks;
mod cfg;
mod code;
mod constants;
mod decode;
mod descriptor;
mod expr;
mod fields;
mod header;
mod interfaces;
mod methods;
mod opcodes;
mod signature;

pub use super::reader::field::FieldInfo;
pub use super::reader::method::MethodInfo;
pub use super::reader::pool::*;
pub use super::reader::ClassFile;

use std::sync::atomic::{AtomicBool, Ordering};

pub static SHOW_CFG: AtomicBool = AtomicBool::new(false);

pub(crate) fn show_cfg() -> bool {
    SHOW_CFG.load(Ordering::Relaxed)
}

pub fn generate_source(class: &ClassFile) -> String {
    let mut source = String::new();

    if let Some(name) = source_file(class) {
        source.push_str(&format!("// decompiled from {}\n", name));
    }
    source.push_str(&format!(
        "// class file version {}.{}\n\n",
        class.major_version, class.minor_version
    ));

    source.push_str(&header::render_class_header(class));
    source.push_str("{\n");

    let fields = fields::render_fields(class);
    source.push_str(&fields);
    if !fields.is_empty() {
        source.push('\n');
    }

    source.push_str(&methods::render_methods(class));
    source.push_str("}\n");

    source
}

fn source_file(class: &ClassFile) -> Option<String> {
    let bytes = methods::find_attribute(&class.attributes, "SourceFile")?;
    if bytes.len() < 2 {
        return None;
    }
    let index = u16::from_be_bytes([bytes[0], bytes[1]]);
    constants::utf8(&class.constant_pool, index).ok()
}
