mod download;
mod process;
mod cnae_secundaria;
mod database;
mod models;
mod utils;
mod api;
mod ui;

use clap::{Parser, Subcommand};
use anyhow::Result;

#[derive(Parser)]
#[command(name = "cnpj-sqlite")]
#[command(about = "Conversor de dados públicos de CNPJ para SQLite", long_about = None)]
struct Cli {
    /// Pula todas as confirmações interativas (yes para tudo)
    #[arg(long, global = true)]
    yes: bool,
    
    /// Modo silencioso (menos saída)
    #[arg(short, long, global = true)]
    quiet: bool,
    
    /// Modo verboso (mais detalhes)
    #[arg(short, long, global = true)]
    verbose: bool,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Baixa os arquivos ZIP do site da Receita Federal
    Download {
        /// Pasta para salvar os arquivos ZIP
        #[arg(short, long, default_value = "dados-publicos-zip")]
        output: String,
    },
    /// Processa os arquivos CSV e gera o banco SQLite
    Process {
        /// Pasta com os arquivos ZIP
        #[arg(short, long, default_value = "dados-publicos-zip")]
        input: String,
        /// Pasta para descompactar e gerar o banco
        #[arg(short, long, default_value = "dados-publicos")]
        output: String,
        /// Apagar arquivos descompactados após uso (padrão: true)
        #[arg(short, long, default_value = "true")]
        cleanup: String,
    },
    /// Cria tabela normalizada de CNAEs secundários
    CnaeSecundaria {
        /// Caminho do banco SQLite
        #[arg(short, long, default_value = "dados-publicos/cnpj.db")]
        database: String,
        /// Usar método com menos memória (Dask-like)
        #[arg(short, long, default_value = "false")]
        low_memory: bool,
    },
    /// Inicia servidor web API para consulta de CNPJ
    Server {
        /// Caminho do banco SQLite
        #[arg(short, long, default_value = "dados-publicos/cnpj.db")]
        database: String,
        /// Porta do servidor
        #[arg(short, long, default_value = "8080")]
        port: u16,
        /// Endereço do servidor
        #[arg(short, long, default_value = "127.0.0.1")]
        host: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Inicializa o módulo de UI com as configurações globais
    ui::init(cli.quiet, cli.verbose);

    match cli.command {
        Commands::Download { output } => {
            download::download_files(&output, cli.yes).await?;
        }
        Commands::Process { input, output, cleanup } => {
            let should_cleanup = cleanup.parse::<bool>().unwrap_or(true);
            process::process_files(&input, &output, should_cleanup, cli.yes)?;
        }
        Commands::CnaeSecundaria { database, low_memory } => {
            cnae_secundaria::create_cnae_secundaria_table(&database, low_memory)?;
        }
        Commands::Server { database, port, host } => {
            api::start_server(&database, &host, port).await?;
        }
    }

    Ok(())
}

