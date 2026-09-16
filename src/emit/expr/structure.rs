use std::collections::BTreeSet;

use super::super::blocks::{self, Block};
use super::super::opcodes::Instruction as Instr;
use super::super::ConstantPool;
use super::ast::{self, Stmt};
use super::interp::{self, Frame, MethodContext, Res};

pub fn emit_body(
    tagged: &[(usize, Instr)],
    pool: &ConstantPool,
    ctx: &MethodContext,
) -> String {
    match structure(tagged, pool, ctx) {
        Ok(body) => ast::render_block(&body, 0),
        Err(reason) => listing(tagged, &reason),
    }
}

pub fn listing(tagged: &[(usize, Instr)], reason: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!("// could not reconstruct this method: {}\n", reason));
    out.push_str("/*\n");
    for (offset, insn) in tagged {
        out.push_str(&format!("{:>5}: {:?}\n", offset, insn));
    }
    out.push_str("*/\n");
    out
}

pub fn structure(
    tagged: &[(usize, Instr)],
    pool: &ConstantPool,
    ctx: &MethodContext,
) -> Res<Vec<Stmt>> {
    if tagged.is_empty() {
        return Ok(Vec::new());
    }

    let block_list = blocks::build_blocks(tagged);

    if block_list.iter().any(has_back_edge) {
        return Err("method contains a loop".to_string());
    }

    let start = block_list
        .first()
        .ok_or_else(|| "no basic blocks".to_string())?
        .start;

    let mut frame = Frame::new(ctx);
    let mut body = region(&block_list, tagged, start, None, &mut frame, pool, ctx)?;

    if !frame.stack.is_empty() {
        return Err(format!(
            "{} value(s) left on the stack at the end of the method",
            frame.stack.len()
        ));
    }

    if let Some(Stmt::Return(None)) = body.last() {
        body.pop();
    }

    Ok(body)
}

fn has_back_edge(block: &Block) -> bool {
    block.branch_target.map_or(false, |t| t <= block.start)
        || block.fall_through.map_or(false, |t| t <= block.start)
}

fn find_block<'b>(blocks: &'b [Block], offset: usize) -> Option<&'b Block> {
    blocks.iter().find(|block| block.start == offset)
}

fn slice_for<'t>(tagged: &'t [(usize, Instr)], block: &Block) -> &'t [(usize, Instr)] {
    let start = tagged
        .iter()
        .position(|(offset, _)| *offset == block.start)
        .unwrap_or(0);
    let end = tagged
        .iter()
        .position(|(offset, _)| *offset == block.end)
        .unwrap_or(tagged.len());
    &tagged[start..end.max(start)]
}

fn is_jump(insn: &Instr) -> bool {
    matches!(insn, Instr::GoTo(_) | Instr::GoToW(_))
}

fn successors(block: &Block) -> Vec<usize> {
    let mut out = Vec::new();
    if let Some(target) = block.branch_target {
        out.push(target);
    }
    if let Some(next) = block.fall_through {
        out.push(next);
    }
    out
}

fn reachable(blocks: &[Block], start: usize) -> BTreeSet<usize> {
    let mut seen = BTreeSet::new();
    let mut queue = vec![start];

    while let Some(offset) = queue.pop() {
        if !seen.insert(offset) {
            continue;
        }
        if let Some(block) = find_block(blocks, offset) {
            queue.extend(successors(block));
        }
    }

    seen
}

fn join_point(blocks: &[Block], left: usize, right: usize) -> Option<usize> {
    let from_left = reachable(blocks, left);
    let from_right = reachable(blocks, right);
    from_left.intersection(&from_right).next().copied()
}

fn region(
    blocks: &[Block],
    tagged: &[(usize, Instr)],
    start: usize,
    stop: Option<usize>,
    frame: &mut Frame,
    pool: &ConstantPool,
    ctx: &MethodContext,
) -> Res<Vec<Stmt>> {
    let mut body = Vec::new();
    let mut current = start;

    loop {
        if Some(current) == stop {
            return Ok(body);
        }

        let block = find_block(blocks, current)
            .ok_or_else(|| format!("no basic block starts at offset {}", current))?;
        let insns = slice_for(tagged, block);

        if !block.conditional {
            for (_, insn) in insns {
                if is_jump(insn) {
                    continue;
                }
                interp::apply(insn, frame, pool, ctx, &mut body)?;
            }

            current = match (block.branch_target, block.fall_through) {
                (Some(target), None) => target,
                (None, Some(next)) => next,
                (None, None) => return Ok(body),
                (Some(_), Some(_)) => {
                    return Err("unconditional block with two successors".to_string())
                }
            };
            continue;
        }

        let (straight, terminator) = insns.split_at(insns.len().saturating_sub(1));
        for (_, insn) in straight {
            interp::apply(insn, frame, pool, ctx, &mut body)?;
        }

        let branch = terminator
            .first()
            .ok_or_else(|| "conditional block is empty".to_string())?;
        let condition = interp::condition(&branch.1, frame)?;

        let taken = block
            .branch_target
            .ok_or_else(|| "conditional block without a branch target".to_string())?;
        let fallthrough = block
            .fall_through
            .ok_or_else(|| "conditional block without a fallthrough".to_string())?;

        let join = join_point(blocks, taken, fallthrough);
        let inner_stop = join.or(stop);
        let depth = frame.stack.len();

        let mut fall_frame = frame.clone();
        let fall_body = region(
            blocks,
            tagged,
            fallthrough,
            inner_stop,
            &mut fall_frame,
            pool,
            ctx,
        )?;

        let mut taken_frame = frame.clone();
        let taken_body = region(blocks, tagged, taken, inner_stop, &mut taken_frame, pool, ctx)?;

        let is_ternary = join.is_some()
            && fall_body.is_empty()
            && taken_body.is_empty()
            && fall_frame.stack.len() == depth + 1
            && taken_frame.stack.len() == depth + 1;

        if is_ternary {
            let fall_value = fall_frame
                .stack
                .pop()
                .ok_or_else(|| "missing ternary value".to_string())?;
            let taken_value = taken_frame
                .stack
                .pop()
                .ok_or_else(|| "missing ternary value".to_string())?;

            *frame = fall_frame;
            frame.merge_locals(&taken_frame);
            frame.push(ast::ternary(condition, taken_value, fall_value));

            current = join.ok_or_else(|| "ternary without a join".to_string())?;
            continue;
        }

        if fall_frame.stack.len() != taken_frame.stack.len() {
            return Err("branches leave different stack depths".to_string());
        }

        body.push(Stmt::If {
            cond: ast::negate(condition),
            then: fall_body,
            otherwise: taken_body,
        });

        *frame = fall_frame;
        frame.merge_locals(&taken_frame);

        match join {
            Some(next) => current = next,
            None => return Ok(body),
        }
    }
}
