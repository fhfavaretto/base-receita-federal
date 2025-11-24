use colored::*;
use std::io::{self, Write};

static mut QUIET: bool = false;
static mut VERBOSE: bool = false;

pub fn init(quiet: bool, verbose: bool) {
    unsafe {
        QUIET = quiet;
        VERBOSE = verbose;
    }
}

fn is_quiet() -> bool {
    unsafe { QUIET }
}

fn is_verbose() -> bool {
    unsafe { VERBOSE }
}

pub fn print_info(message: &str) {
    if !is_quiet() {
        println!("{} {}", "ℹ".blue(), message);
    }
}

pub fn print_success(message: &str) {
    if !is_quiet() {
        println!("{} {}", "✓".green().bold(), message.green());
    }
}

pub fn print_warning(message: &str) {
    if !is_quiet() {
        println!("{} {}", "⚠".yellow().bold(), message.yellow());
    }
}

pub fn print_error(message: &str) {
    eprintln!("{} {}", "✗".red().bold(), message.red());
}

pub fn print_header(message: &str) {
    if !is_quiet() {
        println!("\n{}", message.bold().cyan());
        println!("{}", "─".repeat(message.len()).cyan());
    }
}

pub fn print_verbose(message: &str) {
    if is_verbose() && !is_quiet() {
        println!("  {}", message.dimmed());
    }
}

pub fn print_step(step: usize, total: usize, message: &str) {
    if !is_quiet() {
        println!("[{}/{}] {}", step, total, message.bold());
    }
}

pub fn ask_confirmation(prompt: &str, default: bool) -> io::Result<bool> {
    if is_quiet() {
        return Ok(default);
    }
    
    let default_str = if default { "Y/n" } else { "y/N" };
    print!("{} [{}]: ", prompt.bold(), default_str);
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    let trimmed = input.trim().to_lowercase();
    if trimmed.is_empty() {
        Ok(default)
    } else {
        Ok(trimmed == "y" || trimmed == "s" || trimmed == "yes" || trimmed == "sim")
    }
}

pub fn ask_confirmation_yes(prompt: &str) -> io::Result<bool> {
    ask_confirmation(prompt, true)
}

pub fn ask_confirmation_no(prompt: &str) -> io::Result<bool> {
    ask_confirmation(prompt, false)
}

pub fn print_statistics(stats: &[(&str, u64)]) {
    if is_quiet() {
        return;
    }
    
    println!("\n{}", "Estatísticas:".bold().cyan());
    for (label, value) in stats {
        println!("  {}: {}", label.bold(), value.to_string().green());
    }
}

pub fn print_separator() {
    if !is_quiet() {
        println!("{}", "=".repeat(60).dimmed());
    }
}

