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
