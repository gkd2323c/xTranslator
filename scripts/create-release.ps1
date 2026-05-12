#!/usr/bin/env pwsh

# GitHub Release creation script for xTranslator
# Creates a new GitHub release with proper assets and changelog

param(
    [string]$Version = "0.2.0",
    [switch]$Draft,
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"

# Color output functions
function Write-Success($message) {
    Write-Host "✅ $message" -ForegroundColor Green
}

function Write-Warning($message) {
    Write-Host "⚠️  $message" -ForegroundColor Yellow
}

function Write-Error($message) {
    Write-Host "❌ $message" -ForegroundColor Red
}

function Write-Info($message) {
    Write-Host "ℹ️  $message" -ForegroundColor Cyan
}

function Write-Header($message) {
    Write-Host "`n=== $message ===" -ForegroundColor Magenta
}

# Check prerequisites
function Test-Prerequisites {
    Write-Header "Checking Prerequisites"
    
    # Check if we're in project root
    if (-not (Test-Path "Cargo.toml")) {
        Write-Error "Not in project root directory. Please run from project root."
        exit 1
    }
    
    # Check if git is clean
    $gitStatus = git status --porcelain
    if ($gitStatus) {
        Write-Warning "Working directory is not clean. Uncommitted changes detected:"
        Write-Warning $gitStatus
        Write-Warning "Consider committing changes before release."
        
        if (-not $Draft) {
            Write-Error "Cannot create release with uncommitted changes. Use -Draft to create a draft release."
            exit 1
        }
    }
    
    # Check for GitHub CLI
    if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
        Write-Error "GitHub CLI (gh) is not installed. Please install it first."
        Write-Info "Install from: https://cli.github.com/"
        exit 1
    }
    
    # Check if authenticated
    $authStatus = gh auth status
    if ($authStatus -match "not logged") {
        Write-Error "Not authenticated with GitHub. Please run 'gh auth login'."
        exit 1
    }
    
    Write-Success "Prerequisites check passed"
}

# Build release artifacts
function Build-Release {
    if ($SkipBuild) {
        Write-Warning "Skipping build as requested"
        return
    }
    
    Write-Header "Building Release Artifacts"
    
    # Clean previous builds
    if (Test-Path "target/release") {
        Remove-Item -Recurse -Force "target/release"
    }
    
    # Build backend
    Write-Info "Building backend..."
    cargo build --workspace --release
    
    # Build frontend
    Write-Info "Building frontend..."
    Push-Location "ui"
    npm run build
    Pop-Location
    
    # Build Tauri application
    Write-Info "Building Tauri application..."
    Push-Location "src-tauri"
    cargo tauri build --bundles msi
    Pop-Location
    
    # Verify build artifacts
    $msiPath = "target/release/bundle/msi/xTranslator_${Version}_x64_en-US.msi"
    if (-not (Test-Path $msiPath)) {
        Write-Error "MSI build artifact not found: $msiPath"
        exit 1
    }
    
    $exePath = "target/release/xtranslator-tauri.exe"
    if (-not (Test-Path $exePath)) {
        Write-Error "EXE build artifact not found: $exePath"
        exit 1
    }
    
    Write-Success "Build artifacts created successfully"
}

# Prepare release notes
function Get-ReleaseNotes {
    Write-Header "Preparing Release Notes"
    
    $changelogPath = "CHANGELOG.md"
    if (-not (Test-Path $changelogPath)) {
        Write-Error "CHANGELOG.md not found"
        exit 1
    }
    
    $changelog = Get-Content $changelogPath -Raw
    
    # Extract current version section
    $versionPattern = "\[${Version}\] - \d{4}-\d{2}-\d{2}"
    $versionSection = [regex]::Match($changelog, "(?s)$versionPattern.*?(?=\n## \[|\z)")
    
    if (-not $versionSection) {
        Write-Error "Version section not found in CHANGELOG.md for version $Version"
        exit 1
    }
    
    # Clean up the release notes
    $releaseNotes = $versionSection.Value -replace "\[${Version}\] - \d{4}-\d{2}-\d{2}\s*", ""
    $releaseNotes = $releaseNotes.Trim()
    
    Write-Success "Release notes extracted"
    return $releaseNotes
}

# Create GitHub release
function Create-GitHubRelease($releaseNotes) {
    Write-Header "Creating GitHub Release"
    
    $releaseTitle = "Release v$Version"
    $tagName = "v$Version"
    $targetBranch = "main"
    
    # Prepare assets
    $assets = @(
        "target/release/bundle/msi/xTranslator_${Version}_x64_en-US.msi",
        "target/release/xtranslator-tauri.exe"
    )
    
    # Create release command
    $releaseArgs = @(
        "release", "create", $tagName,
        "--title", $releaseTitle,
        "--target", $targetBranch,
        "--notes", $releaseNotes
    )
    
    if ($Draft) {
        $releaseArgs += "--draft"
        Write-Info "Creating DRAFT release"
    } else {
        Write-Info "Creating PUBLISHED release"
    }
    
    # Add assets
    foreach ($asset in $assets) {
        if (Test-Path $asset) {
            $assetName = Split-Path $asset -Leaf
            Write-Info "Adding asset: $assetName"
            $releaseArgs += $asset
        } else {
            Write-Warning "Asset not found: $asset"
        }
    }
    
    # Execute release creation
    Write-Info "Executing: gh $($releaseArgs -join ' ')"
    $releaseOutput = & gh @releaseArgs
    
    if ($LASTEXITCODE -eq 0) {
        Write-Success "GitHub release created successfully!"
        
        # Extract release URL
        $releaseUrl = [regex]::Match($releaseOutput, "https://github.com/[^)]+")
        if ($releaseUrl) {
            Write-Info "Release URL: $($releaseUrl.Value)"
        }
    } else {
        Write-Error "Failed to create GitHub release"
        Write-Error $releaseOutput
        exit 1
    }
}

# Create git tag and push
function Create-GitTag {
    Write-Header "Creating Git Tag"
    
    $tagName = "v$Version"
    
    # Check if tag already exists
    $tagExists = git tag -l | Select-String $tagName
    if ($tagExists) {
        Write-Warning "Tag $tagName already exists"
        return
    }
    
    # Create and push tag
    Write-Info "Creating tag: $tagName"
    git tag $tagName
    
    Write-Info "Pushing tag to origin"
    git push origin $tagName
    
    Write-Success "Git tag created and pushed"
}

# Main execution
function Main {
    Write-Header "xTranslator Release Creator v$Version"
    Write-Info "Starting release creation process..."
    
    try {
        Test-Prerequisites
        Build-Release
        $releaseNotes = Get-ReleaseNotes
        
        if (-not $Draft) {
            Create-GitTag
        }
        
        Create-GitHubRelease $releaseNotes
        
        Write-Header "Release Summary"
        Write-Success "Version: $Version"
        Write-Success "Status: $(if ($Draft) { 'Draft' } else { 'Published' })"
        Write-Success "Assets: $($assets.Count) files"
        Write-Success "Release created successfully!"
        
        if ($Draft) {
            Write-Info "Remember to publish the draft when ready:"
            Write-Info "1. Go to GitHub releases page"
            Write-Info "2. Edit and publish the draft release"
        }
    }
    catch {
        Write-Error "Release creation failed: $($_.Exception.Message)"
        exit 1
    }
}

# Run main function
Main
