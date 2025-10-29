use serde::{Deserialize, Serialize};

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
