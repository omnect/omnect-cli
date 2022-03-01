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
#[serde(deny_unknown_fields)]
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
                format!("Json parsing failed with error {}", e.to_string()),
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
