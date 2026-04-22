mod m20260325_create_api_keys;
mod m20260325_create_hosts;
mod m20260325_create_metering_logs;
mod m20260327_add_api_key_id_to_metering_logs;
mod m20260406_add_total_ms_to_metering_logs;
mod m20260411_000001_create_beta_invites;
mod m20260411_000002_create_users;
mod m20260412_000001_create_pools;
mod m20260412_000002_create_pool_members;
mod m20260412_000003_create_host_tokens;
mod m20260412_000004_create_host_pool_memberships;
mod m20260412_000005_add_user_pool_to_api_keys;
mod m20260412_000006_add_user_to_hosts;
mod m20260412_000007_add_pool_id_to_metering_logs;
mod m20260420_000001_add_name_to_hosts;
mod m20260420_000002_add_is_active;

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
            Box::new(m20260411_000001_create_beta_invites::M20260411CreateBetaInvites),
            Box::new(m20260411_000002_create_users::M20260411CreateUsers),
            Box::new(m20260412_000001_create_pools::M20260412CreatePools),
            Box::new(m20260412_000002_create_pool_members::M20260412CreatePoolMembers),
            Box::new(m20260412_000003_create_host_tokens::M20260412CreateHostTokens),
            Box::new(m20260412_000004_create_host_pool_memberships::M20260412CreateHostPoolMemberships),
            Box::new(m20260412_000005_add_user_pool_to_api_keys::M20260412AddUserPoolToApiKeys),
            Box::new(m20260412_000006_add_user_to_hosts::M20260412AddUserToHosts),
            Box::new(m20260412_000007_add_pool_id_to_metering_logs::M20260412AddPoolIdToMeteringLogs),
            Box::new(m20260420_000001_add_name_to_hosts::M20260420AddNameToHosts),
            Box::new(m20260420_000002_add_is_active::M20260420AddIsActive),
        ]
    }
}
