//! Time-Stamp Protocol (RFC 3161): the request sent to an authority and the
//! check of its signed reply. No network here; the app sends the request.
//!
//! A reply is checked fully offline: the authority's status, the signed
//! TSTInfo (what was stamped, when), the CMS signature over it by the
//! certificate in the token, that certificate's time-stamping purpose and
//! dates, and each certificate in the token signed by the next one up.
//! Whether the top of that chain is an authority to trust is left to the
//! reader (the verifier names it); no root list is built in.

use chrono::{DateTime, Utc};
use cms::cert::CertificateChoices;
use cms::content_info::ContentInfo;
use cms::signed_data::{SignedData, SignerIdentifier, SignerInfo};
use der::asn1::{Int, OctetString};
use der::oid::ObjectIdentifier;
use der::{Any, Decode, Encode};
use sha2::{Digest, Sha256, Sha384, Sha512};
use x509_cert::Certificate;
use x509_cert::ext::pkix::{ExtendedKeyUsage, SubjectKeyIdentifier};
use x509_cert::name::Name;
use x509_cert::spki::AlgorithmIdentifier;
use x509_tsp::{MessageImprint, TimeStampReq, TimeStampResp, TspVersion, TstInfo};

const fn oid(s: &str) -> ObjectIdentifier {
    ObjectIdentifier::new_unwrap(s)
}

const SHA256: ObjectIdentifier = oid("2.16.840.1.101.3.4.2.1");
const SHA384: ObjectIdentifier = oid("2.16.840.1.101.3.4.2.2");
const SHA512: ObjectIdentifier = oid("2.16.840.1.101.3.4.2.3");
const RSA: ObjectIdentifier = oid("1.2.840.113549.1.1.1");
const RSA_SHA256: ObjectIdentifier = oid("1.2.840.113549.1.1.11");
const RSA_SHA384: ObjectIdentifier = oid("1.2.840.113549.1.1.12");
const RSA_SHA512: ObjectIdentifier = oid("1.2.840.113549.1.1.13");
const ECDSA_SHA256: ObjectIdentifier = oid("1.2.840.10045.4.3.2");
const ECDSA_SHA384: ObjectIdentifier = oid("1.2.840.10045.4.3.3");
const ECDSA_SHA512: ObjectIdentifier = oid("1.2.840.10045.4.3.4");
const SIGNED_DATA: ObjectIdentifier = oid("1.2.840.113549.1.7.2");
const TST_INFO: ObjectIdentifier = oid("1.2.840.113549.1.9.16.1.4");
const CONTENT_TYPE: ObjectIdentifier = oid("1.2.840.113549.1.9.3");
const MESSAGE_DIGEST: ObjectIdentifier = oid("1.2.840.113549.1.9.4");
const EXT_KEY_USAGE: ObjectIdentifier = oid("2.5.29.37");
const KEY_ID: ObjectIdentifier = oid("2.5.29.14");
const TIME_STAMPING: ObjectIdentifier = oid("1.3.6.1.5.5.7.3.8");
const COMMON_NAME: ObjectIdentifier = oid("2.5.4.3");
const ORGANIZATION: ObjectIdentifier = oid("2.5.4.10");

/// Certificates in a token checked up the chain at most this far.
const MAX_CHAIN: usize = 6;

/// The DER of a TimeStampReq for a SHA-256 `imprint`, asking for the
/// authority's certificate in the reply. `nonce` ties the reply to this
/// request; it is sent as a positive 8-byte integer.
pub fn request(imprint: &[u8; 32], nonce: u64) -> Vec<u8> {
    let req = TimeStampReq {
        version: TspVersion::V1,
        message_imprint: MessageImprint {
            hash_algorithm: AlgorithmIdentifier {
                oid: SHA256,
                parameters: Some(Any::null()),
            },
            hashed_message: OctetString::new(imprint.to_vec()).expect("32 bytes"),
        },
        req_policy: None,
        nonce: Some(Int::new(&nonce_bytes(nonce)).expect("a minimal integer")),
        cert_req: true,
        extensions: None,
    };
    req.to_der().expect("a request encodes")
}

/// `nonce` as the bytes of a positive DER integer: eight bytes with the top
/// one between 0x01 and 0x7f, so no sign or padding byte is needed.
pub fn nonce_bytes(nonce: u64) -> [u8; 8] {
    ((nonce & 0x7fff_ffff_ffff_ffff) | 0x0100_0000_0000_0000).to_be_bytes()
}

