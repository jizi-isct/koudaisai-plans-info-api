use crate::util::deep_merge;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use worker::kv::{KvError, KvStore};

use super::products::{ProductsCreate, ProductsRead, ProductsUpdate};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreatePlanDetails {
    pub products: ProductsCreate,
    pub additional_info: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReadPlanDetails {
    pub products: ProductsRead,
    pub additional_info: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdatePlanDetails {
    pub products: Option<ProductsUpdate>,
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
            products: self.products.into(),
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
