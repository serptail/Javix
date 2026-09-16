use super::attribute::AttributeInfo;

#[derive(Debug)]
pub struct MethodInfo {
    pub access_flags: u16,
    pub name_index: u16,
    pub descriptor_index: u16,
    pub attributes: Vec<AttributeInfo>,
}

impl super::MemberInfo for MethodInfo {
    fn new(
        access_flags: u16,
        name_index: u16,
        descriptor_index: u16,
        attributes: Vec<AttributeInfo>,
    ) -> Self {
        MethodInfo {
            access_flags,
            name_index,
            descriptor_index,
            attributes,
        }
    }
}
