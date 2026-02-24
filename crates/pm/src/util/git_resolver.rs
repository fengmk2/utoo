//! Git package resolver.
//!
//! Thin wrappers around ruborist's git clone functionality,
//! injecting the PM cache directory.

use anyhow::Result;

pub use utoo_ruborist::git::GitCloneResult;

use super::cache::get_cache_dir;

/// Resolve a git package spec by cloning the repo, checking out the ref,
/// reading package.json, and caching the result.
///
/// # Arguments
/// * `url` - Git URL, e.g. `git+https://github.com/user/repo.git`
/// * `commit_ish` - Optional branch, tag, or commit to check out
pub async fn resolve_git_spec(url: &str, commit_ish: Option<&str>) -> Result<GitCloneResult> {
    let cache_dir = get_cache_dir();
    utoo_ruborist::git::clone_repo(&cache_dir, url, commit_ish).await
}

/// Convert a `github:owner/repo` shorthand to a git+ URL and resolve.
pub async fn resolve_github_spec(
    owner: &str,
    repo: &str,
    commit_ish: Option<&str>,
) -> Result<GitCloneResult> {
    let url = format!("git+https://github.com/{}/{}.git", owner, repo);
    resolve_git_spec(&url, commit_ish).await
}
