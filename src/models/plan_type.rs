use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum PlanTypeRead {
    Booth {
        #[serde(default)]
        categories: Vec<BoothPlanCategory>,
    },
    General {
        #[serde(default)]
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
