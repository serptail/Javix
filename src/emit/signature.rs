use super::constants as cp;
use super::methods::{find_attribute, MethodRenderer};

const ACC_PUBLIC: u16 = 0x0001;
const ACC_PRIVATE: u16 = 0x0002;
const ACC_PROTECTED: u16 = 0x0004;
const ACC_STATIC: u16 = 0x0008;
const ACC_FINAL: u16 = 0x0010;
const ACC_SYNCHRONIZED: u16 = 0x0020;
const ACC_BRIDGE: u16 = 0x0040;
const ACC_VARARGS: u16 = 0x0080;
const ACC_NATIVE: u16 = 0x0100;
const ACC_ABSTRACT: u16 = 0x0400;
const ACC_STRICT: u16 = 0x0800;
const ACC_SYNTHETIC: u16 = 0x1000;

impl<'a> MethodRenderer<'a> {
    pub(super) fn build_signature(&mut self) -> String {
        let flags = self.info.access_flags;
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
        if flags & ACC_ABSTRACT != 0 {
            sig.push_str("abstract ");
            self.should_render_body = false;
        }
        if flags & ACC_FINAL != 0 {
            sig.push_str("final ");
        }
        if flags & ACC_SYNCHRONIZED != 0 {
            sig.push_str("synchronized ");
        }
        if flags & ACC_BRIDGE != 0 {
            sig.push_str("/* bridge */ ");
        }
        if flags & ACC_VARARGS != 0 {
            self.is_varargs = true;
        }
        if flags & ACC_STRICT != 0 {
            sig.push_str("/* strict */ ");
        }
        if flags & ACC_NATIVE != 0 {
            sig.push_str("native ");
            self.should_render_body = false;
        }

        let name = self.utf8_at(self.info.name_index);

        if name == "<clinit>" {
            return "static".to_string();
        }

        let descriptor = self.utf8_at(self.info.descriptor_index);
        let return_type = match super::descriptor::parse_method(&descriptor) {
            Some((_, return_type)) => return_type,
            None => {
                sig.push_str(&format!("/* unreadable descriptor {} */", descriptor));
                return sig;
            }
        };

        if name == "<init>" {
            sig.push_str(simple_name(&self.ctx.owner));
        } else {
            sig.push_str(&return_type.to_string());
            sig.push(' ');
            sig.push_str(&name);
        }

        sig.push('(');
        sig.push_str(&self.render_args());
        sig.push(')');

        if let Some(exceptions) = find_attribute(&self.info.attributes, "Exceptions") {
            sig.push_str(&self.render_throws(exceptions));
        }

        sig
    }

    fn render_args(&self) -> String {
        let joined = self
            .ctx
            .params
            .iter()
            .map(|(slot, ty)| format!("{} {}", ty, self.ctx.param_name(*slot)))
            .collect::<Vec<_>>()
            .join(", ");

        if !self.is_varargs {
            return joined;
        }

        match joined.rfind("[]") {
            Some(position) => {
                let (head, tail) = joined.split_at(position);
                format!("{}{}", head, tail.replacen("[]", "...", 1))
            }
            None => joined,
        }
    }

    fn render_throws(&self, exceptions: &[u8]) -> String {
        if exceptions.len() < 2 {
            return String::new();
        }

        let count = u16::from_be_bytes([exceptions[0], exceptions[1]]) as usize;
        let mut names = Vec::with_capacity(count);

        for i in 0..count {
            let offset = 2 * (i + 1);
            if offset + 1 >= exceptions.len() {
                break;
            }
            let index = u16::from_be_bytes([exceptions[offset], exceptions[offset + 1]]);
            if let Ok(name) = cp::class_name(self.pool, index) {
                names.push(name);
            }
        }

        if names.is_empty() {
            String::new()
        } else {
            format!(" throws {}", names.join(", "))
        }
    }
}

pub(super) fn simple_name(dotted: &str) -> &str {
    dotted.rsplit('.').next().unwrap_or(dotted)
}
