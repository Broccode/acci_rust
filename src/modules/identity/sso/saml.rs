use samael::{
    metadata::{ContactPerson, ContactType, EntityDescriptor, KeyDescriptor, KeyTypes, Organization},
    service_provider::ServiceProvider,
    verify::VerifySettings,
};
use time::OffsetDateTime;
use uuid::Uuid;
use x509_parser::prelude::*;

use crate::shared::error::{Error, Result};

use super::models::SsoProvider;

/// SAML configuration
#[derive(Debug, Clone)]
pub struct SamlConfig {
    pub certificate: String,
    pub private_key: String,
    pub organization_name: String,
    pub organization_display_name: String,
    pub organization_url: String,
    pub technical_contact_name: String,
    pub technical_contact_email: String,
}

/// SAML service for handling SAML authentication
#[derive(Debug)]
pub struct SamlService {
    config: SamlConfig,
}

impl SamlService {
    /// Creates a new SamlService instance
    pub fn new(config: SamlConfig) -> Self {
        Self { config }
    }

    /// Generates service provider metadata
    pub fn generate_metadata(&self, provider: &SsoProvider) -> Result<String> {
        let cert = parse_x509_pem(self.config.certificate.as_bytes())
            .map_err(|e| Error::Internal(format!("Failed to parse certificate: {}", e)))?
            .1;

        let key_descriptor = KeyDescriptor {
            key_type: KeyTypes::Signing,
            certificate: self.config.certificate.clone(),
            signing: true,
            encryption: false,
        };

        let organization = Organization {
            name: self.config.organization_name.clone(),
            display_name: self.config.organization_display_name.clone(),
            url: self.config.organization_url.clone(),
        };

        let technical_contact = ContactPerson {
            contact_type: ContactType::Technical,
            company: None,
            given_name: Some(self.config.technical_contact_name.clone()),
            sur_name: None,
            email_address: Some(self.config.technical_contact_email.clone()),
            telephone_number: None,
        };

        let entity_descriptor = EntityDescriptor {
            entity_id: provider.entity_id.clone().unwrap_or_default(),
            valid_until: None,
            cache_duration: None,
            organization: Some(organization),
            contact_person: vec![technical_contact],
            key_descriptors: vec![key_descriptor],
            assertion_consumer_services: vec![provider
                .assertion_consumer_service_url
                .clone()
                .unwrap_or_default()],
            single_logout_services: provider
                .single_logout_url
                .clone()
                .map(|url| vec![url])
                .unwrap_or_default(),
            name_id_formats: vec!["urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress".to_string()],
            authn_requests_signed: true,
            want_assertions_signed: true,
        };

        entity_descriptor
            .to_xml()
            .map_err(|e| Error::Internal(format!("Failed to generate metadata: {}", e)))
    }

    /// Creates a new authentication request
    pub fn create_auth_request(&self, provider: &SsoProvider) -> Result<(String, String)> {
        let sp = ServiceProvider::new(
            provider.entity_id.clone().unwrap_or_default(),
            provider
                .assertion_consumer_service_url
                .clone()
                .unwrap_or_default(),
            self.config.private_key.clone(),
            self.config.certificate.clone(),
        )
        .map_err(|e| Error::Internal(format!("Failed to create service provider: {}", e)))?;

        let request_id = format!("_{}", Uuid::new_v4());
        let now = OffsetDateTime::now_utc();

        let (auth_request, relay_state) = sp
            .make_authentication_request(Some(request_id), now.into())
            .map_err(|e| Error::Internal(format!("Failed to create auth request: {}", e)))?;

        Ok((auth_request, relay_state))
    }

