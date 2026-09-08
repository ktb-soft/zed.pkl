use std::fs;
use std::io::Read;

use zed::serde_json::{self, Value};
use zed::settings::LspSettings;
use zed_extension_api::{self as zed, LanguageServerId, Result};

const PKL_LSP_REPO: &str = "apple/pkl-lsp";
const JAR_PREFIX: &str = "pkl-lsp-";
const JAR_SUFFIX: &str = ".jar";
const ZIP_MAGIC: [u8; 4] = *b"PK\x03\x04";
const MINIMUM_JAVA_VERSION: u32 = 23;

struct PklExtension {
    cached_jar_path: Option<String>,
}

fn is_valid_jar(path: &str) -> bool {
    let Ok(mut file) = fs::File::open(path) else {
        return false;
    };

    let mut magic = [0u8; 4];
    file.read_exact(&mut magic).is_ok() && magic == ZIP_MAGIC
}

fn find_downloaded_jar() -> Option<String> {
    fs::read_dir(".")
        .ok()?
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with(JAR_PREFIX) && name.ends_with(JAR_SUFFIX))
        .filter(|name| is_valid_jar(name))
        .max()
}

fn remove_jars_except(kept_jar: &str) {
    let Ok(entries) = fs::read_dir(".") else {
        return;
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with(JAR_PREFIX) && name != kept_jar {
            fs::remove_file(entry.path()).ok();
        }
    }
}

fn absolute_path(path: &str) -> Result<String> {
    std::path::absolute(path)
        .map_err(|err| err.to_string())?
        .to_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("path to {path} is not valid UTF-8"))
}

fn download_jar(asset: &zed::GithubReleaseAsset, destination: &str) -> Result<()> {
    let staging_path = format!("{destination}.part");

    zed::download_file(
        &asset.download_url,
        &staging_path,
        zed::DownloadedFileType::Uncompressed,
    )
    .map_err(|err| format!("failed to download {}: {err}", asset.name))?;

    if !is_valid_jar(&staging_path) {
        fs::remove_file(&staging_path).ok();
        return Err(format!(
            "downloaded {} is not a valid jar; the transfer was truncated or corrupted",
            asset.name
        ));
    }

    fs::rename(&staging_path, destination).map_err(|err| err.to_string())
}

fn install_latest_jar(language_server_id: &LanguageServerId) -> Result<String> {
    let release = zed::latest_github_release(
        PKL_LSP_REPO,
        zed::GithubReleaseOptions {
            require_assets: true,
            pre_release: false,
        },
    )?;

    let jar_name = format!("{JAR_PREFIX}{}{JAR_SUFFIX}", release.version);
    if is_valid_jar(&jar_name) {
        return Ok(jar_name);
    }

    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == jar_name)
        .ok_or_else(|| format!("the latest pkl-lsp release has no asset named {jar_name}"))?;

    zed::set_language_server_installation_status(
        language_server_id,
        &zed::LanguageServerInstallationStatus::Downloading,
    );
    download_jar(asset, &jar_name)?;

    Ok(jar_name)
}

fn missing_java_message(java_path: &str) -> String {
    format!(
        "pkl-lsp needs a Java {MINIMUM_JAVA_VERSION}+ runtime, but `{java_path}` could not \
         report a version. On macOS `/usr/bin/java` is a stub that exists even when no JDK is \
         installed. Install a JDK {MINIMUM_JAVA_VERSION} or newer and restart Zed."
    )
}

fn parse_java_major_version(version_output: &str) -> Option<u32> {
    let mut components = version_output.split('"').nth(1)?.split(['.', '-', '_']);

    let leading = components.next()?;
    if leading == "1" {
        components.next()?.parse().ok()
    } else {
        leading.parse().ok()
    }
}

fn resolve_java_path(worktree: &zed::Worktree) -> Result<String> {
    let java_path = worktree.which("java").ok_or_else(|| {
        format!("no `java` found on PATH. pkl-lsp needs a Java {MINIMUM_JAVA_VERSION}+ runtime.")
    })?;

    let output = zed::Command::new(java_path.as_str())
        .arg("-version")
        .envs(worktree.shell_env())
        .output()
        .map_err(|err| format!("failed to run `{java_path} -version`: {err}"))?;

    let reported = String::from_utf8_lossy(&output.stderr);
    let version =
        parse_java_major_version(&reported).ok_or_else(|| missing_java_message(&java_path))?;

    if version < MINIMUM_JAVA_VERSION {
        return Err(format!(
            "pkl-lsp needs Java {MINIMUM_JAVA_VERSION}+, but `{java_path}` is Java {version}. \
             Install a newer JDK and put it ahead of that one on your PATH."
        ));
    }

    Ok(java_path)
}

fn user_lsp_settings(
    language_server_id: &LanguageServerId,
    worktree: &zed::Worktree,
) -> LspSettings {
    LspSettings::for_worktree(language_server_id.as_ref(), worktree).unwrap_or_default()
}

