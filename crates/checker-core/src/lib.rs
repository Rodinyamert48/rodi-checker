//! Rodi Checker core library.
//!
//! Everything here is independent of Tauri so it can be shared by the desktop
//! app and the `rodi-checker` CLI, and tested without a GUI.

pub mod checker;
pub mod error;
pub mod runner;
pub mod scheduler;
pub mod settings;
pub mod storage;

pub use checker::{CheckContext, CheckKind, CheckResult, Checker, Status};
pub use error::{CoreError, Result};
pub use runner::{JobRunner, Progress};
pub use scheduler::Scheduler;
pub use settings::AppSettings;
pub use storage::Storage;
