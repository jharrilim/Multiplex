use actix_web::{App, HttpServer, web, HttpResponse, Responder, HttpRequest, client::Client, Error};
use serde::{Deserialize, Serialize};

use chashmap::CHashMap;
use std::ops::Deref;
use std::thread;
use core::borrow::Borrow;
use actix_rt;
use std::rc::Rc;
use futures_util::future::try_future::TryFutureExt;

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
    subscribers: CHashMap<String, CHashMap<String, Subscriber>>,
}


async fn get(
    data: web::Data<AppState>,
    key: web::Path<String>,
) -> impl Responder {
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
    _req: HttpRequest,
    state: web::Data<AppState>,
    key: web::Path<String>,
    value: web::Bytes,
) -> impl Responder {
    if let Some(subs) = &state.subscribers.get(&key.clone()) {
        let subss = *subs.into_iter();
        for (subscriber, _) in *subss {
            state.client.get(&subscriber).send();
        }
    }
    println!("{}", String::from_utf8(value.to_vec()).unwrap_or("".to_string()));
    &state.storage.insert(key.into_inner(), value);
    HttpResponse::Ok()
}

async fn sub(
    state: web::Data<AppState>,
    key: web::Path<String>,
    body: web::Json<Subscription>,
) -> impl Responder {

    match &state.subscribers.get(&key.clone()) {
        Some(subs) => {
            subs.insert(body.callback_url.clone(), Subscriber {
                callback_url: body.callback_url.clone(),
                failed_attempts: 032,
            });
            if let Some(sub) = subs.get(&body.callback_url) {

            }
        },
        None => {
            &state.subscribers.insert(key.into_inner(), CHashMap::new());
        },

    }
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
