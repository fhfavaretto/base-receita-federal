use anyhow::Result;
use rusqlite::params;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use zip::ZipArchive;
use chrono::Local;
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};
use crate::database::Database;
use crate::utils;
use crate::ui;

pub fn process_files(input_dir: &str, output_dir: &str, cleanup: bool, auto_yes: bool) -> Result<()> {
    ui::print_header("⚙️  Processamento de Arquivos CSV → SQLite");
    ui::print_info(&format!("Hora de início: {}", Local::now().format("%Y-%m-%d %H:%M:%S")));
    ui::print_info(&format!("Diretório de entrada: {}", input_dir));
    ui::print_info(&format!("Diretório de saída: {}", output_dir));
    ui::print_info(&format!("Limpar arquivos temporários: {}", if cleanup { "Sim" } else { "Não" }));
    
    utils::ensure_dir(output_dir)?;
    
    let db_path = Path::new(output_dir).join("cnpj.db");
    if db_path.exists() {
        ui::print_error(&format!("O arquivo {:?} já existe!", db_path));
        ui::print_info("Apague o arquivo existente e execute novamente, ou use um diretório de saída diferente.");
        anyhow::bail!("Banco de dados já existe: {:?}", db_path);
    }
    
    // Descompacta arquivos ZIP
    let zip_files = utils::get_files_by_extension(input_dir, ".zip")?;
    
    if zip_files.is_empty() {
        anyhow::bail!("Nenhum arquivo ZIP encontrado em {}", input_dir);
    }
    
    if zip_files.len() != 37 {
        ui::print_warning(&format!("A pasta {} deveria conter 37 arquivos ZIP, mas contém {}.", input_dir, zip_files.len()));
        ui::print_warning("É recomendável prosseguir apenas com todos os arquivos, senão a base ficará incompleta.");
        
        let should_continue = if auto_yes {
            true
        } else {
            ui::ask_confirmation_no("Deseja prosseguir assim mesmo?")?
        };
        
        if !should_continue {
            ui::print_info("Operação cancelada pelo usuário.");
            return Ok(());
        }
    } else {
        ui::print_success(&format!("{} arquivo(s) ZIP encontrado(s)", zip_files.len()));
    }
    
    // Barra de progresso para descompactação
    ui::print_separator();
    ui::print_step(1, 4, "Descompactando arquivos ZIP");
    
    let mp = MultiProgress::new();
    let pb_extract = mp.add(ProgressBar::new(zip_files.len() as u64));
    pb_extract.set_style(
        ProgressStyle::default_bar()
            .template("  [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) | {msg}")?
            .progress_chars("#>-"),
    );
    for (idx, zip_file) in zip_files.iter().enumerate() {
        let filename = zip_file.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("arquivo");
        pb_extract.set_message(format!("{} ({}/{})", filename, idx + 1, zip_files.len()));
        extract_zip(zip_file, output_dir)?;
        pb_extract.inc(1);
    }
    pb_extract.finish_with_message("Descompactação concluída!");
    
    // Detecta data de referência
    let data_referencia = detect_data_referencia(output_dir)?;
    ui::print_info(&format!("Data de referência detectada: {}", data_referencia));
    
    // Cria banco de dados
    ui::print_step(2, 4, "Criando estrutura do banco de dados");
    let mut db = Database::new(db_path.to_str().unwrap())?;
    db.create_tables()?;
    ui::print_success("Estrutura do banco criada com sucesso");
    
    // Carrega tabelas de código (pequenas)
    ui::print_step(3, 4, "Carregando tabelas de referência");
    load_codigo_tables(&mut db, output_dir, cleanup, &mp)?;
    
    // Carrega tabelas grandes
    ui::print_step(4, 4, "Carregando tabelas principais");
    load_large_tables(&mut db, output_dir, cleanup, &mp)?;
    
    // Finaliza processamento
    ui::print_info("Finalizando processamento (criando índices, ajustando dados)...");
    db.finalize_processing(&data_referencia)?;
    
    // Estatísticas finais
    let conn = db.get_connection();
    let empresas: i64 = conn.query_row(
        "SELECT COUNT(*) FROM empresas",
        [],
        |row| row.get(0),
    )?;
    
    let estabelecimentos: i64 = conn.query_row(
        "SELECT COUNT(*) FROM estabelecimento",
        [],
        |row| row.get(0),
    )?;
    
    let socios: i64 = conn.query_row(
        "SELECT COUNT(*) FROM socios",
        [],
        |row| row.get(0),
    )?;
    
    ui::print_separator();
    ui::print_success("Processamento concluído!");
    ui::print_info(&format!("Arquivo criado: {:?}", db_path));
    ui::print_info(&format!("Hora de término: {}", Local::now().format("%Y-%m-%d %H:%M:%S")));
    
    ui::print_statistics(&[
        ("Empresas (matrizes)", empresas as u64),
        ("Estabelecimentos (matrizes e filiais)", estabelecimentos as u64),
        ("Sócios", socios as u64),
    ]);
    
    Ok(())
}

