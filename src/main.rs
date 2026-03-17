mod engine;
mod ui;

use eframe::egui;
// Trocamos mpsc por crossbeam_channel para garantir Thread Safety (Send)
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::thread;
use crate::engine::{ParsedPage, fetch_page, resolve_smart_query};
use crate::ui::{theme, renderer};

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 860.0])
            .with_title("Forge Browser"),
        ..Default::default()
    };

    eframe::run_native(
        "Forge Browser",
        options,
        Box::new(|cc| {
            theme::apply_style(&cc.egui_ctx);
            // Retorno Ok(Box::new(...)) exigido pela versão 0.24+
            Ok(Box::new(ForgeApp::new()))
        }),
    )
}

// ── Estado da aplicação ───────────────────────────────────────────────────────

struct ForgeApp {
    url_input: String,
    query_home: String,
    page: Option<ParsedPage>,
    history: Vec<String>,
    history_index: usize,
    tx: Sender<ParsedPage>,
    rx: Receiver<ParsedPage>, // crossbeam::Receiver implementa Send
    loading: bool,
    show_home: bool,
    status_msg: String,
}

impl ForgeApp {
    fn new() -> Self {
        // Criando canal crossbeam
        let (tx, rx) = unbounded();
        Self {
            url_input: String::new(),
            query_home: String::new(),
            page: None,
            history: Vec::new(),
            history_index: 0,
            tx,
            rx,
            loading: false,
            show_home: true,
            status_msg: String::new(),
        }
    }

    fn navigate(&mut self, input: String, ctx: &egui::Context, save_history: bool) {
        let url = resolve_smart_query(&input);
        if url.is_empty() { return; }

        self.url_input = url.clone();
        self.loading = true;
        self.show_home = false;
        self.status_msg = format!("Carregando {}", url);

        if save_history {
            if self.history_index < self.history.len() {
                self.history.truncate(self.history_index + 1);
            }
            self.history.push(url.clone());
            self.history_index = self.history.len().saturating_sub(1);
        }

        let tx = self.tx.clone();
        let ctx_c = ctx.clone();
        let url_c = url;

        thread::spawn(move || {
            let page = fetch_page(&url_c);
            let _ = tx.send(page);
            ctx_c.request_repaint();
        });
    }

    fn go_back(&mut self, ctx: &egui::Context) {
        if self.history_index > 0 {
            self.history_index -= 1;
            let url = self.history[self.history_index].clone();
            self.navigate(url, ctx, false);
        }
    }

    fn go_forward(&mut self, ctx: &egui::Context) {
        if self.history_index + 1 < self.history.len() {
            self.history_index += 1;
            let url = self.history[self.history_index].clone();
            self.navigate(url, ctx, false);
        }
    }

    fn reload(&mut self, ctx: &egui::Context) {
        if !self.url_input.is_empty() {
            let url = self.url_input.clone();
            self.navigate(url, ctx, false);
        }
    }

    fn go_home(&mut self) {
        self.show_home = true;
        self.url_input.clear();
        self.query_home.clear();
        self.page = None;
        self.status_msg.clear();
    }
}

// ── Loop principal egui ───────────────────────────────────────────────────────

