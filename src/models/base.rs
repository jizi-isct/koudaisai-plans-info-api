use serde::{Deserialize, Serialize};

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
