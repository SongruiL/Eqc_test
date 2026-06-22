//! 数据结构定义模块
//!
//! 定义方程 DSL 的核心数据结构。

mod equation;
mod equation_file;
mod parameter;
mod variable;

pub use equation::Equation;
pub use equation_file::{Calibration, EquationFile, Metadata};
pub use parameter::Parameter;
pub use variable::{DataType, VarClass, Variable, VariableType};
