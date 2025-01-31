# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- Single Sign-On (SSO) support
  - SAML 2.0 integration with metadata management
  - OpenID Connect (OIDC) integration with discovery
  - Multi-provider support per tenant
  - SSO session management
  - User mapping and federation
  - Audit logging for SSO events
- Multi-Factor Authentication (MFA) support
  - TOTP implementation with QR code generation
  - Backup codes system
  - MFA configuration management
  - Database schema for MFA and backup codes
  - Integration with user authentication
  - Audit logging for MFA changes
- Multi-tenant infrastructure with PostgreSQL RLS
- Identity Management with authentication and authorization
- Session Management with JWT and Redis
- RBAC implementation with permission caching
- Comprehensive integration tests
- Error handling with proper HTTP status codes
- Database migrations system
- Redis session store implementation
- Password hashing with Argon2
- User repository with CRUD operations
- Tenant management system
- API error responses with correlation IDs

### Changed
- Moved PermissionCheck trait from shared to identity module
- Improved error handling in authentication service
- Enhanced database query performance with proper indexing
- Updated user model to support MFA and SSO
- Consolidated database migrations
- Improved audit logging with MFA and SSO events
- Enhanced tenant service with better error handling
- Improved test coverage for tenant and identity modules
- Refactored error handling system for better type safety
- Updated database types for better SQLx integration
- Improved error handling in tenant tests with proper UUID validation
- Enhanced tenant handler responses for better error cases

### Fixed
- Fixed Option unwrapping in authentication service
- Corrected transaction handling in database operations
- Fixed import paths for identity module components
- Fixed database schema for MFA and SSO support
- Fixed tenant isolation in database queries
- Corrected error handling in tenant service
- Fixed type conversions for database IDs
- Fixed tenant handler tests to use valid UUID format
- Fixed tenant response types in handlers

## [0.1.0] - 2025-01-28
### Added
- Initial project setup
- Basic project structure
- Core dependencies configuration
