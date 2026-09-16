use super::expr::kind::Type;

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn next(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.pos += 1;
        Some(byte)
    }
}

fn parse_one(cursor: &mut Cursor<'_>) -> Option<Type> {
    Some(match cursor.next()? {
        b'B' => Type::Byte,
        b'C' => Type::Char,
        b'D' => Type::Double,
        b'F' => Type::Float,
        b'I' => Type::Int,
        b'J' => Type::Long,
        b'S' => Type::Short,
        b'Z' => Type::Boolean,
        b'V' => Type::Void,
        b'[' => Type::Array(Box::new(parse_one(cursor)?)),
        b'L' => {
            let start = cursor.pos;
            while cursor.peek()? != b';' {
                cursor.pos += 1;
            }
            let name = std::str::from_utf8(&cursor.bytes[start..cursor.pos]).ok()?;
            cursor.pos += 1;
            Type::Class(class_name(name))
        }
        _ => return None,
    })
}

pub fn class_name(internal: &str) -> String {
    internal.replace('/', ".")
}

pub fn parse_field(descriptor: &str) -> Option<Type> {
    let mut cursor = Cursor {
        bytes: descriptor.as_bytes(),
        pos: 0,
    };
    let ty = parse_one(&mut cursor)?;
    if cursor.pos == descriptor.len() {
        Some(ty)
    } else {
        None
    }
}

pub fn parse_method(descriptor: &str) -> Option<(Vec<Type>, Type)> {
    let bytes = descriptor.as_bytes();
    if bytes.first() != Some(&b'(') {
        return None;
    }

    let mut cursor = Cursor { bytes, pos: 1 };
    let mut params = Vec::new();

    while cursor.peek()? != b')' {
        params.push(parse_one(&mut cursor)?);
    }
    cursor.pos += 1;

    let ret = parse_one(&mut cursor)?;
    Some((params, ret))
}

pub fn slot_size(ty: &Type) -> u16 {
    match ty {
        Type::Long | Type::Double => 2,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(ty: &Type) -> String {
        ty.to_string()
    }

    #[test]
    fn parses_mixed_parameters() {
        let (params, ret) = parse_method("([IJLjava/lang/String;)Z").unwrap();
        let rendered: Vec<String> = params.iter().map(render).collect();
        assert_eq!(rendered, vec!["int[]", "long", "java.lang.String"]);
        assert_eq!(render(&ret), "boolean");
    }

    #[test]
    fn parses_nested_arrays() {
        assert_eq!(render(&parse_field("[[[D").unwrap()), "double[][][]");
    }

    #[test]
    fn rejects_trailing_junk() {
        assert!(parse_field("IJ").is_none());
    }

    #[test]
    fn counts_slots() {
        let (params, _) = parse_method("(JDI)V").unwrap();
        let total: u16 = params.iter().map(slot_size).sum();
        assert_eq!(total, 5);
    }
}