fn extract_zip(zip_path: &Path, output_dir: &str) -> Result<()> {
    let file = fs::File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = Path::new(output_dir).join(file.name());
        
        if file.is_dir() {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                fs::create_dir_all(p)?;
            }
            let mut outfile = fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }
    
    Ok(())
}

fn detect_data_referencia(output_dir: &str) -> Result<String> {
    let files = utils::get_files_by_extension(output_dir, ".EMPRECSV")?;
    if let Some(first_file) = files.first() {
        if let Some(filename) = first_file.file_name().and_then(|n| n.to_str()) {
            if let Some(date) = utils::parse_date_from_filename(filename) {
                return Ok(date);
            }
        }
    }
    Ok("xx/xx/2024".to_string())
}

fn load_codigo_tables(db: &mut Database, output_dir: &str, cleanup: bool, mp: &MultiProgress) -> Result<()> {
    let tables = vec![
        (".CNAECSV", "cnae"),
        (".MOTICSV", "motivo"),
        (".MUNICCSV", "municipio"),
        (".NATJUCSV", "natureza_juridica"),
        (".PAISCSV", "pais"),
        (".QUALSCSV", "qualificacao_socio"),
    ];
    
    let pb = mp.add(ProgressBar::new(tables.len() as u64));
    pb.set_style(
        ProgressStyle::default_bar()
            .template("  [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) | {msg}")?
            .progress_chars("#>-"),
    );
    
    for (_idx, (ext, table_name)) in tables.iter().enumerate() {
        let files = utils::get_files_by_extension(output_dir, ext)?;
        if let Some(file) = files.first() {
            pb.set_message(format!("Carregando: {}", table_name));
            load_codigo_table(db, file, table_name)?;
            
            if cleanup {
                fs::remove_file(file)?;
            }
        }
        pb.inc(1);
    }
    
    pb.finish_with_message("Tabelas de referência carregadas!");
    Ok(())
}

fn load_codigo_table(db: &mut Database, file_path: &Path, table_name: &str) -> Result<()> {
    // Usa reader com encoding Latin1, igual aos outros arquivos
    let reader = utils::create_latin1_reader(file_path)?;
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b';')
        .has_headers(false)
        .from_reader(reader);
    
    let tx = db.begin_transaction()?;
    
    for result in rdr.records() {
        let record = result?;
        if record.len() >= 2 {
            let codigo = record.get(0).unwrap_or("").to_string();
            let descricao = record.get(1).unwrap_or("").to_string();
            
            let sql = format!("INSERT OR REPLACE INTO {} (codigo, descricao) VALUES (?1, ?2)", table_name);
            tx.execute(&sql, params![codigo, descricao])?;
        }
    }
    
    tx.commit()?;
    
    // Cria índice
    db.create_index(table_name, "codigo")?;
    
    Ok(())
}

fn load_large_tables(db: &mut Database, output_dir: &str, cleanup: bool, mp: &MultiProgress) -> Result<()> {
    // Empresas
    load_table_with_polars(
        db,
        output_dir,
        ".EMPRECSV",
        "empresas",
        &[
            "cnpj_basico", "razao_social", "natureza_juridica",
            "qualificacao_responsavel", "capital_social_str",
            "porte_empresa", "ente_federativo_responsavel",
        ],
        cleanup,
        mp,
    )?;
    
    // Estabelecimento
    load_table_with_polars(
        db,
        output_dir,
        ".ESTABELE",
        "estabelecimento",
        &[
            "cnpj_basico", "cnpj_ordem", "cnpj_dv", "matriz_filial",
            "nome_fantasia", "situacao_cadastral", "data_situacao_cadastral",
            "motivo_situacao_cadastral", "nome_cidade_exterior", "pais",
            "data_inicio_atividades", "cnae_fiscal", "cnae_fiscal_secundaria",
            "tipo_logradouro", "logradouro", "numero", "complemento",
            "bairro", "cep", "uf", "municipio", "ddd1", "telefone1",
            "ddd2", "telefone2", "ddd_fax", "fax", "correio_eletronico",
            "situacao_especial", "data_situacao_especial",
        ],
        cleanup,
        mp,
    )?;
    
    // Sócios
    load_table_with_polars(
        db,
        output_dir,
        ".SOCIOCSV",
        "socios_original",
        &[
            "cnpj_basico", "identificador_de_socio", "nome_socio",
            "cnpj_cpf_socio", "qualificacao_socio", "data_entrada_sociedade",
            "pais", "representante_legal", "nome_representante",
            "qualificacao_representante_legal", "faixa_etaria",
        ],
        cleanup,
        mp,
    )?;
    
    // Simples
    load_table_with_polars(
        db,
        output_dir,
        ".SIMPLES.CSV",
        "simples",
        &[
            "cnpj_basico", "opcao_simples", "data_opcao_simples",
            "data_exclusao_simples", "opcao_mei", "data_opcao_mei",
            "data_exclusao_mei",
        ],
        cleanup,
        mp,
    )?;
    
    Ok(())
}

