use crate::Result;

#[derive(PartialEq, Debug, Default)]
pub struct GitUrlComponents {
    pub domain: String,
    pub username: String,
    pub repo: String,
    pub suffix: String,
}

pub fn parse_git_url_https(http_url: &str) -> Option<GitUrlComponents> {
    let mut split: Vec<_> = http_url.split('/').collect();

    while let Some(&"") = split.last() {
        split.pop();
    }
    if split.len() != 5 {
        return None;
    }
    if split[0] != "https:" && split[0] != "http:" {
        return None;
    }
    let domain = split[2];
    let username = split[3];
    let repo = split[4];
    let suffix = match domain {
        "git.sr.ht" => "",
        _ => {
            if repo.ends_with(".git") {
                ""
            } else {
                ".git"
            }
        }
    };

    Some(GitUrlComponents {
        domain: domain.to_string(),
        username: username.to_string(),
        repo: repo.to_string(),
        suffix: suffix.to_string(),
    })
}

pub fn fetch_and_checkout_git_repo(repo: &git2::Repository) -> Result<()> {
    repo.find_remote("origin")?.fetch(&["master"], None, None)?;
    repo.set_head("FETCH_HEAD")?;
    let mut opts = git2::build::CheckoutBuilder::new();
    opts.force();
    repo.checkout_head(Some(&mut opts))?;
    Ok(())
}

#[test]
fn parse_git_url_https_test() {
    assert_eq!(
        parse_git_url_https("https://github.com/dpc/trust"),
        Some(GitUrlComponents {
            domain: "github.com".to_string(),
            username: "dpc".to_string(),
            repo: "trust".to_string(),
            suffix: ".git".to_string()
        })
    );
    assert_eq!(
        parse_git_url_https("https://gitlab.com/hackeraudit/web.git"),
        Some(GitUrlComponents {
            domain: "gitlab.com".to_string(),
            username: "hackeraudit".to_string(),
            repo: "web.git".to_string(),
            suffix: "".to_string()
        })
    );
    assert_eq!(
        parse_git_url_https("https://gitlab.com/hackeraudit/web.git/"),
        Some(GitUrlComponents {
            domain: "gitlab.com".to_string(),
            username: "hackeraudit".to_string(),
            repo: "web.git".to_string(),
            suffix: "".to_string()
        })
    );
    assert_eq!(
        parse_git_url_https("https://gitlab.com/hackeraudit/web.git/////////"),
        Some(GitUrlComponents {
            domain: "gitlab.com".to_string(),
            username: "hackeraudit".to_string(),
            repo: "web.git".to_string(),
            suffix: "".to_string()
        })
    );
}

pub fn https_to_git_url(http_url: &str) -> Option<String> {
    parse_git_url_https(http_url).map(|components| {
        format!(
            "git@{}:{}/{}{}",
            components.domain, components.username, components.repo, components.suffix
        )
    })
}

#[test]
fn https_to_git_url_test() {
    assert_eq!(
        https_to_git_url("https://github.com/dpc/trust"),
        Some("git@github.com:dpc/trust.git".into())
    );
    assert_eq!(
        https_to_git_url("https://gitlab.com/hackeraudit/web.git"),
        Some("git@gitlab.com:hackeraudit/web.git".into())
    );
    assert_eq!(
        https_to_git_url("https://gitlab.com/hackeraudit/web.git/"),
        Some("git@gitlab.com:hackeraudit/web.git".into())
    );
    assert_eq!(
        https_to_git_url("https://gitlab.com/hackeraudit/web.git/////////"),
        Some("git@gitlab.com:hackeraudit/web.git".into())
    );
    assert_eq!(
        https_to_git_url("https://git.sr.ht/~ireas/crev-proofs"),
        Some("git@git.sr.ht:~ireas/crev-proofs".into())
    );
}
