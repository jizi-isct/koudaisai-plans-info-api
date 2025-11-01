use serde::{Deserialize, Serialize};
use thiserror::Error;
use worker::kv::{KvError, KvStore};

use super::products::{ProductsCreate, ProductsRead};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreatePlanDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product: Option<ProductsCreate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_info: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReadPlanDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product: Option<ProductsRead>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_info: Option<String>,
}

#[derive(Error, Debug)]
pub enum PlanDetailsCreateError {
    #[error(transparent)]
    KvError(#[from] KvError),
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
}

impl CreatePlanDetails {
    pub async fn create(self, kv: KvStore, id: &str) -> Result<(), PlanDetailsCreateError> {
        // Overwrite (upsert) semantics for PUT
        let plan_details = ReadPlanDetails {
            product: self.product.map(Into::into),
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
