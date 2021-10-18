use filemagic::Magic;
use std::io::{Error, ErrorKind};
use std::path::PathBuf;
use std::fs::remove_file;
use tokio::fs::File;

#[tokio::main]
async fn decompress_xz(image_file_name: &PathBuf) -> Result<PathBuf, tokio::io::Error> {
    let mut new_image_file = image_file_name.to_path_buf();
    new_image_file.set_extension("unxz.tmp");
    let destination = File::create(new_image_file.clone()).await?;
    let mut source = File::open(image_file_name).await?;
    let mut decompressor = async_compression::tokio::write::XzDecoder::new(destination);
    tokio::io::copy(&mut source, &mut decompressor).await?;
    return Ok(new_image_file);
}

#[tokio::main]
async fn compress_xz(uncompressed_file_name: &PathBuf, compressed_file_name: &PathBuf) -> Result<(), tokio::io::Error> {
    let destination = File::create(compressed_file_name.clone()).await?;
    let mut source = File::open(uncompressed_file_name).await?;
    let mut decompressor = async_compression::tokio::write::XzEncoder::new(destination);
    tokio::io::copy(&mut source, &mut decompressor).await?;
    return Ok(());
}

#[tokio::main]
async fn decompress_bzip2(image_file_name: &PathBuf) -> Result<PathBuf, tokio::io::Error> {
    let mut new_image_file = image_file_name.to_path_buf();
    new_image_file.set_extension("bzip.tmp");
    let destination = File::create(new_image_file.clone()).await?;
    let mut source = File::open(image_file_name).await?;
    let mut decompressor = async_compression::tokio::write::BzDecoder::new(destination);
    tokio::io::copy(&mut source, &mut decompressor).await?;
    return Ok(new_image_file);
}

#[tokio::main]
async fn compress_bzip(uncompressed_file_name: &PathBuf, compressed_file_name: &PathBuf) -> Result<(), tokio::io::Error> {
    let destination = File::create(compressed_file_name.clone()).await?;
    let mut source = File::open(uncompressed_file_name).await?;
    let mut decompressor = async_compression::tokio::write::BzEncoder::new(destination);
    tokio::io::copy(&mut source, &mut decompressor).await?;
    return Ok(());
}

#[tokio::main]
async fn decompress_gzip(image_file_name: &PathBuf) -> Result<PathBuf, tokio::io::Error> {
    let mut new_image_file = image_file_name.to_path_buf();
    new_image_file.set_extension("gz.tmp");
    let destination = File::create(new_image_file.clone()).await?;
    let mut source = File::open(image_file_name).await?;
    let mut decompressor = async_compression::tokio::write::GzipDecoder::new(destination);
    tokio::io::copy(&mut source, &mut decompressor).await?;
    return Ok(new_image_file);
}

#[tokio::main]
async fn compress_gzip(uncompressed_file_name: &PathBuf, compressed_file_name: &PathBuf) -> Result<(), tokio::io::Error> {
    let destination = File::create(compressed_file_name.clone()).await?;
    let mut source = File::open(uncompressed_file_name).await?;
    let mut decompressor = async_compression::tokio::write::GzipEncoder::new(destination);
    tokio::io::copy(&mut source, &mut decompressor).await?;
    return Ok(());
}

pub fn validate_image(
    image_file_name: &PathBuf,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    println!("Detecting magic for {}",image_file_name.to_string_lossy());
    let detector = Magic::open(Default::default());
    let detector = match detector {
        Err(e) => {
            return Err(Box::new(Error::new(
                ErrorKind::Other,
                format!("libmagic open failed with error {}", e.to_string()),
            )));
        }
        Ok(d) => d,
    };
    match detector.load::<String>(&[]) {
        Err(e) => {
            return Err(Box::new(Error::new(
                ErrorKind::Other,
                format!("libmagic load failed with error {}", e.to_string()),
            )));
        }
        _ => {}
    }
    let magic = detector.file(&image_file_name)?;
    println!("magic= {}", magic);
    if magic.find("XZ compressed data") != None {
        println!("xz compressed file found, decompressing...");
        let new_image_file = decompress_xz(image_file_name)?;
        println!("Decompressed to {}", new_image_file.to_string_lossy());
        return Ok(new_image_file);
    } else if magic.find("bzip2 compressed data") != None {
        println!("bzip2 compressed file found, decompressing...");
        let new_image_file = decompress_bzip2(image_file_name)?;
        println!("Decompressed to {}", new_image_file.to_string_lossy());
        return Ok(new_image_file);
    } else if magic.find("gzip compressed data") != None {
        println!("gzip compressed file found, decompressing...");
        let new_image_file = decompress_gzip(image_file_name)?;
        println!("Decompressed to {}", new_image_file.to_string_lossy());
        return Ok(new_image_file);
    }
    Ok(image_file_name.to_path_buf())
}

pub fn postprocess_image(
    original_file_name: &PathBuf,
    unpacked_file_name: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    if original_file_name != unpacked_file_name {
        match unpacked_file_name.to_string_lossy().find("unxz.tmp") {
            Some(_) => {
                println!("xz compressing {} to {} ...", unpacked_file_name.to_string_lossy(), original_file_name.to_string_lossy());
                match compress_xz(unpacked_file_name, original_file_name) {
                    Ok(_e) => { 
                        match remove_file(unpacked_file_name) {
                            Err(e) =>{
                                return Err(Box::new(Error::new(
                                    ErrorKind::Other,
                                    format!("Deleting temporary file failed with error {}", e.to_string()),
                                )));
                            }
                            Ok(_) => { return Ok(()); }
                        }
                    }
                    Err(e) =>{
                        return Err(Box::new(Error::new(
                            ErrorKind::Other,
                            format!("Recompressing failed with error {}", e.to_string()),
                        )));
                    }
                }
            }
            None => {}
        }
        match unpacked_file_name.to_string_lossy().find("bzip.tmp") {
            Some(_) => {
                println!("bzip2 compressing {} to {} ...", unpacked_file_name.to_string_lossy(), original_file_name.to_string_lossy());
                match compress_bzip(unpacked_file_name, original_file_name) {
                    Ok(_e) => {
                        match remove_file(unpacked_file_name) {
                            Err(e) =>{
                                return Err(Box::new(Error::new(
                                    ErrorKind::Other,
                                    format!("Deleting temporary file failed with error {}", e.to_string()),
                                )));
                            }
                            Ok(_) => { return Ok(()); }
                        }
                    }
                    Err(e) =>{
                        return Err(Box::new(Error::new(
                            ErrorKind::Other,
                            format!("Recompressing failed with error {}", e.to_string()),
                        )));
                    }
                }
            }
            None => {}
        }
        match unpacked_file_name.to_string_lossy().find("gz.tmp") {
            Some(_) => {
                println!("gzip compressing {} to {} ...", unpacked_file_name.to_string_lossy(), original_file_name.to_string_lossy());
                match compress_gzip(unpacked_file_name, original_file_name) {
                    Ok(_e) => {
                        match remove_file(unpacked_file_name) {
                            Err(e) =>{
                                return Err(Box::new(Error::new(
                                    ErrorKind::Other,
                                    format!("Deleting temporary file failed with error {}", e.to_string()),
                                )));
                            }
                            Ok(_) => { return Ok(()); }
                        }
                    }
                    Err(e) =>{
                        return Err(Box::new(Error::new(
                            ErrorKind::Other,
                            format!("Recompressing failed with error {}", e.to_string()),
                        )));
                    }
                }
            }
            None => {}
        }

    }
    Ok(())
}

