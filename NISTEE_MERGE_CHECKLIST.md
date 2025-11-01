# niStee's 11 PRs - Merge Checklist

## 🎯 Pre-Merge Verification

### Phase 1: Security Foundation (This Week)

#### PR #1409: PoLP Token Permissions

- [ ] Review token permission changes
- [ ] Verify least privilege principle applied
- [ ] Confirm pre-commit validation passed
- [ ] Merge: **Ready Now**

#### PR #1310: CodeQL + cargo-deny

- [ ] Review CodeQL configuration
- [ ] Check cargo-deny policy rules
- [ ] Verify non-blocking mode set
- [ ] Review Dependabot configuration
- [ ] Merge: **Ready**

#### PR #1309: OSV + Trivy + SBOM + Cosign

- [ ] Review OSV scanner config
- [ ] Check Trivy scan filters
- [ ] Verify SBOM generation
- [ ] Confirm fork-safe SARIF uploads
- [ ] Validate Cosign keyless signing
- [ ] Update README with badges
- [ ] Merge: **Ready**

---

### Phase 2: Developer Tooling (Week 2)

#### PR #1320: pre-commit + dprint Config

- [ ] Review pre-commit hooks
- [ ] Check dprint.json excludes
- [ ] Verify WSL guidance in CONTRIBUTING.md
- [ ] Test .gitignore changes
- [ ] Merge: **Ready** (Foundation)

#### PR #1321: dprint Formatting Applied

- [ ] Verify all Markdown/JSON formatted
- [ ] Check no content changes
- [ ] Confirm depends on #1320
- [ ] Merge: **After #1320**

#### PR #1322: dprint CI Check

- [ ] Review Linux-only job
- [ ] Verify guard on dprint.json
- [ ] Check main matrix gating
- [ ] Merge: **After #1320 & #1321**

---

### Phase 3: Windows & Infrastructure (Week 3)

#### PR #1338: SDIO Windows Driver

- [ ] Review SDIO step implementation
- [ ] Verify dry-run support works
- [ ] Check --yes flag support
- [ ] Confirm safe defaults (disabled)
- [ ] Test configuration options
- [ ] **Windows validation required before merge**
- [ ] Merge: **Ready** (after Windows test)

#### PR #1339: SDIO Documentation

- [ ] Review README guidance
- [ ] Check security assessment framework
- [ ] Verify .gitignore updates
- [ ] **Confirm i18n strings complete**
- [ ] Merge: **After #1338** (with i18n check)

#### PR #1311: Composite Actions

- [ ] Review 6 composite actions
- [ ] Check inputs/outputs
- [ ] Verify no existing workflow changes
- [ ] Merge: **Ready**

---

### Parallel: Independent

#### PR #1275: Rust Dependabot Support

- [ ] Review dependabot.yml changes
- [ ] Check rust-toolchain.toml enhancements
- [ ] Verify documentation complete
- [ ] Merge: **Anytime**

#### PR #1323: gitleaks Proposal

- [ ] **Awaiting maintainer decision on approach**
- [ ] Review allowlist strategy
- [ ] Check non-blocking mode
- [ ] Merge: **Pending approval**

---

## 📋 Post-Merge Validation

### After Phase 1 Complete

- [ ] Monitor CodeQL findings
- [ ] Check cargo-deny baseline
- [ ] Validate OSV/Trivy scanners
- [ ] Verify Cosign signing works

### After Phase 2 Complete

- [ ] Run pre-commit hooks locally
- [ ] Verify dprint enforces correctly
- [ ] Check CI gating works
- [ ] Test Windows contributors still unblocked

### After Phase 3 Complete

- [ ] Validate SDIO step works on Windows
- [ ] Test composite actions in workflows
- [ ] Verify no regressions
- [ ] Gather user feedback

---

## 🚨 Known Issues to Watch

