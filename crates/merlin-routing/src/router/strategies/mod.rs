/// Complexity-based routing strategy
pub mod complexity;
/// Long-context routing strategy
pub mod context;
/// Cost optimization strategy
pub mod cost;
/// Quality-critical routing strategy
pub mod quality;

pub use complexity::ComplexityBasedStrategy;
pub use context::LongContextStrategy;
pub use cost::CostOptimizationStrategy;
pub use quality::QualityCriticalStrategy;
