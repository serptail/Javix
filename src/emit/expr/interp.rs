use std::collections::HashMap;

use super::super::constants as cp;
use super::super::descriptor;
use super::super::opcodes::Instruction as Instr;
use super::super::ConstantPool;
use super::ast::{self, BinOp, CallTarget, Expr, Stmt, UnOp};
use super::kind::Type;

pub type Res<T> = Result<T, String>;

#[derive(Clone)]
pub struct Local {
    pub name: String,
    pub ty: Type,
    pub declared: bool,
}

pub struct MethodContext {
    pub owner: String,
    pub params: Vec<(u16, Type)>,
    pub locals: HashMap<u16, Local>,
}

impl MethodContext {
    pub fn new(
        is_static: bool,
        owner: String,
        method_descriptor: &str,
        named: &HashMap<u16, (String, Type)>,
    ) -> Res<Self> {
        let (types, _) = descriptor::parse_method(method_descriptor)
            .ok_or_else(|| format!("bad method descriptor: {}", method_descriptor))?;

        let mut locals: HashMap<u16, Local> = HashMap::new();
        let mut params = Vec::with_capacity(types.len());
        let mut slot = 0u16;

        if !is_static {
            locals.insert(
                0,
                Local {
                    name: "this".to_string(),
                    ty: Type::Class(owner.clone()),
                    declared: true,
                },
            );
            slot = 1;
        }

        for (position, ty) in types.iter().enumerate() {
            let name = match named.get(&slot) {
                Some((name, _)) => name.clone(),
                None => format!("arg{}", position + if is_static { 0 } else { 1 }),
            };

            params.push((slot, ty.clone()));
            locals.insert(
                slot,
                Local {
                    name,
                    ty: ty.clone(),
                    declared: true,
                },
            );

            slot += descriptor::slot_size(ty);
        }

        for (slot, (name, ty)) in named {
            locals.entry(*slot).or_insert_with(|| Local {
                name: name.clone(),
                ty: ty.clone(),
                declared: false,
            });
        }

        Ok(MethodContext {
            owner,
            params,
            locals,
        })
    }

    pub fn param_name(&self, slot: u16) -> String {
        match self.locals.get(&slot) {
            Some(local) => local.name.clone(),
            None => format!("var{}", slot),
        }
    }
}

#[derive(Clone)]
pub struct Frame {
    pub stack: Vec<Expr>,
    pub locals: HashMap<u16, Local>,
}

impl Frame {
    pub fn new(ctx: &MethodContext) -> Self {
        Frame {
            stack: Vec::new(),
            locals: ctx.locals.clone(),
        }
    }

    pub fn push(&mut self, value: Expr) {
        self.stack.push(value);
    }

    pub fn pop(&mut self) -> Res<Expr> {
        self.stack
            .pop()
            .ok_or_else(|| "stack underflow".to_string())
    }

    pub fn merge_locals(&mut self, other: &Frame) {
        for (slot, local) in &other.locals {
            match self.locals.get_mut(slot) {
                Some(existing) => existing.declared |= local.declared,
                None => {
                    self.locals.insert(*slot, local.clone());
                }
            }
        }
    }
}

fn binary(frame: &mut Frame, op: BinOp, ty: Type) -> Res<()> {
    let right = frame.pop()?;
    let left = frame.pop()?;
    frame.push(Expr::Binary {
        op,
        left: Box::new(left),
        right: Box::new(right),
        ty,
    });
    Ok(())
}

fn unary(frame: &mut Frame, op: UnOp, ty: Type) -> Res<()> {
    let operand = frame.pop()?;
    frame.push(Expr::Unary {
        op,
        operand: Box::new(operand),
        ty,
    });
    Ok(())
}

fn convert(frame: &mut Frame, to: Type) -> Res<()> {
    let operand = frame.pop()?;
    frame.push(Expr::Cast {
        to,
        operand: Box::new(operand),
    });
    Ok(())
}

fn compare(frame: &mut Frame) -> Res<()> {
    let right = frame.pop()?;
    let left = frame.pop()?;
    frame.push(Expr::Cmp {
        left: Box::new(left),
        right: Box::new(right),
    });
    Ok(())
}

fn array_load(frame: &mut Frame, ty: Type) -> Res<()> {
    let index = frame.pop()?;
    let array = frame.pop()?;
    frame.push(Expr::ArrayElem {
        array: Box::new(array),
        index: Box::new(index),
        ty,
    });
    Ok(())
}

