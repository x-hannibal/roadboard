use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;

static SEMVER_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^\d+\.\d+\.\d+$").expect("static regex is valid"));

pub fn bump_command(
    version_arg: Option<&str>,
    dry_run: bool,
    no_push: bool,
    no_tag: bool,
) -> Result<()> {
    let repo_root = find_repo_root()?;

    // 1. Validate working tree is clean
    check_clean_worktree(&repo_root)?;

    // 2. Determine target version
    let current = read_workspace_version(&repo_root)?;
    let target = match version_arg {
        Some(v) => parse_semver(v)?,
        None => increment_patch(&current)?,
    };

    // 3. Read [Unreleased] section
    let changelog_path = repo_root.join("CHANGELOG.md");
    let changelog = std::fs::read_to_string(&changelog_path)
        .context("failed to read CHANGELOG.md")?;
    let unreleased = extract_unreleased(&changelog)?;

    if unreleased.trim().is_empty() {
        if is_ai_caller() {
            bail!(
                "EMPTY_UNRELEASED: [Unreleased] section in CHANGELOG.md is empty. \
                 Update the changelog before bumping the version."
            );
        }
        eprintln!("Warning: [Unreleased] section is empty — proceeding.");
    }

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    println!("Bumping {} → {}", current, target);

    if dry_run {
        println!("[dry-run] Would transform CHANGELOG.md: [Unreleased] → [{target}] - {today}");
        println!("[dry-run] Would set workspace version to {target}");
        println!("[dry-run] Would commit: chore(release): v{target}");
        if !no_tag {
            println!("[dry-run] Would tag: v{target}");
        }
        if !no_push {
            println!("[dry-run] Would push with --follow-tags");
        }
        return Ok(());
    }

    // 4. Transform CHANGELOG.md
    let new_changelog = transform_changelog(&changelog, &target, &today)?;
    std::fs::write(&changelog_path, &new_changelog)
        .context("failed to write CHANGELOG.md")?;

    // 5. Bump workspace version in Cargo.toml
    bump_workspace_version(&repo_root, &target)?;

    // 5b. Bump apps/web/package.json if present
    let pkg_json = repo_root.join("apps/web/package.json");
    if pkg_json.exists() {
        bump_package_json(&pkg_json, &target)?;
    }

    // 6. Stage and commit
    let mut git_add = Command::new("git");
    git_add.current_dir(&repo_root).arg("add").arg("CHANGELOG.md").arg("Cargo.toml");
    if pkg_json.exists() {
        git_add.arg("apps/web/package.json");
    }
    let status = git_add.status().context("git add failed")?;
    if !status.success() {
        bail!("git add failed");
    }

    let commit_msg = format!("chore(release): v{target}");
    let status = Command::new("git")
        .current_dir(&repo_root)
        .args(["commit", "-m", &commit_msg])
        .status()
        .context("git commit failed")?;
    if !status.success() {
        bail!("git commit failed");
    }

    // 7. Tag
    if !no_tag {
        let tag = format!("v{target}");
        let status = Command::new("git")
            .current_dir(&repo_root)
            .args(["tag", &tag, "-m", &tag])
            .status()
            .context("git tag failed")?;
        if !status.success() {
            bail!("git tag failed");
        }
        println!("Tagged {tag}");
    }

    // 8. Push
    if !no_push {
        let branch = current_branch(&repo_root)?;
        let status = Command::new("git")
            .current_dir(&repo_root)
            .args(["push", "origin", &branch, "--follow-tags"])
            .status()
            .context("git push failed")?;
        if !status.success() {
            bail!("git push failed");
        }
    }

    println!("Released v{target}");
    Ok(())
}

// ─── helpers ─────────────────────────────────────────────────────────────────

fn find_repo_root() -> Result<PathBuf> {
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("git rev-parse --show-toplevel failed")?;
    let path = String::from_utf8(out.stdout)?.trim().to_string();
    Ok(PathBuf::from(path))
}

