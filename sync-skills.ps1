<#
.SYNOPSIS
  Sync skills across Agent frameworks (Claude, Cursor, Copilot, Augment, OpenCode).

.DESCRIPTION
  Manages skill synchronization across multiple AI coding assistant frameworks:
  - .agents/     → central source for user skills
  - .claude/     → Claude Code (gstack + GSD + user skills)
  - .cursor/     → Cursor (GSD skills)
  - .copilot/    → GitHub Copilot (GSD skills)
  - .augment/    → Augment (GSD skills)
  - .opencode/   → OpenCode (per-project, in xTranslator)

  Sync modes:
    user  - .agents/skills/* → .claude/skills/* (user-written skills)
    gsd   - .claude/skills/gsd-* → .cursor|.copilot|.augment (GSD workflow skills)
    gstack- .claude/skills/gstack* → other frameworks (gstack core + branded skills)
    all   - every mode above

  Always does a dry-run first showing what would change, then applies on confirmation.

.PARAMETER Mode
  What to sync: "user", "gsd", "gstack", "all" (default).

.PARAMETER Force
  Skip confirmation prompt, apply changes directly.

.PARAMETER DryRun
  Show what would change but don't apply anything (default unless -Force).

.EXAMPLE
  .\sync-skills.ps1              # dry-run all modes
  .\sync-skills.ps1 -Force       # apply all changes
  .\sync-skills.ps1 -Mode user   # only sync user skills (dry-run)
#>

param(
  [ValidateSet("user", "gsd", "gstack", "all")]
  [string]$Mode = "all",
  [switch]$Force,
  [switch]$DryRun
)

$ErrorActionPreference = "Stop"
$HomeDir = $env:USERPROFILE

# ── Framework roots ──────────────────────────────────────────────────
$Frameworks = @{
  agents  = "$HomeDir\.agents\skills"
  claude  = "$HomeDir\.claude\skills"
  cursor  = "$HomeDir\.cursor\skills"
  copilot = "$HomeDir\.copilot\skills"
  augment = "$HomeDir\.augment\skills"
}

# ── Project-level OpenCode roots ─────────────────────────────────────
$Projects = @(
  @{
    Name = "xTranslator"
    Root = "C:\Users\gkd2323c\Documents\xTranslator\.opencode\skills"
    SyncSkills = @('find-skills', 'skill-creator')  # shared with .agents
  }
)

# ── Sync rules ───────────────────────────────────────────────────────
$Rules = @()

# 1. User skills: .agents → .claude (for shared user-written skills)
if ($Mode -in @("user", "all")) {
  $Rules += @{
    Name     = "User skills"
    Source   = $Frameworks.agents
    Targets  = @($Frameworks.claude)
    Filter   = { $true }  # all skills in .agents
    Preserve = '*_EXCEPT_gsd-*'  # don't delete gsd-* from claude
  }
}

# 2. GSD skills: .claude/gsd-* → .cursor, .copilot, .augment
if ($Mode -in @("gsd", "all")) {
  $Rules += @{
    Name     = "GSD skills"
    Source   = $Frameworks.claude
    Targets  = @($Frameworks.cursor, $Frameworks.copilot, $Frameworks.augment)
    Filter   = { $_.Name -like 'gsd-*' }
    Preserve = $null  # these dirs only contain GSD skills, OK to clean orphans
  }
}

# 3. Project OpenCode skills: .agents/common-skills → .opencode/skills/ (per project)
if ($Mode -in @("user", "all")) {
  foreach ($proj in $Projects) {
    $projectSkills = $proj.SyncSkills
    $Rules += @{
      Name     = "Project: $($proj.Name)"
      Source   = $Frameworks.agents
      Targets  = @($proj.Root)
      Filter   = { $_.Name -in $projectSkills }
      Preserve = '*_EXCEPT_gsd-*'
    }
  }
}

# 4. Gstack core + branded skills: .claude/gstack* → .cursor, .copilot, .augment
if ($Mode -in @("gstack", "all")) {
  # gstack core (non-gsd meta-skills from gstack package)
  $gstackSkills = @('autoplan', 'benchmark', 'browse', 'canary', 'careful',
    'checkpoint', 'codex', 'connect-chrome', 'cso', 'design-consultation',
    'design-html', 'design-review', 'design-shotgun', 'devex-review',
    'document-release', 'freeze', 'gstack', 'gstack-autoplan', 'gstack-benchmark',
    'gstack-browse', 'gstack-canary', 'gstack-careful', 'gstack-checkpoint',
    'gstack-cso', 'gstack-design-consultation', 'gstack-design-html',
    'gstack-design-review', 'gstack-design-shotgun', 'gstack-devex-review',
    'gstack-document-release', 'gstack-freeze', 'gstack-guard', 'gstack-health',
    'gstack-investigate', 'gstack-land-and-deploy', 'gstack-learn',
    'gstack-office-hours', 'gstack-open-gstack-browser',
    'gstack-openclaw-ceo-review', 'gstack-openclaw-investigate',
    'gstack-openclaw-office-hours', 'gstack-openclaw-retro',
    'gstack-pair-agent', 'gstack-plan-ceo-review', 'gstack-plan-design-review',
    'gstack-plan-devex-review', 'gstack-plan-eng-review', 'gstack-qa',
    'gstack-qa-only', 'gstack-retro', 'gstack-review',
    'gstack-setup-browser-cookies', 'gstack-setup-deploy', 'gstack-ship',
    'gstack-unfreeze', 'gstack-upgrade', 'guard', 'health', 'investigate',
    'land-and-deploy', 'learn', 'office-hours', 'open-gstack-browser',
    'pair-agent', 'plan-ceo-review', 'plan-design-review', 'plan-devex-review',
    'plan-eng-review', 'qa', 'qa-only', 'retro', 'review',
    'setup-browser-cookies', 'setup-deploy', 'ship', 'unfreeze',
    'gstack-upgrade'
  )

  # Build filter: any skill in claude that matches gstack skill names
  $gstackFilter = {
    $n = $_.Name
    $gstackSkills -contains $n -or
    $n -eq 'gstack-upgrade' -or
    $n -like 'gstack-*'
  }

  $Rules += @{
    Name     = "Gstack skills"
    Source   = $Frameworks.claude
    Targets  = @($Frameworks.cursor, $Frameworks.copilot, $Frameworks.augment)
    Filter   = $gstackFilter
    Preserve = $null
  }
}

# ── Helper functions ─────────────────────────────────────────────────
function Get-SkillDirs {
  param([string]$Path)
  if (-not (Test-Path $Path)) { return @() }
  Get-ChildItem -LiteralPath $Path -Directory | Where-Object {
    Test-Path (Join-Path $_.FullName "SKILL.md")
  }
}

function Get-DiffActions {
  param(
    [string]$SourceRoot,
    [string[]]$TargetRoots,
    [scriptblock]$Filter
  )
  $actions = @()  # [PSCustomObject]@{ Action; Skill; SourcePath; TargetPath; TargetRoot }

  $sourceSkills = Get-SkillDirs $SourceRoot | Where-Object $Filter
  if (-not $sourceSkills) { return $actions }

  foreach ($targetRoot in $TargetRoots) {
    # Ensure target dir exists
    if (-not (Test-Path $targetRoot)) {
      foreach ($s in $sourceSkills) {
        $actions += [PSCustomObject]@{
          Action     = "CREATE_DIR"
          Skill      = $s.Name
          SourcePath = $s.FullName
          TargetPath = Join-Path $targetRoot $s.Name
          TargetRoot = $targetRoot
        }
      }
      continue
    }

    $targetSkills = Get-SkillDirs $targetRoot
    $targetNames  = $targetSkills | ForEach-Object Name

    foreach ($s in $sourceSkills) {
      $targetSkillPath = Join-Path $targetRoot $s.Name
      $targetSkMd = Join-Path $targetSkillPath "SKILL.md"
      if (-not (Test-Path $targetSkMd)) {
        $actions += [PSCustomObject]@{
          Action     = "CREATE"
          Skill      = $s.Name
          SourcePath = $s.FullName
          TargetPath = $targetSkillPath
          TargetRoot = $targetRoot
        }
      } else {
        $srcHash = (Get-FileHash (Join-Path $s.FullName "SKILL.md") -Algorithm MD5).Hash
        $tgtHash = (Get-FileHash $targetSkMd -Algorithm MD5).Hash
        if ($srcHash -ne $tgtHash) {
          $actions += [PSCustomObject]@{
            Action     = "UPDATE"
            Skill      = $s.Name
            SourcePath = $s.FullName
            TargetPath = $targetSkillPath
            TargetRoot = $targetRoot
          }
        }
      }
    }
  }

  return $actions
}

function Copy-Skill {
  param($Action)
  $src = $Action.SourcePath
  $tgt = $Action.TargetPath
  $tgtDir = Split-Path $tgt -Parent

  if (-not (Test-Path $tgtDir)) {
    New-Item -ItemType Directory -Path $tgtDir -Force | Out-Null
  }

  # Remove old target first to clean up stale files
  if (Test-Path $tgt) {
    Remove-Item -LiteralPath $tgt -Recurse -Force
  }

  # Robocopy for reliable mirroring: /MIR mirrors dir tree, /NJH /NJS /NDL suppresses noise
  $log = robocopy $src $tgt /MIR /NJH /NJS /NDL /NP 2>&1
  if ($LASTEXITCODE -ge 8) {
    Write-Warning "robocopy failed for $($Action.Skill): $log"
  }
}

# ── Main ─────────────────────────────────────────────────────────────
Write-Host "`n=== Skill Sync ===" -ForegroundColor Cyan
Write-Host "Mode: $Mode`n" -ForegroundColor Cyan

$allActions = @()

foreach ($rule in $Rules) {
  Write-Host "--- $($rule.Name) ---" -ForegroundColor Yellow
  $actions = Get-DiffActions -SourceRoot $rule.Source -TargetRoots $rule.Targets -Filter $rule.Filter
  if ($actions.Count -eq 0) {
    Write-Host "  No changes needed." -ForegroundColor Green
    continue
  }

  # Group by target root for display
  $byTarget = $actions | Group-Object TargetRoot
  foreach ($grp in $byTarget) {
    $targetLabel = Split-Path $grp.Name -Parent | Split-Path -Leaf
    $createCount = ($grp.Group | Where-Object { $_.Action -eq 'CREATE' -or $_.Action -eq 'CREATE_DIR' }).Count
    $updateCount = ($grp.Group | Where-Object { $_.Action -eq 'UPDATE' }).Count
    Write-Host "  → .$targetLabel/skills/: $createCount new, $updateCount updated" -ForegroundColor Gray
    foreach ($a in $grp.Group) {
      $color = if ($a.Action -eq 'CREATE' -or $a.Action -eq 'CREATE_DIR') { 'Green' } else { 'Yellow' }
      Write-Host "    [$($a.Action)] $($a.Skill)" -ForegroundColor $color
    }
  }

  $allActions += $actions
  Write-Host ""
}

if ($allActions.Count -eq 0) {
  Write-Host "`nAll skills are in sync. Nothing to do." -ForegroundColor Green
  return
}

# ── Confirmation ─────────────────────────────────────────────────────
$totalCreate = ($allActions | Where-Object { $_.Action -eq 'CREATE' -or $_.Action -eq 'CREATE_DIR' }).Count
$totalUpdate = ($allActions | Where-Object { $_.Action -eq 'UPDATE' }).Count
Write-Host "Summary: $totalCreate new, $totalUpdate updates across $($Frameworks.Count) frameworks" -ForegroundColor Cyan

if ($DryRun -or -not $Force) {
  if (-not $Force) {
    $answer = Read-Host "`nApply these changes? (y/N)"
    if ($answer -notin @('y', 'Y', 'yes', 'YES')) {
      Write-Host "Cancelled." -ForegroundColor Red
      return
    }
  }
}

# ── Apply ────────────────────────────────────────────────────────────
Write-Host "`nApplying..." -ForegroundColor Cyan
foreach ($a in $allActions) {
  if ($a.Action -eq 'CREATE_DIR' -or $a.Action -eq 'CREATE' -or $a.Action -eq 'UPDATE') {
    Write-Host "  [$($a.Action)] $($a.Skill) → $((Split-Path $a.TargetRoot -Parent | Split-Path -Leaf))" -ForegroundColor Gray
    Copy-Skill $a
  }
}

Write-Host "`nDone." -ForegroundColor Green