fn array_store(frame: &mut Frame, ty: Type) -> Res<Stmt> {
    let value = frame.pop()?;
    let index = frame.pop()?;
    let array = frame.pop()?;
    Ok(Stmt::Assign {
        target: Expr::ArrayElem {
            array: Box::new(array),
            index: Box::new(index),
            ty,
        },
        value,
    })
}

fn is_wide(expr: &Expr) -> bool {
    matches!(expr.ty(), Type::Long | Type::Double)
}

fn load_local(frame: &Frame, slot: u16, hint: Type) -> Expr {
    match frame.locals.get(&slot) {
        Some(local) => Expr::Var {
            name: local.name.clone(),
            ty: local.ty.clone(),
        },
        None => Expr::Var {
            name: format!("var{}", slot),
            ty: hint,
        },
    }
}

fn push_local(frame: &mut Frame, slot: u16, hint: Type) {
    let value = load_local(frame, slot, hint);
    frame.push(value);
}

fn declared_type(value: &Expr, hint: Type) -> Type {
    match value.ty() {
        Type::Unknown => hint,
        known => known,
    }
}

fn store_local(frame: &mut Frame, slot: u16, hint: Type) -> Res<Stmt> {
    let value = frame.pop()?;

    if let Some(local) = frame.locals.get_mut(&slot) {
        if local.declared {
            return Ok(Stmt::Assign {
                target: Expr::Var {
                    name: local.name.clone(),
                    ty: local.ty.clone(),
                },
                value,
            });
        }

        local.declared = true;
        return Ok(Stmt::Declare {
            ty: local.ty.clone(),
            name: local.name.clone(),
            value: Some(value),
        });
    }

    let ty = declared_type(&value, hint);
    let name = format!("var{}", slot);
    frame.locals.insert(
        slot,
        Local {
            name: name.clone(),
            ty: ty.clone(),
            declared: true,
        },
    );

    Ok(Stmt::Declare {
        ty,
        name,
        value: Some(value),
    })
}

fn invoke(
    frame: &mut Frame,
    pool: &ConstantPool,
    ctx: &MethodContext,
    ref_index: u16,
    is_static: bool,
    is_special: bool,
) -> Res<Vec<Stmt>> {
    let member = cp::member_ref(pool, ref_index)?;
    let (param_types, return_type) = descriptor::parse_method(&member.descriptor)
        .ok_or_else(|| format!("bad method descriptor: {}", member.descriptor))?;

    let mut args = Vec::with_capacity(param_types.len());
    for _ in 0..param_types.len() {
        args.push(frame.pop()?);
    }
    args.reverse();

    if is_special && member.name == "<init>" {
        return constructor(frame, ctx, member, args);
    }

    let target = if is_static {
        CallTarget::Static(member.class.clone())
    } else {
        CallTarget::Instance(Box::new(frame.pop()?))
    };

    let call = Expr::Call {
        target,
        name: member.name,
        args,
        ty: return_type.clone(),
    };

    if return_type == Type::Void {
        Ok(vec![Stmt::Expr(call)])
    } else {
        frame.push(call);
        Ok(Vec::new())
    }
}

fn constructor(
    frame: &mut Frame,
    ctx: &MethodContext,
    member: cp::MemberRef,
    args: Vec<Expr>,
) -> Res<Vec<Stmt>> {
    let receiver = frame.pop()?;

    match receiver {
        Expr::New {
            class,
            args: pending,
        } => {
            if !pending.is_empty() {
                return Err("constructor call on an already initialised object".to_string());
            }

            let completed = Expr::New {
                class: class.clone(),
                args,
            };

            let duplicate = frame.stack.iter().rposition(|value| match value {
                Expr::New {
                    class: other,
                    args: other_args,
                } => *other == class && other_args.is_empty(),
                _ => false,
            });

            match duplicate {
                Some(position) => {
                    frame.stack[position] = completed;
                    Ok(Vec::new())
                }
                None => Ok(vec![Stmt::Expr(completed)]),
            }
        }

        Expr::Var { name, .. } => {
            if name != "this" {
                return Err(format!("constructor call on local `{}`", name));
            }

            let kind = if member.class == ctx.owner {
                "this"
            } else {
                "super"
            };

            if kind == "super" && args.is_empty() {
                Ok(Vec::new())
            } else {
                Ok(vec![Stmt::Expr(Expr::CtorCall { kind, args })])
            }
        }

        other => Err(format!(
            "constructor call on an unexpected receiver: {}",
            other
        )),
    }
}

