#![allow(dead_code)]

use super::kind::Type;
use std::fmt;

pub const PREC_TERNARY: u8 = 1;
pub const PREC_UNARY: u8 = 12;
pub const PREC_CAST: u8 = 12;
pub const PREC_POSTFIX: u8 = 13;
pub const PREC_PRIMARY: u8 = 14;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BinOp {
    Mul,
    Div,
    Rem,
    Add,
    Sub,
    Shl,
    Shr,
    UShr,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
    BitAnd,
    BitXor,
    BitOr,
    And,
    Or,
}

impl BinOp {
    pub fn symbol(self) -> &'static str {
        match self {
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Rem => "%",
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Shl => "<<",
            BinOp::Shr => ">>",
            BinOp::UShr => ">>>",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::BitAnd => "&",
            BinOp::BitXor => "^",
            BinOp::BitOr => "|",
            BinOp::And => "&&",
            BinOp::Or => "||",
        }
    }

    pub fn precedence(self) -> u8 {
        match self {
            BinOp::Mul | BinOp::Div | BinOp::Rem => 11,
            BinOp::Add | BinOp::Sub => 10,
            BinOp::Shl | BinOp::Shr | BinOp::UShr => 9,
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => 8,
            BinOp::Eq | BinOp::Ne => 7,
            BinOp::BitAnd => 6,
            BinOp::BitXor => 5,
            BinOp::BitOr => 4,
            BinOp::And => 3,
            BinOp::Or => 2,
        }
    }

    pub fn negated(self) -> Option<BinOp> {
        Some(match self {
            BinOp::Eq => BinOp::Ne,
            BinOp::Ne => BinOp::Eq,
            BinOp::Lt => BinOp::Ge,
            BinOp::Ge => BinOp::Lt,
            BinOp::Gt => BinOp::Le,
            BinOp::Le => BinOp::Gt,
            _ => return None,
        })
    }

    pub fn is_comparison(self) -> bool {
        matches!(
            self,
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge | BinOp::Eq | BinOp::Ne
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Clone)]
pub enum CallTarget {
    Static(String),
    Instance(Box<Expr>),
}

#[derive(Clone)]
pub enum Expr {
    Null,
    Bool(bool),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    Str(String),
    ClassLit(String),

    Var {
        name: String,
        ty: Type,
    },

    StaticField {
        class: String,
        name: String,
        ty: Type,
    },
    Field {
        target: Box<Expr>,
        name: String,
        ty: Type,
    },

    ArrayElem {
        array: Box<Expr>,
        index: Box<Expr>,
        ty: Type,
    },
    ArrayLen(Box<Expr>),

    Unary {
        op: UnOp,
        operand: Box<Expr>,
        ty: Type,
    },
    Cast {
        to: Type,
        operand: Box<Expr>,
    },
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
        ty: Type,
    },
    Ternary {
        cond: Box<Expr>,
        then: Box<Expr>,
        other: Box<Expr>,
        ty: Type,
    },
    InstanceOf {
        operand: Box<Expr>,
        class: String,
    },

    Call {
        target: CallTarget,
        name: String,
        args: Vec<Expr>,
        ty: Type,
    },
    New {
        class: String,
        args: Vec<Expr>,
    },
    CtorCall {
        kind: &'static str,
        args: Vec<Expr>,
    },
    NewArray {
        elem: Type,
        dims: Vec<Expr>,
        extra: usize,
    },

    Cmp {
        left: Box<Expr>,
        right: Box<Expr>,
    },
}

