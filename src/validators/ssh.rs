use regex::Regex;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result};

const VALID_KEY_TYPES: [&str; 1] = ["ed25519"];

fn validate_key_type(root_ca_file: &Path) -> Result<()> {
    let key = std::fs::read_to_string(root_ca_file).context("validate key type")?;

    let re = Regex::new(r"ssh-(.*) [^ ]+ [^ ]+").unwrap();

    let mat = re
        .captures(&key)
        .ok_or_else(|| anyhow::anyhow!("not a valid ssh public key"))?;

    let key_type = mat.get(1).unwrap().as_str(); // safe

    VALID_KEY_TYPES
        .iter()
        .find(|&&valid_key_type| valid_key_type == key_type)
        .map(|_| ())
        .ok_or_else(|| anyhow::anyhow!("unsupported key type: {key_type}"))
}

fn validate_key_format(root_ca_file: &Path) -> Result<()> {
    let status = Command::new("ssh-keygen")
        .args(["-l", "-f", &root_ca_file.to_string_lossy()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("validate ssh key format")?;

    status
        .success()
        .then_some(())
        .ok_or_else(|| anyhow::anyhow!("invalid key format"))
}

pub fn validate_ssh_pub_key(root_ca_file: &Path) -> Result<()> {
    validate_key_type(root_ca_file)?;

    validate_key_format(root_ca_file)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{path::PathBuf, str::FromStr};

    fn ed25519_key() -> PathBuf {
        PathBuf::from_str("testfiles/ssh_ca_ed25519.pub").unwrap()
    }

    fn non_ed25519_key() -> PathBuf {
        PathBuf::from_str("testfiles/ssh_ca_rsa.pub").unwrap()
    }

    fn invalid_file() -> PathBuf {
        PathBuf::from_str("testfiles/non_ssh_ca").unwrap()
    }

    #[test]
    fn validate_ed25519_key_type() {
        let key = ed25519_key();

        assert!(matches!(validate_key_type(&key), Ok(())));
    }

    #[test]
    fn decline_non_ed25519_key_type() {
        let key = non_ed25519_key();

        assert!(matches!(validate_key_type(&key), Err(anyhow::Error { .. })));
    }

    #[test]
    fn decline_invalid_key_type() {
        let key = invalid_file();

        assert!(matches!(validate_key_type(&key), Err(anyhow::Error { .. })));
    }

    #[test]
    fn validate_ed25519_key_format() {
        let key = ed25519_key();

        assert!(matches!(validate_key_format(&key), Ok(())));
    }

    #[test]
    fn decline_invalid_key_format() {
        let key = invalid_file();

        assert!(matches!(
            validate_key_format(&key),
            Err(anyhow::Error { .. })
        ));
    }

    #[test]
    fn validate_ed25519_ssh_pub_key() {
        let key = ed25519_key();

        assert!(matches!(validate_ssh_pub_key(&key), Ok(())));
    }

    #[test]
    fn decline_non_ed25519_ssh_pub_key() {
        let key = non_ed25519_key();

        assert!(matches!(
            validate_ssh_pub_key(&key),
            Err(anyhow::Error { .. })
        ));
    }

    #[test]
    fn decline_invalid_ssh_pub_key() {
        let key = invalid_file();

        assert!(matches!(
            validate_ssh_pub_key(&key),
            Err(anyhow::Error { .. })
        ));
    }
}
