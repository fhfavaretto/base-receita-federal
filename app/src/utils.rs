use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use encoding_rs;
use encoding_rs_io::DecodeReaderBytesBuilder;
use glob;

pub fn ensure_dir(path: &str) -> Result<()> {
    if !Path::new(path).exists() {
        fs::create_dir_all(path)
            .with_context(|| format!("Falha ao criar diretório: {}", path))?;
    }
    Ok(())
}

pub fn is_dir_empty(path: &str) -> Result<bool> {
    let mut dir = fs::read_dir(path)
        .with_context(|| format!("Falha ao ler diretório: {}", path))?;
    Ok(dir.next().is_none())
}

pub fn get_files_by_extension(dir: &str, ext: &str) -> Result<Vec<PathBuf>> {
    let pattern = format!("{}/*{}", dir, ext);
    let files: Vec<PathBuf> = glob::glob(&pattern)?
        .filter_map(|entry| entry.ok())
        .collect();
    Ok(files)
}

pub fn create_latin1_reader(file_path: &Path) -> Result<Box<dyn std::io::Read>> {
    let file = fs::File::open(file_path)
        .with_context(|| format!("Falha ao abrir arquivo: {:?}", file_path))?;
    
    let reader = DecodeReaderBytesBuilder::new()
        .encoding(Some(encoding_rs::WINDOWS_1252)) // Latin1 equivalente
        .build(file);
    
    Ok(Box::new(reader))
}

pub fn parse_date_from_filename(filename: &str) -> Option<String> {
    // Formato: arquivo.D30610.EMPRECSV -> D30610 -> 10/06/2023
    if let Some(start) = filename.find('D') {
        let date_part = &filename[start..start.min(filename.len()) + 6];
        if date_part.len() == 6 && date_part.starts_with('D') {
            let year = &date_part[1..2];
            let month = &date_part[2..4];
            let day = &date_part[4..6];
            return Some(format!("{}/{}/202{}", day, month, year));
        }
    }
    None
}

pub fn format_progress(current: usize, total: usize) -> String {
    let percent = (current as f64 / total as f64) * 100.0;
    format!("{:.1}% ({}/{})", percent, current, total)
}

