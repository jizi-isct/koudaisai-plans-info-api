use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Location {
    #[serde(rename = "indoor")]
    IndoorLocation { building: String, room: String },
    #[serde(rename = "outdoor")]
    OutdoorLocation { name: String },
}

#[derive(Serialize, Deserialize)]
pub struct Schedule {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    Available,
    Limited,
    SoldOut,
}

#[derive(Serialize, Deserialize)]
pub struct Product {
    pub name: String,
    pub price: u32,
    pub description: String,
    pub availability: Availability,
}

#[derive(Serialize, Deserialize)]
pub struct PlanDetails {
    pub products: Vec<Product>,
    pub additional_info: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename = "plan_type")]
pub enum PlanTypeCreate {
    Booth {},
    General {},
    Stage {},
    Labo { is_lab_tour: bool },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename = "plan_type")]
pub enum PlanTypeRead {
    Booth {},
    General {},
    Stage {},
    Labo { is_lab_tour: bool },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename = "plan_type")]
pub enum PlanTypeUpdate {
    Booth {},
    General {},
    Stage {},
    Labo {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        is_lab_tour: Option<bool>,
    },
}

#[derive(Serialize, Deserialize)]
pub struct PlanCreate {
    #[serde(flatten)]
    pub r#type: PlanTypeCreate,
    pub organization_name: String,
    pub plan_name: String,
    pub description: String,
    pub is_child_friendly: bool,
    pub is_recommended: bool,
    pub schedule: Vec<Schedule>,
    pub location: Vec<Location>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<PlanDetails>,
}

#[derive(Serialize, Deserialize)]
pub struct PlanRead {
    pub id: String,
    #[serde(flatten)]
    pub r#type: PlanTypeRead,
    pub organization_name: String,
    pub plan_name: String,
    pub description: String,
    pub is_child_friendly: bool,
    pub is_recommended: bool,
    pub schedule: Vec<Schedule>,
    pub location: Vec<Location>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<PlanDetails>,
}

#[derive(Serialize, Deserialize)]
pub struct PlanUpdate {
    #[serde(flatten)]
    pub r#type: PlanTypeUpdate,
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
    pub schedule: Option<Vec<Schedule>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<Vec<Location>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<PlanDetails>,
}
