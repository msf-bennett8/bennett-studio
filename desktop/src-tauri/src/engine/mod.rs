
//! Engine launcher and process management modules
//! Desktop app spawns engine as a child process on startup

pub mod launcher;
pub use launcher::{start_engine, EngineProcess};