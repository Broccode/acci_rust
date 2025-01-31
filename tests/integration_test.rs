use acci_rust::{
    core::{
        config::{Config, DatabaseConfig, RedisConfig, ServerConfig},
        Core,
    },
    modules::identity::{
        models::{Credentials, Permission, PermissionAction, Role, RoleType, User},
        AuthenticationService, IdentityModule,
    },
    shared::{
        error::Result,
        types::{TenantId, UserId},
    },
};
use time::OffsetDateTime;
use uuid::Uuid;

#[tokio::test]
async fn test_core_initialization() -> Result<()> {
    let config = Config {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
            cors_allowed_origins: vec!["http://localhost:3000".to_string()],
        },
        database: DatabaseConfig {
            host: "localhost".to_string(),
            port: 5432,
            username: "postgres".to_string(),
            password: "postgres".to_string(),
            database: "acci_rust_test".to_string(),
            max_connections: 5,
            ssl_mode: false,
        },
        redis: RedisConfig {
            url: "redis://localhost:6379".to_string(),
        },
    };

    let _core = Core::new(config).await?;
    Ok(())
}

#[tokio::test]
async fn test_user_authentication() -> Result<()> {
    let config = Config {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
            cors_allowed_origins: vec!["http://localhost:3000".to_string()],
        },
        database: DatabaseConfig {
            host: "localhost".to_string(),
            port: 5432,
            username: "postgres".to_string(),
            password: "postgres".to_string(),
            database: "acci_rust_test".to_string(),
            max_connections: 5,
            ssl_mode: false,
        },
        redis: RedisConfig {
            url: "redis://localhost:6379".to_string(),
        },
    };

    let _core = Core::new(config).await?;
    let (identity_module, auth_service) = create_test_identity_module().await?;

    // Create test user
    let user = create_test_user(&identity_module).await?;

    // Test authentication
    let credentials = Credentials {
        email: "test@example.com".to_string(),
        password: "password123".to_string(),
        tenant_id: user.tenant_id,
        mfa_code: None,
    };

    let session = auth_service.authenticate(credentials).await?;
    assert_eq!(session.user_id, user.id);

    Ok(())
}

async fn create_test_identity_module() -> Result<(IdentityModule, AuthenticationService)> {
    let config = Config {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
            cors_allowed_origins: vec!["http://localhost:3000".to_string()],
        },
        database: DatabaseConfig {
            host: "localhost".to_string(),
            port: 5432,
            username: "postgres".to_string(),
            password: "postgres".to_string(),
            database: "acci_rust_test".to_string(),
            max_connections: 5,
            ssl_mode: false,
        },
        redis: RedisConfig {
            url: "redis://localhost:6379".to_string(),
        },
    };

    let core = Core::new(config).await?;
    acci_rust::modules::identity::create_identity_module(core.database).await
}

async fn create_test_user(identity_module: &IdentityModule) -> Result<User> {
    let user = User {
        id: UserId::new(),
        tenant_id: TenantId::new(),
        email: "test@example.com".to_string(),
        password_hash: "$argon2id$v=19$m=4096,t=3,p=1$salt$hash".to_string(),
        roles: vec![{
            let mut role = Role::new(RoleType::Admin, "Admin".to_string());
            role.permissions = vec![Permission {
                id: Uuid::new_v4(),
                name: "Create User".to_string(),
                action: PermissionAction::Create,
                resource: "users".to_string(),
            }];
            role
        }],
        active: true,
        last_login: None,
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
        mfa_enabled: false,
        mfa_secret: None,
    };

    identity_module.create_user(&user).await
}

#[tokio::test]
async fn test_user_permissions() -> Result<()> {
    let (identity_module, _) = create_test_identity_module().await?;
    let user = create_test_user(&identity_module).await?;

    // Test permission check
    let has_permission = identity_module
        .check_permission(&user, PermissionAction::Create, "users")
        .await?;
    assert!(has_permission);

    // Test permission does not exist
    let has_permission = identity_module
        .check_permission(&user, PermissionAction::Delete, "users")
        .await?;
    assert!(!has_permission);

    Ok(())
}
