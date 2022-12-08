use cargo_registry::util::AppResponse;
use serde_json::Value;
use std::marker::PhantomData;

use conduit::HandlerResult;

use http::{header, StatusCode};

/// A type providing helper methods for working with responses
#[must_use]
pub struct Response<T> {
    response: AppResponse,
    return_type: PhantomData<T>,
}

impl<T> Response<T>
where
    for<'de> T: serde::Deserialize<'de>,
{
    /// Assert that the response is good and deserialize the message
    #[track_caller]
    pub fn good(self) -> T {
        if !self.status().is_success() {
            panic!("bad response: {:?}", self.status());
        }
        json(self.response)
    }
}

impl<T> Response<T> {
    #[track_caller]
    pub(super) fn new(response: HandlerResult) -> Self {
        Self {
            response: assert_ok!(response),
            return_type: PhantomData,
        }
    }

    /// Consume the response body and convert it to a JSON value
    #[track_caller]
    pub fn into_json(self) -> Value {
        json(self.response)
    }

    pub fn status(&self) -> StatusCode {
        self.response.status()
    }

    #[track_caller]
    pub fn assert_redirect_ends_with(&self, target: &str) -> &Self {
        assert!(self
            .response
            .headers()
            .get(header::LOCATION)
            .unwrap()
            .to_str()
            .unwrap()
            .ends_with(target));
        self
    }
}

impl Response<()> {
    /// Assert that the status code is 404
    #[track_caller]
    pub fn assert_not_found(&self) {
        assert_eq!(StatusCode::NOT_FOUND, self.status());
    }

    /// Assert that the status code is 403
    #[track_caller]
    pub fn assert_forbidden(&self) {
        assert_eq!(StatusCode::FORBIDDEN, self.status());
    }
}

fn json<T>(r: AppResponse) -> T
where
    for<'de> T: serde::Deserialize<'de>,
{
    use conduit::Body::*;

    let content_type = r
        .headers()
        .get(header::CONTENT_TYPE)
        .expect("Missing content-type header");

    assert_eq!(content_type, "application/json; charset=utf-8");

    let content_length: usize = r
        .headers()
        .get(header::CONTENT_LENGTH)
        .expect("Missing content-length header")
        .to_str()
        .unwrap()
        .parse()
        .unwrap();

    let body: std::borrow::Cow<'static, [u8]> = match r.into_body() {
        Static(slice) => slice.into(),
        Owned(vec) => vec.into(),
        File(_) => unimplemented!(),
    };

    assert_eq!(content_length, body.len());

    match serde_json::from_slice(&body) {
        Ok(t) => t,
        Err(e) => panic!("failed to decode: {:?}", e),
    }
}
