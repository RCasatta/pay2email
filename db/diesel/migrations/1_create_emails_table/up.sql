CREATE TABLE emails (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    payment_hash CHAR(64) NOT NULL,

    reply_to_email VARCHAR,
    to_email VARCHAR NOT NULL,

    subject VARCHAR NOT NULL,
    message VARCHAR NOT NULL,

    sent BOOLEAN NOT NULL DEFAULT 0
);

CREATE UNIQUE INDEX idx_emails
ON emails (payment_hash);