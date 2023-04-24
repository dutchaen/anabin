mod assist;
mod block;

use may::go;
use std::thread;
use block::Block;
use mongodb::bson::doc;
use chrono::prelude::*;
use std::time::Duration;
use actix_files::NamedFile;
use std::sync::mpsc::{self, Sender};
use serde::{Serialize, Deserialize};
use mongodb::{Client, options::ClientOptions, Database, Collection};
use actix_web::{get, post, web, App, HttpResponse, HttpServer, middleware::Logger};

const PORT: u16 = 8080;
const DAY: i64 = 60 * 60 * 24;
const HOST: &str = "127.0.0.1";

#[derive(Default, Serialize, Deserialize)]
struct Paste {
    text: String
}

#[derive(Serialize, Deserialize, Clone)]
struct PasteEntry {
    text: String,
    id: String,
    unix_created: i64
}

#[derive(Serialize, Deserialize, Clone)]
struct PasteLookup {
    id: String
}

struct ServerStruct {
    database: Database,
    send_channel: Sender<PasteEntry>
}

#[get("/")]
async fn index() -> Result<NamedFile, Box<dyn std::error::Error>> {
    return Ok(NamedFile::open("index.html")?);
}


#[get("/hi")]
async fn hi() -> Result<NamedFile, Box<dyn std::error::Error>> {
    return Ok(NamedFile::open("index.html")?);
}

#[post("/paste")]
async fn send_paste(server_struct: web::Data<ServerStruct>, paste: web::Json<Paste>) -> HttpResponse {

    let paste_id = assist::generate_paste_id();
    let collection: Collection<PasteEntry> = server_struct.database.collection("Pastes");

    let paste = PasteEntry{ 
        text: paste.text.clone(),
        id: paste_id.clone(),
        unix_created: Utc::now().timestamp()
    };

    if let Err(e) = collection.insert_one(paste.clone(), None).await
    {
        dbg!(e);
        return HttpResponse::InternalServerError().body("whoops ;p");
    }

    if let Err(e) = server_struct.send_channel.send(paste) {
        dbg!(e);
        return HttpResponse::InternalServerError().body("whoops ;p");
    }


    let mut response_builder = HttpResponse::Ok();
    return response_builder.insert_header(("Location", format!("http://{}:{}/pastes/{}", HOST, PORT, &paste_id)))
        .finish();

}

#[get("/pastes/{id}")]
async fn get_paste(server_struct: web::Data<ServerStruct>, query: web::Path<String>) -> Result<HttpResponse, Box<dyn std::error::Error>> {

    let collection: Collection<PasteEntry> = server_struct.database.collection("Pastes");

    if let Some(paste_entry) = collection.find_one(doc! { "id" : &query.into_inner() }, None).await? {
        return Ok(HttpResponse::Ok().body(paste_entry.text));
    }

    return Ok(HttpResponse::NotFound().body(r#"{"anabin_error":"paste does not exist lol"}"#));
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        let client_options = ClientOptions::parse("mongodb://localhost:27017").wait().unwrap();
        let client = Client::with_options(client_options).unwrap();

        let (tx, rx) = mpsc::channel::<PasteEntry>();

        let ss = web::Data::new(ServerStruct {
            database: client.database("主要"),
            send_channel: tx
        });

        let db = ss.database.clone();
        
        thread::spawn(move || {
            while let Ok(paste) = rx.recv() {
                let db = db.clone();
                go!(move || {                    
                    let unix_delete = paste.unix_created + DAY;
                    while Utc::now().timestamp() < unix_delete {
                        thread::sleep(Duration::from_millis(5));
                    }

                    let collection: Collection<PasteEntry> = db.collection("Pastes");
                    if let Err(e) = collection.delete_one(doc! { "id" : paste.id }, None).wait() {
                        dbg!(e);
                    }
                });
            }
        });

        App::new()
            .wrap(Logger::default())
            .service(hi)
            .service(index)
            .service(send_paste)
            .service(get_paste)
            .app_data(ss)
    })
    .bind((HOST, PORT))?
    .run()
    .await
}
