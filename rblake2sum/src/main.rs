mod opts;

use common_failures::prelude::*;
use structopt::StructOpt;

use std::collections::HashSet;

fn main() -> Result<()> {
    let opts = opts::Opts::from_args();

    for path in opts.paths {
        let digest = crev_recursive_digest::get_recursive_digest_for_dir::<blake2::Blake2b, _>(
            &path,
            &HashSet::new(),
        )?;
        println!(
            "{} {}",
            if opts.base64 {
                base64::encode_config(&digest, base64::URL_SAFE)
            } else {
                hex::encode(digest)
            },
            path.display()
        );
    }
    Ok(())
}
