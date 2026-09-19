$ErrorActionPreference = 'Stop'

if (
    (Get-Command Get-InstalledPSResource -ErrorAction SilentlyContinue) -and
    (Get-Command Uninstall-PSResource -ErrorAction SilentlyContinue)
) {
    Get-InstalledPSResource -Version '*' |
        Where-Object Type -eq 'Module' |
        Group-Object Name |
        ForEach-Object {
            $_.Group |
                Sort-Object Version -Descending |
                Select-Object -Skip 1 |
                Uninstall-PSResource -Confirm:$false
        }
    return
}

Get-InstalledModule | ForEach-Object {
    Get-InstalledModule -Name $_.Name -AllVersions |
        Sort-Object Version -Descending |
        Select-Object -Skip 1 |
        Uninstall-Module -Confirm:$false
}
