use crate::metrics::HealthCheckReporting;
use actix_web::{get, web, HttpResponse, Responder};
use std::sync::Arc;

pub mod accounts;
pub mod transactions;
pub(crate) mod ws;

type HealthReporter = Arc<dyn HealthCheckReporting>;

#[get("/health")]
async fn health_check(data: web::Data<HealthReporter>) -> impl Responder {
    let report = data.report();

    let mut response = if report.healthy() {
        HttpResponse::Ok()
    } else {
        HttpResponse::InternalServerError()
    };

    response.json(report)
}
