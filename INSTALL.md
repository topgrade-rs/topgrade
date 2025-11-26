# Guia de Instalação - Topgrade GUI

Este guia mostra como instalar o Topgrade (CLI) e Topgrade GUI no seu sistema Linux.

## Opção 1: Instalação usando Cargo (Recomendado)

### Passo 1: Compilar ambos os binários

```bash
# Compilar o topgrade CLI
cargo build --release --bin topgrade

# Compilar o topgrade-gui
cargo build --release --bin topgrade-gui --features gui
```

### Passo 2: Instalar no sistema

```bash
# Criar diretório para binários locais (se não existir)
mkdir -p ~/.local/bin

# Copiar os binários
cp target/release/topgrade ~/.local/bin/
cp target/release/topgrade-gui ~/.local/bin/

# Tornar executáveis
chmod +x ~/.local/bin/topgrade
chmod +x ~/.local/bin/topgrade-gui

# Adicionar ao PATH (se ainda não estiver)
if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
    source ~/.bashrc
fi
```

### Passo 3: Instalar arquivo desktop

```bash
# Criar diretório de aplicações (se não existir)
mkdir -p ~/.local/share/applications

# Copiar arquivo desktop
cp topgrade-gui.desktop ~/.local/share/applications/

# Atualizar banco de dados de aplicações
update-desktop-database ~/.local/share/applications/
```

### Passo 4: Instalar ícone (opcional)

```bash
# Criar diretório de ícones
mkdir -p ~/.local/share/icons/hicolor/256x256/apps

# Copiar ícone
cp doc/topgrade.png ~/.local/share/icons/hicolor/256x256/apps/topgrade.png

# Atualizar cache de ícones
gtk-update-icon-cache ~/.local/share/icons/hicolor/ 2>/dev/null || true
```

## Opção 2: Instalação System-Wide (requer sudo)

### Passo 1: Compilar ambos os binários

```bash
# Compilar o topgrade CLI
cargo build --release --bin topgrade

# Compilar o topgrade-gui
cargo build --release --bin topgrade-gui --features gui
```

### Passo 2: Instalar no sistema

```bash
# Copiar binários para /usr/local/bin
sudo cp target/release/topgrade /usr/local/bin/
sudo cp target/release/topgrade-gui /usr/local/bin/

# Tornar executáveis
sudo chmod +x /usr/local/bin/topgrade
sudo chmod +x /usr/local/bin/topgrade-gui
```

### Passo 3: Instalar arquivo desktop

```bash
# Copiar arquivo desktop
sudo cp topgrade-gui.desktop /usr/share/applications/

# Atualizar banco de dados de aplicações
sudo update-desktop-database
```

### Passo 4: Instalar ícone

```bash
# Criar diretório de ícones
sudo mkdir -p /usr/share/icons/hicolor/256x256/apps

# Copiar ícone
sudo cp doc/topgrade.png /usr/share/icons/hicolor/256x256/apps/topgrade.png

# Atualizar cache de ícones
sudo gtk-update-icon-cache /usr/share/icons/hicolor/
```

## Verificação da Instalação

Após a instalação, verifique se tudo está funcionando:

```bash
# Verificar se topgrade está no PATH
which topgrade
which topgrade-gui

# Testar execução
topgrade --version
topgrade-gui --version  # Pode não ter flag --version, mas deve abrir a GUI

# Verificar se o desktop file está instalado
ls ~/.local/share/applications/topgrade-gui.desktop
# ou
ls /usr/share/applications/topgrade-gui.desktop
```

## Executando

Após a instalação, você pode executar:

- **CLI**: `topgrade` (no terminal)
- **GUI**: `topgrade-gui` (no terminal) ou através do menu de aplicações do sistema

## Desinstalação

### Se instalou localmente (~/.local):

```bash
rm ~/.local/bin/topgrade
rm ~/.local/bin/topgrade-gui
rm ~/.local/share/applications/topgrade-gui.desktop
rm ~/.local/share/icons/hicolor/256x256/apps/topgrade.png
update-desktop-database ~/.local/share/applications/
```

### Se instalou system-wide:

```bash
sudo rm /usr/local/bin/topgrade
sudo rm /usr/local/bin/topgrade-gui
sudo rm /usr/share/applications/topgrade-gui.desktop
sudo rm /usr/share/icons/hicolor/256x256/apps/topgrade.png
sudo update-desktop-database
```

