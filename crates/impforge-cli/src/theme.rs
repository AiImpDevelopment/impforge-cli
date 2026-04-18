// SPDX-License-Identifier: MIT
//! Futuristic terminal theme — Opera-GX-inspired neon cyberpunk.
//!
//! The banner is intentionally tiny (fits in ~1 ms render budget) so
//! running `impforge-cli` feels instant, not sluggish.  When `--quiet`
//! is passed, nothing is printed.

/// Primary neon green accent — matches the commercial ImpForge brand.
pub const ACCENT_NEON: &str = "\x1b[38;2;0;255;102m";
/// Cyan — data / metrics.
pub const ACCENT_CYAN: &str = "\x1b[38;2;0;204;255m";
/// Magenta — AI / generation.
pub const ACCENT_MAGENTA: &str = "\x1b[38;2;255;51;153m";
/// Dim grey — body text.
pub const DIM: &str = "\x1b[38;2;160;160;176m";
/// Reset.
pub const RESET: &str = "\x1b[0m";
/// Bold.
pub const BOLD: &str = "\x1b[1m";

pub fn print_banner() {
    if std::env::var_os("NO_COLOR").is_some() {
        println!("impforge-cli — MCP-native AI coding companion");
        return;
    }
    println!(
        "{neon}{bold}  ▗▄▄▄▖▗▖  ▗▖▗▄▄▖ ▗▄▄▄▖ ▗▄▖ ▗▄▄▖  ▗▄▄▖▗▄▄▄▖{reset}",
        neon = ACCENT_NEON,
        bold = BOLD,
        reset = RESET
    );
    println!(
        "{neon}{bold}    █  ▐▌▐▌▐▌▐▌ ▐▌▐▌   ▐▌ ▐▌▐▌ ▐▌▐▌   ▐▌   {reset}",
        neon = ACCENT_NEON,
        bold = BOLD,
        reset = RESET
    );
    println!(
        "{neon}{bold}    █  ▐▌▐▌ ▐▌▐▛▀▘ ▐▛▀▀▘▐▌ ▐▌▐▛▀▚▖▐▌▝▜▌▐▛▀▀▘{reset}",
        neon = ACCENT_NEON,
        bold = BOLD,
        reset = RESET
    );
    println!(
        "{cyan}  impforge-cli {dim}·{reset} {magenta}v0.1.0{reset} {dim}·{reset} {neon}MIT{reset}",
        cyan = ACCENT_CYAN,
        magenta = ACCENT_MAGENTA,
        neon = ACCENT_NEON,
        dim = DIM,
        reset = RESET
    );
    println!(
        "{dim}  78 templates · 2 600 compliance rules · MCP-native · local-model-first{reset}",
        dim = DIM,
        reset = RESET
    );
    println!();
}

pub fn print_success(msg: &str) {
    if std::env::var_os("NO_COLOR").is_some() {
        println!("[OK] {msg}");
        return;
    }
    println!("{ACCENT_NEON}{BOLD}✓{RESET} {msg}");
}

pub fn print_warning(msg: &str) {
    if std::env::var_os("NO_COLOR").is_some() {
        println!("[WARN] {msg}");
        return;
    }
    println!("\x1b[38;2;255;170;0m{BOLD}⚠{RESET} {msg}");
}

pub fn print_error(msg: &str) {
    if std::env::var_os("NO_COLOR").is_some() {
        eprintln!("[ERROR] {msg}");
        return;
    }
    eprintln!("\x1b[38;2;255;51;51m{BOLD}✗{RESET} {msg}");
}

pub fn print_info(msg: &str) {
    if std::env::var_os("NO_COLOR").is_some() {
        println!("[INFO] {msg}");
        return;
    }
    println!("{ACCENT_CYAN}ℹ{RESET} {msg}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_are_non_empty() {
        assert!(!ACCENT_NEON.is_empty());
        assert!(!ACCENT_CYAN.is_empty());
        assert!(!ACCENT_MAGENTA.is_empty());
        assert!(!RESET.is_empty());
    }
}