impl eframe::App for ForgeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Recebe página carregada (try_recv do crossbeam)
        if let Ok(page) = self.rx.try_recv() {
            self.status_msg = format!("{} — {}", page.title, page.url);
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(
                format!("{} — Forge", page.title)
            ));
            self.page = Some(page);
            self.loading = false;
        }

        // ── Barra de navegação superior ──────────────────────────────────────
        egui::TopBottomPanel::top("nav_bar")
            .frame(
                egui::Frame::none()
                    .fill(theme::PANEL_DARK)
                    .inner_margin(egui::Margin::symmetric(16.0, 8.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;

                    let can_back = self.history_index > 0;
                    let can_fwd = self.history_index + 1 < self.history.len();

                    if ui.add_enabled(can_back, egui::Button::new(
                        egui::RichText::new("◀").size(16.0)
                    )).clicked() { self.go_back(ctx); }

                    if ui.add_enabled(can_fwd, egui::Button::new(
                        egui::RichText::new("▶").size(16.0)
                    )).clicked() { self.go_forward(ctx); }

                    if ui.button(egui::RichText::new("⟳").size(18.0)).clicked() {
                        self.reload(ctx);
                    }
                    if ui.button(egui::RichText::new("🏠").size(16.0)).clicked() {
                        self.go_home();
                    }

                    ui.add_space(8.0);

                    let bar_width = ui.available_width() - if self.loading { 32.0 } else { 0.0 };
                    let (rect, _) = ui.allocate_at_least(
                        egui::vec2(bar_width, 32.0),
                        egui::Sense::hover(),
                    );
                    ui.painter().rect_filled(rect, 16.0, egui::Color32::from_rgb(38, 38, 48));

                    let te = ui.put(
                        rect.shrink2(egui::vec2(14.0, 4.0)),
                        egui::TextEdit::singleline(&mut self.url_input)
                            .id(egui::Id::new("addr_bar"))
                            .hint_text("Digite uma URL ou pesquise...")
                            .frame(false),
                    );

                    if te.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let q = self.url_input.clone();
                        self.navigate(q, ctx, true);
                    }

                    if self.loading {
                        ui.add_space(4.0);
                        ui.spinner();
                    }
                });
            });

        // ── Barra de status inferior ──────────────────────────────────────────
        if !self.status_msg.is_empty() {
            egui::TopBottomPanel::bottom("status_bar")
                .frame(
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(16, 16, 22))
                        .inner_margin(egui::Margin::symmetric(16.0, 3.0)),
                )
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new(&self.status_msg)
                            .size(11.0)
                            .color(egui::Color32::from_rgb(130, 130, 150)),
                    );
                });
        }

        // ── Conteúdo central ─────────────────────────────────────────────────
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(theme::BACKGROUND_DARK))
            .show(ctx, |ui| {
                if self.show_home {
                    // Renderização da home page
                    ui.vertical_centered(|ui| {
                        ui.add_space(ui.available_height() * 0.22);

                        ui.label(
                            egui::RichText::new("FORGE")
                                .size(72.0)
                                .color(theme::ACCENT_PURPLE)
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new("Motor. Motor. Motor.")
                                .size(18.0)
                                .color(theme::TEXT_GRAY)
                                .italics(),
                        );
                        ui.add_space(36.0);

                        let (rect, _) = ui.allocate_at_least(
                            egui::vec2(540.0, 44.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_filled(rect, 22.0, egui::Color32::from_rgb(38, 38, 52));

                        let res = ui.put(
                            rect.shrink2(egui::vec2(18.0, 6.0)),
                            egui::TextEdit::singleline(&mut self.query_home)
                                .hint_text("O que você quer descobrir?")
                                .frame(false)
                                .font(egui::FontId::proportional(18.0)),
                        );

                        if res.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            let q = self.query_home.clone();
                            self.navigate(q, ctx, true);
                        }

                        ui.add_space(18.0);
                        if ui.add(
                            egui::Button::new(
                                egui::RichText::new("  Explorar  ").size(16.0),
                            )
                            .rounding(20.0)
                            .fill(theme::ACCENT_PURPLE),
                        ).clicked() {
                            let q = self.query_home.clone();
                            self.navigate(q, ctx, true);
                        }
                    });
                } else {
                    // ── Área de renderização da página ────────────────────────
                    let available = ui.available_rect_before_wrap();
                    ui.painter().rect_filled(available, 0.0, egui::Color32::from_rgb(18, 18, 24));

                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            let inner_rect = egui::Rect::from_min_size(
                                available.min,
                                egui::vec2(available.width().min(960.0), available.height()),
                            );

                            ui.set_max_width(inner_rect.width());

                            ui.add_space(16.0);
                            ui.horizontal(|ui| {
                                ui.add_space(24.0);
                                ui.vertical(|ui| {
                                    ui.set_max_width(inner_rect.width() - 48.0);

                                    if self.loading {
                                        loading_placeholder(ui);
                                    } else if let Some(page) = &self.page {
                                        let nav_url = renderer::render(ui, &page.dom);
                                        if let Some(url) = nav_url {
                                            ctx.memory_mut(|m| {
                                                m.data.insert_temp(egui::Id::new("pending_nav"), url);
                                            });
                                        }
                                    }
                                });
                            });
                            ui.add_space(40.0);
                        });

                    // Processa navegação pendente
                    let pending = ctx.memory_mut(|m| {
                        m.data.get_temp::<String>(egui::Id::new("pending_nav"))
                    });
                    if let Some(url) = pending {
                        ctx.memory_mut(|m| {
                            m.data.remove::<String>(egui::Id::new("pending_nav"))
                        });
                        if !url.is_empty() && !url.starts_with('#') && !url.starts_with("javascript:") {
                            self.navigate(url, ctx, true);
                        }
                    }
                }
            });
    }
}

// ── Helpers de UI ─────────────────────────────────────────────────────────────

fn loading_placeholder(ui: &mut egui::Ui) {
    ui.add_space(60.0);
    ui.vertical_centered(|ui| {
        ui.spinner();
        ui.add_space(16.0);
        ui.label(
            egui::RichText::new("Carregando página...")
                .size(16.0)
                .color(theme::TEXT_GRAY),
        );
    });
}