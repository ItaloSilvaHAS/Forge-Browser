mod engine;
mod ui;

use eframe::egui;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};
use crate::engine::{ElementoWeb, baixar_html_bruto, processar_html_semantico, resolve_smart_query};
use crate::ui::theme;

const TEMPO_POMODORO: u32 = 1500;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("Forge Browser"),
        ..Default::default()
    };
    
    eframe::run_native(
        "Forge Browser",
        options,
        Box::new(|cc| {
            theme::apply_style(&cc.egui_ctx);
            Box::new(ForgeApp::new())
        }),
    )
}

struct ForgeApp {
    query: String,
    content: Vec<ElementoWeb>,
    tx: Sender<Vec<ElementoWeb>>,
    rx: Receiver<Vec<ElementoWeb>>,
    segundos_restantes: u32,
    timer_ativo: bool,
    ultimo_tick: Instant,
    loading: bool,
}

impl ForgeApp {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            query: "google.com".to_owned(),
            content: vec![ElementoWeb::Texto("Forge Browser: Navegação focada e minimalista.".to_string())],
            tx,
            rx,
            segundos_restantes: TEMPO_POMODORO,
            timer_ativo: false,
            ultimo_tick: Instant::now(),
            loading: false,
        }
    }
}

fn draw_shadow(_ui: &mut egui::Ui, _rect: egui::Rect, _rounding: impl Into<egui::Rounding>) {
    // Shadow não disponível na versão 0.24 ou requer import diferente
}

impl eframe::App for ForgeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.timer_ativo && self.segundos_restantes > 0 {
            if self.ultimo_tick.elapsed() >= Duration::from_secs(1) {
                self.segundos_restantes -= 1;
                self.ultimo_tick = Instant::now();
            }
        }

        if let Ok(novos_elementos) = self.rx.try_recv() {
            self.content = novos_elementos;
            self.loading = false;
        }

        // Top Panel - UI Estilo Opera / Moderna
        egui::TopBottomPanel::top("top_bar")
            .frame(egui::Frame::none().fill(theme::PANEL_DARK).inner_margin(egui::Margin::symmetric(20.0, 15.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(5.0);
                    // Botões de controle simulados (Estilo Mac/Opera)
                    let (rect, _) = ui.allocate_at_least(egui::vec2(60.0, 20.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.min + egui::vec2(10.0, 10.0), 6.0, egui::Color32::from_rgb(255, 95, 87));
                    ui.painter().circle_filled(rect.min + egui::vec2(30.0, 10.0), 6.0, egui::Color32::from_rgb(255, 189, 46));
                    ui.painter().circle_filled(rect.min + egui::vec2(50.0, 10.0), 6.0, egui::Color32::from_rgb(40, 201, 64));
                    ui.add_space(20.0);

                    // Container da Barra de Endereço Arredondada
                    let search_query = egui::TextEdit::singleline(&mut self.query)
                        .hint_text("Search with Forge or type URL...")
                        .desired_width(ui.available_width() - 150.0)
                        .margin(egui::vec2(15.0, 10.0))
                        .frame(false);

                    let response = ui.add(search_query);
                    
                    // Desenha fundo da barra de busca
                    ui.painter().rect_filled(response.rect, 20.0, egui::Color32::from_rgb(40, 40, 40));

                    if (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) || ui.button("EXPLORE").clicked() {
                        let target = resolve_smart_query(&self.query);
                        let tx = self.tx.clone();
                        let ctx_clone = ctx.clone();
                        self.loading = true;
                        thread::spawn(move || {
                            let html = baixar_html_bruto(&target);
                            let elementos = processar_html_semantico(&html);
                            let _ = tx.send(elementos);
                            ctx_clone.request_repaint(); // Força atualização da UI após carregar
                        });
                    }
                    
                    if self.loading {
                        ui.add_space(10.0);
                        ui.spinner();
                    }
                });
            });

        // Sidebar Minimalista
        egui::SidePanel::left("left_sidebar")
            .frame(egui::Frame::none().fill(theme::BACKGROUND_DARK).inner_margin(15.0))
            .width_range(60.0..=80.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    ui.label(egui::RichText::new("F").size(32.0).color(theme::ACCENT_PURPLE).strong());
                    ui.add_space(40.0);
                    
                    // Ícones simulados
                    let icons = ["🏠", "🔍", "🔖", "⚙"];
                    for icon in icons {
                        if ui.button(egui::RichText::new(icon).size(20.0)).clicked() {}
                        ui.add_space(20.0);
                    }
                });
            });

        // Central Area - Renderização com "Glass" Effect simulado
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(theme::BACKGROUND_DARK))
            .show(ctx, |ui| {
                let rect = ui.available_rect_before_wrap();
                ui.painter().rect_filled(rect.shrink(10.0), 15.0, egui::Color32::from_rgb(20, 20, 20));
                
                ui.allocate_ui_at_rect(rect.shrink(30.0), |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            for elemento in &self.content {
                                match elemento {
                                    ElementoWeb::Titulo(t) => {
                                        ui.label(egui::RichText::new(t).size(28.0).strong().color(theme::ACCENT_PURPLE));
                                        ui.add_space(15.0);
                                    }
                                    ElementoWeb::Texto(txt) => {
                                        ui.label(egui::RichText::new(txt).size(16.0).color(theme::TEXT_GRAY));
                                        ui.add_space(10.0);
                                    }
                                }
                            }
                        });
                });
            });

        // Pomodoro Floating Widget (Estilo Opera Player)
        egui::Window::new("Flux Player")
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-20.0, -20.0))
            .resizable(false)
            .collapsible(true)
            .title_bar(false)
            .frame(egui::Frame::window(&ctx.style()).fill(egui::Color32::from_rgb(35, 35, 35)).rounding(20.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    let min = self.segundos_restantes / 60;
                    let seg = self.segundos_restantes % 60;
                    ui.label(egui::RichText::new(format!("{:02}:{:02}", min, seg)).size(24.0).strong());
                    
                    ui.add_space(10.0);
                    if ui.button(if self.timer_ativo { "⏸" } else { "▶" }).clicked() {
                        self.timer_ativo = !self.timer_ativo;
                        self.ultimo_tick = Instant::now();
                    }
                    ui.add_space(10.0);
                });
            });

        ctx.request_repaint_after(Duration::from_millis(100));
    }
}