fn load_table_with_polars(
    db: &mut Database,
    output_dir: &str,
    pattern: &str,
    table_name: &str,
    columns: &[&str],
    cleanup: bool,
    mp: &MultiProgress,
) -> Result<()> {
    let files = utils::get_files_by_extension(output_dir, pattern)?;
    let files: Vec<PathBuf> = files.into_iter().filter(|f| {
        f.to_string_lossy().contains(pattern.trim_start_matches('.'))
    }).collect();
    
    let pb_table = mp.add(ProgressBar::new(files.len() as u64));
    pb_table.set_style(
        ProgressStyle::default_bar()
            .template(&format!("  {}: [{{bar:40.cyan/blue}}] {{pos}}/{{len}} ({{percent}}%) | {{msg}}", table_name))?
            .progress_chars("#>-"),
    );
    
    for (idx, file) in files.iter().enumerate() {
        let filename = file.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("arquivo");
        pb_table.set_message(format!("{} ({}/{})", filename, idx + 1, files.len()));
        
        // Usa csv crate para leitura mais simples e eficiente
        load_csv_to_sqlite(db, file, table_name, columns, mp)?;
        
        if cleanup {
            fs::remove_file(file)?;
        }
        
        pb_table.inc(1);
    }
    
    pb_table.finish_with_message(format!("{} concluída!", table_name));
    Ok(())
}

fn load_csv_to_sqlite(
    db: &mut Database,
    file_path: &Path,
    table_name: &str,
    columns: &[&str],
    mp: &MultiProgress,
) -> Result<()> {
    // Estima total de linhas pelo tamanho do arquivo (aproximado)
    let file_size = fs::metadata(file_path)?.len();
    let estimated_lines = (file_size / 200) as u64; // Estimativa: ~200 bytes por linha
    
    let reader = utils::create_latin1_reader(file_path)?;
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b';')
        .has_headers(false)
        .from_reader(reader);
    
    let num_cols = columns.len();
    let placeholders: Vec<String> = (0..num_cols)
        .map(|i| format!("?{}", i + 1))
        .collect();
    let sql = format!(
        "INSERT INTO {} VALUES ({})",
        table_name,
        placeholders.join(", ")
    );
    
    let pb = mp.add(ProgressBar::new(estimated_lines));
    pb.set_style(
        ProgressStyle::default_bar()
            .template("  Registros: [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) | {msg} | ETA: {eta}")?
            .progress_chars("#>-"),
    );
    
    let mut count = 0u64;
    let chunk_size = 50_000;
    let mut current_chunk = Vec::new();
    let start_time = Instant::now();
    let mut last_update = Instant::now();
    
    for result in rdr.records() {
        let record = result?;
        
        let mut params: Vec<String> = Vec::new();
        for i in 0..num_cols {
            let value = record.get(i).unwrap_or("");
            params.push(value.to_string());
        }
        
        current_chunk.push(params);
        count += 1;
        
        // Atualiza barra de progresso a cada 10k registros ou a cada segundo
        if count % 10_000 == 0 || last_update.elapsed().as_secs() >= 1 {
            pb.set_position(count);
            let elapsed = start_time.elapsed().as_secs_f64();
            if elapsed > 0.0 {
                let speed = count as f64 / elapsed;
                pb.set_message(format!("{:.0} registros/s", speed));
            }
            last_update = Instant::now();
        }
        
        if current_chunk.len() >= chunk_size {
            // Insere chunk
            let mut tx = db.begin_transaction()?;
            {
                let mut stmt = tx.prepare(&sql)?;
                for params in &current_chunk {
                    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();
                    stmt.execute(&param_refs[..])?;
                }
            }
            tx.commit()?;
            current_chunk.clear();
        }
    }
    
    // Insere chunk final
    if !current_chunk.is_empty() {
        let mut tx = db.begin_transaction()?;
        {
            let mut stmt = tx.prepare(&sql)?;
            for params in &current_chunk {
                let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();
                stmt.execute(&param_refs[..])?;
            }
        }
        tx.commit()?;
    }
    
    pb.set_position(count);
    let elapsed = start_time.elapsed().as_secs_f64();
    let avg_speed = if elapsed > 0.0 {
        count as f64 / elapsed
    } else {
        0.0
    };
    pb.finish_with_message(format!("{} registros | {:.0} registros/s", count, avg_speed));
    
    Ok(())
}


