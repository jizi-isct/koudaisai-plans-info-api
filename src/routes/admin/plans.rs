use crate::models::{
    put_keys, PlanCreate, PlanCreateError, PlanRead, PlanReadError, PlanUpdate, PlanUpdateError,
};
use crate::service::discord::Discord;
use crate::KV_PLANS;
use worker::{console_error, Error, Request, Response, RouteContext};

pub mod details;
pub mod icon;

pub async fn put_plan(mut req: Request, ctx: RouteContext<()>) -> Result<Response, Error> {
    let plan_id = ctx.param("plan_id").map_or("", |v| v);

    match req.json::<PlanCreate>().await {
        Ok(plan_create) => {
            let kv = ctx.env.kv(KV_PLANS)?;
            match plan_create.clone().create(kv, plan_id).await {
                Ok(_) => {
                    // Discord通知
                    let discord = Discord::new_from_env(&ctx.env);
                    match discord.send_create_plan(plan_id.into(), &plan_create).await {
                        Ok(_) => {}
                        Err(err) => {
                            console_error!("Discord webhook error: {}", err)
                        }
                    }

                    // Update keys cache
                    let kv_cache = ctx.env.kv(KV_PLANS)?;
                    if let Err(err) = put_keys(&kv_cache).await {
                        console_error!("Failed to update keys cache: {:?}", err);
                    }

                    // 企画作成成功時は204 No Contentを返す
                    Ok(Response::empty()?.with_status(204))
                }
                Err(PlanCreateError::Conflict) => Ok(Response::from_json(&serde_json::json!({
                    "code": 409,
                    "message": "指定されたIDの企画が既に存在します"
                }))?
                .with_status(409)),
                Err(PlanCreateError::KvError(_)) => Ok(Response::from_json(&serde_json::json!({
                    "code": 500,
                    "message": "内部エラーが発生しました"
                }))?
                .with_status(500)),
            }
        }
        Err(e) => Ok(Response::from_json(&serde_json::json!({
            "code": 400,
            "message": e.to_string()
        }))?
        .with_status(400)),
    }
}

pub async fn patch_plan(mut req: Request, ctx: RouteContext<()>) -> Result<Response, Error> {
    let plan_id = ctx.param("plan_id").map_or("", |v| v);

    let kv = ctx.env.kv(KV_PLANS)?;

    match req.json::<PlanUpdate>().await {
        Ok(plan_update) => {
            match plan_update.clone().update(kv, plan_id).await {
                Ok(_) => {
                    // discord通知
                    let discord = Discord::new_from_env(&ctx.env);
                    match discord.send_update_plan(plan_id.into(), &plan_update).await {
                        Ok(_) => {}
                        Err(err) => {
                            console_error!("Discord webhook error: {}", err)
                        }
                    }
                    // 企画更新成功時は204 No Contentを返す
                    Ok(Response::empty()?.with_status(204))
                }
                Err(PlanUpdateError::NotFound) => Ok(Response::from_json(&serde_json::json!({
                    "code": 404,
                    "message": "企画が見つかりません"
                }))?
                .with_status(404)),
                Err(_) => Ok(Response::from_json(&serde_json::json!({
                    "code": 500,
                    "message": "内部エラーが発生しました"
                }))?
                .with_status(500)),
            }
        }
        Err(_) => Ok(Response::from_json(&serde_json::json!({
            "code": 400,
            "message": "リクエストが無効です"
        }))?
        .with_status(400)),
    }
}

pub async fn delete_plan(_req: Request, ctx: RouteContext<()>) -> Result<Response, Error> {
    let plan_id = ctx.param("plan_id").map_or("", |v| v);

    let kv = ctx.env.kv(KV_PLANS)?;

    // 企画が存在するか確認
    match PlanRead::read(kv.clone(), plan_id).await {
        Ok(_) => {
            // 削除実行
            match kv.delete(plan_id).await {
                Ok(_) => {
                    // discord通知
                    let discord = Discord::new_from_env(&ctx.env);
                    match discord.send_delete_plan(plan_id.into()).await {
                        Ok(_) => {}
                        Err(err) => {
                            console_error!("Discord webhook error: {}", err)
                        }
                    }

                    // Update keys cache
                    let kv_cache = ctx.env.kv(KV_PLANS)?;
                    if let Err(err) = put_keys(&kv_cache).await {
                        console_error!("Failed to update keys cache: {:?}", err);
                    }

                    Ok(Response::empty()?.with_status(204))
                }
                Err(_) => Ok(Response::from_json(&serde_json::json!({
                    "code": 500,
                    "message": "内部エラーが発生しました"
                }))?
                .with_status(500)),
            }
        }
        Err(PlanReadError::NotFound) => Ok(Response::from_json(&serde_json::json!({
            "code": 404,
            "message": "企画が見つかりません"
        }))?
        .with_status(404)),
        Err(_) => Ok(Response::from_json(&serde_json::json!({
            "code": 500,
            "message": "内部エラーが発生しました"
        }))?
        .with_status(500)),
    }
}

