# ci(security): add gitleaks secret scanning with conservative baseline (non-blocking initially)

## Summary

Propose adding a small CI workflow to run gitleaks on PRs and pushes to `main` with a conservative allowlist/baseline. This complements existing security checks (DevSkim, dependency-review, scorecards) and keeps local developer ergonomics intact (our pre-commit config already exposes gitleaks as a manual hook).

Key characteristics:

- Scope: new GitHub Actions workflow only (Linux runner), pinned action versions, least privileges.
- Mode: non-blocking initially (informational) to let us tune the allowlist/baseline; option to flip to blocking later.
- Redaction: enabled so findings are redacted in logs; avoids leaking sensitive content.
- Allowlist/baseline: start with a narrow `.gitleaksignore` + path ignores (e.g., locales, OS-release fingerprints, images, vendored content). Optionally seed a baseline file if needed to mute known benign matches.

## Rationale

- Shift-left detection: catches credentials before merge without relying on every contributor to run local scans.
- Low friction: runs on Ubuntu only, informational at first, and uploads results to PR checks (and/or SARIF to code scanning) to gather signal.
- Safe by default: redaction on; conservative allowlist prevents noisy false positives.

## Proposed scope (no code in this PR)

- Add a workflow (e.g., `.github/workflows/secret-scan-gitleaks.yml`) that:
  - Triggers on `pull_request` and `push` to `main`.
  - Uses `actions/checkout@v5`.
  - Runs `gitleaks` (pinned version) in `--redact` mode against the repo.
  - Honors a repo-level allowlist: `.gitleaksignore` and (optionally) a baseline file (e.g., `.gitleaks.baseline.json`).
  - Uploads results as an artifact and/or SARIF (optional) for code scanning.
  - Initially does not fail the job on findings (non-blocking) while we tune.

No changes to application code or existing workflows in this PR. Actual workflow YAML and allowlist/baseline files would land in a follow-up PR.

## Allowlist/baseline strategy

- Start with minimal ignores for known benign areas:
  - `locales/**` (translations), `src/steps/os/os_release/**` (fingerprints), `**/*.png`, `**/*.gif`, `doc/**/assets/**`, vendored caches.
- Prefer path-based ignores and narrowly-scoped regexes over broad disables.
- If we observe persistent false positives in code, add specific regex allow rules with comments.
- Optionally generate a baseline snapshot to suppress legacy matches; we’ll review and prune it over time.

## How to test (once workflow exists)

- Open a PR with a known fake token pattern to verify detection (e.g., a dummy AWS-style key that gitleaks can recognize), then remove it.
- Verify redaction in logs and that ignores apply to `locales/**` and OS-release fingerprints.
- Confirm the job remains non-blocking while we tune.

## Maintainer options

- Keep non-blocking (informational) while allowlist/baseline stabilizes.
- Flip to blocking after a short trial period by enabling failure on findings and making the job a required check.
- Alternatively, keep only local/manual scanning (status quo) if CI scanning isn’t desired.

## Checklist

- [x] Separate PR for security CI proposal (no code changes here)
- [x] Conservative, redacted, Ubuntu-only run described
- [x] Allowlist/baseline approach documented
- [x] Non-blocking initially; clear path to blocking later
- [x] Aligns with current practice: gitleaks remains manual in pre-commit

## Additional context

- Existing security workflows: DevSkim (`check_security_vulnerability.yml`), dependency review, scorecards. gitleaks is not currently present in CI.
- This proposal avoids interaction with formatting or developer tooling PRs (#1320/#1321) to keep scopes clean.
