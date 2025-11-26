#![cfg(unix)]
#![cfg(feature = "gui")]

use eframe::egui;
use rust_i18n::{i18n, t};
use std::env;
use std::process::Command;

// Init i18n - carrega traduções do diretório locales
// O macro i18n! compila as traduções no binário em tempo de compilação
i18n!("locales", fallback = "en");

struct TopgradeApp {
    topgrade_path: String,
    locale: String,
}

impl Default for TopgradeApp {
    fn default() -> Self {
        Self {
            topgrade_path: find_topgrade_executable(),
            locale: String::new(), // Será configurado no main
        }
    }
}

impl eframe::App for TopgradeApp {
    fn update(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(_ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);

                // Title
                ui.heading(t!("Topgrade GUI - Title"));
                ui.add_space(10.0);

                // Explanation text
                ui.label(t!("Topgrade GUI - Description"));
                ui.add_space(20.0);

                // Start button
                let button_text = t!("Topgrade GUI - Start Button");

                if ui.add(egui::Button::new(button_text).min_size(egui::vec2(200.0, 40.0))).clicked() {
                    self.start_topgrade_external();
                }

                ui.add_space(10.0);

                // Info sobre terminal externo
                ui.label(egui::RichText::new(t!("Topgrade GUI - External Terminal Info")).small().weak());
            });
        });
    }
}

impl TopgradeApp {
    fn start_topgrade_external(&self) {
        // Executar topgrade em terminal externo para permitir interação completa
        // (senhas, confirmações, etc.)
        let topgrade_path = self.topgrade_path.clone();

        // Tentar encontrar terminal disponível
        let (terminal_cmd, args) = if which_crate::which("gnome-terminal").is_ok() {
            ("gnome-terminal", vec!["--", "bash", "-c"])
        } else if which_crate::which("xterm").is_ok() {
            ("xterm", vec!["-e", "bash", "-c"])
        } else if which_crate::which("x-terminal-emulator").is_ok() {
            ("x-terminal-emulator", vec!["-e", "bash", "-c"])
        } else if which_crate::which("konsole").is_ok() {
            ("konsole", vec!["-e", "bash", "-c"])
        } else if which_crate::which("tilix").is_ok() {
            ("tilix", vec!["-e", "bash", "-c"])
        } else {
            eprintln!("Nenhum terminal encontrado. Execute topgrade manualmente no terminal.");
            return;
        };

        // Commando que executa topgrade e espera antes de fechar
        // Usar tradução para a mensagem de fechar
        let close_message = t!("Topgrade GUI - Press Enter to close");
        let command = format!("LC_ALL={}; {} 2>&1; echo ''; echo '{}'; read",
                             self.locale, topgrade_path, close_message);

        if let Err(e) = Command::new(terminal_cmd)
            .args(&args)
            .arg(&command)
            .spawn()
        {
            eprintln!("Error ao abrir terminal: {}", e);
        }
    }
}

fn find_topgrade_executable() -> String {
    // First, try to find topgrade in PATH
    if let Ok(path) = which_crate::which("topgrade") {
        return path.to_string_lossy().to_string();
    }

    // If not found, try to use the current executable's directory
    if let Ok(exe_path) = env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            let topgrade_path = parent.join("topgrade");
            if topgrade_path.exists() {
                return topgrade_path.to_string_lossy().to_string();
            }
        }

        // Try to find in common build directories (for development)
        if let Some(workspace_root) = exe_path.parent().and_then(|p| {
            p.ancestors().find(|p| p.join("Cargo.toml").exists())
        }) {
            let debug_path = workspace_root.join("target/debug/topgrade");
            if debug_path.exists() {
                return debug_path.to_string_lossy().to_string();
            }
            let release_path = workspace_root.join("target/release/topgrade");
            if release_path.exists() {
                return release_path.to_string_lossy().to_string();
            }
        }
    }

    // Fallback to just "topgrade"
    "topgrade".to_string()
}

fn main() -> Result<(), eframe::Error> {
    // Detectar locale do sistema
    let system_locale = sys_locale::get_locale().unwrap_or_else(|| "en".to_string());

    // Normalizar locale para o formato esperado pelo rust-i18n:
    // 1. Remover .UTF-8, .utf8, etc.
    // 2. Converter hífen (-) para underscore (_) porque o YAML usa pt_BR, não pt-BR
    let mut normalized_locale = if let Some(dot_pos) = system_locale.find('.') {
        system_locale[..dot_pos].to_string()
    } else {
        system_locale.clone()
    };

    // Converter hífen para underscore (pt-BR -> pt_BR)
    normalized_locale = normalized_locale.replace('-', "_");

    // Configurar locale normalizado
    rust_i18n::set_locale(&normalized_locale);

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(800.0, 600.0)),
        ..Default::default()
    };

    // Criar app e configurar locale
    let mut app = TopgradeApp::default();
    app.locale = normalized_locale.clone();

    eframe::run_native(
        &t!("Topgrade GUI - Title"),
        options,
        Box::new(move |_cc| Box::new(app)),
    )
}
