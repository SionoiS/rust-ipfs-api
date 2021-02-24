// Copyright 2021 rust-ipfs-api Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
//

use crate::error::Error;

use async_trait::async_trait;

use bytes::Bytes;

use futures::{FutureExt, Stream, StreamExt, TryFutureExt, TryStreamExt};

use http::{
    header::{HeaderName, HeaderValue},
    uri::Scheme,
    StatusCode, Uri,
};

use ipfs_api_prelude::{ApiRequest, Backend, TryFromUri};

use reqwest::multipart::Form;

use reqwest::{Client, Request, Response};

use serde::Serialize;

pub struct ReqwestBackend {
    base: Uri,
    client: Client,
}

impl Default for ReqwestBackend {
    fn default() -> Self {
        Self::from_ipfs_config()
            .unwrap_or_else(|| Self::from_host_and_port(Scheme::HTTP, "localhost", 5001).unwrap())
    }
}

impl TryFromUri for ReqwestBackend {
    fn build_with_base_uri(base: Uri) -> Self {
        let client = Client::default();

        ReqwestBackend { base, client }
    }
}

#[async_trait(?Send)]
impl Backend for ReqwestBackend {
    type HttpRequest = Request;

    type HttpResponse = Response;

    type Error = Error;

    fn build_base_request<Req>(
        &self,
        req: &Req,
        form: Option<Form>,
    ) -> Result<Self::HttpRequest, Error>
    where
        Req: ApiRequest,
    {
        let url = format!("{}{}", self.base, Req::PATH);

        //event!(Level::INFO, url = ?url);

        let mut builder = self.client.post(&url).query(&req);

        if let Some(form) = form {
            builder = builder.multipart(form);
        }

        match builder.build() {
            Ok(req) => Ok(req),
            Err(e) => Err(e.into()),
        }
    }

    fn get_header<'a>(res: &'a Self::HttpResponse, key: HeaderName) -> Option<&'a HeaderValue> {
        res.headers().get(key)
    }

    async fn request_raw<Req>(
        &self,
        req: Req,
        form: Option<Form>,
    ) -> Result<(StatusCode, Bytes), Self::Error>
    where
        Req: ApiRequest + Serialize,
    {
        let req = self.build_base_request(&req, form)?;

        let res = self.client.execute(req).await?;
        let status = res.status();
        let body = res.bytes().await?;

        Ok((status, body))
    }

    fn response_to_byte_stream(
        res: Self::HttpResponse,
    ) -> Box<dyn Stream<Item = Result<Bytes, Self::Error>> + Unpin> {
        Box::new(res.bytes_stream().err_into())
    }

    fn request_stream<Res, F, OutStream>(
        &self,
        req: Self::HttpRequest,
        process: F,
    ) -> Box<dyn Stream<Item = Result<Res, Self::Error>> + Unpin>
    where
        OutStream: Stream<Item = Result<Res, Self::Error>> + Unpin,
        F: 'static + Fn(Self::HttpResponse) -> OutStream,
    {
        let stream = self
            .client
            .execute(req)
            .err_into()
            .map_ok(move |res| {
                match res.status() {
                    StatusCode::OK => process(res).right_stream(),
                    // If the server responded with an error status code, the body
                    // still needs to be read so an error can be built. This block will
                    // read the entire body stream, then immediately return an error.
                    //
                    _ => res
                        .bytes()
                        .boxed()
                        .map(|maybe_body| match maybe_body {
                            Ok(body) => Err(Self::process_error_from_body(body)),
                            Err(e) => Err(e.into()),
                        })
                        .into_stream()
                        .left_stream(),
                }
            })
            .try_flatten_stream();

        Box::new(stream)
    }
}
