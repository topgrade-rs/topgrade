use std::fmt::{Display, Formatter};
use std::io;
use std::io::Write;
use std::path::Path;
use std::process::Command;

use color_eyre::eyre::Context;
use color_eyre::eyre::Result;
use color_eyre::eyre::{eyre, OptionExt};
use tracing::{debug, error, warn};
use wildmatch::WildMatch;

use crate::command::CommandExt;
use crate::error::{self, SkipStep, TopgradeError};
use crate::terminal::print_separator;
use crate::{execution_context::ExecutionContext, utils::require};
use rust_i18n::t;

// A string found in the output of docker for containers that weren't found in
// the docker registry. We use this to gracefully handle and skip containers
// that cannot be pulled, likely because they don't exist in the registry in
// the first place. This happens e.g. when the user tags an image locally
// themselves or when using docker-compose.
const NONEXISTENT_REPO: &str = "repository does not exist";

// A string found in the output of docker when Docker Desktop is not running.
const DOCKER_NOT_RUNNING: &str = "We recommend to activate the WSL integration in Docker Desktop settings.";

/// Uniquely identifies a `Container`.
#[derive(Debug)]
struct Container {
    /// `Repository` and `Tag`
    ///
    /// format: `Repository:Tag`, e.g., `nixos/nix:latest`.
    repo_tag: String,
    /// Platform
    ///
    /// format: `OS/Architecture`, e.g., `linux/amd64`.
    platform: String,
}

impl Container {
    /// Construct a new `Container`.
    fn new(repo_tag: String, platform: String) -> Self {
        Self { repo_tag, platform }
    }
}

impl Display for Container {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // e.g., "`fedora:latest` for `linux/amd64`"
        write!(
            f,
            "{}",
            t!(
                "`{repo_tag}` for `{platform}`",
                repo_tag = self.repo_tag,
                platform = self.platform
            )
        )
    }
}

/// Returns a Vector of all containers, with Strings in the format
/// "REGISTRY/[PATH/]CONTAINER_NAME:TAG"
///
/// Containers specified in `ignored_containers` will be filtered out.
fn list_containers(crt: &Path, ignored_containers: Option<&Vec<String>>) -> Result<Vec<Container>> {
    let ignored_containers = ignored_containers.map(|patterns| {
        patterns
            .iter()
            .map(|pattern| WildMatch::new(pattern))
            .collect::<Vec<WildMatch>>()
    });

    debug!(
        "Querying '{} image ls --format \"{{{{.Repository}}}}:{{{{.Tag}}}}/{{{{.ID}}}}\"' for containers",
        crt.display()
    );
    let output = Command::new(crt)
        .args(["image", "ls", "--format", "{{.Repository}}:{{.Tag}} {{.ID}}"])
        .output_checked_utf8()?;

    let mut retval = vec![];
    for line in output.stdout.lines() {
        if line.starts_with("localhost") {
            // Don't know how to update self-built containers
            debug!("Skipping self-built container '{}'", line);
            continue;
        }

        if line.contains("<none>") {
            // Bogus/dangling container or intermediate layer
            debug!("Skipping bogus container '{}'", line);
            continue;
        }

        if line.starts_with("vsc-") {
            debug!("Skipping visual studio code dev container '{}'", line);
            continue;
        }

        debug!("Using container '{}'", line);

        // line is of format: `Repository:Tag ImageID`, e.g., `nixos/nix:latest d80fea9c32b4`
        let split_res = line.split(' ').collect::<Vec<&str>>();
        if split_res.len() != 2 {
            return Err(eyre!(format!(
                "Got erroneous output from `{} image ls --format \"{{.Repository}}:{{.Tag}} {{.ID}}\"; Expected line to split into 2 parts",
                crt.display()
            )));
        }
        let (repo_tag, image_id) = (split_res[0], split_res[1]);

        if let Some(ref ignored_containers) = ignored_containers {
            if ignored_containers.iter().any(|pattern| pattern.matches(repo_tag)) {
                debug!("Skipping ignored container '{}'", line);
                continue;
            }
        }

        debug!(
            "Querying '{} image inspect --format \"{{{{.Os}}}}/{{{{.Architecture}}}}\"' for container {}",
            crt.display(),
            image_id
        );
        let inspect_output = Command::new(crt)
            .args(["image", "inspect", image_id, "--format", "{{.Os}}/{{.Architecture}}"])
            .output_checked_utf8()?;
        let mut platform = inspect_output.stdout;
        // truncate the tailing new line character
        platform.truncate(platform.len() - 1);
        if !platform.contains('/') {
            return Err(eyre!(format!(
                "Got erroneous output from `{} image ls --format \"{{.Repository}}:{{.Tag}} {{.ID}}\"; Expected platform to contain '/'",
                crt.display()
            )));
        }

        retval.push(Container::new(repo_tag.to_string(), platform));
    }

    Ok(retval)
}

