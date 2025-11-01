# 🚀 PR #1310 - EXECUTION PLAN (Following Best Practices)

**Decision:** ✅ YES - Proceed with merge  
**Reason:** These changes are essential security improvements not yet in main  
**Timeline:** 45-60 minutes total  
**Status:** Ready to execute  

---

## 📋 5-Step Execution Plan

### STEP 1: Resolve Workflow Conflicts (10 minutes)

**Branch:** `pr-1310` (already checked out)  
**Files to fix:** 2

- `.github/workflows/ci.yml`
- `.github/workflows/create_release_assets.yml`

**Command:**

```powershell
# Back to pr-1310 branch
git checkout pr-1310
git merge main --no-ff
```

**Conflicts will appear:** Use **CONFLICT_FIXES.md** for exact solutions

**After fixing both files:**

```powershell
git add .github/workflows/ci.yml
git add .github/workflows/create_release_assets.yml
git status
# Should show both files staged
```

---

### STEP 2: Investigate Large Deletions (20 minutes)

**Why:** Understand if these changes are intentional or side effects

**Commands:**

```powershell
# View deletions in key files
git diff main..pr-1310 -- src/sudo.rs | Select-Object -First 100
git diff main..pr-1310 -- src/main.rs | Select-Object -First 100
git diff main..pr-1310 -- locales/app.yml | Select-Object -First 50

# Check Cargo.toml for dependency changes
git diff main..pr-1310 -- Cargo.toml
```

**Questions to answer:**

- [ ] Is src/sudo.rs refactoring intentional?
- [ ] Is src/main.rs refactoring intentional?
- [ ] Are translations being removed intentionally?
- [ ] Are dependency changes necessary?

**Decision points:**

- ✅ If intentional → Keep everything, document why
- ❌ If accidental → Revert those files, keep security changes
- ⚠️ If unclear → Ask niStee or maintainers

---

### STEP 3: Local Verification (10 minutes)

**Test that everything builds:**

```powershell
# Clean build
cargo clean
cargo build

# Run tests
cargo test

# Check formatting
cargo fmt --all -- --check

# Lint check
cargo clippy --all-targets -- -D warnings
```

**Expected outcome:**

- ✅ Build succeeds
- ✅ Tests pass
- ✅ No format issues
- ✅ No clippy warnings

**If anything fails:**

- Check git status
- Review diffs again
- Ask for help if needed

---

### STEP 4: Commit & Push (5 minutes)

**If all deletions are intentional:**

```powershell
git commit -m "Merge main: resolve workflow conflicts and add security scanning

- Add CodeQL for static analysis
- Add cargo-deny for dependency policy
- Add workflow hardening (timeouts, checkout security)
- Resolve conflicts with current main branch
- Includes refactoring: src/sudo.rs, src/main.rs optimization"

git push -f origin pr-1310
```

**If some deletions should be reverted:**

```powershell
# Revert specific files to main version
git checkout main -- src/sudo.rs
git checkout main -- locales/app.yml

# Stage the reverts
git add src/sudo.rs locales/app.yml

# Commit
git commit -m "Merge main: resolve conflicts, keep security changes only

- Add CodeQL for static analysis
- Add cargo-deny for dependency policy
- Add workflow hardening (timeouts, checkout security)
- Resolve conflicts with current main
- Revert unrelated refactoring changes"

git push -f origin pr-1310
```

---

### STEP 5: Monitor & Verify (5-10 minutes)

**GitHub Actions will run:**

1. ✅ CodeQL analysis (new)
2. ✅ cargo-deny check (new)
3. ✅ Existing CI tests
4. ✅ Security validations

**What to watch for:**

- ✓ All workflow jobs pass
- ✓ No YAML syntax errors
- ✓ CodeQL scan completes
- ✓ cargo-deny results appear
- ✓ Security baseline established

**If anything fails:**

- Review GitHub Actions logs
- Check YAML syntax
- Make corrections
- Push again

---

## 🎯 Key Decision Points

### Decision 1: Scope of Deletions

**After investigation in STEP 2:**

**Option A: Keep all changes (Refactoring is intentional)**

- Proceed with full commit
- Document the refactoring in commit message
- Make sure tests pass

**Option B: Revert deletions (Keep only security)**

- Checkout main version for deleted files
- Commit with security-focused message
- Cleaner PR scope

**Recommendation:** Option B (cleaner, focused on security)

---

## ✅ Success Criteria

### After STEP 1 (Conflicts Resolved)

- [ ] `.github/workflows/ci.yml` has no conflict markers
- [ ] `.github/workflows/create_release_assets.yml` has no conflict markers
- [ ] Files are staged and ready to commit

