#!/bin/bash

# Script de instalação do Topgrade e Topgrade GUI
# Uso: ./install.sh [--system-wide]

set -e

SYSTEM_WIDE=false
if [[ "$1" == "--system-wide" ]]; then
    SYSTEM_WIDE=true
    if [[ $EUID -ne 0 ]]; then
        echo "Error: Instalação system-wide requer privilégios de root (use sudo)"
        exit 1
    fi
fi

echo "=== Instalação do Topgrade e Topgrade GUI ==="
echo ""

# Verificar se estamos no diretório correto
if [[ ! -f "Cargo.toml" ]]; then
    echo "Error: Execute este script no diretório raiz do projeto topgrade"
    exit 1
fi

# Passo 1: Compilar
echo "Passo 1: Compilando binários..."
echo "Compilando topgrade CLI..."
cargo build --release --bin topgrade

echo "Compilando topgrade-gui..."
cargo build --release --bin topgrade-gui --features gui

echo "✓ Compilação concluída!"
echo ""

# Passo 2: Verificar e parar processors em execução
echo "Passo 2: Verificando processors em execução..."
if pgrep -f "topgrade-gui" > /dev/null; then
    echo "⚠ topgrade-gui está em execução. Tentando encerrar..."
    pkill -f "topgrade-gui" || true
    sleep 1
    if pgrep -f "topgrade-gui" > /dev/null; then
        echo "❌ Não foi possível encerrar topgrade-gui. Por favor, feche manualmente e tente novamente."
        exit 1
    fi
    echo "✓ topgrade-gui encerrado"
fi

# Passo 3: Instalar binários
if [[ "$SYSTEM_WIDE" == true ]]; then
    echo "Passo 3: Instalando binários system-wide..."
    # Fazer backup se existir
    if [[ -f /usr/local/bin/topgrade ]]; then
        sudo cp /usr/local/bin/topgrade /usr/local/bin/topgrade.bak 2>/dev/null || true
    fi
    if [[ -f /usr/local/bin/topgrade-gui ]]; then
        sudo cp /usr/local/bin/topgrade-gui /usr/local/bin/topgrade-gui.bak 2>/dev/null || true
    fi

    # Remover versões antigas antes de instalar
    sudo rm -f /usr/local/bin/topgrade /usr/local/bin/topgrade-gui

    sudo cp target/release/topgrade /usr/local/bin/
    sudo cp target/release/topgrade-gui /usr/local/bin/
    sudo chmod +x /usr/local/bin/topgrade
    sudo chmod +x /usr/local/bin/topgrade-gui
    BIN_DIR="/usr/local/bin"
    APP_DIR="/usr/share/applications"
    ICON_DIR="/usr/share/icons/hicolor/256x256/apps"
else
    echo "Passo 3: Instalando binários localmente..."
    mkdir -p ~/.local/bin

    # Fazer backup se existir
    if [[ -f ~/.local/bin/topgrade ]]; then
        cp ~/.local/bin/topgrade ~/.local/bin/topgrade.bak 2>/dev/null || true
    fi
    if [[ -f ~/.local/bin/topgrade-gui ]]; then
        cp ~/.local/bin/topgrade-gui ~/.local/bin/topgrade-gui.bak 2>/dev/null || true
    fi

    # Remover versões antigas antes de instalar
    rm -f ~/.local/bin/topgrade ~/.local/bin/topgrade-gui

    cp target/release/topgrade ~/.local/bin/
    cp target/release/topgrade-gui ~/.local/bin/
    chmod +x ~/.local/bin/topgrade
    chmod +x ~/.local/bin/topgrade-gui

    # Adicionar ao PATH se necessário
    if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
        echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
        echo "✓ Adicionado ~/.local/bin ao PATH no ~/.bashrc"
        echo "  Execute 'source ~/.bashrc' ou abra um novo terminal para usar os commandos"
    fi

    BIN_DIR="$HOME/.local/bin"
    APP_DIR="$HOME/.local/share/applications"
    ICON_DIR="$HOME/.local/share/icons/hicolor/256x256/apps"
fi

echo "✓ Binários instalados em $BIN_DIR"
echo ""

# Passo 4: Instalar arquivo desktop
echo "Passo 4: Instalando arquivo desktop..."
mkdir -p "$APP_DIR"
cp topgrade-gui.desktop "$APP_DIR/"

if [[ "$SYSTEM_WIDE" == true ]]; then
    update-desktop-database
else
    update-desktop-database "$APP_DIR" 2>/dev/null || true
fi

echo "✓ Arquivo desktop instalado"
echo ""

# Passo 5: Instalar ícone
echo "Passo 5: Instalando ícone..."
if [[ -f "doc/topgrade.png" ]]; then
    mkdir -p "$ICON_DIR"
    cp doc/topgrade.png "$ICON_DIR/topgrade.png"

    if [[ "$SYSTEM_WIDE" == true ]]; then
        gtk-update-icon-cache /usr/share/icons/hicolor/ 2>/dev/null || true
    else
        gtk-update-icon-cache ~/.local/share/icons/hicolor/ 2>/dev/null || true
    fi

    echo "✓ Ícone instalado"
else
    echo "⚠ Aviso: Arquivo doc/topgrade.png não encontrado, pulando instalação do ícone"
fi
echo ""

# Verificação
echo "=== Verificação da Instalação ==="
echo ""

if command -v topgrade &> /dev/null; then
    echo "✓ topgrade encontrado: $(which topgrade)"
    topgrade --version 2>/dev/null || echo "  (versão não disponível via --version)"
else
    echo "⚠ topgrade não encontrado no PATH"
    echo "  Localização: $BIN_DIR/topgrade"
fi

if command -v topgrade-gui &> /dev/null; then
    echo "✓ topgrade-gui encontrado: $(which topgrade-gui)"
else
    echo "⚠ topgrade-gui não encontrado no PATH"
    echo "  Localização: $BIN_DIR/topgrade-gui"
    if [[ "$SYSTEM_WIDE" == false ]]; then
        echo "  Execute 'source ~/.bashrc' ou abra um novo terminal"
    fi
fi

if [[ -f "$APP_DIR/topgrade-gui.desktop" ]]; then
    echo "✓ Arquivo desktop instalado: $APP_DIR/topgrade-gui.desktop"
else
    echo "⚠ Arquivo desktop não encontrado"
fi

echo ""
echo "=== Instalação Concluída! ==="
echo ""
echo "Você pode executar:"
echo "  - CLI: topgrade"
echo "  - GUI: topgrade-gui (ou pelo menu de aplicações)"
echo ""
