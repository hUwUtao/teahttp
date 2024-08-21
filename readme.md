# teahttpd

an unsophisticated \(fetch wrapper\) http client for wasm

ok pls propose something because this ductape plane is running on a motive of "should works, would works and it works!"

## how to use

```rs
    let _res: web_sys::Request = teahttp::TeaRequest::get("/api/something")
        .header("Accept", "application/json")?
        .clone()
        .slice_body(&encoded)
        .invoke()
        .await?;

    let _res: web_sys::Request = teahttp::TeaRequest::post("/api/upload")
        .header("Content-Type", "application/octet-stream")?
        .header("Content-Length", &encoded.len().to_string())?
        .clone()
        .slice_body(&encoded)
        .invoke()
        .await?;
```

## todo if needed

- better error handling
- serde
- umm diy and wrap this?