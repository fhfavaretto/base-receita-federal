use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use chrono::Local;

pub fn create_cnae_secundaria_table(db_path: &str, low_memory: bool) -> Result<()> {
    println!("Iniciando criação da tabela cnae_secundaria...");
    println!("Hora de início: {}", Local::now().format("%Y-%m-%d %H:%M:%S"));
    
    let mut conn = Connection::open(db_path)
        .with_context(|| format!("Falha ao abrir banco: {}", db_path))?;
    
    conn.execute("DROP TABLE IF EXISTS cnae_secundaria", [])?;
    
    if low_memory {
        create_with_low_memory(&mut conn)?;
    } else {
        create_with_pandas_approach(&mut conn)?;
    }
    
    println!("Hora de término: {}", Local::now().format("%Y-%m-%d %H:%M:%S"));
    println!("Tabela cnae_secundaria criada com sucesso!");
    
    Ok(())
}

fn create_with_pandas_approach(conn: &mut Connection) -> Result<()> {
    println!("Usando método Pandas (carrega tudo na memória)...");
    
    // Primeiro coletamos todos os dados
    let mut data = Vec::new();
    {
        let mut stmt = conn.prepare("SELECT cnpj, cnae_fiscal_secundaria FROM estabelecimento WHERE cnae_fiscal_secundaria != ''")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
            ))
        })?;
        
        for row in rows {
            let (cnpj, cnaes_str) = row?;
            let cnaes: Vec<&str> = cnaes_str.split(',').map(|s| s.trim()).collect();
            
            for cnae in cnaes {
                if !cnae.is_empty() {
                    data.push((cnpj.clone(), cnae.to_string()));
                }
            }
        }
    }
    
    // Cria tabela
    conn.execute(
        "CREATE TABLE cnae_secundaria (cnpj TEXT, cnae_fiscal_secundaria TEXT)",
        [],
    )?;
    
    // Insere em chunks
    let chunk_size = 100_000;
    let mut count = 0;
    
    for chunk in data.chunks(chunk_size) {
        let mut tx = conn.transaction()?;
        {
            let mut insert_stmt = tx.prepare("INSERT INTO cnae_secundaria (cnpj, cnae_fiscal_secundaria) VALUES (?1, ?2)")?;
            for (cnpj, cnae) in chunk {
                insert_stmt.execute(params![cnpj, cnae])?;
                count += 1;
            }
        }
        tx.commit()?;
        
        if count % 100_000 == 0 {
            println!("  Processados {} registros...", count);
        }
    }
    
    // Cria índices
    conn.execute("CREATE INDEX IF NOT EXISTS idx_cnae_secundaria_cnpj ON cnae_secundaria(cnpj)", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_cnae_secundaria_cnae ON cnae_secundaria(cnae_fiscal_secundaria)", [])?;
    
    println!("Total de registros inseridos: {}", count);
    
    Ok(())
}

fn create_with_low_memory(conn: &mut Connection) -> Result<()> {
    println!("Usando método Dask-like (baixo uso de memória)...");
    
    // Cria tabela temporária
    conn.execute(
        "CREATE TEMP TABLE tmp_cnae AS SELECT cnpj, cnae_fiscal_secundaria FROM estabelecimento WHERE cnae_fiscal_secundaria != ''",
        [],
    )?;
    
    // Cria tabela final
    conn.execute(
        "CREATE TABLE cnae_secundaria (cnpj TEXT, cnae_fiscal_secundaria TEXT)",
        [],
    )?;
    
    // Processa em chunks usando SQL
    // SQLite não tem SPLIT nativo, então fazemos em Rust
    // Primeiro coletamos todos os dados
    let mut data = Vec::new();
    {
        let mut stmt = conn.prepare("SELECT cnpj, cnae_fiscal_secundaria FROM tmp_cnae")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
            ))
        })?;
        
        for row in rows {
            let (cnpj, cnaes_str) = row?;
            let cnaes: Vec<&str> = cnaes_str.split(',').map(|s| s.trim()).collect();
            
            for cnae in cnaes {
                if !cnae.is_empty() {
                    data.push((cnpj.clone(), cnae.to_string()));
                }
            }
        }
    }
    
    // Agora processa em chunks
    let mut count = 0;
    let chunk_size = 100_000;
    
    for chunk in data.chunks(chunk_size) {
        let mut tx = conn.transaction()?;
        {
            let mut insert_stmt = tx.prepare("INSERT INTO cnae_secundaria (cnpj, cnae_fiscal_secundaria) VALUES (?1, ?2)")?;
            for (cnpj, cnae) in chunk {
                insert_stmt.execute(params![cnpj, cnae])?;
                count += 1;
            }
        }
        tx.commit()?;
        
        if count % 100_000 == 0 {
            println!("  Processados {} registros...", count);
        }
    }
    
    // Remove tabela temporária
    conn.execute("DROP TABLE IF EXISTS tmp_cnae", [])?;
    
    // Cria índices
    conn.execute("CREATE INDEX IF NOT EXISTS idx_cnae_secundaria_cnpj ON cnae_secundaria(cnpj)", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_cnae_secundaria_cnae ON cnae_secundaria(cnae_fiscal_secundaria)", [])?;
    
    println!("Total de registros inseridos: {}", count);
    
    Ok(())
}