impl Expr {
    pub fn ty(&self) -> Type {
        match self {
            Expr::Null => Type::Unknown,
            Expr::Bool(_) => Type::Boolean,
            Expr::Int(_) => Type::Int,
            Expr::Long(_) => Type::Long,
            Expr::Float(_) => Type::Float,
            Expr::Double(_) => Type::Double,
            Expr::Str(_) => Type::Class("java.lang.String".to_string()),
            Expr::ClassLit(_) => Type::Class("java.lang.Class".to_string()),
            Expr::Var { ty, .. } => ty.clone(),
            Expr::StaticField { ty, .. } => ty.clone(),
            Expr::Field { ty, .. } => ty.clone(),
            Expr::ArrayElem { ty, .. } => ty.clone(),
            Expr::ArrayLen(_) => Type::Int,
            Expr::Unary { ty, .. } => ty.clone(),
            Expr::Cast { to, .. } => to.clone(),
            Expr::Binary { ty, .. } => ty.clone(),
            Expr::Ternary { ty, .. } => ty.clone(),
            Expr::InstanceOf { .. } => Type::Boolean,
            Expr::Call { ty, .. } => ty.clone(),
            Expr::New { class, .. } => Type::Class(class.clone()),
            Expr::CtorCall { .. } => Type::Void,
            Expr::NewArray { elem, dims, extra } => {
                let mut ty = elem.clone();
                for _ in 0..(dims.len() + extra) {
                    ty = Type::Array(Box::new(ty));
                }
                ty
            }
            Expr::Cmp { .. } => Type::Int,
        }
    }

    pub fn precedence(&self) -> u8 {
        match self {
            Expr::Binary { op, .. } => op.precedence(),
            Expr::Ternary { .. } => PREC_TERNARY,
            Expr::Unary { .. } => PREC_UNARY,
            Expr::Cast { .. } => PREC_CAST,
            Expr::InstanceOf { .. } => 8,
            Expr::Field { .. }
            | Expr::ArrayElem { .. }
            | Expr::ArrayLen(_)
            | Expr::Call { .. } => PREC_POSTFIX,
            _ => PREC_PRIMARY,
        }
    }

    pub fn is_pure(&self) -> bool {
        match self {
            Expr::Call { .. }
            | Expr::New { .. }
            | Expr::NewArray { .. }
            | Expr::CtorCall { .. } => false,
            Expr::Field { target, .. } => target.is_pure(),
            Expr::ArrayElem { array, index, .. } => array.is_pure() && index.is_pure(),
            Expr::ArrayLen(inner) => inner.is_pure(),
            Expr::Unary { operand, .. } | Expr::Cast { operand, .. } => operand.is_pure(),
            Expr::Binary { left, right, .. } | Expr::Cmp { left, right } => {
                left.is_pure() && right.is_pure()
            }
            Expr::Ternary {
                cond, then, other, ..
            } => cond.is_pure() && then.is_pure() && other.is_pure(),
            Expr::InstanceOf { operand, .. } => operand.is_pure(),
            _ => true,
        }
    }

    fn as_bool_constant(&self) -> Option<bool> {
        match self {
            Expr::Bool(b) => Some(*b),
            Expr::Int(0) => Some(false),
            Expr::Int(1) => Some(true),
            _ => None,
        }
    }
}

pub fn fold_zero_compare(op: BinOp, value: Expr) -> Expr {
    match value {
        Expr::Cmp { left, right } => Expr::Binary {
            op,
            left,
            right,
            ty: Type::Boolean,
        },
        other => {
            if other.ty() == Type::Boolean {
                match op {
                    BinOp::Ne => return other,
                    BinOp::Eq => return negate(other),
                    _ => {}
                }
            }
            Expr::Binary {
                op,
                left: Box::new(other),
                right: Box::new(Expr::Int(0)),
                ty: Type::Boolean,
            }
        }
    }
}

pub fn null_compare(op: BinOp, value: Expr) -> Expr {
    Expr::Binary {
        op,
        left: Box::new(value),
        right: Box::new(Expr::Null),
        ty: Type::Boolean,
    }
}

pub fn negate(cond: Expr) -> Expr {
    match cond {
        Expr::Bool(b) => Expr::Bool(!b),
        Expr::Unary {
            op: UnOp::Not,
            operand,
            ..
        } => *operand,
        Expr::Binary {
            op,
            left,
            right,
            ty,
        } => match op.negated() {
            Some(flipped) => Expr::Binary {
                op: flipped,
                left,
                right,
                ty,
            },
            None => Expr::Unary {
                op: UnOp::Not,
                operand: Box::new(Expr::Binary {
                    op,
                    left,
                    right,
                    ty,
                }),
                ty: Type::Boolean,
            },
        },
        other => Expr::Unary {
            op: UnOp::Not,
            operand: Box::new(other),
            ty: Type::Boolean,
        },
    }
}

