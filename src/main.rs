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

use chrono::{DateTime, Utc};
pub use error::Error;
use rocket::fs::NamedFile;
use rocket::http::hyper::header::{CACHE_CONTROL, IF_MODIFIED_SINCE, LAST_MODIFIED};
use rocket::http::Status;
use rocket::response::Responder;
use rocket::{response, Request, Response};
use std::env;
use std::path::{Path, PathBuf};

#[database("diesel")]
pub struct Db(diesel::SqliteConnection);

struct CachedFile(NamedFile, String);

impl<'r> Responder<'r, 'static> for CachedFile {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        if req
            .headers()
            .get(IF_MODIFIED_SINCE.as_str())
            .any(|s| s == &self.1)
        {
            Response::build().status(Status::NotModified).ok()
        } else {
            Response::build_from(self.0.respond_to(req)?)
                .raw_header(CACHE_CONTROL.as_str(), "max-age=86400") //  24h (24*60*60)
                .raw_header(LAST_MODIFIED.as_str(), self.1)
                .ok()
        }
    }
}

#[get("/<file..>")]
async fn files(file: PathBuf) -> Option<CachedFile> {
    let buf = Path::new("static/").join(file);
    let file = if buf.is_dir() {
        buf.join("index.html")
    } else {
        buf
    };
    match NamedFile::open(&file).await {
        Ok(named_file) => {
            let metadata = std::fs::metadata(&file).ok()?;
            let last_modified: DateTime<Utc> = metadata.modified().ok()?.into();
            let last_modified = last_modified.to_rfc3339();
            Some(CachedFile(named_file, last_modified))
        }
        Err(_) => None,
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
