use serde::Serialize;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RecoveryEmailAssociation {
    pub user_id: String,
    pub normalized_email: String,
    pub email_masked: String,
    pub verified_at: Option<String>,
    pub encrypted_private_key: Option<String>,
    pub encryption_algorithm: Option<String>,
    pub encryption_version: Option<i64>,
    pub key_nonce: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EmailVerificationToken {
    pub token_hash: String,
    pub user_id: String,
    pub target_email: String,
    pub email_masked: String,
    pub encrypted_private_key: String,
    pub encryption_algorithm: String,
    pub encryption_version: i64,
    pub key_nonce: String,
    pub requester_ip: String,
    pub used_by_ip: Option<String>,
    pub expires_at: String,
    pub used_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RecoveryEmailStatusResponse {
    pub associated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_masked: Option<String>,
    pub verified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_at: Option<String>,
}

impl RecoveryEmailStatusResponse {
    pub fn none() -> Self {
        Self {
            associated: false,
            email_masked: None,
            verified: false,
            verified_at: None,
        }
    }

    pub fn from_association(association: &RecoveryEmailAssociation) -> Self {
        Self {
            associated: true,
            email_masked: Some(association.email_masked.clone()),
            verified: association.verified_at.is_some(),
            verified_at: association.verified_at.clone(),
        }
    }
}
