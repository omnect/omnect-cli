#[macro_use]
pub mod auth;
pub mod cli;
pub mod config;
pub mod device_update;
pub mod docker;
pub mod file;
pub mod image;
pub mod ssh;
mod validators;
use anyhow::{Context, Result};
use cli::{
    Command,
    Docker::Inject,
    File::{CopyFromImage, CopyToImage},
    IdentityConfig::{
        SetConfig, SetDeviceCertificate, SetDeviceCertificateNoEst, SetEdgeCaCertificate,
        SetIotLeafSasConfig, SetIotedgeGatewayConfig,
    },
    ImageOutput,
    IotHubDeviceUpdate::{self, SetDeviceConfig as IotHubDeviceUpdateSet},
    SshConfig::{SetCertificate, SetConnection},
};
use file::{
    bmap::{self, Bmap},
    compression::{self, Compression},
    functions::FileCopyToParams,
    output::{self, Staged, check_conflicts, output_image},
    sparse,
};
use log::info;
use std::{
    fs,
    path::{Path, PathBuf},
};
use tempfile::TempDir;

fn run_image_command<F>(image_file: PathBuf, output: ImageOutput, command: F) -> Result<()>
where
    F: FnOnce(&PathBuf) -> Result<()>,
{
    let ImageOutput {
        output_bmap,
        input_bmap,
        no_input_bmap,
        output_image: explicit_output,
        compress_image: target_compression,
    } = output;

    ensure_image_exists(&image_file)?;

    let input_bmap = if output_bmap && !no_input_bmap {
        bmap::find_input_bmap(&image_file, input_bmap.as_deref())?
            .map(|path| {
                Bmap::read(&path)
                    .with_context(|| input_bmap_error(&path))
                    .map(|bmap| (path, bmap))
            })
            .transpose()?
    } else {
        None
    };
    let source_compression = Compression::from_file(&image_file)?;
    let dest_image_file = output_image(
        &image_file,
        source_compression.as_ref(),
        target_compression.as_ref(),
        explicit_output.as_deref(),
    )?;
    check_conflicts(
        &image_file,
        &dest_image_file,
        explicit_output.is_some(),
        output_bmap,
        input_bmap.as_ref().map(|(path, _)| path.as_path()),
    )?;

    let verify_input_bmap = |image: &Path| -> Result<()> {
        let Some((path, bmap)) = &input_bmap else {
            return Ok(());
        };
        bmap::verify_checksums(image, bmap).with_context(|| input_bmap_error(path))
    };

    // a wrong bmap of an uncompressed image fails before the copy
    if source_compression.is_none() {
        verify_input_bmap(&image_file)?;
    }
    let tmp_dir = tempfile::tempdir().context("cannot create a temp dir")?;
    let tmp_image_file = temp_image(&image_file, source_compression.as_ref(), tmp_dir.path())?;
    if source_compression.is_some() {
        verify_input_bmap(&tmp_image_file)?;
    }

    // run command
    command(&tmp_image_file)?;

    let image = Staged::write(&dest_image_file, |dest| match &target_compression {
        Some(c) => compression::compress(&tmp_image_file, c, dest),
        None => sparse::copy_file(&tmp_image_file, dest),
    })?;
    let bmap = output_bmap
        .then(|| {
            match &input_bmap {
                Some((path, _)) => info!("the bmap maps the ranges of {}", path.display()),
                None => info!("no input bmap, so the bmap maps the whole image"),
            }
            Staged::write(&bmap::bmap_path(&dest_image_file), |dest| {
                bmap::write_bmap(
                    &tmp_image_file,
                    input_bmap.as_ref().map(|(_, bmap)| bmap),
                    dest,
                )
            })
        })
        .transpose()?;

    output::commit(image, bmap)
}

fn input_bmap_error(path: &Path) -> String {
    format!(
        "cannot use input bmap {}; pass the right one with --input-bmap, or map the whole \
         image with --no-input-bmap",
        path.display()
    )
}

/// Runs `command` on the image, or on a decompressed temp copy of a compressed image.
fn run_read_only_image_command<F>(image_file: PathBuf, command: F) -> Result<()>
where
    F: FnOnce(&PathBuf) -> Result<()>,
{
    ensure_image_exists(&image_file)?;

    match Compression::from_file(&image_file)? {
        Some(c) => {
            let tmp_dir = tempfile::tempdir().context("cannot create a temp dir")?;
            command(&compression::decompress(&image_file, &c, tmp_dir.path())?)
        }
        None => command(&image_file),
    }
}

fn ensure_image_exists(image_file: &Path) -> Result<()> {
    anyhow::ensure!(
        image_file.try_exists().is_ok_and(|exists| exists),
        "image doesn't exist {}",
        image_file.display()
    );
    Ok(())
}

