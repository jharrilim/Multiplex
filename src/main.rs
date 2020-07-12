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
use actix_rt;
use awc::SendClientRequest;
use atomic_counter::{RelaxedCounter, AtomicCounter};
use ctrlc;
use std::sync::{Arc, Mutex};
use evmap::{
    ReadHandle,
    WriteHandle,
    new
};
use std::collections::hash_map::RandomState;
use log::kv::value::ToValue;
use core::borrow::{Borrow, BorrowMut};
use arc_swap::access::Access;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
struct Subscription {
    callback_url: String,
}

#[derive(Debug, Clone, ShallowCopy)]
struct Subscriber {
    failed_attempts: usize,
}

type EvMap<K, V> = (ReadHandle<K, V, (), RandomState>, WriteHandle<K, V, () , RandomState>);

#[derive(Clone)]
struct AppState {
    client: Client,
    storage_r: ReadHandle<String, String>,
    storage_w: Mutex<WriteHandle<String, String>>,
//    storage: EvMap<String, web::Bytes>,
    subscribers_r: ReadHandle<String, EvMap<String, Subscriber>>,
    subscribers_w: Mutex<WriteHandle<String, EvMap<String, Subscriber>>>,
//    subscribers: EvMap<String, EvMap<String, Subscriber>>,
}

async fn get(
    state: web::Data<AppState>,
    key: web::Path<String>,
) -> impl Responder {
    match state.storage.get(&key.into_inner()) {
        Some(rec) => {
            if let Ok(value) = String::from_utf8(rec.to_vec()) {
                return HttpResponse::Ok().body(value);
            }
            HttpResponse::UnprocessableEntity().finish()
        },
        None => {
            HttpResponse::NotFound().finish()
        },
    }
}

async fn set(
    _req: HttpRequest,
    state: web::Data<AppState>,
    key: web::Path<String>,
    value: web::Bytes,
) -> impl Responder {
    let subscribers_r = &state.subscribers_r;
    // Get subs for this key
    if let Some(key_by_subs) = subscribers_r.get(&key.clone()) {
        for (sub_r, mut sub_w) in key_by_subs.iter() {
            if let Some(a) = sub_r.read() {
                for (url, sub) in a.iter() {
                    match state.client.post(url.clone()).send_body(value.clone()) {
                        SendClientRequest::Fut(_, _, _) => {
                            if sub.get_one().unwrap().failed_attempts > 0 {
                                sub_w.update(url.clone(), Subscriber {
                                    failed_attempts: 0,
                                });
                            }
                        },
                        SendClientRequest::Err(_) => {
                            sub_w.update(url.clone(), Subscriber {
                                failed_attempts
                            });
                            if sub.get_one().unwrap().failed_attempts > 20 {
                                sub_w.borrow().clear(key.clone());
                            }
                        },
                    }

                }
            }
        }
    }
    let val_as_string = String::from_utf8(value.to_vec()).unwrap_or("".to_string());
    println!("[SET] {}", val_as_string.clone());
    &state.storage.insert(key.into_inner(), val_as_string);
    HttpResponse::Ok()
}

async fn sub(
    state: web::Data<AppState>,
    key: web::Path<String>,
    body: web::Json<Subscription>,
) -> impl Responder {
    match &state.subscribers_r.get(&key.clone()) {
        Some(subs) => {
            subs.insert(body.callback_url.clone(), Subscriber {
                failed_attempts: 0,
            });
        },
        None => {
            let (
                map_r,
                mut map_w
            ) = evmap::new();
            map_w.insert(body.callback_url.clone(), Subscriber {
                failed_attempts: 0,
            });
            &state.subscribers_w.insert(key.into_inner(), (map_r, map_w));
        },
    }
    HttpResponse::Ok()
}

#[actix_rt::main]
pub async fn main() -> std::result::Result<(), std::io::Error> {
    let (subscribers_r, subscribers_w): EvMap<String, EVMap<String, Subscriber>> = evmap::new();
    let (storage_r, storage_w): EvMap<String, String> = evmap::new();

    let server = HttpServer::new(move || {
        App::new()
            .data(web::Data::new(AppState {
                client: Client::default(),
                storage_r: storage_r.clone(),
                storage_w: Mutex::new(storage_w),
                subscribers_r: subscribers_r.clone(),
                subscribers_w: Mutex::new(subscribers_w),
            }))
            .route("/store/{key}", web::get().to(get))
            .route("/store/{key}", web::post().to(set))
            .route("/sub/{key}", web::post().to(sub))
    })
        .bind("127.0.0.1:8080")?
        .run();
    println!("Started server at http://localhost:8080");
    ctrlc::set_handler(move || {
        println!("Shutting down multiplex server.");
    }).expect("Error setting ctrl-c handler.");
    server.await
}