/// What a checked token says.
#[derive(Debug, Clone, PartialEq)]
pub struct Stamp {
    /// When the authority signed.
    pub gen_time: DateTime<Utc>,
    /// What it signed: the hash sent in the request.
    pub imprint: Vec<u8>,
    /// The imprint is a SHA-256 hash (the only kind this app sends).
    pub sha256: bool,
    pub nonce: Option<Vec<u8>>,
    /// The token's serial number (hex).
    pub serial: String,
    /// The certificate that signed: its common name and organization.
    pub signer: String,
    /// Who issued the top certificate in the token: the authority to trust.
    pub top: String,
    /// Certificates in the token whose signature was checked, signer first.
    pub chain: usize,
}

type Check<T> = std::result::Result<T, String>;

/// Reads a TimeStampResp (`reply`) and checks it; see the module notes.
/// Errors are short sentences for the writer or a reader of the verifier.
pub fn check(reply: &[u8]) -> Check<Stamp> {
    let resp = TimeStampResp::from_der(reply).map_err(|e| format!("응답을 읽을 수 없음 ({e})"))?;
    // 0 granted, 1 granted with changes; the rest are refusals.
    if resp.status.status as u8 > 1 {
        return Err(format!("인증 기관이 거절함 ({:?})", resp.status.status));
    }
    let token = resp
        .time_stamp_token
        .ok_or_else(|| "응답에 토큰이 없음".to_string())?;
    check_token(&token)
}

/// Checks a TimeStampToken (the ContentInfo inside a reply).
pub fn check_token(token: &ContentInfo) -> Check<Stamp> {
    if token.content_type != SIGNED_DATA {
        return Err("서명된 토큰이 아님".into());
    }
    let signed = token
        .content
        .decode_as::<SignedData>()
        .map_err(|e| format!("토큰을 읽을 수 없음 ({e})"))?;
    let encap = &signed.encap_content_info;
    if encap.econtent_type != TST_INFO {
        return Err("시각 인증 내용이 아님".into());
    }
    let tst_der = encap
        .econtent
        .as_ref()
        .ok_or_else(|| "시각 인증 내용이 비어 있음".to_string())?
        .value();
    let tst =
        TstInfo::from_der(tst_der).map_err(|e| format!("시각 인증 내용을 읽을 수 없음 ({e})"))?;

    let certs: Vec<&Certificate> = signed
        .certificates
        .iter()
        .flat_map(|set| set.0.iter())
        .filter_map(|c| match c {
            CertificateChoices::Certificate(c) => Some(c),
            CertificateChoices::Other(_) => None,
        })
        .collect();
    let signer_info = signed
        .signer_infos
        .0
        .iter()
        .next()
        .ok_or_else(|| "서명이 없음".to_string())?;
    let signer = certs
        .iter()
        .copied()
        .find(|c| is_signer(c, signer_info))
        .ok_or_else(|| "서명한 인증서가 토큰에 없음".to_string())?;

    let gen_time = to_utc(tst.gen_time.to_unix_duration())?;
    check_signer_info(signer_info, signer, tst_der)?;
    check_purpose(signer, gen_time)?;
    let (chain, top) = check_chain(signer, &certs)?;

    Ok(Stamp {
        gen_time,
        imprint: tst.message_imprint.hashed_message.as_bytes().to_vec(),
        sha256: tst.message_imprint.hash_algorithm.oid == SHA256,
        nonce: tst.nonce.map(|n| n.as_bytes().to_vec()),
        serial: crate::journal::hex(tst.serial_number.as_bytes()),
        signer: describe(&signer.tbs_certificate.subject),
        top,
        chain,
    })
}

fn to_utc(d: std::time::Duration) -> Check<DateTime<Utc>> {
    DateTime::from_timestamp(d.as_secs() as i64, d.subsec_nanos())
        .ok_or_else(|| "시각을 읽을 수 없음".to_string())
}

fn is_signer(cert: &Certificate, info: &SignerInfo) -> bool {
    match &info.sid {
        SignerIdentifier::IssuerAndSerialNumber(id) => {
            cert.tbs_certificate.issuer == id.issuer
                && cert.tbs_certificate.serial_number == id.serial_number
        }
        SignerIdentifier::SubjectKeyIdentifier(key_id) => cert
            .tbs_certificate
            .extensions
            .iter()
            .flatten()
            .filter(|e| e.extn_id == KEY_ID)
            .filter_map(|e| SubjectKeyIdentifier::from_der(e.extn_value.as_bytes()).ok())
            .any(|own| own.0 == key_id.0),
    }
}

