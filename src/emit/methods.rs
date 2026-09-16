use super::constants as cp;
use super::expr::interp::MethodContext;
use super::{ClassFile, ConstantPool, MethodInfo};
use crate::reader::attribute::AttributeInfo;

const ACC_STATIC: u16 = 0x0008;

pub fn render_methods(class: &ClassFile) -> String {
    let mut rendered = String::new();

    for (position, info) in class.methods.iter().enumerate() {
        if position > 0 {
            rendered.push('\n');
        }

        match MethodRenderer::new(class, info) {
            Ok(mut method) => rendered.push_str(&method.render()),
            Err(reason) => {
                rendered.push_str(&format!("\t// skipped a method: {}\n", reason));
            }
        }
    }

    rendered
}

pub(super) fn find_attribute<'a>(
    attributes: &'a [AttributeInfo],
    name: &str,
) -> Option<&'a Vec<u8>> {
    attributes
        .iter()
        .find(|attribute| attribute.name == name)
        .map(|attribute| &attribute.info)
}

pub(super) struct MethodRenderer<'a> {
    pub(super) should_render_body: bool,
    pub(super) is_varargs: bool,
    pub(super) info: &'a MethodInfo,
    pub(super) signature: String,
    pub(super) ctx: MethodContext,
    pub(super) pool: &'a ConstantPool,
}

impl<'a> MethodRenderer<'a> {
    fn new(class: &'a ClassFile, info: &'a MethodInfo) -> Result<Self, String> {
        let pool = &class.constant_pool;
        let descriptor = cp::utf8(pool, info.descriptor_index)?;
        let owner = cp::class_name(pool, class.this_class)?;
        let is_static = info.access_flags & ACC_STATIC != 0;
        let named = super::code::local_variable_names(info, pool);
        let ctx = MethodContext::new(is_static, owner, &descriptor, &named)?;

        Ok(MethodRenderer {
            should_render_body: true,
            is_varargs: false,
            info,
            signature: String::new(),
            ctx,
            pool,
        })
    }

    fn render(&mut self) -> String {
        let mut out = String::from('\t');
        out.push_str(&self.signature());

        if !self.should_render_body {
            out.push_str(";\n");
            return out;
        }

        out.push_str(" {\n");

        for line in self.render_body().lines() {
            if line.is_empty() {
                out.push('\n');
                continue;
            }
            out.push_str("\t\t");
            out.push_str(line);
            out.push('\n');
        }

        out.push_str("\t}\n");
        out
    }

    fn signature(&mut self) -> String {
        if self.signature.is_empty() {
            self.signature = self.build_signature();
        }
        self.signature.clone()
    }

    pub(super) fn utf8_at(&self, index: u16) -> String {
        cp::utf8(self.pool, index).unwrap_or_else(|_| format!("#{}", index))
    }
}
