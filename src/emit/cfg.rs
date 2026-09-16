use super::blocks::build_blocks;
use super::opcodes::Instruction as Instr;
use std::fmt::Write;

pub fn build_dot(method_name: &str, instructions: &[(usize, Instr)]) -> String {
    let blocks = build_blocks(instructions);
    let mut dot = String::new();

    let _ = writeln!(dot, "digraph \"{}\" {{", method_name);
    let _ = writeln!(dot, "  node [shape=box, fontname=monospace];");

    for block in &blocks {
        let _ = writeln!(
            dot,
            "  B{} [label=\"[{}, {})\"];",
            block.start, block.start, block.end
        );
    }

    for block in &blocks {
        if let Some(target) = block.branch_target {
            let label = if block.conditional {
                " [label=\"true\"]"
            } else {
                ""
            };
            let _ = writeln!(dot, "  B{} -> B{}{};", block.start, target, label);
        }
        if let Some(target) = block.fall_through {
            let label = if block.conditional {
                " [label=\"false\"]"
            } else {
                ""
            };
            let _ = writeln!(dot, "  B{} -> B{}{};", block.start, target, label);
        }
    }

    let _ = writeln!(dot, "}}");
    dot
}

pub fn render_comment(method_name: &str, tagged: &[(usize, Instr)]) -> String {
    if tagged.is_empty() {
        return String::new();
    }

    let mut comment = String::from("/* control flow graph (Graphviz DOT):\n");
    for line in build_dot(method_name, tagged).lines() {
        comment.push_str(line);
        comment.push('\n');
    }
    comment.push_str("*/\n");
    comment
}
