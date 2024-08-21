use core::fmt::Debug;

use wasm_bindgen::JsValue;
use wasm_bindgen_futures::{wasm_bindgen, JsFuture};
use web_sys::{
    js_sys, wasm_bindgen::JsCast, window, Request, RequestInit, Response, Window, WorkerGlobalScope,
};

#[derive(Debug)]
pub enum TeaError {
    JSErr(JsValue),
    HellNoSuchProvider,
    InvalidMethod,
}

impl From<JsValue> for TeaError {
    fn from(value: JsValue) -> Self {
        Self::JSErr(value)
    }
}

impl core::fmt::Display for TeaError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TeaError::JSErr(err) => {
                f.write_str(&err.as_string().unwrap_or("Unstringable Error".to_string()))
            }
            TeaError::HellNoSuchProvider => f.write_str("where did y run this lib lol"),
            TeaError::InvalidMethod => f.write_str("oh uh y probably send something on get"),
        }
    }
}

// anon is this correct?
#[cfg(feature = "std")]
impl std::error::Error for TeaError {}

pub(crate) trait FetchProvider {
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

pub enum FetchProviders {
    WorkerProvider(Box<WorkerProvider>),
    WindowProvider(Box<WindowProvider>),
}

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

macro_rules! impl_cst {
    ($mth:tt, $MTHE:tt ) => {
        pub fn $mth(url: &'a str) -> TeaConstructor {
            Self::from_str(Method::$MTHE, url)
        }
    };
}

/**
### Use this struct to construct your request
```
TeaRequest::get("/api/something")
    .clone()
    .invoke()
    .await? // web_sys::Response
```
*/
#[derive(Clone)]
pub struct TeaRequest<'a>(Method, &'a str);
impl<'a> TeaRequest<'a> {
    #[inline(always)]
    fn from_str(method: Method, url: &'a str) -> TeaConstructor {
        TeaConstructor::new(Self(method, url))
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

macro_rules! impl_body {
    ($fn:tt,$vt:ty,$Wrapper:tt) => {
        pub fn $fn(self, v: &'a $vt) -> TeaBodyConstructor {
            TeaBodyConstructor(self, Body::$Wrapper(&v))
        }
    };
}

#[derive(Clone)]
pub struct TeaConstructor<'a>(TeaRequest<'a>, Request);
impl<'a> TeaConstructor<'a> {
    fn new(base: TeaRequest<'a>) -> Self {
        let url = base.1;
        Self(
            base,
            web_sys::Request::new_with_str(&url).expect("cannot create Request"),
        )
    }

    pub fn header(&'a mut self, key: &str, value: &str) -> Result<&'a mut Self, TeaError> {
        self.1.headers().set(key, value)?;
        Ok(self)
    }

    // pub fn slice_body<'b>(self, slice: &'b [u8]) -> BodifiedConstruct {
    //     BodifiedConstruct(self, Body::BorrowedSlice(&slice))
    // }

    // pub fn str_body<'b>(self, str: &'b str) -> BodifiedConstruct {
    //     BodifiedConstruct(self, Body::BorrowedSlice(&slice))
    // }

    impl_body!(slice_body, [u8], BorrowedSlice);
    impl_body!(str_body, str, BorrowedString);

    pub fn string_body(self, str: String) -> TeaBodyConstructor<'a> {
        TeaBodyConstructor(self, Body::CopiedString(str))
    }
}

pub trait Constructable {
    fn init(&self) -> RequestInit;
}

impl Constructable for TeaConstructor<'_> {
    fn init(&self) -> RequestInit {
        let opts = web_sys::RequestInit::new();
        opts.set_method(&format!("{:?}", self.0 .0));
        opts
    }
}

pub trait RequestInvokable: Constructable {
    fn base_request(&self) -> Request;
    /**
     * To invoke a web request
     */
    #[allow(async_fn_in_trait)]
    async fn invoke(&self) -> Result<Response, TeaError> {
        let request = Request::new_with_request_and_init(&self.base_request(), &self.init())?;
        FetchProviders::pls()?.fetch(&request).await
    }
}

pub enum Body<'a> {
    BorrowedSlice(&'a [u8]),
    BorrowedString(&'a str),
    CopiedString(String),
}

impl<'a> From<&'a [u8]> for Body<'a> {
    fn from(value: &'a [u8]) -> Self {
        Self::BorrowedSlice(value)
    }
}

impl<'a> From<&'a str> for Body<'a> {
    fn from(value: &'a str) -> Self {
        Self::BorrowedString(value)
    }
}

impl From<String> for Body<'_> {
    fn from(value: String) -> Self {
        Self::CopiedString(value)
    }
}

pub struct TeaBodyConstructor<'a>(TeaConstructor<'a>, Body<'a>);
impl<'a> TeaBodyConstructor<'a> {
    fn as_value(&self) -> JsValue {
        match &self.1 {
            Body::BorrowedSlice(slc) => {
                let arr: js_sys::Uint8Array = (*slc).into();
                let val: JsValue = arr.into();
                val
            }
            Body::BorrowedString(str) => JsValue::from_str(*str),
            Body::CopiedString(str) => JsValue::from_str(&str),
        }
    }
}
impl<'a> Constructable for TeaBodyConstructor<'a> {
    fn init(&self) -> RequestInit {
        let init = self.0.init();
        init.set_body(&self.as_value());
        init
    }
}
impl<'a> RequestInvokable for TeaBodyConstructor<'a> {
    fn base_request(&self) -> Request {
        self.0 .1.clone().expect("cannot clone Request")
    }
}
