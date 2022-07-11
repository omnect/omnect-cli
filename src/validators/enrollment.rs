use log::debug;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufReader, Error, ErrorKind};
use validator::Validate;

lazy_static! {
    static ref RE_CONNECTION_STRING: regex::Regex = regex::Regex::new(
        "^((HostName=[^;]*;?)|(SharedAccessKeyName=[^;]*;?)|(SharedAccessKey=[^;]*;?))+$"
    )
    .unwrap();
}

#[derive(Debug, Validate, Deserialize)]
#[allow(dead_code)]
struct EnrollmentConfig {
    #[validate(regex = "RE_CONNECTION_STRING")]
    #[serde(rename = "dpsConnectionString")]
    dps_connection_string: String,
    #[validate(regex = "RE_CONNECTION_STRING")]
    #[serde(rename = "iothubConnectionString")]
    iothub_connection_string: String,
    tags: Option<BTreeMap<String, String>>,
}

pub fn validate_enrollment(
    config_file_name: &std::path::PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(config_file_name)?;
    let reader = BufReader::new(file);
    let des = &mut serde_json::Deserializer::from_reader(reader);
    let body: Result<EnrollmentConfig, _> = serde_path_to_error::deserialize(des);
    let body = match body {
        Err(e) => {
            return Err(Box::new(Error::new(
                ErrorKind::Other,
                format!(
                    "{} parsing failed with error {}",
                    config_file_name.to_string_lossy(),
                    e.to_string()
                ),
            )));
        }
        Ok(body) => body,
    };
    match body.validate() {
        Ok(_) => {
            debug!("Enrollment config validated.");
        }
        Err(e) => {
            return Err(Box::new(Error::new(
                ErrorKind::Other,
                format!("Enrollment validation failed with error {}", e.to_string()),
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn enrollment_config_missing_dps() {
        assert_ne!(
            None,
            validate_enrollment(&PathBuf::from(
                "testfiles/enrollment_static_missing_dps.json"
            ),)
            .unwrap_err()
            .to_string()
            .find("missing field `dpsConnectionString`")
        );
    }

    #[test]
    fn enrollment_config_missing_iothub() {
        assert_ne!(
            None,
            validate_enrollment(&PathBuf::from(
                "testfiles/enrollment_static_missing_iothub.json"
            ),)
            .unwrap_err()
            .to_string()
            .find("missing field `iothubConnectionString`")
        );
    }

    #[test]
    fn enrollment_config_unknown_key() {
        assert_eq!(
            true,
            validate_enrollment(&PathBuf::from(
                "testfiles/enrollment_static_unknown_key.json"
            ),)
            .is_ok()
        );
    }

    #[test]
    fn enrollment_config_invalid_connection_string() {
        assert_ne!(
            None,
            validate_enrollment(&PathBuf::from(
                "testfiles/enrollment_static_invalid_connection_string.json"
            ),)
            .unwrap_err()
            .to_string()
            .find("Enrollment validation failed")
        );
    }
}
