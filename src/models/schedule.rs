use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};

#[derive(Clone, Ord, PartialOrd, PartialEq, Eq, Debug, Copy)]
pub struct Time(u16);

impl Time {
    pub fn new(hour: u8, minute: u8) -> Option<Self> {
        if hour < 24 && minute < 60 {
            Some(Self(hour as u16 * 60 + minute as u16))
        } else {
            None
        }
    }
}

impl Serialize for Time {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{:02}:{:02}", self.0 / 60, self.0 % 60))
    }
}

impl<'a> Deserialize<'a> for Time {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        let s = String::deserialize(deserializer)?;
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err(serde::de::Error::custom("invalid time format"));
        }
        let hour = parts[0].parse::<u8>().unwrap();
        let minute = parts[1].parse::<u8>().unwrap();
        Self::new(hour, minute).ok_or(serde::de::Error::custom("invalid HH:mm format"))
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DaySchedule {
    pub start_time: Time,
    pub end_time: Time,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ScheduleCreate {
    pub day1: Vec<DaySchedule>,
    pub day2: Vec<DaySchedule>,
}

#[derive(Clone, Deserialize)]
#[serde(untagged)]
pub enum ScheduleRead {
    Combined {
        day1: Option<DaySchedule>,
        day2: Option<DaySchedule>,
    },
    NotCombined {
        day1: Vec<DaySchedule>,
        day2: Vec<DaySchedule>,
    },
}

impl ScheduleRead {
    pub fn combine(&self) -> ScheduleRead {
        match self {
            ScheduleRead::Combined { day1, day2 } => ScheduleRead::Combined {
                day1: day1.clone(),
                day2: day2.clone(),
            },
            ScheduleRead::NotCombined { day1, day2 } => ScheduleRead::Combined {
                day1: Some(ScheduleRead::combine_schedule(day1)),
                day2: Some(ScheduleRead::combine_schedule(day2)),
            },
        }
    }
    pub fn combine_mut(&mut self) {
        *self = self.combine()
    }
    pub fn uncombine(&self) -> ScheduleRead {
        match self {
            ScheduleRead::Combined { day1, day2 } => {
                let day1 = match day1 {
                    None => vec![],
                    Some(day1) => vec![day1.clone()],
                };
                let day2 = match day2 {
                    None => vec![],
                    Some(day2) => vec![day2.clone()],
                };
                ScheduleRead::NotCombined { day1, day2 }
            }
            ScheduleRead::NotCombined { day1, day2 } => ScheduleRead::NotCombined {
                day1: day1.clone(),
                day2: day2.clone(),
            },
        }
    }
    pub fn uncombine_mut(&mut self) {
        *self = self.uncombine()
    }
    fn combine_schedule(day: &Vec<DaySchedule>) -> DaySchedule {
        let mut start_time = day[0].start_time.clone();
        let mut end_time = day[0].end_time.clone();
        for schedule in day.iter().skip(1) {
            if schedule.start_time < start_time {
                start_time = schedule.start_time.clone();
            }
            if schedule.end_time > end_time {
                end_time = schedule.end_time.clone();
            }
        }
        DaySchedule {
            start_time,
            end_time,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ScheduleUpdate {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day1: Option<Option<Vec<DaySchedule>>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day2: Option<Option<Vec<DaySchedule>>>,
}

impl Into<ScheduleRead> for ScheduleCreate {
    fn into(self) -> ScheduleRead {
        ScheduleRead::NotCombined {
            day1: self.day1,
            day2: self.day2,
        }
    }
}

impl Serialize for ScheduleRead {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            ScheduleRead::Combined { day1, day2 } => {
                let mut state = serializer.serialize_struct("ScheduleRead", 2)?;
                match day1 {
                    Some(day1) => state.serialize_field("day1", &vec![day1])?,
                    None => state.serialize_field::<Vec<String>>("day1", &vec![])?,
                }
                match day2 {
                    Some(day2) => state.serialize_field("day2", &vec![day2])?,
                    None => state.serialize_field::<Vec<String>>("day2", &vec![])?,
                }
                state.end()
            }
            ScheduleRead::NotCombined { day1, day2 } => {
                let mut state = serializer.serialize_struct("ScheduleRead", 2)?;
                state.serialize_field("day1", &day1)?;
                state.serialize_field("day2", &day2)?;
                state.end()
            }
        }
    }
}
