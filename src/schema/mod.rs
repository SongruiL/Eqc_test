//! 数据结构定义模块
//!
//! 定义方程 DSL 的核心数据结构。

mod equation;
mod equation_file;
mod parameter;
mod structure;
mod variable;

pub use equation::{Equation, GpTarget};
pub use equation_file::{Calibration, EquationFile, Metadata};
pub use parameter::Parameter;
pub use structure::{EntityDecl, Instance, InstanceTag, StructureInfo, TopoEdge};
pub use variable::{DataType, VarClass, Variable, VariableType};
