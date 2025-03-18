// ユーティリティモジュール
// コンパイラ全体で使用される共通のユーティリティ機能を提供します

pub mod profiler;
pub mod file_system;
pub mod parallel;
pub mod memory_tracker;
pub mod hash;
pub mod logging;
pub mod error_formatter;
pub mod string_interner;
pub mod arena;
pub mod config_parser;

// 再エクスポート
pub use self::profiler::{Profiler, ProfilingEvent, ProfilingScope};
pub use self::file_system::{FileSystem, VirtualFileSystem, FileWatcher, FileChangeEvent};
pub use self::parallel::{WorkQueue, Task, TaskPriority, ThreadPool};
pub use self::memory_tracker::{MemoryTracker, MemoryUsageSnapshot};
pub use self::hash::{HashAlgorithm, ContentHasher};
pub use self::logging::{Logger, LogLevel, LogMessage};
pub use self::error_formatter::{ErrorFormatter, FormattingOptions};
pub use self::string_interner::StringInterner;
pub use self::arena::{Arena, TypedArena};
pub use self::config_parser::ConfigParser; 