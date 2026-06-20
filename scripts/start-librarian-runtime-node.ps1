# Librarian Runtime Node Service Launcher
# This script is used by the Windows Service (via NSSM) to start the router.

$WorkDir = "G:\openwork\librarian-runtime-node"
$PythonExe = "C:\Python314\python.exe"
$RouterScript = "G:\openwork\librarian-runtime-node\router\router.py"
$Port = 9130

# Set working directory
Set-Location -LiteralPath $WorkDir

# Launch router with unbuffered output for better logging
& $PythonExe -u $RouterScript --port $Port
