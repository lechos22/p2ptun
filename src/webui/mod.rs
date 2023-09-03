use actix_web::{
    dev::{ServiceFactory, ServiceRequest},
    http::header::ContentType,
    get, App, HttpResponse,
};

#[get("/webui")]
async fn webui_index() -> HttpResponse {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(include_str!("./index.html"))
}

#[get("/webui/style.css")]
async fn webui_style() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/css")
        .body(include_str!("./style.css"))
}

pub trait WebUI {
    fn webui(self) -> Self;
}

impl<T> WebUI for App<T>
where
    T: ServiceFactory<ServiceRequest, Config = (), Error = actix_web::Error, InitError = ()>,
{
    fn webui(self) -> Self {
        self.service(webui_index).service(webui_style)
    }
}