pub fn ternary(cond: Expr, then: Expr, other: Expr) -> Expr {
    match (then.as_bool_constant(), other.as_bool_constant()) {
        (Some(true), Some(false)) => return cond,
        (Some(false), Some(true)) => return negate(cond),
        _ => {}
    }

    let ty = if then.ty() == Type::Unknown {
        other.ty()
    } else {
        then.ty()
    };

    Expr::Ternary {
        cond: Box::new(cond),
        then: Box::new(then),
        other: Box::new(other),
        ty,
    }
}

fn operand(f: &mut fmt::Formatter<'_>, child: &Expr, parent: u8) -> fmt::Result {
    if child.precedence() < parent {
        write!(f, "({})", child)
    } else {
        write!(f, "{}", child)
    }
}

fn right_operand(f: &mut fmt::Formatter<'_>, child: &Expr, parent: u8) -> fmt::Result {
    if child.precedence() <= parent {
        write!(f, "({})", child)
    } else {
        write!(f, "{}", child)
    }
}

fn comma_list(f: &mut fmt::Formatter<'_>, items: &[Expr]) -> fmt::Result {
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            f.write_str(", ")?;
        }
        write!(f, "{}", item)?;
    }
    Ok(())
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Null => f.write_str("null"),
            Expr::Bool(v) => write!(f, "{}", v),
            Expr::Int(v) => write!(f, "{}", v),
            Expr::Long(v) => write!(f, "{}L", v),
            Expr::Float(v) => f.write_str(&java_float(*v)),
            Expr::Double(v) => f.write_str(&java_double(*v)),
            Expr::Str(v) => write!(f, "\"{}\"", escape_java(v)),
            Expr::ClassLit(name) => write!(f, "{}.class", name),

            Expr::Var { name, .. } => f.write_str(name),

            Expr::StaticField { class, name, .. } => write!(f, "{}.{}", class, name),
            Expr::Field { target, name, .. } => {
                operand(f, target, PREC_POSTFIX)?;
                write!(f, ".{}", name)
            }

            Expr::ArrayElem { array, index, .. } => {
                operand(f, array, PREC_POSTFIX)?;
                write!(f, "[{}]", index)
            }
            Expr::ArrayLen(array) => {
                operand(f, array, PREC_POSTFIX)?;
                f.write_str(".length")
            }

            Expr::Unary { op, operand: inner, .. } => {
                f.write_str(match op {
                    UnOp::Neg => "-",
                    UnOp::Not => "!",
                })?;
                operand(f, inner, PREC_UNARY)
            }
            Expr::Cast { to, operand: inner } => {
                write!(f, "({}) ", to)?;
                operand(f, inner, PREC_CAST)
            }

            Expr::Binary {
                op, left, right, ..
            } => {
                let prec = op.precedence();
                operand(f, left, prec)?;
                write!(f, " {} ", op.symbol())?;
                right_operand(f, right, prec)
            }

            Expr::Ternary {
                cond, then, other, ..
            } => {
                operand(f, cond, PREC_TERNARY + 1)?;
                f.write_str(" ? ")?;
                operand(f, then, PREC_TERNARY)?;
                f.write_str(" : ")?;
                operand(f, other, PREC_TERNARY)
            }

            Expr::InstanceOf { operand: inner, class } => {
                operand(f, inner, 8)?;
                write!(f, " instanceof {}", class)
            }

            Expr::Call {
                target, name, args, ..
            } => {
                match target {
                    CallTarget::Static(class) => write!(f, "{}", class)?,
                    CallTarget::Instance(receiver) => operand(f, receiver, PREC_POSTFIX)?,
                }
                write!(f, ".{}(", name)?;
                comma_list(f, args)?;
                f.write_str(")")
            }

            Expr::New { class, args } => {
                write!(f, "new {}(", class)?;
                comma_list(f, args)?;
                f.write_str(")")
            }

            Expr::CtorCall { kind, args } => {
                write!(f, "{}(", kind)?;
                comma_list(f, args)?;
                f.write_str(")")
            }

            Expr::NewArray { elem, dims, extra } => {
                write!(f, "new {}", elem)?;
                for dim in dims {
                    write!(f, "[{}]", dim)?;
                }
                for _ in 0..*extra {
                    f.write_str("[]")?;
                }
                Ok(())
            }

            Expr::Cmp { left, right } => {
                write!(f, "/* unfolded compare */ compare({}, {})", left, right)
            }
        }
    }
}

