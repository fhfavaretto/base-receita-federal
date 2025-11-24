# 📊 Receita Federal DB - Conversor de Dados Públicos de CNPJ

Ferramenta em Rust para baixar, processar e consultar os dados públicos de CNPJ da Receita Federal, convertendo-os para um banco de dados SQLite otimizado com API REST.

## 📋 Índice

- [Pré-requisitos](#-pré-requisitos)
- [Instalação](#-instalação)
- [Uso Rápido](#-uso-rápido)
- [Uso Detalhado](#-uso-detalhado)
- [API REST](#-api-rest)
- [Estrutura do Projeto](#-estrutura-do-projeto)
- [Comandos Disponíveis](#-comandos-disponíveis)
- [Troubleshooting](#-troubleshooting)

## 🔧 Pré-requisitos

### Obrigatórios

1. **Rust e Cargo** (versão 1.70 ou superior)
   - **Linux/macOS**: Execute no terminal:
     ```bash
     curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
     ```
   - **Windows**: Baixe e execute o instalador em [rustup.rs](https://rustup.rs/)
   - Verifique a instalação:
     ```bash
     cargo --version
     rustc --version
     ```

2. **Espaço em disco** (recomendado: mínimo 50GB livre)
   - Arquivos ZIP: ~15GB
   - Arquivos descompactados: ~30GB (temporário)
   - Banco SQLite final: ~5-10GB

3. **Conexão com internet** estável para download dos arquivos

### Opcionais

- **SQLite Browser** (para visualizar o banco manualmente)
- **curl** ou **Postman** (para testar a API)

## 📦 Instalação

1. **Clone ou baixe o repositório**:
   ```bash
   git clone <url-do-repositorio>
   cd receita-federal-db
   ```

2. **Compile o projeto** (opcional - será feito automaticamente pelos scripts):
   ```bash
   cd app
   cargo build --release
   cd ..
   ```

   O binário será gerado em `app/target/release/cnpj-sqlite` (Linux/macOS) ou `app/target/release/cnpj-sqlite.exe` (Windows).

## 🚀 Uso Rápido

### Método 1: Script Automatizado (Recomendado)

O método mais fácil é usar os scripts fornecidos que executam todo o processo automaticamente:

#### Linux/macOS:
```bash
./scripts/run.sh
```

#### Windows:
```cmd
scripts\run.bat
```

O script irá:
1. ✅ Verificar se o Rust está instalado
2. ✅ Compilar o projeto se necessário
3. ✅ Baixar os arquivos ZIP da Receita Federal
4. ✅ Processar os arquivos CSV e criar o banco SQLite
5. ✅ Criar a tabela de CNAE Secundário

### Método 2: Comandos Manuais

Se preferir executar cada etapa manualmente:

```bash
cd app

# 1. Baixar arquivos
cargo run --release -- download

# 2. Processar arquivos
cargo run --release -- process

# 3. Criar tabela CNAE Secundário
cargo run --release -- cnae-secundaria --database dados-publicos/cnpj.db
```

## 📖 Uso Detalhado

### 1. Download dos Arquivos

Baixa os arquivos ZIP do site da Receita Federal:

```bash
cargo run --release -- download --output dados-publicos-zip
```

**Parâmetros:**
- `--output`: Diretório onde salvar os arquivos ZIP (padrão: `dados-publicos-zip`)

**O que faz:**
- Acessa o site da Receita Federal
- Baixa todos os arquivos ZIP necessários (~15GB)
- Salva no diretório especificado

**Tempo estimado:** 1-3 horas (dependendo da velocidade da internet)

### 2. Processamento dos Arquivos

Processa os arquivos CSV e cria o banco SQLite:

```bash
cargo run --release -- process \
  --input dados-publicos-zip \
  --output dados-publicos \
  --cleanup true
```

**Parâmetros:**
- `--input`: Diretório com os arquivos ZIP (padrão: `dados-publicos-zip`)
- `--output`: Diretório para descompactar e gerar o banco (padrão: `dados-publicos`)
- `--cleanup`: Apagar arquivos CSV após processamento (padrão: `true`)

**O que faz:**
- Descompacta os arquivos ZIP
- Processa todos os CSVs
- Cria o banco SQLite `cnpj.db` com todas as tabelas
- Remove arquivos CSV temporários (se `--cleanup true`)

**Tempo estimado:** 2-6 horas (dependendo do hardware)

**Tabelas criadas:**
- `empresas` - Dados das empresas
- `estabelecimentos` - Dados dos estabelecimentos
- `socios` - Dados dos sócios
- `simples` - Dados do Simples Nacional
- `cnaes` - Códigos CNAE
- `municipios` - Municípios
- `naturezas` - Naturezas jurídicas
- `qualificacoes` - Qualificações
- `paises` - Países
- `motivos` - Motivos de situação cadastral

### 3. CNAE Secundário

Cria uma tabela normalizada com os CNAEs secundários:

```bash
cargo run --release -- cnae-secundaria \
  --database dados-publicos/cnpj.db \
  --low-memory false
```

**Parâmetros:**
- `--database`: Caminho do banco SQLite (padrão: `dados-publicos/cnpj.db`)
- `--low-memory`: Usar método com menos memória (padrão: `false`)

**O que faz:**
- Processa os CNAEs secundários do campo `cnae_fiscal_secundaria`
- Cria a tabela `cnae_secundaria` com relacionamento estabelecimento ↔ CNAE

**Tempo estimado:** 10-30 minutos

## 🌐 API REST

Após criar o banco de dados, você pode iniciar um servidor API REST para consultar os dados:

### Iniciar o Servidor

#### Linux/macOS:
```bash
./scripts/start-api.sh
```

#### Windows:
```cmd
scripts\start-api.bat
```

#### Manualmente:
```bash
cargo run --release -- server \
  --database dados-publicos/cnpj.db \
  --host 127.0.0.1 \
  --port 8080
```

**Parâmetros:**
- `--database`: Caminho do banco SQLite (padrão: `dados-publicos/cnpj.db`)
- `--host`: Endereço do servidor (padrão: `127.0.0.1`)
- `--port`: Porta do servidor (padrão: `8080`)

### Endpoints Disponíveis

#### 1. Consultar CNPJ
```http
GET /cnpj/{cnpj}
```

**Exemplo:**
```bash
curl http://127.0.0.1:8080/cnpj/00000000000191
```

**Resposta:**
```json
{
  "cnpj": "00000000000191",
  "estabelecimento": {
    "cnpj": "00000000000191",
    "nome_fantasia": "BANCO DO BRASIL S.A.",
    "situacao_cadastral": "2",
    "cnae_fiscal": "64121000",
    "cnae_fiscal_desc": "Bancos múltiplos, com carteira comercial",
    "logradouro": "SETOR BANCARIO SUL QUADRA 1",
    "numero": "LOTE 32",
    "bairro": "ASA SUL",
    "cep": "70072900",
    "uf": "DF",
    "municipio": "7107",
    "municipio_desc": "BRASILIA",
    ...
  },
  "empresa": {
    "cnpj_basico": "00000000",
    "razao_social": "BANCO DO BRASIL S.A.",
    "natureza_juridica": "2011",
    "qualificacao_responsavel": "5",
    ...
  },
  "socios": [
    {
      "cnpj_basico": "00000000",
      "identificador_socio": "1",
      "nome_socio": "UNIAO",
      "cnpj_cpf_socio": "",
      "qualificacao_socio": "49",
      "data_entrada": "20001105",
      ...
    }
  ],
  "simples": {
    "cnpj_basico": "00000000",
    "opcao_simples": "N",
    "data_opcao_simples": "",
    "data_exclusao_simples": "",
    "opcao_mei": "N",
    "data_opcao_mei": "",
    "data_exclusao_mei": ""
  }
}
```

**Formato do CNPJ:**
- Aceita com ou sem formatação: `00.000.000/0001-91` ou `00000000000191`
- Deve ter 14 dígitos

#### 2. Health Check
```http
GET /health
```

**Exemplo:**
```bash
curl http://127.0.0.1:8080/health
```

**Resposta:**
```json
{
  "status": "ok",
  "mensagem": "API CNPJ está funcionando"
}
```

### Configuração Avançada do Servidor

#### Variáveis de Ambiente

Você pode configurar o servidor usando variáveis de ambiente:

```bash
# Linux/macOS
export API_HOST=0.0.0.0
export API_PORT=3000
./scripts/start-api.sh

# Windows
set API_HOST=0.0.0.0
set API_PORT=3000
scripts\start-api.bat
```

#### Expor para Rede Local

Para permitir acesso de outros dispositivos na mesma rede:

```bash
./scripts/start-api.sh --host 0.0.0.0 --port 8080
```

## 📁 Estrutura do Projeto

```
receita-federal-db/
├── app/                          # Aplicação principal
│   ├── src/                      # Código fonte
│   │   ├── main.rs               # Ponto de entrada e CLI
│   │   ├── download.rs           # Módulo de download
│   │   ├── process.rs            # Módulo de processamento
│   │   ├── cnae_secundaria.rs    # CNAE secundário
│   │   ├── database.rs           # Configuração do banco
│   │   ├── api.rs                # Servidor API REST
│   │   ├── models.rs             # Modelos de dados
│   │   └── ...
│   ├── dados-publicos-zip/      # Arquivos ZIP baixados (~15GB)
│   ├── dados-publicos/           # Banco SQLite e arquivos temporários
│   │   └── cnpj.db               # Banco de dados final
│   └── Cargo.toml                # Dependências Rust
├── scripts/                      # Scripts de automação
│   ├── run.sh                    # Script completo (Linux/macOS)
│   ├── run.bat                   # Script completo (Windows)
│   ├── start-api.sh              # Iniciar API (Linux/macOS)
│   └── start-api.bat             # Iniciar API (Windows)
└── README.md                     # Este arquivo
```

## 🛠️ Comandos Disponíveis

### Comandos Principais

```bash
# Download
cargo run --release -- download [--output DIR]

# Processamento
cargo run --release -- process [--input DIR] [--output DIR] [--cleanup BOOL]

# CNAE Secundário
cargo run --release -- cnae-secundaria [--database PATH] [--low-memory BOOL]

# Servidor API
cargo run --release -- server [--database PATH] [--host HOST] [--port PORT]
```

### Opções Globais

```bash
# Pular confirmações interativas
cargo run --release -- --yes download

# Modo silencioso
cargo run --release -- --quiet process

# Modo verboso (mais detalhes)
cargo run --release -- --verbose server
```

## 🔍 Troubleshooting

### Problema: "Rust/Cargo não está instalado"

**Solução:**
1. Instale o Rust seguindo as instruções em [rustup.rs](https://rustup.rs/)
2. Reinicie o terminal após a instalação
3. Verifique com `cargo --version`

### Problema: "Espaço em disco insuficiente"

**Solução:**
- Libere espaço (mínimo 50GB recomendado)
- Ou use um diretório externo:
  ```bash
  cargo run --release -- download --output /caminho/externo/dados-publicos-zip
  cargo run --release -- process --input /caminho/externo/dados-publicos-zip --output /caminho/externo/dados-publicos
  ```

### Problema: "Download muito lento"

**Solução:**
- O download pode levar várias horas dependendo da conexão
- Os arquivos são grandes (~15GB total)
- Considere executar durante a noite ou em horários de menor tráfego

### Problema: "Processamento travou ou está muito lento"

**Solução:**
- O processamento é intensivo e pode levar várias horas
- Verifique se há espaço em disco suficiente
- Considere usar `--low-memory true` no comando `cnae-secundaria` se tiver pouca RAM

### Problema: "Erro ao iniciar o servidor API"

**Solução:**
1. Verifique se o banco de dados existe:
   ```bash
   ls -lh app/dados-publicos/cnpj.db
   ```
2. Verifique se a porta está em uso:
   ```bash
   # Linux/macOS
   lsof -i :8080
   
   # Windows
   netstat -ano | findstr :8080
   ```
3. Use outra porta:
   ```bash
   ./scripts/start-api.sh --port 3000
   ```

### Problema: "CNPJ não encontrado na API"

**Solução:**
- Verifique se o CNPJ tem 14 dígitos
- Verifique se o banco de dados foi processado completamente
- Alguns CNPJs podem não existir na base da Receita Federal

### Problema: "Erro de permissão"

**Solução:**
- **Linux/macOS**: Dê permissão de execução aos scripts:
  ```bash
  chmod +x scripts/*.sh
  ```
- **Windows**: Execute o PowerShell ou CMD como Administrador se necessário

## 📝 Notas Importantes

1. **Primeira execução**: O download e processamento podem levar várias horas
2. **Atualização dos dados**: Os dados da Receita Federal são atualizados periodicamente. Execute o processo completo novamente para atualizar
3. **Backup**: Faça backup do arquivo `cnpj.db` após o processamento completo
4. **Performance**: O banco SQLite usa WAL mode para melhor performance em leitura simultânea

## 🤝 Contribuindo

Contribuições são bem-vindas! Sinta-se à vontade para abrir issues ou pull requests.

## 📄 Licença

Este projeto é fornecido "como está", sem garantias. Os dados são públicos e fornecidos pela Receita Federal do Brasil.

## 🔗 Links Úteis

- [Site da Receita Federal - Dados Públicos](https://dados.gov.br/dados/conjuntos-dados/cadastro-nacional-da-pessoa-juridica-cnpj)
- [Documentação do Rust](https://doc.rust-lang.org/)
- [Documentação do SQLite](https://www.sqlite.org/docs.html)

---

**Desenvolvido com ❤️ em Rust**

