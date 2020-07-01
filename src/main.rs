use actix_web::{App, HttpServer, web, HttpResponse, Responder, HttpRequest};

use chashmap::CHashMap;
use std::ops::Deref;
use bytes::buf::Buf;

struct AppState {
    storage: CHashMap<String, web::Bytes>,
}

async fn get(data: web::Data<AppState>, key: web::Path<String>) -> impl Responder {
    println!("{}", key);
    let result = String::from_utf8(data
        .storage
        .get(&key.into_inner())
        .unwrap()
        .deref()
        .to_vec())
        .unwrap();
    println!("{}", &result);
    HttpResponse::Ok().body(result)
}

async fn set(data: web::Data<AppState>, key: web::Path<String>, value: web::Bytes) -> impl Responder {
    println!("{}", String::from_utf8(value.to_vec()).unwrap_or("".to_string()));
    &data.storage.insert_new(key.into_inner(), value);
    HttpResponse::Ok()
}



#[actix_rt::main]
async fn main() -> std::result::Result<(), std::io::Error>{
    let server = HttpServer::new(|| {
        App::new()
            .data(AppState {
                storage: CHashMap::new(),
            })
            .route("/get/{key}", web::get().to(get))
            .route("/set/{key}", web::post().to(set))
    })
        .bind("127.0.0.1:8080")?
        .run();
    println!("Started server at http://localhost:8080");

    server.await
}
