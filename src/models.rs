use crate::util::{deep_merge, kv_bulk_get_values};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use worker::kv::{KvError, KvStore};

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum Location {
    #[serde(rename = "indoor")]
    IndoorLocation { building: String, room: String },
    #[serde(rename = "outdoor")]
    OutdoorLocation { name: String },
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DaySchedule {
    pub start_time: String, // HH:mm format
    pub end_time: String,   // HH:mm format
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ScheduleCreate {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day1: Option<DaySchedule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day2: Option<DaySchedule>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ScheduleRead {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day1: Option<DaySchedule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day2: Option<DaySchedule>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ScheduleUpdate {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day1: Option<Option<DaySchedule>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day2: Option<Option<DaySchedule>>,
}

impl Into<ScheduleRead> for ScheduleCreate {
    fn into(self) -> ScheduleRead {
        ScheduleRead {
            day1: self.day1,
            day2: self.day2,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    Available,
    Limited,
    SoldOut,
}

impl Into<&str> for Availability {
    fn into(self) -> &'static str {
        match self {
            Availability::Available => "available",
            Availability::Limited => "limited",
            Availability::SoldOut => "sold_out",
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Product {
    pub name: String,
    pub price: u32,
    pub description: String,
    pub availability: Availability,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum PlanTypeCreate {
    Booth {},
    General {},
    Stage {},
    Labo { is_lab_tour: bool },
}

impl Into<PlanTypeRead> for PlanTypeCreate {
    fn into(self) -> PlanTypeRead {
        match self {
            PlanTypeCreate::Booth {} => PlanTypeRead::Booth {},
            PlanTypeCreate::General {} => PlanTypeRead::General {},
            PlanTypeCreate::Stage {} => PlanTypeRead::Stage {},
            PlanTypeCreate::Labo { is_lab_tour } => PlanTypeRead::Labo { is_lab_tour },
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum PlanTypeRead {
    Booth {},
    General {},
    Stage {},
    Labo { is_lab_tour: bool },
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum PlanTypeUpdate {
    Booth {},
    General {},
    Stage {},
    Labo {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        is_lab_tour: Option<bool>,
    },
}

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
}

pub enum PlanCreateError {
    Conflict,
    KvError(KvError),
}

impl From<KvError> for PlanCreateError {
    fn from(e: KvError) -> Self {
        PlanCreateError::KvError(e)
    }
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

impl PlanRead {
    pub async fn read(kv: KvStore, id: &str) -> Result<PlanRead, PlanReadError> {
        match kv.get(id).json::<PlanRead>().await? {
            Some(plan) => Ok(plan),
            None => Err(PlanReadError::NotFound),
        }
    }

    pub async fn read_all(kv: &KvStore) -> Result<Vec<PlanRead>, PlanReadError> {
        let mut values = vec![];

        loop {
            let list = kv.list().execute().await?;
            // bulk_getの最大数が100なのでkeyを100ごとに分割する
            for chunk in list.keys.chunks(100) {
                let keys = chunk
                    .iter()
                    .map(|key| key.name.clone())
                    .collect::<Vec<String>>();
                let values_chunk = kv_bulk_get_values::<PlanRead>(kv, keys.as_slice(), "json")
                    .await?
                    .into_values()
                    .collect::<Vec<Option<PlanRead>>>();
                let mut values_chunk = values_chunk
                    .into_iter()
                    .filter_map(|value| value)
                    .collect::<Vec<PlanRead>>();
                values.append(&mut values_chunk);
            }
            if list.list_complete {
                break;
            }
        }
        Ok(values)
    }
}

pub enum PlanUpdateError {
    NotFound,
    KvError(KvError),
    WorkerError(worker::Error),
    SerdeError(serde_json::Error),
}

impl From<KvError> for PlanUpdateError {
    fn from(e: KvError) -> Self {
        PlanUpdateError::KvError(e)
    }
}

impl From<worker::Error> for PlanUpdateError {
    fn from(e: worker::Error) -> Self {
        Self::WorkerError(e)
    }
}

impl From<serde_json::Error> for PlanUpdateError {
    fn from(e: serde_json::Error) -> Self {
        Self::SerdeError(e)
    }
}

impl PlanUpdate {
    pub async fn update(self, kv: KvStore, id: &str) -> Result<(), PlanUpdateError> {
        let Some(mut plan) = kv.get(id).json::<Value>().await? else {
            return Err(PlanUpdateError::NotFound);
        };

        let patch = serde_json::to_value(self.clone())?;
        deep_merge(&mut plan, patch);

        kv.put(id, plan)?.execute().await?;

        Ok(())
    }
}
