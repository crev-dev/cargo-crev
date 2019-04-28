mod opts;

use common_failures::prelude::*;
use structopt::StructOpt;

use std::collections::HashSet;

fn main() -> Result<()> {
    let opts = opts::Opts::from_args();

    for path in opts.paths {
        let digest = crev_recursive_digest::get_recursive_digest_for_dir::<
            crev_common::Blake2b256,
            _,
        >(&path, &HashSet::new())?;
        println!(
            "{} {}",
            if opts.base64 {
                crev_common::base64_encode(&digest)
            } else {
                hex::encode(digest)
            },
            path.display()
        );
    }
    Ok(())
}
