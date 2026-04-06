mod m20260325_create_api_keys;
mod m20260325_create_hosts;
mod m20260325_create_metering_logs;
mod m20260327_add_api_key_id_to_metering_logs;
mod m20260406_add_total_ms_to_metering_logs;

use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260325_create_api_keys::M20260325CreateApiKeys),
            Box::new(m20260325_create_hosts::M20260325CreateHosts),
            Box::new(m20260325_create_metering_logs::M20260325CreateMeteringLogs),
            Box::new(m20260327_add_api_key_id_to_metering_logs::M20260327AddApiKeyIdToMeteringLogs),
            Box::new(m20260406_add_total_ms_to_metering_logs::M20260406AddTotalMsToMeteringLogs),
        ]
    }
}