/// Copies `image_file` into `tmp_dir`, decompressed if it is compressed, and returns the copy.
fn temp_image(
    image_file: &Path,
    source_compression: Option<&Compression>,
    tmp_dir: &Path,
) -> Result<PathBuf> {
    match source_compression {
        Some(c) => compression::decompress(image_file, c, tmp_dir),
        None => {
            let tmp_image_file = tmp_dir.join(
                image_file
                    .file_name()
                    .context("cannot get image file name")?,
            );
            let tmp_image = fs::File::create(&tmp_image_file)
                .context(format!("cannot create {}", tmp_image_file.display()))?;
            sparse::copy_file(image_file, &tmp_image)?;
            Ok(tmp_image_file)
        }
    }
}

/// A new device certificate and key in a temp dir, so parallel runs do not share them and no
/// private key is left behind.
struct CertInfo {
    _dir: TempDir,
    cert_path: PathBuf,
    key_path: PathBuf,
}

struct CertificateOptions<'a> {
    intermediate_full_chain_cert: &'a Path,
    intermediate_key: &'a Path,
    target_cert: &'a str,
    target_key: &'a str,
    subject: &'a str,
    validity_days: u32,
}

fn create_image_cert(cert_opts: CertificateOptions) -> Result<CertInfo> {
    let intermediate_full_chain_cert_str =
        std::fs::read_to_string(cert_opts.intermediate_full_chain_cert)
            .context("create_and_set_image_cert: couldn't read intermediate fullchain cert")?;
    let intermediate_key_str = std::fs::read_to_string(cert_opts.intermediate_key)
        .context("create_and_set_image_cert: couldn't read intermediate key")?;
    let crypto = omnect_crypto::Crypto::new(
        intermediate_key_str.as_bytes(),
        intermediate_full_chain_cert_str.as_bytes(),
    )?;
    let (cert_pem, key_pem) = crypto
        .create_cert_and_key(cert_opts.subject, cert_opts.validity_days)
        .context("create_and_set_image_cert: couldn't create device cert and key")?;

    let dir = tempfile::tempdir().context("create_and_set_image_cert: cannot create a temp dir")?;
    let cert_path = dir.path().join(cert_opts.target_cert);
    let key_path = dir.path().join(cert_opts.target_key);

    fs::write(&cert_path, cert_pem).context("create_and_set_image_cert: write device_cert_path")?;
    fs::write(&key_path, key_pem).context("create_and_set_image_cert: write device_key_path")?;

    Ok(CertInfo {
        _dir: dir,
        cert_path,
        key_path,
    })
}

