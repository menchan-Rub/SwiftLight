// ユーティリティモジュール
// 様々なユーティリティ機能を提供します

pub mod config;
pub mod diagnostics;
pub mod error_handling;
pub mod hash;
pub mod logger;
pub mod memory_tracker;
pub mod parallel;
pub mod profiler;

// 他のモジュールに便利な機能をre-exportする
pub use config::CompilerConfig;
pub use diagnostics::{DiagnosticLevel, DiagnosticEmitter};
pub use error_handling::{CompilerError, CompilerResult, ErrorHandler, BasicErrorHandler};
pub use logger::{Logger, LogLevel, LogEntry, CompositeLogger, ConsoleLogger, FileLogger};
pub use memory_tracker::MemoryTracker;
pub use parallel::{ThreadPool, WorkQueue, Task, TaskPriority};
pub use profiler::{Profiler, ProfilingEvent}; 