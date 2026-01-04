//! Nixpacks Build Action
//!
//! Handles building applications using Nixpacks - a build system that
//! automatically detects and builds applications.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::actions::{Action, ActionError, ActionResult, CommandBuilder};

/// Supported Nixpacks application types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NixpacksAppType {
    Node,
    Python,
    Rust,
    Go,
    Ruby,
    Php,
    Java,
    Elixir,
    Haskell,
    Clojure,
    Crystal,
    Deno,
    Dart,
    Dotnet,
    Fsharp,
    Scala,
    Swift,
    Zig,
    Staticfile,
    Unknown,
}

impl std::fmt::Display for NixpacksAppType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NixpacksAppType::Node => write!(f, "node"),
            NixpacksAppType::Python => write!(f, "python"),
            NixpacksAppType::Rust => write!(f, "rust"),
            NixpacksAppType::Go => write!(f, "go"),
            NixpacksAppType::Ruby => write!(f, "ruby"),
            NixpacksAppType::Php => write!(f, "php"),
            NixpacksAppType::Java => write!(f, "java"),
            NixpacksAppType::Elixir => write!(f, "elixir"),
            NixpacksAppType::Haskell => write!(f, "haskell"),
            NixpacksAppType::Clojure => write!(f, "clojure"),
            NixpacksAppType::Crystal => write!(f, "crystal"),
            NixpacksAppType::Deno => write!(f, "deno"),
            NixpacksAppType::Dart => write!(f, "dart"),
            NixpacksAppType::Dotnet => write!(f, "dotnet"),
            NixpacksAppType::Fsharp => write!(f, "fsharp"),
            NixpacksAppType::Scala => write!(f, "scala"),
            NixpacksAppType::Swift => write!(f, "swift"),
            NixpacksAppType::Zig => write!(f, "zig"),
            NixpacksAppType::Staticfile => write!(f, "staticfile"),
            NixpacksAppType::Unknown => write!(f, "unknown"),
        }
    }
}

impl From<&str> for NixpacksAppType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "node" | "nodejs" => NixpacksAppType::Node,
            "python" => NixpacksAppType::Python,
            "rust" => NixpacksAppType::Rust,
            "go" | "golang" => NixpacksAppType::Go,
            "ruby" => NixpacksAppType::Ruby,
            "php" => NixpacksAppType::Php,
            "java" => NixpacksAppType::Java,
            "elixir" => NixpacksAppType::Elixir,
            "haskell" => NixpacksAppType::Haskell,
            "clojure" => NixpacksAppType::Clojure,
            "crystal" => NixpacksAppType::Crystal,
            "deno" => NixpacksAppType::Deno,
            "dart" => NixpacksAppType::Dart,
            "dotnet" | ".net" | "csharp" => NixpacksAppType::Dotnet,
            "fsharp" | "f#" => NixpacksAppType::Fsharp,
            "scala" => NixpacksAppType::Scala,
            "swift" => NixpacksAppType::Swift,
            "zig" => NixpacksAppType::Zig,
            "staticfile" | "static" => NixpacksAppType::Staticfile,
            _ => NixpacksAppType::Unknown,
        }
    }
}

/// Input for Nixpacks build
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NixpacksBuildInput {
    /// Application ID
    pub application_id: Uuid,
    /// Working directory containing source code
    pub workdir: String,
    /// Target image name
    pub image_name: String,
    /// Environment variables for build
    pub env_vars: HashMap<String, String>,
    /// Custom build command (optional)
    pub build_command: Option<String>,
    /// Custom start command (optional)
    pub start_command: Option<String>,
    /// Custom install command (optional)
    pub install_command: Option<String>,
    /// Force no cache
    pub no_cache: bool,
    /// Build pack specific variables (e.g., NIXPACKS_NODE_VERSION)
    pub nixpacks_vars: HashMap<String, String>,
    /// Whether this is for a static site
    pub is_static: bool,
    /// Static image to use (e.g., nginx:alpine) for static sites
    pub static_image: Option<String>,
}

