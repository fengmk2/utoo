use std::fs;
use std::io::Write;
use std::process::Command;

use flate2::Compression;
use flate2::write::GzEncoder;
use serde_json::{Value, json};
use tempfile::tempdir;

fn tarball(manifest: Value, source: &str) -> Vec<u8> {
    let mut archive = tar::Builder::new(Vec::new());
    for (path, body) in [
        ("package/package.json", manifest.to_string()),
        ("package/index.js", source.to_string()),
    ] {
        let mut header = tar::Header::new_gnu();
        header.set_size(body.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        archive
            .append_data(&mut header, path, body.as_bytes())
            .unwrap();
    }
    let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
    gzip.write_all(&archive.into_inner().unwrap()).unwrap();
    gzip.finish().unwrap()
}

fn check_shared_tarball(different_url: bool) {
    let mut server = mockito::Server::new();
    let shared_url = format!("{}/shared.tgz", server.url());
    let dependency_url = if different_url {
        format!("{}/other.tgz", server.url())
    } else {
        shared_url.clone()
    };
    let shared = tarball(
        json!({ "name": "shared", "version": "1.0.0", "main": "index.js" }),
        "module.exports = new Map();",
    );
    let _shared = server
        .mock("GET", "/shared.tgz")
        .with_body(&shared)
        .create();
    let _other = server.mock("GET", "/other.tgz").with_body(&shared).create();
    let _consumer = server
        .mock("GET", "/consumer.tgz")
        .with_body(tarball(
            json!({
                "name": "consumer", "version": "1.0.0", "main": "index.js",
                "dependencies": { "shared": dependency_url }
            }),
            "module.exports = require('shared');",
        ))
        .create();
    let project = tempdir().unwrap();
    let cache = tempdir().unwrap();
    fs::write(
        project.path().join("package.json"),
        json!({
            "name": "http-dedup", "version": "1.0.0", "private": true,
            "dependencies": {
                "shared": shared_url,
                "consumer": format!("{}/consumer.tgz", server.url())
            }
        })
        .to_string(),
    )
    .unwrap();

    // Check both fresh resolution and installation from the generated lockfile.
    for reinstall in [false, true] {
        if reinstall {
            fs::remove_dir_all(project.path().join("node_modules")).unwrap();
        }
        let output = Command::new(env!("CARGO_BIN_EXE_utoo"))
            .current_dir(project.path())
            .env("UTOO_CACHE_DIR", cache.path())
            .env("NO_UPDATE_NOTIFIER", "1")
            .env("NO_PROXY", "127.0.0.1,localhost")
            .env("no_proxy", "127.0.0.1,localhost")
            .args(["install", "--ignore-scripts"])
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");

        let lock: Value =
            serde_json::from_slice(&fs::read(project.path().join("package-lock.json")).unwrap())
                .unwrap();
        let packages = lock["packages"].as_object().unwrap();
        assert_eq!(packages.len(), if different_url { 4 } else { 3 });
        assert_eq!(packages["node_modules/shared"]["resolved"], shared_url);
        let nested = "node_modules/consumer/node_modules/shared";
        assert_eq!(packages.contains_key(nested), different_url);
        assert_eq!(project.path().join(nested).exists(), different_url);
        if different_url {
            assert_eq!(packages[nested]["resolved"], dependency_url);
        }

        let output = Command::new("node")
            .current_dir(project.path())
            .args([
                "-e",
                &format!(
                    "const assert = require('node:assert/strict');
                     const shared = require('shared');
                     const consumer = require('consumer');
                     shared.set('example', 42);
                     assert.equal(consumer === shared, {});
                     assert.equal(consumer.get('example'), {});",
                    !different_url,
                    if different_url { "undefined" } else { "42" }
                ),
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
    }
}

#[test]
fn identical_http_tarballs_share_module_state() {
    check_shared_tarball(false);
}

#[test]
fn different_http_tarballs_with_same_version_stay_separate() {
    check_shared_tarball(true);
}
