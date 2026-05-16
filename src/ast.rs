use std::fmt::Debug;
use std::hash::Hash;
use std::ops::Deref;
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
pub struct Located<'a, T> where T: Debug + Clone + PartialEq + Eq + Hash {
    pub value: T,
    pub span: FileSpan<'a>,
}

impl<'a, T> Located<'a, T> where T: Debug + Clone + PartialEq + Eq + Hash {
    pub fn new(value: T, span: FileSpan<'a>) -> Self {
        Self { value, span }
    }
}

impl<'a, T> Deref for Located<'a, T> where T: Debug + Clone + PartialEq + Eq + Hash {
   type Target = T;
   fn deref(&self) -> &Self::Target { &self.value }
}

pub type LocatedExpression<'a> = Located<'a, Expression<'a>>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expression<'a> {
    // If you add the ternary conditional, a puppy dies.
    Literal(u16),
    #[allow(dead_code)] Variable(&'a str),
    #[allow(dead_code)] Unary(UnaryOperator, Box<LocatedExpression<'a>>),
    //Binary(Box<LocatedExpression<'a>>, BinaryOperator, Box<LocatedExpression<'a>>),
    //FunctionCall(&'a str, Vec<LocatedExpression<'a>>),
    //Member(Box<LocatedExpression<'a>>, &'a str),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Statement<'a> {
    Yield(LocatedExpression<'a>),
    Assign(Located<'a, &'a str>, LocatedExpression<'a>),
    // Assignment
    // Functioncall?!
    // IfThen
    // IfThenElse
    // WhileDo
    // Break
    // Continue
    // For
}

pub type LocatedStatement<'a> = Located<'a, Statement<'a>>;
