use crate::models::{
    CreatePlanDetails, PlanDetailsCreateError, PlanDetailsUpdateError, UpdatePlanDetails,
};
use crate::KV_PLAN_DETAILS;
use worker::{Error, Request, Response, RouteContext};

pub async fn put_details(mut req: Request, ctx: RouteContext<()>) -> Result<Response, Error> {
    let plan_id = ctx.param("plan_id").map_or("", |v| v);

    match req.json::<CreatePlanDetails>().await {
        Ok(plan_details_create) => {
            let kv = ctx.env.kv(KV_PLAN_DETAILS)?;
            match plan_details_create.create(kv, plan_id).await {
                Ok(_) => {
                    // 詳細情報作成・更新成功時は204 No Contentを返す
                    Ok(Response::empty()?.with_status(204))
                }
                Err(PlanDetailsCreateError::KvError(_)) => {
                    Ok(Response::from_json(&serde_json::json!({
                        "code": 500,
                        "message": "内部エラーが発生しました"
                    }))?
                    .with_status(500))
                }
                Err(PlanDetailsCreateError::SerdeError(_)) => {
                    Ok(Response::from_json(&serde_json::json!({
                        "code": 500,
                        "message": "内部エラーが発生しました"
                    }))?
                    .with_status(500))
                }
                Err(PlanDetailsCreateError::Conflict) => {
                    Ok(Response::from_json(&serde_json::json!({
                        "code": 409,
                        "message": "指定されたIDの企画詳細が既に存在します"
                    }))?
                    .with_status(409))
                }
            }
        }
        Err(e) => Ok(Response::from_json(&serde_json::json!({
            "code": 400,
            "message": e.to_string()
        }))?
        .with_status(400)),
    }
}

pub async fn patch_details(mut req: Request, ctx: RouteContext<()>) -> Result<Response, Error> {
    let plan_id = ctx.param("plan_id").map_or("", |v| v);

    match req.json::<UpdatePlanDetails>().await {
        Ok(plan_details_update) => {
            let kv = ctx.env.kv(KV_PLAN_DETAILS)?;
            match plan_details_update.update(kv, plan_id).await {
                Ok(_) => {
                    // 詳細情報更新成功時は204 No Contentを返す
                    Ok(Response::empty()?.with_status(204))
                }
                Err(PlanDetailsUpdateError::NotFound) => {
                    Ok(Response::from_json(&serde_json::json!({
                        "code": 404,
                        "message": "企画詳細が見つかりません"
                    }))?
                    .with_status(404))
                }
                Err(PlanDetailsUpdateError::KvError(_)) => {
                    Ok(Response::from_json(&serde_json::json!({
                        "code": 500,
                        "message": "内部エラーが発生しました"
                    }))?
                    .with_status(500))
                }
                Err(PlanDetailsUpdateError::WorkerError(_)) => {
                    Ok(Response::from_json(&serde_json::json!({
                        "code": 500,
                        "message": "内部エラーが発生しました"
                    }))?
                    .with_status(500))
                }
                Err(PlanDetailsUpdateError::SerdeError(_)) => {
                    Ok(Response::from_json(&serde_json::json!({
                        "code": 500,
                        "message": "内部エラーが発生しました"
                    }))?
                    .with_status(500))
                }
            }
        }
        Err(e) => Ok(Response::from_json(&serde_json::json!({
            "code": 400,
            "message": e.to_string()
        }))?
        .with_status(400)),
    }
}
