use anyhow::{anyhow, Result};
use log::info;
use regex::Regex;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::PathBuf;
use validator::Validate;

lazy_static! {
    //
    static ref RE_HOSTNAME: Regex = Regex::new(
        // hostname validation against https://www.rfc-editor.org/rfc/rfc1035 in order to pass "iotedge check"
        r"^[a-zA-Z]([a-zA-Z0-9-]*[a-zA-Z0-9])?(\.[a-zA-Z]([a-zA-Z0-9-]*[a-zA-Z0-9])?)*$"
    )
    .unwrap();
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct CertAutoRenew {
    rotate_key: bool,
    threshold: String,
    retry: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct IdentityCert {
    method: String,
    common_name: String,
    auto_renew: Option<CertAutoRenew>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct Attestation {
    method: String,
    registration_id: Option<String>,
    trust_bundle_cert: Option<String>,
    identity_cert: Option<IdentityCert>,
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
#[allow(dead_code)]
struct Payload {
    uri: String,
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
    payload: Option<Payload>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct TpmHierarchyAuthorization {
    endorsement: Option<String>,
    owner: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct TpmEndpoints {
    aziot_tpmd: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct Tpm {
    tcti: Option<String>,
    auth_key_index: Option<u32>,
    hierarchy_authorization: Option<TpmHierarchyAuthorization>,
    endpoints: Option<TpmEndpoints>,
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
struct Auth {
    bootstrap_identity_cert: String,
    bootstrap_identity_pk: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct Urls {
    default: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code, clippy::upper_case_acronyms)]
struct EST {
    auth: Auth,
    urls: Urls,
    trusted_certs: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct CertIssuance {
    est: Option<EST>,
}

#[derive(Debug, Validate, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct IdentityConfig {
    #[validate(regex(
        path = "RE_HOSTNAME",
        code = "hostname validation",
        message = "hostname is not compliant with rfc1035"
    ))]
    hostname: String,
    local_gateway_hostname: Option<String>,
    provisioning: Option<Provisioning>,
    tpm: Option<Tpm>,
    edge_ca: Option<EdgeCA>,
    cert_issuance: Option<CertIssuance>,
}

pub enum IdentityType {
    Standalone,
    Leaf,
    Gateway,
}

const PAYLOAD_FILEPATH: &str = "file:///etc/omnect/dps-payload.json";
const WARN_MISSING_PROVISIONING: &str = "A provisioning section should be specified.";
const WARN_MISSING_DPS_PARAMS: &str =
    "For provisioning source dps, global_endpoint and id_scope should be specified.";
const WARN_MISSING_MANUAL_PARAMS: &str =
    "For provisioning source manual, either connection_string or iothub_hostname and device_id are required.";
const WARN_MISSING_AUTHENTICATION: &str =
    "For provisioning source manual, an authentication section should be present in the provisioning section.";
const WARN_MISSING_ATTESTATION: &str =
    "For provisioning source dps an attestation section should be present in the provisioning section.";
const WARN_ATTESTATION_VALID_METHOD_EXPECTED: &str =
    "The attestation method should be tpm, x509 or symmetric_key.";
const WARN_INVALID_SOURCE: &str = "The provisioning source should be dps or manual.";
const WARN_AUTHENTICATION_VALID_METHOD_EXPECTED: &str = "The authentication method should be sas.";
const WARN_UNEXPECTED_PATH: &str = "Unexpected path found.";
const WARN_UNEQUAL_COMMON_NAME_AND_REGISTRATION_ID: &str =
    "provisioning.attestation.registration_id is not equal to provisioning.attestation.identity_cert.common_name";
const WARN_PAYLOAD_FILEPATH_MISSING: &str = "Payload file is configred but file is missing.";
const WARN_PAYLOAD_CONFIG_MISSING: &str = "Payload file is passed but not configred.";

pub fn validate_identity(
    _id_type: IdentityType,
    config_file_name: &PathBuf,
    payload: &Option<PathBuf>,
) -> Result<Vec<&'static str>> {
    let mut out = Vec::<&'static str>::new();
    let file_content = std::fs::read_to_string(config_file_name)?;
    info!("validate identity for:\n{}", file_content);
    let des = &mut toml::Deserializer::new(&file_content);
    let body: Result<IdentityConfig, _> = serde_path_to_error::deserialize(des);
    let body = match body {
        Err(e) => {
            return Err(anyhow!(
                "{} parsing failed with error {}",
                config_file_name.to_string_lossy(),
                e
            ));
        }
        Ok(body) => body,
    };
    body.validate()?;
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
                    Some(a) => match a.method.as_str() {
                        "x509" => {
                            if Some(false)
                                == a.identity_cert
                                    .map(|ic| ic.common_name == a.registration_id.unwrap())
                            {
                                out.push(WARN_UNEQUAL_COMMON_NAME_AND_REGISTRATION_ID)
                            }
                        }
                        "tpm" | "symmetric_key" => {}
                        _ => out.push(WARN_ATTESTATION_VALID_METHOD_EXPECTED),
                    },
                }
                if p.payload.is_some() {
                    if p.payload.unwrap().uri.ne(PAYLOAD_FILEPATH) {
                        out.push(WARN_UNEXPECTED_PATH);
                    } else if payload.is_none() {
                        out.push(WARN_PAYLOAD_FILEPATH_MISSING);
                    } else {
                        let payload = payload.as_deref();
                        let file_content = std::fs::read_to_string(payload.unwrap())?;
                        let _: serde::de::IgnoredAny = serde_json::from_str(&file_content)
                            .map_err(|e| {
                                anyhow!(
                                    "{} parsing failed with error {}",
                                    payload.unwrap().to_string_lossy(),
                                    e
                                )
                            })?;
                    }
                } else if payload.is_some() {
                    out.push(WARN_PAYLOAD_CONFIG_MISSING);
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

    if Some(false)
        == body
            .cert_issuance
            .as_ref()
            .and_then(|ci| ci.est.as_ref())
            .map(|est| {
                est.auth.bootstrap_identity_cert.as_str()
                    == "file:///mnt/cert/priv/device_id_cert.pem"
                    && est.auth.bootstrap_identity_cert.as_str()
                        == "file:///mnt/cert/priv/device_id_cert.pem"
                    && est
                        .trusted_certs
                        .iter()
                        .any(|e| e == "file:///mnt/cert/ca/ca.crt")
            })
    {
        out.push(WARN_UNEXPECTED_PATH)
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
        assert_ne!(
            None,
            validate_identity(
                IdentityType::Standalone,
                &PathBuf::from("testfiles/identity_config_hostname_empty.toml"),
                &None,
            )
            .unwrap_err()
            .to_string()
            .find("hostname is not compliant with rfc1035")
        );
    }

    #[test]
    fn identity_config_hostname_invalid() {
        lazy_static::initialize(&LOG);
        assert_ne!(
            None,
            validate_identity(
                IdentityType::Standalone,
                &PathBuf::from("testfiles/identity_config_hostname_invalid.toml"),
                &None,
            )
            .unwrap_err()
            .to_string()
            .find("hostname is not compliant with rfc1035")
        );
    }

    #[test]
    fn identity_config_hostname_valid() {
        lazy_static::initialize(&LOG);
        assert!(validate_identity(
            IdentityType::Standalone,
            &PathBuf::from("testfiles/identity_config_hostname_valid.toml"),
            &None,
        )
        .is_ok());
    }

    #[test]
    fn identity_config_standalone_empty() {
        lazy_static::initialize(&LOG);
        assert_ne!(
            None,
            validate_identity(
                IdentityType::Standalone,
                &PathBuf::from("testfiles/identity_config_empty.toml"),
                &None,
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
            &PathBuf::from("testfiles/identity_config_minimal.toml"),
            &None,
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
            &PathBuf::from("testfiles/identity_config_dps.toml"),
            &None,
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
            &PathBuf::from("testfiles/identity_config_manual.toml"),
            &None,
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
            &PathBuf::from("testfiles/identity_config_manual_connection_string.toml"),
            &None,
        )
        .unwrap();

        assert_eq!(0, result.len());
    }

    #[test]
    fn identity_config_standalone_dps_tpm() {
        lazy_static::initialize(&LOG);
        let result = validate_identity(
            IdentityType::Standalone,
            &PathBuf::from("testfiles/identity_config_dps_tpm.toml"),
            &None,
        )
        .unwrap();
        assert_eq!(0, result.len());
    }

    #[test]
    fn identity_config_standalone_dps_sas() {
        lazy_static::initialize(&LOG);
        let result = validate_identity(
            IdentityType::Standalone,
            &PathBuf::from("testfiles/identity_config_dps_sas.toml"),
            &None,
        )
        .unwrap();
        assert_eq!(1, result.len());
        assert_ne!(
            None,
            result[0].find("attestation method should be tpm, x509 or symmetric_key")
        );
    }

    #[test]
    fn identity_config_dps_x509_est() {
        lazy_static::initialize(&LOG);
        let result = validate_identity(
            IdentityType::Standalone,
            &PathBuf::from("testfiles/identity_config_dps_x509_est.toml"),
            &None,
        )
        .unwrap();
        assert_eq!(0, result.len());
    }

    #[test]
    fn identity_config_dps_payload() {
        lazy_static::initialize(&LOG);
        let result = validate_identity(
            IdentityType::Standalone,
            &PathBuf::from("testfiles/identity_config_dps_payload.toml"),
            &Some(PathBuf::from("testfiles/dps-payload.json")),
        )
        .unwrap();
        assert_eq!(0, result.len());
    }

    #[test]
    fn identity_config_leaf_empty() {
        lazy_static::initialize(&LOG);
        assert_ne!(
            None,
            validate_identity(
                IdentityType::Leaf,
                &PathBuf::from("testfiles/identity_config_empty.toml"),
                &None,
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
            &PathBuf::from("testfiles/identity_config_minimal.toml"),
            &None,
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
            &PathBuf::from("testfiles/identity_config_dps.toml"),
            &None,
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
            &PathBuf::from("testfiles/identity_config_manual.toml"),
            &None,
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
            &PathBuf::from("testfiles/identity_config_manual_sas.toml"),
            &None,
        )
        .unwrap();
        assert_eq!(0, result.len());
    }

    #[test]
    fn identity_config_leaf_manual_tpm() {
        lazy_static::initialize(&LOG);
        let result = validate_identity(
            IdentityType::Leaf,
            &PathBuf::from("testfiles/identity_config_manual_tpm.toml"),
            &None,
        )
        .unwrap();
        assert_eq!(1, result.len());
        assert_ne!(None, result[0].find("authentication method should be sas"));
    }

    #[test]
    fn identity_config_gateway_empty() {
        lazy_static::initialize(&LOG);
        assert_ne!(
            None,
            validate_identity(
                IdentityType::Gateway,
                &PathBuf::from("testfiles/identity_config_empty.toml"),
                &None,
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
            &PathBuf::from("testfiles/identity_config_minimal.toml"),
            &None,
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
            &PathBuf::from("testfiles/identity_config_dps.toml"),
            &None,
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
            &PathBuf::from("testfiles/identity_config_manual.toml"),
            &None,
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
            &PathBuf::from("testfiles/identity_config_dps_tpm.toml"),
            &None,
        )
        .unwrap();
        assert_eq!(0, result.len());
    }

    #[test]
    fn identity_config_gateway_dps_sas() {
        lazy_static::initialize(&LOG);
        let result = validate_identity(
            IdentityType::Gateway,
            &PathBuf::from("testfiles/identity_config_dps_sas.toml"),
            &None,
        )
        .unwrap();
        assert_eq!(1, result.len());
        assert_ne!(
            None,
            result[0].find("attestation method should be tpm, x509 or symmetric_key")
        );
    }
}
