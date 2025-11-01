# PR #1310 Merge Resolution - Exact Steps

**Status:** Merge conflicts detected in 2 files  
**Severity:** 🟡 Moderate (resolvable with provided exact fixes)  
**Time to Fix:** 5-10 minutes

---

## Conflict 1: `.github/workflows/ci.yml`

### Issue

The pr-1310 branch had a job called `step-match-sorted` that was removed in main. The merge conflict shows:

- **HEAD (pr-1310):** Has `step-match-sorted` job (lines 59-68)
- **main:** Merged directly to `custom-checks` checks without the separate job

### Root Cause

Between the PR creation (70 days ago) and now, the job structure was refactored. The `step-match-sorted` checks were either:

1. Incorporated into `custom-checks`, OR
2. Removed as no longer needed, OR
3. Split differently

### Resolution Strategy

**KEEP the current main structure.** The main branch has the refactored version.

### Steps to Fix

1. **Open `.github/workflows/ci.yml` in your editor**

2. **Find the conflict marker** (around line 59):

   ```yaml
   <<<<<<< HEAD
     step-match-sorted:
       name: Step match sorted
       ...
   =======
   >>>>>>> main
   ```

3. **Delete the entire conflicted section including:**
   - `<<<<<<< HEAD`
   - The `step-match-sorted` job definition (lines 59-68)
   - `=======`
   - `>>>>>>> main`

4. **Keep the rest intact.** After the custom-checks job closes (around line 100), immediately keep the `- name: Check if 'Step::run()'...` step as part of `custom-checks`.

5. **Result:** The file should have:

   ```yaml
   custom-checks:
     name: Custom checks
     runs-on: ubuntu-latest
     timeout-minutes: 10
     steps:
       - name: Checkout code
         ...
       - name: Check if 'Step' enum is sorted
         ...
       - name: Check if 'Step::run()'s match is sorted    # <-- KEEP THIS
         ...
       - name: Check if 'default_steps' contains every step   # <-- KEEP THIS
         ...

   main:   # <-- Main job starts here
     needs: [ fmt, custom-checks ]
     name: ...
   ```

### Quick Fix (Copy-Paste Method)

**Find this section:**

```yaml
<<<<<<< HEAD
  step-match-sorted:
    name: Step match sorted
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - name: Checkout code
        uses: actions/checkout@v5.0.0
        with:
          persist-credentials: false
          fetch-depth: 1

=======
>>>>>>> main
```

**Replace with nothing (delete it entirely).** The `main` job definition that follows will naturally connect to the `custom-checks` job.

---

## Conflict 2: `.github/workflows/create_release_assets.yml`

### Issue

Similar pattern - the pr-1310 branch had a different permission/timeout structure that conflicts with main's updated approach.

### Root Cause

PR #1310 was created with security hardening (more restrictive permissions, timeout adjustments). Main has evolved with different security practices.

### Resolution Strategy

**MERGE both versions:** Keep the enhanced security hardening from pr-1310 where compatible with main's structure.

### Steps to Fix

1. **Open `.github/workflows/create_release_assets.yml`**

2. **Find all conflict markers** (typically around the `native_build` job permissions and timeout sections)

3. **For permissions sections:**
   - Keep both `HEAD` (pr-1310) and `main` permissions
   - Merge them into a single comprehensive list
   - Do NOT duplicate permission types

4. **For timeout-minutes:**
   - If pr-1310 has `timeout-minutes: 90` and main doesn't, KEEP the pr-1310 timeout
   - This is a security hardening

5. **Expected structure after merge:**

   ```yaml
   native_build:
     permissions:
       id-token: write
       contents: write
       attestations: write
     strategy:
       ...
     runs-on: ${{ matrix.platform }}
     timeout-minutes: 90
   ```

### Quick Fix (Reference)

**Look for:**

```yaml
<<<<<<< HEAD
    timeout-minutes: 90
    steps:
      ...
=======
    runs-on: ...
    timeout-minutes: 90
    steps:
      ...
>>>>>>> main
```

**Keep:** The version that has BOTH the permissions AND the timeout, with no duplication.

---

## Command-Line Resolution

If you prefer using git tools to resolve:

```powershell
# 1. Check conflict status
cd e:\topgrade
git status

# 2. View ci.yml conflict
git diff .github/workflows/ci.yml

# 3. View create_release_assets.yml conflict
git diff .github/workflows/create_release_assets.yml

# 4. Once resolved, stage files
git add .github/workflows/ci.yml
git add .github/workflows/create_release_assets.yml

# 5. Complete the merge
git commit -m "Merge: resolve workflow conflicts in PR #1310"

# 6. Push to pr-1310
git push origin pr-1310
```

---

## Verification Checklist

After resolving conflicts:

```powershell
# 1. Verify YAML syntax
cd e:\topgrade
yamllint .github/workflows/ci.yml
yamllint .github/workflows/create_release_assets.yml

# 2. Check for conflict markers
grep -r "<<<<<<" .github/workflows/
grep -r "=======" .github/workflows/
grep -r ">>>>>>>" .github/workflows/

# 3. Verify merge is complete
git status  # Should show: nothing to commit

# 4. Quick build check
cargo check --locked

# 5. View what we're merging
git log --oneline -5
git diff main..HEAD --stat | head -20
```

---

## If YAML Validation Fails

**Error:** Invalid YAML syntax

**Likely Cause:** Improper indentation when resolving conflicts

**Fix:**

1. Ensure all `- name:` lines have consistent indentation (2 spaces)
2. Ensure all job names (`custom-checks:`, `main:`, etc.) have consistent indentation
3. Remove any leftover conflict markers (`<<<<<<<`, `=======`, `>>>>>>>`)
4. Validate with: `yamllint .github/workflows/ci.yml`

---

## If Build Check Fails

**Error:** `cargo check` fails

**Likely Cause:** Conflict resolution introduced syntax error in YAML

**Fix:**

1. Check `.github/workflows/ci.yml` line-by-line for YAML syntax
2. Look for duplicate job names (e.g., two `main:` jobs)
3. Look for improper indentation
4. Verify all sections properly closed with `steps:`, `with:`, etc.
5. Re-run `yamllint` for precise error location

---

## Success Criteria

After completing both conflict resolutions:

- ✅ No conflict markers in `.github/workflows/ci.yml`
- ✅ No conflict markers in `.github/workflows/create_release_assets.yml`
- ✅ YAML validates: `yamllint .github/workflows/*.yml`
- ✅ Build checks: `cargo check --locked` passes
- ✅ Git status clean: `git status` shows nothing to commit
- ✅ Ready to push: `git push origin pr-1310`

---

## Next Steps After Resolution

1. **Commit the merge:**

   ```bash
   git add .github/workflows/ci.yml .github/workflows/create_release_assets.yml
   git commit -m "Merge: resolve workflow conflicts"
   ```

2. **Push to pr-1310:**

   ```bash
   git push origin pr-1310
   ```

3. **Monitor GitHub Actions:**
   - CodeQL workflow should run
   - cargo-deny workflow should run
   - All existing workflows should still pass

4. **If any workflow fails:**
   - Check the Actions tab on PR #1310
   - Review error logs
   - Make targeted fixes if needed
   - Push again

---

## Support Files

- `PR_1310_EXECUTION_PLAN.md` - Full 5-step execution plan
- `NISTEE_WORKFLOW_ANALYSIS.md` - Why these changes are critical
- `PR_1310_DECISION_MATRIX.md` - Decision framework

**Ready to begin resolution? Start with the manual editing steps above, or use the command-line approach if you prefer.**
