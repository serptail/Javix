pub mod attribute;
pub mod cursor;
pub mod field;
pub mod method;
pub mod pool;
mod primitives;
mod records;

pub trait MemberInfo {
    fn new(
        access_flags: u16,
        name_index: u16,
        descriptor_index: u16,
        attributes: Vec<attribute::AttributeInfo>,
    ) -> Self;
}

#[derive(Debug)]
pub struct ClassFile {
    pub minor_version: u16,
    pub major_version: u16,
    pub constant_pool: pool::ConstantPool,
    pub access_flags: u16,
    pub this_class: u16,
    pub super_class: u16,
    pub interfaces: Vec<u16>,
    pub fields: Vec<field::FieldInfo>,
    pub methods: Vec<method::MethodInfo>,
    pub attributes: Vec<attribute::AttributeInfo>,
}

impl ClassFile {
    pub fn new(bytes: &[u8]) -> crate::error::Result<ClassFile> {
        cursor::ClassFileBuilder::new(bytes).parse()
    }
}
