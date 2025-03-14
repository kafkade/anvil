<#
.SYNOPSIS
Checks if directory is in environment variable PATH

.DESCRIPTION
Checks if a directory is included in the environment variable PATH. If not, it
will be added. The check is case insensitive and trims the trailing backslash.

.PARAMETER directory
Directory to check

.EXAMPLE
Test_InPath "C:\Program Files\Git\"

.NOTES
General notes
#>
function Test_InPath($directory) {
    $separator = [IO.Path]::PathSeparator
    $path = [Environment]::GetEnvironmentVariable("PATH", "Machine")
    return ($path -split $separator).ToLower().TrimEnd("\") -contains $directory.ToLower().TrimEnd("\")
}

<#
.SYNOPSIS
Adds directory to environment variable PATH

.DESCRIPTION
Adds a directory to the environment variable PATH. The directory is modified to
be lower case with trailings trimmed and added to the end of the PATH variable.

.PARAMETER directory
Directory to add

.EXAMPLE
Add_ToPath "C:\Program Files\Git\"

.NOTES
General notes
#>
function Add_ToPath($directory) {
    $separator = [IO.Path]::PathSeparator
    $path = [Environment]::GetEnvironmentVariable("PATH", "Machine").TrimEnd($separator) + $separator + $directory.ToLower().TrimEnd("\")
    [Environment]::SetEnvironmentVariable("PATH", $path, "Machine")

    $paths = "Machine", "User" |
        ForEach-Object {
            [Environment]::GetEnvironmentVariable("PATH", $_) -split $separator
        } |
        Select-Object -Unique

    $env:path = $paths -join $separator

}

Export-ModuleMember -Function Test_InPath, Add_ToPath
