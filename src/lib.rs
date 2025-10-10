mod icon;
mod jwks;
mod jwt_verifier;
mod models;
mod routes;
mod service;
mod util;

use crate::routes::plans::details::{get_details, patch_details, put_details};
use crate::routes::plans::icon::{get_icon, post_icon_import, put_icon};
use crate::routes::plans::{
    delete_plan, get_plan, get_plans, patch_plan, post_plans_bulk, put_plan,
};
use worker::*;

const VAR_JWKS_URL: &str = "JWKS_URL";
const KV_PLANS: &str = "PLANS";
const KV_PLAN_DETAILS: &str = "PLAN_DETAILS";
const R2_PLAN_IMAGES: &str = "plan_icons";

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();

    // Router設定
    let router = Router::new();

    router
        .get_async("/v1/plans", get_plans)
        .get_async("/v1/plans/:plan_id", get_plan)
        .put_async("/v1/plans/:plan_id", put_plan)
        .patch_async("/v1/plans/:plan_id", patch_plan)
        .delete_async("/v1/plans/:plan_id", delete_plan)
        .post_async("/v1/plans:bulk", post_plans_bulk)
        .put_async("/v1/plans/:plan_id/icon", put_icon)
        .get_async("/v1/plans/:plan_id/icon", get_icon)
        .post_async("/v1/plans/:plan_id/icon:import", post_icon_import)
        .get_async("/v1/plans/:plan_id/details", get_details)
        .put_async("/v1/plans/:plan_id/details", put_details)
        .patch_async("/v1/plans/:plan_id/details", patch_details)
        .run(req, env)
        .await
}
