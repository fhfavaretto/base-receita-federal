use actix_web::{web, HttpResponse, Result as ActixResult};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use anyhow::Context;
use crate::ui;

#[derive(Serialize, Deserialize)]
pub struct CnpjResponse {
    pub cnpj: String,
    pub estabelecimento: Option<EstabelecimentoCompleto>,
    pub empresa: Option<EmpresaCompleta>,
    pub socios: Vec<SocioCompleto>,
    pub simples: Option<SimplesDados>,
}

#[derive(Serialize, Deserialize)]
pub struct EstabelecimentoCompleto {
    // Dados do estabelecimento
    pub cnpj: String,
    pub cnpj_basico: String,
    pub cnpj_ordem: String,
    pub cnpj_dv: String,
    pub matriz_filial: String,
    pub nome_fantasia: String,
    pub situacao_cadastral: String,
    pub data_situacao_cadastral: String,
    pub motivo_situacao_cadastral: String,
    pub motivo_situacao_cadastral_desc: Option<String>,
    pub nome_cidade_exterior: String,
    pub pais: String,
    pub pais_desc: Option<String>,
    pub data_inicio_atividades: String,
    pub cnae_fiscal: String,
    pub cnae_fiscal_desc: Option<String>,
    pub cnae_fiscal_secundaria: String,
    pub tipo_logradouro: String,
    pub logradouro: String,
    pub numero: String,
    pub complemento: String,
    pub bairro: String,
    pub cep: String,
    pub uf: String,
    pub municipio: String,
    pub municipio_desc: Option<String>,
    pub ddd1: String,
    pub telefone1: String,
    pub ddd2: String,
    pub telefone2: String,
    pub ddd_fax: String,
    pub fax: String,
    pub correio_eletronico: String,
    pub situacao_especial: String,
    pub data_situacao_especial: String,
}

#[derive(Serialize, Deserialize)]
pub struct EmpresaCompleta {
    pub cnpj_basico: String,
    pub razao_social: String,
    pub natureza_juridica: String,
    pub natureza_juridica_desc: Option<String>,
    pub qualificacao_responsavel: String,
    pub qualificacao_responsavel_desc: Option<String>,
    pub capital_social: Option<f64>,
    pub porte_empresa: String,
    pub ente_federativo_responsavel: String,
}

#[derive(Serialize, Deserialize)]
pub struct SocioCompleto {
    pub cnpj: String,
    pub cnpj_basico: String,
    pub identificador_de_socio: String,
    pub nome_socio: String,
    pub cnpj_cpf_socio: String,
    pub qualificacao_socio: String,
    pub qualificacao_socio_desc: Option<String>,
    pub data_entrada_sociedade: String,
    pub pais: String,
    pub pais_desc: Option<String>,
    pub representante_legal: String,
    pub nome_representante: String,
    pub qualificacao_representante_legal: String,
    pub qualificacao_representante_legal_desc: Option<String>,
    pub faixa_etaria: String,
}

#[derive(Serialize, Deserialize)]
pub struct SimplesDados {
    pub cnpj_basico: String,
    pub opcao_simples: String,
    pub data_opcao_simples: String,
    pub data_exclusao_simples: String,
    pub opcao_mei: String,
    pub data_opcao_mei: String,
    pub data_exclusao_mei: String,
}

pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
}

pub async fn consultar_cnpj(
    cnpj: web::Path<String>,
    state: web::Data<AppState>,
) -> ActixResult<HttpResponse> {
    let cnpj_limpo = cnpj.replace(".", "").replace("/", "").replace("-", "");
    
    if cnpj_limpo.len() != 14 {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "erro": "CNPJ deve ter 14 dígitos",
            "cnpj_recebido": cnpj_limpo
        })));
    }

    let db = state.db.lock().unwrap();
    
    // Busca dados do estabelecimento e empresa (query principal)
    let estabelecimento = buscar_estabelecimento(&db, &cnpj_limpo)?;
    
    let empresa = if let Some(ref est) = estabelecimento {
        buscar_empresa(&db, &est.cnpj_basico)?
    } else {
        None
    };
    
    // Busca sócios
    let socios = buscar_socios(&db, &cnpj_limpo)?;
    
    // Busca dados do Simples
    let simples = if let Some(ref est) = estabelecimento {
        buscar_simples(&db, &est.cnpj_basico)?
    } else {
        None
    };
    
    let response = CnpjResponse {
        cnpj: cnpj_limpo,
        estabelecimento,
        empresa,
        socios,
        simples,
    };
    
    Ok(HttpResponse::Ok().json(response))
}

