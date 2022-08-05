use crate::error::Result;
use crate::Db;
use chrono::{NaiveDateTime, Utc};
use diesel::dsl::count;
use diesel::prelude::*;
use rocket::serde::{Deserialize, Serialize};
use rocket::{Build, Rocket};
use rocket_sync_db_pools::diesel;

#[derive(Debug, Clone, Deserialize, Serialize, Queryable, Insertable, Identifiable)]
#[serde(crate = "rocket::serde")]
#[table_name = "invoices"]
pub struct InvoiceRow {
    pub id: String, // hex of the payment hash (64 chars)
    pub bolt11: String,

    #[serde(with = "my_date_format")]
    pub expiration: NaiveDateTime,

    pub paid: bool,
    pub showed: bool,
}

table! {
    invoices (id) {
        id -> Text,
        bolt11 -> Text,
        expiration -> Timestamp,
        paid -> Bool,
        showed -> Bool,
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Queryable, Insertable, Identifiable)]
#[serde(crate = "rocket::serde")]
#[table_name = "emails"]
pub struct EmailRow {
    #[serde(skip_deserializing)]
    pub id: Option<i32>,
    pub payment_hash: String, // 64
    pub reply_to_email: Option<String>,
    pub to_email: String,
    pub subject: String,
    pub message: String,
    pub sent: bool,
}

table! {
    emails (id) {
        id -> Nullable<Integer>,
        payment_hash -> Text,
        reply_to_email -> Nullable<Text>,
        to_email -> Text,
        subject -> Text,
        message -> Text,
        sent -> Bool,
    }
}

impl InvoiceRow {
    /// Get the invoice_row identified by `payment_hash`
    pub async fn get(db: &Db, payment_hash: String) -> Result<InvoiceRow> {
        Ok(db
            .run(move |conn| {
                invoices::table
                    .find(payment_hash)
                    .get_result::<InvoiceRow>(conn)
            })
            .await?)
    }

    /// Add the given `invoice_row` in db
    pub async fn add(db: &Db, invoice_row: InvoiceRow) -> Result<usize> {
        Ok(db
            .run(move |conn| {
                diesel::insert_into(invoices::table)
                    .values(invoice_row)
                    .execute(conn)
            })
            .await?)
    }

    /// Return the id of an invoice (payment hash) which is not payed, not expired and not showed
    /// before. Set it to showed
    pub async fn get_first_available(db: &Db) -> Result<InvoiceRow> {
        let mut invoice: InvoiceRow = db
            .run(move |conn| {
                invoices::table
                    .filter(invoices::showed.eq(false))
                    .filter(invoices::paid.eq(false))
                    .filter(invoices::expiration.gt(max_expiration()))
                    .first::<InvoiceRow>(conn)
            })
            .await?;

        let invoice_cloned = invoice.clone();
        db.run(move |conn| {
            diesel::update(&invoice_cloned)
                .set(invoices::showed.eq(true))
                .execute(conn)
        })
        .await?;
        invoice.showed = true;

        Ok(invoice)
    }

    /// List all available invoices, never showed, nor paid, nor expired
    pub async fn count_available(db: &Db) -> Result<i64> {
        Ok(db
            .run(move |conn| {
                invoices::table
                    .select(count(invoices::id))
                    .filter(invoices::showed.eq(false))
                    .filter(invoices::paid.eq(false))
                    .filter(invoices::expiration.gt(max_expiration()))
                    .first(conn)
            })
            .await?)
    }

    /// Set the expired flag to true
    pub async fn set_paid(&mut self, db: &Db) -> Result<()> {
        let cloned = self.clone();
        db.run(move |conn| {
            diesel::update(&cloned)
                .set(invoices::paid.eq(true))
                .execute(conn)
        })
        .await?;
        self.paid = true;
        Ok(())
    }
}

fn max_expiration() -> NaiveDateTime {
    Utc::now().naive_utc() + chrono::Duration::hours(1)
}

impl EmailRow {
    /// Return the prepared email associated with given `payment_hash`
    pub async fn get(db: &Db, payment_hash: String) -> Result<EmailRow> {
        let email_row: EmailRow = db
            .run(move |conn| {
                emails::table
                    .filter(emails::payment_hash.eq(payment_hash))
                    .first::<EmailRow>(conn)
            })
            .await?;

        Ok(email_row)
    }

    /// Add the given `email_row` in db
    pub async fn add(db: &Db, email_row: EmailRow) -> Result<usize> {
        Ok(db
            .run(move |conn| {
                diesel::insert_into(emails::table)
                    .values(email_row)
                    .execute(conn)
            })
            .await?)
    }

    /// Set the sent flag to true
    pub async fn set_sent(&mut self, db: &Db) -> Result<()> {
        let cloned = self.clone();
        db.run(move |conn| {
            diesel::update(&cloned)
                .set(emails::sent.eq(true))
                .execute(conn)
        })
        .await?;
        self.sent = true;
        Ok(())
    }

    /// Count sent email
    pub async fn count_sent(db: &Db) -> Result<i64> {
        Ok(db
            .run(move |conn| {
                emails::table
                    .select(count(emails::id))
                    .filter(emails::sent.eq(true))
                    .first(conn)
            })
            .await?)
    }
}

pub async fn run_migrations(rocket: Rocket<Build>) -> Rocket<Build> {
    // This macro from `diesel_migrations` defines an `embedded_migrations`
    // module containing a function named `run` that runs the migrations in the
    // specified directory, initializing the database.
    embed_migrations!("db/diesel/migrations");

    let conn = Db::get_one(&rocket).await.expect("database connection");
    conn.run(|c| embedded_migrations::run(c))
        .await
        .expect("diesel migrations");

    rocket
}

mod my_date_format {
    use chrono::NaiveDateTime;
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &'static str = "%Y-%m-%d %H:%M:%S";
    pub fn serialize<S>(date: &NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        NaiveDateTime::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)
    }
}
