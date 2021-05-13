mod document;
pub use document::{Document, KV};
mod serenity;
pub use crate::serenity::{get, NicknameDb, SerenityInit};
