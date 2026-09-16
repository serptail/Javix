use super::opcodes::Instruction as Instr;
use std::collections::BTreeSet;

pub struct Block {
    pub start: usize,
    pub end: usize,
    pub branch_target: Option<usize>,
    pub conditional: bool,
    pub fall_through: Option<usize>,
}

pub fn branch_target(offset: usize, delta: i32) -> usize {
    (offset as i64 + delta as i64) as usize
}

pub fn jump_target(insn: &Instr, offset: usize) -> Option<usize> {
    let delta = match insn {
        Instr::IfACmpEq(t)
        | Instr::IfACmpNe(t)
        | Instr::IfICmpEq(t)
        | Instr::IfICmpGe(t)
        | Instr::IfICmpGt(t)
        | Instr::IfICmpLe(t)
        | Instr::IfICmpLt(t)
        | Instr::IfICmpNe(t)
        | Instr::IfEq(t)
        | Instr::IfGe(t)
        | Instr::IfGt(t)
        | Instr::IfLe(t)
        | Instr::IfLt(t)
        | Instr::IfNe(t)
        | Instr::IfNonNull(t)
        | Instr::IfNull(t)
        | Instr::GoTo(t) => *t as i32,
        Instr::GoToW(t) => *t,
        _ => return None,
    };

    Some(branch_target(offset, delta))
}

pub fn is_conditional_branch(insn: &Instr) -> bool {
    !matches!(insn, Instr::GoTo(_) | Instr::GoToW(_))
}

pub fn falls_through(insn: &Instr) -> bool {
    !matches!(
        insn,
        Instr::GoTo(_)
            | Instr::GoToW(_)
            | Instr::Return
            | Instr::AReturn
            | Instr::DReturn
            | Instr::FReturn
            | Instr::IReturn
            | Instr::LReturn
            | Instr::AThrow
            | Instr::TableSwitch { .. }
            | Instr::LookupSwitch { .. }
    )
}

fn find_leaders(instructions: &[(usize, Instr)]) -> BTreeSet<usize> {
    let mut leaders = BTreeSet::new();

    if let Some((first_offset, _)) = instructions.first() {
        leaders.insert(*first_offset);
    }

    for (i, (offset, insn)) in instructions.iter().enumerate() {
        let next_offset = instructions.get(i + 1).map(|(o, _)| *o);

        if let Some(target) = jump_target(insn, *offset) {
            leaders.insert(target);
            if let Some(next) = next_offset {
                leaders.insert(next);
            }
        } else if !falls_through(insn) {
            if let Some(next) = next_offset {
                leaders.insert(next);
            }
        }
    }

    leaders
}

pub fn build_blocks(instructions: &[(usize, Instr)]) -> Vec<Block> {
    let leaders: Vec<usize> = find_leaders(instructions).into_iter().collect();
    let mut blocks = Vec::with_capacity(leaders.len());

    for (i, &start) in leaders.iter().enumerate() {
        let end = leaders.get(i + 1).copied();

        let block_instructions: Vec<&(usize, Instr)> = instructions
            .iter()
            .filter(|(offset, _)| *offset >= start && end.map_or(true, |e| *offset < e))
            .collect();

        let terminator = block_instructions.last();

        let (branch_target, conditional, fall_through) = match terminator {
            Some((offset, insn)) => {
                let target = jump_target(insn, *offset);
                let conditional = target.is_some() && is_conditional_branch(insn);
                let fall_through = if falls_through(insn) { end } else { None };
                (target, conditional, fall_through)
            }
            None => (None, false, end),
        };

        blocks.push(Block {
            start,
            end: end.unwrap_or_else(|| instructions.last().map_or(start, |(o, _)| *o + 1)),
            branch_target,
            conditional,
            fall_through,
        });
    }

    blocks
}
