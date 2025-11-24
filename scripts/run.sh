#!/bin/bash

# Script de inicialização do processo completo de CNPJ-SQLite
# Executa as 3 partes: Download, Processamento e CNAE Secundário
# Execute este script na raiz do projeto

set -e  # Para na primeira ocorrência de erro

# Cores para output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Obtém o diretório do script e volta para a raiz do projeto
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

# Diretórios
APP_DIR="app"
BINARY_PATH="$APP_DIR/target/release/cnpj-sqlite"
ZIP_DIR="$APP_DIR/dados-publicos-zip"
DB_DIR="$APP_DIR/dados-publicos"
DB_PATH="$DB_DIR/cnpj.db"

# Função para imprimir mensagens
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Verifica se está no diretório correto
if [ ! -f "$APP_DIR/Cargo.toml" ]; then
    print_error "Arquivo Cargo.toml não encontrado. Execute este script na raiz do projeto."
    exit 1
fi

# Verifica se o Rust está instalado
if ! command -v cargo &> /dev/null; then
    print_error "Rust/Cargo não está instalado. Por favor, instale o Rust primeiro."
    exit 1
fi

print_info "Iniciando processo completo de CNPJ-SQLite..."
echo ""

# Verifica se o binário existe, se não, compila
if [ ! -f "$BINARY_PATH" ]; then
    print_warning "Binário não encontrado. Compilando o projeto..."
    cd "$APP_DIR"
    cargo build --release
    cd ..
    print_success "Compilação concluída!"
    echo ""
else
    print_info "Binário encontrado: $BINARY_PATH"
    echo ""
fi

# Parte 1: Download
print_info "═══════════════════════════════════════════════════════════"
print_info "PARTE 1/3: Download dos arquivos ZIP"
print_info "═══════════════════════════════════════════════════════════"
echo ""

if [ -d "$ZIP_DIR" ] && [ "$(ls -A $ZIP_DIR/*.zip 2>/dev/null)" ]; then
    print_warning "Diretório $ZIP_DIR já contém arquivos ZIP."
    read -p "Deseja baixar novamente? (s/N): " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Ss]$ ]]; then
        print_info "Pulando download. Usando arquivos existentes."
    else
        "$BINARY_PATH" download --output "$ZIP_DIR"
        print_success "Download concluído!"
    fi
else
    print_info "Iniciando download dos arquivos..."
    "$BINARY_PATH" download --output "$ZIP_DIR"
    print_success "Download concluído!"
fi

echo ""

# Parte 2: Processamento
print_info "═══════════════════════════════════════════════════════════"
print_info "PARTE 2/3: Processamento dos arquivos CSV para SQLite"
print_info "═══════════════════════════════════════════════════════════"
echo ""

if [ -f "$DB_PATH" ]; then
    print_warning "Banco de dados $DB_PATH já existe."
    read -p "Deseja reprocessar? Isso apagará o banco existente. (s/N): " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Ss]$ ]]; then
        print_info "Pulando processamento. Usando banco existente."
    else
        print_info "Removendo banco existente..."
        rm -f "$DB_PATH" "$DB_PATH-shm" "$DB_PATH-wal"
        print_info "Iniciando processamento..."
        "$BINARY_PATH" process --input "$ZIP_DIR" --output "$DB_DIR" --cleanup true
        print_success "Processamento concluído!"
    fi
else
    print_info "Iniciando processamento..."
    "$BINARY_PATH" process --input "$ZIP_DIR" --output "$DB_DIR" --cleanup true
    print_success "Processamento concluído!"
fi

echo ""

# Parte 3: CNAE Secundário
print_info "═══════════════════════════════════════════════════════════"
print_info "PARTE 3/3: Criação da tabela de CNAE Secundário"
print_info "═══════════════════════════════════════════════════════════"
echo ""

if [ ! -f "$DB_PATH" ]; then
    print_error "Banco de dados não encontrado. Execute a parte 2 primeiro."
    exit 1
fi

print_info "Criando tabela de CNAE Secundário..."
"$BINARY_PATH" cnae-secundaria --database "$DB_PATH"
print_success "Tabela de CNAE Secundário criada!"

echo ""
print_info "═══════════════════════════════════════════════════════════"
print_success "Processo completo finalizado com sucesso!"
print_info "═══════════════════════════════════════════════════════════"
echo ""
print_info "Banco de dados disponível em: $DB_PATH"
print_info "Para iniciar o servidor API, execute:"
echo "  ./scripts/start-api.sh"
echo "  ou"
echo "  $BINARY_PATH server --database $DB_PATH"
echo ""