pub fn apply(
    insn: &Instr,
    frame: &mut Frame,
    pool: &ConstantPool,
    ctx: &MethodContext,
    out: &mut Vec<Stmt>,
) -> Res<()> {
    match insn {
        Instr::Nop => {}
        Instr::AConstNull => frame.push(Expr::Null),
        Instr::IConstM1 => frame.push(Expr::Int(-1)),
        Instr::IConst0 => frame.push(Expr::Int(0)),
        Instr::IConst1 => frame.push(Expr::Int(1)),
        Instr::IConst2 => frame.push(Expr::Int(2)),
        Instr::IConst3 => frame.push(Expr::Int(3)),
        Instr::IConst4 => frame.push(Expr::Int(4)),
        Instr::IConst5 => frame.push(Expr::Int(5)),
        Instr::LConst0 => frame.push(Expr::Long(0)),
        Instr::LConst1 => frame.push(Expr::Long(1)),
        Instr::FConst0 => frame.push(Expr::Float(0.0)),
        Instr::FConst1 => frame.push(Expr::Float(1.0)),
        Instr::FConst2 => frame.push(Expr::Float(2.0)),
        Instr::DConst0 => frame.push(Expr::Double(0.0)),
        Instr::DConst1 => frame.push(Expr::Double(1.0)),
        Instr::BiPush(value) => frame.push(Expr::Int(*value as i32)),
        Instr::SiPush(value) => frame.push(Expr::Int(*value as i32)),
        Instr::Ldc(index) => {
            let value = cp::constant(pool, *index as u16)?;
            frame.push(value);
        }
        Instr::LdcW(index) | Instr::Ldc2W(index) => {
            let value = cp::constant(pool, *index)?;
            frame.push(value);
        }

        Instr::ILoad(slot) => push_local(frame, *slot, Type::Int),
        Instr::LLoad(slot) => push_local(frame, *slot, Type::Long),
        Instr::FLoad(slot) => push_local(frame, *slot, Type::Float),
        Instr::DLoad(slot) => push_local(frame, *slot, Type::Double),
        Instr::ALoad(slot) => push_local(frame, *slot, Type::Unknown),
        Instr::ILoad0 => push_local(frame, 0, Type::Int),
        Instr::ILoad1 => push_local(frame, 1, Type::Int),
        Instr::ILoad2 => push_local(frame, 2, Type::Int),
        Instr::ILoad3 => push_local(frame, 3, Type::Int),
        Instr::LLoad0 => push_local(frame, 0, Type::Long),
        Instr::LLoad1 => push_local(frame, 1, Type::Long),
        Instr::LLoad2 => push_local(frame, 2, Type::Long),
        Instr::LLoad3 => push_local(frame, 3, Type::Long),
        Instr::FLoad0 => push_local(frame, 0, Type::Float),
        Instr::FLoad1 => push_local(frame, 1, Type::Float),
        Instr::FLoad2 => push_local(frame, 2, Type::Float),
        Instr::FLoad3 => push_local(frame, 3, Type::Float),
        Instr::DLoad0 => push_local(frame, 0, Type::Double),
        Instr::DLoad1 => push_local(frame, 1, Type::Double),
        Instr::DLoad2 => push_local(frame, 2, Type::Double),
        Instr::DLoad3 => push_local(frame, 3, Type::Double),
        Instr::ALoad0 => push_local(frame, 0, Type::Unknown),
        Instr::ALoad1 => push_local(frame, 1, Type::Unknown),
        Instr::ALoad2 => push_local(frame, 2, Type::Unknown),
        Instr::ALoad3 => push_local(frame, 3, Type::Unknown),

        Instr::IALoad => array_load(frame, Type::Int)?,
        Instr::LALoad => array_load(frame, Type::Long)?,
        Instr::FALoad => array_load(frame, Type::Float)?,
        Instr::DALoad => array_load(frame, Type::Double)?,
        Instr::BALoad => array_load(frame, Type::Byte)?,
        Instr::CALoad => array_load(frame, Type::Char)?,
        Instr::SALoad => array_load(frame, Type::Short)?,
        Instr::AALoad => {
            let index = frame.pop()?;
            let array = frame.pop()?;
            let ty = match array.ty() {
                Type::Array(element) => *element,
                _ => Type::Unknown,
            };
            frame.push(Expr::ArrayElem {
                array: Box::new(array),
                index: Box::new(index),
                ty,
            });
        }
        Instr::ArrayLength => {
            let array = frame.pop()?;
            frame.push(Expr::ArrayLen(Box::new(array)));
        }

        Instr::IStore(slot) => out.push(store_local(frame, *slot, Type::Int)?),
        Instr::LStore(slot) => out.push(store_local(frame, *slot, Type::Long)?),
        Instr::FStore(slot) => out.push(store_local(frame, *slot, Type::Float)?),
        Instr::DStore(slot) => out.push(store_local(frame, *slot, Type::Double)?),
        Instr::AStore(slot) => out.push(store_local(frame, *slot, Type::Unknown)?),
        Instr::IStore0 => out.push(store_local(frame, 0, Type::Int)?),
        Instr::IStore1 => out.push(store_local(frame, 1, Type::Int)?),
        Instr::IStore2 => out.push(store_local(frame, 2, Type::Int)?),
        Instr::IStore3 => out.push(store_local(frame, 3, Type::Int)?),
        Instr::LStore0 => out.push(store_local(frame, 0, Type::Long)?),
        Instr::LStore1 => out.push(store_local(frame, 1, Type::Long)?),
        Instr::LStore2 => out.push(store_local(frame, 2, Type::Long)?),
        Instr::LStore3 => out.push(store_local(frame, 3, Type::Long)?),
        Instr::FStore0 => out.push(store_local(frame, 0, Type::Float)?),
        Instr::FStore1 => out.push(store_local(frame, 1, Type::Float)?),
        Instr::FStore2 => out.push(store_local(frame, 2, Type::Float)?),
        Instr::FStore3 => out.push(store_local(frame, 3, Type::Float)?),
        Instr::DStore0 => out.push(store_local(frame, 0, Type::Double)?),
        Instr::DStore1 => out.push(store_local(frame, 1, Type::Double)?),
        Instr::DStore2 => out.push(store_local(frame, 2, Type::Double)?),
        Instr::DStore3 => out.push(store_local(frame, 3, Type::Double)?),
        Instr::AStore0 => out.push(store_local(frame, 0, Type::Unknown)?),
        Instr::AStore1 => out.push(store_local(frame, 1, Type::Unknown)?),
        Instr::AStore2 => out.push(store_local(frame, 2, Type::Unknown)?),
        Instr::AStore3 => out.push(store_local(frame, 3, Type::Unknown)?),

        Instr::IAStore => out.push(array_store(frame, Type::Int)?),
        Instr::LAStore => out.push(array_store(frame, Type::Long)?),
        Instr::FAStore => out.push(array_store(frame, Type::Float)?),
        Instr::DAStore => out.push(array_store(frame, Type::Double)?),
        Instr::BAStore => out.push(array_store(frame, Type::Byte)?),
        Instr::CAStore => out.push(array_store(frame, Type::Char)?),
        Instr::SAStore => out.push(array_store(frame, Type::Short)?),
        Instr::AAStore => out.push(array_store(frame, Type::Unknown)?),

        Instr::Pop => {
            let value = frame.pop()?;
            if !value.is_pure() {
                out.push(Stmt::Expr(value));
            }
        }
        Instr::Pop2 => {
            let value = frame.pop()?;
            let wide = is_wide(&value);
            if !value.is_pure() {
                out.push(Stmt::Expr(value));
            }
            if !wide {
                let second = frame.pop()?;
                if !second.is_pure() {
                    out.push(Stmt::Expr(second));
                }
            }
        }
        Instr::Dup => {
            let value = frame.pop()?;
            frame.push(value.clone());
            frame.push(value);
        }
        Instr::DupX1 => {
            let top = frame.pop()?;
            let under = frame.pop()?;
            frame.push(top.clone());
            frame.push(under);
            frame.push(top);
        }
        Instr::Dup2 => {
            let top = frame.pop()?;
            if is_wide(&top) {
                frame.push(top.clone());
                frame.push(top);
            } else {
                let under = frame.pop()?;
                frame.push(under.clone());
                frame.push(top.clone());
                frame.push(under);
                frame.push(top);
            }
        }
        Instr::Swap => {
            let top = frame.pop()?;
            let under = frame.pop()?;
            frame.push(top);
            frame.push(under);
        }

        Instr::IAdd => binary(frame, BinOp::Add, Type::Int)?,
        Instr::LAdd => binary(frame, BinOp::Add, Type::Long)?,
        Instr::FAdd => binary(frame, BinOp::Add, Type::Float)?,
        Instr::DAdd => binary(frame, BinOp::Add, Type::Double)?,
        Instr::ISub => binary(frame, BinOp::Sub, Type::Int)?,
        Instr::LSub => binary(frame, BinOp::Sub, Type::Long)?,
        Instr::FSub => binary(frame, BinOp::Sub, Type::Float)?,
        Instr::DSub => binary(frame, BinOp::Sub, Type::Double)?,
        Instr::IMul => binary(frame, BinOp::Mul, Type::Int)?,
        Instr::LMul => binary(frame, BinOp::Mul, Type::Long)?,
        Instr::FMul => binary(frame, BinOp::Mul, Type::Float)?,
        Instr::DMul => binary(frame, BinOp::Mul, Type::Double)?,
        Instr::IDiv => binary(frame, BinOp::Div, Type::Int)?,
        Instr::LDiv => binary(frame, BinOp::Div, Type::Long)?,
        Instr::FDiv => binary(frame, BinOp::Div, Type::Float)?,
        Instr::DDiv => binary(frame, BinOp::Div, Type::Double)?,
        Instr::IRem => binary(frame, BinOp::Rem, Type::Int)?,
        Instr::LRem => binary(frame, BinOp::Rem, Type::Long)?,
        Instr::FRem => binary(frame, BinOp::Rem, Type::Float)?,
        Instr::DRem => binary(frame, BinOp::Rem, Type::Double)?,
        Instr::INeg => unary(frame, UnOp::Neg, Type::Int)?,
        Instr::LNeg => unary(frame, UnOp::Neg, Type::Long)?,
        Instr::FNeg => unary(frame, UnOp::Neg, Type::Float)?,
        Instr::DNeg => unary(frame, UnOp::Neg, Type::Double)?,
        Instr::IShl => binary(frame, BinOp::Shl, Type::Int)?,
        Instr::LShl => binary(frame, BinOp::Shl, Type::Long)?,
        Instr::IShr => binary(frame, BinOp::Shr, Type::Int)?,
        Instr::LShr => binary(frame, BinOp::Shr, Type::Long)?,
        Instr::IUShr => binary(frame, BinOp::UShr, Type::Int)?,
        Instr::LUShr => binary(frame, BinOp::UShr, Type::Long)?,
        Instr::IAnd => binary(frame, BinOp::BitAnd, Type::Int)?,
        Instr::LAnd => binary(frame, BinOp::BitAnd, Type::Long)?,
        Instr::IOr => binary(frame, BinOp::BitOr, Type::Int)?,
        Instr::LOr => binary(frame, BinOp::BitOr, Type::Long)?,
        Instr::IXor => binary(frame, BinOp::BitXor, Type::Int)?,
        Instr::LXor => binary(frame, BinOp::BitXor, Type::Long)?,

        Instr::IInc(slot, amount) => {
            let target = load_local(frame, *slot, Type::Int);
            let (op, magnitude) = if *amount < 0 {
                (BinOp::Sub, -(*amount as i32))
            } else {
                (BinOp::Add, *amount as i32)
            };
            out.push(Stmt::Assign {
                target: target.clone(),
                value: Expr::Binary {
                    op,
                    left: Box::new(target),
                    right: Box::new(Expr::Int(magnitude)),
                    ty: Type::Int,
                },
            });
        }

        Instr::I2L | Instr::F2L | Instr::D2L => convert(frame, Type::Long)?,
        Instr::I2F | Instr::L2F | Instr::D2F => convert(frame, Type::Float)?,
        Instr::I2D | Instr::L2D | Instr::F2D => convert(frame, Type::Double)?,
        Instr::L2I | Instr::F2I | Instr::D2I => convert(frame, Type::Int)?,
        Instr::I2B => convert(frame, Type::Byte)?,
        Instr::I2C => convert(frame, Type::Char)?,
        Instr::I2S => convert(frame, Type::Short)?,

        Instr::LCmp | Instr::FCmpL | Instr::FCmpG | Instr::DCmpL | Instr::DCmpG => compare(frame)?,

        Instr::GetStatic(index) => {
            let member = cp::member_ref(pool, *index)?;
            let ty = field_type(&member)?;
            frame.push(Expr::StaticField {
                class: member.class,
                name: member.name,
                ty,
            });
        }
        Instr::GetField(index) => {
            let member = cp::member_ref(pool, *index)?;
            let ty = field_type(&member)?;
            let target = frame.pop()?;
            frame.push(Expr::Field {
                target: Box::new(target),
                name: member.name,
                ty,
            });
        }
        Instr::PutStatic(index) => {
            let member = cp::member_ref(pool, *index)?;
            let ty = field_type(&member)?;
            let value = frame.pop()?;
            out.push(Stmt::Assign {
                target: Expr::StaticField {
                    class: member.class,
                    name: member.name,
                    ty,
                },
                value,
            });
        }
        Instr::PutField(index) => {
            let member = cp::member_ref(pool, *index)?;
            let ty = field_type(&member)?;
            let value = frame.pop()?;
            let target = frame.pop()?;
            out.push(Stmt::Assign {
                target: Expr::Field {
                    target: Box::new(target),
                    name: member.name,
                    ty,
                },
                value,
            });
        }

        Instr::InvokeVirtual(index) => out.extend(invoke(frame, pool, ctx, *index, false, false)?),
        Instr::InvokeInterface(index) => out.extend(invoke(frame, pool, ctx, *index, false, false)?),
        Instr::InvokeStatic(index) => out.extend(invoke(frame, pool, ctx, *index, true, false)?),
        Instr::InvokeSpecial(index) => out.extend(invoke(frame, pool, ctx, *index, false, true)?),

        Instr::New(index) => {
            let class = cp::class_name(pool, *index)?;
            frame.push(Expr::New {
                class,
                args: Vec::new(),
            });
        }
        Instr::NewArray(tag) => {
            let elem = primitive_array_type(*tag)?;
            let size = frame.pop()?;
            frame.push(Expr::NewArray {
                elem,
                dims: vec![size],
                extra: 0,
            });
        }
        Instr::ANewArray(index) => {
            let elem = cp::class_entry_type(pool, *index)?;
            let size = frame.pop()?;
            frame.push(Expr::NewArray {
                elem,
                dims: vec![size],
                extra: 0,
            });
        }
        Instr::MultiANewArray(index, dimensions) => {
            let full = cp::class_entry_type(pool, *index)?;
            let count = *dimensions as usize;
            let (elem, depth) = peel_array(full);
            let mut dims = Vec::with_capacity(count);
            for _ in 0..count {
                dims.push(frame.pop()?);
            }
            dims.reverse();
            frame.push(Expr::NewArray {
                elem,
                dims,
                extra: depth.saturating_sub(count),
            });
        }

        Instr::CheckCast(index) => {
            let to = cp::class_entry_type(pool, *index)?;
            convert(frame, to)?;
        }
        Instr::InstanceOf(index) => {
            let ty = cp::class_entry_type(pool, *index)?;
            let operand = frame.pop()?;
            frame.push(Expr::InstanceOf {
                operand: Box::new(operand),
                class: ty.to_string(),
            });
        }

        Instr::Return => out.push(Stmt::Return(None)),
        Instr::IReturn | Instr::LReturn | Instr::FReturn | Instr::DReturn | Instr::AReturn => {
            let value = frame.pop()?;
            out.push(Stmt::Return(Some(value)));
        }
        Instr::AThrow => {
            let value = frame.pop()?;
            out.push(Stmt::Throw(value));
        }

        Instr::MonitorEnter | Instr::MonitorExit => {
            frame.pop()?;
            out.push(Stmt::Comment(
                "synchronized block not reconstructed".to_string(),
            ));
        }
        Instr::TableSwitch { low, targets, .. } => {
            return Err(format!(
                "tableswitch ({} cases from {}) is not reconstructed yet",
                targets.len(),
                low
            ))
        }
        Instr::LookupSwitch { pairs, .. } => {
            return Err(format!(
                "lookupswitch ({} cases) is not reconstructed yet",
                pairs.len()
            ))
        }
        Instr::InvokeDynamic(index) => {
            return Err(format!(
                "invokedynamic #{} (lambda or string concatenation) is not supported",
                index
            ))
        }
        Instr::Jsr(offset) => return Err(format!("jsr {:+} is not supported", offset)),
        Instr::JsrW(offset) => return Err(format!("jsr_w {:+} is not supported", offset)),
        Instr::Ret(slot) => return Err(format!("ret from slot {} is not supported", slot)),
        Instr::GoTo(offset) => return Err(format!("unexpected goto {:+}", offset)),
        Instr::GoToW(offset) => return Err(format!("unexpected goto_w {:+}", offset)),
        Instr::DupX2 => return Err("dup_x2 is not supported".to_string()),
        Instr::Dup2X1 => return Err("dup2_x1 is not supported".to_string()),
        Instr::Dup2X2 => return Err("dup2_x2 is not supported".to_string()),
        Instr::Breakpoint => return Err("breakpoint is not a real instruction".to_string()),
        Instr::ImpDep1 | Instr::ImpDep2 => {
            return Err("implementation dependent opcode".to_string())
        }

        Instr::IfEq(_)
        | Instr::IfNe(_)
        | Instr::IfLt(_)
        | Instr::IfGe(_)
        | Instr::IfGt(_)
        | Instr::IfLe(_)
        | Instr::IfNull(_)
        | Instr::IfNonNull(_)
        | Instr::IfICmpEq(_)
        | Instr::IfICmpNe(_)
        | Instr::IfICmpLt(_)
        | Instr::IfICmpGe(_)
        | Instr::IfICmpGt(_)
        | Instr::IfICmpLe(_)
        | Instr::IfACmpEq(_)
        | Instr::IfACmpNe(_) => {
            return Err(format!(
                "conditional branch {:?} reached apply() instead of condition() - \
                 this means it was not the last instruction in its basic block",
                insn
            ))
        }
    }

    Ok(())
}

