# The same four checks CI runs. Formatting needs nightly; see CONTRIBUTING.md.
$ErrorActionPreference = "Stop"

function Invoke-Check {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Command)
    & $Command[0] $Command[1..($Command.Length - 1)]
    if ($LASTEXITCODE -ne 0) {
        throw "failed: $($Command -join ' ')"
    }
}

Invoke-Check cargo +nightly fmt -- --check --color always
Invoke-Check cargo clippy --all-targets --all-features -- -D warnings
Invoke-Check cargo test --all-features
$env:RUSTDOCFLAGS = "-Dwarnings"
Invoke-Check cargo doc --no-deps --all-features
