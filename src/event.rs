use serde::{Deserialize, Serialize};

/// Represents an action broadcasted to the entire Grid
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub sender: String,
    pub action: String,
    pub content: String,
}