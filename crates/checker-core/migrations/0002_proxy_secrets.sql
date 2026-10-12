-- Encrypted proxy passwords (see secret.rs) and the last check message.
ALTER TABLE proxies ADD COLUMN password_enc TEXT;
ALTER TABLE proxies ADD COLUMN message TEXT;
