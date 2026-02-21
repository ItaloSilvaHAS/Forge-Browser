use eframe::egui;
use scraper::{Html, Selector};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

// --- CONFIGURAÇÕES DO MOTOR FLUX ---
const ROXO_FLUX: egui::Color32 = egui::Color32::from_rgb(100, 50, 200);
const TEMPO_POMODORO: u32 = 1500; // 25 minutos em segundos

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        // CORREÇÃO: Novo sistema de Viewport do eframe
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 750.0])
            .with_title("Flux Engine - Produtividade"),
        ..Default::default()
    };
    
    eframe::run_native(
        "Flux Engine",
        options,
        Box::new(|_cc| Box::new(MyApp::new())),
    )
}

struct MyApp {
    url: String,
    conteudo_renderizado: Vec<ElementoWeb>,
    tx: Sender<Vec<ElementoWeb>>,
    rx: Receiver<Vec<ElementoWeb>>,
    // Estado do Pomodoro
    segundos_restantes: u32,
    timer_ativo: bool,
    ultimo_tick: Instant,
}

#[derive(Clone)]
enum ElementoWeb {
    Titulo(String),
    Texto(String),
}

impl MyApp {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            url: "www.google.com".to_owned(),
            conteudo_renderizado: vec![ElementoWeb::Texto("Pronto para focar? Digite um site e comece.".to_string())],
            tx,
            rx,
            segundos_restantes: TEMPO_POMODORO,
            timer_ativo: false,
            ultimo_tick: Instant::now(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. LÓGICA DO CRONÔMETRO
        if self.timer_ativo && self.segundos_restantes > 0 {
            if self.ultimo_tick.elapsed() >= Duration::from_secs(1) {
                self.segundos_restantes -= 1;
                self.ultimo_tick = Instant::now();
            }
        }

        // 2. COMUNICAÇÃO ENTRE THREADS (RECEBER SITE)
        if let Ok(novos_elementos) = self.rx.try_recv() {
            self.conteudo_renderizado = novos_elementos;
        }

        // 3. UI - PAINEL LATERAL (FLUX TOOLS)
        egui::SidePanel::right("panel_ferramentas")
            .resizable(false)
            .default_width(230.0)
            .show(ctx, |ui| {
                ui.add_space(20.0);
                ui.vertical_centered(|ui| {
                    ui.heading("🚀 FLUX TOOLS");
                });
                ui.separator();
                
                ui.add_space(15.0);
                ui.label("⏱ MODO POMODORO");
                
                let min = self.segundos_restantes / 60;
                let seg = self.segundos_restantes % 60;
                ui.label(egui::RichText::new(format!("{:02}:{:02}", min, seg))
                    .size(55.0)
                    .color(if self.timer_ativo { ROXO_FLUX } else { egui::Color32::GRAY })
                    .strong());

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button(if self.timer_ativo { "PAUSAR" } else { "INICIAR" }).clicked() {
                        self.timer_ativo = !self.timer_ativo;
                        self.ultimo_tick = Instant::now();
                    }
                    if ui.button("RESET").clicked() {
                        self.segundos_restantes = TEMPO_POMODORO;
                        self.timer_ativo = false;
                    }
                });

                ui.add_space(40.0);
                ui.separator();
                ui.label("💡 DICA:");
                ui.weak("Este motor remove CSS/JS para garantir que sua leitura seja 100% focada.");
            });

        // 4. UI - PAINEL CENTRAL (NAVEGADOR)
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("🌐 URL:").strong());
                let edit = ui.text_edit_singleline(&mut self.url);
                
                let enter_pressionado = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                
                if enter_pressionado || ui.button("NAVEGAR").clicked() {
                    let host = self.url.clone();
                    let tx = self.tx.clone();
                    self.conteudo_renderizado = vec![ElementoWeb::Texto("Iniciando requisição TCP...".to_string())];
                    
                    thread::spawn(move || {
                        let html = baixar_html_bruto(&host);
                        let elementos = processar_html_semantico(&html);
                        let _ = tx.send(elementos);
                    });
                }
            });

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);

            // RENDERIZADOR (VIEWPORT DE CONTEÚDO)
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for elemento in &self.conteudo_renderizado {
                        match elemento {
                            ElementoWeb::Titulo(t) => {
                                ui.add_space(12.0);
                                ui.add(egui::Label::new(
                                    egui::RichText::new(format!("  {}  ", t.to_uppercase()))
                                        .heading()
                                        .background_color(ROXO_FLUX)
                                        .color(egui::Color32::WHITE)
                                ));
                                ui.add_space(8.0);
                            }
                            ElementoWeb::Texto(txt) => {
                                ui.add(egui::Label::new(txt).wrap(true));
                                ui.add_space(6.0);
                            }
                        }
                    }
                });
        });

        // Atualiza a cada 100ms para manter o timer fluido
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

// --- ENGINE BACKEND ---

fn baixar_html_bruto(host: &str) -> String {
    let clean_host = host.trim_start_matches("http://").trim_start_matches("https://");
    let address = format!("{}:80", clean_host);
    
    match TcpStream::connect_timeout(&address.parse().unwrap_or(address.parse().ok().unwrap()), Duration::from_secs(5)) {
        Ok(mut stream) => {
            let request = format!(
                "GET / HTTP/1.1\r\nHost: {}\r\nUser-Agent: FluxEngine/1.0\r\nConnection: close\r\n\r\n",
                clean_host
            );
            let _ = stream.write_all(request.as_bytes());
            let mut buffer = Vec::new();
            if let Ok(_) = stream.read_to_end(&mut buffer) {
                String::from_utf8_lossy(&buffer).to_string()
            } else {
                "Erro ao ler resposta do servidor.".to_string()
            }
        }
        Err(e) => format!("Erro de Conexão: {}. (Lembre-se: este motor ainda não suporta HTTPS/443)", e),
    }
}

fn processar_html_semantico(html: &str) -> Vec<ElementoWeb> {
    let document = Html::parse_document(html);
    let mut elementos = Vec::new();

    let seletor_h = Selector::parse("h1, h2, h3").unwrap();
    let seletor_p = Selector::parse("p").unwrap();

    // Ordem de processamento simples
    for h in document.select(&seletor_h) {
        let texto = h.text().collect::<Vec<_>>().join(" ");
        if !texto.trim().is_empty() {
            elementos.push(ElementoWeb::Titulo(texto));
        }
    }

    for p in document.select(&seletor_p) {
        let texto = p.text().collect::<Vec<_>>().join(" ");
        if !texto.trim().is_empty() {
            elementos.push(ElementoWeb::Texto(texto));
        }
    }

    if elementos.is_empty() {
        elementos.push(ElementoWeb::Texto("Nenhum conteúdo semântico encontrado. O site pode ser protegido ou carregar via JavaScript.".to_string()));
    }

    elementos
}