/// The signed attributes name the TSTInfo and carry its hash, and the
/// signature over them is the signer's.
fn check_signer_info(info: &SignerInfo, signer: &Certificate, tst_der: &[u8]) -> Check<()> {
    let attrs = info
        .signed_attrs
        .as_ref()
        .ok_or_else(|| "서명된 속성이 없음".to_string())?;
    let value_of = |id: ObjectIdentifier| {
        attrs
            .iter()
            .find(|a| a.oid == id)
            .and_then(|a| a.values.iter().next())
    };
    let content_type = value_of(CONTENT_TYPE)
        .and_then(|v| v.decode_as::<ObjectIdentifier>().ok())
        .ok_or_else(|| "내용 종류가 서명되지 않음".to_string())?;
    if content_type != TST_INFO {
        return Err("서명된 내용 종류가 다름".into());
    }
    let digest = value_of(MESSAGE_DIGEST)
        .and_then(|v| v.decode_as::<OctetString>().ok())
        .ok_or_else(|| "내용의 지문이 서명되지 않음".to_string())?;
    let hash = Hash::of_digest(&info.digest_alg.oid)?;
    if digest.as_bytes() != hash.digest(tst_der).as_slice() {
        return Err("서명된 지문이 시각 인증 내용과 맞지 않음".into());
    }
    let signed = attrs
        .to_der()
        .map_err(|e| format!("서명된 속성을 읽을 수 없음 ({e})"))?;
    verify(
        signer,
        &info.signature_algorithm.oid,
        Some(hash),
        &signed,
        info.signature.as_bytes(),
    )
    .map_err(|e| format!("토큰 서명이 맞지 않음 ({e})"))
}

/// The signer is a time-stamping certificate valid when it signed.
fn check_purpose(signer: &Certificate, at: DateTime<Utc>) -> Check<()> {
    let validity = &signer.tbs_certificate.validity;
    let from = to_utc(validity.not_before.to_unix_duration())?;
    let until = to_utc(validity.not_after.to_unix_duration())?;
    if at < from || at > until {
        return Err("서명한 인증서가 그 시각에 유효하지 않음".into());
    }
    let stamping = signer
        .tbs_certificate
        .extensions
        .iter()
        .flatten()
        .filter(|e| e.extn_id == EXT_KEY_USAGE)
        .filter_map(|e| ExtendedKeyUsage::from_der(e.extn_value.as_bytes()).ok())
        .any(|usage| usage.0.contains(&TIME_STAMPING));
    if !stamping {
        return Err("서명한 인증서가 시각 인증용이 아님".into());
    }
    Ok(())
}

/// Follows issuers through the certificates in the token, checking each
/// signature. Returns how many certificates were checked and who issued the
/// top one.
fn check_chain(signer: &Certificate, certs: &[&Certificate]) -> Check<(usize, String)> {
    let mut current = signer;
    let mut checked = 1;
    for _ in 0..MAX_CHAIN {
        let tbs = &current.tbs_certificate;
        if tbs.issuer == tbs.subject {
            // Self-signed: its own signature says nothing more.
            break;
        }
        let Some(issuer) = certs
            .iter()
            .copied()
            .find(|c| c.tbs_certificate.subject == tbs.issuer)
        else {
            break;
        };
        let body = tbs
            .to_der()
            .map_err(|e| format!("인증서를 읽을 수 없음 ({e})"))?;
        let signature = current
            .signature
            .as_bytes()
            .ok_or_else(|| "인증서 서명을 읽을 수 없음".to_string())?;
        verify(
            issuer,
            &current.signature_algorithm.oid,
            None,
            &body,
            signature,
        )
        .map_err(|e| format!("인증서 사슬이 맞지 않음 ({e})"))?;
        current = issuer;
        checked += 1;
    }
    Ok((checked, describe(&current.tbs_certificate.issuer)))
}

#[derive(Clone, Copy)]
enum Hash {
    Sha256,
    Sha384,
    Sha512,
}

impl Hash {
    fn of_digest(id: &ObjectIdentifier) -> Check<Hash> {
        match *id {
            SHA256 => Ok(Hash::Sha256),
            SHA384 => Ok(Hash::Sha384),
            SHA512 => Ok(Hash::Sha512),
            _ => Err(format!("지원하지 않는 지문 방식 {id}")),
        }
    }

    fn digest(self, bytes: &[u8]) -> Vec<u8> {
        match self {
            Hash::Sha256 => Sha256::digest(bytes).to_vec(),
            Hash::Sha384 => Sha384::digest(bytes).to_vec(),
            Hash::Sha512 => Sha512::digest(bytes).to_vec(),
        }
    }
}

