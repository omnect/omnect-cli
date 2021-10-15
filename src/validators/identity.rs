use serde::Deserialize;
use std::collections::BTreeMap;
use std::io::{Error, ErrorKind};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Attestation {
    method: String,
    registration_id: Option<String>,
    trust_bundle_cert: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
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
    device_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EdgeCA {
    cert: String,
    pk: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
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
const WARN_STANDALONE_MISSING_PROVISIONING: &'static str =
    "Warning: for a standalone device, a provisioning section should be specified.";
const WARN_STANDALONE_MISSING_DPS_PARAMS: &'static str =
    "Warning: for provisioning source dps, global_endpoint and id_scope are required.";
const WARN_STANDALONE_DPS_EXPECTED: &'static str = "Warning: provisioning source should be dps";
const WARN_STANDALONE_MISSING_ATTESTATION: &'static str =
    "Warning: for a standalone device, an attestation section should be present in the provisioning section.";
const WARN_STANDALONE_TPM_EXPECTED: &'static str =
    "Warning: for a standalone device, currently only tpm attestation is supported.";
const WARN_LEAF_GATEWAY_HOSTNAME_EXPECTED: &'static str =
    "Warning: for a leaf device, a local_gateway_hostname entry should be specified.";
const WARN_LEAF_MISSING_PROVISIONING: &'static str =
    "Warning: for a leaf device, a provisioning section should be specified.";
const WARN_LEAF_MISSING_MANUAL_PARAMS: &'static str =
    "Warning: for provisioning source manual, iothub_hostname and device_id are required.";
const WARN_LEAF_MANUAL_EXPECTED: &'static str =
    "Warning: for a leaf device, provisioning source should be manual";
const WARN_LEAF_MISSING_AUTHENTICATION: &'static str =
    "Warning: for a leaf device, an authentication section should be present in the provisioning section.";
const WARN_LEAF_SAS_EXPECTED: &'static str =
    "Warning: for a leaf device, currently only sas authentication is supported.";
const WARN_LEAF_MISSING_DEVICE_ID_PK: &'static str =
    "Warning: for sas authentication, a device_id_pk entry should be specified.";
const WARN_LEAF_VALUE_OR_URI_EXPECTED: &'static str =
    "Warning: for sas authentication, the device_id_pk map should contain either a 'value' or an 'uri' entry.";
const WARN_GATEWAY_MISSING_PROVISIONING: &'static str =
    "Warning: for a gateway device, a provisioning section should be specified.";
const WARN_GATEWAY_MISSING_DPS_PARAMS: &'static str =
    "Warning: for provisioning source dps, global_endpoint and id_scope are required.";
const WARN_GATEWAY_DPS_EXPECTED: &'static str =
    "Warning: for a gateway device, provisioning source should be dps";
const WARN_GATEWAY_MISSING_ATTESTATION: &'static str =
    "Warning: for a gateway device, an attestation section should be present in the provisioning section.";
const WARN_GATEWAY_TPM_EXPECTED: &'static str =
    "Warning: for a gateway device, currently only tpm attestation is supported.";
const WARN_GATEWAY_MISSING_EDGE_CA: &'static str =
    "Warning: for a gateway device, an edge_ca section should be specified.";

pub fn validate_identity(
    id_type: IdentityType,
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
    match id_type {
        IdentityType::Standalone => match body.provisioning {
            None => {
                out.push(WARN_STANDALONE_MISSING_PROVISIONING);
            }
            Some(p) => {
                match p.source.as_str() {
                    "dps" => {
                        if p.global_endpoint == None || p.id_scope == None {
                            out.push(WARN_STANDALONE_MISSING_DPS_PARAMS);
                        }
                    }
                    _ => {
                        out.push(WARN_STANDALONE_DPS_EXPECTED);
                    }
                }
                match p.attestation {
                    None => {
                        out.push(WARN_STANDALONE_MISSING_ATTESTATION);
                    }
                    Some(a) => {
                        if a.method != "tpm" {
                            out.push(WARN_STANDALONE_TPM_EXPECTED);
                        }
                    }
                }
            }
        },
        IdentityType::Leaf => {
            if body.local_gateway_hostname == None {
                out.push(WARN_LEAF_GATEWAY_HOSTNAME_EXPECTED);
            }
            match body.provisioning {
                None => {
                    out.push(WARN_LEAF_MISSING_PROVISIONING);
                }
                Some(p) => {
                    match p.source.as_str() {
                        "manual" => {
                            if p.iothub_hostname == None || p.device_id == None {
                                out.push(WARN_LEAF_MISSING_MANUAL_PARAMS);
                            }
                        }
                        _ => {
                            out.push(WARN_LEAF_MANUAL_EXPECTED);
                        }
                    }
                    match p.authentication {
                        None => {
                            out.push(WARN_LEAF_MISSING_AUTHENTICATION);
                        }
                        Some(a) => match a.method.as_str() {
                            "sas" => match a.device_id_pk {
                                None => {
                                    out.push(WARN_LEAF_MISSING_DEVICE_ID_PK);
                                }
                                Some(pk) => {
                                    if pk.get("value") == None && pk.get("uri") == None {
                                        out.push(WARN_LEAF_VALUE_OR_URI_EXPECTED);
                                    }
                                }
                            },
                            _ => {
                                out.push(WARN_LEAF_SAS_EXPECTED);
                            }
                        },
                    }
                }
            }
        }
        IdentityType::Gateway => {
            match body.provisioning {
                None => {
                    out.push(WARN_GATEWAY_MISSING_PROVISIONING);
                }
                Some(p) => {
                    match p.source.as_str() {
                        "dps" => {
                            if p.global_endpoint == None || p.id_scope == None {
                                out.push(WARN_GATEWAY_MISSING_DPS_PARAMS);
                            }
                        }
                        _ => {
                            out.push(WARN_GATEWAY_DPS_EXPECTED);
                        }
                    }
                    match p.attestation {
                        None => {
                            out.push(WARN_GATEWAY_MISSING_ATTESTATION);
                        }
                        Some(a) => {
                            if a.method != "tpm" {
                                out.push(WARN_GATEWAY_TPM_EXPECTED);
                            }
                        }
                    }
                }
            }
            match body.edge_ca {
                None => {
                    out.push(WARN_GATEWAY_MISSING_EDGE_CA);
                }
                Some(_e) => {}
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_config_standalone_empty() {
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
        let result = validate_identity(
            IdentityType::Standalone,
            &std::path::PathBuf::from("testfiles/identity_config_minimal.toml"),
        )
        .unwrap();
        assert_eq!(1, result.len());
        assert_ne!(None, result[0].find("provisioning section"));
        assert_ne!(None, result[0].find("for a standalone device"));
    }

    #[test]
    fn identity_config_standalone_dps() {
        let result = validate_identity(
            IdentityType::Standalone,
            &std::path::PathBuf::from("testfiles/identity_config_dps.toml"),
        )
        .unwrap();
        assert_eq!(2, result.len());
        assert_ne!(None, result[0].find("for provisioning source dps"));
        assert_ne!(None, result[0].find("global_endpoint and id_scope"));
        assert_ne!(None, result[1].find("attestation section"));
        assert_ne!(None, result[1].find("for a standalone device"));
    }

    #[test]
    fn identity_config_standalone_manual() {
        let result = validate_identity(
            IdentityType::Standalone,
            &std::path::PathBuf::from("testfiles/identity_config_manual.toml"),
        )
        .unwrap();
        assert_eq!(2, result.len());
        assert_ne!(None, result[0].find("should be dps"));
        assert_ne!(None, result[1].find("attestation section"));
        assert_ne!(None, result[1].find("for a standalone device"));
    }

    #[test]
    fn identity_config_standalone_dps_tpm() {
        let result = validate_identity(
            IdentityType::Standalone,
            &std::path::PathBuf::from("testfiles/identity_config_dps_tpm.toml"),
        )
        .unwrap();
        assert_eq!(0, result.len());
    }

    #[test]
    fn identity_config_standalone_dps_sas() {
        let result = validate_identity(
            IdentityType::Standalone,
            &std::path::PathBuf::from("testfiles/identity_config_dps_sas.toml"),
        )
        .unwrap();
        assert_eq!(1, result.len());
        assert_ne!(None, result[0].find("only tpm"));
        assert_ne!(None, result[0].find("for a standalone device"));
    }

    #[test]
    fn identity_config_leaf_empty() {
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
        let result = validate_identity(
            IdentityType::Leaf,
            &std::path::PathBuf::from("testfiles/identity_config_minimal.toml"),
        )
        .unwrap();
        assert_eq!(2, result.len());
        assert_ne!(None, result[0].find("for a leaf device"));
        assert_ne!(None, result[0].find("local_gateway_hostname"));
        assert_ne!(None, result[1].find("for a leaf device"));
        assert_ne!(None, result[1].find("provisioning section"));
    }

    #[test]
    fn identity_config_leaf_dps() {
        let result = validate_identity(
            IdentityType::Leaf,
            &std::path::PathBuf::from("testfiles/identity_config_dps.toml"),
        )
        .unwrap();
        assert_eq!(3, result.len());
        assert_ne!(None, result[0].find("for a leaf device"));
        assert_ne!(None, result[0].find("local_gateway_hostname"));
        assert_ne!(None, result[1].find("should be manual"));
        assert_ne!(None, result[1].find("for a leaf device"));
        assert_ne!(None, result[2].find("authentication section"));
        assert_ne!(None, result[2].find("for a leaf device"));
    }

    #[test]
    fn identity_config_leaf_manual() {
        let result = validate_identity(
            IdentityType::Leaf,
            &std::path::PathBuf::from("testfiles/identity_config_manual.toml"),
        )
        .unwrap();
        assert_eq!(3, result.len());
        assert_ne!(None, result[0].find("for a leaf device"));
        assert_ne!(None, result[0].find("local_gateway_hostname"));
        assert_ne!(None, result[1].find("iothub_hostname and device_id"));
        assert_ne!(None, result[1].find("for provisioning source manual"));
        assert_ne!(None, result[2].find("authentication section"));
        assert_ne!(None, result[2].find("for a leaf device"));
    }

    #[test]
    fn identity_config_leaf_manual_sas() {
        let result = validate_identity(
            IdentityType::Leaf,
            &std::path::PathBuf::from("testfiles/identity_config_manual_sas.toml"),
        )
        .unwrap();
        assert_eq!(1, result.len());
        assert_ne!(None, result[0].find("device_id_pk"));
        assert_ne!(None, result[0].find("for sas authentication"));
    }

    #[test]
    fn identity_config_leaf_manual_tpm() {
        let result = validate_identity(
            IdentityType::Leaf,
            &std::path::PathBuf::from("testfiles/identity_config_manual_tpm.toml"),
        )
        .unwrap();
        assert_eq!(1, result.len());
        assert_ne!(None, result[0].find("only sas authentication"));
        assert_ne!(None, result[0].find("for a leaf device"));
    }

    #[test]
    fn identity_config_leaf_no_warn() {
        let result = validate_identity(
            IdentityType::Leaf,
            &std::path::PathBuf::from("conf/config.toml.ics-iot-leaf.template"),
        )
        .unwrap();
        assert_eq!(0, result.len());
    }

    #[test]
    fn identity_config_gateway_empty() {
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
        let result = validate_identity(
            IdentityType::Gateway,
            &std::path::PathBuf::from("testfiles/identity_config_minimal.toml"),
        )
        .unwrap();
        assert_eq!(2, result.len());
        assert_ne!(None, result[0].find("for a gateway device"));
        assert_ne!(None, result[0].find("provisioning section"));
        assert_ne!(None, result[1].find("for a gateway device"));
        assert_ne!(None, result[1].find("edge_ca"));
    }

    #[test]
    fn identity_config_gateway_dps() {
        let result = validate_identity(
            IdentityType::Gateway,
            &std::path::PathBuf::from("testfiles/identity_config_dps.toml"),
        )
        .unwrap();
        assert_eq!(3, result.len());
        assert_ne!(None, result[0].find("for provisioning source dps"));
        assert_ne!(None, result[0].find("global_endpoint and id_scope"));
        assert_ne!(None, result[1].find("for a gateway device"));
        assert_ne!(None, result[1].find("attestation section"));
        assert_ne!(None, result[2].find("for a gateway device"));
        assert_ne!(None, result[2].find("edge_ca"));
    }

    #[test]
    fn identity_config_gateway_manual() {
        let result = validate_identity(
            IdentityType::Gateway,
            &std::path::PathBuf::from("testfiles/identity_config_manual.toml"),
        )
        .unwrap();
        assert_eq!(3, result.len());
        assert_ne!(None, result[0].find("should be dps"));
        assert_ne!(None, result[1].find("for a gateway device"));
        assert_ne!(None, result[1].find("attestation section"));
        assert_ne!(None, result[2].find("for a gateway device"));
        assert_ne!(None, result[2].find("edge_ca"));
    }

    #[test]
    fn identity_config_gateway_dps_tpm() {
        let result = validate_identity(
            IdentityType::Gateway,
            &std::path::PathBuf::from("testfiles/identity_config_dps_tpm.toml"),
        )
        .unwrap();
        assert_eq!(1, result.len());
        assert_ne!(None, result[0].find("for a gateway device"));
        assert_ne!(None, result[0].find("edge_ca"));
    }

    #[test]
    fn identity_config_gateway_dps_sas() {
        let result = validate_identity(
            IdentityType::Gateway,
            &std::path::PathBuf::from("testfiles/identity_config_dps_sas.toml"),
        )
        .unwrap();
        assert_eq!(2, result.len());
        assert_ne!(None, result[0].find("for a gateway device"));
        assert_ne!(None, result[0].find("only tpm"));
        assert_ne!(None, result[1].find("for a gateway device"));
        assert_ne!(None, result[1].find("edge_ca"));
    }

    #[test]
    fn identity_config_gateway_no_warn() {
        let result = validate_identity(
            IdentityType::Gateway,
            &std::path::PathBuf::from("conf/config.toml.ics-iotedge-gateway.template"),
        )
        .unwrap();
        assert_eq!(0, result.len());
    }
}
