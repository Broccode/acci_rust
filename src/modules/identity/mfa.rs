use rand::Rng;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use totp_rs::{Algorithm, TOTP};
use uuid::Uuid;

use crate::shared::{
    error::{Error, Result},
    types::{TenantId, UserId},
};

/// MFA configuration for TOTP
#[derive(Debug, Clone)]
pub struct MfaConfig {
    pub digits: usize,
    pub step: u64,
    pub window: i64,
    pub issuer: String,
}

impl Default for MfaConfig {
    fn default() -> Self {
        Self {
            digits: 6,
            step: 30,
            window: 1,
            issuer: "ACCI Framework".to_string(),
        }
    }
}

/// MFA backup code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfaBackupCode {
    pub id: Uuid,
    pub user_id: UserId,
    pub tenant_id: TenantId,
    pub code: String,
    pub used: bool,
    pub created_at: OffsetDateTime,
    pub used_at: Option<OffsetDateTime>,
}

/// MFA service for handling TOTP and backup codes
#[derive(Debug)]
pub struct MfaService {
    config: MfaConfig,
}

impl MfaService {
    /// Creates a new MfaService instance
    pub fn new(config: MfaConfig) -> Self {
        Self { config }
    }

    /// Generates a new TOTP secret
    pub fn generate_secret(&self) -> Result<String> {
        let mut rng = rand::thread_rng();
        let secret: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        Ok(base32::encode(
            base32::Alphabet::RFC4648 { padding: true },
            &secret,
        ))
    }

    /// Generates a QR code for the TOTP secret
    pub fn generate_qr_code(&self, email: &str, secret: &str) -> Result<String> {
        let provisioning_uri = format!(
            "otpauth://totp/{}:{}?secret={}&issuer={}&digits={}&period={}",
            self.config.issuer,
            email,
            secret,
            self.config.issuer,
            self.config.digits,
            self.config.step
        );

        let code = qrcode::QrCode::new(provisioning_uri.as_bytes())
            .map_err(|e| Error::Internal(format!("Failed to generate QR code: {}", e)))?;

        Ok(code.render::<char>()
            .quiet_zone(false)
            .module_dimensions(2, 1)
            .build())
    }

    /// Verifies a TOTP code
    pub fn verify_code(&self, secret: &str, code: &str) -> Result<bool> {
        let totp = self.create_totp(secret)?;
        match totp.check_current(code) {
            Ok(result) => Ok(result),
            Err(_) => Ok(false),
        }
    }

    /// Generates backup codes
    pub fn generate_backup_codes(&self) -> Vec<String> {
        let mut rng = rand::thread_rng();
        (0..10)
            .map(|_| format!("{:08x}", rng.gen::<u32>()))
            .collect()
    }

    /// Creates a TOTP instance from a secret
    pub fn create_totp(&self, secret: &str) -> Result<TOTP> {
        let decoded = base32::decode(base32::Alphabet::RFC4648 { padding: true }, secret)
            .ok_or_else(|| Error::Internal("Failed to decode secret".to_string()))?;

        TOTP::new(
            Algorithm::SHA1,
            self.config.digits,
            self.config.window as u8,
            self.config.step,
            decoded,
        )
        .map_err(|e| Error::Internal(format!("Failed to create TOTP: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mfa_flow() {
        let service = MfaService::new(MfaConfig::default());

        // Generate secret
        let secret = service.generate_secret().unwrap();
        assert!(!secret.is_empty());

        // Generate QR code
        let qr_code = service
            .generate_qr_code("test@example.com", &secret)
            .unwrap();
        assert!(!qr_code.is_empty());

        // Create TOTP instance
        let totp = service.create_totp(&secret).unwrap();

        // Generate and verify code
        let code = totp.generate_current().unwrap();
        assert!(service.verify_code(&secret, &code).unwrap());

        // Test invalid code
        assert!(!service.verify_code(&secret, "000000").unwrap());
    }

    #[test]
    fn test_backup_codes() {
        let service = MfaService::new(MfaConfig::default());
        let codes = service.generate_backup_codes();

        assert_eq!(codes.len(), 10);
        for code in codes {
            assert_eq!(code.len(), 8);
            assert!(code.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }
}