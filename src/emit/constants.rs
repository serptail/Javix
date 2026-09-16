use super::descriptor;
use super::expr::ast::Expr;
use super::expr::kind::Type;
use super::{CPIndexType, ConstantPool};

pub type Res<T> = Result<T, String>;

pub fn entry(pool: &ConstantPool, index: u16) -> Res<CPIndexType> {
    match pool.entry_at(index) {
        CPIndexType::Unusable => Err(format!("constant pool #{} is empty or out of range", index)),
        other => Ok(other),
    }
}

pub fn utf8(pool: &ConstantPool, index: u16) -> Res<String> {
    match entry(pool, index)? {
        CPIndexType::Utf8(text) => Ok(text),
        other => Err(format!("#{} is not a Utf8 entry: {:?}", index, other)),
    }
}

pub fn internal_class_name(pool: &ConstantPool, index: u16) -> Res<String> {
    match entry(pool, index)? {
        CPIndexType::Class(name_index) => utf8(pool, name_index),
        other => Err(format!("#{} is not a Class entry: {:?}", index, other)),
    }
}

pub fn class_name(pool: &ConstantPool, index: u16) -> Res<String> {
    Ok(descriptor::class_name(&internal_class_name(pool, index)?))
}

pub struct MemberRef {
    pub class: String,
    pub name: String,
    pub descriptor: String,
}

pub fn member_ref(pool: &ConstantPool, index: u16) -> Res<MemberRef> {
    let (class_index, nat_index) = match entry(pool, index)? {
        CPIndexType::FieldRef {
            class_index,
            name_and_type_index,
        }
        | CPIndexType::MethodRef {
            class_index,
            name_and_type_index,
        }
        | CPIndexType::InterfaceMethodRef {
            class_index,
            name_and_type_index,
        } => (class_index, name_and_type_index),
        other => return Err(format!("#{} is not a member reference: {:?}", index, other)),
    };

    let (name_index, descriptor_index) = match entry(pool, nat_index)? {
        CPIndexType::NameAndType {
            name_index,
            descriptor_index,
        } => (name_index, descriptor_index),
        other => return Err(format!("#{} is not a NameAndType: {:?}", nat_index, other)),
    };

    Ok(MemberRef {
        class: class_name(pool, class_index)?,
        name: utf8(pool, name_index)?,
        descriptor: utf8(pool, descriptor_index)?,
    })
}

pub fn constant(pool: &ConstantPool, index: u16) -> Res<Expr> {
    Ok(match entry(pool, index)? {
        CPIndexType::Integer(value) => Expr::Int(value),
        CPIndexType::Float(value) => Expr::Float(value),
        CPIndexType::Long(value) => Expr::Long(value),
        CPIndexType::Double(value) => Expr::Double(value),
        CPIndexType::String(text_index) => Expr::Str(utf8(pool, text_index)?),
        CPIndexType::Class(name_index) => {
            Expr::ClassLit(descriptor::class_name(&utf8(pool, name_index)?))
        }
        other => return Err(format!("#{} is not a loadable constant: {:?}", index, other)),
    })
}

pub fn class_entry_type(pool: &ConstantPool, index: u16) -> Res<Type> {
    let name = internal_class_name(pool, index)?;
    if name.starts_with('[') {
        descriptor::parse_field(&name).ok_or_else(|| format!("bad array descriptor: {}", name))
    } else {
        Ok(Type::Class(descriptor::class_name(&name)))
    }
}
