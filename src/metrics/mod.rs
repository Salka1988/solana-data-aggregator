use serde::Serialize;
pub mod connection_health_tracker;
#[cfg_attr(feature = "test-helpers", mockall::automock)]
pub trait HealthCheck: Send + Sync {
    fn healthy(&self) -> bool;
}
#[cfg_attr(feature = "test-helpers", mockall::automock)]
pub trait HealthReporting: Send + Sync {
    fn report(&self) -> HealthReport;
}
pub trait HealthCheckReporting: HealthCheck + HealthReporting {}
#[cfg_attr(feature = "test-helpers", mockall::automock)]
impl<T: HealthCheck + HealthReporting> HealthCheckReporting for T {}

#[derive(Debug, Serialize)]
pub struct HealthReport {
    pub solana_rpc_adapter: bool,
}

impl HealthReport {
    pub fn healthy(&self) -> bool {
        self.solana_rpc_adapter
    }
}

pub type HealthChecker = Box<dyn HealthCheck>;
