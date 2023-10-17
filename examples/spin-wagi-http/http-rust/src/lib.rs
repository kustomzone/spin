use anyhow::Result;
use spin_sdk::wasi_http_component;

/// A simple Spin HTTP component.
#[wasi_http_component]
fn goodbye_world(req: http::Request<()>) -> Result<http::Response<&'static str>> {
    println!("{:?}", req.headers());
    Ok(http::Response::builder()
        .status(200)
        .header("foo", "bar")
        .body("Goodbye, Fermyon!\n")?)
}
