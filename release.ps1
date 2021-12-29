[CmdletBinding()]
param(
  [Parameter(Mandatory)]
  [String] $Ref
)

$ErrorActionPreference = 'Stop'

$null, $type, $name = $Ref -split '/'

if ($type -ne 'tags') {
  throw "Expected ref type in '$Ref' to be 'tags' but was '$type'"
}

if ($name -match 'v(?<version>\d+\.\d+\.\d+)') {
  $manifest = Get-Content -Path 'src\manifest.json' | ConvertFrom-Json

  if ($manifest.version -ne $Matches.version) {
    throw "Tagged version '$($Matches.version)' in '$Ref' does not match version '$($manifest.version)' found in src/manifest.json"
  }
}
else {
  throw "Tag '$Ref' is not a valid version tag, should match format 'v\d+\.\d+\.\d+'"
}

cargo test --manifest-path=native\Cargo.toml

if ($LastExitCode) {
  throw 'Native tests failed'
}

cargo build --release --manifest-path=native\Cargo.toml

if ($LastExitCode) {
  throw 'Native build failed'
}

@(
  'native\target\release\archive.exe'
  Get-ChildItem -Path 'src'
) | Compress-Archive -DestinationPath 'archive.zip' -Force
