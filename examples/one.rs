use teahttp::{TeaError, TeaRequestInvoker};

/**
### This example is not meant to be run
It only suppose to test the type system, It will not work because it only implemented for WASM (actually compilable).
*/
#[async_std::main]
async fn main() -> Result<(), TeaError> {
    let _req = teahttp::TeaRequest::get("/api").invoke().await?;
    let some_body = b"Bogus wa Lorem ipsum dolor si amet\r\n";
    let _req = teahttp::TeaRequest::post("/api/submit")
        .slice_body(some_body.as_slice())
        .invoke()
        .await?;
    // teahttp::
    Ok(())
}
