use acci_rust::{
    core::{
        config::{ServerConfig, DatabaseConfig, ServerSettings},
        database::Database,
        server::Server,
    },
    modules::{
        identity::{
            AuthenticationService, Credentials, IdentityModule, Permission,
            PermissionAction, Role, RbacService, Session, User,
        },
        tenant::{Tenant, TenantModule},
    },
    shared::{
        error::Result,
        types::{TenantId, UserId},
    },
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use sqlx::PgPool;
use time::OffsetDateTime;
use tower::ServiceExt;
use uuid::Uuid;

/// Test helper to create a test database
async fn setup_test_db() -> Result<Database> {
    let config = DatabaseConfig {
        host: "localhost".to_string(),
        port: 5432,
        username: "postgres".to_string(),
        password: "postgres".to_string(),
        database: "acci_rust_test".to_string(),
        max_connections: 5,
        ssl_mode: false,
    };

    // Create test database
    let db = Database::connect(&config).await?;
    
    // Run migrations
    sqlx::migrate!("./migrations")
        .run(db.pool())
        .await
        .expect("Failed to run migrations");

    Ok(db)
}

/// Test helper to create a test server
async fn setup_test_server() -> Result<(Server, TenantModule, AuthenticationService)> {
    let db = setup_test_db().await?;
    
    let config = ServerConfig {
        database: DatabaseConfig {
            host: "localhost".to_string(),
            port: 5432,
            username: "postgres".to_string(),
            password: "postgres".to_string(),
            database: "acci_rust_test".to_string(),
            max_connections: 5,
            ssl_mode: false,
        },
        server: ServerSettings {
            host: "127.0.0.1".to_string(),
            port: 3000,
            cors_allowed_origins: vec!["http://localhost:3000".to_string()],
        },
    };

    let server = Server::new(&config).await?;
    let tenant_module = TenantModule::new(db.clone());
    let (_, auth_service) = IdentityModule::create_identity_module(db).await?;

    Ok((server, tenant_module, auth_service))
}

/// Test helper to create a test tenant
async fn create_test_tenant(module: &TenantModule) -> Result<Tenant> {
    let tenant = Tenant::new("Test Tenant".to_string());
    module.create_tenant(tenant.clone()).await
}

/// Test helper to create a test user
async fn create_test_user(
    auth_service: &AuthenticationService,
    tenant_id: TenantId,
    roles: Vec<Role>,
) -> Result<User> {
    let user = User {
        id: UserId::new(),
        tenant_id,
        email: "test@example.com".to_string(),
        password_hash: AuthenticationService::hash_password("password123")?,
        roles,
        active: true,
        last_login: None,
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
    };

    Ok(user)
}

#[tokio::test]
async fn test_multi_tenant_isolation() -> Result<()> {
    let (_, tenant_module, auth_service) = setup_test_server().await?;

    // Create two tenants
    let tenant1 = create_test_tenant(&tenant_module).await?;
    let tenant2 = create_test_tenant(&tenant_module).await?;

    // Create users in different tenants
    let user1 = create_test_user(
        &auth_service,
        tenant1.id,
        vec![Role {
            id: Uuid::new_v4(),
            name: "User".to_string(),
            permissions: vec![],
        }],
    )
    .await?;

    let user2 = create_test_user(
        &auth_service,
        tenant2.id,
        vec![Role {
            id: Uuid::new_v4(),
            name: "User".to_string(),
            permissions: vec![],
        }],
    )
    .await?;

    // Authenticate users
    let creds1 = Credentials {
        email: user1.email.clone(),
        password: "password123".to_string(),
        tenant_id: tenant1.id,
    };

    let creds2 = Credentials {
        email: user2.email.clone(),
        password: "password123".to_string(),
        tenant_id: tenant2.id,
    };

    let (_, session1) = auth_service.authenticate(creds1).await?;
    let (_, session2) = auth_service.authenticate(creds2).await?;

    // Verify sessions are tenant-specific
    assert_eq!(session1.tenant_id, tenant1.id);
    assert_eq!(session2.tenant_id, tenant2.id);

    Ok(())
}

#[tokio::test]
async fn test_rbac_permissions() -> Result<()> {
    let (_, tenant_module, auth_service) = setup_test_server().await?;
    let tenant = create_test_tenant(&tenant_module).await?;

    // Create user with specific permissions
    let user = create_test_user(
        &auth_service,
        tenant.id,
        vec![Role {
            id: Uuid::new_v4(),
            name: "Editor".to_string(),
            permissions: vec![
                Permission {
                    id: Uuid::new_v4(),
                    name: "read_posts".to_string(),
                    resource: "posts".to_string(),
                    action: PermissionAction::Read,
                },
                Permission {
                    id: Uuid::new_v4(),
                    name: "write_posts".to_string(),
                    resource: "posts".to_string(),
                    action: PermissionAction::Write,
                },
            ],
        }],
    )
    .await?;

    // Check permissions
    let rbac = RbacService::new();
    
    assert!(rbac.has_permission(&user, &PermissionCheck::new("posts", PermissionAction::Read)));
    assert!(rbac.has_permission(&user, &PermissionCheck::new("posts", PermissionAction::Write)));
    assert!(!rbac.has_permission(&user, &PermissionCheck::new("posts", PermissionAction::Delete)));

    Ok(())
}

#[tokio::test]
async fn test_session_management() -> Result<()> {
    let (_, tenant_module, auth_service) = setup_test_server().await?;
    let tenant = create_test_tenant(&tenant_module).await?;

    // Create user
    let user = create_test_user(
        &auth_service,
        tenant.id,
        vec![Role {
            id: Uuid::new_v4(),
            name: "User".to_string(),
            permissions: vec![],
        }],
    )
    .await?;

    // Authenticate
    let creds = Credentials {
        email: user.email.clone(),
        password: "password123".to_string(),
        tenant_id: tenant.id,
    };

    let (_, session) = auth_service.authenticate(creds).await?;

    // Validate session
    let validated = auth_service.validate_session(&session.token).await?;
    assert_eq!(validated.user_id, user.id);
    assert_eq!(validated.tenant_id, tenant.id);

    // Refresh session
    let refreshed = auth_service.refresh_session(session.id).await?;
    assert!(refreshed.expires_at > session.expires_at);

    // Logout
    auth_service.logout(session.id).await?;

    // Session should be invalid after logout
    let result = auth_service.validate_session(&session.token).await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_api_endpoints() -> Result<()> {
    let (server, tenant_module, auth_service) = setup_test_server().await?;
    let tenant = create_test_tenant(&tenant_module).await?;

    // Create user with admin permissions
    let user = create_test_user(
        &auth_service,
        tenant.id,
        vec![Role {
            id: Uuid::new_v4(),
            name: "Admin".to_string(),
            permissions: vec![Permission {
                id: Uuid::new_v4(),
                name: "admin".to_string(),
                resource: "tenants".to_string(),
                action: PermissionAction::Admin,
            }],
        }],
    )
    .await?;

    // Authenticate
    let creds = Credentials {
        email: user.email.clone(),
        password: "password123".to_string(),
        tenant_id: tenant.id,
    };

    let (_, session) = auth_service.authenticate(creds).await?;

    // Test API endpoints
    let app = server.create_router();

    // Health check should be accessible without authentication
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Protected endpoints should require authentication
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/tenants")
                .header("Authorization", format!("Bearer {}", session.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    Ok(())
}