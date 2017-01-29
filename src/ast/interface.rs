use ast::*;
use span::Span;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Interface
{
    pub name: String,
    pub functions: Vec<FunctionSignature>,
    pub span: Span,
}

impl Interface
{
    pub fn to_type(&self) -> InterfaceType
    {
        InterfaceType{
            name: self.name.clone(),
            functions: self.functions.clone(),
        }
    }
}

pub fn interface(name: String, functions: Vec<FunctionSignature>, span: Span) -> Interface
{
    Interface{
        name: name,
        functions: functions,
        span: span,
    }
}

impl TreePrinter for Interface
{
    fn print(&self, level: usize)
    {
        let p = prefix(level);
        println!("{}interface {} ({})", p, self.name, self.span);
        for func in &self.functions {
            func.print(level + 1);
        }
    }
}
