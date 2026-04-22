use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

/// Injects a `X-Request-Id` header into every response for tracing.
pub async fn request_id(mut req: Request, next: Next) -> Response {
    let id = Uuid::new_v4().to_string();
    req.extensions_mut().insert(id.clone());

    let mut response = next.run(req).await;
    response.headers_mut().insert(
        "x-request-id",
        id.parse().expect("UUID is always a valid header value"),
    );
    response
}
