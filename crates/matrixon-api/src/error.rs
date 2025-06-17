#[derive(Debug, Error)]
pub enum MatrixonError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    
    #[error("Authentication error: {0}")]
    AuthenticationError(String),
    
    #[error("Authorization error: {0}")]
    AuthorizationError(String),
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    
    #[error("Internal server error: {0}")]
    InternalError(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Conflict: {0}")]
    Conflict(String),
    
    #[error("Bad request: {0}")]
    BadRequest(String),
    
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    
    #[error("Forbidden: {0}")]
    Forbidden(String),
    
    #[error("Method not allowed: {0}")]
    MethodNotAllowed(String),
    
    #[error("Not acceptable: {0}")]
    NotAcceptable(String),
    
    #[error("Request timeout: {0}")]
    RequestTimeout(String),
    
    #[error("Too many requests: {0}")]
    TooManyRequests(String),
    
    #[error("Internal server error: {0}")]
    ServerError(String),
    
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
    
    #[error("Gateway timeout: {0}")]
    GatewayTimeout(String),
    
    #[error("HTTP version not supported: {0}")]
    HttpVersionNotSupported(String),
    
    #[error("Variant also negotiates: {0}")]
    VariantAlsoNegotiates(String),
    
    #[error("Insufficient storage: {0}")]
    InsufficientStorage(String),
    
    #[error("Loop detected: {0}")]
    LoopDetected(String),
    
    #[error("Not extended: {0}")]
    NotExtended(String),
    
    #[error("Network authentication required: {0}")]
    NetworkAuthenticationRequired(String),
}

impl From<MatrixonError> for StatusCode {
    fn from(err: MatrixonError) -> Self {
        match err {
            MatrixonError::BadRequest(_) => StatusCode::BAD_REQUEST,
            MatrixonError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            MatrixonError::Forbidden(_) => StatusCode::FORBIDDEN,
            MatrixonError::NotFound(_) => StatusCode::NOT_FOUND,
            MatrixonError::MethodNotAllowed(_) => StatusCode::METHOD_NOT_ALLOWED,
            MatrixonError::NotAcceptable(_) => StatusCode::NOT_ACCEPTABLE,
            MatrixonError::RequestTimeout(_) => StatusCode::REQUEST_TIMEOUT,
            MatrixonError::Conflict(_) => StatusCode::CONFLICT,
            MatrixonError::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
            MatrixonError::InternalError(_) | MatrixonError::ServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            MatrixonError::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            MatrixonError::GatewayTimeout(_) => StatusCode::GATEWAY_TIMEOUT,
            MatrixonError::HttpVersionNotSupported(_) => StatusCode::HTTP_VERSION_NOT_SUPPORTED,
            MatrixonError::VariantAlsoNegotiates(_) => StatusCode::VARIANT_ALSO_NEGOTIATES,
            MatrixonError::InsufficientStorage(_) => StatusCode::INSUFFICIENT_STORAGE,
            MatrixonError::LoopDetected(_) => StatusCode::LOOP_DETECTED,
            MatrixonError::NotExtended(_) => StatusCode::NOT_EXTENDED,
            MatrixonError::NetworkAuthenticationRequired(_) => StatusCode::NETWORK_AUTHENTICATION_REQUIRED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for MatrixonError {
    fn into_response(self) -> Response {
        let status = StatusCode::from(self.clone());
        let body = Json(json!({
            "errcode": self.to_string(),
            "error": self.to_string()
        }));
        (status, body).into_response()
    }
} 
