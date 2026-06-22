//! DAG（有向无环图）构建模块

mod builder;

pub use builder::{build_dag, collapse_dag, Dag, DagEdge, DagLevel, DagNode, EdgeType, NodeType};
