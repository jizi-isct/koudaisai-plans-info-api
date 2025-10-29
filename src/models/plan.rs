use crate::util::{deep_merge, kv_bulk_get_values};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use worker::kv::{KvError, KvStore};

use super::base::{Coordinates, Location};
use super::keys::{get_keys, GetKeysError};
use super::plan_type::{PlanTypeCreate, PlanTypeRead, PlanTypeUpdate};
use super::schedule::{ScheduleCreate, ScheduleRead, ScheduleUpdate};

#[derive(Serialize, Deserialize, Clone)]
pub struct PlanCreate {
    #[serde(flatten)]
    pub r#type: PlanTypeCreate,
    pub organization_name: String,
    pub plan_name: String,
    pub description: String,
    pub is_child_friendly: bool,
    pub is_recommended: bool,
    pub schedule: ScheduleCreate,
    pub location: Vec<Location>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coordinates: Option<Coordinates>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PlanRead {
    pub id: String,
    #[serde(flatten)]
    pub r#type: PlanTypeRead,
    pub organization_name: String,
    pub plan_name: String,
    pub description: String,
    pub is_child_friendly: bool,
    pub is_recommended: bool,
    pub schedule: ScheduleRead,
    pub location: Vec<Location>,
    #[serde(default)]
    pub coordinates: Option<Coordinates>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PlanUpdate {
    #[serde(default, skip_serializing_if = "Option::is_none", flatten)]
    pub r#type: Option<PlanTypeUpdate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organization_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_child_friendly: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_recommended: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule: Option<ScheduleUpdate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<Vec<Location>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coordinates: Option<Option<Coordinates>>,
}

#[derive(Error, Debug)]
pub enum PlanCreateError {
    #[error("Conflict")]
    Conflict,
    #[error(transparent)]
    KvError(#[from] KvError),
}

impl PlanCreate {
    pub async fn create(self, kv: KvStore, id: &str) -> Result<(), PlanCreateError> {
        // conflict check
        if kv.get(id).text().await? != None {
            return Err(PlanCreateError::Conflict);
        }

        // create
        kv.put(
            id,
            serde_json::to_string(&PlanRead {
                id: id.parse().unwrap(),
                r#type: self.r#type.into(),
                organization_name: self.organization_name,
                plan_name: self.plan_name,
                description: self.description,
                is_child_friendly: self.is_child_friendly,
                is_recommended: self.is_recommended,
                schedule: self.schedule.into(),
                location: self.location,
                coordinates: self.coordinates,
            })
            .unwrap(),
        )?
        .execute()
        .await?;

        Ok(())
    }
}

pub enum PlanReadError {
    NotFound,
    KvError(KvError),
    WorkerError(worker::Error),
    GetKeysError(GetKeysError),
}

impl From<KvError> for PlanReadError {
    fn from(e: KvError) -> Self {
        PlanReadError::KvError(e)
    }
}

impl From<worker::Error> for PlanReadError {
    fn from(e: worker::Error) -> Self {
        PlanReadError::WorkerError(e)
    }
}

impl From<GetKeysError> for PlanReadError {
    fn from(e: GetKeysError) -> Self {
        PlanReadError::GetKeysError(e)
    }
}

impl PlanRead {
    pub async fn read(kv: KvStore, id: &str) -> Result<PlanRead, PlanReadError> {
        match kv.get(id).json::<PlanRead>().await? {
            Some(plan) => Ok(plan),
            None => Err(PlanReadError::NotFound),
        }
    }

    pub async fn read_all(kv: &KvStore) -> Result<Vec<PlanRead>, PlanReadError> {
        let mut values = vec![];

        // Get keys from cache instead of direct kv.list()
        let all_keys = get_keys(kv).await?;

        // Filter out cache keys and process in chunks of 100
        let plan_keys: Vec<String> = all_keys
            .into_iter()
            .filter(|key| !key.starts_with("keys:"))
            .collect();

        // bulk_getの最大数が100なのでkeyを100ごとに分割する
        for chunk in plan_keys.chunks(100) {
            let values_chunk = kv_bulk_get_values::<PlanRead>(kv, chunk, "json")
                .await?
                .into_values()
                .collect::<Vec<Option<PlanRead>>>();
            let mut values_chunk = values_chunk
                .into_iter()
                .filter_map(|value| value)
                .collect::<Vec<PlanRead>>();
            values.append(&mut values_chunk);
        }

        values.sort_by(|a, b| a.id.cmp(&b.id));

        Ok(values)
    }
}

#[derive(Error, Debug)]
pub enum PlanUpdateError {
    #[error("Not found")]
    NotFound,
    #[error(transparent)]
    KvError(#[from] KvError),
    #[error(transparent)]
    WorkerError(#[from] worker::Error),
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
}

impl PlanUpdate {
    pub async fn update(self, kv: KvStore, id: &str) -> Result<(), PlanUpdateError> {
        let Some(mut plan) = kv.get(id).json::<Value>().await? else {
            return Err(PlanUpdateError::NotFound);
        };

        let patch = serde_json::to_value(self.clone())?;
        deep_merge(&mut plan, patch);

        kv.put(id, serde_json::to_string(&plan)?)?.execute().await?;

        Ok(())
    }
}
