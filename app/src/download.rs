use anyhow::{Context, Result};
use scraper::{Html, Selector};
use std::fs;
use std::io::{Write, BufWriter};
use std::path::Path;
use std::sync::Arc;
use std::time::{Instant, Duration};
use indicatif::{ProgressBar, ProgressStyle, HumanBytes, MultiProgress};
use crate::utils;
use crate::ui;
use futures::future::join_all;
use tokio::sync::Semaphore;

const URL_DADOS_ABERTOS: &str = "https://arquivos.receitafederal.gov.br/dados/cnpj/dados_abertos_cnpj/";

// Função para parsear tamanho do formato do servidor (ex: "1.0K", "47M", "1.8G")
fn parse_size(size_str: &str) -> u64 {
    let size_str = size_str.trim();
    if size_str.is_empty() || size_str == "-" {
        return 0;
    }
    
    // Remove espaços e converte para maiúsculas
    let size_str = size_str.replace(" ", "").to_uppercase();
    
    // Tenta extrair número e unidade
    let mut num_str = String::new();
    let mut unit = String::new();
    
    for ch in size_str.chars() {
        if ch.is_ascii_digit() || ch == '.' {
            num_str.push(ch);
        } else if ch.is_ascii_alphabetic() {
            unit.push(ch);
        }
    }
    
    let num: f64 = num_str.parse().unwrap_or(0.0);
    
    match unit.as_str() {
        "K" | "KB" => (num * 1024.0) as u64,
        "M" | "MB" => (num * 1024.0 * 1024.0) as u64,
        "G" | "GB" => (num * 1024.0 * 1024.0 * 1024.0) as u64,
        "T" | "TB" => (num * 1024.0 * 1024.0 * 1024.0 * 1024.0) as u64,
        _ => num as u64, // Assume bytes se não houver unidade
    }
}

