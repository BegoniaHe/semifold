use std::{collections::HashMap, path::Path, process::Command};

use regex::Regex;

use crate::{
    config::{PackageConfig, ResolverConfig, VersionMode},
    context,
    error::ResolveError,
    resolver::{ResolvedPackage, Resolver, ResolverType},
    utils,
};

/// Represents a parsed go.mod file
#[derive(Debug)]
#[allow(dead_code)]
struct GoMod {
    pub module: String,
    pub go_version: Option<String>,
    pub require: Vec<GoRequire>,
}

#[derive(Debug)]
#[allow(dead_code)]
struct GoRequire {
    pub path: String,
    pub version: String,
}

/// Represents a go.work file for Go workspaces
#[derive(Debug)]
#[allow(dead_code)]
struct GoWork {
    pub go_version: Option<String>,
    pub use_dirs: Vec<String>,
}

pub struct GoResolver;

impl GoResolver {
    /// Parse go.mod file content
    fn parse_go_mod(content: &str, path: &Path) -> Result<GoMod, ResolveError> {
        let mut module = String::new();
        let mut go_version = None;
        let mut require = Vec::new();

        let module_re = Regex::new(r"^module\s+(.+)$").map_err(|e| ResolveError::ParseError {
            path: path.to_path_buf(),
            reason: format!("Invalid regex: {}", e),
        })?;

        let go_re = Regex::new(r"^go\s+([\d.]+)").map_err(|e| ResolveError::ParseError {
            path: path.to_path_buf(),
            reason: format!("Invalid regex: {}", e),
        })?;

        let require_single_re =
            Regex::new(r"^require\s+(\S+)\s+(\S+)").map_err(|e| ResolveError::ParseError {
                path: path.to_path_buf(),
                reason: format!("Invalid regex: {}", e),
            })?;

        let require_line_re =
            Regex::new(r"^\s*(\S+)\s+(\S+)").map_err(|e| ResolveError::ParseError {
                path: path.to_path_buf(),
                reason: format!("Invalid regex: {}", e),
            })?;

        let mut in_require_block = false;

        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with("//") {
                continue;
            }

            // Parse module directive
            if let Some(caps) = module_re.captures(line) {
                module = caps[1].to_string();
                continue;
            }

            // Parse go directive
            if let Some(caps) = go_re.captures(line) {
                go_version = Some(caps[1].to_string());
                continue;
            }

            // Parse require block start
            if line == "require (" {
                in_require_block = true;
                continue;
            }

            // Parse require block end
            if line == ")" && in_require_block {
                in_require_block = false;
                continue;
            }

            // Parse single-line require
            if let Some(caps) = require_single_re.captures(line) {
                require.push(GoRequire {
                    path: caps[1].to_string(),
                    version: caps[2].to_string(),
                });
                continue;
            }

            // Parse require block entries
            if in_require_block {
                if let Some(caps) = require_line_re.captures(line) {
                    let dep_path = caps[1].to_string();
                    let dep_version = caps[2].to_string();
                    // Skip indirect dependencies marker
                    if !dep_path.starts_with("//") {
                        require.push(GoRequire {
                            path: dep_path,
                            version: dep_version,
                        });
                    }
                }
            }
        }

        if module.is_empty() {
            return Err(ResolveError::ParseError {
                path: path.to_path_buf(),
                reason: "module directive not found in go.mod".to_string(),
            });
        }