fn merge_top_level(base: &mut Value, overrides: Option<Value>) {
    let (Some(base), Some(Value::Object(overrides))) = (base.as_object_mut(), overrides) else {
        return;
    };

    base.extend(overrides);
}

impl PklExtension {
    fn language_server_path(&mut self, language_server_id: &LanguageServerId) -> Result<String> {
        if let Some(path) = &self.cached_jar_path
            && is_valid_jar(path)
        {
            return Ok(path.clone());
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let installed_jar = install_latest_jar(language_server_id)
            .or_else(|install_error| find_downloaded_jar().ok_or(install_error));

        let jar_name = match installed_jar {
            Ok(jar_name) => jar_name,
            Err(install_error) => {
                zed::set_language_server_installation_status(
                    language_server_id,
                    &zed::LanguageServerInstallationStatus::Failed(install_error.clone()),
                );
                return Err(install_error);
            }
        };

        remove_jars_except(&jar_name);

        let jar_path = absolute_path(&jar_name)?;
        self.cached_jar_path = Some(jar_path.clone());

        Ok(jar_path)
    }
}

impl zed::Extension for PklExtension {
    fn new() -> Self {
        Self {
            cached_jar_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let binary = user_lsp_settings(language_server_id, worktree).binary;
        let (path, arguments, env) = match binary {
            Some(binary) => (binary.path, binary.arguments, binary.env),
            None => (None, None, None),
        };

        let command = match path {
            Some(path) => path,
            None => resolve_java_path(worktree)?,
        };

        let args = match arguments {
            Some(arguments) => arguments,
            None => vec![
                "-jar".into(),
                self.language_server_path(language_server_id)?,
            ],
        };

        let mut shell_env = worktree.shell_env();
        shell_env.extend(env.unwrap_or_default());

        Ok(zed::Command {
            command,
            args,
            env: shell_env,
        })
    }

    fn language_server_initialization_options(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<Value>> {
        let mut options = serde_json::json!({
            "extendedClientCapabilities": {
                "actionableRuntimeNotifications": true,
                "pklConfigureCommand": true
            }
        });

        let overrides = user_lsp_settings(language_server_id, worktree).initialization_options;
        merge_top_level(&mut options, overrides);

        Ok(Some(options))
    }

    fn language_server_workspace_configuration(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<Value>> {
        let mut configuration = serde_json::json!({
            "pkl.cli.path": worktree.which("pkl")
        });

        let overrides = user_lsp_settings(language_server_id, worktree).settings;
        merge_top_level(&mut configuration, overrides);

        Ok(Some(configuration))
    }
}

zed::register_extension!(PklExtension);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modern_java_version() {
        let output = "openjdk version \"23.0.1\" 2024-10-15\nOpenJDK Runtime Environment\n";
        assert_eq!(parse_java_major_version(output), Some(23));
    }

    #[test]
    fn parses_major_only_java_version() {
        assert_eq!(
            parse_java_major_version("openjdk version \"21\" 2023-09-19"),
            Some(21)
        );
    }

    #[test]
    fn parses_early_access_java_version() {
        assert_eq!(
            parse_java_major_version("openjdk version \"25-ea\" 2025-09-16"),
            Some(25)
        );
    }

    #[test]
    fn parses_legacy_java_version() {
        assert_eq!(
            parse_java_major_version("java version \"1.8.0_281\""),
            Some(8)
        );
    }

    #[test]
    fn rejects_macos_java_stub_output() {
        let output = "The operation couldn't be completed. Unable to locate a Java Runtime.\n";
        assert_eq!(parse_java_major_version(output), None);
    }

    #[test]
    fn rejects_unparsable_java_version() {
        assert_eq!(parse_java_major_version("openjdk version \"jdk\""), None);
        assert_eq!(parse_java_major_version("openjdk version \"1.x\""), None);
        assert_eq!(parse_java_major_version(""), None);
    }

    #[test]
    fn overrides_replace_defaults_and_keep_untouched_keys() {
        let mut base = serde_json::json!({ "pkl.cli.path": "/usr/bin/pkl", "keep": 1 });
        merge_top_level(
            &mut base,
            Some(serde_json::json!({ "pkl.cli.path": "/opt/pkl" })),
        );

        assert_eq!(
            base,
            serde_json::json!({ "pkl.cli.path": "/opt/pkl", "keep": 1 })
        );
    }

    #[test]
    fn non_object_overrides_are_ignored() {
        let original = serde_json::json!({ "pkl.cli.path": "/usr/bin/pkl" });

        let mut base = original.clone();
        merge_top_level(&mut base, None);
        assert_eq!(base, original);

        let mut base = original.clone();
        merge_top_level(&mut base, Some(serde_json::json!("nonsense")));
        assert_eq!(base, original);
    }
}
