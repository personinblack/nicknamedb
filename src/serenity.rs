use std::{collections::HashMap, sync::Arc};

use chrono::Duration;
use futures::lock::Mutex;
use serenity::{
    client::{ClientBuilder, Context},
    model::{
        guild::Member,
        id::{GuildId, UserId},
    },
    prelude::TypeMapKey,
};
use tokio::sync::RwLock;

use crate::Document;

type Documents = RwLock<HashMap<(UserId, GuildId), Arc<Mutex<Document>>>>;
pub struct NicknameDb {
    prefix: char,
    documents: Documents,
}

impl TypeMapKey for NicknameDb {
    type Value = Arc<NicknameDb>;
}

impl NicknameDb {
    pub async fn get_document(&self, member: Member) -> Arc<Mutex<Document>> {
        let mut documents = self.documents.write().await;
        let ug = (member.user.id, member.guild_id);
        if let Some(document) = documents.get(&ug) {
            let documentl = document.lock().await;
            if documentl.name == member.display_name().to_string() {
                return document.clone();
            }
        }

        let document = Arc::new(Mutex::new(Document::new(
            member.display_name().to_string(),
            self.prefix,
        )));
        documents.insert(ug, document);
        let document = documents.get(&ug).unwrap().clone();
        drop(documents);
        self.clear_cache().await;
        document
    }

    pub async fn remove_document(&self, member: Member) {
        let mut documents = self.documents.write().await;
        let ug = (member.user.id, member.guild_id);
        documents.remove(&ug);
    }

    async fn clear_cache(&self) {
        let mut to_remove: Vec<(UserId, GuildId)> = vec![];

        {
            let documents = self.documents.read().await;
            for (member, document) in documents.iter().step_by(2) {
                if let Some(document) = document.try_lock() {
                    if document.since_last_access().await > Duration::minutes(1) {
                        to_remove.push(*member);
                    }
                }
            }
        }

        let mut documents = self.documents.write().await;
        for member in to_remove {
            documents.remove(&member);
        }
    }
}

pub trait SerenityInit {
    fn register_nicknamedb(self, prefix: char) -> Self;
}

impl SerenityInit for ClientBuilder<'_> {
    fn register_nicknamedb(self, prefix: char) -> Self {
        self.type_map_insert::<NicknameDb>(Arc::new(NicknameDb {
            prefix,
            documents: Default::default(),
        }))
    }
}

pub async fn get(ctx: &Context) -> Option<Arc<NicknameDb>> {
    let data = ctx.data.read().await;
    data.get::<NicknameDb>().cloned()
}
