use openidconnect::{
    core::{
        CoreAuthenticationFlow, CoreClient, CoreIdToken, CoreIdTokenClaims, CoreProviderMetadata,
        CoreResponseType, CoreTokenResponse,
    },
    reqwest::async_http_client,
    AccessToken, AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, OAuth2TokenResponse,
    RedirectUrl, Scope, TokenResponse,
};
use time::OffsetDateTime;
use url::Url;

use crate::shared::error::{Error, Result};

use super::models::SsoProvider;

/// OIDC configuration
#[derive(Debug, Clone)]
pub struct OidcConfig {
    pub redirect_url: String,
}

/// OIDC service for handling OpenID Connect authentication
#[derive(Debug)]
pub struct OidcService {
    config: OidcConfig,
}

impl OidcService {
    /// Creates a new OidcService instance
    pub fn new(config: OidcConfig) -> Self {
        Self { config }
    }

    /// Creates an OIDC client for a provider
    async fn create_client(&self, provider: &SsoProvider) -> Result<CoreClient> {
        let issuer_url = provider
            .issuer
            .as_ref()
            .ok_or_else(|| Error::Internal("Missing issuer URL".to_string()))?;

        let client_id = provider
            .client_id
            .as_ref()
            .ok_or_else(|| Error::Internal("Missing client ID".to_string()))?;

        let client_secret = provider
            .client_secret
            .as_ref()
            .ok_or_else(|| Error::Internal("Missing client secret".to_string()))?;

        let provider_metadata = if let Some(discovery_url) = &provider.discovery_url {
            CoreProviderMetadata::discover_async(
                IssuerUrl::new(discovery_url.clone())
                    .map_err(|e| Error::Internal(format!("Invalid discovery URL: {}", e)))?,
                async_http_client,
            )
            .await
            .map_err(|e| Error::Internal(format!("Failed to discover provider metadata: {}", e)))?
        } else {
            CoreProviderMetadata::discover_async(
                IssuerUrl::new(issuer_url.clone())
                    .map_err(|e| Error::Internal(format!("Invalid issuer URL: {}", e)))?,
                async_http_client,
            )
            .await
            .map_err(|e| Error::Internal(format!("Failed to discover provider metadata: {}", e)))?
        };

        CoreClient::from_provider_metadata(
            provider_metadata,
            ClientId::new(client_id.clone()),
            Some(ClientSecret::new(client_secret.clone())),
        )
        .set_redirect_uri(
            RedirectUrl::new(self.config.redirect_url.clone())
                .map_err(|e| Error::Internal(format!("Invalid redirect URL: {}", e)))?,
        )
        .map_err(|e| Error::Internal(format!("Failed to create OIDC client: {}", e)))
    }

    /// Creates an authorization URL
    pub async fn create_auth_url(&self, provider: &SsoProvider) -> Result<(Url, CsrfToken, Nonce)> {
        let client = self.create_client(provider).await?;

        let (auth_url, csrf_token, nonce) = client
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .add_scope(Scope::new("profile".to_string()))
            .url();

        Ok((auth_url, csrf_token, nonce))
    }

    /// Validates an authorization code and exchanges it for tokens
    pub async fn validate_auth_code(
        &self,
        provider: &SsoProvider,
        code: &str,
        nonce: Nonce,
    ) -> Result<(String, String)> {
        let client = self.create_client(provider).await?;

        let token_response = client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .request_async(async_http_client)
            .await
            .map_err(|e| Error::Authentication(format!("Failed to exchange auth code: {}", e)))?;

        let id_token = token_response
            .id_token()
            .ok_or_else(|| Error::Authentication("Missing ID token".to_string()))?;

        let claims = id_token
            .claims(&client.id_token_verifier(), &nonce)
            .map_err(|e| Error::Authentication(format!("Failed to verify ID token: {}", e)))?;

        let subject = claims.subject().to_string();
        let email = claims
            .email()
            .map(|e| e.to_string())
            .unwrap_or_else(|| subject.clone());

        Ok((subject, email))
    }

    /// Validates an ID token
    pub fn validate_id_token(
        &self,
        provider: &SsoProvider,
        id_token: &str,
    ) -> Result<CoreIdTokenClaims> {
        let client_id = provider
            .client_id
            .as_ref()
            .ok_or_else(|| Error::Internal("Missing client ID".to_string()))?;

        let issuer = provider
            .issuer
            .as_ref()
            .ok_or_else(|| Error::Internal("Missing issuer URL".to_string()))?;

        let token = CoreIdToken::from_str(id_token)
            .map_err(|e| Error::Authentication(format!("Invalid ID token: {}", e)))?;

        let claims = token
            .claims(&CoreIdTokenClaims::options_for_basic_validation(
                ClientId::new(client_id.clone()),
                IssuerUrl::new(issuer.clone())
                    .map_err(|e| Error::Internal(format!("Invalid issuer URL: {}", e)))?,
            ))
            .map_err(|e| Error::Authentication(format!("Failed to validate ID token: {}", e)))?;

        Ok(claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::types::TenantId;

    #[tokio::test]
    async fn test_oidc_auth_url() {
        let config = OidcConfig {
            redirect_url: "http://localhost:3000/auth/callback".to_string(),
        };

        let service = OidcService::new(config);

        let provider = SsoProvider::new_oidc(
            TenantId::new(),
            "Test Provider".to_string(),
            None,
            "client_id".to_string(),
            "client_secret".to_string(),
            "https://accounts.google.com".to_string(),
            Some("https://accounts.google.com/.well-known/openid-configuration".to_string()),
        );

        let result = service.create_auth_url(&provider).await;
        assert!(result.is_err()); // Will fail without a real provider
    }

    #[test]
    fn test_id_token_validation() {
        let config = OidcConfig {
            redirect_url: "http://localhost:3000/auth/callback".to_string(),
        };

        let service = OidcService::new(config);

        let provider = SsoProvider::new_oidc(
            TenantId::new(),
            "Test Provider".to_string(),
            None,
            "client_id".to_string(),
            "client_secret".to_string(),
            "https://accounts.google.com".to_string(),
            None,
        );

        let invalid_token = "invalid.token.here";
        let result = service.validate_id_token(&provider, invalid_token);
        assert!(result.is_err());
    }
}