pub async fn post_plans_bulk(mut req: Request, ctx: RouteContext<()>) -> Result<Response, Error> {
    match req
        .json::<std::collections::HashMap<String, PlanCreate>>()
        .await
    {
        Ok(plans_map) => {
            let kv = ctx.env.kv(KV_PLANS)?;
            let mut errors = Vec::new();

            // すべてのエントリーに対して作成を試行
            for (id, plan_create) in plans_map {
                match plan_create.create(kv.clone(), &id).await {
                    Ok(_) => {
                        // 企画作成成功
                    }
                    Err(PlanCreateError::Conflict) => {
                        errors.push(serde_json::json!({
                            "plan_id": id,
                            "code": 409,
                            "message": format!("指定されたID「{}」の企画が既に存在します", id)
                        }));
                    }
                    Err(PlanCreateError::KvError(_)) => {
                        errors.push(serde_json::json!({
                            "plan_id": id,
                            "code": 500,
                            "message": format!("ID「{}」の企画作成中に内部エラーが発生しました", id)
                        }));
                    }
                }
            }

            if errors.is_empty() {
                // discord
                let discord = Discord::new_from_env(&ctx.env);
                match discord.send_bulk_create_plan().await {
                    Ok(_) => {}
                    Err(err) => {
                        console_error!("Discord webhook error: {}", err)
                    }
                }

                // Update keys cache
                let kv_cache = ctx.env.kv(KV_PLANS)?;
                if let Err(err) = put_keys(&kv_cache).await {
                    console_error!("Failed to update keys cache: {:?}", err);
                }

                // 全て成功した場合は201 Createdで空のレスポンスを返す
                Ok(Response::empty()?.with_status(201))
            } else {
                // 失敗したエントリーがある場合は207 Multi-Statusでエラー一覧を返す
                Ok(Response::from_json(&serde_json::json!({
                    "errors": errors
                }))?
                .with_status(207))
            }
        }
        Err(e) => Ok(Response::from_json(&serde_json::json!({
            "code": 400,
            "message": e.to_string()
        }))?
        .with_status(400)),
    }
}

pub async fn patch_plans_bulk(mut req: Request, ctx: RouteContext<()>) -> Result<Response, Error> {
    match req
        .json::<std::collections::HashMap<String, PlanUpdate>>()
        .await
    {
        Ok(plans_map) => {
            let kv = ctx.env.kv(KV_PLANS)?;
            let mut errors = Vec::new();

            // すべてのエントリーに対して更新を試行
            for (id, plan_update) in plans_map.clone() {
                match plan_update.update(kv.clone(), &id).await {
                    Ok(_) => {
                        // 企画更新成功
                    }
                    Err(PlanUpdateError::NotFound) => {
                        errors.push(serde_json::json!({
                            "plan_id": id,
                            "code": 404,
                            "message": format!("指定されたID「{}」の企画が見つかりません", id)
                        }));
                    }
                    Err(_) => {
                        errors.push(serde_json::json!({
                            "plan_id": id,
                            "code": 500,
                            "message": format!("ID「{}」の企画更新中に内部エラーが発生しました", id)
                        }));
                    }
                }
            }

            if errors.is_empty() {
                // discord通知
                let discord = Discord::new_from_env(&ctx.env);
                match discord
                    .send_bulk_update_plan(
                        plans_map
                            .iter()
                            .map(|e| (e.0.clone(), e.1.clone()))
                            .collect(),
                    )
                    .await
                {
                    Ok(_) => {}
                    Err(err) => {
                        console_error!("Discord webhook error: {}", err)
                    }
                }
                Ok(Response::empty()?.with_status(204))
            } else {
                // 失敗したエントリーがある場合は207 Multi-Statusでエラー一覧を返す
                Ok(Response::from_json(&serde_json::json!({
                    "errors": errors
                }))?
                .with_status(207))
            }
        }
        Err(e) => Ok(Response::from_json(&serde_json::json!({
            "code": 400,
            "message": e.to_string()
        }))?
        .with_status(400)),
    }
}