pub async fn download_files(output_dir: &str, auto_yes: bool) -> Result<()> {
    ui::print_header("📥 Download de Arquivos da Receita Federal");
    ui::print_info(&format!("Diretório de saída: {}", output_dir));
    
    utils::ensure_dir(output_dir)?;
    
    // Verifica se a pasta está vazia
    if !utils::is_dir_empty(output_dir)? {
        let files = utils::get_files_by_extension(output_dir, ".zip")?;
        if !files.is_empty() {
            ui::print_warning(&format!("A pasta {} contém {} arquivo(s) ZIP existente(s)!", output_dir, files.len()));
            
            let should_delete = if auto_yes {
                true
            } else {
                ui::ask_confirmation_no("Deseja apagar os arquivos existentes?")?
            };
            
            if should_delete {
                for file in &files {
                    fs::remove_file(file)?;
                    ui::print_verbose(&format!("Removido: {:?}", file));
                }
                ui::print_success(&format!("{} arquivo(s) removido(s)", files.len()));
            } else {
                ui::print_info("Operação cancelada pelo usuário.");
                return Ok(());
            }
        }
    }

    // Busca a pasta mais recente
    ui::print_info("Conectando ao servidor da Receita Federal...");
    // Cliente com timeout maior para downloads grandes (30 minutos)
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .timeout(Duration::from_secs(1800)) // 30 minutos de timeout total para downloads grandes
        .connect_timeout(Duration::from_secs(60)) // 60 segundos para conectar
        .tcp_keepalive(Duration::from_secs(60))
        .build()
        .context("Falha ao criar cliente HTTP")?;
    
    ui::print_verbose(&format!("Buscando pasta mais recente em: {}", URL_DADOS_ABERTOS));
    
    // Tenta conectar com retry
    let mut last_error: Option<reqwest::Error> = None;
    let mut response = None;
    for attempt in 1..=5 {
        match client.get(URL_DADOS_ABERTOS)
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .header("Accept-Language", "pt-BR,pt;q=0.9,en-US;q=0.8,en;q=0.7")
            .header("Accept-Encoding", "gzip, deflate, br")
            .header("Connection", "keep-alive")
            .header("Upgrade-Insecure-Requests", "1")
            .send()
            .await
        {
            Ok(resp) => {
                response = Some(resp);
                if attempt > 1 {
                    ui::print_success(&format!("Conexão estabelecida na tentativa {}", attempt));
                }
                break;
            }
            Err(e) => {
                last_error = Some(e);
                if attempt < 5 {
                    let wait_time = attempt * 2; // 2, 4, 6, 8 segundos
                    ui::print_warning(&format!("Tentativa {}/5 falhou. Aguardando {} segundos antes de tentar novamente...", attempt, wait_time));
                    tokio::time::sleep(Duration::from_secs(wait_time as u64)).await;
                }
            }
        }
    }
    
    let response = response.ok_or_else(|| {
        anyhow::anyhow!("Falha ao conectar após 5 tentativas. Último erro: {}", 
            last_error.as_ref().map(|e| e.to_string()).unwrap_or_else(|| "Erro desconhecido".to_string()))
            .context("Não foi possível conectar ao servidor da Receita Federal. Verifique sua conexão com a internet.")
    })?;
    
    ui::print_verbose("Lendo conteúdo da página...");
    let html = response.text().await
        .context("Falha ao ler resposta do servidor")?;
    
    ui::print_verbose("Processando HTML...");
    let document = Html::parse_document(&html);
    
    let link_selector = Selector::parse("a").unwrap();
    let mut folders: Vec<String> = document
        .select(&link_selector)
        .filter_map(|link| {
            link.value().attr("href")
                .and_then(|href| {
                    if href.starts_with("20") && href.ends_with('/') {
                        Some(href.to_string())
                    } else {
                        None
                    }
                })
        })
        .collect();
    
    folders.sort();
    let ultima_referencia = folders.last()
        .context("Não encontrou pastas na página de dados abertos")?;
    
    let url = format!("{}{}", URL_DADOS_ABERTOS, ultima_referencia);
    ui::print_info(&format!("Pasta de referência: {}", ultima_referencia.trim_end_matches('/')));
    ui::print_verbose(&format!("URL completa: {}", url));
    
    // Lista arquivos ZIP
    ui::print_info("Listando arquivos disponíveis...");
    let mut list_response: Option<reqwest::Response> = None;
    let mut list_error: Option<reqwest::Error> = None;
    for attempt in 1..=3 {
        match client.get(&url)
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .header("Accept-Language", "pt-BR,pt;q=0.9,en-US;q=0.8,en;q=0.7")
            .header("Connection", "keep-alive")
            .send()
            .await
        {
            Ok(resp) => {
                list_response = Some(resp);
                break;
            }
            Err(e) => {
                list_error = Some(e);
                if attempt < 3 {
                    ui::print_verbose(&format!("Tentativa {}/3 de listar arquivos falhou, tentando novamente...", attempt));
                    tokio::time::sleep(Duration::from_secs(2 * attempt as u64)).await;
                }
            }
        }
    }
    
    let response = list_response.ok_or_else(|| {
        anyhow::anyhow!("Falha ao listar arquivos após 3 tentativas. Último erro: {}", 
            list_error.as_ref().map(|e| e.to_string()).unwrap_or_else(|| "Erro desconhecido".to_string()))
            .context("Falha ao listar arquivos da pasta de referência")
    })?;
    let html = response.text().await
        .context("Falha ao ler lista de arquivos")?;
    let document = Html::parse_document(&html);
    
    // Extrai arquivos com seus tamanhos da tabela HTML
    ui::print_info("Extraindo lista de arquivos e tamanhos...");
    let mut files_with_size: Vec<(String, u64)> = Vec::new();
    
    // Seleciona todas as linhas da tabela (tr)
    let row_selector = Selector::parse("tr").unwrap();
    let td_selector = Selector::parse("td").unwrap();
    
    for row in document.select(&row_selector) {
        let cells: Vec<String> = row.select(&td_selector)
            .map(|td| td.text().collect::<String>().trim().to_string())
            .collect();
        
        // A tabela tem: [ícone, nome, data, tamanho, descrição]
        // Procuramos por linhas que têm um link .zip
        if let Some(link) = row.select(&link_selector).next() {
            if let Some(href) = link.value().attr("href") {
                if href.ends_with(".zip") {
                    let file_url = if href.starts_with("http") {
                        href.to_string()
                    } else {
                        format!("{}{}", url, href)
                    };
                    
                    // Tenta extrair o tamanho da célula (geralmente a 4ª coluna)
                    let size = if cells.len() >= 4 {
                        parse_size(&cells[3])
                    } else {
                        0
                    };
                    
                    files_with_size.push((file_url, size));
                }
            }
        }
    }
    
    if files_with_size.is_empty() {
        anyhow::bail!("Nenhum arquivo ZIP encontrado na pasta de referência");
    }
    
    ui::print_success(&format!("{} arquivo(s) ZIP encontrado(s)", files_with_size.len()));
    
    // Ordena do menor para o maior
    files_with_size.sort_by_key(|(_, size)| *size);
    
    let total_size: u64 = files_with_size.iter().map(|(_, size)| *size).sum();
    
    ui::print_info(&format!("Tamanho total estimado: {}", HumanBytes(total_size)));
    ui::print_verbose("Arquivos ordenados do menor para o maior:");
    for (url, size) in &files_with_size {
        let filename = url.split('/').last().unwrap_or("arquivo");
        ui::print_verbose(&format!("  {}: {}", filename, HumanBytes(*size)));
    }
    
    let should_download = if auto_yes {
        true
    } else {
        ui::ask_confirmation_yes(&format!("Deseja baixar {} arquivo(s) para {}?", files_with_size.len(), output_dir))?
    };
    
    if !should_download {
        ui::print_info("Operação cancelada pelo usuário.");
        return Ok(());
    }
    
    // Extrai apenas as URLs ordenadas
    let file_urls: Vec<String> = files_with_size.into_iter().map(|(url, _)| url).collect();
    
    ui::print_info("Iniciando downloads paralelos (3 arquivos simultâneos)...");
    ui::print_separator();
    
    // Download paralelo controlado: 3 arquivos simultâneos
    let total_files = file_urls.len();
    const MAX_CONCURRENT: usize = 3;
    
    // Cria um MultiProgress para gerenciar múltiplas barras de progresso
    let multi = MultiProgress::new();
    
    // Cria uma barra de progresso geral
    let pb_overall = multi.add(ProgressBar::new(total_files as u64));
    pb_overall.set_style(
        ProgressStyle::default_bar()
            .template("📊 Total: [{bar:40.cyan/blue}] {pos}/{len} arquivos ({percent}%) | Faltam: {remaining} | {msg}")?
            .progress_chars("#>-"),
    );
    pb_overall.set_message("Iniciando downloads...");
    
    // Semáforo global para limitar downloads simultâneos
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT));
    let mut all_tasks = Vec::new();
    let mut all_errors = Vec::new();
    
    // Cria uma task para cada arquivo - o semáforo garante que apenas 3 rodem simultaneamente
    for (idx, url) in file_urls.iter().enumerate() {
        let filename = url.split('/').last().unwrap_or("file.zip").to_string();
        let client_clone = client.clone();
        let url_clone = url.clone();
        let output_dir_clone = output_dir.to_string();
        let multi_clone = multi.clone();
        let pb_overall_clone = pb_overall.clone();
        let semaphore_clone = Arc::clone(&semaphore);
        let current_idx = idx + 1;
        let total_files_clone = total_files;
        
        // Cria uma task assíncrona para cada download
        let task = tokio::spawn(async move {
            // Adquire permissão do semáforo (limita a 3 downloads simultâneos)
            let _permit = semaphore_clone.acquire().await
                .map_err(|e| anyhow::anyhow!("Erro ao adquirir semáforo: {}", e))?;
            
            let file_path = Path::new(&output_dir_clone).join(&filename);
            
            // Cria barra de progresso individual para este arquivo
            let pb = multi_clone.add(ProgressBar::new(0));
            pb.set_style(
                ProgressStyle::default_bar()
                    .template(&format!("{{spinner:.green}} [{{elapsed_precise}}] [{{wide_bar:.cyan/blue}}] {{bytes}}/{{total_bytes}} | {{msg}}"))?
                    .progress_chars("#>-"),
            );
            pb.set_message(format!("{} ({}/{})", filename, current_idx, total_files_clone));
            
            // Tenta baixar com retry (até 5 tentativas com timeout progressivo)
            let mut last_error = None;
            for attempt in 1..=5 {
                match download_single_file_with_progress(&client_clone, &url_clone, &file_path, pb.clone(), attempt).await {
                    Ok(_) => {
                        pb_overall_clone.inc(1);
                        let remaining = total_files_clone - pb_overall_clone.position() as usize;
                        pb_overall_clone.set_message(format!("{} arquivos restantes", remaining));
                        pb.finish_with_message(format!("✓ {} concluído", filename));
                        return Ok(());
                    }
                    Err(e) => {
                        last_error = Some(e);
                        if attempt < 5 {
                            let wait_time = (attempt * 3) as u64; // 3, 6, 9, 12 segundos
                            pb.set_message(format!("{} | Tentativa {}/5 falhou, aguardando {}s...", filename, attempt, wait_time));
                            tokio::time::sleep(Duration::from_secs(wait_time)).await;
                        }
                    }
                }
            }
            
            // Se chegou aqui, todas as tentativas falharam
            pb.finish_with_message(format!("✗ Erro após 3 tentativas: {}", 
                last_error.as_ref().map(|e| e.to_string()).unwrap_or_else(|| "Erro desconhecido".to_string())));
            Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Falha no download após 3 tentativas")))
        });
        
        all_tasks.push(task);
    }
    
    // Aguarda todos os downloads terminarem
    let results = join_all(all_tasks).await;
    
    // Verifica erros
    for result in results {
        match result {
            Ok(Ok(_)) => {
                // Sucesso
            }
            Ok(Err(e)) => all_errors.push(e),
            Err(e) => all_errors.push(anyhow::anyhow!("Erro na task: {}", e)),
        }
    }
    
    pb_overall.finish_with_message("Download concluído!");
    
    if !all_errors.is_empty() {
        ui::print_error(&format!("{} erro(s) durante o download:", all_errors.len()));
        for error in &all_errors {
            ui::print_error(&format!("  - {}", error));
        }
        return Err(all_errors.into_iter().next().unwrap());
    }
    
    let downloaded = utils::get_files_by_extension(output_dir, ".zip")?;
    ui::print_separator();
    ui::print_success(&format!("Download concluído! {} arquivo(s) baixado(s) com sucesso!", downloaded.len()));
    
    Ok(())
}

