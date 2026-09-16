#[derive(Clone, PartialEq)]
pub enum Type {
    Byte,
    Char,
    Double,
    Float,
    Int,
    Long,
    Class(String),
    Short,
    Boolean,
    Array(Box<Type>),
    Void,
    Unknown,
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Type::Byte => "byte",
            Type::Char => "char",
            Type::Double => "double",
            Type::Float => "float",
            Type::Int => "int",
            Type::Long => "long",
            Type::Class(name) => name,
            Type::Short => "short",
            Type::Boolean => "boolean",
            Type::Array(element) => {
                let element: &Type = element.as_ref();
                return write!(f, "{}[]", element);
            }
            Type::Void => "void",
            Type::Unknown => "Object",
        })
    }
}
