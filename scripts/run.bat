@echo off
REM Script de inicialização do processo completo de CNPJ-SQLite
REM Executa as 3 partes: Download, Processamento e CNAE Secundário
REM Execute este script na raiz do projeto

setlocal enabledelayedexpansion

REM Obtém o diretório do script e volta para a raiz do projeto
cd /d "%~dp0\.."

REM Diretórios
set "APP_DIR=app"
set "BINARY_PATH=%APP_DIR%\target\release\cnpj-sqlite.exe"
set "ZIP_DIR=%APP_DIR%\dados-publicos-zip"
set "DB_DIR=%APP_DIR%\dados-publicos"
set "DB_PATH=%DB_DIR%\cnpj.db"

REM Verifica se está no diretório correto
if not exist "%APP_DIR%\Cargo.toml" (
    echo [ERROR] Arquivo Cargo.toml não encontrado. Execute este script na raiz do projeto.
    exit /b 1
)

REM Verifica se o Rust está instalado
where cargo >nul 2>&1
if errorlevel 1 (
    echo [ERROR] Rust/Cargo não está instalado. Por favor, instale o Rust primeiro.
    exit /b 1
)

echo [INFO] Iniciando processo completo de CNPJ-SQLite...
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
    echo.
)

REM Parte 1: Download
echo [INFO] ═══════════════════════════════════════════════════════════
echo [INFO] PARTE 1/3: Download dos arquivos ZIP
echo [INFO] ═══════════════════════════════════════════════════════════
echo.

set "DOWNLOAD_NEW=0"
if exist "%ZIP_DIR%" (
    dir /b "%ZIP_DIR%\*.zip" >nul 2>&1
    if not errorlevel 1 (
        echo [WARNING] Diretório %ZIP_DIR% já contém arquivos ZIP.
        set /p "REPLY=Deseja baixar novamente? (s/N): "
        echo.
        if /i "!REPLY!"=="s" (
            set "DOWNLOAD_NEW=1"
        ) else (
            echo [INFO] Pulando download. Usando arquivos existentes.
        )
    ) else (
        set "DOWNLOAD_NEW=1"
    )
) else (
    set "DOWNLOAD_NEW=1"
)

if "!DOWNLOAD_NEW!"=="1" (
    echo [INFO] Iniciando download dos arquivos...
    "%BINARY_PATH%" download --output "%ZIP_DIR%"
    if errorlevel 1 (
        echo [ERROR] Falha no download.
        exit /b 1
    )
    echo [SUCCESS] Download concluído!
)

echo.

REM Parte 2: Processamento
echo [INFO] ═══════════════════════════════════════════════════════════
echo [INFO] PARTE 2/3: Processamento dos arquivos CSV para SQLite
echo [INFO] ═══════════════════════════════════════════════════════════
echo.

set "PROCESS_NEW=0"
if exist "%DB_PATH%" (
    echo [WARNING] Banco de dados %DB_PATH% já existe.
    set /p "REPLY=Deseja reprocessar? Isso apagará o banco existente. (s/N): "
    echo.
    if /i "!REPLY!"=="s" (
        set "PROCESS_NEW=1"
        echo [INFO] Removendo banco existente...
        if exist "%DB_PATH%" del /f /q "%DB_PATH%"
        if exist "%DB_PATH%-shm" del /f /q "%DB_PATH%-shm"
        if exist "%DB_PATH%-wal" del /f /q "%DB_PATH%-wal"
    ) else (
        echo [INFO] Pulando processamento. Usando banco existente.
    )
) else (
    set "PROCESS_NEW=1"
)

if "!PROCESS_NEW!"=="1" (
    echo [INFO] Iniciando processamento...
    "%BINARY_PATH%" process --input "%ZIP_DIR%" --output "%DB_DIR%" --cleanup true
    if errorlevel 1 (
        echo [ERROR] Falha no processamento.
        exit /b 1
    )
    echo [SUCCESS] Processamento concluído!
)

echo.

REM Parte 3: CNAE Secundário
echo [INFO] ═══════════════════════════════════════════════════════════
echo [INFO] PARTE 3/3: Criação da tabela de CNAE Secundário
echo [INFO] ═══════════════════════════════════════════════════════════
echo.

if not exist "%DB_PATH%" (
    echo [ERROR] Banco de dados não encontrado. Execute a parte 2 primeiro.
    exit /b 1
)

echo [INFO] Criando tabela de CNAE Secundário...
"%BINARY_PATH%" cnae-secundaria --database "%DB_PATH%"
if errorlevel 1 (
    echo [ERROR] Falha na criação da tabela de CNAE Secundário.
    exit /b 1
)
echo [SUCCESS] Tabela de CNAE Secundário criada!

echo.
echo [INFO] ═══════════════════════════════════════════════════════════
echo [SUCCESS] Processo completo finalizado com sucesso!
echo [INFO] ═══════════════════════════════════════════════════════════
echo.
echo [INFO] Banco de dados disponível em: %DB_PATH%
echo [INFO] Para iniciar o servidor API, execute:
echo   scripts\start-api.bat
echo   ou
echo   %BINARY_PATH% server --database %DB_PATH%
echo.

endlocal