/// Checks `signature` over `message` with `key_holder`'s public key.
/// `digest` is the hash for a bare `rsaEncryption` algorithm (CMS signer
/// infos name the hash separately).
fn verify(
    key_holder: &Certificate,
    algorithm: &ObjectIdentifier,
    digest: Option<Hash>,
    message: &[u8],
    signature: &[u8],
) -> Check<()> {
    let spki = key_holder
        .tbs_certificate
        .subject_public_key_info
        .to_der()
        .map_err(|e| e.to_string())?;
    let (ecdsa, hash) = match *algorithm {
        RSA => (false, digest.ok_or("지문 방식이 없음")?),
        RSA_SHA256 => (false, Hash::Sha256),
        RSA_SHA384 => (false, Hash::Sha384),
        RSA_SHA512 => (false, Hash::Sha512),
        ECDSA_SHA256 => (true, Hash::Sha256),
        ECDSA_SHA384 => (true, Hash::Sha384),
        ECDSA_SHA512 => (true, Hash::Sha512),
        _ => return Err(format!("지원하지 않는 서명 방식 {algorithm}")),
    };
    if ecdsa {
        verify_ecdsa(&spki, &hash.digest(message), signature)
    } else {
        verify_rsa(&spki, hash, message, signature)
    }
}

fn verify_rsa(spki: &[u8], hash: Hash, message: &[u8], signature: &[u8]) -> Check<()> {
    use rsa::pkcs1v15::{Signature, VerifyingKey};
    use rsa::pkcs8::DecodePublicKey;
    use rsa::signature::Verifier;
    let key = rsa::RsaPublicKey::from_public_key_der(spki).map_err(|e| e.to_string())?;
    let signature = Signature::try_from(signature).map_err(|e| e.to_string())?;
    match hash {
        Hash::Sha256 => VerifyingKey::<Sha256>::new(key).verify(message, &signature),
        Hash::Sha384 => VerifyingKey::<Sha384>::new(key).verify(message, &signature),
        Hash::Sha512 => VerifyingKey::<Sha512>::new(key).verify(message, &signature),
    }
    .map_err(|e| e.to_string())
}

fn verify_ecdsa(spki: &[u8], prehash: &[u8], signature: &[u8]) -> Check<()> {
    use p256::ecdsa::signature::hazmat::PrehashVerifier;
    use p256::pkcs8::DecodePublicKey;
    if let Ok(key) = p256::ecdsa::VerifyingKey::from_public_key_der(spki) {
        let sig = p256::ecdsa::Signature::from_der(signature).map_err(|e| e.to_string())?;
        return key.verify_prehash(prehash, &sig).map_err(|e| e.to_string());
    }
    if let Ok(key) = p384::ecdsa::VerifyingKey::from_public_key_der(spki) {
        let sig = p384::ecdsa::Signature::from_der(signature).map_err(|e| e.to_string())?;
        return key.verify_prehash(prehash, &sig).map_err(|e| e.to_string());
    }
    Err("지원하지 않는 공개 키".into())
}

/// "CN (O)" of a name, or the whole name when it has no common name.
fn describe(name: &Name) -> String {
    let find = |id: ObjectIdentifier| {
        name.0
            .iter()
            .flat_map(|rdn| rdn.0.iter())
            .find(|atv| atv.oid == id)
            .and_then(|atv| text_of(&atv.value))
    };
    match (find(COMMON_NAME), find(ORGANIZATION)) {
        (Some(cn), Some(o)) if !cn.contains(&o) => format!("{cn} ({o})"),
        (Some(cn), _) => cn,
        _ => name.to_string(),
    }
}

fn text_of(value: &Any) -> Option<String> {
    use der::asn1::{Ia5StringRef, PrintableStringRef, TeletexStringRef, Utf8StringRef};
    value
        .decode_as::<Utf8StringRef>()
        .map(|s| s.as_str().to_string())
        .or_else(|_| {
            value
                .decode_as::<PrintableStringRef>()
                .map(|s| s.as_str().to_string())
        })
        .or_else(|_| {
            value
                .decode_as::<Ia5StringRef>()
                .map(|s| s.as_str().to_string())
        })
        .or_else(|_| {
            value
                .decode_as::<TeletexStringRef>()
                .map(|s| s.as_str().to_string())
        })
        .ok()
}

/// The token (ContentInfo) inside a reply, as DER: what the bundle and
/// `openssl ts -verify -token_in` work with.
pub fn token_of(reply: &[u8]) -> Check<Vec<u8>> {
    let resp = TimeStampResp::from_der(reply).map_err(|e| format!("응답을 읽을 수 없음 ({e})"))?;
    resp.time_stamp_token
        .ok_or_else(|| "응답에 토큰이 없음".to_string())?
        .to_der()
        .map_err(|e| e.to_string())
}
