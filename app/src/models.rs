
#[derive(Debug, Clone)]
pub struct Empresa {
    pub cnpj_basico: String,
    pub razao_social: String,
    pub natureza_juridica: String,
    pub qualificacao_responsavel: String,
    pub capital_social_str: String,
    pub porte_empresa: String,
    pub ente_federativo_responsavel: String,
}

#[derive(Debug, Clone)]
pub struct Estabelecimento {
    pub cnpj_basico: String,
    pub cnpj_ordem: String,
    pub cnpj_dv: String,
    pub matriz_filial: String,
    pub nome_fantasia: String,
    pub situacao_cadastral: String,
    pub data_situacao_cadastral: String,
    pub motivo_situacao_cadastral: String,
    pub nome_cidade_exterior: String,
    pub pais: String,
    pub data_inicio_atividades: String,
    pub cnae_fiscal: String,
    pub cnae_fiscal_secundaria: String,
    pub tipo_logradouro: String,
    pub logradouro: String,
    pub numero: String,
    pub complemento: String,
    pub bairro: String,
    pub cep: String,
    pub uf: String,
    pub municipio: String,
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

#[derive(Debug, Clone)]
pub struct Socio {
    pub cnpj_basico: String,
    pub identificador_de_socio: String,
    pub nome_socio: String,
    pub cnpj_cpf_socio: String,
    pub qualificacao_socio: String,
    pub data_entrada_sociedade: String,
    pub pais: String,
    pub representante_legal: String,
    pub nome_representante: String,
    pub qualificacao_representante_legal: String,
    pub faixa_etaria: String,
}

#[derive(Debug, Clone)]
pub struct Simples {
    pub cnpj_basico: String,
    pub opcao_simples: String,
    pub data_opcao_simples: String,
    pub data_exclusao_simples: String,
    pub opcao_mei: String,
    pub data_opcao_mei: String,
    pub data_exclusao_mei: String,
}

#[derive(Debug, Clone)]
pub struct CodigoDescricao {
    pub codigo: String,
    pub descricao: String,
}

