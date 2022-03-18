use serde::Deserialize;
use std::collections::BTreeMap;
use std::io::{Error, ErrorKind};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct Attestation {
    method: String,
    registration_id: Option<String>,
    trust_bundle_cert: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct Authentication {
    method: String,
    device_id_pk: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Provisioning {
    source: String,
    global_endpoint: Option<String>,
    id_scope: Option<String>,
    attestation: Option<Attestation>,
    authentication: Option<Authentication>,
    iothub_hostname: Option<String>,
    connection_string: Option<String>,
    device_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct EdgeCA {
    cert: String,
    pk: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct IdentityConfig {
    hostname: String,
    local_gateway_hostname: Option<String>,
    provisioning: Option<Provisioning>,
    edge_ca: Option<EdgeCA>,
}

pub enum IdentityType {
    Standalone,
    Leaf,
    Gateway,
}
const WARN_EMPTY_HOSTNAME: &'static str = "The hostname is an empty String.";
const WARN_MISSING_PROVISIONING: &'static str = "A provisioning section should be specified.";
const WARN_MISSING_DPS_PARAMS: &'static str =
    "For provisioning source dps, global_endpoint and id_scope should be specified.";
const WARN_MISSING_MANUAL_PARAMS: &'static str =
    "For provisioning source manual, either connection_string or iothub_hostname and device_id are required.";
const WARN_MISSING_AUTHENTICATION: &'static str =
    "For provisioning source manual, an authentication section should be present in the provisioning section.";
const WARN_MISSING_ATTESTATION: &'static str =
    "For provisioning source dps an attestation section should be present in the provisioning section.";
const WARN_ATTESTATION_VALID_METHOD_EXPECTED: &'static str =
    "The attestation method should be tpm, x509 or symmetric_key.";
const WARN_INVALID_SOURCE: &'static str = "The provisioning source should be dps or manual.";
const WARN_AUTHENTICATION_VALID_METHOD_EXPECTED: &'static str =
    "The authentication method should be sas.";

pub fn validate_identity(
    _id_type: IdentityType,
    config_file_name: &std::path::PathBuf,
) -> Result<Vec<&'static str>, Box<dyn std::error::Error>> {
    let mut out = Vec::<&'static str>::new();
    let file_content = std::fs::read_to_string(config_file_name)?;
    let des = &mut toml::Deserializer::new(&file_content);
    let body: Result<IdentityConfig, _> = serde_path_to_error::deserialize(des);
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
    if body.hostname.is_empty() {
        out.push(WARN_EMPTY_HOSTNAME)
    }
    match body.provisioning {
        None => {
            out.push(WARN_MISSING_PROVISIONING);
        }
        Some(p) => match p.source.as_str() {
            "dps" => {
                if p.global_endpoint.is_none() || p.id_scope.is_none() {
                    out.push(WARN_MISSING_DPS_PARAMS);
                }
                match p.attestation {
                    None => {
                        out.push(WARN_MISSING_ATTESTATION);
                    }
                    Some(a) => {
                        if a.method != "tpm" && a.method != "x509" && a.method != "symmetric_key" {
                            out.push(WARN_ATTESTATION_VALID_METHOD_EXPECTED);
                        }
                    }
                }
            }
            "manual" => {
                if p.connection_string.is_none()
                    && (p.iothub_hostname.is_none() || p.device_id.is_none())
                {
                    out.push(WARN_MISSING_MANUAL_PARAMS);
                }

                if p.connection_string.is_none() {
                    match p.authentication {
                        None => {
                            out.push(WARN_MISSING_AUTHENTICATION);
                        }
                        Some(a) => {
                            if a.method != "sas" {
                                out.push(WARN_AUTHENTICATION_VALID_METHOD_EXPECTED);
                            }
                        }
                    }
                }
            }
            &_ => {
                out.push(WARN_INVALID_SOURCE);
            }
        },
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    lazy_static! {
        static ref LOG: () =
            env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
                .init();
    }
    #[test]
    fn identity_config_hostname_empty() {
        lazy_static::initialize(&LOG);
        assert_eq!(
            None,
            validate_identity(
                IdentityType::Standalone,
                &std::path::PathBuf::from("testfiles/identity_config_hostname_empty.toml"),
            )
            .unwrap_err()
            .to_string()
            .find(WARN_EMPTY_HOSTNAME)
        );
    }

    #[test]
    fn identity_config_standalone_empty() {
        lazy_static::initialize(&LOG);
        assert_ne!(
            None,
            validate_identity(
                IdentityType::Standalone,
                &std::path::PathBuf::from("testfiles/identity_config_empty.toml"),
            )
            .unwrap_err()
            .to_string()
            .find("missing field `hostname`")
        );
    }

    #[test]
    fn identity_config_standalone_minimal() {
        lazy_static::initialize(&LOG);
        let result = validate_identity(
            IdentityType::Standalone,
            &std::path::PathBuf::from("testfiles/identity_config_minimal.toml"),
        )
        .unwrap();
        assert_eq!(1, result.len());
        assert_ne!(
            None,
            result[0].find("provisioning section should be specified")
        );
    }

    #[test]
    fn identity_config_standalone_dps() {
        lazy_static::initialize(&LOG);
        let result = validate_identity(
            IdentityType::Standalone,
            &std::path::PathBuf::from("testfiles/identity_config_dps.toml"),
        )
        .unwrap();
        assert_eq!(2, result.len());
        assert_ne!(None, result[0].find("provisioning source dps"));
        assert_ne!(None, result[1].find("provisioning source dps"));
        assert_ne!(None, result[0].find("global_endpoint and id_scope"));
        assert_ne!(None, result[1].find("attestation section"));
    }

    #[test]
    fn identity_config_standalone_manual() {
        lazy_static::initialize(&LOG);
        let result = validate_identity(
            IdentityType::Standalone,
            &std::path::PathBuf::from("testfiles/identity_config_manual.toml"),
        )
        .unwrap();

        assert_eq!(2, result.len());
        assert_ne!(None, result[0].find("provisioning source manual"));
        assert_ne!(
            None,
            result[0].find("either connection_string or iothub_hostname and device_id")
        );
        assert_ne!(None, result[1].find("provisioning source manual"));
        assert_ne!(None, result[1].find("authentication section"));
    }

    #[test]
    fn identity_config_standalone_manual_con_str() {
        lazy_static::initialize(&LOG);
        let result = validate_identity(
            IdentityType::Standalone,
            &std::path::PathBuf::from("testfiles/identity_config_manual_connection_string.toml"),
        )
        .unwrap();

        assert_eq!(0, result.len());
    }

    #[test]
    fn identity_config_standalone_dps_tpm() {
        lazy_static::initialize(&LOG);
        let result = validate_identity(
            IdentityType::Standalone,
            &std::path::PathBuf::from("testfiles/identity_config_dps_tpm.toml"),
        )
        .unwrap();
        assert_eq!(0, result.len());
    }

    #[test]
    fn identity_config_standalone_dps_sas() {
        lazy_static::initialize(&LOG);
        let result = validate_identity(
            IdentityType::Standalone,
            &std::path::PathBuf::from("testfiles/identity_config_dps_sas.toml"),
        )
        .unwrap();
        assert_eq!(1, result.len());
        assert_ne!(
            None,
            result[0].find("attestation method should be tpm, x509 or symmetric_key")
        );
    }

    #[test]
    fn identity_config_leaf_empty() {
        lazy_static::initialize(&LOG);
        assert_ne!(
            None,
            validate_identity(
                IdentityType::Leaf,
                &std::path::PathBuf::from("testfiles/identity_config_empty.toml"),
            )
            .unwrap_err()
            .to_string()
            .find("missing field `hostname`")
        );
    }

    #[test]
    fn identity_config_leaf_minimal() {
        lazy_static::initialize(&LOG);
        let result = validate_identity(
            IdentityType::Leaf,
            &std::path::PathBuf::from("testfiles/identity_config_minimal.toml"),
        )
        .unwrap();
        assert_eq!(1, result.len());
        assert_ne!(
            None,
            result[0].find("provisioning section should be specified")
        );
    }

    #[test]
    fn identity_config_leaf_dps() {
        lazy_static::initialize(&LOG);
        let result = validate_identity(
            IdentityType::Leaf,
            &std::path::PathBuf::from("testfiles/identity_config_dps.toml"),
        )
        .unwrap();
        assert_eq!(2, result.len());
        assert_ne!(None, result[0].find("provisioning source dps"));
        assert_ne!(None, result[0].find("global_endpoint and id_scope"));
        assert_ne!(None, result[1].find("provisioning source dps"));
        assert_ne!(None, result[1].find("attestation section"));
    }

    #[test]
    fn identity_config_leaf_manual() {
        lazy_static::initialize(&LOG);
        let result = validate_identity(
            IdentityType::Leaf,
            &std::path::PathBuf::from("testfiles/identity_config_manual.toml"),
        )
        .unwrap();
        assert_eq!(2, result.len());
        assert_ne!(None, result[0].find("provisioning source manual"));
        assert_ne!(
            None,
            result[0].find("either connection_string or iothub_hostname and device_id")
        );
        assert_ne!(None, result[1].find("provisioning source manual"));
        assert_ne!(None, result[1].find("authentication section"));
    }

    #[test]
    fn identity_config_leaf_manual_sas() {
        lazy_static::initialize(&LOG);
        let result = validate_identity(
            IdentityType::Leaf,
            &std::path::PathBuf::from("testfiles/identity_config_manual_sas.toml"),
        )
        .unwrap();
        assert_eq!(0, result.len());
    }

    #[test]
    fn identity_config_leaf_manual_tpm() {
        lazy_static::initialize(&LOG);
        let result = validate_identity(
            IdentityType::Leaf,
            &std::path::PathBuf::from("testfiles/identity_config_manual_tpm.toml"),
        )
        .unwrap();
        assert_eq!(1, result.len());
        assert_ne!(None, result[0].find("authentication method should be sas"));
    }

    #[test]
    fn identity_config_leaf_no_warn() {
        lazy_static::initialize(&LOG);
        let result = validate_identity(
            IdentityType::Leaf,
            &std::path::PathBuf::from("conf/config.toml.ics-iot-leaf.template"),
        )
        .unwrap();
        assert_eq!(0, result.len());
    }

    #[test]
    fn identity_config_gateway_empty() {
        lazy_static::initialize(&LOG);
        assert_ne!(
            None,
            validate_identity(
                IdentityType::Gateway,
                &std::path::PathBuf::from("testfiles/identity_config_empty.toml"),
            )
            .unwrap_err()
            .to_string()
            .find("missing field `hostname`")
        );
    }

    #[test]
    fn identity_config_gateway_minimal() {
        lazy_static::initialize(&LOG);
        let result = validate_identity(
            IdentityType::Gateway,
            &std::path::PathBuf::from("testfiles/identity_config_minimal.toml"),
        )
        .unwrap();
        assert_eq!(1, result.len());
        assert_ne!(
            None,
            result[0].find("provisioning section should be specified")
        );
    }

    #[test]
    fn identity_config_gateway_dps() {
        lazy_static::initialize(&LOG);
        let result = validate_identity(
            IdentityType::Gateway,
            &std::path::PathBuf::from("testfiles/identity_config_dps.toml"),
        )
        .unwrap();
        assert_eq!(2, result.len());
        assert_ne!(None, result[0].find("provisioning source dps"));
        assert_ne!(None, result[0].find("global_endpoint and id_scope"));
        assert_ne!(None, result[1].find("provisioning source dps"));
        assert_ne!(None, result[1].find("attestation section"));
    }

    #[test]
    fn identity_config_gateway_manual() {
        lazy_static::initialize(&LOG);
        let result = validate_identity(
            IdentityType::Gateway,
            &std::path::PathBuf::from("testfiles/identity_config_manual.toml"),
        )
        .unwrap();
        assert_eq!(2, result.len());
        assert_ne!(None, result[0].find("provisioning source manual"));
        assert_ne!(
            None,
            result[0].find("either connection_string or iothub_hostname and device_id")
        );
        assert_ne!(None, result[1].find("provisioning source manual"));
        assert_ne!(None, result[1].find("authentication section"));
    }

    #[test]
    fn identity_config_gateway_dps_tpm() {
        lazy_static::initialize(&LOG);
        let result = validate_identity(
            IdentityType::Gateway,
            &std::path::PathBuf::from("testfiles/identity_config_dps_tpm.toml"),
        )
        .unwrap();
        assert_eq!(0, result.len());
    }

    #[test]
    fn identity_config_gateway_dps_sas() {
        lazy_static::initialize(&LOG);
        let result = validate_identity(
            IdentityType::Gateway,
            &std::path::PathBuf::from("testfiles/identity_config_dps_sas.toml"),
        )
        .unwrap();
        assert_eq!(1, result.len());
        assert_ne!(
            None,
            result[0].find("attestation method should be tpm, x509 or symmetric_key")
        );
    }

    #[test]
    fn identity_config_gateway_no_warn() {
        lazy_static::initialize(&LOG);
        let result = validate_identity(
            IdentityType::Gateway,
            &std::path::PathBuf::from("conf/config.toml.ics-iotedge-gateway.template"),
        )
        .unwrap();
        assert_eq!(0, result.len());
    }
}
