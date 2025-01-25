# ACCI Framework Architecture

## Table of Contents

1. [Core Concepts](#core-concepts)
2. [Project Requirements](#project-requirements)
3. [Technical Architecture](#technical-architecture)
   - [API Layer](#api-layer)
   - [Database Layer](#database-layer)
   - [Event System](#event-system)
   - [Cache Strategy](#cache-strategy)
   - [Security](#security)
4. [CQRS Architecture](#cqrs-architecture)
5. [Infrastructure](#infrastructure)
6. [Development Guidelines](#development-guidelines)
7. [Quality Assurance](#quality-assurance)
8. [Operations](#operations)
9. [Internationalization](#internationalization)

---

## Core Concepts

### Purpose

```rust
struct ACCIFramework {
    multi_tenancy: bool,    // True - Base requirement
    user_management: bool,  // True - Built-in
    enterprise_ready: bool, // True - Default setting
}
```

### Key Principles

- Multi-tenant first
- API-driven architecture
- Security by design
- Enterprise-grade scalability
- Comprehensive observability
- Internationalization (I18N) support

---

## Project Requirements

### Language Support Matrix

| Language | Code | Comments | Documentation | UI | API Docs |
|----------|------|----------|---------------|----|---------|
| English  | ✓    | ✓        | ✓             | ✓  | ✓       |
| German   | -    | -        | ✓             | ✓  | ✓       |
| Albanian | -    | -        | ✓             | ✓  | ✓       |
| French   | -    | -        | -             | ✓  | -       |
| Spanish  | -    | -        | -             | ✓  | -       |

### Documentation Structure

```
doc/
├── architecture/    # Technical documentation (English only)
├── api/            # API documentation (Multi-language)
│   ├── en/         # English API docs
│   ├── de/         # German API docs
│   └── sq/         # Albanian API docs
├── development/    # Development guides (English only)
└── user/           # User documentation (Multi-language)
    ├── en/         # English user docs
    ├── de/         # German user docs
    ├── sq/         # Albanian user docs
    ├── fr/         # French UI docs only
    └── es/         # Spanish UI docs only
```

---

## Technical Architecture

### API Layer

The API layer provides REST and GraphQL APIs, with built-in support for **internationalization (I18N)**.

#### REST API

- **I18N for API Responses**:
  - Use the `Accept-Language` header to determine the user's preferred language.
  - Return localized error messages and validation feedback.
  - Example:

    ```rust
    #[derive(Serialize)]
    struct ApiError {
        code: String,
        message: String, // Localized error message
    }

    async fn get_user(
        user_id: UserId,
        locale: &str,
    ) -> Result<UserResponse, ApiError> {
        let user = user_repository.get(user_id).await?;
        let message = translate("user_not_found", locale);
        Ok(UserResponse::from(user))
    }
    ```

#### GraphQL API

- **I18N for GraphQL Responses**:
  - Use a `locale` argument in GraphQL queries to return localized data.
  - Example:

    ```graphql
    query GetUser($userId: ID!, $locale: String!) {
        user(id: $userId) {
            id
            name(locale: $locale)
        }
    }
    ```

---

### Database Layer

The database layer supports **event sourcing**, **CQRS**, and **multi-tenancy**, with considerations for **I18N**.

#### Event Sourcing

- **Event Storage**:
  - Use **PostgreSQL** or **Kafka with custom event storage** for event persistence.
  - Store events in a structured format (e.g., JSON or Protobuf).
  - Implement **event versioning** and **snapshots** for schema evolution and performance optimization.
- **Localized Event Payloads**:
  - Store event payloads in a language-agnostic format (e.g., using translation keys).
  - Resolve localized content at the query or projection level.
  - Example:

    ```rust
    #[derive(Serialize)]
    struct UserRegisteredEvent {
        user_id: UserId,
        tenant_id: TenantId,
        message_key: String, // Translation key (e.g., "user_registered")
    }
    ```

#### CQRS

- **Read Models**:
  - Use **materialized views** for read models to optimize query performance.
  - Support **localized content** in read models by joining with translation tables or embedding translations in JSONB fields.
  - Example:

    ```sql
    CREATE TABLE translations (
        id SERIAL PRIMARY KEY,
        key TEXT NOT NULL,          -- Translation key (e.g., "welcome_message")
        locale TEXT NOT NULL,       -- Locale (e.g., "en", "de")
        value TEXT NOT NULL,        -- Localized value
        UNIQUE (key, locale)
    );
    ```

#### Multi-Tenancy

- **Database Isolation**:
  - Use **schema-per-tenant** or **row-level isolation** with tenant-specific configurations.
  - Implement **row-level security (RLS)** in PostgreSQL to enforce tenant isolation.
- **Tenant Onboarding/Offboarding**:
  - Automate tenant provisioning (e.g., creating schemas or rows) and data archiving/deletion.

---

### Event System

The event system supports **event-driven architecture** with **localized event payloads**.

#### Event Schema Design

- **Event Metadata**:
  - Include `event_id`, `event_type`, `event_version`, `timestamp`, `tenant_id`, and `correlation_id`.
- **Localized Payloads**:
  - Store event payloads using translation keys and resolve localized content at the query or projection level.

#### Message Bus

- **Event Bus**:
  - Use **Kafka** or **RabbitMQ** for reliable event streaming.
  - Implement **dead letter queues** for failed events.

---

### Cache Strategy

The cache strategy includes **in-memory** and **distributed caching** with support for **localized content**.

#### Redis Cache

- **Tenant-Aware Caching**:
  - Use tenant-specific cache keys to prevent cache pollution.
  - Implement **cache invalidation** strategies to ensure data consistency.
- **Localized Content**:
  - Cache localized content with **TTL (Time-to-Live)** to prevent stale reads.

---

### Security

The security layer includes **authentication**, **authorization**, and **audit logging** with support for **localized messages**.

#### Authentication Flow

- **Localized Error Messages**:
  - Return localized error messages for authentication failures.
  - Example:

    ```rust
    #[derive(Serialize)]
    struct AuthError {
        code: String,
        message: String, // Localized error message
    }
    ```

#### Audit Logging

- **Localized Audit Logs**:
  - Store audit log messages using translation keys.
  - Resolve localized messages at the UI or reporting level.
  - Example:

    ```rust
    #[derive(Serialize)]
    struct AuditLog {
        event_type: String, // Translation key (e.g., "user_login")
        timestamp: DateTime<Utc>,
        metadata: JsonValue,
    }
    ```

---

## CQRS Architecture

The CQRS architecture separates the **command side** (write model) from the **query side** (read model), with support for **localized content**.

### Command Side

- **Command Handlers**:
  - Process commands and produce events.
  - Ensure strong consistency and business rule enforcement.

### Query Side

- **Read Models**:
  - Provide read-optimized views of the data.
  - Support **localized content** by joining with translation tables or embedding translations in JSONB fields.

---

## Infrastructure

The infrastructure includes **container architecture**, **service mesh**, and **monitoring stack** with support for **PPC64LE**.

### Container Architecture

- **Multi-Architecture Builds**:
  - Use `buildx` to build and push Docker images for both `amd64` and `ppc64le`.

### Monitoring Stack

- **Metrics**:
  - Use **Prometheus** and **Grafana** to monitor database performance, API latency, and error rates.
- **Logging**:
  - Use **structured logging** with tenant-specific context (e.g., `tenant_id`, `user_id`).

---

## Development Guidelines

The development guidelines include **code organization**, **policy development**, and **error handling** with support for **localized content**.

### Code Organization

```
src/
├── api/          # API layer (REST & GraphQL)
├── domain/       # Business logic
├── infrastructure/ # External services
├── policies/     # Oso policy files
│   ├── global/   # Global policies
│   ├── rbac/     # Role-based policies
│   └── tenant/   # Tenant-specific policies
└── common/       # Shared utilities
```

### Localized UI and Email Templates

- **UI Translations**:
  - Use **Fluent (FTL)** for UI translations.
- **Email Templates**:
  - Store email templates in the database with support for multiple locales.
  - Example:

    ```sql
    CREATE TABLE email_templates (
        id SERIAL PRIMARY KEY,
        template_key TEXT NOT NULL,  -- Template key (e.g., "welcome_email")
        locale TEXT NOT NULL,        -- Locale (e.g., "en", "de")
        subject TEXT NOT NULL,       -- Localized subject
        body TEXT NOT NULL,          -- Localized body
        UNIQUE (template_key, locale)
    );
    ```

---

## Quality Assurance

The quality assurance includes **testing strategy**, **performance requirements**, and **security testing** with support for **I18N**.

### I18N Testing

- **Translation Coverage**:
  - Use automated tests to verify that all translation keys have corresponding values in supported locales.
  - Example:

    ```rust
    #[test]
    fn test_translation_coverage() {
        let locales = vec!["en", "de", "sq", "fr", "es"];
        for locale in locales {
            let translations = load_translations(locale);
            assert!(!translations.is_empty(), "Missing translations for locale: {}", locale);
        }
    }
    ```

---

## Operations

The operations include **deployment process**, **monitoring & alerting**, and **health checks** with support for **I18N**.

### Deployment Process

- **Rolling Deployment**:
  - Use rolling deployment strategy with health checks and automatic rollback.

---

## Internationalization

The internationalization section includes **translation system**, **message categories**, and **CI/CD integration** with support for **localized content**.

### Translation System

- **Primary Language**:
  - Use English as the primary language.
- **Fully Supported Languages**:
  - Support German and Albanian for all documentation and APIs
- **UI-Only Languages**:
  - French and Spanish support limited to UI elements only
- **Translation Files**:
  - Store translations in `i18n/{lang}/` directories
- **Fallback Language**:
  - Fall back to English if a translation is missing

---

This fully integrated architecture documentation ensures that **I18N** is treated as a core concern throughout the system, with detailed guidance on implementation and best practices. Let me know if you'd like further refinements or additional details!

```
