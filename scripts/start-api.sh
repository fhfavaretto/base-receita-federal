#!/bin/bash

# Script para iniciar o servidor API de consulta de CNPJ
# Uso: ./scripts/start-api.sh [--host HOST] [--port PORT] [--database PATH]

set -e  # Para na primeira ocorrência de erro

# Cores para output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Obtém o diretório do script e volta para a raiz do projeto
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

# Diretórios padrão
APP_DIR="app"
BINARY_PATH="$APP_DIR/target/release/cnpj-sqlite"
DB_DIR="$APP_DIR/dados-publicos"
DB_PATH="$DB_DIR/cnpj.db"

# Configurações padrão
HOST="${API_HOST:-127.0.0.1}"
PORT="${API_PORT:-8080}"

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

print_api() {
    echo -e "${CYAN}[API]${NC} $1"
}

# Função para exibir ajuda
show_help() {
    echo "Uso: $0 [OPÇÕES]"
    echo ""
    echo "Opções:"
    echo "  --host HOST       Endereço do servidor (padrão: 127.0.0.1)"
    echo "  --port PORT       Porta do servidor (padrão: 8080)"
    echo "  --database PATH   Caminho do banco SQLite (padrão: app/dados-publicos/cnpj.db)"
    echo "  --help, -h        Exibe esta ajuda"
    echo ""
    echo "Variáveis de ambiente:"
    echo "  API_HOST          Endereço do servidor (sobrescreve --host)"
    echo "  API_PORT          Porta do servidor (sobrescreve --port)"
    echo ""
    echo "Exemplos:"
    echo "  $0"
    echo "  $0 --port 3000"
    echo "  $0 --host 0.0.0.0 --port 3000"
    echo "  $0 --database /caminho/para/cnpj.db"
    exit 0
}

# Processa argumentos
while [[ $# -gt 0 ]]; do
    case $1 in
        --host)
            HOST="$2"
            shift 2
            ;;
        --port)
            PORT="$2"
            shift 2
            ;;
        --database)
            DB_PATH="$2"
            shift 2
            ;;
        --help|-h)
            show_help
            ;;
        *)
            print_error "Opção desconhecida: $1"
            echo "Use --help para ver as opções disponíveis"
            exit 1
            ;;
    esac
done

# Verifica se está no diretório correto
if [ ! -f "$APP_DIR/Cargo.toml" ]; then
    print_error "Arquivo Cargo.toml não encontrado. Execute este script na raiz do projeto."
    exit 1
fi

# Verifica se o Rust está instalado
if ! command -v cargo &> /dev/null; then
    print_error "Rust/Cargo não está instalado. Por favor, instale o Rust primeiro."
    echo "Visite: https://www.rust-lang.org/tools/install"
    exit 1
fi

print_info "═══════════════════════════════════════════════════════════"
print_info "Iniciando servidor API de consulta de CNPJ"
print_info "═══════════════════════════════════════════════════════════"
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
fi

# Verifica se o banco de dados existe
if [ ! -f "$DB_PATH" ]; then
    print_error "Banco de dados não encontrado: $DB_PATH"
    echo ""
    print_info "Para criar o banco de dados, execute:"
    echo "  ./scripts/run.sh"
    echo ""
    print_info "Ou manualmente:"
    echo "  cd app"
    echo "  cargo run --release -- download"
    echo "  cargo run --release -- process"
    exit 1
fi

# Verifica se o banco está acessível
if [ ! -r "$DB_PATH" ]; then
    print_error "Sem permissão de leitura no banco de dados: $DB_PATH"
    exit 1
fi

print_success "Banco de dados encontrado: $DB_PATH"
echo ""

# Valida porta
if ! [[ "$PORT" =~ ^[0-9]+$ ]] || [ "$PORT" -lt 1 ] || [ "$PORT" -gt 65535 ]; then
    print_error "Porta inválida: $PORT (deve ser um número entre 1 e 65535)"
    exit 1
fi

# Exibe informações de configuração
print_info "Configuração do servidor:"
echo "  📁 Banco de dados: $DB_PATH"
echo "  🌐 Host: $HOST"
echo "  🔌 Porta: $PORT"
echo "  🔗 URL: http://$HOST:$PORT"
echo ""

# Inicia o servidor
print_api "═══════════════════════════════════════════════════════════"
print_success "Iniciando servidor..."
print_api "═══════════════════════════════════════════════════════════"
echo ""

print_info "Endpoints disponíveis:"
echo "  📋 GET /cnpj/{cnpj}  - Consulta dados completos de um CNPJ"
echo "  ❤️  GET /health       - Verifica status do servidor"
echo ""

print_info "Exemplo de uso:"
echo "  curl http://$HOST:$PORT/cnpj/00000000000191"
echo "  curl http://$HOST:$PORT/health"
echo ""

print_warning "Pressione Ctrl+C para parar o servidor"
echo ""

# Executa o servidor
"$BINARY_PATH" server \
    --database "$DB_PATH" \
    --host "$HOST" \
    --port "$PORT"

