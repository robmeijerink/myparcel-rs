use aes_gcm::aead::AeadInOut;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce, Tag};
use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};
use p256::pkcs8::{DecodePrivateKey, EncodePrivateKey, LineEnding};
use serde_json::json;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::connect::ConnectError;
use crate::connect::config::EncryptionKey;

const FORMAT_VERSION: u8 = 1;
const IV_LEN: usize = 12;
const TAG_LEN: usize = 16;
const MIN_CIPHERTEXT_LEN: usize = 1 + IV_LEN + TAG_LEN;

pub(crate) fn base64url(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

pub(crate) fn base64url_decode(text: &str) -> Option<Vec<u8>> {
    URL_SAFE_NO_PAD.decode(text).ok()
}

pub(crate) fn random_bytes<const N: usize>() -> Result<[u8; N], ConnectError> {
    let mut bytes = [0u8; N];
    getrandom::fill(&mut bytes).map_err(|_| ConnectError::Encryption)?;
    Ok(bytes)
}

pub(crate) fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(u64::MAX, |elapsed| elapsed.as_secs())
}

pub(crate) struct DpopKey(SigningKey);

impl DpopKey {
    pub fn generate() -> Result<Self, ConnectError> {
        loop {
            let seed = Zeroizing::new(random_bytes::<32>()?);
            if let Ok(key) = SigningKey::from_slice(seed.as_slice()) {
                return Ok(Self(key));
            }
        }
    }

    pub fn from_pem(pem: &str) -> Result<Self, ConnectError> {
        if !pem.trim_start().starts_with("-----BEGIN") {
            return Err(ConnectError::Decryption);
        }
        SigningKey::from_pkcs8_pem(pem)
            .or_else(|_| {
                use p256::elliptic_curve::SecretKey;
                SecretKey::<p256::NistP256>::from_sec1_pem(pem).map(SigningKey::from)
            })
            .map(Self)
            .map_err(|_| ConnectError::Decryption)
    }

    pub fn to_pem(&self) -> Result<Zeroizing<String>, ConnectError> {
        self.0
            .to_pkcs8_pem(LineEnding::LF)
            .map_err(|_| ConnectError::Encryption)
    }

    fn coordinates(&self) -> (String, String) {
        let point = self.0.verifying_key().to_encoded_point(false);
        let x = point.x().expect("an uncompressed point has x");
        let y = point.y().expect("an uncompressed point has y");
        (base64url(x), base64url(y))
    }

    pub fn thumbprint(&self) -> String {
        let (x, y) = self.coordinates();
        let canonical = format!(r#"{{"crv":"P-256","kty":"EC","x":"{x}","y":"{y}"}}"#);
        base64url(&Sha256::digest(canonical.as_bytes()))
    }

    pub fn proof(
        &self,
        method: &str,
        url: &str,
        nonce: Option<&str>,
        access_token: Option<&str>,
        issued_at: u64,
    ) -> Result<String, ConnectError> {
        if url.contains(['?', '#']) {
            return Err(ConnectError::InvalidCallback(
                "the proof url must not have a query or fragment".into(),
            ));
        }
        let (x, y) = self.coordinates();
        let header = json!({"typ": "dpop+jwt", "alg": "ES256", "jwk": {"crv": "P-256", "kty": "EC", "x": x, "y": y}});
        let mut claims = json!({
            "jti": base64url(&random_bytes::<32>()?),
            "htm": method,
            "htu": url,
            "iat": issued_at,
        });
        if let Some(nonce) = nonce.filter(|nonce| !nonce.is_empty()) {
            claims["nonce"] = json!(nonce);
        }
        if let Some(token) = access_token {
            claims["ath"] = json!(base64url(&Sha256::digest(token.as_bytes())));
        }
        let signing_input = format!("{}.{}", encode_json(&header)?, encode_json(&claims)?);
        let signature: Signature = self.0.sign(signing_input.as_bytes());
        Ok(format!(
            "{signing_input}.{}",
            base64url(&signature.to_bytes())
        ))
    }
}

fn encode_json(value: &serde_json::Value) -> Result<String, ConnectError> {
    serde_json::to_vec(value)
        .map(|bytes| base64url(&bytes))
        .map_err(|_| ConnectError::Encryption)
}

pub(crate) struct Cipher {
    key: Zeroizing<[u8; 32]>,
}

impl Cipher {
    pub fn new(key: &EncryptionKey) -> Self {
        Self {
            key: Zeroizing::new(Sha256::digest(key.as_bytes()).into()),
        }
    }