/// Nixpacks build plan (parsed from nixpacks plan output)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NixpacksPlan {
    /// Detected providers/app types
    pub providers: Vec<String>,
    /// Build phases
    pub phases: NixpacksPhases,
    /// Variables to be set
    pub variables: HashMap<String, String>,
    /// Start phase configuration
    pub start: Option<NixpacksStartPhase>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NixpacksPhases {
    /// Setup phase (apt packages, etc.)
    pub setup: Option<NixpacksSetupPhase>,
    /// Install phase
    pub install: Option<NixpacksInstallPhase>,
    /// Build phase
    pub build: Option<NixpacksBuildPhase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NixpacksSetupPhase {
    /// Nix packages to install
    #[serde(default)]
    pub nix_pkgs: Vec<String>,
    /// Apt packages to install
    #[serde(rename = "aptPkgs", default)]
    pub apt_pkgs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NixpacksInstallPhase {
    /// Install command
    pub cmd: Option<String>,
    /// Commands to run
    #[serde(default)]
    pub cmds: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NixpacksBuildPhase {
    /// Build command
    pub cmd: Option<String>,
    /// Commands to run
    #[serde(default)]
    pub cmds: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NixpacksStartPhase {
    /// Start command
    pub cmd: Option<String>,
    /// Run image (for multi-stage builds)
    pub run_image: Option<String>,
}

/// Output from Nixpacks build
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NixpacksBuildOutput {
    /// Detected application type
    pub app_type: NixpacksAppType,
    /// Generated Nixpacks plan
    pub plan: NixpacksPlan,
    /// Path to generated Dockerfile
    pub dockerfile_path: String,
    /// Built image name
    pub image_name: String,
    /// Commands that were executed
    pub commands: Vec<String>,
    /// Build successful
    pub success: bool,
    /// Build message/logs
    pub message: String,
}

/// Nixpacks build action
pub struct NixpacksBuild;

impl NixpacksBuild {
    pub fn new() -> Self {
        Self
    }

    /// Generate the nixpacks plan command
    pub fn plan_command(&self, input: &NixpacksBuildInput) -> String {
        let mut cmd = format!("nixpacks plan -f json");

        // Add environment variables
        for (key, value) in &input.env_vars {
            if !value.is_empty() {
                cmd.push_str(&format!(" --env {}={}", key, shell_escape(value)));
            }
        }

        // Add nixpacks-specific variables
        for (key, value) in &input.nixpacks_vars {
            if !value.is_empty() {
                cmd.push_str(&format!(" --env {}={}", key, shell_escape(value)));
            }
        }

        // Add custom commands
        if let Some(ref build_cmd) = input.build_command {
            cmd.push_str(&format!(" --build-cmd \"{}\"", build_cmd));
        }
        if let Some(ref start_cmd) = input.start_command {
            cmd.push_str(&format!(" --start-cmd \"{}\"", start_cmd));
        }
        if let Some(ref install_cmd) = input.install_command {
            cmd.push_str(&format!(" --install-cmd \"{}\"", install_cmd));
        }

        cmd.push_str(&format!(" {}", input.workdir));
        cmd
    }

    /// Generate the nixpacks detect command
    pub fn detect_command(&self, workdir: &str) -> String {
        format!("nixpacks detect {}", workdir)
    }

    /// Generate the nixpacks build command
    pub fn build_command(&self, input: &NixpacksBuildInput, plan_path: &str) -> String {
        let mut cmd = format!(
            "nixpacks build -c {} --no-error-without-start -n {} {}",
            plan_path,
            input.image_name,
            input.workdir
        );

        if input.no_cache {
            cmd = cmd.replace("nixpacks build", "nixpacks build --no-cache");
        }

        cmd.push_str(&format!(" -o {}", input.workdir));
        cmd
    }

    /// Generate docker build command from nixpacks Dockerfile
    pub fn docker_build_command(
        &self,
        input: &NixpacksBuildInput,
        use_buildkit: bool,
        build_secrets: Option<&str>,
    ) -> String {
        let dockerfile_path = format!("{}/.nixpacks/Dockerfile", input.workdir);

        let mut cmd = if use_buildkit {
            "DOCKER_BUILDKIT=1 docker build".to_string()
        } else {
            "docker build".to_string()
        };

        if input.no_cache {
            cmd.push_str(" --no-cache");
        }

        cmd.push_str(" --network host");
        cmd.push_str(&format!(" -f {}", dockerfile_path));

        if let Some(secrets) = build_secrets {
            cmd.push_str(secrets);
        }

        cmd.push_str(" --progress plain");
        cmd.push_str(&format!(" -t {}", input.image_name));
        cmd.push_str(&format!(" {}", input.workdir));

        cmd
    }

    /// Apply Laravel-specific finetunes to the plan
    pub fn laravel_finetunes(&self, plan: &mut NixpacksPlan) {
        // Set PHP fallback path for Laravel
        plan.variables.entry("NIXPACKS_PHP_FALLBACK_PATH".to_string())
            .or_insert("/index.php".to_string());

        // Set PHP root directory
        plan.variables.entry("NIXPACKS_PHP_ROOT_DIR".to_string())
            .or_insert("/app/public".to_string());
    }

    /// Apply Node.js-specific finetunes to the plan
    pub fn node_finetunes(&self, plan: &mut NixpacksPlan) {
        // Warn if NIXPACKS_NODE_VERSION is not set (default is Node 18 which is EOL)
        if !plan.variables.contains_key("NIXPACKS_NODE_VERSION") {
            tracing::warn!(
                "NIXPACKS_NODE_VERSION not set. Nixpacks will use Node.js 18 by default, which is EOL. \
                 Consider setting NIXPACKS_NODE_VERSION=22 in environment variables."
            );
        }
    }

    /// Ensure required apt packages are installed
    pub fn ensure_apt_packages(&self, plan: &mut NixpacksPlan, packages: &[&str]) {
        if let Some(ref mut setup) = plan.phases.setup {
            for pkg in packages {
                if !setup.apt_pkgs.contains(&pkg.to_string()) {
                    setup.apt_pkgs.push(pkg.to_string());
                }
            }
        } else {
            plan.phases.setup = Some(NixpacksSetupPhase {
                nix_pkgs: Vec::new(),
                apt_pkgs: packages.iter().map(|s| s.to_string()).collect(),
            });
        }
    }

    /// Generate all build commands
    pub fn generate_commands(&self, input: &NixpacksBuildInput) -> Vec<String> {
        let mut builder = CommandBuilder::new();
        let plan_path = format!("{}/.nixpacks/plan.json", input.workdir);

        // Ensure directories exist
        builder.mkdir(format!("{}/.nixpacks", input.workdir));

        // Generate plan
        builder.add(self.plan_command(input));

        // Detect app type
        builder.add(self.detect_command(&input.workdir));

        // Build with nixpacks
        builder.add(self.build_command(input, &plan_path));

        // Docker build from generated Dockerfile
        builder.add(self.docker_build_command(input, true, None));

        builder.build()
    }
}

impl Default for NixpacksBuild {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Action for NixpacksBuild {
    type Input = NixpacksBuildInput;
    type Output = NixpacksBuildOutput;

    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        tracing::info!(
            application_id = %input.application_id,
            workdir = %input.workdir,
            image = %input.image_name,
            "Starting Nixpacks build"
        );

        // Generate commands (would be executed via SSH in real implementation)
        let commands = self.generate_commands(&input);

        // Placeholder for actual execution
        // In real implementation:
        // 1. Execute plan command, parse JSON output
        // 2. Execute detect command, get app type
        // 3. Apply finetunes based on app type
        // 4. Execute build command
        // 5. Execute docker build
        // 6. Return results

        let plan = NixpacksPlan {
            providers: vec!["node".to_string()],
            phases: NixpacksPhases::default(),
            variables: input.env_vars.clone(),
            start: Some(NixpacksStartPhase {
                cmd: input.start_command.clone(),
                run_image: None,
            }),
        };

        Ok(NixpacksBuildOutput {
            app_type: NixpacksAppType::Node,
            plan,
            dockerfile_path: format!("{}/.nixpacks/Dockerfile", input.workdir),
            image_name: input.image_name,
            commands,
            success: true,
            message: "Nixpacks build initiated".to_string(),
        })
    }

    fn name(&self) -> &'static str {
        "nixpacks_build"
    }
}

/// Shell escape a value for command line
fn shell_escape(value: &str) -> String {
    if value.contains(|c: char| c.is_whitespace() || c == '\'' || c == '"' || c == '$' || c == '\\') {
        format!("'{}'", value.replace('\'', "'\\''"))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_command() {
        let action = NixpacksBuild::new();
        let mut env_vars = HashMap::new();
        env_vars.insert("NODE_ENV".to_string(), "production".to_string());

        let input = NixpacksBuildInput {
            application_id: Uuid::new_v4(),
            workdir: "/app".to_string(),
            image_name: "my-app:latest".to_string(),
            env_vars,
            build_command: Some("npm run build".to_string()),
            start_command: Some("npm start".to_string()),
            install_command: None,
            no_cache: false,
            nixpacks_vars: HashMap::new(),
            is_static: false,
            static_image: None,
        };

        let cmd = action.plan_command(&input);
        assert!(cmd.contains("nixpacks plan -f json"));
        assert!(cmd.contains("NODE_ENV=production"));
        assert!(cmd.contains("--build-cmd"));
        assert!(cmd.contains("--start-cmd"));
    }

    #[test]
    fn test_app_type_from_string() {
        assert_eq!(NixpacksAppType::from("node"), NixpacksAppType::Node);
        assert_eq!(NixpacksAppType::from("NodeJS"), NixpacksAppType::Node);
        assert_eq!(NixpacksAppType::from("python"), NixpacksAppType::Python);
        assert_eq!(NixpacksAppType::from("rust"), NixpacksAppType::Rust);
        assert_eq!(NixpacksAppType::from("unknown_type"), NixpacksAppType::Unknown);
    }
}
