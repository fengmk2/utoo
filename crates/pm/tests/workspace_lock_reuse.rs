use std::fs;
use std::process::Command;

use serde_json::{Value, json};
use tempfile::tempdir;

#[test]
fn workspace_nested_dependency_preserves_locked_metadata() {
    let mut server = mockito::Server::new();
    let resolved = format!("{}/shared-2.0.0.tgz", server.url());
    // The registry now has more metadata than the original lockfile. Reusing
    // the workspace's pinned package must not fetch and replace its manifest.
    let manifest = json!({
        "name": "shared", "version": "2.0.0",
        "dist": { "tarball": resolved },
        "license": "MIT", "scripts": { "test": "node test.js" }
    });
    let packument = server
        .mock("GET", "/shared")
        .with_body(
            json!({
                "name": "shared", "dist-tags": { "latest": "2.0.0" },
                "versions": { "2.0.0": manifest }
            })
            .to_string(),
        )
        .expect(0)
        .create();
    let version = server
        .mock("GET", "/shared/2.0.0")
        .with_body(manifest.to_string())
        .expect(0)
        .create();
    let project = tempdir().unwrap();
    let cache = tempdir().unwrap();
    let root = json!({
        "name": "root", "version": "1.0.0", "workspaces": ["packages/*"],
        "dependencies": { "shared": "1.0.0" }
    });
    let workspace = json!({
        "name": "app", "version": "1.0.0",
        "dependencies": { "shared": "2.0.0" }
    });
    fs::create_dir_all(project.path().join("packages/app")).unwrap();
    fs::write(project.path().join("package.json"), root.to_string()).unwrap();
    fs::write(
        project.path().join("packages/app/package.json"),
        workspace.to_string(),
    )
    .unwrap();
    let baseline = json!({
        "name": "root", "version": "1.0.0", "lockfileVersion": 3, "requires": true,
        "packages": {
            "": root,
            "packages/app": workspace,
            "node_modules/app": { "name": "app", "resolved": "packages/app", "link": true },
            "node_modules/shared": {
                "name": "shared", "version": "1.0.0",
                "resolved": format!("{}/shared-1.0.0.tgz", server.url())
            },
            "packages/app/node_modules/shared": {
                "name": "shared", "version": "2.0.0", "resolved": resolved
            }
        }
    });
    fs::write(
        project.path().join("package-lock.json"),
        baseline.to_string(),
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_utoo"))
        .current_dir(project.path())
        .env("UTOO_CACHE_DIR", cache.path())
        .env("NO_UPDATE_NOTIFIER", "1")
        .env("NO_PROXY", "127.0.0.1,localhost")
        .env("no_proxy", "127.0.0.1,localhost")
        .args(["deps", "--registry", &server.url()])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let lock: Value =
        serde_json::from_slice(&fs::read(project.path().join("package-lock.json")).unwrap())
            .unwrap();
    assert_eq!(lock["packages"], baseline["packages"]);
    packument.assert();
    version.assert();
}
