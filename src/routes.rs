use crate::db::run_migrations;
use crate::db::{EmailRow, InvoiceRow};
use crate::encrypt::decrypt;
use crate::error::Result;
use crate::{qr, Db, Error};
use bitcoin_hashes::hex::{FromHex, ToHex};
use bitcoin_hashes::{sha256, Hash};
use chrono::NaiveDateTime;
use lettre::message::{Mailbox, Mailboxes};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use lightning_invoice::Invoice;
use rocket::fairing::AdHoc;
use rocket::form::{DataField, Form, FromFormField, ValueField};
use rocket::http::{ContentType, Status};
use rocket::request::{FromRequest, Outcome};
use rocket::response::status::Created;
use rocket::serde::json::Json;
use rocket::{form, Request};
use serde::Serialize;
use std::env;
use std::time::UNIX_EPOCH;

struct HttpAuth;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for HttpAuth {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let map = request.headers();
        let authorization = map.get("authorization").next();
        match authorization {
            Some(str) if str == &env::var("HTTP_AUTH_BASIC").unwrap() => Outcome::Success(HttpAuth),
            _ => Outcome::Failure((Status::Unauthorized, Error::Unauthorized)),
        }
    }
}

/// Add an invoice in the database
#[post("/invoice", data = "<bolt11>")]
async fn invoice_add(db: Db, bolt11: String, _auth: HttpAuth) -> Result<Created<Json<InvoiceRow>>> {
    let invoice: Invoice = bolt11.parse()?;
    let time = invoice.timestamp() + invoice.expiry_time();
    let expiration =
        NaiveDateTime::from_timestamp(time.duration_since(UNIX_EPOCH).unwrap().as_secs() as i64, 0);

    if invoice.is_expired() {
        return Err(Error::InvoiceExpired);
    }

    let invoice_row = InvoiceRow {
        id: invoice.payment_hash().to_hex(),
        bolt11,
        expiration,
        paid: false,
        showed: false,
    };
    println!("invoice: {:?}", invoice_row);

    InvoiceRow::add(&db, invoice_row.clone()).await?;

    Ok(Created::new("/").body(Json(invoice_row)))
}

/// Returns all the invoices available, checking they are not expired or near expiration
#[get("/invoice/count")]
async fn invoice_count(db: Db) -> Result<Json<i64>> {
    Ok(Json(InvoiceRow::count_available(&db).await?))
}

/// Returns email sent
#[get("/email/sent")]
async fn email_sent(db: Db, _auth: HttpAuth) -> Result<Json<i64>> {
    Ok(Json(EmailRow::count_sent(&db).await?))
}

/// Set the invoice to paid, send the email
#[post("/invoice/paid", data = "<preimage>")]
async fn invoice_paid(db: Db, preimage: String, _auth: HttpAuth) -> Result<Json<InvoiceRow>> {
    let preimage = Vec::<u8>::from_hex(&preimage)?;
    let payment_hash = sha256::Hash::hash(&preimage);
    let mut invoice = InvoiceRow::get(&db, payment_hash.into_inner().to_hex()).await?;
    invoice.set_paid(&db).await?;

    let mut email_row = EmailRow::get(&db, invoice.id.clone()).await?;
    send_email(&email_row).await?;
    email_row.set_sent(&db).await?;

    Ok(Json(invoice))
}

#[derive(Serialize)]
struct Info {
    payment_hash: String,
    invoice_paid: bool,
    email_sent: bool,
}

/// get info if the invoice is paid and the mail sent
#[post("/info", data = "<payment_hash>")]
async fn info(db: Db, payment_hash: String) -> Option<Json<Info>> {
    let invoice_row = InvoiceRow::get(&db, payment_hash.clone()).await.ok()?;
    if invoice_row.paid {
        // gonna check if email is sent only if invoice is paid because it surely comes first
        let email_row = EmailRow::get(&db, payment_hash.clone()).await.ok()?;

        let info = Info {
            email_sent: email_row.sent,
            invoice_paid: invoice_row.paid,
            payment_hash,
        };
        Some(Json(info))
    } else {
        let info = Info {
            email_sent: false,
            invoice_paid: false,
            payment_hash,
        };
        Some(Json(info))
    }
}

