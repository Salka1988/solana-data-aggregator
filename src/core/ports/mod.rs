pub mod blockchain;
pub mod runner;
pub mod storage;

#[cfg(test)]
mod tests {
    use crate::core::services::health_reporter_service::HealthReporterService;
    use crate::metrics::{HealthCheck, HealthChecker, HealthReport, HealthReporting};

    #[derive(Clone)]
    struct DummyHealthChecker {
        healthy_value: bool,
    }

    impl HealthCheck for DummyHealthChecker {
        fn healthy(&self) -> bool {
            self.healthy_value
        }
    }

    impl HealthReporting for DummyHealthChecker {
        fn report(&self) -> HealthReport {
            HealthReport {
                solana_rpc_adapter: self.healthy_value,
            }
        }
    }

    #[test]
    fn test_health_reporter_service_healthy_true() {
        let dummy: HealthChecker = Box::new(DummyHealthChecker {
            healthy_value: true,
        });
        let service = HealthReporterService::new(dummy);
        assert!(service.healthy(), "Expected service to be healthy");
    }

    #[test]
    fn test_health_reporter_service_healthy_false() {
        let dummy: HealthChecker = Box::new(DummyHealthChecker {
            healthy_value: false,
        });
        let service = HealthReporterService::new(dummy);
        assert!(!service.healthy(), "Expected service to be unhealthy");
    }

    #[test]
    fn test_health_reporter_service_report() {
        let dummy: HealthChecker = Box::new(DummyHealthChecker {
            healthy_value: true,
        });
        let service = HealthReporterService::new(dummy);
        let report = service.report();
        assert!(
            report.solana_rpc_adapter,
            "Expected report to indicate healthy"
        );

        let dummy: HealthChecker = Box::new(DummyHealthChecker {
            healthy_value: false,
        });
        let service = HealthReporterService::new(dummy);
        let report = service.report();
        assert!(
            !report.solana_rpc_adapter,
            "Expected report to indicate unhealthy"
        );
    }
}
