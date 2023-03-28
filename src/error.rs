use age::{DecryptError, EncryptError};
use lettre::address::AddressError;
use lettre::transport::smtp;
use lightning_invoice::ParseOrSemanticError;
use qr_code::bmp_monochrome::BmpError;
use qr_code::types::QrError;
use rocket::http::{ContentType, Status};
use rocket::response::Responder;
use rocket::Request;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Bech32(bech32::Error),
    Decryption(DecryptError),
    Encryption(EncryptError),
    Diesel(diesel::result::Error),
    Bolt11(ParseOrSemanticError),
    EmailAddress(AddressError),
    Lettre(lettre::error::Error),
    Hex(bitcoin_hashes::hex::Error),
    Smtp(smtp::Error),
    Qr(QrError),
    Bmp(BmpError),
    Serde(serde_json::Error),
    InvalidContentType(ContentType),
    InvoiceExpired,
    MissingTo,
    OnlyOneTo,
    MissingSubject,
    OnlyOneSubject,
    EmptyMessage,
    Unauthorized,
    InvoiceNotFound,
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Serde(e)
    }
}

impl From<BmpError> for Error {
    fn from(e: BmpError) -> Self {
        Error::Bmp(e)
    }
}

impl From<QrError> for Error {
    fn from(e: QrError) -> Self {
        Error::Qr(e)
    }
}

impl From<bitcoin_hashes::hex::Error> for Error {
    fn from(e: bitcoin_hashes::hex::Error) -> Self {
        Error::Hex(e)
    }
}

impl From<smtp::Error> for Error {
    fn from(e: smtp::Error) -> Self {
        Error::Smtp(e)
    }
}

impl From<lettre::error::Error> for Error {
    fn from(e: lettre::error::Error) -> Self {
        Error::Lettre(e)
    }
}

impl From<AddressError> for Error {
    fn from(e: AddressError) -> Self {
        Error::EmailAddress(e)
    }
}

impl From<ParseOrSemanticError> for Error {
    fn from(e: ParseOrSemanticError) -> Self {
        Error::Bolt11(e)
    }
}

impl From<bech32::Error> for Error {
    fn from(e: bech32::Error) -> Self {
        Error::Bech32(e)
    }
}

impl From<age::DecryptError> for Error {
    fn from(e: DecryptError) -> Self {
        Error::Decryption(e)
    }
}

impl From<age::EncryptError> for Error {
    fn from(e: EncryptError) -> Self {
        Error::Encryption(e)
    }
}

impl From<diesel::result::Error> for Error {
    fn from(e: diesel::result::Error) -> Self {
        Error::Diesel(e)
    }
}

impl<'r> Responder<'r, 'static> for Error {
    fn respond_to(self, _request: &'r Request<'_>) -> rocket::response::Result<'static> {
        println!("{:?}", self);
        Err(Status::ImATeapot)
    }
}
