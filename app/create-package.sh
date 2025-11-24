#!/bin/bash

# Script para criar pacote de distribuição com executáveis

set -e

VERSION=$(grep "^version" Cargo.toml | cut -d '"' -f 2)
PACKAGE_NAME="cnpj-sqlite-v${VERSION}"
BUILD_DIR="release-builds"

echo "📦 Criando pacote de distribuição: $PACKAGE_NAME"
echo ""

# Criar diretório do pacote
rm -rf "$PACKAGE_NAME"
mkdir -p "$PACKAGE_NAME"

# Copiar executáveis disponíveis
if [ -f "$BUILD_DIR/linux-x86_64/cnpj-sqlite" ]; then
    cp "$BUILD_DIR/linux-x86_64/cnpj-sqlite" "$PACKAGE_NAME/"
    chmod +x "$PACKAGE_NAME/cnpj-sqlite"
    echo "✅ Copiado executável Linux"
fi

if [ -f "$BUILD_DIR/windows-x86_64/cnpj-sqlite.exe" ]; then
    cp "$BUILD_DIR/windows-x86_64/cnpj-sqlite.exe" "$PACKAGE_NAME/"
    echo "✅ Copiado executável Windows"
fi

if [ -f "$BUILD_DIR/macos-x86_64/cnpj-sqlite" ]; then
    cp "$BUILD_DIR/macos-x86_64/cnpj-sqlite" "$PACKAGE_NAME/cnpj-sqlite-macos-intel"
    chmod +x "$PACKAGE_NAME/cnpj-sqlite-macos-intel"
    echo "✅ Copiado executável macOS Intel"
fi

if [ -f "$BUILD_DIR/macos-arm64/cnpj-sqlite" ]; then
    cp "$BUILD_DIR/macos-arm64/cnpj-sqlite" "$PACKAGE_NAME/cnpj-sqlite-macos-arm64"
    chmod +x "$PACKAGE_NAME/cnpj-sqlite-macos-arm64"
    echo "✅ Copiado executável macOS Apple Silicon"
fi

# Criar README
cat > "$PACKAGE_NAME/README.txt" << EOF
CNPJ SQLite v${VERSION}
=====================

Conversor de dados públicos de CNPJ para SQLite em Rust.

Executáveis incluídos:
- cnpj-sqlite: Linux (x86_64)
- cnpj-sqlite.exe: Windows (x86_64)
- cnpj-sqlite-macos-intel: macOS Intel (x86_64)
- cnpj-sqlite-macos-arm64: macOS Apple Silicon (ARM64)

Uso Básico:
-----------

1. Download dos arquivos:
   ./cnpj-sqlite download

2. Processar e criar banco:
   ./cnpj-sqlite process

3. Iniciar servidor web:
   ./cnpj-sqlite server

4. Consultar CNPJ:
   ./cnpj-sqlite --help

Servidor Web:
-------------
O servidor web permite consultar CNPJs via API REST:
- GET /cnpj/{cnpj} - Consulta dados de um CNPJ
- GET /health - Status do servidor
- GET /api/database/status - Status do banco
- GET /api/progress - Progresso de operações
- POST /api/download/start - Iniciar download
- POST /api/process/start - Iniciar processamento

Iniciar servidor:
  ./cnpj-sqlite server --port 8080

Acesse http://localhost:8080 no navegador.

Requisitos:
-----------
- Linux: Nenhum (executável standalone)
- Windows: Nenhum (executável standalone)
- macOS: Nenhum (executável standalone)

Nota: Os executáveis são standalone e não requerem instalação de Rust.
EOF

# Copiar LICENSE se existir
if [ -f "../LICENSE" ]; then
    cp "../LICENSE" "$PACKAGE_NAME/"
elif [ -f "LICENSE" ]; then
    cp "LICENSE" "$PACKAGE_NAME/"
fi

echo ""
echo "📦 Criando arquivos compactados..."

# Criar tar.gz
tar -czf "${PACKAGE_NAME}.tar.gz" "$PACKAGE_NAME" 2>/dev/null && \
    echo "✅ Criado ${PACKAGE_NAME}.tar.gz" || \
    echo "⚠️  Não foi possível criar .tar.gz"

# Criar zip (se zip estiver disponível)
if command -v zip &> /dev/null; then
    zip -r "${PACKAGE_NAME}.zip" "$PACKAGE_NAME" > /dev/null 2>&1 && \
        echo "✅ Criado ${PACKAGE_NAME}.zip" || \
        echo "⚠️  Não foi possível criar .zip"
else
    echo "⚠️  zip não encontrado, pulando criação de .zip"
fi

echo ""
echo "✅ Pacote criado: $PACKAGE_NAME/"
echo "📊 Tamanho:"
du -sh "$PACKAGE_NAME" 2>/dev/null || echo "   (não foi possível calcular)"
echo ""
echo "💡 Para distribuir, envie o diretório $PACKAGE_NAME/ ou os arquivos .tar.gz/.zip"