### After STEP 2 (Investigation Done)

- [ ] You understand all 1,445 deleted lines
- [ ] You've decided: keep or revert?
- [ ] Rationale is documented

### After STEP 3 (Local Verification)

- [ ] `cargo build` succeeds
- [ ] `cargo test` passes
- [ ] No linting errors

### After STEP 4 (Pushed)

- [ ] Code is pushed to `pr-1310`
- [ ] Commit message is clear
- [ ] GitHub Actions is running

### After STEP 5 (Verified)

- [ ] GitHub Actions workflows complete
- [ ] CodeQL scan appears in PR
- [ ] cargo-deny check appears in PR
- [ ] All checks pass

---

## 📊 What Gets Added to Main

### New Security Infrastructure

```
✅ .github/workflows/codeql.yml (41 lines)
   - Static analysis on push/PR/weekly
   - Catches code vulnerabilities early

✅ .github/workflows/cargo-deny.yml (41 lines)
   - Dependency policy enforcement
   - Prevents supply chain attacks

✅ deny.toml (34 lines)
   - Security policy configuration
   - Advisories, licenses, bans
```

### Improved Workflows

```
✅ .github/workflows/ci.yml
   - Added job separation (better organization)
   - Added timeouts (resource management)
   - Added security checkouts (prevent token exposure)

✅ .github/workflows/create_release_assets.yml
   - Added timeouts on release builds
   - Added security hardening
```

### Security Baseline

```
✅ GitHub's recommended security practices
✅ SLSA Framework compliance
✅ Supply chain security
✅ Code quality baseline
```

---

## 📝 Commit Message Template

**If keeping all changes:**

```
ci(p0): add CodeQL and cargo-deny; harden CI workflows

Security improvements:
- Add CodeQL static analysis (push, PR, weekly)
- Add cargo-deny dependency policy checks
- Non-blocking initially (plan to enforce after triaging)

Workflow hardening:
- Add timeouts to all CI jobs
- Add security-focused checkout parameters
- Improve cross-compilation installation

Code improvements:
- Refactor src/sudo.rs for clarity
- Optimize src/main.rs
- [Other changes as appropriate]

Resolves: #1310
```

**If reverting to security-only:**

```
ci(p0): add CodeQL and cargo-deny; harden CI workflows

Security improvements:
- Add CodeQL static analysis (push, PR, weekly)  
- Add cargo-deny dependency policy checks
- Non-blocking initially (plan to enforce after triaging)

Workflow hardening:
- Add timeouts to all CI jobs
- Add security-focused checkout parameters  
- Improve cross-compilation installation

Configuration:
- Add deny.toml with recommended policy
- Update Dependabot labels and grouping

Resolves: #1310
```

---

## 🚨 Risk Mitigation

### Risk 1: Conflicts Cause Build Failures

**Mitigation:** CONFLICT_FIXES.md has exact solutions; follow them precisely

### Risk 2: Large Deletions Break Something

**Mitigation:** Local testing in STEP 3 catches this before push

### Risk 3: New Workflows Have Syntax Errors

**Mitigation:** GitHub Actions validates YAML immediately after push

### Risk 4: Security Scanning is Too Strict

**Mitigation:** PR uses non-blocking approach; plan to enforce after review

---

## ⏱️ Time Breakdown

| Step | Time | Activity |
|------|------|----------|
| 1. Resolve conflicts | 10 min | Fix markers in 2 files |
| 2. Investigate deletions | 20 min | Understand changes |
| 3. Local verification | 10 min | Build & test |
| 4. Commit & push | 5 min | Git operations |
| 5. Monitor | 5-10 min | Watch CI complete |
| **TOTAL** | **50-55 min** | **Full execution** |

---

## 🎓 Best Practices Followed

✅ **Security-first approach**

- Non-blocking initial rollout
- Comprehensive security baseline
- Industry-standard tools

✅ **Risk-aware strategy**

- Conflict resolution verified
- Local testing before push
- Investigation of scope

✅ **Professional process**

- Clear commit messages
- Documented decisions
- Staged rollout timeline

✅ **Clean code practices**

- No leftover conflict markers
- Proper YAML formatting
- Security hardening throughout

---

## 🚀 Ready to Execute?

**Before you start:**

- [ ] Read CONFLICT_FIXES.md
- [ ] Understand the 2 conflicts
- [ ] Have editor ready
- [ ] Have 50-55 minutes available

**Then follow steps 1-5 in order**

---

*Execution Plan: November 1, 2025*  
*Status: Ready to proceed*  
*Recommendation: Begin with STEP 1*
