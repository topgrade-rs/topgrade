#!/bin/bash

# Script de desinstalação do Topgrade e Topgrade GUI
# Uso: ./uninstall.sh [--system-wide]

set -e

SYSTEM_WIDE=false
if [[ "$1" == "--system-wide" ]]; then
    SYSTEM_WIDE=true
    if [[ $EUID -ne 0 ]]; then
        echo "Erro: Desinstalação system-wide requer privilégios de root (use sudo)"
        exit 1
    fi
fi

echo "=== Desinstalação do Topgrade e Topgrade GUI ==="
echo ""

# Verificar e parar processos em execução
if pgrep -f "topgrade-gui" > /dev/null; then
    echo "⚠ topgrade-gui está em execução. Encerrando..."
    pkill -f "topgrade-gui" || true
    sleep 1
fi

if pgrep -f "topgrade" > /dev/null && ! pgrep -f "topgrade-gui" > /dev/null; then
    echo "⚠ topgrade está em execução. Encerrando..."
    pkill -f "^topgrade$" || true
    sleep 1
fi

if [[ "$SYSTEM_WIDE" == true ]]; then
    echo "Desinstalando binários system-wide..."
    
    if [[ -f /usr/local/bin/topgrade ]]; then
        sudo rm -f /usr/local/bin/topgrade
        echo "✓ Removido /usr/local/bin/topgrade"
    fi
    
    if [[ -f /usr/local/bin/topgrade-gui ]]; then
        sudo rm -f /usr/local/bin/topgrade-gui
        echo "✓ Removido /usr/local/bin/topgrade-gui"
    fi
    
    # Remover backups
    sudo rm -f /usr/local/bin/topgrade.bak /usr/local/bin/topgrade-gui.bak
    
    # Remover arquivo desktop
    if [[ -f /usr/share/applications/topgrade-gui.desktop ]]; then
        sudo rm -f /usr/share/applications/topgrade-gui.desktop
        sudo update-desktop-database
        echo "✓ Removido arquivo desktop"
    fi
    
    # Remover ícone
    if [[ -f /usr/share/icons/hicolor/256x256/apps/topgrade.png ]]; then
        sudo rm -f /usr/share/icons/hicolor/256x256/apps/topgrade.png
        sudo gtk-update-icon-cache /usr/share/icons/hicolor/ 2>/dev/null || true
        echo "✓ Removido ícone"
    fi
else
    echo "Desinstalando binários locais..."
    
    if [[ -f ~/.local/bin/topgrade ]]; then
        rm -f ~/.local/bin/topgrade
        echo "✓ Removido ~/.local/bin/topgrade"
    fi
    
    if [[ -f ~/.local/bin/topgrade-gui ]]; then
        rm -f ~/.local/bin/topgrade-gui
        echo "✓ Removido ~/.local/bin/topgrade-gui"
    fi
    
    # Remover backups
    rm -f ~/.local/bin/topgrade.bak ~/.local/bin/topgrade-gui.bak
    
    # Remover arquivo desktop
    if [[ -f ~/.local/share/applications/topgrade-gui.desktop ]]; then
        rm -f ~/.local/share/applications/topgrade-gui.desktop
        update-desktop-database ~/.local/share/applications/ 2>/dev/null || true
        echo "✓ Removido arquivo desktop"
    fi
    
    # Remover ícone
    if [[ -f ~/.local/share/icons/hicolor/256x256/apps/topgrade.png ]]; then
        rm -f ~/.local/share/icons/hicolor/256x256/apps/topgrade.png
        gtk-update-icon-cache ~/.local/share/icons/hicolor/ 2>/dev/null || true
        echo "✓ Removido ícone"
    fi
fi

echo ""
echo "=== Desinstalação Concluída! ==="

