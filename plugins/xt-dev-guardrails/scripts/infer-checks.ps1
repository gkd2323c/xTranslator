param(
  [Parameter(ValueFromRemainingArguments = $true)]
  [string[]]$Paths
)

$ErrorActionPreference = "Stop"

if (-not $Paths -or $Paths.Count -eq 0) {
  $gitPaths = git diff --name-only --cached
  if (-not $gitPaths) {
    $gitPaths = git diff --name-only
  }
  $Paths = @($gitPaths | Where-Object { $_ -and $_.Trim() -ne "" })
}

if (-not $Paths -or $Paths.Count -eq 0) {
  Write-Output "No changed files detected."
  Write-Output "Baseline checks:"
  Write-Output "- cargo test -p xt-core --lib"
  Write-Output "- Set-specific checks after you know the touched files"
  exit 0
}

$normalized = $Paths | ForEach-Object { $_.Replace("/", "\") } | Sort-Object -Unique
$checks = New-Object System.Collections.Generic.List[string]
$notes = New-Object System.Collections.Generic.List[string]
$warnings = New-Object System.Collections.Generic.List[string]
$fixes = New-Object System.Collections.Generic.List[string]

function Add-UniqueLine {
  param(
    [System.Collections.Generic.List[string]]$List,
    [string]$Line
  )
  if (-not $List.Contains($Line)) {
    $List.Add($Line) | Out-Null
  }
}

function Add-UniqueBlock {
  param(
    [System.Collections.Generic.List[string]]$List,
    [string]$Text
  )
  if (-not $List.Contains($Text)) {
    $List.Add($Text) | Out-Null
  }
}

function New-FixLines {
  return New-Object System.Collections.Generic.List[string]
}

function Add-FixLine {
  param(
    [System.Collections.Generic.List[string]]$Lines,
    [string]$Line
  )
  $Lines.Add($Line) | Out-Null
}

function Join-FixLines {
  param(
    [System.Collections.Generic.List[string]]$Lines
  )
  return [string]::Join("`n", $Lines)
}

function Get-DiffText {
  param(
    [string]$Path
  )

  $cached = git diff --cached -- $Path 2>$null
  if ($cached) {
    return ($cached -join "`n")
  }

  $worktree = git diff -- $Path 2>$null
  if ($worktree) {
    return ($worktree -join "`n")
  }

  return ""
}

function Get-RustDtoNames {
  param(
    [string]$Path
  )

  $content = Get-Content -LiteralPath $Path -Raw
  $matches = [regex]::Matches($content, 'pub struct\s+([A-Za-z0-9_]+)|pub enum\s+([A-Za-z0-9_]+)')
  $names = foreach ($match in $matches) {
    if ($match.Groups[1].Success) {
      $match.Groups[1].Value
    } elseif ($match.Groups[2].Success) {
      $match.Groups[2].Value
    }
  }
  return @($names | Sort-Object -Unique)
}

function Get-TsExportedNames {
  param(
    [string]$Path
  )

  $content = Get-Content -LiteralPath $Path -Raw
  $matches = [regex]::Matches($content, 'export interface\s+([A-Za-z0-9_]+)|export type\s+([A-Za-z0-9_]+)')
  $names = foreach ($match in $matches) {
    if ($match.Groups[1].Success) {
      $match.Groups[1].Value
    } elseif ($match.Groups[2].Success) {
      $match.Groups[2].Value
    }
  }
  return @($names | Sort-Object -Unique)
}

function Get-RustDtoBlocks {
  param(
    [string]$Path
  )

  $lines = Get-Content -LiteralPath $Path
  $map = @{}

  for ($i = 0; $i -lt $lines.Count; $i++) {
    $line = $lines[$i]
    if ($line -match 'pub (struct|enum)\s+([A-Za-z0-9_]+)\s*\{') {
      $name = $matches[2]
      $startLine = $i + 1
      $braceDepth = 0
      $bodyLines = New-Object System.Collections.Generic.List[string]
      $started = $false

      for ($j = $i; $j -lt $lines.Count; $j++) {
        $currentLine = $lines[$j]
        $openCount = ([regex]::Matches($currentLine, '\{')).Count
        $closeCount = ([regex]::Matches($currentLine, '\}')).Count
        $braceDepth += $openCount
        if ($started) {
          $bodyLines.Add($currentLine) | Out-Null
        }
        if ($openCount -gt 0) {
          $started = $true
        }
        $braceDepth -= $closeCount
        if ($started -and $braceDepth -eq 0) {
          $endLine = $j + 1
          $body = ($bodyLines | Select-Object -SkipLast 1) -join "`n"
          $map[$name] = [pscustomobject]@{
            Body = $body
            StartLine = $startLine
            EndLine = $endLine
          }
          $i = $j
          break
        }
      }
    }
  }

  return $map
}

function Get-TsExportBlocks {
  param(
    [string]$Path
  )

  $lines = Get-Content -LiteralPath $Path
  $map = @{}

  for ($i = 0; $i -lt $lines.Count; $i++) {
    $line = $lines[$i]
    if ($line -match 'export interface\s+([A-Za-z0-9_]+)\s*\{') {
      $name = $matches[1]
      $startLine = $i + 1
      $braceDepth = 0
      $bodyLines = New-Object System.Collections.Generic.List[string]
      $started = $false

      for ($j = $i; $j -lt $lines.Count; $j++) {
        $currentLine = $lines[$j]
        $openCount = ([regex]::Matches($currentLine, '\{')).Count
        $closeCount = ([regex]::Matches($currentLine, '\}')).Count
        $braceDepth += $openCount
        if ($started) {
          $bodyLines.Add($currentLine) | Out-Null
        }
        if ($openCount -gt 0) {
          $started = $true
        }
        $braceDepth -= $closeCount
        if ($started -and $braceDepth -eq 0) {
          $endLine = $j + 1
          $body = ($bodyLines | Select-Object -SkipLast 1) -join "`n"
          $map[$name] = [pscustomobject]@{
            Body = $body
            StartLine = $startLine
            EndLine = $endLine
          }
          $i = $j
          break
        }
      }
    } elseif ($line -match 'export type\s+([A-Za-z0-9_]+)\s*=') {
      $name = $matches[1]
      $map[$name] = [pscustomobject]@{
        Body = $line
        StartLine = $i + 1
        EndLine = $i + 1
      }
    }
  }

  return $map
}

function Get-RustFieldNames {
  param(
    [string]$Body
  )

  $lines = $Body -split "`r?`n"
  $fields = foreach ($line in $lines) {
    $trimmed = $line.Trim()
    if ($trimmed -match '^pub\s+([A-Za-z0-9_]+)\s*:') {
      $matches[1]
    }
  }
  return @($fields | Sort-Object -Unique)
}

function Get-TsFieldNames {
  param(
    [string]$Body
  )

  $lines = $Body -split "`r?`n"
  $fields = foreach ($line in $lines) {
    $trimmed = $line.Trim()
    if ($trimmed -match '^([A-Za-z0-9_]+)\??\s*:') {
      $matches[1]
    }
  }
  return @($fields | Sort-Object -Unique)
}

function Get-ChangedNewLineNumbers {
  param(
    [string]$DiffText
  )

  $lineNumbers = New-Object System.Collections.Generic.List[int]
  $lines = $DiffText -split "`r?`n"
  $newLine = 0

  foreach ($line in $lines) {
    if ($line -match '^@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@') {
      $newLine = [int]$matches[1]
      continue
    }

    if ($line.StartsWith('+++') -or $line.StartsWith('---') -or $line.StartsWith('diff --git') -or $line.StartsWith('index ')) {
      continue
    }

    if ($line.StartsWith('+') -and -not $line.StartsWith('+++')) {
      $lineNumbers.Add($newLine) | Out-Null
      $newLine++
      continue
    }

    if ($line.StartsWith('-') -and -not $line.StartsWith('---')) {
      continue
    }

    if ($line.StartsWith(' ')) {
      $newLine++
      continue
    }
  }

  return @($lineNumbers | Sort-Object -Unique)
}

function Get-BlockNamesForChangedLines {
  param(
    [hashtable]$Blocks,
    [int[]]$ChangedLines
  )

  $names = New-Object System.Collections.Generic.List[string]

  foreach ($name in $Blocks.Keys) {
    $block = $Blocks[$name]
    foreach ($lineNumber in $ChangedLines) {
      if ($lineNumber -ge $block.StartLine -and $lineNumber -le $block.EndLine) {
        $names.Add($name) | Out-Null
        break
      }
    }
  }

  return @($names | Sort-Object -Unique)
}

function Get-AddedRustCommands {
  param(
    [string]$DiffText
  )

  $matches = [regex]::Matches($DiffText, '^\+\s*pub async fn\s+([A-Za-z0-9_]+)', [System.Text.RegularExpressions.RegexOptions]::Multiline)
  $names = foreach ($match in $matches) {
    $match.Groups[1].Value
  }
  return @($names | Sort-Object -Unique)
}

function Get-AllRustCommands {
  param(
    [string]$Path
  )

  $content = Get-Content -LiteralPath $Path -Raw
  $matches = [regex]::Matches($content, '#\[tauri::command\]\s*pub async fn\s+([A-Za-z0-9_]+)', [System.Text.RegularExpressions.RegexOptions]::Singleline)
  $names = foreach ($match in $matches) {
    $match.Groups[1].Value
  }
  return @($names | Sort-Object -Unique)
}

function Get-MainRegistrations {
  param(
    [string]$Path
  )

  $content = Get-Content -LiteralPath $Path -Raw
  $handlerMatch = [regex]::Match($content, 'generate_handler!\[(?<body>[\s\S]*?)\]')
  if (-not $handlerMatch.Success) {
    return @()
  }

  $body = $handlerMatch.Groups["body"].Value
  $tokenMatches = [regex]::Matches($body, '([A-Za-z_][A-Za-z0-9_]*)')
  $names = foreach ($match in $tokenMatches) {
    $match.Groups[1].Value
  }
  return @($names | Sort-Object -Unique)
}

function Get-MainImports {
  param(
    [string]$Path
  )

  $content = Get-Content -LiteralPath $Path -Raw
  $useMatch = [regex]::Match($content, 'use commands::\s*\{(?<body>[\s\S]*?)\};')
  if (-not $useMatch.Success) {
    return @()
  }

  $body = $useMatch.Groups["body"].Value
  $tokenMatches = [regex]::Matches($body, '([A-Za-z_][A-Za-z0-9_]*)')
  $names = foreach ($match in $tokenMatches) {
    $match.Groups[1].Value
  }
  return @($names | Sort-Object -Unique)
}

function Get-UiInvokeCommands {
  param(
    [string]$Path
  )

  $content = Get-Content -LiteralPath $Path -Raw
  $matches = [regex]::Matches($content, 'invoke(?:<[^>]+>)?\(\s*"([A-Za-z0-9_]+)"')
  $names = foreach ($match in $matches) {
    $match.Groups[1].Value
  }
  return @($names | Sort-Object -Unique)
}

function Get-ChangedUiInvokeCommands {
  param(
    [string]$DiffText
  )

  $matches = [regex]::Matches($DiffText, '^\+\s*(?:return\s+|await\s+)?invoke(?:<[^>]+>)?\(\s*"([A-Za-z0-9_]+)"', [System.Text.RegularExpressions.RegexOptions]::Multiline)
  $names = foreach ($match in $matches) {
    $match.Groups[1].Value
  }
  return @($names | Sort-Object -Unique)
}

function To-CamelCaseName {
  param(
    [string]$SnakeCase
  )

  $parts = $SnakeCase -split '_'
  if ($parts.Count -eq 0) {
    return $SnakeCase
  }

  $first = $parts[0]
  $rest = foreach ($part in ($parts | Select-Object -Skip 1)) {
    if ($part.Length -gt 0) {
      $part.Substring(0, 1).ToUpper() + $part.Substring(1)
    }
  }

  return ($first + ($rest -join ''))
}

Add-UniqueLine $checks "cargo test -p xt-core --lib"

$touchesDto = $normalized -contains "crates\xt-shared\src\dto.rs"
$touchesTsApi = $normalized -contains "ui\src\api\strings.ts"
$touchesUi = $normalized | Where-Object { $_ -like "ui\src\*" }
$touchesCommands = $normalized -contains "src-tauri\src\commands.rs"
$touchesMain = $normalized -contains "src-tauri\src\main.rs"
$touchesTauri = $normalized | Where-Object { $_ -like "src-tauri\*" }
$touchesCore = $normalized | Where-Object { $_ -like "crates\xt-core\*" }
$touchesTests = $normalized | Where-Object { $_ -like "crates\xt-core\tests\*" -or $_ -like "tests\*" }
$touchesStore = $normalized -contains "ui\src\stores\appStore.ts"
$touchesComponents = $normalized | Where-Object { $_ -like "ui\src\components\*" }
$touchesConfig = $normalized -contains "src-tauri\tauri.conf.json" -or $normalized -contains "dev.ps1"

if ($touchesUi) {
  Add-UniqueLine $checks "cd ui; npx tsc --noEmit"
}

if ($touchesCommands -or $touchesMain -or $touchesDto -or $touchesTsApi) {
  Add-UniqueLine $checks "cargo build -p xtranslator-tauri"
}

if ($touchesTests) {
  Add-UniqueLine $notes "Touched test artifacts; review whether fixtures or snapshots need regeneration."
}

if ($touchesDto -and -not $touchesTsApi) {
  Add-UniqueLine $notes "DTO changed: also inspect ui/src/api/strings.ts for TypeScript mirror sync."
}

if ($touchesTsApi -and -not $touchesDto) {
  Add-UniqueLine $notes "TypeScript API changed: confirm crates/xt-shared/src/dto.rs still matches the frontend contract."
}

if ($touchesCommands -and -not $touchesMain) {
  Add-UniqueLine $notes "IPC command implementation changed: confirm generate_handler! registration in src-tauri/src/main.rs."
}

if ($touchesStore -or $touchesComponents) {
  Add-UniqueLine $notes "Frontend table/editor changes: preserve selectedId-based updates and avoid index-based identity."
}

if ($touchesCommands -or $touchesStore -or $touchesComponents) {
  Add-UniqueLine $notes "Strings loading path should remain chunk-first via get_strings_chunk for large datasets."
}

if ($touchesConfig) {
  Add-UniqueLine $notes "Tauri dev on Windows should still use dev.ps1 or a separate Vite terminal; beforeDevCommand is intentionally echo-only."
}

if ($touchesCore) {
  Add-UniqueLine $notes "Core crate touched: prefer adding or updating targeted xt-core tests when behavior changes."
}

if ($touchesDto -or $touchesTsApi) {
  $dtoPath = "crates\xt-shared\src\dto.rs"
  $tsApiPath = "ui\src\api\strings.ts"
  if ((Test-Path -LiteralPath $dtoPath) -and (Test-Path -LiteralPath $tsApiPath)) {
    $rustNames = Get-RustDtoNames -Path $dtoPath
    $tsNames = Get-TsExportedNames -Path $tsApiPath
    $rustBlocks = Get-RustDtoBlocks -Path $dtoPath
    $tsBlocks = Get-TsExportBlocks -Path $tsApiPath
    $dtoDiff = Get-DiffText -Path $dtoPath
    $tsDiff = Get-DiffText -Path $tsApiPath

    $addedRustNames = @([regex]::Matches($dtoDiff, '^\+\s*pub struct\s+([A-Za-z0-9_]+)|^\+\s*pub enum\s+([A-Za-z0-9_]+)', [System.Text.RegularExpressions.RegexOptions]::Multiline) | ForEach-Object {
      if ($_.Groups[1].Success) { $_.Groups[1].Value } elseif ($_.Groups[2].Success) { $_.Groups[2].Value }
    } | Sort-Object -Unique)

    $addedTsNames = @([regex]::Matches($tsDiff, '^\+\s*export interface\s+([A-Za-z0-9_]+)|^\+\s*export type\s+([A-Za-z0-9_]+)', [System.Text.RegularExpressions.RegexOptions]::Multiline) | ForEach-Object {
      if ($_.Groups[1].Success) { $_.Groups[1].Value } elseif ($_.Groups[2].Success) { $_.Groups[2].Value }
    } | Sort-Object -Unique)

    foreach ($name in $addedRustNames) {
      if ($tsNames -notcontains $name) {
        Add-UniqueLine $warnings "Rust DTO '$name' exists in crates/xt-shared/src/dto.rs but no matching TypeScript export was found in ui/src/api/strings.ts."
      }
    }

    foreach ($name in $addedTsNames) {
      if ($rustNames -notcontains $name) {
        Add-UniqueLine $warnings "TypeScript export '$name' exists in ui/src/api/strings.ts but no matching Rust DTO was found in crates/xt-shared/src/dto.rs."
      }
    }

    $changedRustNames = Get-BlockNamesForChangedLines -Blocks $rustBlocks -ChangedLines (Get-ChangedNewLineNumbers -DiffText $dtoDiff)
    $changedTsNames = Get-BlockNamesForChangedLines -Blocks $tsBlocks -ChangedLines (Get-ChangedNewLineNumbers -DiffText $tsDiff)
    $fieldCheckNames = @($changedRustNames + $changedTsNames | Sort-Object -Unique)

    foreach ($name in $fieldCheckNames) {
      if (-not $rustBlocks.ContainsKey($name) -or -not $tsBlocks.ContainsKey($name)) {
        continue
      }

      $rustFields = Get-RustFieldNames -Body $rustBlocks[$name].Body
      $tsFields = Get-TsFieldNames -Body $tsBlocks[$name].Body
      if ($rustFields.Count -eq 0 -or $tsFields.Count -eq 0) {
        continue
      }

      $missingInTs = @($rustFields | Where-Object { $tsFields -notcontains $_ })
      $missingInRust = @($tsFields | Where-Object { $rustFields -notcontains $_ })

      if ($missingInTs.Count -gt 0) {
        Add-UniqueLine $warnings "DTO '$name' has Rust fields missing in TypeScript: $($missingInTs -join ', ')."
      }
      if ($missingInRust.Count -gt 0) {
        Add-UniqueLine $warnings "DTO '$name' has TypeScript fields missing in Rust: $($missingInRust -join ', ')."
      }
    }

    if ($touchesDto -and $touchesTsApi -and $addedRustNames.Count -eq 0 -and $addedTsNames.Count -eq 0 -and $fieldCheckNames.Count -eq 0) {
      Add-UniqueLine $notes "DTO and TS API both changed, but no changed exported type names were detected; still review field-level parity manually."
    }
  }
}

if ($touchesCommands -or $touchesMain) {
  $commandsPath = "src-tauri\src\commands.rs"
  $mainPath = "src-tauri\src\main.rs"
  if ((Test-Path -LiteralPath $commandsPath) -and (Test-Path -LiteralPath $mainPath)) {
    $commandDiff = Get-DiffText -Path $commandsPath
    $addedCommands = Get-AddedRustCommands -DiffText $commandDiff
    $mainImports = Get-MainImports -Path $mainPath
    $mainRegistrations = Get-MainRegistrations -Path $mainPath

    foreach ($commandName in $addedCommands) {
      if ($mainImports -notcontains $commandName) {
        Add-UniqueLine $warnings "New command '$commandName' was added in src-tauri/src/commands.rs but is missing from 'use commands::{ ... }' in src-tauri/src/main.rs."
      }
      if ($mainRegistrations -notcontains $commandName) {
        Add-UniqueLine $warnings "New command '$commandName' was added in src-tauri/src/commands.rs but is missing from generate_handler![] in src-tauri/src/main.rs."
      }
    }
  }
}

if ($touchesCommands -or $touchesMain -or $touchesTsApi) {
  $commandsPath = "src-tauri\src\commands.rs"
  $mainPath = "src-tauri\src\main.rs"
  $tsApiPath = "ui\src\api\strings.ts"

  if ((Test-Path -LiteralPath $commandsPath) -and (Test-Path -LiteralPath $mainPath) -and (Test-Path -LiteralPath $tsApiPath)) {
    $allCommands = Get-AllRustCommands -Path $commandsPath
    $mainImports = Get-MainImports -Path $mainPath
    $mainRegistrations = Get-MainRegistrations -Path $mainPath
    $uiInvokeCommands = Get-UiInvokeCommands -Path $tsApiPath

    $commandsDiff = Get-DiffText -Path $commandsPath
    $mainDiff = Get-DiffText -Path $mainPath
    $tsApiDiff = Get-DiffText -Path $tsApiPath

    $addedCommands = Get-AddedRustCommands -DiffText $commandsDiff
    $addedUiInvokes = Get-ChangedUiInvokeCommands -DiffText $tsApiDiff

    foreach ($commandName in $addedCommands) {
      if ($uiInvokeCommands -notcontains $commandName) {
        Add-UniqueLine $warnings "New backend command '$commandName' exists but no invoke(\"$commandName\") was found in ui/src/api/strings.ts."
        $wrapperName = To-CamelCaseName -SnakeCase $commandName
        $fixLines = New-FixLines
        Add-FixLine $fixLines "Patch-shaped suggestion for ui/src/api/strings.ts"
        Add-FixLine $fixLines "*** Begin Patch"
        Add-FixLine $fixLines "*** Update File: ui/src/api/strings.ts"
        Add-FixLine $fixLines "@@"
        Add-FixLine $fixLines "+export async function $wrapperName(/* args */): Promise</* return type */> {"
        Add-FixLine $fixLines "+  return invoke(""$commandName"", { /* args */ });"
        Add-FixLine $fixLines "+}"
        Add-FixLine $fixLines "*** End Patch"
        $fixText = Join-FixLines $fixLines
        Add-UniqueBlock $fixes $fixText
      }
    }

    foreach ($commandName in $addedUiInvokes) {
      if ($allCommands -notcontains $commandName) {
        Add-UniqueLine $warnings "Frontend invoke('$commandName') was added in ui/src/api/strings.ts but no matching #[tauri::command] backend function was found in src-tauri/src/commands.rs."
        $fixLines = New-FixLines
        Add-FixLine $fixLines "Patch-shaped suggestion for src-tauri/src/commands.rs"
        Add-FixLine $fixLines "*** Begin Patch"
        Add-FixLine $fixLines "*** Update File: src-tauri/src/commands.rs"
        Add-FixLine $fixLines "@@"
        Add-FixLine $fixLines "+#[tauri::command]"
        Add-FixLine $fixLines "+pub async fn $commandName(/* params */) -> Result</* return type */, String> {"
        Add-FixLine $fixLines "+    todo!()"
        Add-FixLine $fixLines "+}"
        Add-FixLine $fixLines "*** End Patch"
        $fixText = Join-FixLines $fixLines
        Add-UniqueBlock $fixes $fixText
      }
      if ($mainImports -notcontains $commandName) {
        Add-UniqueLine $warnings "Frontend invoke('$commandName') exists but '$commandName' is missing from 'use commands::{ ... }' in src-tauri/src/main.rs."
        $fixLines = New-FixLines
        Add-FixLine $fixLines "Patch-shaped suggestion for src-tauri/src/main.rs import list"
        Add-FixLine $fixLines "*** Begin Patch"
        Add-FixLine $fixLines "*** Update File: src-tauri/src/main.rs"
        Add-FixLine $fixLines "@@"
        Add-FixLine $fixLines "+    $commandName,"
        Add-FixLine $fixLines "*** End Patch"
        $fixText = Join-FixLines $fixLines
        Add-UniqueBlock $fixes $fixText
      }
      if ($mainRegistrations -notcontains $commandName) {
        Add-UniqueLine $warnings "Frontend invoke('$commandName') exists but '$commandName' is missing from generate_handler![] in src-tauri/src/main.rs."
        $fixLines = New-FixLines
        Add-FixLine $fixLines "Patch-shaped suggestion for src-tauri/src/main.rs generate_handler"
        Add-FixLine $fixLines "*** Begin Patch"
        Add-FixLine $fixLines "*** Update File: src-tauri/src/main.rs"
        Add-FixLine $fixLines "@@"
        Add-FixLine $fixLines "+            $commandName,"
        Add-FixLine $fixLines "*** End Patch"
        $fixText = Join-FixLines $fixLines
        Add-UniqueBlock $fixes $fixText
      }
    }

    if (($touchesCommands -or $touchesMain -or $touchesTsApi) -and $addedCommands.Count -eq 0 -and $addedUiInvokes.Count -eq 0) {
      Add-UniqueLine $notes "IPC-related files changed, but no new command/invoke names were added; registration or wrapper edits may still need manual review."
    }
  }
}

Write-Output "Changed files:"
$normalized | ForEach-Object { Write-Output "- $_" }
Write-Output ""
Write-Output "Recommended checks:"
$checks | ForEach-Object { Write-Output "- $_" }

if ($notes.Count -gt 0) {
  Write-Output ""
  Write-Output "Project-specific reminders:"
  $notes | ForEach-Object { Write-Output "- $_" }
}

if ($warnings.Count -gt 0) {
  Write-Output ""
  Write-Output "Detected issues:"
  $warnings | ForEach-Object { Write-Output "- $_" }
}

if ($fixes.Count -gt 0) {
  Write-Output ""
  Write-Output "Suggested fixes:"
  $fixes | ForEach-Object { Write-Output $_ }
}
