use anyhow::{Context, Result};
use rusqlite::{Connection, params, Transaction};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)
            .with_context(|| format!("Falha ao criar/abrir banco: {}", path))?;
        
        Ok(Self { conn })
    }

    pub fn create_tables(&self) -> Result<()> {
        // Tabelas de referência
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS cnae (codigo TEXT PRIMARY KEY, descricao TEXT)",
            [],
        )?;
        
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS motivo (codigo TEXT PRIMARY KEY, descricao TEXT)",
            [],
        )?;
        
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS municipio (codigo TEXT PRIMARY KEY, descricao TEXT)",
            [],
        )?;
        
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS natureza_juridica (codigo TEXT PRIMARY KEY, descricao TEXT)",
            [],
        )?;
        
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS pais (codigo TEXT PRIMARY KEY, descricao TEXT)",
            [],
        )?;
        
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS qualificacao_socio (codigo TEXT PRIMARY KEY, descricao TEXT)",
            [],
        )?;

        // Tabelas principais
        self.conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS empresas (
                cnpj_basico TEXT,
                razao_social TEXT,
                natureza_juridica TEXT,
                qualificacao_responsavel TEXT,
                capital_social_str TEXT,
                porte_empresa TEXT,
                ente_federativo_responsavel TEXT
            )
            "#,
            [],
        )?;

        self.conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS estabelecimento (
                cnpj_basico TEXT,
                cnpj_ordem TEXT,
                cnpj_dv TEXT,
                matriz_filial TEXT,
                nome_fantasia TEXT,
                situacao_cadastral TEXT,
                data_situacao_cadastral TEXT,
                motivo_situacao_cadastral TEXT,
                nome_cidade_exterior TEXT,
                pais TEXT,
                data_inicio_atividades TEXT,
                cnae_fiscal TEXT,
                cnae_fiscal_secundaria TEXT,
                tipo_logradouro TEXT,
                logradouro TEXT,
                numero TEXT,
                complemento TEXT,
                bairro TEXT,
                cep TEXT,
                uf TEXT,
                municipio TEXT,
                ddd1 TEXT,
                telefone1 TEXT,
                ddd2 TEXT,
                telefone2 TEXT,
                ddd_fax TEXT,
                fax TEXT,
                correio_eletronico TEXT,
                situacao_especial TEXT,
                data_situacao_especial TEXT
            )
            "#,
            [],
        )?;

        self.conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS socios_original (
                cnpj_basico TEXT,
                identificador_de_socio TEXT,
                nome_socio TEXT,
                cnpj_cpf_socio TEXT,
                qualificacao_socio TEXT,
                data_entrada_sociedade TEXT,
                pais TEXT,
                representante_legal TEXT,
                nome_representante TEXT,
                qualificacao_representante_legal TEXT,
                faixa_etaria TEXT
            )
            "#,
            [],
        )?;

        self.conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS simples (
                cnpj_basico TEXT,
                opcao_simples TEXT,
                data_opcao_simples TEXT,
                data_exclusao_simples TEXT,
                opcao_mei TEXT,
                data_opcao_mei TEXT,
                data_exclusao_mei TEXT
            )
            "#,
            [],
        )?;

        Ok(())
    }

    pub fn insert_codigo_descricao(&self, table: &str, codigo: &str, descricao: &str) -> Result<()> {
        let sql = format!("INSERT OR REPLACE INTO {} (codigo, descricao) VALUES (?1, ?2)", table);
        self.conn.execute(&sql, params![codigo, descricao])?;
        Ok(())
    }

    pub fn create_index(&self, table: &str, column: &str) -> Result<()> {
        let index_name = format!("idx_{}_{}", table, column);
        let sql = format!("CREATE INDEX IF NOT EXISTS {} ON {}({})", index_name, table, column);
        self.conn.execute(&sql, [])?;
        Ok(())
    }

    pub fn begin_transaction(&mut self) -> Result<Transaction> {
        Ok(self.conn.transaction()?)
    }

    pub fn execute(&self, sql: &str) -> Result<()> {
        self.conn.execute(sql, [])?;
        Ok(())
    }

    pub fn execute_with_params(&self, sql: &str, params: &[&dyn rusqlite::ToSql]) -> Result<()> {
        self.conn.execute(sql, params)?;
        Ok(())
    }

    pub fn finalize_processing(&self, data_referencia: &str) -> Result<()> {
        // Ajusta capital social
        self.conn.execute(
            "ALTER TABLE empresas ADD COLUMN capital_social REAL",
            [],
        ).ok(); // Ignora se já existe
        
        self.conn.execute(
            "UPDATE empresas SET capital_social = CAST(REPLACE(capital_social_str, ',', '.') AS REAL)",
            [],
        )?;
        
        self.conn.execute(
            "ALTER TABLE empresas DROP COLUMN capital_social_str",
            [],
        ).ok(); // Ignora se não existe

        // Cria campo CNPJ completo
        self.conn.execute(
            "ALTER TABLE estabelecimento ADD COLUMN cnpj TEXT",
            [],
        ).ok();
        
        self.conn.execute(
            "UPDATE estabelecimento SET cnpj = cnpj_basico || cnpj_ordem || cnpj_dv",
            [],
        )?;

        // Cria índices principais
        self.create_index("empresas", "cnpj_basico")?;
        self.create_index("empresas", "razao_social")?;
        self.create_index("estabelecimento", "cnpj_basico")?;
        self.create_index("estabelecimento", "cnpj")?;
        self.create_index("estabelecimento", "nome_fantasia")?;
        self.create_index("socios_original", "cnpj_basico")?;

        // Cria tabela socios apenas com matrizes
        self.conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS socios AS 
            SELECT te.cnpj as cnpj, ts.*
            FROM socios_original ts
            LEFT JOIN estabelecimento te ON te.cnpj_basico = ts.cnpj_basico
            WHERE te.matriz_filial = '1'
            "#,
            [],
        )?;

        self.conn.execute("DROP TABLE IF EXISTS socios_original", [])?;

        // Índices na tabela socios
        self.create_index("socios", "cnpj")?;
        self.create_index("socios", "cnpj_cpf_socio")?;
        self.create_index("socios", "nome_socio")?;
        self.create_index("socios", "representante_legal")?;
        self.create_index("socios", "nome_representante")?;
        self.create_index("simples", "cnpj_basico")?;

        // Tabela de referência
        self.conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS _referencia (
                referencia TEXT,
                valor TEXT
            )
            "#,
            [],
        )?;

        let qtde_cnpjs: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM estabelecimento",
            [],
            |row| row.get(0),
        )?;

        self.conn.execute(
            "INSERT INTO _referencia (referencia, valor) VALUES ('CNPJ', ?1)",
            params![data_referencia],
        )?;
        
        self.conn.execute(
            "INSERT INTO _referencia (referencia, valor) VALUES ('cnpj_qtde', ?1)",
            params![qtde_cnpjs.to_string()],
        )?;

        Ok(())
    }

    pub fn get_connection(&self) -> &Connection {
        &self.conn
    }
}

