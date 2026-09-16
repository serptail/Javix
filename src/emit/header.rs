use super::constants as cp;
use super::signature::simple_name;
use super::ClassFile;

const ACC_PUBLIC: u16 = 0x0001;
const ACC_FINAL: u16 = 0x0010;
const ACC_INTERFACE: u16 = 0x0200;
const ACC_ABSTRACT: u16 = 0x0400;
const ACC_SYNTHETIC: u16 = 0x1000;
const ACC_ANNOTATION: u16 = 0x2000;
const ACC_ENUM: u16 = 0x4000;

pub fn render_class_header(class: &ClassFile) -> String {
    let pool = &class.constant_pool;
    let flags = class.access_flags;
    let mut header = String::new();

    if flags & ACC_PUBLIC != 0 {
        header.push_str("public ");
    }
    if flags & ACC_SYNTHETIC != 0 {
        header.push_str("/* synthetic */ ");
    }

    let is_interface = flags & ACC_INTERFACE != 0;

    if is_interface {
        if flags & ACC_ANNOTATION != 0 {
            header.push('@');
        }
        header.push_str("interface ");
    } else if flags & ACC_ENUM != 0 {
        header.push_str("enum ");
    } else {
        if flags & ACC_ABSTRACT != 0 {
            header.push_str("abstract ");
        }
        if flags & ACC_FINAL != 0 {
            header.push_str("final ");
        }
        header.push_str("class ");
    }

    let own_name = cp::class_name(pool, class.this_class).unwrap_or_else(|_| "Unknown".to_string());
    header.push_str(simple_name(&own_name));

    if let Ok(super_name) = cp::class_name(pool, class.super_class) {
        if super_name != "java.lang.Object" && !is_interface {
            header.push_str(" extends ");
            header.push_str(&super_name);
        }
    }

    let interfaces = super::interfaces::resolve_interfaces(class);
    if !interfaces.is_empty() {
        header.push_str(if is_interface {
            " extends "
        } else {
            " implements "
        });
        header.push_str(&interfaces.join(", "));
    }

    header.push(' ');
    header
}