fn check_clean_worktree(repo_root: &Path) -> Result<()> {
    let out = Command::new("git")
        .current_dir(repo_root)
        .args(["status", "--porcelain"])
        .output()
        .context("git status failed")?;
    let stdout = String::from_utf8(out.stdout)?;
    if !stdout.trim().is_empty() {
        bail!(
            "Working tree is not clean. Commit or stash changes before bumping.\n{stdout}"
        );
    }
    Ok(())
}

fn read_workspace_version(repo_root: &Path) -> Result<String> {
    let text = std::fs::read_to_string(repo_root.join("Cargo.toml"))?;
    let doc: toml_edit::DocumentMut = text.parse()?;
    let v = doc["workspace"]["package"]["version"]
        .as_str()
        .context("[workspace.package] version not found in Cargo.toml")?
        .to_string();
    Ok(v)
}

pub fn parse_semver(s: &str) -> Result<String> {
    if !SEMVER_RE.is_match(s) {
        bail!("Invalid version '{}'. Only MAJOR.MINOR.PATCH is accepted.", s);
    }
    Ok(s.to_string())
}

pub fn increment_patch(version: &str) -> Result<String> {
    let parts: Vec<&str> = version.splitn(3, '.').collect();
    if parts.len() != 3 {
        bail!("Cannot parse version '{version}' as MAJOR.MINOR.PATCH");
    }
    let major: u64 = parts[0].parse().context("invalid major")?;
    let minor: u64 = parts[1].parse().context("invalid minor")?;
    let patch: u64 = parts[2].parse().context("invalid patch")?;
    Ok(format!("{major}.{minor}.{}", patch + 1))
}

/// Extract the raw content of the `[Unreleased]` section (everything between
/// the header line and the next `## [` line, exclusive of both).
pub fn extract_unreleased(changelog: &str) -> Result<String> {
    let mut in_section = false;
    let mut content = String::new();
    for line in changelog.lines() {
        if line.starts_with("## [Unreleased]") {
            in_section = true;
            continue;
        }
        if in_section {
            if line.starts_with("## [") {
                break;
            }
            content.push_str(line);
            content.push('\n');
        }
    }
    Ok(content)
}

/// Move [Unreleased] content into a new `## [version] - date` section,
/// leaving a fresh empty `## [Unreleased]` above it.
pub fn transform_changelog(changelog: &str, version: &str, date: &str) -> Result<String> {
    let header = "## [Unreleased]";
    let Some(pos) = changelog.find(header) else {
        bail!("## [Unreleased] not found in CHANGELOG.md");
    };

    let after_header = pos + header.len();
    let rest = &changelog[after_header..];

    // Find the start of the next versioned section (skip the leading newline)
    let next_section = rest.find("\n## [").map(|i| i + 1).unwrap_or(rest.len());
    let unreleased_body = &rest[..next_section]; // includes leading newline(s)
    let tail = &rest[next_section..];

    let before = &changelog[..pos]; // everything before the header

    Ok(format!(
        "{before}{header}\n\n## [{version}] - {date}{unreleased_body}{tail}"
    ))
}

fn bump_workspace_version(repo_root: &Path, new_version: &str) -> Result<()> {
    let path = repo_root.join("Cargo.toml");
    let text = std::fs::read_to_string(&path)?;
    let mut doc: toml_edit::DocumentMut = text.parse()?;
    doc["workspace"]["package"]["version"] = toml_edit::value(new_version);
    std::fs::write(&path, doc.to_string())?;
    Ok(())
}

fn bump_package_json(path: &Path, new_version: &str) -> Result<()> {
    let text = std::fs::read_to_string(path)?;
    let mut json: serde_json::Value = serde_json::from_str(&text)?;
    json["version"] = serde_json::Value::String(new_version.to_string());
    std::fs::write(path, format!("{}\n", serde_json::to_string_pretty(&json)?))?;
    Ok(())
}

