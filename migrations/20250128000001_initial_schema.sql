-- Enable necessary extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Enable Row Level Security and set current tenant
DO $$ 
BEGIN 
    EXECUTE 'ALTER DATABASE ' || current_database() || ' SET app.current_tenant = ''''';
END $$;

-- Create base tables
CREATE TABLE IF NOT EXISTS tenants (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    domain VARCHAR(255) NOT NULL UNIQUE,
    active BOOLEAN DEFAULT true NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL
);

CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    email VARCHAR(255) NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    active BOOLEAN DEFAULT true NOT NULL,
    roles TEXT[] DEFAULT '{}' NOT NULL,
    last_login TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    mfa_enabled BOOLEAN DEFAULT false NOT NULL,
    mfa_secret VARCHAR(255),
    UNIQUE(tenant_id, email)
);

CREATE TABLE IF NOT EXISTS mfa_backup_codes (
    id UUID PRIMARY KEY NOT NULL,
    tenant_id UUID NOT NULL,
    user_id UUID NOT NULL,
    code TEXT NOT NULL,
    used BOOLEAN NOT NULL DEFAULT FALSE,
    used_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE(tenant_id, user_id, code)
);

CREATE TABLE IF NOT EXISTS audit_log (
    id UUID PRIMARY KEY NOT NULL,
    tenant_id UUID NOT NULL,
    user_id UUID,
    action TEXT NOT NULL,
    table_name TEXT NOT NULL,
    record_id TEXT NOT NULL,
    old_values JSONB,
    new_values JSONB,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE
);

-- SSO related tables
CREATE TABLE IF NOT EXISTS sso_providers (
    id UUID PRIMARY KEY NOT NULL,
    tenant_id UUID NOT NULL,
    name TEXT NOT NULL,
    provider_type TEXT NOT NULL,
    client_id TEXT NOT NULL,
    client_secret TEXT NOT NULL,
    metadata_url TEXT,
    issuer TEXT,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS sso_mappings (
    id UUID PRIMARY KEY NOT NULL,
    tenant_id UUID NOT NULL,
    provider_id UUID NOT NULL,
    external_id TEXT NOT NULL,
    user_id UUID NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
    FOREIGN KEY (provider_id) REFERENCES sso_providers(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Create sessions table
CREATE TABLE IF NOT EXISTS sessions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    token VARCHAR(255) NOT NULL UNIQUE,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL
);

-- Enable Row Level Security for all tables
ALTER TABLE tenants ENABLE ROW LEVEL SECURITY;
ALTER TABLE users ENABLE ROW LEVEL SECURITY;
ALTER TABLE audit_log ENABLE ROW LEVEL SECURITY;
ALTER TABLE mfa_backup_codes ENABLE ROW LEVEL SECURITY;
ALTER TABLE sso_providers ENABLE ROW LEVEL SECURITY;
ALTER TABLE sso_mappings ENABLE ROW LEVEL SECURITY;

-- Create RLS policies for tenant isolation
CREATE POLICY tenant_isolation_policy ON tenants
    USING (id::text = COALESCE(current_setting('app.current_tenant', true), ''));

CREATE POLICY tenant_isolation_policy ON users
    USING (tenant_id::text = current_setting('app.current_tenant', true));

CREATE POLICY tenant_isolation_policy ON audit_log
    USING (tenant_id::text = COALESCE(current_setting('app.current_tenant', true), ''));

CREATE POLICY tenant_isolation_policy ON mfa_backup_codes
    USING (tenant_id::text = COALESCE(current_setting('app.current_tenant', true), ''));

CREATE POLICY tenant_isolation_policy ON sso_providers
    USING (tenant_id::text = COALESCE(current_setting('app.current_tenant', true), ''));

CREATE POLICY tenant_isolation_policy ON sso_mappings
    USING (tenant_id::text = COALESCE(current_setting('app.current_tenant', true), ''));

-- Create functions
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Create triggers
CREATE TRIGGER update_tenants_updated_at
    BEFORE UPDATE ON tenants
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER update_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_tenant_id ON users(tenant_id);
CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_sessions_token ON sessions(token); 