use bril_rs::Type;
use strum_macros::{Display, EnumIter};

#[derive(Clone, Debug, PartialEq, Default)]
pub enum Order {
    Parallel,
    #[default]
    Sequential,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub enum Id {
    Unique(i64),
    #[default]
    Shared,
}

#[derive(Clone, Debug, PartialEq, EnumIter, Default, Display)]
pub enum PureBinOp {
    #[default]
    Add,
    Sub,
    Mul,
    LessThan,
    And,
    Or,
}

impl PureBinOp {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Add" => Some(PureBinOp::Add),
            "Sub" => Some(PureBinOp::Sub),
            "Mul" => Some(PureBinOp::Mul),
            "LessThan" => Some(PureBinOp::LessThan),
            "And" => Some(PureBinOp::And),
            "Or" => Some(PureBinOp::Or),
            _ => None,
        }
    }

    pub fn input_types(&self) -> (Type, Type) {
        match self {
            PureBinOp::Add | PureBinOp::Sub | PureBinOp::Mul | PureBinOp::LessThan => {
                (Type::Int, Type::Int)
            }
            PureBinOp::And | PureBinOp::Or => (Type::Bool, Type::Bool),
        }
    }

    pub fn output_type(&self) -> Type {
        match self {
            PureBinOp::Add | PureBinOp::Sub | PureBinOp::Mul => Type::Int,
            PureBinOp::LessThan | PureBinOp::And | PureBinOp::Or => Type::Bool,
        }
    }
}

#[derive(Clone, Debug, PartialEq, EnumIter, Default, Display)]
pub enum PureUnaryOp {
    #[default]
    Not,
}

impl PureUnaryOp {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Not" => Some(PureUnaryOp::Not),
            _ => None,
        }
    }

    pub fn input_type(&self) -> Type {
        match self {
            PureUnaryOp::Not => Type::Bool,
        }
    }

    pub fn output_type(&self) -> Type {
        match self {
            PureUnaryOp::Not => Type::Bool,
        }
    }
}

#[derive(Clone, Debug, PartialEq, EnumIter)]
pub enum Expr {
    Num(i64),
    Boolean(bool),
    BinOp(PureBinOp, Box<Expr>, Box<Expr>),
    UnaryOp(PureUnaryOp, Box<Expr>),
    Get(Box<Expr>, usize),
    /// Concat is a convenient built-in way
    /// to put two tuples together.
    /// It's not strictly necessary, but
    /// doing it by constructing a new big tuple is tedius and slow.
    Concat(Box<Expr>, Box<Expr>),
    Print(Box<Expr>),
    Read(Box<Expr>),
    Write(Box<Expr>, Box<Expr>),
    All(Id, Order, Vec<Expr>),
    /// A pred and a list of branches
    Switch(Box<Expr>, Vec<Expr>),
    /// Should only be a child of `Switch`
    /// Represents a single branch of a switch, giving
    /// it a unique id
    Branch(Id, Box<Expr>),
    Loop(Id, Box<Expr>, Box<Expr>),
    Let(Id, Box<Expr>, Box<Expr>),
    Arg(Id),
    Function(Id, String, TreeType, TreeType, Box<Expr>),
    /// A list of functions, with the first
    /// being the main function.
    Program(Vec<Expr>),
    Call(Id, Box<Expr>),
}

impl Default for Expr {
    fn default() -> Self {
        Expr::Num(0)
    }
}

impl Expr {
    /// Runs `func` on every child of this expression.
    pub fn for_each_child(&mut self, mut func: impl FnMut(&mut Expr)) {
        match self {
            Expr::Num(_) | Expr::Boolean(_) | Expr::Arg(_) => {}
            Expr::BinOp(_, a, b) => {
                func(a);
                func(b);
            }
            Expr::UnaryOp(_, a) => {
                func(a);
            }
            Expr::Concat(a, b) | Expr::Write(a, b) => {
                func(a);
                func(b);
            }
            Expr::Print(a) | Expr::Read(a) => {
                func(a);
            }
            Expr::Get(a, _) | Expr::Function(_, _, _, _, a) | Expr::Call(_, a) => {
                func(a);
            }
            Expr::All(_, _, children) => {
                for child in children {
                    func(child);
                }
            }
            Expr::Switch(input, children) => {
                func(input);
                for child in children {
                    func(child);
                }
            }
            Expr::Branch(_id, child) => {
                func(child);
            }
            Expr::Loop(_, pred, output) | Expr::Let(_, pred, output) => {
                func(pred);
                func(output);
            }
            Expr::Program(functions) => {
                for function in functions {
                    func(function);
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Num(i64),
    Boolean(bool),
    Tuple(Vec<Value>),
}

#[derive(Clone, PartialEq, Debug, Default)]
pub enum TreeType {
    #[default]
    Unit,
    Bril(Type),
    Tuple(Vec<TreeType>),
}

pub enum TypeError {
    ExpectedType(Expr, TreeType, TreeType),
    ExpectedTupleType(Expr, TreeType),
    ExpectedLoopOutputType(Expr, TreeType),
    NoArg(Expr),
}
