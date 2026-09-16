#[derive(Debug)]
pub struct ConstantPool {
    pub indexes: Vec<CPIndexType>,
}

impl ConstantPool {
    pub fn new(indexes: Vec<CPIndexType>) -> Self {
        ConstantPool { indexes }
    }

    pub fn entry_at(&self, index: u16) -> CPIndexType {
        if index == 0 {
            return CPIndexType::Unusable;
        }
        self.indexes
            .get((index - 1) as usize)
            .cloned()
            .unwrap_or(CPIndexType::Unusable)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum CPIndexType {
    Unusable,
    Class(u16),
    FieldRef {
        class_index: u16,
        name_and_type_index: u16,
    },
    MethodRef {
        class_index: u16,
        name_and_type_index: u16,
    },
    InterfaceMethodRef {
        class_index: u16,
        name_and_type_index: u16,
    },
    String(u16),
    Integer(i32),
    Float(f32),
    Long(i64),
    Double(f64),
    NameAndType {
        name_index: u16,
        descriptor_index: u16,
    },
    Utf8(String),
    MethodHandle {
        reference_kind: u8,
        reference_index: u16,
    },
    MethodType {
        descriptor_index: u16,
    },
    InvokeDynamic {
        bootstrap_method_attr_index: u16,
        name_and_type_index: u16,
    },
}
