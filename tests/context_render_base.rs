mod helpers;

use helpers::TestEnv;
use predicates::prelude::*;
use std::fs;

#[test]
fn test_context_render_with_base_sigil() -> anyhow::Result<()> {
    let env = TestEnv::new();
    helpers::setup_git_repo(env.root());

    // Create a dummy file and commit it on main
    fs::write(env.root().join("file.txt"), "original")?;
    run_git(env.root(), &["add", "file.txt"])?;
    run_git(env.root(), &["commit", "-m", "initial"])?;

    // Create a branch and modify the file
    run_git(env.root(), &["checkout", "-b", "feature"])?;
    fs::write(env.root().join("file.txt"), "modified")?;
    run_git(env.root(), &["add", "file.txt"])?;
    run_git(env.root(), &["commit", "-m", "feature change"])?;

    // Initialize mem
    env.command().arg("init").assert().success();

    // Create context.json with @base sigil
    let context_json = env.root().join(".mem").join("feature").join("context.json");
    fs::create_dir_all(context_json.parent().unwrap())?;
    fs::write(
        &context_json,
        r#"{
        "default": {
            "artifacts": [],
            "diff": "@base...HEAD"
        }
    }"#,
    )?;

    // Run mem context render --base main
    env.command()
        .arg("context")
        .arg("render")
        .arg("--base")
        .arg("main")
        .assert()
        .success()
        .stdout(predicate::str::contains("<diff>"))
        .stdout(predicate::str::contains("+modified"))
        .stdout(predicate::str::contains("-original"));

    // Run without --base should fail
    env.command()
        .arg("context")
        .arg("render")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "uses '@base' in diff, but no --base branch was provided",
        ));

    Ok(())
}

#[test]
fn test_context_render_with_base_sigil_in_artifacts_and_include() -> anyhow::Result<()> {
    let env = TestEnv::new();
    helpers::setup_git_repo(env.root());

    // Setup main branch with an artifact
    fs::write(env.root().join("file.txt"), "main content")?;
    run_git(env.root(), &["add", "file.txt"])?;
    run_git(env.root(), &["commit", "-m", "main"])?;

    // Setup mem artifacts on main
    let main_mem = env.root().join(".mem").join("main");
    fs::create_dir_all(&main_mem.join("spec"))?;
    fs::write(main_mem.join("spec").join("index.md"), "# Main Index")?;
    fs::write(
        main_mem.join("context.json"),
        r#"{
        "brief": {
            "artifacts": ["./spec/index.md"]
        }
    }"#,
    )?;

    // Create a feature branch
    run_git(env.root(), &["checkout", "-b", "feature"])?;

    // Setup context.json on feature using @base
    let feature_mem = env.root().join(".mem").join("feature");
    fs::create_dir_all(&feature_mem)?;
    fs::write(
        feature_mem.join("context.json"),
        r#"{
        "default": {
            "artifacts": ["@base:spec/index.md"],
            "include": ["@base:brief"]
        }
    }"#,
    )?;

    // Run mem context render --base main
    env.command()
        .arg("context")
        .arg("render")
        .arg("--base")
        .arg("main")
        .assert()
        .success()
        // Artifacts are deduplicated, so we should see it once
        .stdout(predicate::str::contains("path=\".mem/main/spec/index.md\""))
        .stdout(predicate::str::contains("# Main Index"));

    // Run without --base should fail for artifacts
    fs::write(
        feature_mem.join("context.json"),
        r#"{
        "default": {
            "artifacts": ["@base:spec/index.md"]
        }
    }"#,
    )?;
    env.command()
        .arg("context")
        .arg("render")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "uses '@base' in artifact path, but no --base branch was provided",
        ));

    // Run without --base should fail for includes
    fs::write(
        feature_mem.join("context.json"),
        r#"{
        "default": {
            "include": ["@base"]
        }
    }"#,
    )?;
    env.command()
        .arg("context")
        .arg("render")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "uses '@base' in include, but no --base branch was provided",
        ));

    Ok(())
}

fn run_git(dir: &std::path::Path, args: &[&str]) -> anyhow::Result<()> {
    let status = std::process::Command::new("git")
        .current_dir(dir)
        .args(args)
        .status()?;
    if !status.success() {
        anyhow::bail!("git command failed: {:?}", args);
    }
    Ok(())
}