fn buscar_estabelecimento(db: &Connection, cnpj: &str) -> ActixResult<Option<EstabelecimentoCompleto>> {
    let mut stmt = db.prepare(
        r#"
        SELECT 
            t.cnpj, t.cnpj_basico, t.cnpj_ordem, t.cnpj_dv, t.matriz_filial,
            t.nome_fantasia, t.situacao_cadastral, t.data_situacao_cadastral,
            t.motivo_situacao_cadastral, t.nome_cidade_exterior, t.pais,
            t.data_inicio_atividades, t.cnae_fiscal, t.cnae_fiscal_secundaria,
            t.tipo_logradouro, t.logradouro, t.numero, t.complemento,
            t.bairro, t.cep, t.uf, t.municipio,
            t.ddd1, t.telefone1, t.ddd2, t.telefone2,
            t.ddd_fax, t.fax, t.correio_eletronico,
            t.situacao_especial, t.data_situacao_especial,
            tmot.descricao as motivo_situacao_cadastral_desc,
            tmun.descricao as municipio_desc,
            tc.descricao as cnae_fiscal_desc,
            tpa.descricao as pais_desc
        FROM estabelecimento t
        LEFT JOIN motivo tmot ON tmot.codigo = t.motivo_situacao_cadastral
        LEFT JOIN municipio tmun ON tmun.codigo = t.municipio
        LEFT JOIN cnae tc ON tc.codigo = t.cnae_fiscal
        LEFT JOIN pais tpa ON tpa.codigo = t.pais
        WHERE t.cnpj = ?1
        "#
    ).map_err(|e| actix_web::error::ErrorInternalServerError(format!("Erro SQL: {}", e)))?;
    
    let row_result = stmt.query_row(params![cnpj], |row| {
        Ok(EstabelecimentoCompleto {
            cnpj: row.get(0)?,
            cnpj_basico: row.get(1)?,
            cnpj_ordem: row.get(2)?,
            cnpj_dv: row.get(3)?,
            matriz_filial: row.get(4)?,
            nome_fantasia: row.get(5)?,
            situacao_cadastral: row.get(6)?,
            data_situacao_cadastral: row.get(7)?,
            motivo_situacao_cadastral: row.get(8)?,
            motivo_situacao_cadastral_desc: row.get(28)?,
            nome_cidade_exterior: row.get(9)?,
            pais: row.get(10)?,
            pais_desc: row.get(31)?,
            data_inicio_atividades: row.get(11)?,
            cnae_fiscal: row.get(12)?,
            cnae_fiscal_desc: row.get(30)?,
            cnae_fiscal_secundaria: row.get(13)?,
            tipo_logradouro: row.get(14)?,
            logradouro: row.get(15)?,
            numero: row.get(16)?,
            complemento: row.get(17)?,
            bairro: row.get(18)?,
            cep: row.get(19)?,
            uf: row.get(20)?,
            municipio: row.get(21)?,
            municipio_desc: row.get(29)?,
            ddd1: row.get(22)?,
            telefone1: row.get(23)?,
            ddd2: row.get(24)?,
            telefone2: row.get(25)?,
            ddd_fax: row.get(26)?,
            fax: row.get(27)?,
            correio_eletronico: row.get::<_, Option<String>>(28)?.unwrap_or_default(),
            situacao_especial: row.get::<_, Option<String>>(29)?.unwrap_or_default(),
            data_situacao_especial: row.get::<_, Option<String>>(30)?.unwrap_or_default(),
        })
    });
    
    match row_result {
        Ok(est) => Ok(Some(est)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(actix_web::error::ErrorInternalServerError(format!("Erro ao buscar estabelecimento: {}", e))),
    }
}

fn buscar_empresa(db: &Connection, cnpj_basico: &str) -> ActixResult<Option<EmpresaCompleta>> {
    let mut stmt = db.prepare(
        r#"
        SELECT 
            te.cnpj_basico, te.razao_social, te.natureza_juridica,
            te.qualificacao_responsavel, te.capital_social, te.porte_empresa,
            te.ente_federativo_responsavel,
            tnat.descricao as natureza_juridica_desc,
            tq.descricao as qualificacao_responsavel_desc
        FROM empresas te
        LEFT JOIN natureza_juridica tnat ON tnat.codigo = te.natureza_juridica
        LEFT JOIN qualificacao_socio tq ON tq.codigo = te.qualificacao_responsavel
        WHERE te.cnpj_basico = ?1
        "#
    ).map_err(|e| actix_web::error::ErrorInternalServerError(format!("Erro SQL: {}", e)))?;
    
    let row_result = stmt.query_row(params![cnpj_basico], |row| {
        Ok(EmpresaCompleta {
            cnpj_basico: row.get(0)?,
            razao_social: row.get(1)?,
            natureza_juridica: row.get(2)?,
            natureza_juridica_desc: row.get(7)?,
            qualificacao_responsavel: row.get(3)?,
            qualificacao_responsavel_desc: row.get(8)?,
            capital_social: row.get(4)?,
            porte_empresa: row.get(5)?,
            ente_federativo_responsavel: row.get(6)?,
        })
    });
    
    match row_result {
        Ok(emp) => Ok(Some(emp)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(actix_web::error::ErrorInternalServerError(format!("Erro ao buscar empresa: {}", e))),
    }
}

fn buscar_socios(db: &Connection, cnpj: &str) -> ActixResult<Vec<SocioCompleto>> {
    let mut stmt = db.prepare(
        r#"
        SELECT 
            ts.cnpj, ts.cnpj_basico, ts.identificador_de_socio, ts.nome_socio,
            ts.cnpj_cpf_socio, ts.qualificacao_socio, ts.data_entrada_sociedade,
            ts.pais, ts.representante_legal, ts.nome_representante,
            ts.qualificacao_representante_legal, ts.faixa_etaria,
            tq.descricao as qualificacao_socio_desc,
            tq2.descricao as qualificacao_representante_legal_desc,
            tpa.descricao as pais_desc
        FROM socios ts
        LEFT JOIN qualificacao_socio tq ON tq.codigo = ts.qualificacao_socio
        LEFT JOIN qualificacao_socio tq2 ON tq2.codigo = ts.qualificacao_representante_legal
        LEFT JOIN pais tpa ON tpa.codigo = ts.pais
        WHERE ts.cnpj = ?1
        "#
    ).map_err(|e| actix_web::error::ErrorInternalServerError(format!("Erro SQL: {}", e)))?;
    
    let rows = stmt.query_map(params![cnpj], |row| {
        Ok(SocioCompleto {
            cnpj: row.get(0)?,
            cnpj_basico: row.get(1)?,
            identificador_de_socio: row.get(2)?,
            nome_socio: row.get(3)?,
            cnpj_cpf_socio: row.get(4)?,
            qualificacao_socio: row.get(5)?,
            qualificacao_socio_desc: row.get(12)?,
            data_entrada_sociedade: row.get(6)?,
            pais: row.get(7)?,
            pais_desc: row.get(14)?,
            representante_legal: row.get(8)?,
            nome_representante: row.get(9)?,
            qualificacao_representante_legal: row.get(10)?,
            qualificacao_representante_legal_desc: row.get(13)?,
            faixa_etaria: row.get(11)?,
        })
    }).map_err(|e| actix_web::error::ErrorInternalServerError(format!("Erro ao buscar sócios: {}", e)))?;
    
    let mut socios = Vec::new();
    for row in rows {
        socios.push(row.map_err(|e| actix_web::error::ErrorInternalServerError(format!("Erro ao processar sócio: {}", e)))?);
    }
    
    Ok(socios)
}

fn buscar_simples(db: &Connection, cnpj_basico: &str) -> ActixResult<Option<SimplesDados>> {
    let mut stmt = db.prepare(
        r#"
        SELECT 
            cnpj_basico, opcao_simples, data_opcao_simples,
            data_exclusao_simples, opcao_mei, data_opcao_mei,
            data_exclusao_mei
        FROM simples
        WHERE cnpj_basico = ?1
        "#
    ).map_err(|e| actix_web::error::ErrorInternalServerError(format!("Erro SQL: {}", e)))?;
    
    let row_result = stmt.query_row(params![cnpj_basico], |row| {
        Ok(SimplesDados {
            cnpj_basico: row.get(0)?,
            opcao_simples: row.get(1)?,
            data_opcao_simples: row.get(2)?,
            data_exclusao_simples: row.get(3)?,
            opcao_mei: row.get(4)?,
            data_opcao_mei: row.get(5)?,
            data_exclusao_mei: row.get(6)?,
        })
    });
    
    match row_result {
        Ok(simples) => Ok(Some(simples)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(actix_web::error::ErrorInternalServerError(format!("Erro ao buscar Simples: {}", e))),
    }
}

pub async fn start_server(db_path: &str, host: &str, port: u16) -> anyhow::Result<()> {
    let conn = Connection::open(db_path)
        .with_context(|| format!("Falha ao abrir banco de dados: {}", db_path))?;
    
    // Configura SQLite para melhor performance em multi-thread
    // PRAGMA journal_mode retorna um valor, então usamos query_row
    let _: String = conn.query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))?;
    conn.execute("PRAGMA synchronous = NORMAL", [])?;
    conn.execute("PRAGMA cache_size = -64000", [])?;
    conn.execute("PRAGMA foreign_keys = ON", [])?;
    
    let app_state = web::Data::new(AppState {
        db: Arc::new(Mutex::new(conn)),
    });
    
    let address = format!("{}:{}", host, port);
    
    ui::print_header("🌐 Servidor API REST");
    ui::print_success(&format!("Servidor iniciando em http://{}", address));
    ui::print_info("Endpoints disponíveis:");
    use colored::Colorize;
    println!("  {} GET /cnpj/{{cnpj}}  - Consulta dados completos de um CNPJ", "•".cyan());
    println!("  {} GET /health         - Verifica status do servidor", "•".cyan());
    ui::print_verbose(&format!("Exemplo: curl http://{}/cnpj/00000000000191", address));
    ui::print_separator();
    
    actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .app_data(app_state.clone())
            .route("/cnpj/{cnpj}", web::get().to(consultar_cnpj))
            .route("/health", web::get().to(health_check))
    })
    .bind(&address)?
    .workers(num_cpus::get()) // Usa todos os cores disponíveis para multi-threading
    .run()
    .await?;
    
    Ok(())
}

async fn health_check() -> ActixResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "mensagem": "API CNPJ está funcionando"
    })))
}

