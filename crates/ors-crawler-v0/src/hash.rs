use sha2::{Digest, Sha256};

pub fn sha256_hex(input: impl AsRef<[u8]>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_ref());
    format!("sha256:{}", hex::encode(hasher.finalize()))
}

pub fn stable_id(input: &str) -> String {
    let full = sha256_hex(input);
    full.replace("sha256:", "")[..16].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_hex() {
        let input = "test";
        let output = sha256_hex(input);
        assert!(output.starts_with("sha256:"));
        assert_eq!(output.len(), 7 + 64);
    }

    #[test]
    fn test_stable_id() {
        let input = "test";
        let id = stable_id(input);
        assert_eq!(id.len(), 16);

        let id2 = stable_id(input);
        assert_eq!(id, id2);
    }
}