pub fn run_containers(ctx: &ExecutionContext) -> Result<()> {
    // Check what runtime is specified in the config
    let container_runtime = ctx.config().containers_runtime().to_string();
    let crt = require(container_runtime)?;
    debug!("Using container runtime '{}'", crt.display());

    print_separator(t!("Containers"));

    let output = Command::new(&crt).arg("--help").output_checked_with(|_| Ok(()))?;
    let status_code = output
        .status
        .code()
        .ok_or_eyre("Couldn't get status code (terminated by signal)")?;
    let stdout = std::str::from_utf8(&output.stdout).wrap_err("Expected output to be valid UTF-8")?;
    if stdout.contains(DOCKER_NOT_RUNNING) && status_code == 1 {
        // Write the output
        io::stdout().write_all(&output.stdout)?;
        io::stderr().write_all(&output.stderr)?;
        // Don't crash, but don't be silent either.
        // This can happen in other ways than Docker Desktop not running, but even in those cases
        //  we don't want to crash, since the containers step is enabled by default.
        warn!(
            "{} seems to be non-functional right now (see above). Is WSL integration enabled for Docker Desktop? Is Docker Desktop running?",
            crt.display()
        );
        return Err(SkipStep(format!(
            "{} seems to be non-functional right now. Possibly WSL integration is not enabled for Docker Desktop, or Docker Desktop is not running.",
            crt.display()
        )).into());
    } else if !output.status.success() {
        // Write the output
        io::stdout().write_all(&output.stdout)?;
        io::stderr().write_all(&output.stderr)?;
        // If we saw the message, but the code is not 1 (e.g. 0, or a non-1 failure), crash, as we expect a 1.
        // If we did not see the message, it's broken in some way we do not understand.
        return Err(eyre!(
            "{0} seems to be non-functional (`{0} --help` returned non-zero exit code {1})",
            crt.display(),
            status_code,
        ));
    }

    let containers =
        list_containers(&crt, ctx.config().containers_ignored_tags()).context("Failed to list Docker containers")?;
    debug!("Containers to inspect: {:?}", containers);

    for container in &containers {
        debug!("Pulling container '{}'", container);
        let mut args = vec!["pull", container.repo_tag.as_str()];
        if container.platform.as_str() != "/" {
            args.push("--platform");
            args.push(container.platform.as_str());
        }

        let mut exec = ctx.execute(&crt);

        if let Err(e) = exec.args(&args).status_checked() {
            error!("Pulling container '{}' failed: {}", container, e);

            // Find out if this is 'skippable'
            // This is necessary e.g. for docker, because unlike podman docker doesn't tell from
            // which repository a container originates (such as `docker.io`). This has the
            // practical consequence that all containers, whether self-built, created by
            // docker-compose or pulled from the docker hub, look exactly the same to us. We can
            // only find out what went wrong by manually parsing the output of the command...
            if match exec.output_checked_utf8() {
                Ok(s) => s.stdout.contains(NONEXISTENT_REPO) || s.stderr.contains(NONEXISTENT_REPO),
                Err(e) => match e.downcast_ref::<TopgradeError>() {
                    Some(TopgradeError::ProcessFailedWithOutput(_, _, stderr)) => stderr.contains(NONEXISTENT_REPO),
                    _ => false,
                },
            } {
                warn!("Skipping unknown container '{}'", container);
                continue;
            }

            return Err(e);
        }
    }

    if ctx.config().containers_system_prune() {
        // Run system prune to clean up unused containers, networks, and build cache
        ctx.execute(&crt)
            .args(["system", "prune", "--force"])
            .status_checked()?
    // Only run `image prune` if we don't run `system prune`
    } else if ctx.config().cleanup() {
        // Remove dangling images
        debug!("Removing dangling images");
        ctx.execute(&crt).args(["image", "prune", "-f"]).status_checked()?
    }

    Ok(())
}