    fn aes(&self) -> Aes256Gcm {
        Aes256Gcm::new_from_slice(self.key.as_slice()).expect("the key has 32 bytes")
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> Result<String, ConnectError> {
        let iv = random_bytes::<IV_LEN>()?;
        let mut buffer = Zeroizing::new(plaintext.to_vec());
        let tag = self
            .aes()
            .encrypt_inout_detached(
                &Nonce::from(iv),
                &[FORMAT_VERSION],
                buffer.as_mut_slice().into(),
            )
            .map_err(|_| ConnectError::Encryption)?;
        let mut out = Vec::with_capacity(MIN_CIPHERTEXT_LEN + buffer.len());
        out.push(FORMAT_VERSION);
        out.extend_from_slice(&iv);
        out.extend_from_slice(&tag);
        out.extend_from_slice(&buffer);
        Ok(STANDARD.encode(out))
    }

    pub fn decrypt(&self, encoded: &str) -> Result<Zeroizing<Vec<u8>>, ConnectError> {
        let raw = STANDARD
            .decode(encoded)
            .map_err(|_| ConnectError::Decryption)?;
        if raw.len() < MIN_CIPHERTEXT_LEN || raw[0] != FORMAT_VERSION {
            return Err(ConnectError::Decryption);
        }
        let (iv, rest) = raw[1..].split_at(IV_LEN);
        let (tag, ciphertext) = rest.split_at(TAG_LEN);
        let iv: [u8; IV_LEN] = iv.try_into().map_err(|_| ConnectError::Decryption)?;
        let tag: [u8; TAG_LEN] = tag.try_into().map_err(|_| ConnectError::Decryption)?;
        let mut buffer = Zeroizing::new(ciphertext.to_vec());
        self.aes()
            .decrypt_inout_detached(
                &Nonce::from(iv),
                &[FORMAT_VERSION],
                buffer.as_mut_slice().into(),
                &Tag::from(tag),
            )
            .map_err(|_| ConnectError::Decryption)?;
        Ok(buffer)
    }

    pub fn encrypt_key(&self, key: &DpopKey) -> Result<String, ConnectError> {
        self.encrypt(key.to_pem()?.as_bytes())
    }

    pub fn decrypt_key(&self, encoded: &str) -> Result<DpopKey, ConnectError> {
        let pem = self.decrypt(encoded)?;
        DpopKey::from_pem(std::str::from_utf8(&pem).map_err(|_| ConnectError::Decryption)?)
    }

    pub fn encrypt_text(&self, text: &str) -> Result<String, ConnectError> {
        self.encrypt(text.as_bytes())
    }

    pub fn decrypt_text(&self, encoded: &str) -> Result<Zeroizing<String>, ConnectError> {
        let bytes = self.decrypt(encoded)?;
        String::from_utf8(bytes.to_vec())
            .map(Zeroizing::new)
            .map_err(|_| ConnectError::Decryption)
    }
}

#[cfg(test)]
mod tests {
    use p256::ecdsa::VerifyingKey;
    use p256::ecdsa::signature::Verifier;

    use super::*;

    fn cipher(secret: &str) -> Cipher {
        Cipher::new(&EncryptionKey::new(secret).unwrap())
    }

