// Modules
pub mod app;
pub mod transcribe;

// Exports
pub type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;
