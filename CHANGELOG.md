# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Basic multi-tenant database structure with PostgreSQL RLS
- Initial database schema for users, roles, and permissions
- Core framework structure with modular architecture
- Identity management module foundation
- Basic server setup with CORS and health check endpoint
- Tenant isolation through Row Level Security
- Authentication service structure with password hashing
- Tenant management API with CRUD operations
- Improved identity module with proper tenant awareness and error handling
- Session management with JWT tokens and Redis storage
- Basic RBAC implementation with permission checks and caching
- Database migrations system with initial schema
