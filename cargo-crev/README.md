# `cargo-crev` - Crev frontend for Rust/cargo/crates.io

`cargo-crev` builds on top of Crev to allow people to review
cargo packages, and share 


### Getting started

`cargo-crev` is work in progress. Please report any problems, and we're
always happy to hear your feedback.


```
cargo install --path . # to install
cd <your-project>
cargo crev id gen # generate your id
cargo crev verify # verify your depedencies
cargo crev review <crate> # record your crate review
cargo crev db git status # check git status of your proof database
cargo crev db git -- ci -a # commit everything
cargo crev db git push # push it to your github repository
cargo crev trust <id> # trust someone with a given CrevId
cargo crev db fetch # fetch updates from all people you trust
cargo crev verify # verify again
cargo crev help # see what other things you can do
```

Join [crev gitter channel](https://gitter.im/dpc/crev) to share your ID with us!
