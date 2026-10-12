use serde::{Serialize, Serializer};

/// Error type returned by Tauri commands; serialized as a plain message.
#[derive(Debug)]
pub struct CommandError(String);

impl<E: std::fmt::Display> From<E> for CommandError {
    fn from(err: E) -> Self {
        CommandError(err.to_string())
    }
}

impl Serialize for CommandError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

pub type CommandResult<T> = Result<T, CommandError>;
