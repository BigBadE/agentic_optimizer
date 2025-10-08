/// Build validation stage
pub mod build;
/// Lint validation stage
pub mod lint;
/// Syntax validation stage
pub mod syntax;
/// Test validation stage
pub mod test;

pub use build::BuildValidationStage;
pub use lint::LintValidationStage;
pub use syntax::SyntaxValidationStage;
pub use test::TestValidationStage;
