mod engine;
mod ui;

use eframe::egui;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use crate::engine::{ElementoWeb, baixar_html_bruto, processar_html_semantico, resolve_smart_query};
use crate::ui::theme;

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
    url_input: String,
    content: Vec<ElementoWeb>,
    history: Vec<String>,
    history_index: usize,
    tx: Sender<Vec<ElementoWeb>>,
    rx: Receiver<Vec<ElementoWeb>>,
    loading: bool,
    show_home: bool,
}

impl ForgeApp {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            query: "".to_owned(),
            url_input: "".to_owned(),
            content: vec![],
            history: vec![],
            history_index: 0,
            tx,
            rx,
            loading: false,
            show_home: true,
        }
    }

    fn navegar(&mut self, query: String, ctx: &egui::Context, save_history: bool) {
        let target_url = resolve_smart_query(&query);
        self.url_input = target_url.clone();
        self.loading = true;
        self.show_home = false;
        
        if save_history {
            if self.history_index < self.history.len() {
                self.history.truncate(self.history_index + 1);
            }
            self.history.push(target_url.clone());
            self.history_index = self.history.len() - 1;
        }

        let tx = self.tx.clone();
        let ctx_clone = ctx.clone();
        let url_to_fetch = target_url.clone();

        thread::spawn(move || {
            let html = baixar_html_bruto(&url_to_fetch);
            let elementos = processar_html_semantico(&html, &url_to_fetch);
            let _ = tx.send(elementos);
            ctx_clone.request_repaint();
        });
    }

    fn go_back(&mut self, ctx: &egui::Context) {
        if self.history_index > 0 {
            self.history_index -= 1;
            let url = self.history[self.history_index].clone();
            self.navegar(url, ctx, false);
        }
    }

    fn go_forward(&mut self, ctx: &egui::Context) {
        if self.history_index + 1 < self.history.len() {
            self.history_index += 1;
            let url = self.history[self.history_index].clone();
            self.navegar(url, ctx, false);
        }
    }

    fn reload(&mut self, ctx: &egui::Context) {
        if !self.url_input.is_empty() {
            self.navegar(self.url_input.clone(), ctx, false);
        }
    }

    fn go_home(&mut self) {
        self.show_home = true;
        self.url_input = "".to_string();
        self.query = "".to_string();
        self.content = vec![];
    }
}

impl eframe::App for ForgeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Ok(novos_elementos) = self.rx.try_recv() {
            self.content = novos_elementos;
            self.loading = false;
        }

        egui::TopBottomPanel::top("top_bar")
            .frame(egui::Frame::none().fill(theme::PANEL_DARK).inner_margin(egui::Margin::symmetric(20.0, 10.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Botões de Navegação
                    ui.spacing_mut().item_spacing.x = 10.0;
                    if ui.button(egui::RichText::new("⬅").size(18.0)).clicked() {
                        self.go_back(ctx);
                    }
                    if ui.button(egui::RichText::new("➡").size(18.0)).clicked() {
                        self.go_forward(ctx);
                    }
                    if ui.button(egui::RichText::new("⟳").size(18.0)).clicked() {
                        self.reload(ctx);
                    }
                    if ui.button(egui::RichText::new("🏠").size(18.0)).clicked() {
                        self.go_home();
                    }

                    ui.add_space(10.0);

                    // Barra de Endereço
                    let rect = ui.available_rect_before_wrap();
                    let (rect, response) = ui.allocate_at_least(egui::vec2(ui.available_width() - 80.0, 32.0), egui::Sense::click());
                    
                    let visual_bg = if response.has_focus() || ui.memory(|m| m.has_focus(egui::Id::new("search_bar"))) {
                        egui::Color32::from_rgb(60, 60, 60)
                    } else {
                        egui::Color32::from_rgb(45, 45, 45)
                    };
                    ui.painter().rect_filled(rect, 16.0, visual_bg);

                    let text_edit_response = ui.put(
                        rect.shrink2(egui::vec2(15.0, 4.0)),
                        egui::TextEdit::singleline(&mut self.url_input)
                            .id(egui::Id::new("search_bar"))
                            .hint_text("Search or type URL...")
                            .frame(false)
                    );
                    
                    if response.clicked() {
                        text_edit_response.request_focus();
                    }
                    
                    if (text_edit_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                        self.navegar(self.url_input.clone(), ctx, true);
                    }
                    
                    if self.loading {
                        ui.add_space(10.0);
                        ui.spinner();
                    }
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(theme::BACKGROUND_DARK))
            .show(ctx, |ui| {
                if self.show_home {
                    // Tela Inicial Centralizada
                    ui.vertical_centered(|ui| {
                        ui.add_space(ui.available_height() * 0.25);
                        ui.label(egui::RichText::new("FORGE").size(80.0).color(theme::ACCENT_PURPLE).strong());
                        ui.label(egui::RichText::new("Navegação pura. Foco total.").size(20.0).color(theme::TEXT_GRAY));
                        ui.add_space(40.0);

                        let (rect, _) = ui.allocate_at_least(egui::vec2(500.0, 45.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 22.5, egui::Color32::from_rgb(45, 45, 45));
                        
                        ui.allocate_ui_at_rect(rect.shrink2(egui::vec2(20.0, 5.0)), |ui| {
                            let res = ui.add(egui::TextEdit::singleline(&mut self.query)
                                .hint_text("O que você quer descobrir hoje?")
                                .frame(false)
                                .font(egui::FontId::proportional(20.0)));
                            
                            if res.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                self.navegar(self.query.clone(), ctx, true);
                            }
                        });
                        
                        ui.add_space(20.0);
                        if ui.add(egui::Button::new(egui::RichText::new("Explorar").size(18.0)).rounding(20.0).fill(theme::ACCENT_PURPLE)).clicked() {
                             self.navegar(self.query.clone(), ctx, true);
                        }
                    });
                } else {
                    // Renderização do Conteúdo
                    let rect = ui.available_rect_before_wrap();
                    ui.painter().rect_filled(rect.shrink(10.0), 10.0, egui::Color32::from_rgb(25, 25, 25));
                    
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
                                                self.navegar(url, ctx, true);
                                            }
                                            ui.add_space(8.0);
                                        }
                                    }
                                }
                            });
                    });
                }
            });
    }
}