pub(crate) async fn send_email(email_row: &EmailRow) -> Result<()> {
    let mut builder = Message::builder()
        .from("Pay2.email <noreply@pay2.email>".parse()?)
        .subject(&email_row.subject);
    for mbox in email_row.to_email.parse::<Mailboxes>()?.into_iter() {
        builder = builder.to(mbox);
    }
    if let Some(reply_to) = email_row.reply_to_email.as_ref() {
        builder = builder.reply_to(reply_to.parse()?)
    }
    let email = builder.body(email_row.message.clone())?;

    let password = env::var("SMTP_PASSWORD").unwrap();
    let creds = Credentials::new("noreply@pay2.email".to_string(), password);

    let relay = "smtp.improvmx.com";
    let tls_parameters = TlsParameters::new(relay.into())?;
    let relay = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(relay)
        .port(587)
        .tls(Tls::Required(tls_parameters));

    let mailer: AsyncSmtpTransport<Tokio1Executor> = relay.credentials(creds).build();

    // Send the email
    mailer.send(email).await?;
    Ok(())
}

#[derive(FromForm, Debug)]
pub struct SendData<'r> {
    /// The reply_to email address used in the email, can optionally contain name such as
    /// "John Smith <example@email.com>"
    reply_to: Option<EMail>,

    /// The message in the email, form limit is 32kb so we don't bother limiti here the size
    message: &'r str,

    /// Email recipient in clear text, use `to_enc` for encrypted version
    to: Option<EMails>,

    /// Encrypted recipient
    to_enc: Option<Encrypted<EMails>>,

    /// Email subkect in clear text, use `subject_enc` for encrypted version
    subject: Option<String>,

    /// Encrypted subject
    subject_enc: Option<Encrypted<String>>,
}

#[derive(Debug)]
struct Encrypted<T>(pub T);

#[derive(Debug)]
struct EMail(Mailbox);

#[derive(Debug)]
struct EMails(Mailboxes);

#[rocket::async_trait]
impl<'r> FromFormField<'r> for EMail {
    fn from_value(field: ValueField<'r>) -> form::Result<'r, Self> {
        let m: Mailbox = field
            .value
            .parse()
            .map_err(|e| form::Error::validation(format!("Cannot parse email: {:?}", e)))?;
        Ok(EMail(m))
    }

    async fn from_data(_field: DataField<'r, '_>) -> form::Result<'r, Self> {
        todo!("async not implemented yet")
    }
}

#[rocket::async_trait]
impl<'r> FromFormField<'r> for Encrypted<EMail> {
    fn from_value(field: ValueField<'r>) -> form::Result<'r, Self> {
        let decrypted_value = decrypt(field.value).map_err(|e| {
            form::Error::validation(format!("Cannot decrypt email field: {:?} ", e))
        })?;
        let m: Mailbox = decrypted_value
            .parse()
            .map_err(|e| form::Error::validation(format!("Cannot parse v: {:?}", e)))?;
        Ok(Encrypted(EMail(m)))
    }

    async fn from_data(_field: DataField<'r, '_>) -> form::Result<'r, Self> {
        todo!("async not implemented yet")
    }
}

#[rocket::async_trait]
impl<'r> FromFormField<'r> for EMails {
    fn from_value(field: ValueField<'r>) -> form::Result<'r, Self> {
        let m: Mailboxes = field
            .value
            .parse()
            .map_err(|e| form::Error::validation(format!("Cannot parse email: {:?}", e)))?;
        Ok(EMails(m))
    }

    async fn from_data(_field: DataField<'r, '_>) -> form::Result<'r, Self> {
        todo!("async not implemented yet")
    }
}

#[rocket::async_trait]
impl<'r> FromFormField<'r> for Encrypted<EMails> {
    fn from_value(field: ValueField<'r>) -> form::Result<'r, Self> {
        let decrypted_value = decrypt(field.value).map_err(|e| {
            form::Error::validation(format!("Cannot decrypt email field: {:?} ", e))
        })?;
        let m: Mailboxes = decrypted_value
            .parse()
            .map_err(|e| form::Error::validation(format!("Cannot parse v: {:?}", e)))?;
        Ok(Encrypted(EMails(m)))
    }

    async fn from_data(_field: DataField<'r, '_>) -> form::Result<'r, Self> {
        todo!("async not implemented yet")
    }
}

#[rocket::async_trait]
impl<'r> FromFormField<'r> for Encrypted<String> {
    fn from_value(field: ValueField<'r>) -> form::Result<'r, Self> {
        let decrypted_value = decrypt(field.value).map_err(|e| {
            form::Error::validation(format!("Cannot decrypt email field: {:?} ", e))
        })?;
        Ok(Encrypted(decrypted_value))
    }

    async fn from_data(_field: DataField<'r, '_>) -> form::Result<'r, Self> {
        todo!("async not implemented yet")
    }
}

