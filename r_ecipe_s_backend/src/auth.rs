use async_trait::async_trait;
use axum::{
    extract::{FromRequest, RequestParts},
    headers::{authorization::Bearer, Authorization, Header, SetCookie},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json, TypedHeader,
};
use r_ecipe_s_model::serde_json;
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Incorrect credentials")]
    WrongCredentials,
    #[error("Missing credentials")]
    MissingCredentials,
    #[error("Incorrect Authentication Token")]
    InvalidToken,
}

pub struct BearerValidation {
    hard_coded_secret: String, // yolo
}

impl BearerValidation {
    pub fn new(secret: &str) -> Self {
        BearerValidation {
            hard_coded_secret: secret.into(),
        }
    }

    pub(crate) fn authorise(
        &self,
        BearerToken(bearer_token): BearerToken,
    ) -> Result<(), AuthError> {
        if bearer_token == (self.hard_coded_secret) {
            Ok(())
        } else {
            Err(AuthError::InvalidToken)
        }
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthError::WrongCredentials => (StatusCode::UNAUTHORIZED, "Wrong credentials"),
            AuthError::MissingCredentials => (StatusCode::BAD_REQUEST, "Missing credentials"),
            AuthError::InvalidToken => (StatusCode::BAD_REQUEST, "Invalid token"),
        };
        let body = Json(serde_json::json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct BearerToken(String);

#[async_trait]
impl<B> FromRequest<B> for BearerToken
where
    B: Send,
{
    type Rejection = AuthError;

    async fn from_request(req: &mut RequestParts<B>) -> std::result::Result<Self, Self::Rejection> {
        // Extract the token from the authorization header
        let TypedHeader(Authorization(bearer)) =
            TypedHeader::<Authorization<Bearer>>::from_request(req)
                .await
                .map_err(|_| AuthError::InvalidToken)?;
        // Decode the user data
        let token_data = bearer.token().to_string();

        Ok(BearerToken(token_data))
    }
}
