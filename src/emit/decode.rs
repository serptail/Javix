use super::opcodes::Instruction;

pub type Res<T> = Result<T, String>;

struct Stream<'a> {
    bytes: &'a [u8],
    index: usize,
}

impl<'a> Stream<'a> {
    fn u8(&mut self) -> Res<u8> {
        let byte = *self
            .bytes
            .get(self.index)
            .ok_or_else(|| format!("code ends mid-instruction at offset {}", self.index))?;
        self.index += 1;
        Ok(byte)
    }

    fn i8(&mut self) -> Res<i8> {
        Ok(self.u8()? as i8)
    }

    fn u16(&mut self) -> Res<u16> {
        let high = self.u8()? as u16;
        let low = self.u8()? as u16;
        Ok((high << 8) | low)
    }

    fn i16(&mut self) -> Res<i16> {
        Ok(self.u16()? as i16)
    }

    fn i32(&mut self) -> Res<i32> {
        let high = self.u16()? as u32;
        let low = self.u16()? as u32;
        Ok(((high << 16) | low) as i32)
    }

    fn align(&mut self) {
        while self.index % 4 != 0 {
            self.index += 1;
        }
    }
}

pub fn decode_with_offsets(code: &[u8]) -> Res<Vec<(usize, Instruction)>> {
    let mut instructions = Vec::new();
    let mut stream = Stream {
        bytes: code,
        index: 0,
    };

    while stream.index < code.len() {
        let offset = stream.index;
        let insn = decode_one(&mut stream, false)?;
        instructions.push((offset, insn));
    }

    Ok(instructions)
}

fn read_slot(s: &mut Stream<'_>, wide: bool) -> Res<u16> {
    if wide {
        s.u16()
    } else {
        Ok(s.u8()? as u16)
    }
}

