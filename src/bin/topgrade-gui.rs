#![cfg(unix)]
#![cfg(feature = "gui")]

use glib::MainContext;
use gtk::prelude::*;
use gtk::{glib, Application, ApplicationWindow, Button, ScrolledWindow, TextView, TextBuffer};
use std::env;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

const APP_ID: &str = "com.topgrade.gui";

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(build_ui);

    app.run()
}

fn build_ui(app: &Application) {
    // Create main window
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Topgrade")
        .default_width(800)
        .default_height(600)
        .resizable(true)
        .build();

    // Create main container (vertical box)
    let vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_start(12)
        .margin_end(12)
        .margin_top(12)
        .margin_bottom(12)
        .build();

    // Create explanatory text
    let explanation_label = gtk::Label::builder()
        .label("Topgrade detects which tools you use and runs the appropriate commands to update them.\n\nThis includes package managers, programming language environments, and other tools.\n\nClick the button below to start the update process.")
        .wrap(true)
        .xalign(0.0)
        .build();

    // Create start button
    let start_button = Button::builder()
        .label("Iniciar Atualização")
        .css_classes(&["suggested-action"])
        .build();

    // Create text view for output
    let text_buffer = TextBuffer::builder().build();
    let text_view = TextView::builder()
        .buffer(&text_buffer)
        .editable(false)
        .monospace(true)
        .css_classes(&["output-text"])
        .build();

    // Create scrolled window for text view
    let scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .hexpand(true)
        .vexpand(true)
        .build();
    scrolled_window.set_child(Some(&text_view));

    // Store reference to text_view for auto-scrolling
    let text_view_for_scroll = text_view.clone();

    // Pack widgets
    vbox.append(&explanation_label);
    vbox.append(&start_button);
    vbox.append(&scrolled_window);

    // State for tracking if process is running
    let is_running = Arc::new(Mutex::new(false));
    let is_running_clone = Arc::clone(&is_running);
    let text_buffer_clone = text_buffer.clone();
    let start_button_clone = start_button.clone();
    let text_view_scroll_clone = text_view_for_scroll.clone();

    // Connect button click
    start_button.connect_clicked(move |button| {
        let is_running = Arc::clone(&is_running_clone);
        let text_buffer = text_buffer_clone.clone();
        let button_clone = button.clone();
        let text_view_scroll = text_view_scroll_clone.clone();

        // Check if already running
        {
            let mut running = is_running.lock().unwrap();
            if *running {
                return;
            }
            *running = true;
        }

        // Disable button
        button.set_sensitive(false);
        button.set_label("Atualizando...");

        // Clear previous output
        text_buffer.set_text("");

        // Append initial message
        let initial_text = "Iniciando Topgrade...\n\n";
        let end_iter = text_buffer.end_iter();
        text_buffer.insert(&end_iter, initial_text);

        // Find topgrade executable
        let topgrade_path = find_topgrade_executable();

        // Spawn thread to run topgrade
        thread::spawn(move || {
            let topgrade_path = topgrade_path.clone();
            let text_buffer = text_buffer.clone();
            let button = button_clone.clone();
            let is_running = Arc::clone(&is_running);
            let text_view_scroll = text_view_scroll_clone.clone();

            match run_topgrade(&topgrade_path, text_buffer.clone(), text_view_scroll.clone()) {
                Ok(exit_code) => {
                    // Update UI in main thread
                    let main_context = MainContext::default();
                    main_context.invoke(move || {
                        let end_iter = text_buffer.end_iter();
                        if exit_code == 0 {
                            text_buffer.insert(&end_iter, "\n\n✓ Atualização concluída com sucesso!\n");
                        } else {
                            text_buffer.insert(&end_iter, &format!("\n\n✗ Atualização concluída com código de saída: {}\n", exit_code));
                        }
                        // Scroll to bottom
                        scroll_to_bottom(&text_view_scroll, &text_buffer);
                        button.set_sensitive(true);
                        button.set_label("Iniciar Atualização");
                        *is_running.lock().unwrap() = false;
                    });
                }
                Err(e) => {
                    let main_context = MainContext::default();
                    main_context.invoke(move || {
                        let end_iter = text_buffer.end_iter();
                        text_buffer.insert(&end_iter, &format!("\n\n✗ Erro ao executar topgrade: {}\n", e));
                        scroll_to_bottom(&text_view_scroll, &text_buffer);
                        button.set_sensitive(true);
                        button.set_label("Iniciar Atualização");
                        *is_running.lock().unwrap() = false;
                    });
                }
            }
        });
    });

    window.set_child(Some(&vbox));
    window.present();
}

fn find_topgrade_executable() -> String {
    // First, try to find topgrade in PATH
    if let Ok(path) = which_crate::which("topgrade") {
        return path.to_string_lossy().to_string();
    }

    // If not found, try to use the current executable's directory
    // If we're running as topgrade-gui, the topgrade binary should be in the same directory
    if let Ok(exe_path) = env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            let topgrade_path = parent.join("topgrade");
            if topgrade_path.exists() {
                return topgrade_path.to_string_lossy().to_string();
            }
        }
    }

    // Fallback to just "topgrade" (will be found in PATH if installed system-wide)
    "topgrade".to_string()
}

fn scroll_to_bottom(text_view: &TextView, text_buffer: &TextBuffer) {
    let end_iter = text_buffer.end_iter();
    text_view.scroll_to_iter(&end_iter, 0.0, false, 0.0, 0.0);
}

fn run_topgrade(topgrade_path: &str, text_buffer: TextBuffer, text_view: TextView) -> Result<i32, String> {
    let mut child = Command::new(topgrade_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn topgrade: {}", e))?;

    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;

    let text_buffer_stdout = text_buffer.clone();
    let text_buffer_stderr = text_buffer.clone();
    let text_view_stdout = text_view.clone();
    let text_view_stderr = text_view.clone();

    // Spawn thread to read stdout
    let stdout_handle = thread::spawn(move || {
        let reader = BufReader::new(stdout);
        let main_context = MainContext::default();
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    let text_buffer = text_buffer_stdout.clone();
                    let text_view = text_view_stdout.clone();
                    main_context.invoke(move || {
                        let end_iter = text_buffer.end_iter();
                        text_buffer.insert(&end_iter, &format!("{}\n", line));
                        // Auto-scroll to bottom
                        scroll_to_bottom(&text_view, &text_buffer);
                    });
                }
                Err(e) => {
                    eprintln!("Error reading stdout: {}", e);
                    break;
                }
            }
        }
    });

    // Spawn thread to read stderr
    let stderr_handle = thread::spawn(move || {
        let reader = BufReader::new(stderr);
        let main_context = MainContext::default();
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    let text_buffer = text_buffer_stderr.clone();
                    let text_view = text_view_stderr.clone();
                    main_context.invoke(move || {
                        let end_iter = text_buffer.end_iter();
                        text_buffer.insert(&end_iter, &format!("{}\n", line));
                        // Auto-scroll to bottom
                        scroll_to_bottom(&text_view, &text_buffer);
                    });
                }
                Err(e) => {
                    eprintln!("Error reading stderr: {}", e);
                    break;
                }
            }
        }
    });

    // Wait for process to finish
    let status = child.wait().map_err(|e| format!("Failed to wait for process: {}", e))?;

    // Wait for reader threads to finish
    stdout_handle.join().unwrap();
    stderr_handle.join().unwrap();

    Ok(status.code().unwrap_or(-1))
}

