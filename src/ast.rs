use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileSpan<'a> {
    pub(crate) source: &'a str,
    pub(crate) chars: Range<usize>,
}

impl<'a> FileSpan<'a> {
    pub fn new(source: &'a str, chars: Range<usize>) -> FileSpan<'a> {
        FileSpan {
            source,
            chars,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UnaryOperator {
    Negative,
    BitwiseNot,
    LogicalNot,
    DerefData,
    DerefInsn,
    AddrOf,
    SizeOf,
    // Instruction, but not grammar:
    // - PreDecr, PostDecr, PreIncr, PostIncr,
    // - Popnt, clz, ctz, rnd
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BinaryOperator {
    Add,
    //Sub,
    Mul,
    //MulHigh??
    //Pow??
    //Root??
    //Div,
    //Rem,
    //ArrayElement, // Oof
    //ShiftRight,
    //ShiftLeft,
    //CmpLT,
    //CmpLE,
    //CmpGT,
    //CmpGE,
    //CmpEQ,
    //CmpNE,
    //BitwiseAnd,
    //BitwiseOr,
    //BitwiseXor,
    //LogicalAnd,
    //LogicalOr,
    //LogicalXor,
    // AssignFoo
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LocatedExpression<'a> {
    pub expr: Expression<'a>,
    pub span: FileSpan<'a>,
}

impl<'a> LocatedExpression<'a> {
    pub fn new(expr: Expression<'a>, span: FileSpan<'a>) -> Self {
        Self { expr, span }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expression<'a> {
    // If you add the ternary conditional, a puppy dies.
    Literal(u16),
    #[allow(dead_code)] Variable(&'a str),
    //Unary(UnaryOperator, Box<LocatedExpression<'a>>),
    //Binary(Box<LocatedExpression<'a>>, BinaryOperator, Box<LocatedExpression<'a>>),
    //FunctionCall(&'a str, Vec<LocatedExpression<'a>>),
    //Member(Box<LocatedExpression<'a>>, &'a str),
}