fn decode_one(s: &mut Stream<'_>, wide: bool) -> Res<Instruction> {
    Ok(match s.u8()? {
        0x00 => Instruction::Nop,
        0x01 => Instruction::AConstNull,
        0x02 => Instruction::IConstM1,
        0x03 => Instruction::IConst0,
        0x04 => Instruction::IConst1,
        0x05 => Instruction::IConst2,
        0x06 => Instruction::IConst3,
        0x07 => Instruction::IConst4,
        0x08 => Instruction::IConst5,
        0x09 => Instruction::LConst0,
        0x0a => Instruction::LConst1,
        0x0b => Instruction::FConst0,
        0x0c => Instruction::FConst1,
        0x0d => Instruction::FConst2,
        0x0e => Instruction::DConst0,
        0x0f => Instruction::DConst1,
        0x10 => Instruction::BiPush(s.i8()?),
        0x11 => Instruction::SiPush(s.i16()?),
        0x12 => Instruction::Ldc(s.u8()?),
        0x13 => Instruction::LdcW(s.u16()?),
        0x14 => Instruction::Ldc2W(s.u16()?),
        0x15 => Instruction::ILoad(read_slot(s, wide)?),
        0x16 => Instruction::LLoad(read_slot(s, wide)?),
        0x17 => Instruction::FLoad(read_slot(s, wide)?),
        0x18 => Instruction::DLoad(read_slot(s, wide)?),
        0x19 => Instruction::ALoad(read_slot(s, wide)?),
        0x1a => Instruction::ILoad0,
        0x1b => Instruction::ILoad1,
        0x1c => Instruction::ILoad2,
        0x1d => Instruction::ILoad3,
        0x1e => Instruction::LLoad0,
        0x1f => Instruction::LLoad1,
        0x20 => Instruction::LLoad2,
        0x21 => Instruction::LLoad3,
        0x22 => Instruction::FLoad0,
        0x23 => Instruction::FLoad1,
        0x24 => Instruction::FLoad2,
        0x25 => Instruction::FLoad3,
        0x26 => Instruction::DLoad0,
        0x27 => Instruction::DLoad1,
        0x28 => Instruction::DLoad2,
        0x29 => Instruction::DLoad3,
        0x2a => Instruction::ALoad0,
        0x2b => Instruction::ALoad1,
        0x2c => Instruction::ALoad2,
        0x2d => Instruction::ALoad3,
        0x2e => Instruction::IALoad,
        0x2f => Instruction::LALoad,
        0x30 => Instruction::FALoad,
        0x31 => Instruction::DALoad,
        0x32 => Instruction::AALoad,
        0x33 => Instruction::BALoad,
        0x34 => Instruction::CALoad,
        0x35 => Instruction::SALoad,
        0x36 => Instruction::IStore(read_slot(s, wide)?),
        0x37 => Instruction::LStore(read_slot(s, wide)?),
        0x38 => Instruction::FStore(read_slot(s, wide)?),
        0x39 => Instruction::DStore(read_slot(s, wide)?),
        0x3a => Instruction::AStore(read_slot(s, wide)?),
        0x3b => Instruction::IStore0,
        0x3c => Instruction::IStore1,
        0x3d => Instruction::IStore2,
        0x3e => Instruction::IStore3,
        0x3f => Instruction::LStore0,
        0x40 => Instruction::LStore1,
        0x41 => Instruction::LStore2,
        0x42 => Instruction::LStore3,
        0x43 => Instruction::FStore0,
        0x44 => Instruction::FStore1,
        0x45 => Instruction::FStore2,
        0x46 => Instruction::FStore3,
        0x47 => Instruction::DStore0,
        0x48 => Instruction::DStore1,
        0x49 => Instruction::DStore2,
        0x4a => Instruction::DStore3,
        0x4b => Instruction::AStore0,
        0x4c => Instruction::AStore1,
        0x4d => Instruction::AStore2,
        0x4e => Instruction::AStore3,
        0x4f => Instruction::IAStore,
        0x50 => Instruction::LAStore,
        0x51 => Instruction::FAStore,
        0x52 => Instruction::DAStore,
        0x53 => Instruction::AAStore,
        0x54 => Instruction::BAStore,
        0x55 => Instruction::CAStore,
        0x56 => Instruction::SAStore,
        0x57 => Instruction::Pop,
        0x58 => Instruction::Pop2,
        0x59 => Instruction::Dup,
        0x5a => Instruction::DupX1,
        0x5b => Instruction::DupX2,
        0x5c => Instruction::Dup2,
        0x5d => Instruction::Dup2X1,
        0x5e => Instruction::Dup2X2,
        0x5f => Instruction::Swap,
        0x60 => Instruction::IAdd,
        0x61 => Instruction::LAdd,
        0x62 => Instruction::FAdd,
        0x63 => Instruction::DAdd,
        0x64 => Instruction::ISub,
        0x65 => Instruction::LSub,
        0x66 => Instruction::FSub,
        0x67 => Instruction::DSub,
        0x68 => Instruction::IMul,
        0x69 => Instruction::LMul,
        0x6a => Instruction::FMul,
        0x6b => Instruction::DMul,
        0x6c => Instruction::IDiv,
        0x6d => Instruction::LDiv,
        0x6e => Instruction::FDiv,
        0x6f => Instruction::DDiv,
        0x70 => Instruction::IRem,
        0x71 => Instruction::LRem,
        0x72 => Instruction::FRem,
        0x73 => Instruction::DRem,
        0x74 => Instruction::INeg,
        0x75 => Instruction::LNeg,
        0x76 => Instruction::FNeg,
        0x77 => Instruction::DNeg,
        0x78 => Instruction::IShl,
        0x79 => Instruction::LShl,
        0x7a => Instruction::IShr,
        0x7b => Instruction::LShr,
        0x7c => Instruction::IUShr,
        0x7d => Instruction::LUShr,
        0x7e => Instruction::IAnd,
        0x7f => Instruction::LAnd,
        0x80 => Instruction::IOr,
        0x81 => Instruction::LOr,
        0x82 => Instruction::IXor,
        0x83 => Instruction::LXor,
        0x84 => {
            let index = read_slot(s, wide)?;
            let amount = if wide { s.i16()? } else { s.i8()? as i16 };
            Instruction::IInc(index, amount)
        }
        0x85 => Instruction::I2L,
        0x86 => Instruction::I2F,
        0x87 => Instruction::I2D,
        0x88 => Instruction::L2I,
        0x89 => Instruction::L2F,
        0x8a => Instruction::L2D,
        0x8b => Instruction::F2I,
        0x8c => Instruction::F2L,
        0x8d => Instruction::F2D,
        0x8e => Instruction::D2I,
        0x8f => Instruction::D2L,
        0x90 => Instruction::D2F,
        0x91 => Instruction::I2B,
        0x92 => Instruction::I2C,
        0x93 => Instruction::I2S,
        0x94 => Instruction::LCmp,
        0x95 => Instruction::FCmpL,
        0x96 => Instruction::FCmpG,
        0x97 => Instruction::DCmpL,
        0x98 => Instruction::DCmpG,
        0x99 => Instruction::IfEq(s.i16()?),
        0x9a => Instruction::IfNe(s.i16()?),
        0x9b => Instruction::IfLt(s.i16()?),
        0x9c => Instruction::IfGe(s.i16()?),
        0x9d => Instruction::IfGt(s.i16()?),
        0x9e => Instruction::IfLe(s.i16()?),
        0x9f => Instruction::IfICmpEq(s.i16()?),
        0xa0 => Instruction::IfICmpNe(s.i16()?),
        0xa1 => Instruction::IfICmpLt(s.i16()?),
        0xa2 => Instruction::IfICmpGe(s.i16()?),
        0xa3 => Instruction::IfICmpGt(s.i16()?),
        0xa4 => Instruction::IfICmpLe(s.i16()?),
        0xa5 => Instruction::IfACmpEq(s.i16()?),
        0xa6 => Instruction::IfACmpNe(s.i16()?),
        0xa7 => Instruction::GoTo(s.i16()?),
        0xa8 => Instruction::Jsr(s.i16()?),
        0xa9 => Instruction::Ret(read_slot(s, wide)?),
        0xaa => {
            s.align();
            let default = s.i32()?;
            let low = s.i32()?;
            let high = s.i32()?;
            if high < low {
                return Err(format!("tableswitch with high {} below low {}", high, low));
            }
            let count = (high - low + 1) as usize;
            let mut targets = Vec::with_capacity(count);
            for _ in 0..count {
                targets.push(s.i32()?);
            }
            Instruction::TableSwitch {
                default,
                low,
                targets,
            }
        }
        0xab => {
            s.align();
            let default = s.i32()?;
            let count = s.i32()?;
            if count < 0 {
                return Err(format!("lookupswitch with negative count {}", count));
            }
            let mut pairs = Vec::with_capacity(count as usize);
            for _ in 0..count {
                pairs.push((s.i32()?, s.i32()?));
            }
            Instruction::LookupSwitch { default, pairs }
        }
        0xac => Instruction::IReturn,
        0xad => Instruction::LReturn,
        0xae => Instruction::FReturn,
        0xaf => Instruction::DReturn,
        0xb0 => Instruction::AReturn,
        0xb1 => Instruction::Return,
        0xb2 => Instruction::GetStatic(s.u16()?),
        0xb3 => Instruction::PutStatic(s.u16()?),
        0xb4 => Instruction::GetField(s.u16()?),
        0xb5 => Instruction::PutField(s.u16()?),
        0xb6 => Instruction::InvokeVirtual(s.u16()?),
        0xb7 => Instruction::InvokeSpecial(s.u16()?),
        0xb8 => Instruction::InvokeStatic(s.u16()?),
        0xb9 => {
            let index = s.u16()?;
            s.u8()?;
            s.u8()?;
            Instruction::InvokeInterface(index)
        }
        0xba => {
            let index = s.u16()?;
            s.u8()?;
            s.u8()?;
            Instruction::InvokeDynamic(index)
        }
        0xbb => Instruction::New(s.u16()?),
        0xbc => Instruction::NewArray(s.u8()?),
        0xbd => Instruction::ANewArray(s.u16()?),
        0xbe => Instruction::ArrayLength,
        0xbf => Instruction::AThrow,
        0xc0 => Instruction::CheckCast(s.u16()?),
        0xc1 => Instruction::InstanceOf(s.u16()?),
        0xc2 => Instruction::MonitorEnter,
        0xc3 => Instruction::MonitorExit,
        0xc4 => {
            if wide {
                return Err("wide prefix applied to another wide prefix".to_string());
            }
            return decode_one(s, true);
        }
        0xc5 => Instruction::MultiANewArray(s.u16()?, s.u8()?),
        0xc6 => Instruction::IfNull(s.i16()?),
        0xc7 => Instruction::IfNonNull(s.i16()?),
        0xc8 => Instruction::GoToW(s.i32()?),
        0xc9 => Instruction::JsrW(s.i32()?),
        0xca => Instruction::Breakpoint,
        0xfe => Instruction::ImpDep1,
        0xff => Instruction::ImpDep2,
        opcode => return Err(format!("unknown opcode {:#04x}", opcode)),
    })
}
