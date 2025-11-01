# QUICK FIX: PR #1310 Merge Conflicts

**Current Status:** 2 files with conflicts detected  
**Current Branch:** `pr-1310`  
**What to do:** Follow the exact edits below

---

## CONFLICT 1: `.github/workflows/ci.yml`

### What's wrong

Around line 59, you'll see:

```
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

### Fix

**Delete the entire block above including the conflict markers.**

The next section that starts with `- name: Check if 'Step::run()'...` should stay and is part of the `custom-checks` job.

### Result

The file should flow directly from `custom-checks` job definition to its steps, with no `step-match-sorted` job.

---

## CONFLICT 2: `.github/workflows/create_release_assets.yml`

### What's wrong

Around the `native_build` permissions/timeouts section, you'll see conflict markers.

### Fix Strategy

For this file, you want to:

1. Keep timeout-minutes: 90 (security hardening from pr-1310)
2. Keep all permission fields (merge both if needed)
3. Remove conflict markers

### Result

The `native_build` job should have proper timeouts and merged permissions.

---

## AUTOMATED FIX (PowerShell)

Run this script to auto-resolve:

```powershell
cd e:\topgrade

# 1. Get current status
Write-Host "Current status:" -ForegroundColor Green
git status

# 2. Check for conflicts
Write-Host "`nConflicts:" -ForegroundColor Green
git diff --name-only --diff-filter=U

# 3. Show ci.yml conflict area
Write-Host "`nCI.yml conflict around line 59:" -ForegroundColor Yellow
git diff .github/workflows/ci.yml | head -50

# 4. Manually resolve step 1 - ci.yml
Write-Host "`nResolving ci.yml..." -ForegroundColor Cyan
# Edit and save manually, then:
git add .github/workflows/ci.yml

# 5. Manually resolve step 2 - create_release_assets.yml  
Write-Host "`nResolving create_release_assets.yml..." -ForegroundColor Cyan
# Edit and save manually, then:
git add .github/workflows/create_release_assets.yml

# 6. Complete merge
Write-Host "`nCompleting merge..." -ForegroundColor Green
git commit -m "Merge: resolve workflow conflicts in PR #1310"

# 7. Push result
Write-Host "`nPushing to pr-1310..." -ForegroundColor Green
git push origin pr-1310

Write-Host "`nDone! PR #1310 is ready for review." -ForegroundColor Green
```

---

## MANUAL EDITING STEPS

### Step 1: Edit `.github/workflows/ci.yml`

1. Open: `e:\topgrade\.github\workflows\ci.yml`
2. Find: `<<<<<<< HEAD` (around line 59)
3. Delete: The entire conflict block (lines 59-69 approximately)
4. Result: File should go from `custom-checks` job directly to checking that `Step::run()` match is sorted
5. Save file

### Step 2: Edit `.github/workflows/create_release_assets.yml`

1. Open: `e:\topgrade\.github\workflows\create_release_assets.yml`
2. Find: `<<<<<<< HEAD`
3. Look at what's in HEAD vs main
4. Keep: The version with both timeout AND permissions
5. Delete: Conflict markers and any duplicates
6. Save file

### Step 3: Verify resolution

```powershell
cd e:\topgrade

# Check for remaining conflict markers
Get-Content .github/workflows/ci.yml | Select-String "<<<<<<|======|>>>>>>"
Get-Content .github/workflows/create_release_assets.yml | Select-String "<<<<<<|======|>>>>>>"

# If nothing returns, you're good!
```

### Step 4: Finalize merge

```powershell
cd e:\topgrade

# Stage files
git add .github/workflows/ci.yml
git add .github/workflows/create_release_assets.yml

# Commit
git commit -m "Merge: resolve workflow conflicts in PR #1310"

# Push
git push origin pr-1310

# Watch for GitHub Actions to run
Write-Host "PR #1310 is now updated. Watch the Actions tab for CodeQL and cargo-deny to run."
```

---

## VERIFY IT WORKED

Run these to confirm:

```powershell
cd e:\topgrade

# No conflicts
git status
# Expected: "working tree clean" or "nothing to commit"

# Can build
cargo check --locked
# Expected: "Finished `dev` profile ..."

# YAML valid
yamllint .github/workflows/ci.yml
yamllint .github/workflows/create_release_assets.yml
# Expected: No output (pass)

# Merge is complete
git log --oneline -1
# Expected: Shows "Merge: resolve..." commit
```

---

## HELP - SOMETHING WENT WRONG

### Problem: "I made a mistake, need to start over"

```powershell
git merge --abort
# Back to before merge attempt
```

### Problem: "Can't find conflict markers"

```powershell
git diff .github/workflows/ci.yml | head -100
git diff .github/workflows/create_release_assets.yml | head -100
# Shows exact diff
```

### Problem: "YAML validation fails"

1. Check all lines have proper indentation (2-space)
2. Check no job names are duplicated
3. Check all conflict markers are removed
4. Try: `yamllint .github/workflows/ci.yml --print-config` for details

### Problem: "Already pushed something wrong"

```powershell
# Undo last push
git reset --hard HEAD~1
git push origin pr-1310 --force
# Start over
```

---

## SUCCESS = This works

```powershell
cd e:\topgrade
git status                          # Working tree clean
cargo check --locked                # Builds successfully  
yamllint .github/workflows/*.yml    # No errors
git log --oneline -5 | head -1      # Shows merge commit
```

---

**Ready? Start with Step 1 above. Should take 5-10 minutes total.**
