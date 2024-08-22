//! ### teahttp
//!
//! Very simple but work well for WASM environment. As many http client tend to screw up WebWorker global, this `ehttp` inspired crate is to help.
//! Although it seems to be a good alt, many things considerably manual.
//!
//! Few example to figure things out
//!
//! ```
//!     TeaRequest::get("/api/something")
//!        .invoke()
//!        .await? // web_sys::Response
//!
//!     let some_body = b"lorem ipsum dolor si amet";
//!     TeaRequest::post("/api/upload")
//!         .header("Content-Length", some_body)?
//!         .slice_body(some_body.as_slice() /* &[u8] */)
//!         .invoke()
//!         .await?
//!
//!     TeaRequest::post("/api/submit")
//!         .header("Content-Length", &12.to_string())?
//!         .str_body("Hello World!" /* &str */)
//!         .invoke()
//!         .await?
//! ```
//!
//! Have fun

use ::core::fmt;
use wasm_bindgen::JsValue;
pub use web_sys;

#[derive(Debug)]
/**
### Handling mostly JS side error
 */
pub enum TeaError {
    JSErr(JsValue),
    HellNoSuchProvider,
}

impl From<JsValue> for TeaError {
    fn from(value: JsValue) -> Self {
        Self::JSErr(value)
    }
}

impl fmt::Display for TeaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TeaError::JSErr(err) => {
                f.write_str(&err.as_string().unwrap_or("Unstringable Error".to_string()))
            }
            TeaError::HellNoSuchProvider => f.write_str("where did y run this lib lol"),
        }
    }
}

// anon is this correct?
#[cfg(feature = "std")]
impl std::error::Error for TeaError {}

mod providers {
    use wasm_bindgen_futures::{
        js_sys,
        wasm_bindgen::{self, JsCast},
        JsFuture,
    };
    use web_sys::{window, Request, Response, Window, WorkerGlobalScope};

    use crate::TeaError;

    pub(crate) trait FetchProvider: Sized {
        async fn fetch(&self, request: &Request) -> Result<Response, TeaError>;
    }

    macro_rules! impl_fetch {
        ($Class:tt) => {
            impl FetchProvider for $Class {
                async fn fetch(&self, request: &Request) -> Result<Response, TeaError> {
                    Ok(JsFuture::from(self.0.fetch_with_request(&request))
                        .await?
                        .dyn_into::<Response>()?)
                }
            }
        };
    }

    #[derive(Clone)]
    pub struct WorkerProvider(WorkerGlobalScope);
    impl Default for WorkerProvider {
        fn default() -> Self {
            Self(
                js_sys::global()
                    .dyn_into::<web_sys::WorkerGlobalScope>()
                    .unwrap(),
            )
        }
    }
    impl_fetch!(WorkerProvider);

    #[derive(Clone)]
    pub struct WindowProvider(Window);
    impl Default for WindowProvider {
        fn default() -> Self {
            Self(web_sys::window().unwrap())
        }
    }
    impl_fetch!(WindowProvider);

    use wasm_bindgen::prelude::wasm_bindgen;
    #[wasm_bindgen(module = "/src/bind.js")]
    extern "C" {
        #[wasm_bindgen(js_name = "isNode")]
        pub(crate) fn is_node() -> bool;

        #[wasm_bindgen(js_name = "isWeb")]
        pub(crate) fn is_web() -> bool;

        #[wasm_bindgen(js_name = "isWorker")]
        pub(crate) fn is_worker() -> bool;

        #[wasm_bindgen(js_name = "isShell")]
        pub(crate) fn is_shell() -> bool;
    }

    pub enum FetchProviders {
        WorkerProvider(Box<WorkerProvider>),
        WindowProvider(Box<WindowProvider>),
    }

    impl FetchProviders {
        pub(crate) fn pls() -> Result<FetchProviders, TeaError> {
            if is_web() || window().is_some() {
                return Ok(FetchProviders::WindowProvider(Box::new(
                    WindowProvider::default(),
                )));
            } else if is_worker() {
                return Ok(FetchProviders::WorkerProvider(Box::new(
                    WorkerProvider::default(),
                )));
            }
            Err(TeaError::HellNoSuchProvider)
        }

