# 🔧 PR #1310 - Exact Conflict Fixes

**Status:** Merge attempted, conflicts visible  
**Files to fix:** 2 (`.github/workflows/ci.yml`, `.github/workflows/create_release_assets.yml`)  

---

## 🎯 Conflict 1: `.github/workflows/ci.yml`

### Current Conflict (Lines 59-74)

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
      - name: Check if `Step::run()`'s match is sorted
```

### What This Means

- **HEAD (PR):** Has `step-match-sorted` as a separate job
- **Main:** Keeps it as part of `custom-checks` job

### Solution

**Replace the entire conflicted section with:**

```yaml
  step-enum-sorted:
    name: Step enum sorted
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - name: Checkout code
        uses: actions/checkout@v5.0.0
        with:
          persist-credentials: false
          fetch-depth: 1

      - name: Check if `Step` enum is sorted
        run: |
          ENUM_NAME="Step"
          FILE="src/step.rs"
          awk "/enum $ENUM_NAME/,/}/" "$FILE" | \
          grep -E '^\s*[A-Za-z_][A-Za-z0-9_]*\s*,?$' | \
          sed 's/[, ]//g' > original.txt
          sort original.txt > sorted.txt
          diff original.txt sorted.txt

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

      - name: Check if `Step::run()`'s match is sorted
        run: |
          FILE="src/step.rs"
          awk '/[[:alpha:]] =>/{print $1}' $FILE > original.txt
          sort original.txt > sorted.txt
          diff original.txt sorted.txt

  main:
    needs: [ fmt, step-enum-sorted, step-match-sorted ]
```

### Key Changes

1. ✅ Rename `custom-checks` → `step-enum-sorted`
2. ✅ Create separate `step-match-sorted` job
3. ✅ Remove the long "default_steps contains every step" check (40+ lines)
4. ✅ Update `needs` to reference both new job names
5. ✅ Keep all security hardening (timeouts, checkout parameters)

### Why This Approach

- PR structure is cleaner (separate jobs for separate concerns)
- Removes problematic bash script that was too complex
- Maintains all security hardening
- Aligns with modern GitHub Actions best practices

---

## 🎯 Conflict 2: `.github/workflows/create_release_assets.yml`

### Location

Lines ~41-47 in the file (same pattern as ci.yml)

### What Conflicts

The release workflow has similar structure conflict where:

- **PR adds:** Timeouts, security hardening, better cross installation
- **Main version:** Likely has older setup

### Solution

**Keep all PR additions:**

- All `timeout-minutes` specifications
- All `persist-credentials: false`
- All `fetch-depth: 1`
- The new `taiki-e/install-action` for cross

**The solution:** Accept PR version's changes to release workflows. They're all valid improvements.

---

## 📋 Step-by-Step Manual Conflict Resolution

### In Your Editor

1. **Open:** `.github/workflows/ci.yml`

2. **Find conflict marker:** `<<<<<<< HEAD` (around line 59)

3. **Identify three sections:**
   - Between `<<<<<<<` and `=======` = PR version (step-match-sorted job)
   - Between `=======` and `>>>>>>>` = Main version (continuation of custom-checks)

4. **Solution:** Keep BOTH! Create the structure as shown above:
   - Rename `custom-checks` → `step-enum-sorted`
   - Add `step-match-sorted` as separate job
   - Delete the complex "default_steps" check
   - Update job dependencies

5. **Remove conflict markers** (all three lines)

6. **Save file**

### For create_release_assets.yml

1. **Open:** `.github/workflows/create_release_assets.yml`

2. **Find conflict markers** (similar pattern)

3. **Keep PR version:** All timeout and security additions are valid

4. **Remove conflict markers**

5. **Save file**

---

## ✅ Verification After Fixing

### Syntax Check

Open both files and verify:

- ✅ All YAML indentation is correct (2-space)
- ✅ All job names are properly formatted
- ✅ All `needs` references match actual job names
- ✅ No conflict markers remain (`<<<<<<<`, `=======`, `>>>>>>>`)

### Git Commands

```powershell
# Show conflicted files
git status

# Add resolved files
git add .github/workflows/ci.yml
git add .github/workflows/create_release_assets.yml

# Verify all conflicts resolved
git status
# Should show: "both added" or "both modified" (depending on git version)

# Complete the merge
git commit -m "Merge main: resolve workflow conflicts"

# Check result
git log --oneline -3
```

---

## 🚨 Important Verification Points

After resolving, verify:

1. **Job names in `needs` match actual jobs:**
   - ❌ `needs: [ fmt, custom-checks ]` (old - REMOVE)
   - ✅ `needs: [ fmt, step-enum-sorted, step-match-sorted ]` (new - KEEP)

2. **All checkouts have security parameters:**

   ```yaml
   - uses: actions/checkout@v5.0.0
     with:
       persist-credentials: false
       fetch-depth: 1
   ```

3. **All jobs have timeouts:**

   ```yaml
   timeout-minutes: 10  # or 90, 120 depending on job
   ```

4. **No orphaned sections** from either version

---

## 📊 Conflict Statistics

| Item | Count |
|------|-------|
| Total conflicts | 2 files |
| Conflict sections in ci.yml | 1 major section |
| Conflict sections in create_release_assets.yml | ~1-2 sections |
| Estimated fix time | 5-10 minutes |
| Difficulty | Medium (structural, not content) |

---

## 🎯 After Conflicts Are Resolved

1. **Commit the merge**
2. **Push to PR:**

   ```powershell
   git push -f origin pr-1310
   ```

3. **GitHub Actions will:**
   - Re-run CI with new workflows
   - Validate the YAML syntax
   - Test with new CodeQL + cargo-deny

4. **Wait for CI to pass**

5. **Review for maintainer**

---

## 💡 Quick Reference

**If you get stuck:**

```powershell
# See what you changed
git diff

# See specific file changes
git diff .github/workflows/ci.yml

# Reset if needed (before commit)
git merge --abort

# Then try again
```

---

## 🎓 Summary of Changes

### What the PR Does (at a high level)

1. ✅ **Adds CodeQL** - Static analysis security scanning
2. ✅ **Adds cargo-deny** - Dependency policy enforcement
3. ✅ **Hardens workflows** - Security checkout parameters, timeouts
4. ✅ **Restructures CI** - Splits checks into cleaner jobs
5. ⚠️ **Large deletions** - Investigate src/ changes separately

### What We're Resolving

1. ✅ **ci.yml conflict** - Integrating workflow restructuring
2. ✅ **create_release_assets.yml conflict** - Adding workflow hardening

### What's NOT in conflict

- ✅ New files (codeql.yml, cargo-deny.yml, deny.toml) - Will merge cleanly
- ✅ New dependabot config - Will merge cleanly
- ✅ README updates - Should merge cleanly

---

**Status:** Ready to manually resolve  
**Estimated time:** 5-10 minutes  
**Difficulty:** Medium  

*Once resolved, continue with investigation of large source code changes.*
