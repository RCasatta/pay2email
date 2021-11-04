use crate::Error;
use age::x25519::{Identity, Recipient};
use bech32::{FromBase32, ToBase32, Variant};
use core::iter;
use std::env;
use std::io::{Read, Write};
use std::str::FromStr;

const PUBLIC_KEY: &str = "age1xxzrgrfjm3yrwh3u6a7exgrldked0pdauvr3mx870wl6xzrwm5ps8s2h0p";

/// Encrypt given `plaintext` for server's public key and returns encrypted data bech32 encoded
#[post("/encrypt", data = "<plaintext>")]
pub fn encrypt(plaintext: &str) -> Result<String, Error> {
    let rec = Recipient::from_str(&PUBLIC_KEY).unwrap();
    let encryptor = age::Encryptor::with_recipients(vec![Box::new(rec)]);

    let mut encrypted = vec![];
    let mut writer = encryptor.wrap_output(&mut encrypted)?;
    writer.write_all(plaintext.as_ref()).unwrap();
    writer.finish().unwrap();

    let bech32_encoded = bech32::encode("e", &encrypted.to_base32(), Variant::Bech32m)?;

    Ok(bech32_encoded)
}

/// Decrypt the given `bech32_encrypted` encoded with bech32 with the server's secret key and return
/// the plaintext converted as String
pub fn decrypt(bech32_encrypted: &str) -> Result<String, Error> {
    let key = Identity::from_str(&env::var("AGE_SECRET_KEY").unwrap()).unwrap();
    let (_hrp, data, variant) = bech32::decode(bech32_encrypted)?;
    assert_eq!(variant, Variant::Bech32m);
    let encrypted = Vec::<u8>::from_base32(&data)?;
    let decryptor = match age::Decryptor::new(&encrypted[..])? {
        age::Decryptor::Recipients(d) => d,
        _ => unreachable!(),
    };

    let mut decrypted = vec![];
    let mut reader = decryptor.decrypt(iter::once(&key as &dyn age::Identity))?;
    reader.read_to_end(&mut decrypted).unwrap();

    let result = std::str::from_utf8(&decrypted).unwrap().to_string();

    Ok(result)
}

#[cfg(test)]
mod test {
    use crate::encrypt::{decrypt, encrypt};

    #[test]
    #[ignore] // requires env AGE_SECRET_KEY
    fn test_enc_dec() {
        let plaintext = "Hello world!";

        let encrypted = encrypt(plaintext).unwrap();
        let decrypted = decrypt(&encrypted).unwrap();

        assert_eq!(decrypted, plaintext);
        assert_ne!(encrypted, plaintext);
    }
}
