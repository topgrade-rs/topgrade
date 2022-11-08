use anyhow::{Context, Result};

use crate::command::CommandExt;
use crate::error::{self, TopgradeError};
use crate::terminal::print_separator;
use crate::{execution_context::ExecutionContext, utils::require};
use log::{debug, error, warn};
use std::path::Path;
use std::process::Command;

// A string found in the output of docker for containers that weren't found in
// the docker registry. We use this to gracefully handle and skip containers
// that cannot be pulled, likely because they don't exist in the registry in
// the first place. This happens e.g. when the user tags an image locally
// themselves or when using docker-compose.
const NONEXISTENT_REPO: &str = "repository does not exist";

/// Returns a Vector of all containers, with Strings in the format
/// "REGISTRY/[PATH/]CONTAINER_NAME:TAG"
fn list_containers(crt: &Path) -> Result<Vec<String>> {
    debug!(
        "Querying '{} image ls --format \"{{{{.Repository}}}}:{{{{.Tag}}}}\"' for containers",
        crt.display()
    );
    let output = Command::new(crt)
        .args(["image", "ls", "--format", "{{.Repository}}:{{.Tag}}"])
        .output_checked_with_utf8(|_| Ok(()))?;

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
        retval.push(String::from(line));
    }

    Ok(retval)
}

pub fn run_containers(ctx: &ExecutionContext) -> Result<()> {
    // Prefer podman, fall back to docker if not present
    let crt = require("podman").or_else(|_| require("docker"))?;
    debug!("Using container runtime '{}'", crt.display());

    print_separator("Containers");
    let mut success = true;
    let containers = list_containers(&crt).context("Failed to list Docker containers")?;
    debug!("Containers to inspect: {:?}", containers);

    for container in containers.iter() {
        debug!("Pulling container '{}'", container);
        let args = vec!["pull", &container[..]];
        let mut exec = ctx.run_type().execute(&crt);

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

            success = false;
        }
    }

    if ctx.config().cleanup() {
        // Remove dangling images
        debug!("Removing dangling images");
        if let Err(e) = ctx
            .run_type()
            .execute(&crt)
            .args(["image", "prune", "-f"])
            .status_checked()
        {
            error!("Removing dangling images failed: {}", e);
            success = false;
        }
    }

    if success {
        Ok(())
    } else {
        Err(anyhow::anyhow!(error::StepFailed))
    }
}