pub fn run() -> Result<()> {
    match cli::from_args() {
        Command::Docker(Inject {
            docker_image,
            image,
            partition,
            dest,
            output,
        }) => run_image_command(image, output, |img| {
            anyhow::ensure!(
                dest.to_string_lossy().ends_with(".tar.gz"),
                format!(
                    "invalid destination file path \"{}\". Must end in \".tar.gz\".",
                    dest.to_string_lossy(),
                ),
            );

            let arch = image::image_arch(img)?;

            let docker_path = docker::pull_image(&docker_image, arch)?;

            let result = file::copy_to_image(
                &[FileCopyToParams::new(
                    &docker_path,
                    partition.clone(),
                    &dest,
                )],
                img,
            );
            std::fs::remove_file(docker_path)?;

            if result.is_ok() {
                println!(
                    "Stored {} to {}:{}",
                    docker_image,
                    partition,
                    dest.to_string_lossy(),
                );
            }

            result
        })?,
        Command::Identity(SetConfig {
            config,
            image,
            payload,
            output,
        }) => run_image_command(image, output, |img| {
            file::set_identity_config(&config, img, payload.as_deref())
        })?,
        Command::Identity(SetDeviceCertificate {
            intermediate_full_chain_cert,
            intermediate_key,
            image,
            device_id,
            days,
            output,
        }) => {
            let cert_info = create_image_cert(CertificateOptions {
                intermediate_full_chain_cert: &intermediate_full_chain_cert,
                intermediate_key: &intermediate_key,
                target_cert: "device_cert_path.pem",
                target_key: "device_key_path.key.pem",
                subject: &device_id,
                validity_days: days,
            })
            .context("set_device_certificate: could not create certificate")?;

            run_image_command(image, output, |img| {
                file::set_device_cert(
                    Some(&intermediate_full_chain_cert),
                    &cert_info.cert_path,
                    &cert_info.key_path,
                    img,
                )
            })?
        }
        Command::Identity(SetEdgeCaCertificate {
            intermediate_full_chain_cert,
            intermediate_key,
            image,
            device_id,
            days,
            output,
        }) => {
            let cert_info = create_image_cert(CertificateOptions {
                intermediate_full_chain_cert: &intermediate_full_chain_cert,
                intermediate_key: &intermediate_key,
                target_cert: "edge_ca_cert_path.pem",
                target_key: "edge_ca_key_path.key.pem",
                subject: &device_id,
                validity_days: days,
            })
            .context("set_edge_ca_certificate: could not create certificate")?;

            run_image_command(image, output, |img| {
                file::set_edge_ca_cert(
                    Some(&intermediate_full_chain_cert),
                    &cert_info.cert_path,
                    &cert_info.key_path,
                    img,
                )
            })?
        }
        Command::Identity(SetDeviceCertificateNoEst {
            device_cert: device_cert_pem,
            device_key: device_key_pem,
            image,
            output,
        }) => run_image_command(image, output, |img| {
            file::set_device_cert(None, &device_cert_pem, &device_key_pem, img)
        })?,
        Command::Identity(SetIotedgeGatewayConfig {
            config,
            image,
            root_ca,
            device_identity,
            device_identity_key,
            output,
        }) => run_image_command(image, output, |img: &PathBuf| {
            file::set_iotedge_gateway_config(
                &config,
                img,
                &root_ca,
                &device_identity,
                &device_identity_key,
            )
        })?,
        Command::Identity(SetIotLeafSasConfig {
            config,
            image,
            root_ca,
            output,
        }) => run_image_command(image, output, |img: &PathBuf| {
            file::set_iot_leaf_sas_config(&config, img, &root_ca)
        })?,
        Command::Ssh(SetCertificate {
            image,
            root_ca,
            output,
        }) => run_image_command(image, output, |img: &PathBuf| {
            file::set_ssh_tunnel_certificate(img, &root_ca)
        })?,
        Command::IotHubDeviceUpdate(IotHubDeviceUpdateSet {
            iot_hub_device_update_config,
            image,
            output,
        }) => run_image_command(image, output, |img: &PathBuf| {
            file::set_iot_hub_device_update_config(&iot_hub_device_update_config, img)
        })?,
        Command::IotHubDeviceUpdate(IotHubDeviceUpdate::ImportUpdate {
            import_manifest: import_manifest_path,
            storage_container_name,
            tenant_id,
            client_id,
            client_secret,
            instance_id,
            device_update_endpoint_url,
            blob_storage_account,
            blob_storage_key,
        }) => device_update::import_update(
            &import_manifest_path,
            &storage_container_name,
            &tenant_id,
            &client_id,
            &client_secret,
            &instance_id,
            &device_update_endpoint_url,
            &blob_storage_account,
            &blob_storage_key,
        )?,
        Command::IotHubDeviceUpdate(IotHubDeviceUpdate::RemoveUpdate {
            tenant_id,
            client_id,
            client_secret,
            instance_id,
            device_update_endpoint_url,
            provider,
            distro_name,
            version,
        }) => device_update::remove_update(
            &tenant_id,
            &client_id,
            &client_secret,
            &instance_id,
            &device_update_endpoint_url,
            &provider,
            &distro_name,
            &version,
        )?,
        Command::IotHubDeviceUpdate(IotHubDeviceUpdate::CreateImportManifest {
            image,
            script,
            manufacturer,
            model,
            compatibilityid,
            provider,
            consent_handler,
            swupdate_handler,
            distro_name,
            version,
        }) => device_update::create_import_manifest(
            &image,
            &script,
            &manufacturer,
            &model,
            &compatibilityid,
            &provider,
            &consent_handler,
            &swupdate_handler,
            &distro_name,
            &version,
        )?,
        Command::Ssh(SetConnection {
            device,
            username,
            dir,
            priv_key_path,
            config_path,
            env,
        }) => {
            #[tokio::main]
            async fn create_ssh_tunnel(
                device: &str,
                username: &str,
                dir: Option<PathBuf>,
                priv_key_path: Option<PathBuf>,
                config_path: Option<PathBuf>,
                env_config: config::BackendConfig,
            ) -> Result<()> {
                let access_token = crate::auth::authorize(env_config.auth)
                    .await
                    .context("create ssh tunnel")?;

                let config = ssh::Config::new(env_config.backend, dir, priv_key_path, config_path)?;

                ssh::ssh_create_tunnel(device, username, config, access_token).await
            }

            let env_conf: config::BackendConfig = if let Some(env_path) = env {
                let config_file = std::fs::read_to_string(env_path)?;

                toml::from_str(&config_file)?
            } else {
                config::BackendConfig {
                    backend: url::Url::parse("https://cp.omnect.conplement.cloud")?,
                    auth: config::AUTH_INFO_PROD.clone(),
                }
            };

            create_ssh_tunnel(
                &device,
                &username,
                dir,
                priv_key_path,
                config_path,
                env_conf,
            )?;
        }
        Command::File(CopyToImage {
            file_copy_params,
            image,
            output,
        }) => run_image_command(image, output, |img: &PathBuf| {
            file::copy_to_image(&file_copy_params, img)
        })?,
        Command::File(CopyFromImage {
            file_copy_params,
            image,
        }) => run_read_only_image_command(image, |img: &PathBuf| {
            file::copy_from_image(&file_copy_params, img)
        })?,
    }

    Ok(())
}
