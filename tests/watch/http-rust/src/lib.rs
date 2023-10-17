use anyhow::Result;
use spin_sdk::{
    http_component,
    wasi_http::{Request, Response},
};

/// A simple Spin HTTP component.
#[http_component]
fn hello_world(req: Request) -> Result<Response> {
    println!("{:?}", req.headers());
    Ok(http::Response::builder()
        .status(200)
        .header("foo", "bar")
        .body(Some("Hello, Fermyon!\n".into()))?)
}
