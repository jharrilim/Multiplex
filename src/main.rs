use actix::Arbiter;
use actix_web::{
    App,
    HttpServer,
    web,
    HttpResponse,
    Responder,
    HttpRequest,
    client::Client,
};
use serde::{Deserialize, Serialize};

use chashmap::CHashMap;
use std::ops::Deref;
use bytes::buf::Buf;
use std::collections::HashSet;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Subscription {
    callback_url: String,
}

struct Subscriber {
    failed_attempts: i32,
    callback_url: String,
}

struct AppState {
    client: Client,
    storage: CHashMap<String, web::Bytes>,
    subscribers: CHashMap<String, HashSet<Subscriber>>,
}


async fn get(data: web::Data<AppState>, key: web::Path<String>) -> impl Responder {
    println!("{}", key);
    let result = String::from_utf8(data
        .storage
        .get(&key.into_inner())
        .unwrap()
        .deref()
        .to_vec()
    ).unwrap();
    println!("{}", &result);
    HttpResponse::Ok().body(result)
}

async fn set(
    req: HttpRequest,
    state: web::Data<AppState>,
    key: web::Path<String>,
    value: web::Bytes,
) -> impl Responder {
    let k = key.into_inner();
    if let Some(read_guard) = state.subscribers.get(&k) {
        let rg_itr = read_guard.iter().clone();
        for subscriber in rg_itr {
            state.client.get(&subscriber.callback_url).send();
        }
    }
    println!("{}", String::from_utf8(value.to_vec()).unwrap_or("".to_string()));
    &state.storage.insert_new(k, value);
    HttpResponse::Ok()
}

async fn sub(
    state: web::Data<AppState>,
    key: web::Path<String>,
    body: web::Json<Subscription>,
) -> impl Responder {
    &state.subscribers.insert_new(key.into_inner(), HashSet::new());
    HttpResponse::Ok()
}

#[actix_rt::main]
async fn main() -> std::result::Result<(), std::io::Error> {
    let server = HttpServer::new(|| {
        App::new()
            .data(AppState {
                client: Client::default(),
                storage: CHashMap::new(),
                subscribers: CHashMap::new(),
            })
            .route("/get/{key}", web::get().to(get))
            .route("/set/{key}", web::post().to(set))
            .route("/sub/{key}", web::post().to(sub))
    })
        .bind("127.0.0.1:8080")?
        .run();
    println!("Started server at http://localhost:8080");

    server.await
}