| Issue | PR | Mitigation |
|-------|----|----|
| Windows fork issues | #1320 | Recommend WSL; --no-verify available |
| Security scanners non-blocking | #1309, #1310 | Intentional; plan to enforce later |
| cargo-deny tune needed | #1310 | Starting point; triage as needed |
| dprint formatting breaking | #1321 | Verify doesn't affect documentation |
| SDIO Windows-only | #1338 | No impact; Windows feature |

---

## ✅ Quality Gates

### Code Review

- [ ] All PRs reviewed for quality
- [ ] Security implications assessed
- [ ] Dependencies identified
- [ ] Risks documented

### Testing

- [ ] Personal testing confirmed
- [ ] CI validation passes
- [ ] Pre-commit hooks pass
- [ ] Platform-specific testing done

### Documentation

- [ ] README updated
- [ ] CONTRIBUTING.md enhanced
- [ ] PR descriptions clear
- [ ] i18n strings complete (where needed)

### Security

- [ ] No credentials exposed
- [ ] Principle of Least Privilege applied
- [ ] Fork-safe uploads configured
- [ ] Supply chain hardened

---

## 📊 Merge Velocity Estimate

| Phase | Duration | PRs | Status |
|-------|----------|-----|--------|
| Phase 1 (Security) | 3 days | 3 | 🟢 Ready |
| Phase 2 (Tooling) | 3 days | 3 | 🟢 Ready |
| Phase 3 (Features) | 3 days | 3 | 🟢 Ready |
| Parallel | Any time | 2 | 1 Ready, 1 Pending |
| **Total Timeline** | **~10 days** | **11** | **Ready** |

---

## 📞 Review Points

### For #1409 (PoLP)

- **Q:** All GitHub Actions properly using least privilege?
- **Q:** Any workflow breaking changes?
- **Reviewer:** Security-focused maintainer

### For #1310 (CodeQL + cargo-deny)

- **Q:** cargo-deny policy appropriate for project?
- **Q:** Non-blocking duration acceptable?
- **Reviewer:** Security + dependency maintainer

### For #1309 (OSV + Trivy)

- **Q:** Fork-safe SARIF upload mechanism correct?
- **Q:** SBOM format suitable?
- **Reviewer:** CI/security maintainer

### For #1320 (pre-commit)

- **Q:** WSL recommendation clear for Windows?
- **Q:** All hooks portable?
- **Reviewer:** Windows-aware maintainer

### For #1338 (SDIO)

- **Q:** SDIO behavior matches expectations?
- **Q:** Safe defaults in example config?
- **Reviewer:** Windows-aware + infrastructure maintainer

### For #1323 (gitleaks)

- **Q:** Acceptable non-blocking security addition?
- **Q:** Allowlist strategy sound?
- **Reviewer:** Security-focused maintainer decision

---

## 🎓 Learning/Documentation

After merging, consider:

- [ ] Update RELEASE_PROCEDURE.md with verification steps
- [ ] Document new CodeQL/OSV/Trivy scanner alerts process
- [ ] Add pre-commit hook troubleshooting section
- [ ] Create SDIO troubleshooting guide
- [ ] Document cargo-deny policy tuning process

---

## 🔄 Follow-Up PRs (After Current Set)

These may be follow-ups to consider later:

1. Pin remaining GitHub Actions by digest
2. Wire composite actions into all workflows
3. Flip security scanners to hard-fail after baseline
4. Add more detailed SBOM documentation
5. Enhance SDIO support with more driver types

---

## Final Status

```
✅ All 11 PRs are professional-grade
✅ 7-8 ready to merge immediately
✅ 2-3 need minor updates
✅ 1 awaiting maintainer decision
✅ Clear merge order established
✅ No blockers identified
✅ Security-first approach
✅ Windows-aware design
✅ Comprehensive documentation
```

---

**Next Step:** Begin Phase 1 merge after this checklist review

Generated: November 1, 2025