pub fn java_double(v: f64) -> String {
    if v.is_nan() {
        return "Double.NaN".to_string();
    }
    if v.is_infinite() {
        return if v > 0.0 {
            "Double.POSITIVE_INFINITY".to_string()
        } else {
            "Double.NEGATIVE_INFINITY".to_string()
        };
    }
    format!("{:?}", v)
}

pub fn java_float(v: f32) -> String {
    if v.is_nan() {
        return "Float.NaN".to_string();
    }
    if v.is_infinite() {
        return if v > 0.0 {
            "Float.POSITIVE_INFINITY".to_string()
        } else {
            "Float.NEGATIVE_INFINITY".to_string()
        };
    }
    format!("{:?}f", v)
}

pub fn escape_java(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\u{:04x}", c as u32))
            }
            c => out.push(c),
        }
    }
    out
}

pub enum Stmt {
    Expr(Expr),
    Declare {
        ty: Type,
        name: String,
        value: Option<Expr>,
    },
    Assign {
        target: Expr,
        value: Expr,
    },
    If {
        cond: Expr,
        then: Vec<Stmt>,
        otherwise: Vec<Stmt>,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
    },
    DoWhile {
        body: Vec<Stmt>,
        cond: Expr,
    },
    Return(Option<Expr>),
    Throw(Expr),
    Break(Option<String>),
    Continue(Option<String>),
    Labeled {
        label: String,
        body: Vec<Stmt>,
    },
    Comment(String),
}

impl Stmt {
    pub fn write_to<W: fmt::Write>(&self, out: &mut W, depth: usize) -> fmt::Result {
        let pad = "    ".repeat(depth);
        match self {
            Stmt::Expr(expr) => writeln!(out, "{}{};", pad, expr),
            Stmt::Declare { ty, name, value } => match value {
                Some(value) => writeln!(out, "{}{} {} = {};", pad, ty, name, value),
                None => writeln!(out, "{}{} {};", pad, ty, name),
            },
            Stmt::Assign { target, value } => writeln!(out, "{}{} = {};", pad, target, value),
            Stmt::If {
                cond,
                then,
                otherwise,
            } => {
                writeln!(out, "{}if ({}) {{", pad, cond)?;
                write_block(out, then, depth + 1)?;
                if otherwise.is_empty() {
                    writeln!(out, "{}}}", pad)
                } else {
                    writeln!(out, "{}}} else {{", pad)?;
                    write_block(out, otherwise, depth + 1)?;
                    writeln!(out, "{}}}", pad)
                }
            }
            Stmt::While { cond, body } => {
                writeln!(out, "{}while ({}) {{", pad, cond)?;
                write_block(out, body, depth + 1)?;
                writeln!(out, "{}}}", pad)
            }
            Stmt::DoWhile { body, cond } => {
                writeln!(out, "{}do {{", pad)?;
                write_block(out, body, depth + 1)?;
                writeln!(out, "{}}} while ({});", pad, cond)
            }
            Stmt::Return(None) => writeln!(out, "{}return;", pad),
            Stmt::Return(Some(value)) => writeln!(out, "{}return {};", pad, value),
            Stmt::Throw(value) => writeln!(out, "{}throw {};", pad, value),
            Stmt::Break(None) => writeln!(out, "{}break;", pad),
            Stmt::Break(Some(label)) => writeln!(out, "{}break {};", pad, label),
            Stmt::Continue(None) => writeln!(out, "{}continue;", pad),
            Stmt::Continue(Some(label)) => writeln!(out, "{}continue {};", pad, label),
            Stmt::Labeled { label, body } => {
                writeln!(out, "{}{}:", pad, label)?;
                write_block(out, body, depth)
            }
            Stmt::Comment(text) => {
                for line in text.lines() {
                    writeln!(out, "{}// {}", pad, line)?;
                }
                Ok(())
            }
        }
    }
}

pub fn write_block<W: fmt::Write>(out: &mut W, body: &[Stmt], depth: usize) -> fmt::Result {
    for stmt in body {
        stmt.write_to(out, depth)?;
    }
    Ok(())
}

pub fn render_block(body: &[Stmt], depth: usize) -> String {
    let mut out = String::new();
    let _ = write_block(&mut out, body, depth);
    out
}

impl fmt::Display for Stmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_to(f, 0)
    }
}
