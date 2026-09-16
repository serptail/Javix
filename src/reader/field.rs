use super::attribute::AttributeInfo;

#[derive(Debug)]
pub struct FieldInfo {
    pub access_flags: u16,
    pub name_index: u16,
    pub descriptor_index: u16,
    pub attributes: Vec<AttributeInfo>,
}

impl super::MemberInfo for FieldInfo {
    fn new(
        access_flags: u16,
        name_index: u16,
        descriptor_index: u16,
        attributes: Vec<AttributeInfo>,
    ) -> Self {
        FieldInfo {
            access_flags,
            name_index,
            descriptor_index,
            attributes,
        }
    }
}
