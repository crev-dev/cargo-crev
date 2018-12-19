//! Bunch of Github-specific stuff

use common_failures::prelude::*;
use std::io::Read;

#[derive(Eq, PartialEq)]
pub(crate) enum CreatedOrExisted {
    Created,
    Existed,
}

impl CreatedOrExisted {
    pub fn was_created(&self) -> bool {
        *self == CreatedOrExisted::Created
    }
}

#[derive(Serialize)]
struct UserRepoCreateRequest {
    name: String,
    description: String,
}

pub(crate) fn create_remote_github_repository(
    username: &str,
    password: &str,
    repository_name: &str,
) -> Result<CreatedOrExisted> {
    let mut handle = curl::easy::Easy::new();
    handle.url("https://api.github.com/user/repos")?;
    handle.post(true)?;

    handle.useragent("CREV")?;
    handle.username(username)?;
    handle.password(password)?;

    let request = UserRepoCreateRequest {
        name: repository_name.to_string(),
        description: "Crev Proof Repository".into(),
    };

    let post_data = serde_json::to_string(&request)?;
    let mut post_data = post_data.as_bytes();
    handle.post_field_size(post_data.len() as u64)?;
    let mut response_bytes = Vec::new();

    {
        let mut transfer = handle.transfer();

        transfer.read_function(|r| Ok(post_data.read(r).unwrap_or(0)))?;

        transfer.write_function(|w| {
            response_bytes.extend_from_slice(w);
            Ok(w.len())
        })?;

        transfer.perform()?;
    }

    let response_code = handle.response_code()?;
    if response_code == 201 {
        Ok(CreatedOrExisted::Created)
    } else {
        bail!("Error code: {}", response_code);
    }
}
