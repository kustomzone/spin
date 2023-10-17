use anyhow::Result;
use spin_sdk::{
    wasi_http::{IntoResponse, Request},
    wasi_http_component,
};

/// Send an HTTP request and return the response.
#[wasi_http_component]
async fn send_outbound(_req: Request) -> Result<impl IntoResponse> {
    let mut res: http::Response<String> = spin_sdk::wasi_http::send(
        http::Request::builder()
            .method("GET")
            .uri("/test/hello")
            .body(())?,
    )
    .await?;
    res.headers_mut()
        .insert("spin-component", "outbound-http-component".try_into()?);
    println!("{:?}", res);
    Ok(res)
}
