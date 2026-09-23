//! Pure candidate matching for dependency reuse, before and after resolution.
//!
//! Graph traversal and override selection stay with the caller. These helpers
//! only inspect metadata already present on the candidate or resolved manifest.

use deno_semver::VersionReq;

use super::semver::{matches, normalize_spec};
use crate::model::graph::PackageNode;
use crate::model::manifest::CoreVersionManifest;
use crate::spec::Protocol;

/// Check the requested spec against an existing package before resolution.
pub(crate) fn matches_spec(candidate: &PackageNode, spec: &str) -> bool {
    match Protocol::strip_prefix(spec) {
        // HTTP tarballs are identified by their source URL, not the
        // version declared in their package.json.
        Some((Protocol::Http, _)) => candidate
            .manifest
            .dist()
            .is_some_and(|dist| dist.tarball.as_deref() == Some(spec)),
        _ => matches(spec, &candidate.version),
    }
}

/// Check a selected override target without resolving it or selecting rules.
pub(crate) fn matches_override_target(
    candidate: &PackageNode,
    name: &str,
    spec: &str,
    target: &str,
) -> bool {
    match Protocol::strip_prefix(target) {
        Some((Protocol::Http, _)) => matches_spec(candidate, target),
        None | Some((Protocol::NpmAlias, _)) => {
            let (target_name, target_range) = normalize_spec(name, target);
            candidate.manifest.name() == target_name
                && VersionReq::parse_from_npm(&target_range).is_ok_and(|req| {
                    // A dist-tag needs registry resolution;
                    // the candidate's version cannot identify it.
                    req.tag().is_none() && matches(&target_range, &candidate.version)
                })
        }
        _ => target == spec,
    }
}

/// Compare the final manifest after resolution, including any override.
/// The original request and override rules must not be applied again here.
pub(crate) fn matches_resolved_manifest(
    candidate: &PackageNode,
    manifest: &CoreVersionManifest,
) -> bool {
    candidate.manifest.name() == manifest.name
        && candidate.version == manifest.version
        && candidate
            .manifest
            .dist()
            .is_some_and(|dist| dist.tarball == manifest.dist.tarball)
}
