# Runs every example in turn. Needs a GPU and a display, so CI cannot do this.
$ErrorActionPreference = "Stop"

foreach ($example in @("hello_world", "triangle", "quad", "multiple_windows", "game_of_life", "lines", "sand", "hdr")) {
    Write-Host "== $example =="
    cargo run --example $example
    if ($LASTEXITCODE -ne 0) {
        throw "example failed: $example"
    }
}
