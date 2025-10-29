use thiserror::Error;
use worker::kv::{KvError, KvStore};

#[derive(Error, Debug)]
pub enum PutKeysError {
    #[error(transparent)]
    WorkersError(#[from] worker::Error),
    #[error(transparent)]
    KvError(#[from] KvError),
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
}

pub async fn put_keys(kv: &KvStore) -> Result<(), PutKeysError> {
    let mut keys = vec![];
    loop {
        let list = kv.list().execute().await?;
        for key in list.keys {
            if key.name.starts_with("keys:") {
                continue;
            }
            keys.push(key.name);
        }
        if list.list_complete {
            break;
        }
    }

    keys.sort();
    kv.put("keys:all", serde_json::to_string(&keys)?)?
        .execute()
        .await?;

    Ok(())
}

#[derive(Error, Debug)]
pub enum GetKeysError {
    #[error(transparent)]
    KvError(#[from] KvError),
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
    #[error(transparent)]
    PutKeysError(#[from] PutKeysError),
}

pub async fn get_keys(kv: &KvStore) -> Result<Vec<String>, GetKeysError> {
    loop {
        match kv.get("keys:all").text().await? {
            Some(keys_json) => {
                let keys: Vec<String> = serde_json::from_str(&keys_json)?;
                return Ok(keys);
            }
            None => {
                // Cache miss - generate cache using put_keys
                put_keys(kv).await?;
            }
        }
    }
}