        pub(crate) async fn fetch(&self, request: &Request) -> Result<Response, TeaError> {
            match self {
                FetchProviders::WorkerProvider(p) => p.fetch(request).await,
                FetchProviders::WindowProvider(p) => p.fetch(request).await,
            }
        }
    }
}

mod misc {
    #[derive(Debug, Clone, Copy)]
    pub enum Method {
        GET,
        HEAD,
        POST,
        PUT,
        DELETE,
        CONNECT,
        OPTIONS,
        TRACE,
        PATCH,
    }
}

mod core {
    use wasm_bindgen::JsValue;
    // use wasm_bindgen_futures::wasm_bindgen;
    use web_sys::{js_sys, Request, RequestInit, Response};

    use crate::{misc, providers, TeaError};

    macro_rules! impl_cst {
        ($mth:tt, $MTHE:tt ) => {
            pub fn $mth(url: &'a str) -> TeaBuilder {
                Self::from_str(misc::Method::$MTHE, url)
            }
        };
    }

    /**
    ### **Use this struct to construct your request**
    ```
    TeaRequest::get("/api/something")
        .invoke()
        .await? // web_sys::Response
    ```
    */
    #[derive(Clone)]
    pub struct TeaRequest<'a>(misc::Method, &'a str);
    impl<'a> TeaRequest<'a> {
        #[inline(always)]
        fn from_str(method: misc::Method, url: &'a str) -> TeaBuilder {
            TeaBuilder::new(Self(method, url))
        }

        impl_cst! { get     , GET     }
        impl_cst! { head    , HEAD    }
        impl_cst! { post    , POST    }
        impl_cst! { put     , PUT     }
        impl_cst! { delete  , DELETE  }
        impl_cst! { connect , CONNECT }
        impl_cst! { options , OPTIONS }
        impl_cst! { trace   , TRACE   }
        impl_cst! { patch   , PATCH   }
    }

    macro_rules! impl_body_trait {
        ($fn:tt,$vt:ty) => {
            fn $fn(&self, v: &'a $vt) -> TeaWithBody<'a>;
        };
    }

    macro_rules! impl_body {
        ($fn:tt,$vt:ty,$Wrapper:tt) => {
            fn $fn(&self, v: &'a $vt) -> TeaWithBody<'a> {
                TeaWithBody(self.clone(), TeaBody::$Wrapper(&v))
            }
        };
    }

    #[derive(Clone)]
    /**
    Step to build options from [TeaRequest]
     */
    pub struct TeaBuilder<'a>(TeaRequest<'a>, Request);
    impl<'a> TeaBuilder<'a> {
        fn new(base: TeaRequest<'a>) -> Self {
            let url = base.1;
            Self(
                base,
                web_sys::Request::new_with_str(&url).expect("cannot create Request"),
            )
        }
    }

    pub(crate) trait TeaRequestOwner<'a, 'b> {
        fn get_request(&'b self) -> &'a Request;
    }

    #[allow(private_bounds)]
    /**
    ### Add header to request
    By default, header list is pretty empty
     */
    pub trait TeaHeaderOwner<'a>: TeaRequestOwner<'a, 'a> {
        /**
        ### Add header with a builder style
         */
        fn header(&'a mut self, key: &str, value: &str) -> Result<&'a mut Self, TeaError>;
    }

    impl<'a, 'b: 'a> TeaRequestOwner<'a, 'b> for TeaBuilder<'b> {
        fn get_request(&'b self) -> &'a Request {
            &self.1
        }
    }

    impl<'a> TeaHeaderOwner<'a> for TeaBuilder<'a> {
        fn header(&'a mut self, key: &str, value: &str) -> Result<&'a mut Self, TeaError> {
            self.get_request().headers().set(key, value)?;
            Ok(self)
        }
    }

    /**
    ### Finalize request by assign body
     */
    pub trait TeaRequestBuilder<'a>: TeaHeaderOwner<'a> {
        // pub fn slice_body<'b>(self, slice: &'b [u8]) -> BodifiedConstruct {
        //     BodifiedConstruct(self, Body::BorrowedSlice(&slice))
        // }

        // pub fn str_body<'b>(self, str: &'b str) -> BodifiedConstruct {
        //     BodifiedConstruct(self, Body::BorrowedSlice(&slice))
        // }
        impl_body_trait!(slice_body, [u8]);
        impl_body_trait!(str_body, str);
        fn string_body(&self, str: String) -> TeaWithBody<'a>;
    }

    impl<'a> TeaRequestBuilder<'a> for TeaBuilder<'a> {
        impl_body!(slice_body, [u8], BorrowedSlice);
        impl_body!(str_body, str, BorrowedString);
        fn string_body(&self, str: String) -> TeaWithBody<'a> {
            TeaWithBody(self.clone(), TeaBody::CopiedString(str))
        }
    }

    pub trait Constructable: Sized {
        fn init(&self) -> Result<RequestInit, TeaError>;
    }

    impl Constructable for TeaBuilder<'_> {
        fn init(&self) -> Result<RequestInit, TeaError> {
            let opts = web_sys::RequestInit::new();
            opts.set_method(&format!("{:?}", self.0 .0));
            Ok(opts)
        }
    }

    pub(crate) trait Based: Sized {
        fn base_request(&self) -> Request;
    }

    #[allow(private_bounds)]
    /**
    ### Trait to actually invoke web request
    actually, it only have a single function that serve purpose
     */
    pub trait TeaRequestInvoker: Constructable + Based + Sized + Clone {
        /**
        ### To invoke a web request

        Confusing borrow-ness error will happen if you dont proceed the request anyway
        */
        #[allow(async_fn_in_trait)]
        async fn invoke(&self) -> Result<Response, TeaError> {
            let request = Request::new_with_request_and_init(&self.base_request(), &self.init()?)?;
            providers::FetchProviders::pls()?.fetch(&request).await
        }
    }

    impl<'a> Based for TeaBuilder<'a> {
        fn base_request(&self) -> Request {
            self.1.clone().expect("cannot clone Request")
        }
    }
    impl TeaRequestInvoker for TeaBuilder<'_> {}

    #[derive(Clone)]
    pub(crate) enum TeaBody<'a> {
        BorrowedSlice(&'a [u8]),
        BorrowedString(&'a str),
        CopiedString(String),
    }

    impl<'a> From<&'a [u8]> for TeaBody<'a> {
        fn from(value: &'a [u8]) -> Self {
            Self::BorrowedSlice(value)
        }
    }

    impl<'a> From<&'a str> for TeaBody<'a> {
        fn from(value: &'a str) -> Self {
            Self::BorrowedString(value)
        }
    }

    impl From<String> for TeaBody<'_> {
        fn from(value: String) -> Self {
            Self::CopiedString(value)
        }
    }

    #[derive(Clone)]
    /**
    Request after [TeaBuilder] attach body. Should only [TeaRequestInvoker::invoke]
     */
    pub struct TeaWithBody<'a>(TeaBuilder<'a>, TeaBody<'a>);
    impl<'a> TeaWithBody<'a> {
        fn as_value(&self) -> JsValue {
            match &self.1 {
                TeaBody::BorrowedSlice(slc) => {
                    let arr: js_sys::Uint8Array = (*slc).into();
                    let val: JsValue = arr.into();
                    val
                }
                TeaBody::BorrowedString(str) => JsValue::from_str(*str),
                TeaBody::CopiedString(str) => JsValue::from_str(&str),
            }
        }
    }
    impl<'a> Constructable for TeaWithBody<'a> {
        fn init(&self) -> Result<RequestInit, TeaError> {
            let init = self.0.init()?;
            init.set_body(&self.as_value());
            Ok(init)
        }
    }
    impl<'a> Based for TeaWithBody<'a> {
        fn base_request(&self) -> Request {
            self.0 .1.clone().expect("cannot clone Request")
        }
    }
    impl TeaRequestInvoker for TeaWithBody<'_> {}
}

pub use core::{
    TeaBuilder, TeaHeaderOwner, TeaRequest, TeaRequestBuilder, TeaRequestInvoker, TeaWithBody,
};
