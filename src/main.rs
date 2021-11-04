#[macro_use]
extern crate rocket;

#[macro_use]
extern crate rocket_sync_db_pools;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate diesel;

mod db;
mod encrypt;
mod error;
mod qr;
mod routes;

pub use error::Error;
use rocket::fs::NamedFile;
use rocket::http::Status;
use rocket::response::Responder;
use rocket::{response, Request, Response};
use std::env;
use std::path::{Path, PathBuf};

#[database("diesel")]
pub struct Db(diesel::SqliteConnection);

struct CachedFile(NamedFile);

impl<'r> Responder<'r, 'static> for CachedFile {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        Response::build_from(self.0.respond_to(req)?)
            .raw_header("Cache-control", "max-age=86400") //  24h (24*60*60)
            .ok()
    }
}

#[get("/<file..>")]
async fn files(file: PathBuf) -> Option<CachedFile> {
    let buf = Path::new("static/").join(file);
    if buf.is_dir() {
        NamedFile::open(buf.join("index.html"))
            .await
            .ok()
            .map(|nf| CachedFile(nf))
    } else {
        NamedFile::open(buf).await.ok().map(|nf| CachedFile(nf))
    }
}

#[catch(401)]
fn unauthorized() -> Authenticate {
    Authenticate
}
struct Authenticate;
impl<'r> Responder<'r, 'static> for Authenticate {
    fn respond_to(self, _request: &'r Request<'_>) -> rocket::response::Result<'static> {
        Response::build()
            .status(Status::Unauthorized)
            .raw_header(
                "WWW-Authenticate",
                "Basic realm=\"Access to restricted API\"",
            )
            .ok()
    }
}

#[launch]
fn rocket() -> _ {
    // fail soon
    let _ = env::var("SMTP_PASSWORD").expect("SMTP_PASSWORD not set");
    let _ = env::var("SMTP_PROVIDER").expect("SMTP_PROVIDER not set");
    let _ = env::var("AGE_SECRET_KEY").expect("AGE_SECRET_KEY not set");
    let _ = env::var("HTTP_AUTH_BASIC").expect("HTTP_AUTH_BASIC not set");

    rocket::build()
        .attach(routes::stage())
        .register("/", catchers![unauthorized])
        .mount("/", routes![files, crate::encrypt::encrypt])
}
