use axum::{async_trait, BoxError, extract::FromRequest, http::StatusCode};
use axum::body::{Bytes, HttpBody};
use axum::extract::{BodyStream, Multipart};
use http::{HeaderMap, Request};
use http::header::CONTENT_TYPE;

#[derive(Debug)]
pub enum CustomFileExtractor {
    MULTIPART(Multipart),
    BODYSTREAM(BodyStream),
}

#[async_trait]
impl<S, B> FromRequest<S, B> for CustomFileExtractor
    where
        B: HttpBody + Send + 'static,
        B::Data: Into<Bytes>,
        B::Error: Into<BoxError>,
        S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request(req: Request<B>, state: &S) -> Result<Self, Self::Rejection> {
        let boundary = parse_boundary(req.headers());
        return if boundary.is_some() {
            //use multipart extractor
            match Multipart::from_request(req, state).await {
                Ok(result) => {
                    Ok(CustomFileExtractor::MULTIPART(result))
                }
                Err(e) => {
                    Err((e.status(), e.body_text()))
                }
            }
        } else {
            //use body stream
            match BodyStream::from_request(req, state).await {
                Ok(result) => {
                    Ok(CustomFileExtractor::BODYSTREAM(result))
                }
                Err(e) => {
                    Err((StatusCode::INTERNAL_SERVER_ERROR, "unknown error when parsing body stream".to_string()))
                }
            }
        };
    }
}

fn parse_boundary(headers: &HeaderMap) -> Option<String> {
    let content_type = headers.get(CONTENT_TYPE)?.to_str().ok()?;
    multer::parse_boundary(content_type).ok()
}
