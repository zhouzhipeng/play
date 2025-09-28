use axum::body::Body;
use axum::body::{Bytes, HttpBody};
use axum::extract::Multipart;
use axum::{extract::FromRequest, http::StatusCode, BoxError};
use http::header::CONTENT_TYPE;
use http::{HeaderMap, Request};
use std::future::Future;

#[derive(Debug)]
pub enum CustomFileExtractor {
    MULTIPART(Multipart),
    BODYSTREAM(Body),
}

impl<S> FromRequest<S> for CustomFileExtractor
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    fn from_request(
        req: Request<axum::body::Body>,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            let boundary = parse_boundary(req.headers());
            return if boundary.is_some() {
                //use multipart extractor
                match Multipart::from_request(req, state).await {
                    Ok(result) => Ok(CustomFileExtractor::MULTIPART(result)),
                    Err(e) => Err((e.status(), e.body_text())),
                }
            } else {
                //use body directly
                Ok(CustomFileExtractor::BODYSTREAM(req.into_body()))
            };
        }
    }
}

fn parse_boundary(headers: &HeaderMap) -> Option<String> {
    let content_type = headers.get(CONTENT_TYPE)?.to_str().ok()?;
    multer::parse_boundary(content_type).ok()
}
