mod document;
pub use document::Document;
mod serenity;
pub use crate::serenity::{get, NicknameDb, SerenityInit};
