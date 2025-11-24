@echo off
REM Script para iniciar o servidor API de consulta de CNPJ
REM Uso: scripts\start-api.bat [--host HOST] [--port PORT] [--database PATH]

setlocal enabledelayedexpansion

REM Obtém o diretório do script e volta para a raiz do projeto
cd /d "%~dp0\.."

REM Diretórios padrão
set "APP_DIR=app"
set "BINARY_PATH=%APP_DIR%\target\release\cnpj-sqlite.exe"
set "DB_DIR=%APP_DIR%\dados-publicos"
set "DB_PATH=%DB_DIR%\cnpj.db"

REM Configurações padrão
if defined API_HOST (
    set "HOST=%API_HOST%"
) else (
    set "HOST=127.0.0.1"
)

if defined API_PORT (
    set "PORT=%API_PORT%"
) else (
    set "PORT=8080"
)

REM Processa argumentos
:parse_args
if "%~1"=="" goto end_parse
if /i "%~1"=="--host" (
    set "HOST=%~2"
    shift
    shift
    goto parse_args
)
if /i "%~1"=="--port" (
    set "PORT=%~2"
    shift
    shift
    goto parse_args
)
if /i "%~1"=="--database" (
    set "DB_PATH=%~2"
    shift
    shift
    goto parse_args
)
if /i "%~1"=="--help" goto show_help
if /i "%~1"=="-h" goto show_help
echo [ERROR] Opção desconhecida: %~1
echo Use --help para ver as opções disponíveis
exit /b 1
shift
goto parse_args

:end_parse

REM Verifica se está no diretório correto
if not exist "%APP_DIR%\Cargo.toml" (
    echo [ERROR] Arquivo Cargo.toml não encontrado. Execute este script na raiz do projeto.
    exit /b 1
)

REM Verifica se o Rust está instalado
where cargo >nul 2>&1
if errorlevel 1 (
    echo [ERROR] Rust/Cargo não está instalado. Por favor, instale o Rust primeiro.
    echo Visite: https://www.rust-lang.org/tools/install
    exit /b 1
)

echo [INFO] ═══════════════════════════════════════════════════════════
echo [INFO] Iniciando servidor API de consulta de CNPJ
echo [INFO] ═══════════════════════════════════════════════════════════
echo.

REM Verifica se o binário existe, se não, compila
if not exist "%BINARY_PATH%" (
    echo [WARNING] Binário não encontrado. Compilando o projeto...
    cd /d "%APP_DIR%"
    cargo build --release
    cd /d "%~dp0\.."
    if errorlevel 1 (
        echo [ERROR] Falha na compilação.
        exit /b 1
    )
    echo [SUCCESS] Compilação concluída!
    echo.
) else (
    echo [INFO] Binário encontrado: %BINARY_PATH%
)

REM Verifica se o banco de dados existe
if not exist "%DB_PATH%" (
    echo [ERROR] Banco de dados não encontrado: %DB_PATH%
    echo.
    echo [INFO] Para criar o banco de dados, execute:
    echo   scripts\run.bat
    echo.
    echo [INFO] Ou manualmente:
    echo   cd app
    echo   cargo run --release -- download
    echo   cargo run --release -- process
    exit /b 1
)

echo [SUCCESS] Banco de dados encontrado: %DB_PATH%
echo.

REM Valida porta (verificação básica)
set /a "PORT_NUM=%PORT%" >nul 2>&1
if errorlevel 1 (
    echo [ERROR] Porta inválida: %PORT% (deve ser um número entre 1 e 65535)
    exit /b 1
)

REM Exibe informações de configuração
echo [INFO] Configuração do servidor:
echo   📁 Banco de dados: %DB_PATH%
echo   🌐 Host: %HOST%
echo   🔌 Porta: %PORT%
echo   🔗 URL: http://%HOST%:%PORT%
echo.

REM Inicia o servidor
echo [API] ═══════════════════════════════════════════════════════════
echo [SUCCESS] Iniciando servidor...
echo [API] ═══════════════════════════════════════════════════════════
echo.

echo [INFO] Endpoints disponíveis:
echo   📋 GET /cnpj/{cnpj}  - Consulta dados completos de um CNPJ
echo   ❤️  GET /health       - Verifica status do servidor
echo.

echo [INFO] Exemplo de uso:
echo   curl http://%HOST%:%PORT%/cnpj/00000000000191
echo   curl http://%HOST%:%PORT%/health
echo.

echo [WARNING] Pressione Ctrl+C para parar o servidor
echo.

REM Executa o servidor
"%BINARY_PATH%" server --database "%DB_PATH%" --host "%HOST%" --port %PORT%

exit /b 0

:show_help
echo Uso: %~nx0 [OPÇÕES]
echo.
echo Opções:
echo   --host HOST       Endereço do servidor (padrão: 127.0.0.1)
echo   --port PORT       Porta do servidor (padrão: 8080)
echo   --database PATH   Caminho do banco SQLite (padrão: app\dados-publicos\cnpj.db)
echo   --help, -h        Exibe esta ajuda
echo.
echo Variáveis de ambiente:
echo   API_HOST          Endereço do servidor (sobrescreve --host)
echo   API_PORT          Porta do servidor (sobrescreve --port)
echo.
echo Exemplos:
echo   %~nx0
echo   %~nx0 --port 3000
echo   %~nx0 --host 0.0.0.0 --port 3000
echo   %~nx0 --database C:\caminho\para\cnpj.db
exit /b 0

