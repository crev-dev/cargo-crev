use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt, Clone)]
#[structopt(
    name = "rblake2sum",
    about = "Calculate recursive blake2 digest for path or directory"
)]
pub struct Opts {
    #[structopt(long = "base64")]
    pub base64: bool,
    #[structopt(parse(from_os_str))]
    pub paths: Vec<PathBuf>,
}
