use std::sync::Arc;

use futures::lock::Mutex;
use serenity::{
    client::{ClientBuilder, Context},
    model::guild::Member,
    prelude::TypeMapKey,
};

use crate::Document;

pub struct NicknameDb {
    prefix: char,
}

impl TypeMapKey for NicknameDb {
    type Value = Arc<NicknameDb>;
}

impl NicknameDb {
    pub async fn get_document(&self, member: Member) -> Arc<Mutex<Document>> {
        Arc::new(Mutex::new(Document::new(
            member.display_name().to_string(),
            self.prefix,
        )))
    }
}

pub trait SerenityInit {
    fn register_nicknamedb(self, prefix: char) -> Self;
}

impl SerenityInit for ClientBuilder<'_> {
    fn register_nicknamedb(self, prefix: char) -> Self {
        self.type_map_insert::<NicknameDb>(Arc::new(NicknameDb { prefix }))
    }
}

pub async fn get(ctx: &Context) -> Option<Arc<NicknameDb>> {
    let data = ctx.data.read().await;
    data.get::<NicknameDb>().cloned()
}