fn field_type(member: &cp::MemberRef) -> Res<Type> {
    descriptor::parse_field(&member.descriptor)
        .ok_or_else(|| format!("bad field descriptor: {}", member.descriptor))
}

fn primitive_array_type(tag: u8) -> Res<Type> {
    Ok(match tag {
        4 => Type::Boolean,
        5 => Type::Char,
        6 => Type::Float,
        7 => Type::Double,
        8 => Type::Byte,
        9 => Type::Short,
        10 => Type::Int,
        11 => Type::Long,
        other => return Err(format!("unknown newarray type {}", other)),
    })
}

fn peel_array(ty: Type) -> (Type, usize) {
    let mut depth = 0;
    let mut current = ty;
    loop {
        match current {
            Type::Array(inner) => {
                depth += 1;
                current = *inner;
            }
            other => return (other, depth),
        }
    }
}

pub fn condition(insn: &Instr, frame: &mut Frame) -> Res<Expr> {
    Ok(match insn {
        Instr::IfEq(_) => against_zero(frame, BinOp::Eq)?,
        Instr::IfNe(_) => against_zero(frame, BinOp::Ne)?,
        Instr::IfLt(_) => against_zero(frame, BinOp::Lt)?,
        Instr::IfGe(_) => against_zero(frame, BinOp::Ge)?,
        Instr::IfGt(_) => against_zero(frame, BinOp::Gt)?,
        Instr::IfLe(_) => against_zero(frame, BinOp::Le)?,

        Instr::IfNull(_) => {
            let value = frame.pop()?;
            ast::null_compare(BinOp::Eq, value)
        }
        Instr::IfNonNull(_) => {
            let value = frame.pop()?;
            ast::null_compare(BinOp::Ne, value)
        }

        Instr::IfICmpEq(_) | Instr::IfACmpEq(_) => pair(frame, BinOp::Eq)?,
        Instr::IfICmpNe(_) | Instr::IfACmpNe(_) => pair(frame, BinOp::Ne)?,
        Instr::IfICmpLt(_) => pair(frame, BinOp::Lt)?,
        Instr::IfICmpGe(_) => pair(frame, BinOp::Ge)?,
        Instr::IfICmpGt(_) => pair(frame, BinOp::Gt)?,
        Instr::IfICmpLe(_) => pair(frame, BinOp::Le)?,

        other => return Err(format!("not a conditional branch: {:?}", other)),
    })
}

fn against_zero(frame: &mut Frame, op: BinOp) -> Res<Expr> {
    let value = frame.pop()?;
    Ok(ast::fold_zero_compare(op, value))
}

fn pair(frame: &mut Frame, op: BinOp) -> Res<Expr> {
    let right = frame.pop()?;
    let left = frame.pop()?;
    Ok(Expr::Binary {
        op,
        left: Box::new(left),
        right: Box::new(right),
        ty: Type::Boolean,
    })
}
