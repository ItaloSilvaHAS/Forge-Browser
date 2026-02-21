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
            loading: false,
        }
    }

    fn navegar(&mut self, url: String, ctx: &egui::Context) {
        let tx = self.tx.clone();
        let ctx_clone = ctx.clone();
        self.loading = true;
        self.query = url.clone();
        thread::spawn(move || {
            let mut target = resolve_smart_query(&url);
            let mut html = baixar_html_bruto(&target);
            
            // Simples loop de redirecionamento
            if html.starts_with("REDIRECT:") {
                target = html.replace("REDIRECT:", "");
                html = baixar_html_bruto(&target);
            }
            
            let elementos = processar_html_semantico(&html);
            let _ = tx.send(elementos);
            ctx_clone.request_repaint();
        });
    }
}

impl eframe::App for ForgeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Ok(novos_elementos) = self.rx.try_recv() {
            self.content = novos_elementos;
            self.loading = false;
        }

        egui::TopBottomPanel::top("top_bar")
            .frame(egui::Frame::none().fill(theme::PANEL_DARK).inner_margin(egui::Margin::symmetric(20.0, 15.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(5.0);
                    let (rect, _) = ui.allocate_at_least(egui::vec2(60.0, 20.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.min + egui::vec2(10.0, 10.0), 6.0, egui::Color32::from_rgb(255, 95, 87));
                    ui.painter().circle_filled(rect.min + egui::vec2(30.0, 10.0), 6.0, egui::Color32::from_rgb(255, 189, 46));
                    ui.painter().circle_filled(rect.min + egui::vec2(50.0, 10.0), 6.0, egui::Color32::from_rgb(40, 201, 64));
                    ui.add_space(20.0);

                    // Container da Barra de Endereço Arredondada
                    let (rect, response) = ui.allocate_at_least(egui::vec2(ui.available_width() - 150.0, 40.0), egui::Sense::click());
                    
                    let visual_bg = if response.has_focus() || ui.memory(|m| m.has_focus(egui::Id::new("search_bar"))) {
                        egui::Color32::from_rgb(50, 50, 50)
                    } else {
                        egui::Color32::from_rgb(40, 40, 40)
                    };
                    ui.painter().rect_filled(rect, 20.0, visual_bg);

                    let text_edit_response = ui.put(
                        rect.shrink2(egui::vec2(15.0, 5.0)),
                        egui::TextEdit::singleline(&mut self.query)
                            .id(egui::Id::new("search_bar"))
                            .hint_text("Search or type URL...")
                            .frame(false)
                    );
                    
                    if response.clicked() {
                        text_edit_response.request_focus();
                    }
                    
                    // Se apertou Enter ou clicou no botão, navega
                    if (text_edit_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) || ui.button("EXPLORE").clicked() {
                        self.navegar(self.query.clone(), ctx);
                    }
                    
                    if self.loading {
                        ui.add_space(10.0);
                        ui.spinner();
                    }
                });
            });

        egui::SidePanel::left("left_sidebar")
            .frame(egui::Frame::none().fill(theme::BACKGROUND_DARK).inner_margin(15.0))
            .width_range(60.0..=80.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    ui.label(egui::RichText::new("F").size(32.0).color(theme::ACCENT_PURPLE).strong());
                    ui.add_space(40.0);
                    let icons = ["🏠", "🔍", "🔖", "⚙"];
                    for icon in icons {
                        if ui.button(egui::RichText::new(icon).size(20.0)).clicked() {}
                        ui.add_space(20.0);
                    }
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(theme::BACKGROUND_DARK))
            .show(ctx, |ui| {
                let rect = ui.available_rect_before_wrap();
                ui.painter().rect_filled(rect.shrink(10.0), 15.0, egui::Color32::from_rgb(20, 20, 20));
                
                ui.allocate_ui_at_rect(rect.shrink(30.0), |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            for elemento in self.content.clone() {
                                match elemento {
                                    ElementoWeb::Titulo(t) => {
                                        ui.label(egui::RichText::new(t).size(28.0).strong().color(theme::ACCENT_PURPLE));
                                        ui.add_space(15.0);
                                    }
                                    ElementoWeb::Texto(txt) => {
                                        ui.label(egui::RichText::new(txt).size(16.0).color(theme::TEXT_GRAY));
                                        ui.add_space(10.0);
                                    }
                                    ElementoWeb::Link(texto, url) => {
                                        if ui.link(egui::RichText::new(format!("🔗 {}", texto)).size(16.0).color(egui::Color32::from_rgb(100, 150, 255))).clicked() {
                                            self.navegar(url, ctx);
                                        }
                                        ui.add_space(8.0);
                                    }
                                }
                            }
                        });
                });
            });
    }
}
