# javix

A Java `.class` decompiler written in Rust, with no dependencies.

```
cargo run -- Test.class
cargo run -- --cfg Test.class     # also dump each method's control flow graph
cargo test                        # descriptor and modified UTF-8 tests
```

## File structure

```
javix/
├── Cargo.toml
├── Test.java                    sample input
├── README.md
└── src/
    ├── main.rs                  CLI entry point: argument parsing, reads the
    │                            .class file, calls into reader:: and emit::
    ├── error.rs                 JavixError, the one error type the class
    │                            file reader returns
    │
    ├── reader/                  bytes -> ClassFile struct. No decompilation
    │   │                        logic lives here - just the file format.
    │   ├── mod.rs                 ClassFile, the MemberInfo trait
    │   ├── cursor.rs               ClassFileBuilder: drives parsing start to
    │   │                           end, checks the magic number
    │   ├── primitives.rs           read_u8/u16/u32/u64/bytes, all bounds
    │   │                           checked against the underlying slice
    │   ├── records.rs              parses the constant pool (incl. modified
    │   │                           UTF-8), interfaces, fields, methods,
    │   │                           attributes
    │   ├── pool.rs                 ConstantPool, CPIndexType - one entry per
    │   │                           constant pool tag
    │   ├── field.rs                FieldInfo
    │   ├── method.rs               MethodInfo
    │   └── attribute.rs            AttributeInfo (name already resolved)
    │
    └── emit/                    ClassFile -> Java source text
        ├── mod.rs                 generate_source(): class header, fields,
        │                          methods, in order. SHOW_CFG flag for --cfg
        ├── opcodes.rs              Instruction enum - one JVM opcode each,
        │                          decoded operands attached
        ├── decode.rs               bytes -> Vec<(offset, Instruction)>.
        │                          Bounds-checked, handles wide and both
        │                          switch instructions
        ├── blocks.rs               Instruction stream -> basic blocks (CFG
        │                          leaders/edges, no decompilation logic)
        ├── cfg.rs                  Graphviz DOT rendering of a method's CFG,
        │                          used by --cfg
        ├── constants.rs            Constant pool lookups that return
        │                          Result<_, String> instead of panicking:
        │                          utf8(), class_name(), member_ref(),
        │                          constant(), class_entry_type()
        ├── descriptor.rs           The one JVM descriptor parser (field and
        │                          method descriptors -> expr::kind::Type)
        ├── code.rs                 Parses the Code attribute: bytecode,
        │                          exception table presence, and the
        │                          LocalVariableTable (variable names/types
        │                          when compiled with -g)
        ├── header.rs                class/interface/enum header line:
        │                          modifiers, name, extends, implements
        ├── interfaces.rs           resolves an interface list to dotted
        │                          class names
        ├── fields.rs                field declarations, including
        │                          ConstantValue initialisers
        ├── methods.rs               MethodRenderer: drives one method's
        │                          signature + body, method list rendering
        ├── signature.rs             method signature line: modifiers,
        │                          return type, name, parameters, throws
        │
        └── expr/                 bytecode -> expression trees -> statements
            ├── mod.rs               re-exports ast, interp, kind, structure
            ├── kind.rs               Type: byte/char/.../Class/Array/Unknown
            ├── ast.rs                Expr and Stmt trees, precedence-correct
            │                        Display, and the folds that turn raw
            │                        bytecode idioms into real Java:
            │                        fold_zero_compare (dcmpl+ifle -> a > b),
            │                        ternary (cond?1:0 -> cond), negate
            ├── interp.rs             Symbolic interpreter: one instruction
            │                        in, an Expr pushed and/or a Stmt emitted
            │                        out. MethodContext/Frame carry parameter
            │                        and local-variable state. Nothing here
            │                        panics - every failure is a Result
            └── structure.rs          CFG walker: turns basic blocks into
                                     if/else (with a real join point) and
                                     ternaries by propagating the symbolic
                                     frame across edges instead of bailing
                                     out when the stack isn't empty at a
                                     block boundary. Falls back to a
                                     commented bytecode listing for anything
                                     it can't reconstruct (loops, switches,
                                     try/catch, invokedynamic, ...)
```

## How it works

```
bytes -> constant pool + members   reader/
      -> instructions              emit/decode.rs
      -> basic blocks / CFG        emit/blocks.rs
      -> expression trees          emit/expr/interp.rs
      -> statements                emit/expr/structure.rs
      -> Java source               emit/expr/ast.rs
```

The important property is that `interp.rs` pushes **expression trees** onto the
symbolic stack rather than instructions, and `structure.rs` carries that stack
along CFG edges rather than scanning the instruction list once. Together those
are what let a value produced by control flow - a ternary, or a materialised
`boolean` - be represented at all.

Two folds do most of the work on real javac output:

* `dcmpl` (or `lcmp`, `fcmpg`, ...) followed by `ifle` is one comparison,
  `a > b`, not two operations. `ast::fold_zero_compare` collapses them.
* `iconst_1 / goto / iconst_0` around a branch is javac materialising a
  `boolean`. `ast::ternary` folds `cond ? true : false` back to `cond`, which
  is also how the declared type comes out as `boolean` rather than `int`.

## What it handles

Constants, locals, arrays, arithmetic, conversions, field access, all four
`invoke` forms, `new`/`<init>` pairing including the `dup` patch-up,
`this()`/`super()`, returns, throws, `instanceof`, casts, `if`/`else` with a
real join point, and ternaries. Parenthesisation is driven by operator
precedence, so `(1 + 2) * 3` survives the round trip. Local names and types
come from the LocalVariableTable when the class was compiled with `-g`.

## What it does not handle yet

Loops, `tableswitch`/`lookupswitch`, try/catch, `jsr`/`ret`, `invokedynamic`
(lambdas and string concatenation on modern javac), `dup_x2`/`dup2_x1`/
`dup2_x2`, and short-circuit `&&`/`||`, which currently come out as nested
`if`s - correct, but not what was written.

A method it cannot reconstruct becomes a comment plus a bytecode listing rather
than a panic or, worse, confident nonsense.

## Known rough edge

A variable first assigned inside both arms of an `if` is declared inside each
arm, which is valid Java but scopes it there. If a later statement uses it the
output will not compile. The fix is a declaration-hoisting pass over the
finished statement tree.