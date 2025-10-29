use crate::models::details::{PlanDetailsReadError, ReadPlanDetails};
use crate::KV_PLAN_DETAILS;
use worker::{Cache, Cors, Error, Method, Request, Response, RouteContext};

pub async fn get_details(req: Request, ctx: RouteContext<()>) -> Result<Response, Error> {
    // cacheからの復元
    let cache_key = Request::new(req.url()?.as_str(), Method::Get)?;
    let cache = Cache::default();
    if let Some(response) = cache.get(&cache_key, false).await? {
        return Ok(response);
    }

    let plan_id = ctx.param("plan_id").map_or("", |v| v);

    let kv = ctx.env.kv(KV_PLAN_DETAILS)?;

    let mut response = match ReadPlanDetails::read(kv, plan_id).await {
        Ok(plan_details) => Response::from_json(&plan_details)?,
        Err(PlanDetailsReadError::NotFound) => Response::from_json(&serde_json::json!({
            "code": 404,
            "message": "Plan details not found."
        }))?
        .with_cors(&Cors::new().with_origins(vec!["*"]))?
        .with_status(404),
        Err(_) => Response::from_json(&serde_json::json!({
            "code": 500,
            "message": "Internal error occurred."
        }))?
        .with_cors(&Cors::new().with_origins(vec!["*"]))?
        .with_status(500),
    };

    response = response.with_cors(&Cors::new().with_origins(vec!["*"]))?;

    let headers = response.headers_mut();
    headers.set("Cache-Control", "public, max-age=600, s-maxage=600")?;

    cache.put(&cache_key, response.cloned()?).await?;

    Ok(response)
}