fn current_branch(repo_root: &Path) -> Result<String> {
    let out = Command::new("git")
        .current_dir(repo_root)
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .context("git rev-parse HEAD failed")?;
    Ok(String::from_utf8(out.stdout)?.trim().to_string())
}

fn is_ai_caller() -> bool {
    let is_set = |var: &str| matches!(std::env::var(var).as_deref(), Ok("1") | Ok("true"));
    is_set("ROBOT_AI") || is_set("CLAUDE_CODE") || is_set("CURSOR_AGENT")
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    #[test]
    fn semver_valid() {
        assert_eq!(parse_semver("1.2.3").unwrap(), "1.2.3");
        assert_eq!(parse_semver("0.0.1").unwrap(), "0.0.1");
        assert_eq!(parse_semver("10.20.300").unwrap(), "10.20.300");
    }

    #[test]
    fn semver_invalid() {
        assert!(parse_semver("0.2").is_err());
        assert!(parse_semver("1.0.0-beta").is_err());
        assert!(parse_semver("abc").is_err());
        assert!(parse_semver("1.0.0.0").is_err());
    }

    #[test]
    fn patch_increment() {
        assert_eq!(increment_patch("0.1.0").unwrap(), "0.1.1");
        assert_eq!(increment_patch("1.2.9").unwrap(), "1.2.10");
        assert_eq!(increment_patch("0.0.0").unwrap(), "0.0.1");
    }

    #[test]
    fn extract_unreleased_with_content() {
        let cl = "# Changelog\n\n## [Unreleased]\n### Added\n- Feature A\n\n## [0.1.0] - 2025-01-01\n- Old\n";
        let u = extract_unreleased(cl).unwrap();
        assert!(u.contains("Feature A"), "should contain feature A");
        assert!(!u.contains("0.1.0"), "should not contain old release");
    }

    #[test]
    fn extract_unreleased_empty() {
        let cl = "# Changelog\n\n## [Unreleased]\n\n## [0.1.0] - 2025-01-01\n- Old\n";
        let u = extract_unreleased(cl).unwrap();
        assert!(u.trim().is_empty(), "empty unreleased should be blank after trim");
    }

    #[test]
    fn transform_moves_content() {
        let cl = "# Changelog\n\n## [Unreleased]\n### Added\n- Foo\n\n## [0.1.0] - 2025-01-01\n- Old\n";
        let result = transform_changelog(cl, "0.2.0", "2025-06-01").unwrap();

        assert!(result.contains("## [0.2.0] - 2025-06-01"), "new section header");
        assert!(result.contains("- Foo"), "content moved");
        assert!(result.contains("## [0.1.0]"), "old section preserved");

        // [Unreleased] must now be empty
        let new_unreleased = extract_unreleased(&result).unwrap();
        assert!(new_unreleased.trim().is_empty(), "unreleased should be empty after transform");
    }

    #[test]
    fn transform_no_prior_releases() {
        let cl = "# Changelog\n\n## [Unreleased]\n### Added\n- Initial scaffold\n";
        let result = transform_changelog(cl, "0.0.1", "2026-04-24").unwrap();

        assert!(result.contains("## [0.0.1] - 2026-04-24"));
        assert!(result.contains("- Initial scaffold"));

        let new_unreleased = extract_unreleased(&result).unwrap();
        assert!(new_unreleased.trim().is_empty());
    }

    #[test]
    fn transform_with_tempfile() {
        use tempfile::NamedTempFile;

        let content = "# Changelog\n\n## [Unreleased]\n### Added\n- New feature\n";
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();

        let read_back = std::fs::read_to_string(file.path()).unwrap();
        let result = transform_changelog(&read_back, "1.0.0", "2025-06-01").unwrap();

        std::fs::write(file.path(), &result).unwrap();
        let final_content = std::fs::read_to_string(file.path()).unwrap();

        assert!(final_content.contains("## [1.0.0] - 2025-06-01"));
        assert!(final_content.contains("- New feature"));
    }
}
