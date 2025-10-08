use crate::util::{deep_merge, kv_bulk_get_values};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use worker::kv::{KvError, KvStore};

#[derive(Serialize, Deserialize, Clone)]
pub struct Coordinates {
    pub latitude: f64,
    pub longitude: f64,
}

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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Product {
    pub name: String,
    pub price: u32,
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum BoothPlanCategory {
    MainRice,
    MainNoodleFlour,
    MainSkewerGrill,
    MainHotSnack,
    MainSoup,
    MainWorldStreet,
    SweetJapanese,
    SweetWestern,
    SweetCold,
    SweetSnack,
    SweetDrink,
    SweetWorld,
    Drink,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum GeneralPlanCategory {
    Play,
    Display,
    Performance,
    Cafe,
    Rest,
    Presentation,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum PlanTypeCreate {
    Booth {
        categories: Vec<BoothPlanCategory>,
    },
    General {
        categories: Vec<GeneralPlanCategory>,
    },
    Stage {},
    Labo {
        is_lab_tour: bool,
    },
}

impl Into<PlanTypeRead> for PlanTypeCreate {
    fn into(self) -> PlanTypeRead {
        match self {
            PlanTypeCreate::Booth { categories } => PlanTypeRead::Booth { categories },
            PlanTypeCreate::General { categories } => PlanTypeRead::General { categories },
            PlanTypeCreate::Stage {} => PlanTypeRead::Stage {},
            PlanTypeCreate::Labo { is_lab_tour } => PlanTypeRead::Labo { is_lab_tour },
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum PlanTypeRead {
    Booth {
        categories: Vec<BoothPlanCategory>,
    },
    General {
        categories: Vec<GeneralPlanCategory>,
    },
    Stage {},
    Labo {
        is_lab_tour: bool,
    },
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum PlanTypeUpdate {
    Booth {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        categories: Option<Vec<BoothPlanCategory>>,
    },
    General {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        categories: Option<Vec<GeneralPlanCategory>>,
    },
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

// PlanDetails models
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreatePlanDetails {
    pub products: Vec<Product>,
    pub additional_info: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReadPlanDetails {
    pub products: Vec<Product>,
    pub additional_info: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdatePlanDetails {
    pub products: Option<Vec<Product>>,
    pub additional_info: Option<String>,
}

#[derive(Error, Debug)]
pub enum PlanDetailsCreateError {
    #[error("Conflict")]
    Conflict,
    #[error(transparent)]
    KvError(#[from] KvError),
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
}

impl CreatePlanDetails {
    pub async fn create(self, kv: KvStore, id: &str) -> Result<(), PlanDetailsCreateError> {
        // Check if plan details already exists
        if let Some(_) = kv.get(id).json::<ReadPlanDetails>().await? {
            return Err(PlanDetailsCreateError::Conflict);
        }

        let plan_details = ReadPlanDetails {
            products: self.products,
            additional_info: self.additional_info,
        };

        kv.put(id, serde_json::to_string(&plan_details)?)?
            .execute()
            .await?;

        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum PlanDetailsReadError {
    #[error("Not found")]
    NotFound,
    #[error(transparent)]
    KvError(#[from] KvError),
}

impl ReadPlanDetails {
    pub async fn read(kv: KvStore, id: &str) -> Result<ReadPlanDetails, PlanDetailsReadError> {
        match kv.get(id).json::<ReadPlanDetails>().await? {
            Some(plan_details) => Ok(plan_details),
            None => Err(PlanDetailsReadError::NotFound),
        }
    }
}

#[derive(Error, Debug)]
pub enum PlanDetailsUpdateError {
    #[error("Not found")]
    NotFound,
    #[error(transparent)]
    KvError(#[from] KvError),
    #[error(transparent)]
    WorkerError(#[from] worker::Error),
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
}

impl UpdatePlanDetails {
    pub async fn update(self, kv: KvStore, id: &str) -> Result<(), PlanDetailsUpdateError> {
        let Some(mut plan_details) = kv.get(id).json::<Value>().await? else {
            return Err(PlanDetailsUpdateError::NotFound);
        };

        let patch = serde_json::to_value(self.clone())?;
        deep_merge(&mut plan_details, patch);

        kv.put(id, serde_json::to_string(&plan_details)?)?
            .execute()
            .await?;

        Ok(())
    }
}

impl CreatePlanDetails {
    pub async fn put(self, kv: KvStore, id: &str) -> Result<(), PlanDetailsCreateError> {
        let plan_details = ReadPlanDetails {
            products: self.products,
            additional_info: self.additional_info,
        };

        kv.put(id, serde_json::to_string(&plan_details)?)?
            .execute()
            .await?;

        Ok(())
    }
}
