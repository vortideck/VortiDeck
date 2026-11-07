param(
    [Parameter(Mandatory=$true)]
    [string]$version,
    [string]$message = "version update"
)

# Colors for output
$green = [System.ConsoleColor]::Green
$red = [System.ConsoleColor]::Red
$yellow = [System.ConsoleColor]::Yellow

function Write-ColoredOutput {
    param($message, $color)
    Write-Host "→ $message" -ForegroundColor $color
}

try {
    # Get the previous commit message
    $previousCommitMessage = git log -1 --pretty=%B

    # Stage changes
    Write-ColoredOutput "Staging changes..." $yellow
    git add .

    # Combine previous commit message with new message
    $fullMessage = "$message"


    # Format commit message properly
    $commitMessages = $fullMessage -split ";"
    $commitArgs = @()
    foreach ($commitMsg in $commitMessages) {
        if ($commitMsg.Trim()) {
            $commitArgs += "-m"
            $commitArgs += $commitMsg.Trim()
        }
    }

    # Single commit with multiple -m flags
    if ($commitArgs.Count -gt 0) {
        Write-ColoredOutput "Committing changes..." $yellow
        git commit @commitArgs
    } else {
        Write-ColoredOutput "No commit message provided. Skipping commit." $red
    }

    # Push changes
    Write-ColoredOutput "Pushing changes..." $yellow
    git push
    
    # Create new tag
    Write-ColoredOutput "Creating new tag..." $yellow
    git tag $version
    
    # Push new tag
    Write-ColoredOutput "Pushing new tag..." $yellow
    git push origin $version
    
    Write-ColoredOutput "All done! 🎉" $green
} catch {
    Write-ColoredOutput "Error: $_" $red
    exit 1
}
