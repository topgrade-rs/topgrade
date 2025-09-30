# Rust Toolchain Dependabot Integration

This document describes the Dependabot integration for automatic Rust toolchain updates in the Topgrade project.

## Overview

As of August 19, 2025, GitHub Dependabot now supports automatic updates for Rust toolchain versions defined in `rust-toolchain.toml` and `rust-toolchain` files. This integration helps keep the project up-to-date with the latest stable Rust releases.

## Configuration

### Dependabot Configuration (`.github/dependabot.yml`)

The Dependabot configuration includes a `rust-toolchain` package ecosystem entry:

```yaml
- package-ecosystem: "rust-toolchain"
  directory: "/"
  schedule:
    interval: "weekly"
    day: "tuesday"
    time: "06:00"
    timezone: "UTC"
  labels: ["dependencies", "rust-toolchain"]
  commit-message:
    prefix: "deps(rust)"
    include: "scope"
```

### Rust Toolchain Configuration (`rust-toolchain.toml`)

The toolchain configuration specifies:

```toml
[toolchain]
channel = "1.84.1"
components = ["rustfmt", "clippy"]
targets = ["x86_64-unknown-linux-gnu", "x86_64-apple-darwin", "aarch64-apple-darwin", "x86_64-pc-windows-msvc"]
```

## Features

### Supported Update Patterns

Dependabot supports updating:

- Versioned toolchains (e.g., `channel = "1.xx.yy"`, `channel = "1.xx"`)
- Dated toolchains (e.g., `channel = "nightly-YYYY-MM-DD"`, `channel = "beta-YYYY-MM-DD"`)

### Benefits

1. **Automatic Updates**: Dependabot will create PRs when new Rust versions are available
2. **Consistency**: Ensures all team members and CI environments use the same Rust version
3. **Security**: Helps keep the project updated with the latest security patches
4. **Reliability**: Maintains compatibility with the specified components and targets

## GitHub Actions Integration

The current CI workflows (`.github/workflows/ci.yml`) automatically respect the toolchain configuration:

- The `rust-toolchain.toml` file is automatically detected by `rustup`
- Components like `rustfmt` and `clippy` are explicitly installed in CI jobs
- Multi-target builds use the specified targets from the toolchain file

## Best Practices

1. **Pin to Specific Versions**: Use specific version numbers (e.g., "1.84.1") rather than channels (e.g., "stable") for reproducible builds
2. **Include Required Components**: Specify `rustfmt` and `clippy` in the components list
3. **Specify Targets**: Include all compilation targets used in CI/CD
4. **Regular Updates**: Weekly update schedule balances freshness with stability
5. **Review PRs**: Always review and test Dependabot PRs before merging

## Testing Toolchain Updates

When Dependabot creates a PR for a toolchain update:

1. Verify that all CI checks pass
2. Test locally with the new toolchain version
3. Check for any breaking changes in the Rust release notes
4. Validate that all specified components and targets work correctly

## Troubleshooting

### Common Issues

1. **Component Not Available**: Some Rust versions may not have all components available
   - Solution: Temporarily remove the component or wait for availability
   
2. **Target Not Supported**: New Rust versions may drop support for older targets
   - Solution: Update CI matrix or remove unsupported targets

3. **Breaking Changes**: Major Rust updates may introduce breaking changes
   - Solution: Review release notes and update code as needed

### Validation Commands

```bash
# Check if toolchain is properly installed
rustup show

# Verify components are available
rustup component list --installed

# Test compilation for all targets
cargo check --all-targets
```

## References

- [GitHub Blog: Dependabot now supports Rust toolchain updates](https://github.blog/changelog/2025-08-19-dependabot-now-supports-rust-toolchain-updates/)
- [Rust Toolchain File Documentation](https://rust-lang.github.io/rustup/overrides.html#the-toolchain-file)
- [Dependabot Options Reference](https://docs.github.com/code-security/dependabot/working-with-dependabot/dependabot-options-reference)
