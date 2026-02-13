#!/bin/bash
set -e

# ─── Configurações ──────────────────────────────────────────
REPO="vitorszvr/faxina-cli"
BINARY_NAME="faxina-cli"

# ─── Detectar OS e Arquitetura ──────────────────────────────
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux*)
    case "$ARCH" in
      x86_64)  TARGET="x86_64-unknown-linux-musl" ;;
      aarch64) TARGET="aarch64-unknown-linux-musl" ;;
      *) echo "❌ Arquitetura não suportada: $ARCH"; exit 1 ;;
    esac
    ;;
  Darwin*)
    case "$ARCH" in
      x86_64)  TARGET="x86_64-apple-darwin" ;;
      arm64)   TARGET="aarch64-apple-darwin" ;;
      *) echo "❌ Arquitetura não suportada: $ARCH"; exit 1 ;;
    esac
    ;;
  *)
    echo "❌ Sistema operacional não suportado: $OS"
    exit 1
    ;;
esac

# ─── Buscar última versão ───────────────────────────────────
echo "🔍 Buscando última versão..."
TAG=$(curl -sS "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')

if [ -z "$TAG" ]; then
  echo "❌ Não foi possível encontrar a última versão. Verifique se existem releases em:"
  echo "   https://github.com/$REPO/releases"
  exit 1
fi

# ─── Montar URL de download ─────────────────────────────────
ASSET_NAME="${BINARY_NAME}-${TARGET}.tar.gz"
URL="https://github.com/$REPO/releases/download/$TAG/$ASSET_NAME"

echo "🚀 Baixando ${BINARY_NAME} ${TAG} para ${TARGET}..."
echo "   $URL"

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

curl -fSL "$URL" -o "$TMPDIR/$ASSET_NAME"

# ─── Extrair ────────────────────────────────────────────────
echo "📦 Extraindo..."
tar -xzf "$TMPDIR/$ASSET_NAME" -C "$TMPDIR"

# Encontrar o binário (pode estar em subdiretório)
BIN_PATH=$(find "$TMPDIR" -name "$BINARY_NAME" -type f | head -1)

if [ -z "$BIN_PATH" ]; then
  echo "❌ Binário '$BINARY_NAME' não encontrado no arquivo baixado."
  exit 1
fi

chmod +x "$BIN_PATH"

# ─── Instalar ───────────────────────────────────────────────
INSTALL_DIR="/usr/local/bin"

if [ -w "$INSTALL_DIR" ]; then
  mv "$BIN_PATH" "$INSTALL_DIR/$BINARY_NAME"
else
  echo "🔒 Permissão necessária para instalar em $INSTALL_DIR"
  sudo mv "$BIN_PATH" "$INSTALL_DIR/$BINARY_NAME"
fi

echo ""
echo "✅ ${BINARY_NAME} ${TAG} instalado com sucesso!"
echo "   Localização: $INSTALL_DIR/$BINARY_NAME"
echo ""
echo "   Experimente:  ${BINARY_NAME} --help"
