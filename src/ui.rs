use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Password, Select};

pub fn print_banner() {
    println!();
    println!("  {}", style("╭───────────────────────────╮").cyan());
    println!(
        "  {}  {}  {}",
        style("│").cyan(),
        style("MSST-Net 客户端安装程序").cyan().bold(),
        style("│").cyan()
    );
    println!("  {}", style("╰───────────────────────────╯").cyan());
    println!();
}

pub fn print_section(title: &str) {
    println!();
    println!("  {} {}", style("──").dim(), style(title).bold());
    println!();
}

pub fn print_success(msg: &str) {
    println!("  {} {}", style("✓").green().bold(), style(msg).bold());
}

pub fn print_error(msg: &str) {
    let enabled = console::colors_enabled_stderr();
    eprintln!(
        "  {} {}",
        style("✗").red().bold().force_styling(enabled),
        style(msg).red().force_styling(enabled)
    );
}

pub fn print_info(msg: &str) {
    println!("  {} {}", style("·").dim(), style(msg).dim());
}

pub fn print_warning(msg: &str) {
    println!("  {} {}", style("⚠").yellow().bold(), style(msg).yellow());
}

pub fn prompt_yn(question: &str) -> bool {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(question)
        .default(false)
        .interact()
        .unwrap_or(false)
}

pub fn prompt_password(prompt: &str) -> String {
    Password::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .allow_empty_password(true)
        .interact()
        .unwrap_or_default()
}

pub fn prompt_select(prompt: &str, options: &[&str]) -> usize {
    Select::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .items(options)
        .default(0)
        .interact()
        .unwrap_or(0)
}
