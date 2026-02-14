# Instalação do Faxina CLI para Windows
# Este script baixa (se necessário) e instala o faxina-cli no diretório do usuário e adiciona ao PATH.

$ErrorActionPreference = "Stop"

$AppName = "faxina-cli"
$InstallDir = "$env:LOCALAPPDATA\faxina-cli"
$BinName = "faxina-cli.exe"
$CurrentDir = Get-Location
$Repo = "vitorszvr/faxina-cli"

# 1. Obter executável (Local ou Download)
if (Test-Path "$CurrentDir\$BinName") {
    Write-Host "📦 Encontrado $BinName na pasta atual." -ForegroundColor Cyan
    $SourceBin = "$CurrentDir\$BinName"
} else {
    Write-Host "☁️  Buscando última versão no GitHub..." -ForegroundColor Cyan
    try {
        $Latest = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest"
        $Asset = $Latest.assets | Where-Object { $_.name -like "*Windows-x86_64.zip" }
        
        if (-not $Asset) {
            Write-Error "Release Windows não encontrada."
        }

        $DownloadUrl = $Asset.browser_download_url
        $ZipPath = "$env:TEMP\faxina-cli.zip"
        
        Write-Host "⬇️  Baixando: $($Asset.name)..." -ForegroundColor Cyan
        Invoke-WebRequest -Uri $DownloadUrl -OutFile $ZipPath
        
        Write-Host "📦 Extraindo..." -ForegroundColor Cyan
        Expand-Archive -Path $ZipPath -DestinationPath "$env:TEMP\faxina-cli-install" -Force
        
        # Encontrar o binário extraído
        $SourceBin = Get-ChildItem -Path "$env:TEMP\faxina-cli-install" -Filter "$BinName" -Recurse | Select-Object -First 1 -ExpandProperty FullName
        
        if (-not $SourceBin) {
            Write-Error "Binário não encontrado dentro do zip."
        }
    } catch {
        Write-Host "❌ Erro ao baixar atualização: $_" -ForegroundColor Red
        exit 1
    }
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
