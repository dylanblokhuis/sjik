$env:PKG_CONFIG_PATH=(Get-Location).Path + '\ffmpeg-master-latest-win64-gpl-shared\lib\pkgconfig'

# Set source directory path
$source = '.\ffmpeg-master-latest-win64-gpl-shared\bin'

# Set target directory path
$target = '.\target\debug'

# Create the target directory if it does not exist
if (!(Test-Path -Path $target)) {
    New-Item -ItemType directory -Path $target
}

# Get all files from the source directory
$files = Get-ChildItem -Path $source -File

# Copy all files to the target directory
foreach ($file in $files) {
    Copy-Item -Path $file.FullName -Destination $target
}

$target = '.\target\release'

# Create the target directory if it does not exist
if (!(Test-Path -Path $target)) {
    New-Item -ItemType directory -Path $target
}

# Get all files from the source directory
$files = Get-ChildItem -Path $source -File

# Copy all files to the target directory
foreach ($file in $files) {
    Copy-Item -Path $file.FullName -Destination $target
}