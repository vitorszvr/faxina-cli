# Instalação do Faxina CLI para Windows
# Este script baixa (se necessário) e instala o faxina-cli no diretório do usuário e adiciona ao PATH.

$ErrorActionPreference = "Stop"

$AppName = "faxina-cli"
$InstallDir = "$env:LOCALAPPDATA\faxina-cli"
$BinName = "faxina-cli.exe"
$CurrentDir = Get-Location

# 1. Verificar se o executável existe na pasta atual (instalação via zip baixado)
if (Test-Path "$CurrentDir\$BinName") {
    Write-Host "📦 Encontrado $BinName na pasta atual." -ForegroundColor Cyan
    $SourceBin = "$CurrentDir\$BinName"
} else {
    Write-Host "❌ $BinName não encontrado na pasta atual." -ForegroundColor Red
    Write-Host "   Certifique-se de ter extraído todo o conteúdo do arquivo .zip."
    exit 1
}

# 2. Criar diretório de instalação
if (-not (Test-Path $InstallDir)) {
    Write-Host "📁 Criando diretório de instalação: $InstallDir" -ForegroundColor Cyan
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
}

# 3. Copiar executável
Write-Host "🚀 Instalando em $InstallDir..." -ForegroundColor Cyan
Copy-Item -Path $SourceBin -Destination "$InstallDir\$BinName" -Force

# 4. Desbloquear o arquivo (Remove Mark of the Web / SmartScreen warning for this file)
Write-Host "🔓 Desbloqueando o executável (Unblock-File)..." -ForegroundColor Cyan
Unblock-File -Path "$InstallDir\$BinName"

# 5. Adicionar ao PATH do Usuário
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    Write-Host "🔗 Adicionando ao PATH do usuário..." -ForegroundColor Cyan
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
    Write-Host "✅ Caminho adicionado ao PATH." -ForegroundColor Green
    Write-Host "⚠️  IMPORTANTE: Você precisará FECHAR e REABRIR seu terminal para que o comando funcione." -ForegroundColor Yellow
} else {
    Write-Host "✅ O caminho já está no PATH." -ForegroundColor Green
}

Write-Host ""
Write-Host "🎉 Instalação concluída com sucesso!" -ForegroundColor Green
Write-Host "   Agora você pode usar o comando '$AppName' em qualquer terminal."
Write-Host ""
Write-Host "   Pressione Enter para sair..."
Read-Host
