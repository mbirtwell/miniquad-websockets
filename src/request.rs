use crate::error::Result;

use http::{Request as HttpRequest, Response as HttpResponse, Uri};

/// Client request type (consistent with Tungstenite).
pub type Request = HttpRequest<()>;

/// Client response type (consistent with Tungstenite)
pub type Response = HttpResponse<()>;

//More copy and paste from Tunstenite

/// Trait for converting various types into HTTP requests used for a client connection.
///
/// This trait is implemented by default for string slices, strings, `url::Url`, `http::Uri` and
/// `http::Request<()>`.
pub trait IntoClientRequest {
    /// Convert into a `Request` that can be used for a client connection.
    fn into_client_request(self) -> Result<Request>;
}

impl<'a> IntoClientRequest for &'a str {
    fn into_client_request(self) -> Result<Request> {
        let uri: Uri = self.parse()?;

        Ok(Request::get(uri).body(())?)
    }
}

impl<'a> IntoClientRequest for &'a String {
    fn into_client_request(self) -> Result<Request> {
        let uri: Uri = self.parse()?;

        Ok(Request::get(uri).body(())?)
    }
}

impl IntoClientRequest for String {
    fn into_client_request(self) -> Result<Request> {
        let uri: Uri = self.parse()?;

        Ok(Request::get(uri).body(())?)
    }
}

impl<'a> IntoClientRequest for &'a Uri {
    fn into_client_request(self) -> Result<Request> {
        Ok(Request::get(self.clone()).body(())?)
    }
}

impl IntoClientRequest for Uri {
    fn into_client_request(self) -> Result<Request> {
        Ok(Request::get(self).body(())?)
    }
}

impl IntoClientRequest for Request {
    fn into_client_request(self) -> Result<Request> {
        Ok(self)
    }
}