        Ok(GoMod {
            module,
            go_version,
            require,
        })
    }

    /// Parse go.work file content
    fn parse_go_work(content: &str, path: &Path) -> Result<GoWork, ResolveError> {
        let mut go_version = None;
        let mut use_dirs = Vec::new();

        let go_re = Regex::new(r"^go\s+([\d.]+)").map_err(|e| ResolveError::ParseError {
            path: path.to_path_buf(),
            reason: format!("Invalid regex: {}", e),
        })?;

        let use_single_re = Regex::new(r"^use\s+(\S+)").map_err(|e| ResolveError::ParseError {
            path: path.to_path_buf(),
            reason: format!("Invalid regex: {}", e),
        })?;

        let use_line_re = Regex::new(r"^\s*(\S+)").map_err(|e| ResolveError::ParseError {
            path: path.to_path_buf(),
            reason: format!("Invalid regex: {}", e),
        })?;

        let mut in_use_block = false;

        for line in content.lines() {
            let line = line.trim();

            if line.is_empty() || line.starts_with("//") {
                continue;
            }

            if let Some(caps) = go_re.captures(line) {
                go_version = Some(caps[1].to_string());
                continue;
            }

            if line == "use (" {
                in_use_block = true;
                continue;
            }

            if line == ")" && in_use_block {
                in_use_block = false;
                continue;
            }

            if let Some(caps) = use_single_re.captures(line) {
                use_dirs.push(caps[1].to_string());
                continue;
            }

            if in_use_block {
                if let Some(caps) = use_line_re.captures(line) {
                    let dir = caps[1].to_string();
                    if dir != ")" && !dir.starts_with("//") {
                        use_dirs.push(dir);
                    }
                }
            }
        }

        Ok(GoWork {
            go_version,
            use_dirs,
        })
    }

    /// Extract version from version.go file
    fn extract_version_from_version_go(
        &self,
        package_path: &Path,
    ) -> Result<Option<String>, ResolveError> {
        let version_go_path = package_path.join("version.go");
        if !version_go_path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&version_go_path)?;

        // Match patterns like:
        // const Version = "1.2.3"
        // var Version = "1.2.3"
        // const VERSION = "1.2.3"
        let version_re = Regex::new(
            r#"(?i)(?:const|var)\s+version\s*=\s*"v?([\d]+\.[\d]+\.[\d]+(?:-[a-zA-Z0-9.-]+)?(?:\+[a-zA-Z0-9.-]+)?)""#,
        )
        .map_err(|e| ResolveError::ParseError {
            path: version_go_path.clone(),
            reason: format!("Invalid regex: {}", e),
        })?;

        if let Some(caps) = version_re.captures(&content) {
            return Ok(Some(caps[1].to_string()));
        }

        Ok(None)
    }

    /// Extract version from git tags
    fn extract_version_from_git_tag(
        &self,
        repo_root: &Path,
        module_path: &str,
    ) -> Result<Option<String>, ResolveError> {
        // Try to get the latest tag matching the module
        let output = Command::new("git")
            .args(["tag", "--list", "--sort=-v:refname"])
            .current_dir(repo_root)
            .output();

        let output = match output {
            Ok(o) => o,
            Err(_) => return Ok(None),
        };

        if !output.status.success() {
            return Ok(None);
        }

        let tags = String::from_utf8_lossy(&output.stdout);

        // For submodules, look for tags like "submodule/v1.2.3"
        // For root modules, look for tags like "v1.2.3"
        let version_re =
            Regex::new(r"^v?([\d]+\.[\d]+\.[\d]+(?:-[a-zA-Z0-9.-]+)?(?:\+[a-zA-Z0-9.-]+)?)$")
                .map_err(|e| ResolveError::ParseError {
                    path: repo_root.to_path_buf(),
                    reason: format!("Invalid regex: {}", e),
                })?;

        for tag in tags.lines() {
            let tag = tag.trim();

            // Check if tag matches module prefix for submodules
            if let Some(stripped) = tag.strip_prefix(&format!("{}/", module_path)) {
                if let Some(caps) = version_re.captures(stripped) {
                    return Ok(Some(caps[1].to_string()));
                }
            }

            // Check for root module version tags
            if let Some(caps) = version_re.captures(tag) {
                return Ok(Some(caps[1].to_string()));
            }
        }

        Ok(None)
    }

    /// Get version for a Go module using priority: custom file > git tag > version.go > default
    fn get_version(
        &self,
        root: &Path,
        package_path: &Path,
        _module_path: &str,
    ) -> Result<String, ResolveError> {
        let full_path = root.join(package_path);

        // Priority 1: Check for version.go
        if let Some(version) = self.extract_version_from_version_go(&full_path)? {
            log::debug!("Found version {} from version.go", version);
            return Ok(version);
        }

        // Priority 2: Try git tag
        if let Some(version) = self.extract_version_from_git_tag(root, _module_path)? {
            log::debug!("Found version {} from git tag", version);
            return Ok(version);
        }

        // Priority 3: Default version
        log::debug!("Using default version 0.0.0");
        Ok("0.0.0".to_string())
    }

    /// Update version in version.go file
    fn update_version_go(
        &self,
        package_path: &Path,
        new_version: &str,
    ) -> Result<(), ResolveError> {
        let version_go_path = package_path.join("version.go");

        if !version_go_path.exists() {
            // Create version.go if it doesn't exist
            let content = format!(
                r#"package main

// Version is the current version of the module.
const Version = "{}"
"#,
                new_version
            );
            std::fs::write(&version_go_path, content)?;
            log::info!("Created {:?} with version {}", version_go_path, new_version);
            return Ok(());
        }

        let content = std::fs::read_to_string(&version_go_path)?;

        // Replace version in existing file
        let version_re = Regex::new(
            r#"(?i)((?:const|var)\s+version\s*=\s*")v?[\d]+\.[\d]+\.[\d]+(?:-[a-zA-Z0-9.-]+)?(?:\+[a-zA-Z0-9.-]+)?(")"#,
        )
        .map_err(|e| ResolveError::ParseError {
            path: version_go_path.clone(),
            reason: format!("Invalid regex: {}", e),
        })?;

        let updated_content = version_re.replace(&content, |caps: &regex::Captures| {
            format!("{}{}{}", &caps[1], new_version, &caps[2])
        });

        std::fs::write(&version_go_path, updated_content.as_ref())?;
        log::info!("Updated {:?} to version {}", version_go_path, new_version);

        Ok(())
    }

    /// Extract module name from module path (last component)
    fn module_name_from_path(module_path: &str) -> String {
        module_path
            .rsplit('/')
            .next()
            .unwrap_or(module_path)
            .to_string()
    }
}

