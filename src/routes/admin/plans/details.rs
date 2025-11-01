use crate::models::details::{
    CreatePlanDetails, PlanDetailsCreateError, PlanDetailsReadError, ReadPlanDetails,
};
use crate::service::discord::Discord;
use crate::KV_PLAN_DETAILS;
use worker::{Error, Request, Response, RouteContext};

pub async fn put_details(mut req: Request, ctx: RouteContext<()>) -> Result<Response, Error> {
    let plan_id = ctx.param("plan_id").map_or("", |v| v).to_string();

    match req.json::<CreatePlanDetails>().await {
        Ok(plan_details_create) => {
            let kv = ctx.env.kv(KV_PLAN_DETAILS)?;
            // keep a clone for Discord notification after successful upsert
            let details_for_notify = plan_details_create.clone();
            match plan_details_create.create(kv, &plan_id).await {
                Ok(_) => {
                    // fire-and-forget Discord notification (do not fail the API on error)
                    let discord = Discord::new_from_env(&ctx.env);
                    if let Err(err) = discord
                        .send_update_plan_details(plan_id.clone(), &details_for_notify)
                        .await
                    {
                        worker::console_log!("Failed to send Discord details update: {}", err);
                    }
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
            }
        }
        Err(e) => Ok(Response::from_json(&serde_json::json!({
            "code": 400,
            "message": e.to_string()
        }))?
        .with_status(400)),
    }
}

pub async fn get_details_admin(_req: Request, ctx: RouteContext<()>) -> Result<Response, Error> {
    let plan_id = ctx.param("plan_id").map_or("", |v| v);
    let kv = ctx.env.kv(KV_PLAN_DETAILS)?;
    match ReadPlanDetails::read(kv, plan_id).await {
        Ok(plan_details) => Ok(Response::from_json(&plan_details)?.with_status(200)),
        Err(PlanDetailsReadError::NotFound) => Ok(Response::from_json(&serde_json::json!({
            "code": 404,
            "message": "企画詳細が見つかりません"
        }))?
        .with_status(404)),
        Err(_) => Ok(Response::from_json(&serde_json::json!({
            "code": 500,
            "message": "内部エラーが発生しました"
        }))?
        .with_status(500)),
    }
}
