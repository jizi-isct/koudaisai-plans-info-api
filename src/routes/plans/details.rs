use crate::models::{PlanDetailsReadError, ReadPlanDetails};
use crate::KV_PLAN_DETAILS;
use worker::{Cors, Error, Request, Response, RouteContext};

pub async fn get_details(_req: Request, ctx: RouteContext<()>) -> Result<Response, Error> {
    let plan_id = ctx.param("plan_id").map_or("", |v| v);

    let kv = ctx.env.kv(KV_PLAN_DETAILS)?;

    match ReadPlanDetails::read(kv, plan_id).await {
        Ok(plan_details) => Response::from_json(&plan_details),
        Err(PlanDetailsReadError::NotFound) => Ok(Response::from_json(&serde_json::json!({
            "code": 404,
            "message": "Plan details not found."
        }))?
        .with_cors(&Cors::new().with_origins(vec!["*"]))?
        .with_status(404)),
        Err(_) => Ok(Response::from_json(&serde_json::json!({
            "code": 500,
            "message": "Internal error occurred."
        }))?
        .with_cors(&Cors::new().with_origins(vec!["*"]))?
        .with_status(500)),
    }
}