    /// Validates a SAML response
    pub fn validate_response(
        &self,
        provider: &SsoProvider,
        response: &str,
        relay_state: &str,
    ) -> Result<(String, Option<String>, Option<String>)> {
        let sp = ServiceProvider::new(
            provider.entity_id.clone().unwrap_or_default(),
            provider
                .assertion_consumer_service_url
                .clone()
                .unwrap_or_default(),
            self.config.private_key.clone(),
            self.config.certificate.clone(),
        )
        .map_err(|e| Error::Internal(format!("Failed to create service provider: {}", e)))?;

        let verify_settings = VerifySettings {
            verify_signature: true,
            verify_recipient: true,
            verify_not_before: true,
            verify_not_on_or_after: true,
        };

        let assertion = sp
            .parse_response(response, Some(relay_state), verify_settings)
            .map_err(|e| Error::Authentication(format!("Failed to validate SAML response: {}", e)))?;

        let name_id = assertion.subject.name_id.value;
        let session_index = assertion.authn_statement.and_then(|stmt| stmt.session_index);
        let email = assertion
            .attribute_statement
            .and_then(|stmt| {
                stmt.attributes
                    .iter()
                    .find(|attr| attr.name == "email" || attr.name == "emailAddress")
                    .and_then(|attr| attr.values.first())
                    .map(|v| v.to_string())
            })
            .unwrap_or_else(|| name_id.clone());

        Ok((name_id, session_index, Some(email)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_CERT: &str = r#"-----BEGIN CERTIFICATE-----
MIIDazCCAlOgAwIBAgIUBR+ZF/QGgYt2K0JDtWNUQ0hHwFUwDQYJKoZIhvcNAQEL
BQAwRTELMAkGA1UEBhMCQVUxEzARBgNVBAgMClNvbWUtU3RhdGUxITAfBgNVBAoM
GEludGVybmV0IFdpZGdpdHMgUHR5IEx0ZDAeFw0yMzA0MjAxMjAwMDBaFw0yNDA0
MTkxMjAwMDBaMEUxCzAJBgNVBAYTAkFVMRMwEQYDVQQIDApTb21lLVN0YXRlMSEw
HwYDVQQKDBhJbnRlcm5ldCBXaWRnaXRzIFB0eSBMdGQwggEiMA0GCSqGSIb3DQEB
AQUAA4IBDwAwggEKAoIBAQC9QFi0Z/3FX5Zn5QXqNXRqQ3VF3j4DjH4VJ1zHZZrb
JqZvHWVH0aEzGqX0qVGq8YvW4/8u8i7kH9TJHr7VZ5JDYNxS7xQVQ9zFHuE7rwQP
UgbHoC8UVy5yW2zO4oZCUhUvUXfJFwFjEkDvXQp5wE+DtZL3oLfA+6ZfXQy8zHF1
rLQeNj8YqU5ZjE0zHYt1GjGV5TYw5JkJ1bEyEHNpP8JsYI5wPGhGh5z5QhxGZCGq
4Yx5PK7LxzKE7PXPj7WXNP0PyJxv5Dq8sCWqXE8+hKghOJvGQwI4q0RNxxv/bQa8
6t3j9TYnXjP0Cs8gZbFXrKY1MXAHZYBxpEkF1SiVAgMBAAGjUzBRMB0GA1UdDgQW
BBQKfPdB7Bw1L3cyviQQn0JvOqjk9TAfBgNVHSMEGDAWgBQKfPdB7Bw1L3cyviQQ
n0JvOqjk9TAPBgNVHRMBAf8EBTADAQH/MA0GCSqGSIb3DQEBCwUAA4IBAQCvvj7l
K3KJ9q0pQ7z8J7zL5P4ojP7M7VLzVhHi8QlE3Bw+MkUZ2QxNg9ij0kHd/LklBWVj
pXZvKK9NcFz4LHXxU4YQh6JuO3iKwZ0C0gz6jLX6R1GYtvNJY+mA6FJAEwvHq9Ck
Oj9Zt5qhF6FZJ7hwuCjZ3ZY7UYF0jg3bQ5ohL8z5M0J2kjwZQmVyGe8WcQ8yG9m1
XcUpqwGqD4C7QtWcO3ZbDZFdyYUxQZ2YgX6QWJHvlh6yjYU5wKtO0W5A6GzKkR2C
X6yO5WkpQW8fB+U3H4lF1C9hKt6ypb6F9X7M8pYGZHrX9dYKGF8Rz0BU6Xt6WmN1
0wNn6JGp1JWQHmMx
-----END CERTIFICATE-----"#;

    const TEST_KEY: &str = r#"-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQC9QFi0Z/3FX5Zn
5QXqNXRqQ3VF3j4DjH4VJ1zHZZrbJqZvHWVH0aEzGqX0qVGq8YvW4/8u8i7kH9TJ
Hr7VZ5JDYNxS7xQVQ9zFHuE7rwQPUgbHoC8UVy5yW2zO4oZCUhUvUXfJFwFjEkDv
XQp5wE+DtZL3oLfA+6ZfXQy8zHF1rLQeNj8YqU5ZjE0zHYt1GjGV5TYw5JkJ1bEy
EHNpP8JsYI5wPGhGh5z5QhxGZCGq4Yx5PK7LxzKE7PXPj7WXNP0PyJxv5Dq8sCWq
XE8+hKghOJvGQwI4q0RNxxv/bQa86t3j9TYnXjP0Cs8gZbFXrKY1MXAHZYBxpEkF
1SiVAgMBAAECggEAJQZxVHPB4LIQX1FZOzKDvpfqUE5c5JV/7bHZ0OGxwkqXhKvL
/Z1J7j9vKsQD0V3U6FvQCW6Z3wPGdQ1yGz5qz7z8oFWYf8CJsk9UPnpHjwxHjS7L
tP1/ayLxeF7WYqFzVXl6y7tBOoPvgbQJHU5JvA7MDRxL7Q9ofxzKckGPGDvXfLwJ
QhXaDy/0n7O4A8ZUoJhZ1hJYbzQJGj8kqCy0qQWNjCrHBb6M6y7m8CTKpQJRF8bF
Y5tUPGxZ9Wj5c2Qk9P3QhUPqP8E7GWz1NbVl9y1JZqfIJJ0rHzOYyqhQ1y5jHZB3
Rl/9d5cF3cd8VHVedQKBgQDlvMwV06bdbWVehYHBPqaXnzLe7Yx7Sh4dLkHQYgL7
dGZVD1xVPvpJhQZhJe/gO+7QXBf6pHAzJj9p1isM6DYqxOXv4YGczD/jK2iQYBfK
7/yRNqZ9JQq6B5h8IJ7LQD+Xxf0a3AHtt5U4CzHLUXVQxuGn7CNQFgXb9HIwKwKB
gQDSjYClQwLUw/F7TXjhY9ePCy+iQl3YLXfO4Hs1uIFrKNS0WirMN6r9duKQZdg3
Qn2XzAJ3d8QYHqGqZv0TjkV5RZNcB3xR3iKACIb5rkqhHLJGkVHXnvhz1yOeBQFo
zF3hqkxe3sBvBXGWpByQUhx9zWEGR1vCk0Aw4e7QBwKBgHxUWgJH1TvNyxRSqRZF
aZqQKnHVqXYdZ5pHYUU+YJrB1ARKzRkV8y0oCmK9nR4W2XBqfFZtpqG/0Jszsyvx
JvLbmhqwZyGn5MaQVjGkpCYIqZ8JRXrHzGJ9nYyxVwg5uX5IfK4+Zx5kQ1Pxu8QI
hB+J0Rp5WBQbVGzQ6Yy5AoGBAJxOKWnqX8QF8Q7F0l0VBzDvPEJjyvZxQbJqxY8H
TqyMKfkEjBB5HpP7GXxpFWLHgU5Zq1ZqBrwPm1yKR5jCUZZ9a5Q3xyDC4JmK3lQw
QWq5Q5xF9cZm5Y9YZxZlN5FQXzYUQlqR0kILUX8yqC7XJxLwVYEbZbPB1+MXK/8h
AoGAeHCVGwyqIxY0UPm1oEQtF9ZEUe3/jxQmyjenwHW6w7RH5TjLFxJGvZ7JhNB5
VF3yN7VOWQzk+vHkE6jVRfDGNNhRGQj2hB4Z4A1yJRk5Xt8Qu6x1QuWHIwoOXjWK
JQf+0Gx5OVjNrNVJw1pL4/Xt4ZJGWIX3JJxmvlz8A5Y=
-----END PRIVATE KEY-----"#;

    #[test]
    fn test_saml_metadata_generation() {
        let config = SamlConfig {
            certificate: TEST_CERT.to_string(),
            private_key: TEST_KEY.to_string(),
            organization_name: "Test Org".to_string(),
            organization_display_name: "Test Organization".to_string(),
            organization_url: "https://test.org".to_string(),
            technical_contact_name: "Test Admin".to_string(),
            technical_contact_email: "admin@test.org".to_string(),
        };

        let service = SamlService::new(config);

        let provider = SsoProvider::new_saml(
            crate::shared::types::TenantId::new(),
            "Test Provider".to_string(),
            None,
            None,
            None,
            "https://test.org/sp".to_string(),
            "https://test.org/acs".to_string(),
            Some("https://test.org/slo".to_string()),
        );

        let metadata = service.generate_metadata(&provider).unwrap();
        assert!(metadata.contains("EntityDescriptor"));
        assert!(metadata.contains("https://test.org/sp"));
        assert!(metadata.contains("https://test.org/acs"));
    }

    #[test]
    fn test_saml_auth_request() {
        let config = SamlConfig {
            certificate: TEST_CERT.to_string(),
            private_key: TEST_KEY.to_string(),
            organization_name: "Test Org".to_string(),
            organization_display_name: "Test Organization".to_string(),
            organization_url: "https://test.org".to_string(),
            technical_contact_name: "Test Admin".to_string(),
            technical_contact_email: "admin@test.org".to_string(),
        };

        let service = SamlService::new(config);

        let provider = SsoProvider::new_saml(
            crate::shared::types::TenantId::new(),
            "Test Provider".to_string(),
            None,
            None,
            None,
            "https://test.org/sp".to_string(),
            "https://test.org/acs".to_string(),
            Some("https://test.org/slo".to_string()),
        );

        let (auth_request, relay_state) = service.create_auth_request(&provider).unwrap();
        assert!(!auth_request.is_empty());
        assert!(!relay_state.is_empty());
    }
}