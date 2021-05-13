use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use futures::lock::Mutex;
use regex::Regex;

pub struct KV {
    key: char,
    value: String,
}

impl ToString for KV {
    fn to_string(&self) -> String {
        format!("{}{}", self.key, self.value)
    }
}

impl From<String> for KV {
    fn from(mut serialized: String) -> Self {
        Self {
            key: serialized.remove(0),
            value: serialized,
        }
    }
}

impl Clone for KV {
    fn clone(&self) -> Self {
        Self {
            key: self.key,
            value: self.value.clone(),
        }
    }
}

#[derive(Debug)]
pub struct Document {
    pub name: String,
    prefix: char,
    regex: Regex,
    last_access: Arc<Mutex<DateTime<Utc>>>,
}

impl Document {
    pub fn new(name: String, prefix: char) -> Self {
        Self {
            name,
            prefix,
            regex: Regex::new(r"(\^(?:\w)(?:\w+)){1}").expect("regex"),
            last_access: Arc::new(Mutex::new(Utc::now())),
        }
    }

    pub async fn insert<T: Into<String>>(&mut self, key: char, value: T) {
        *self.last_access.lock().await = Utc::now();
        if self.exists(key) {
            return;
        }

        let mut kv_chain = self.fetch_all().unwrap_or_else(Vec::new);
        kv_chain.push(KV {
            key,
            value: value.into(),
        });

        self.push_kv(kv_chain);
    }

    pub async fn update<T: Into<String>>(&mut self, key: char, value: T) {
        *self.last_access.lock().await = Utc::now();
        if !self.exists(key) {
            return;
        }

        let value = value.into();
        let kv_chain = self
            .fetch_all()
            .unwrap()
            .iter()
            .map(|kv| {
                if kv.key == key {
                    KV {
                        key,
                        value: value.clone(),
                    }
                } else {
                    kv.clone()
                }
            })
            .collect::<Vec<KV>>();

        self.push_kv(kv_chain);
    }

    pub async fn delete<T: Into<String> + Clone>(&mut self, key: char, value: Option<T>) {
        *self.last_access.lock().await = Utc::now();
        if !self.exists(key) {
            return;
        }

        let kv_chain = self
            .fetch_all()
            .unwrap()
            .iter()
            .cloned()
            .filter(|kv| {
                if kv.key == key {
                    if let Some(value) = value.clone() {
                        return kv.value != value.into();
                    }

                    return false;
                }

                true
            })
            .collect::<Vec<KV>>();

        self.push_kv(kv_chain);
    }

    pub async fn fetch(&self, key: char) -> Option<&str> {
        *self.last_access.lock().await = Utc::now();
        if !self.exists(key) {
            return None;
        }

        let result = self.regex.find_iter(&self.name).find(|mat| {
            let mut kv = mat.as_str().to_string();
            kv.remove(0);
            let matkey = kv.remove(0);

            matkey == key
        });

        if let Some(result) = result {
            Some(result.as_str().split_at(2).1)
        } else {
            None
        }
    }

    pub fn exists(&self, key: char) -> bool {
        self.name.contains(&format!("{}{}", self.prefix, key))
    }

    pub async fn since_last_access(&self) -> Duration {
        Utc::now() - *self.last_access.lock().await
    }

    fn fetch_all(&self) -> Option<Vec<KV>> {
        let nick = &self.name;
        if !self.regex.is_match(&nick) {
            return None;
        }

        Some(
            self.regex
                .find_iter(&nick)
                .map(|mat| {
                    let mut kv = mat.as_str().to_string();
                    kv.remove(0);
                    let key = kv.remove(0);
                    KV { key, value: kv }
                })
                .collect::<Vec<KV>>(),
        )
    }

    fn push_kv(&mut self, kv: Vec<KV>) {
        let kv_string = kv
            .iter()
            .map(|kv| "^".to_owned() + &kv.to_string())
            .collect::<String>();

        let name_current = &self.name;
        let name_new = self.regex.replace_all(&name_current, "");
        let name_new = name_new.to_string().trim().to_owned() + " " + &kv_string;

        self.name = name_new;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn insert() {
        let mut document = Document::new("menfie".to_owned(), '^');
        document.insert('A', "FOO").await;
        document.insert('b', "BAR").await;
        assert_eq!(document.name, "menfie ^AFOO^bBAR");
    }

    #[tokio::test]
    async fn update() {
        let mut document = Document::new("menfie".to_owned(), '^');
        document.insert('A', "FOO").await;
        document.insert('b', "BAR").await;
        document.update('A', "BAZ").await;
        document.update('b', "FOO").await;
        assert_eq!(document.name, "menfie ^ABAZ^bFOO");
    }

    #[tokio::test]
    async fn delete() {
        let mut document = Document::new("menfie".to_owned(), '^');
        document.insert('A', "FOO").await;
        document.insert('b', "BAR").await;
        document.delete::<String>('A', None).await;
        document.insert('A', "FOO").await;
        document.delete('A', Some("FOO")).await;
        assert_eq!(document.name, "menfie ^bBAR");
    }

    #[tokio::test]
    async fn fetch() {
        let mut document = Document::new("menfie".to_owned(), '^');
        document.insert('A', "FOO").await;
        assert_eq!(document.fetch('A').await, Some("FOO"));
    }
}