impl Resolver for GoResolver {
    fn resolve(
        &mut self,
        root: &Path,
        pkg_config: &PackageConfig,
    ) -> Result<ResolvedPackage, ResolveError> {
        let go_mod_path = root.join(&pkg_config.path).join("go.mod");
        if !go_mod_path.exists() {
            return Err(ResolveError::FileOrDirNotFound {
                path: go_mod_path.clone(),
            });
        }

        let go_mod_str = std::fs::read_to_string(&go_mod_path)?;
        let go_mod = Self::parse_go_mod(&go_mod_str, &go_mod_path)?;

        let version_str = self.get_version(root, &pkg_config.path, &go_mod.module)?;
        let version = semver::Version::parse(&version_str)?;

        let package = ResolvedPackage {
            name: Self::module_name_from_path(&go_mod.module),
            version,
            path: pkg_config.path.clone(),
            private: false,
        };

        Ok(package)
    }

    fn resolve_all(&mut self, root: &Path) -> Result<Vec<ResolvedPackage>, ResolveError> {
        // First check for go.work (Go workspace)
        let go_work_path = root.join("go.work");
        if go_work_path.exists() {
            let go_work_str = std::fs::read_to_string(&go_work_path)?;
            let go_work = Self::parse_go_work(&go_work_str, &go_work_path)?;

            let packages = go_work
                .use_dirs
                .iter()
                .map(|dir| {
                    let rel_path = if dir == "." {
                        ".".into()
                    } else {
                        dir.trim_start_matches("./").into()
                    };
                    self.resolve(
                        root,
                        &PackageConfig {
                            path: rel_path,
                            resolver: ResolverType::Go,
                            version_mode: VersionMode::Semantic,
                            assets: vec![],
                        },
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;

            return Ok(packages);
        }

        // Check for single go.mod
        let go_mod_path = root.join("go.mod");
        if !go_mod_path.exists() {
            log::warn!(
                "Cannot resolve package in {}, go.mod not found.",
                root.display()
            );
            return Ok(vec![]);
        }

        let package = self.resolve(
            root,
            &PackageConfig {
                path: ".".into(),
                resolver: ResolverType::Go,
                version_mode: VersionMode::Semantic,
                assets: vec![],
            },
        )?;

        Ok(vec![package])
    }

    fn bump(
        &mut self,
        ctx: &context::Context,
        root: &Path,
        package: &ResolvedPackage,
        version: &semver::Version,
    ) -> Result<(), ResolveError> {
        let bumped_version = version.to_string();
        let package_path = root.join(&package.path);

        if ctx.dry_run {
            log::warn!(
                "Skip bump for {} to version {} due to dry run",
                package.name,
                bumped_version
            );
            return Ok(());
        }

        // Update version.go
        self.update_version_go(&package_path, &bumped_version)?;

        Ok(())
    }

    fn sort_packages(
        &mut self,
        root: &Path,
        packages: &mut Vec<(String, PackageConfig)>,
    ) -> Result<(), ResolveError> {
        let cached_packages = packages
            .iter()
            .filter(|(_, cfg)| cfg.resolver == ResolverType::Go)
            .try_fold(HashMap::new(), |mut acc, (name, cfg)| {
                let go_mod_path = root.join(&cfg.path).join("go.mod");
                let go_mod_str = std::fs::read_to_string(&go_mod_path)?;
                let go_mod = Self::parse_go_mod(&go_mod_str, &go_mod_path)?;
                acc.insert(name.clone(), go_mod);
                Ok::<_, ResolveError>(acc)
            })?;

        // Build a map of module path -> package name for dependency resolution
        let module_to_name: HashMap<String, String> = cached_packages
            .iter()
            .map(|(name, go_mod)| (go_mod.module.clone(), name.clone()))
            .collect();

        packages.sort_by(|(a, a_cfg), (b, b_cfg)| {
            if a_cfg.resolver == ResolverType::Go && b_cfg.resolver == ResolverType::Go {
                let a_mod = cached_packages.get(a).unwrap();
                let b_mod = cached_packages.get(b).unwrap();

                // Check if a depends on b
                let a_depends_on_b = a_mod.require.iter().any(|req| {
                    module_to_name
                        .get(&req.path)
                        .is_some_and(|dep_name| dep_name == b)
                });

                // Check if b depends on a
                let b_depends_on_a = b_mod.require.iter().any(|req| {
                    module_to_name
                        .get(&req.path)
                        .is_some_and(|dep_name| dep_name == a)
                });

                if a_depends_on_b {
                    std::cmp::Ordering::Greater
                } else if b_depends_on_a {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Equal
                }
            } else {
                std::cmp::Ordering::Equal
            }
        });

        Ok(())
    }

    fn publish(
        &mut self,
        package: &ResolvedPackage,
        resolver_config: &ResolverConfig,
        dry_run: bool,
    ) -> Result<(), ResolveError> {
        // Go modules don't have a traditional publish step,
        // versioning is done via git tags

        log::info!("Running prepublish commands for {}", package.name);
        for prepublish in &resolver_config.prepublish {
            let args = prepublish.args.clone().unwrap_or_default();
            if dry_run && !prepublish.dry_run.unwrap_or(false) {
                log::warn!(
                    "Skip prepublish command {} {} due to dry run",
                    prepublish.command,
                    args.join(" ")
                );
                continue;
            }
            log::info!("Running {} {}", prepublish.command, args.join(" "));
            utils::run_command(prepublish, &package.path)?;
        }

        log::info!("Running publish commands for {}", package.name);
        for publish in &resolver_config.publish {
            let args = publish.args.clone().unwrap_or_default();
            if dry_run && !publish.dry_run.unwrap_or(false) {
                log::warn!(
                    "Skip publish command {} {} due to dry run",
                    publish.command,
                    args.join(" ")
                );
                continue;
            }
            log::info!("Running {} {}", publish.command, args.join(" "));
            utils::run_command(publish, &package.path)?;
        }

        Ok(())
    }
}
