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

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
struct Subscription {
    callback_url: String,
}

#[derive(Debug)]
struct Subscriber {
    failed_attempts: RelaxedCounter,
    callback_url: String,
}

struct AppState {
    client: Client,
    storage: CHashMap<String, web::Bytes>,
    subscribers: CHashMap<String, CHashMap<String, Subscriber>>,
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
    if let Some(subs) = &state.subscribers.get(&key.clone()) {
        subs.retain(| url, sub | {
            match state.client.post(url).send_body(value.clone()) {
                SendClientRequest::Fut(_, _, _) => {
                    if sub.failed_attempts.get() > 0 {
                        sub.failed_attempts.reset();
                    }
                    true
                },
                SendClientRequest::Err(_) => {
                    sub.failed_attempts.inc();
                    if (*sub).failed_attempts.get() > 20 {
                        return false;
                    }
                    true
                },
            }
        });
    }
    println!("[SET] {}", String::from_utf8(value.to_vec()).unwrap_or("".to_string()));
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
                failed_attempts: RelaxedCounter::new(0),
            });
        },
        None => {
            let map = CHashMap::new();
            map.insert(body.callback_url.clone(), Subscriber {
                callback_url: body.callback_url.clone(),
                failed_attempts: RelaxedCounter::new(0),
            });
            &state.subscribers.insert(key.into_inner(), map);
        },
    }
    HttpResponse::Ok()
}

#[actix_rt::main]
pub async fn main() -> std::result::Result<(), std::io::Error> {
    let server = HttpServer::new(|| {
        App::new()
            .data(AppState {
                client: Client::default(),
                storage: CHashMap::new(),
                subscribers: CHashMap::new(),
            })
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