struct AcceptEncoding(ContentType);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AcceptEncoding {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let accept_encoding = request.headers().get("accept").next();
        match accept_encoding {
            Some(str) if str == "application/json" => {
                Outcome::Success(AcceptEncoding(ContentType::JSON))
            }
            _ => Outcome::Success(AcceptEncoding(ContentType::HTML)),
        }
    }
}

#[derive(Debug)]
struct Referer(Option<String>);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Referer {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let referer = request
            .headers()
            .get("referer")
            .next()
            .map(|e| e.to_string());
        Outcome::Success(Referer(referer))
    }
}

#[derive(Serialize)]
struct JsonResult {
    pub bolt11: String,
    pub reply_to: Option<String>,
    pub message: String,
    pub payment_hash: String,
}

#[post("/", data = "<data>")]
async fn email(
    db: Db,
    data: Form<SendData<'_>>,
    encoding: AcceptEncoding,
    referer: Referer,
) -> Result<(ContentType, String)> {
    let to = match (data.to.as_ref(), data.to_enc.as_ref()) {
        (Some(_), Some(_)) => return Err(Error::OnlyOneTo),
        (Some(e), None) => e,
        (None, Some(e)) => &e.0,
        (None, None) => return Err(Error::MissingTo),
    };
    let subject = match (data.subject.as_ref(), data.subject_enc.as_ref()) {
        (Some(_), Some(_)) => return Err(Error::OnlyOneSubject),
        (Some(s), None) => s,
        (None, Some(s)) => &s.0,
        (None, None) => return Err(Error::MissingSubject),
    };
    if data.message.is_empty() {
        return Err(Error::EmptyMessage);
    }

    let invoice = InvoiceRow::get_first_available(&db).await?;

    let email_row = EmailRow {
        id: None,
        payment_hash: invoice.id.to_string(),
        reply_to_email: data.reply_to.as_ref().map(|e| e.0.to_string()),
        to_email: to.0.to_string(),
        subject: subject.to_string(),
        message: data.message.to_string(),
        sent: false,
    };

    let message = email_row.message.clone();
    let reply_to = email_row.reply_to_email.clone();
    EmailRow::add(&db, email_row).await?;

    if encoding.0.is_json() {
        let json_result = JsonResult {
            bolt11: invoice.bolt11.clone(),
            message,
            reply_to,
            payment_hash: invoice.id.clone(),
        };
        return Ok((encoding.0, serde_json::to_string(&json_result)?));
    } else if encoding.0.is_html() {
        let qr = qr::create_bmp_base64_qr(&invoice.bolt11.to_ascii_uppercase())?;
        let link = format!("lightning:{}", &invoice.bolt11);
        let mut template = include_str!("../static/invoice.html").to_string();
        if let Some(str) = referer.0.as_ref() {
            let back_to = format!("<p>Back to <a href=\"{}\">{}</a></p>", str, str);
            template = template.replace("{{ BACK_TO }}", &back_to);
        }
        template = template.replace("{{ REPLY_TO }}", &reply_to.unwrap_or("N/A".to_string()));
        template = template.replace("{{ MESSAGE }}", &message);
        template = template.replace("{{ QR }}", &qr);
        template = template.replace("{{ INVOICE }}", &invoice.bolt11);
        template = template.replace("{{ PAYMENT_HASH }}", &invoice.id);
        template = template.replace("{{ LINK }}", &link);

        Ok((encoding.0, template))
    } else {
        return Err(Error::InvalidContentType(encoding.0));
    }
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Diesel SQLite Stage", |rocket| async {
        rocket
            .attach(Db::fairing())
            .attach(AdHoc::on_ignite("Diesel Migrations", run_migrations))
            .mount(
                "/",
                routes![
                    invoice_count,
                    invoice_add,
                    invoice_paid,
                    email,
                    info,
                    email_sent
                ],
            )
    })
}

#[cfg(test)]
mod test {
    use lettre::message::{Mailbox, Mailboxes};

    #[test]
    fn test_parse() {
        let s = "mail@example.com";
        assert!(s.parse::<Mailbox>().is_ok());

        let s = "mail@example.com ";
        assert!(s.parse::<Mailbox>().is_err());

        let s = "mail@example.com ";
        assert!(s.trim().parse::<Mailbox>().is_ok());

        let s = "mail@example.com, another@asa.it";
        let mbs = s.trim().parse::<Mailboxes>();
        assert!(mbs.is_ok());

        let s = "mail@example.com";
        let mbs = s.trim().parse::<Mailboxes>();
        assert!(mbs.is_ok());
    }
}
