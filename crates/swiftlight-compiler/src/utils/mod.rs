// ユーティリティモジュール
// 様々なユーティリティ機能を提供します

pub mod config;
pub mod error_handling;
pub mod hash;
pub mod logger;
pub mod memory_tracker;
pub mod parallel;

// 他のモジュールに便利な機能をre-exportする
pub use config::CompilerConfig;
pub use error_handling::{CompilerError, CompilerResult, ErrorHandler, BasicErrorHandler};
pub use logger::{Logger, LogLevel, LogEntry, CompositeLogger, ConsoleLogger, FileLogger};
pub use memory_tracker::MemoryTracker;
pub use parallel::{ThreadPool, WorkQueue, Task, TaskPriority}; 