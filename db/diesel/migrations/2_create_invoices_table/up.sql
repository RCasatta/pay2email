CREATE TABLE invoices (
    id CHAR(64) NOT NULL PRIMARY KEY,

    bolt11 VARCHAR NOT NULL,

    expiration TIMESTAMP NOT NULL,

    paid BOOLEAN NOT NULL DEFAULT 0,
    showed BOOLEAN NOT NULL DEFAULT 0
);

CREATE INDEX idx_invoices
ON invoices (expiration, paid, showed) where paid = false AND showed = false;
