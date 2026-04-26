use std::fmt;
#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;
#[cfg(not(target_arch = "wasm32"))]
use std::io::{self, Read};
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
use md5::Md5;
#[cfg(not(target_arch = "wasm32"))]
use sha2::{Digest, Sha256, Sha512};

#[cfg(not(target_arch = "wasm32"))]
const BUFFER_SIZE: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum HashAlgorithm {
    #[default]
    Blake3,
    Sha256,
    Sha512,
    Md5,
}

impl HashAlgorithm {
    pub fn all() -> &'static [HashAlgorithm] {
        &[
            HashAlgorithm::Blake3,
            HashAlgorithm::Sha256,
            HashAlgorithm::Sha512,
            HashAlgorithm::Md5,
        ]
    }

    pub fn id(self) -> &'static str {
        match self {
            HashAlgorithm::Blake3 => "BLAKE3",
            HashAlgorithm::Sha256 => "SHA256",
            HashAlgorithm::Sha512 => "SHA512",
            HashAlgorithm::Md5 => "MD5",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            HashAlgorithm::Blake3 => "BLAKE3",
            HashAlgorithm::Sha256 => "SHA-256",
            HashAlgorithm::Sha512 => "SHA-512",
            HashAlgorithm::Md5 => "MD5",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            HashAlgorithm::Blake3 => "기본값, 빠른 일반 검사에 적합",
            HashAlgorithm::Sha256 => "범용 호환성이 높은 256비트 해시",
            HashAlgorithm::Sha512 => "긴 다이제스트가 필요한 검사용",
            HashAlgorithm::Md5 => "레거시 비교용, 보안 용도 아님",
        }
    }

    pub fn from_id(value: &str) -> HashAlgorithm {
        match value.trim().to_ascii_uppercase().as_str() {
            "SHA256" | "SHA-256" => HashAlgorithm::Sha256,
            "SHA512" | "SHA-512" => HashAlgorithm::Sha512,
            "MD5" => HashAlgorithm::Md5,
            _ => HashAlgorithm::Blake3,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn hash_path(self, path: &Path) -> io::Result<String> {
        match self {
            HashAlgorithm::Blake3 => hash_blake3(path),
            HashAlgorithm::Sha256 => hash_digest::<Sha256>(path),
            HashAlgorithm::Sha512 => hash_digest::<Sha512>(path),
            HashAlgorithm::Md5 => hash_digest::<Md5>(path),
        }
    }
}

impl fmt::Display for HashAlgorithm {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn hash_blake3(path: &Path) -> io::Result<String> {
    if let Ok(hash) = blake3::Hasher::new().update_mmap_rayon(path) {
        return Ok(hash.finalize().to_hex().to_string());
    }

    let mut file = File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0; BUFFER_SIZE];

    loop {
        let read = file.read(&mut buffer)?;

        if read == 0 {
            break;
        }

        hasher.update(&buffer[..read]);
    }

    Ok(hasher.finalize().to_hex().to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn hash_digest<D>(path: &Path) -> io::Result<String>
where
    D: Digest + Default,
{
    let mut file = File::open(path)?;
    let mut hasher = D::default();
    let mut buffer = vec![0; BUFFER_SIZE];

    loop {
        let read = file.read(&mut buffer)?;

        if read == 0 {
            break;
        }

        hasher.update(&buffer[..read]);
    }

    Ok(hex_bytes(hasher.finalize().as_slice()))
}

#[cfg(not(target_arch = "wasm32"))]
fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