    #[test]
    fn encryption_round_trips_and_uses_the_documented_layout() {
        let sealed = cipher("secret").encrypt(b"hello").unwrap();
        let raw = STANDARD.decode(&sealed).unwrap();
        assert_eq!(raw[0], 1);
        assert_eq!(raw.len(), 1 + 12 + 16 + 5);
        assert_eq!(&*cipher("secret").decrypt(&sealed).unwrap(), b"hello");
    }

    #[test]
    fn decryption_rejects_the_wrong_key_version_and_tampering() {
        let sealed = cipher("secret").encrypt(b"hello").unwrap();
        assert!(cipher("other").decrypt(&sealed).is_err());

        let mut raw = STANDARD.decode(&sealed).unwrap();
        raw[0] = 2;
        assert!(cipher("secret").decrypt(&STANDARD.encode(&raw)).is_err());

        let mut raw = STANDARD.decode(&sealed).unwrap();
        *raw.last_mut().unwrap() ^= 1;
        assert!(cipher("secret").decrypt(&STANDARD.encode(&raw)).is_err());

        assert!(
            cipher("secret")
                .decrypt(&STANDARD.encode([1u8; 10]))
                .is_err()
        );
    }

    #[test]
    fn the_key_round_trips_through_pem() {
        let key = DpopKey::generate().unwrap();
        let sealed = cipher("secret").encrypt_key(&key).unwrap();
        let restored = cipher("secret").decrypt_key(&sealed).unwrap();
        assert_eq!(key.thumbprint(), restored.thumbprint());
        assert!(DpopKey::from_pem("not a pem").is_err());
    }

    #[test]
    fn the_thumbprint_is_the_hash_of_the_canonical_jwk() {
        let key = DpopKey::generate().unwrap();
        let (x, y) = key.coordinates();
        let canonical = format!(r#"{{"crv":"P-256","kty":"EC","x":"{x}","y":"{y}"}}"#);
        assert_eq!(key.thumbprint(), base64url(&Sha256::digest(canonical)));
        assert_eq!(key.thumbprint().len(), 43);
    }

    #[test]
    fn proofs_verify_with_the_embedded_key_and_carry_the_claims() {
        let key = DpopKey::generate().unwrap();
        let proof = key
            .proof(
                "POST",
                "https://x.test/path",
                Some("n1"),
                Some("token"),
                1_700_000_000,
            )
            .unwrap();
        let (signing_input, signature) = proof.rsplit_once('.').unwrap();
        let (header, claims) = signing_input.split_once('.').unwrap();

        let header: serde_json::Value =
            serde_json::from_slice(&base64url_decode(header).unwrap()).unwrap();
        let claims: serde_json::Value =
            serde_json::from_slice(&base64url_decode(claims).unwrap()).unwrap();
        assert_eq!(header["typ"], "dpop+jwt");
        assert_eq!(header["alg"], "ES256");
        assert_eq!(claims["htm"], "POST");
        assert_eq!(claims["htu"], "https://x.test/path");
        assert_eq!(claims["iat"], 1_700_000_000u64);
        assert_eq!(claims["nonce"], "n1");
        assert_eq!(claims["ath"], base64url(&Sha256::digest(b"token")));

        let x = base64url_decode(header["jwk"]["x"].as_str().unwrap()).unwrap();
        let y = base64url_decode(header["jwk"]["y"].as_str().unwrap()).unwrap();
        let mut point = vec![4u8];
        point.extend(x);
        point.extend(y);
        let verifying = VerifyingKey::from_sec1_bytes(&point).unwrap();
        let signature = Signature::from_slice(&base64url_decode(signature).unwrap()).unwrap();
        verifying
            .verify(signing_input.as_bytes(), &signature)
            .unwrap();
    }

    #[test]
    fn proofs_refuse_urls_with_a_query() {
        let key = DpopKey::generate().unwrap();
        assert!(
            key.proof("GET", "https://x.test/?a=b", None, None, 0)
                .is_err()
        );
        assert!(
            key.proof("GET", "https://x.test/#a", None, None, 0)
                .is_err()
        );
    }
}
