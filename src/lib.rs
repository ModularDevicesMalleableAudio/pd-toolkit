pub mod analysis;
pub mod model;
pub mod parser;
pub mod rewrite;

// Re-export rewrite for tests and mutation code
pub use rewrite::serialize;
