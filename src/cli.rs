use structopt::StructOpt;

const ABOUT: &'static str = "- manage your ics-dm devices";
const COPYRIGHT: &'static str = "© 2021 conplement AG";

#[derive(StructOpt, Debug, PartialEq)]
#[structopt(about = ABOUT)]
#[structopt(after_help = COPYRIGHT)]
pub enum IdentityConfig {
    Set {
        #[structopt(short = "c", long = "config")]
        #[structopt(parse(from_os_str))]
        config: std::path::PathBuf,
        #[structopt(short = "i", long = "image")]
        #[structopt(parse(from_os_str))]
        image: std::path::PathBuf,
    },
    Info {
        #[structopt(short = "i", long = "image")]
        #[structopt(parse(from_os_str))]
        image: std::path::PathBuf,
    }
}

#[derive(StructOpt, Debug, PartialEq)]
#[structopt(about = ABOUT)]
#[structopt(after_help = COPYRIGHT)]
pub enum WifiConfig {
    Set {
        /// absolute path to config file
        #[structopt(short = "c", long = "config")]
        #[structopt(parse(from_os_str))]
        config: std::path::PathBuf,
         /// absolute path to uncompressed image file
        #[structopt(short = "i", long = "image")]
        #[structopt(parse(from_os_str))]
        image: std::path::PathBuf,
    },
    Info {
        #[structopt(short = "i", long = "image")]
        #[structopt(parse(from_os_str))]
        image: std::path::PathBuf,
    }
}


#[derive(StructOpt, Debug, PartialEq)]
#[structopt(about = ABOUT)]
#[structopt(after_help = COPYRIGHT)]
pub enum Command {
    Identity(IdentityConfig),
    DockerInfo,
    Wifi(WifiConfig),
}

pub fn from_args() -> Command { Command::from_args()}