async fn download_single_file_with_progress(
    client: &reqwest::Client,
    url: &str,
    file_path: &Path,
    pb: ProgressBar,
    attempt: usize,
) -> Result<()> {
    // Cache de verificação: verifica apenas localmente primeiro
    // Evita HEAD requests desnecessários
    if file_path.exists() {
        if let Ok(metadata) = fs::metadata(file_path) {
            // Para arquivos grandes (> 1KB), assume que está completo se o tamanho é razoável
            // Só faz HEAD request se realmente necessário
            if metadata.len() > 1024 {
                // Para arquivos maiores, tenta verificar tamanho remoto
                // mas não bloqueia se falhar (timeout ou erro)
                if let Ok(head_response) = tokio::time::timeout(
                    Duration::from_secs(10),
                    client.head(url).send()
                ).await {
                    if let Ok(resp) = head_response {
                        if let Some(remote_size) = resp.content_length() {
                            if metadata.len() == remote_size {
                                pb.finish_with_message(format!("✓ {} já existe e está completo", 
                                    file_path.file_name().and_then(|n| n.to_str()).unwrap_or("arquivo")));
                                return Ok(());
                            }
                        }
                    }
                }
                // Se HEAD falhou mas arquivo existe e é grande, assume que pode estar completo
                // Mas continua o download para garantir
            } else {
                // Arquivo muito pequeno, sempre verifica
                if let Ok(head_response) = tokio::time::timeout(
                    Duration::from_secs(10),
                    client.head(url).send()
                ).await {
                    if let Ok(resp) = head_response {
                        if let Some(remote_size) = resp.content_length() {
                            if metadata.len() == remote_size {
                                pb.finish_with_message(format!("✓ {} já existe e está completo", 
                                    file_path.file_name().and_then(|n| n.to_str()).unwrap_or("arquivo")));
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Tenta resumir download se arquivo parcial existe
    let mut start_from = 0u64;
    if file_path.exists() {
        if let Ok(metadata) = fs::metadata(file_path) {
            let existing_size = metadata.len();
            if existing_size > 0 {
                start_from = existing_size;
                pb.set_message(format!("{} | Retomando download de {}...", 
                    file_path.file_name().and_then(|n| n.to_str()).unwrap_or("arquivo"),
                    HumanBytes(existing_size)));
            }
        }
    }
    
    // Abre arquivo para escrita (cria novo ou sobrescreve se não estiver retomando)
    let file = if start_from > 0 {
        // Se está retomando, abre em modo append
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)?
    } else {
        fs::File::create(file_path)?
    };
    
    // Cria request com Range header se estiver retomando
    let mut request = client.get(url);
    if start_from > 0 {
        request = request.header("Range", format!("bytes={}-", start_from));
    }
    
    // Timeout progressivo: aumenta com cada tentativa
    let timeout_duration = match attempt {
        1 => Duration::from_secs(600),  // 10 minutos na primeira tentativa
        2 => Duration::from_secs(900),  // 15 minutos na segunda
        3 => Duration::from_secs(1200), // 20 minutos na terceira
        4 => Duration::from_secs(1800), // 30 minutos na quarta
        _ => Duration::from_secs(2400), // 40 minutos na quinta
    };
    
    let response = tokio::time::timeout(timeout_duration, request.send()).await
        .map_err(|_| anyhow::anyhow!("Timeout ao iniciar download ({}s)", timeout_duration.as_secs()))?
        .with_context(|| format!("Erro ao iniciar download de {}", url))?;
    
    let status = response.status();
    let total_size = if start_from > 0 {
        // Se está retomando, o tamanho total é o que já tem + o que falta
        if let Some(content_range) = response.headers().get("Content-Range") {
            if let Ok(range_str) = content_range.to_str() {
                // Formato: "bytes 100-200/500" ou "bytes 100-*/500"
                if let Some(slash_pos) = range_str.rfind('/') {
                    if let Ok(size) = range_str[slash_pos + 1..].parse::<u64>() {
                        size
                    } else {
                        response.content_length().unwrap_or(0) + start_from
                    }
                } else {
                    response.content_length().unwrap_or(0) + start_from
                }
            } else {
                response.content_length().unwrap_or(0) + start_from
            }
        } else {
            response.content_length().unwrap_or(0) + start_from
        }
    } else {
        response.content_length().unwrap_or(0)
    };
    
    // Se status é 206 (Partial Content) ou 200 (OK)
    if !(status == 206 || status == 200 || status == 416) {
        anyhow::bail!("Erro HTTP {} ao baixar {}", status, url);
    }
    
    let filename = file_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("arquivo");
    
    // Configura a barra de progresso com o tamanho total
    pb.set_length(total_size);
    pb.set_position(start_from);
    
    // Usa BufWriter com buffer adaptativo baseado no tamanho do arquivo
    // Para arquivos grandes (> 100MB), usa buffer de até 2GB
    // Para arquivos menores, usa buffer proporcional
    let buffer_size = if total_size > 100 * 1024 * 1024 {
        // Arquivos grandes: buffer de 64MB a 2GB (limitado a 2GB)
        std::cmp::min(2 * 1024 * 1024 * 1024, std::cmp::max(64 * 1024 * 1024, total_size / 32))
    } else if total_size > 10 * 1024 * 1024 {
        // Arquivos médios: buffer de 8MB
        8 * 1024 * 1024
    } else {
        // Arquivos pequenos: buffer de 1MB
        1024 * 1024
    };
    
    let mut writer = BufWriter::with_capacity(buffer_size as usize, file);
    let mut downloaded = start_from;
    
    // Controle de porcentagem (atualiza a cada 10%)
    let mut last_percent = if start_from > 0 {
        ((start_from * 100) / total_size.max(1)) / 10 * 10
    } else {
        0u64
    };
    const PERCENT_STEP: u64 = 10;
    
    let mut stream = response.bytes_stream();
    let mut last_chunk_time = Instant::now();
    
    use futures::StreamExt;
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        writer.write_all(&chunk)?;
        
        downloaded += chunk.len() as u64;
        last_chunk_time = Instant::now();
        
        // Calcula porcentagem atual
        let current_percent = if total_size > 0 {
            (downloaded * 100) / total_size
        } else {
            0
        };
        
        // Atualiza barra de progresso apenas quando muda 10% ou no final
        if current_percent >= last_percent + PERCENT_STEP || downloaded == total_size {
            pb.set_position(downloaded);
            pb.set_message(format!("{} ({}/{}) - {}%", 
                filename, 
                HumanBytes(downloaded),
                HumanBytes(total_size),
                current_percent));
            last_percent = (current_percent / PERCENT_STEP) * PERCENT_STEP;
        }
        
        // Verifica se está travado (sem dados por mais de 5 minutos)
        if last_chunk_time.elapsed() > Duration::from_secs(300) {
            anyhow::bail!("Download travado: sem dados por mais de 5 minutos");
        }
    }
    
    // Flush final do buffer
    writer.flush()?;
    
    // Verifica se o arquivo está completo
    if downloaded < total_size && total_size > 0 {
        anyhow::bail!("Download incompleto: {} de {} bytes", downloaded, total_size);
    }
    
    pb.set_position(downloaded);
    pb.finish_with_message(format!("✓ {} concluído (100%)", filename));
    
    Ok(())
}

