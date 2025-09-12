# Release Procedure

> This document lists the steps that lead to a successful release of Topgrade.

1. Open a PR that:

   > Here is an [Example PR](https://github.com/topgrade-rs/topgrade/pull/652)
   > that you can refer to.

   1. bumps the version number.

      > If there are breaking changes, the major version number should be increased.

   2. If the major versioin number gets bumped, update [SECURITY.md][SECURITY_file_link].

      [SECURITY_file_link]: https://github.com/topgrade-rs/topgrade/blob/main/SECURITY.md

   3. Overwrite [`BREAKINGCHANGES`][breaking_changes] with
      [`BREAKINGCHANGES_dev`][breaking_changes_dev], and create a new dev file:

   ```sh
   cd topgrade
   mv BREAKINGCHANGES_dev.md BREAKINGCHANGES.md
   touch BREAKINGCHANGES_dev.md
   ```

      [breaking_changes_dev]: https://github.com/topgrade-rs/topgrade/blob/main/BREAKINGCHANGES_dev.md
      [breaking_changes]: https://github.com/topgrade-rs/topgrade/blob/main/BREAKINGCHANGES.md

2. Check and merge that PR.

3. Go to the [release](https://github.com/topgrade-rs/topgrade/releases) page
   and click the [Draft a new release button](https://github.com/topgrade-rs/topgrade/releases/new)

4. Write the release notes

   We usually use GitHub's [Automatically generated release notes][auto_gen_release_notes]
   functionality to generate release notes, but you write your own one instead.

   [auto_gen_release_notes]: https://docs.github.com/en/repositories/releasing-projects-on-github/automatically-generated-release-notes

5. Attaching binaries

   You don't need to do this as our CI will automatically do it for you,
   binaries for Linux, macOS and Windows will be created and attached.

   And the CI will publish the new binary to:

   1. AUR
   2. PyPi
   3. Homebrew (seems that this is not working correctly)
   4. Winget

6. Manually release it to Crates.io

   > Yeah, this is unfortunate, our CI won't do this for us. We should probably add one.

   1. `cd` to the Topgrade directory, make sure that it is the latest version
      (i.e., including the PR that bumps the version number).
   2. Set up your token with `cargo login`.
   3. Dry-run the publish `cargo publish --dry-run`.
   4. If step 3 works, then do the final release `cargo publish`.

   > You can also take a look at the official tutorial [Publishing on crates.io][doc]
   >
   > [doc]: https://doc.rust-lang.org/cargo/reference/publishing.html

---

## Verifying release artifacts

Starting from recent releases, each asset published by CI includes:

- Per-file Cosign signatures: `<asset>.sig` and certificate `<asset>.crt`
- A combined checksum manifest: `SHA256SUMS`
- Cosign signature and certificate for the manifest: `SHA256SUMS.sig` and `SHA256SUMS.crt`
- A CycloneDX SBOM: `sbom.cdx.json`

You can verify integrity and provenance as follows.

### 1. Verify checksums

- Linux/macOS:

   ```sh
   # In the directory where you downloaded the assets
   sha256sum -c SHA256SUMS  # or: shasum -a 256 -c SHA256SUMS
   ```

- Windows (PowerShell):

   ```powershell
   Get-Content .\SHA256SUMS | ForEach-Object {
      $parts = $_ -split "\s+"; $expected=$parts[0]; $file=$parts[-1]
      $actual = (Get-FileHash -Algorithm SHA256 $file).Hash.ToLower()
      if ($actual -eq $expected) { "OK `t $file" } else { "FAIL`t $file" }
   }
   ```

### 2. Verify Cosign signatures (keyless)

Cosign uses GitHub OIDC for keyless signing. You do not need our private keys.

- Verify a single asset (Linux/macOS/Windows with cosign installed):

   ```sh
   cosign verify-blob \
      --signature topgrade-<tag>-<triple>.tar.gz.sig \
      --certificate topgrade-<tag>-<triple>.tar.gz.crt \
      topgrade-<tag>-<triple>.tar.gz
   ```

- Verify the checksum manifest (recommended):

   ```sh
   cosign verify-blob \
      --signature SHA256SUMS.sig \
      --certificate SHA256SUMS.crt \
      SHA256SUMS
   ```

Cosign will validate the Fulcio certificate chain and Rekor inclusion. You can add issuer/identity filters for stricter checks, for example:

```sh
cosign verify-blob \
   --certificate-oidc-issuer https://token.actions.githubusercontent.com \
   --certificate-identity-regexp 'https://github.com/topgrade-rs/topgrade/.+' \
   --signature SHA256SUMS.sig \
   --certificate SHA256SUMS.crt \
   SHA256SUMS
```

### 3. Inspect the SBOM

The `sbom.cdx.json` file lists components and licenses in CycloneDX format. You can view it with any CycloneDX viewer or a JSON viewer.

