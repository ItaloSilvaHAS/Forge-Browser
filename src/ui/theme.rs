use eframe::egui;

pub const BACKGROUND_DARK: egui::Color32 = egui::Color32::from_rgb(10, 10, 10);
pub const PANEL_DARK: egui::Color32 = egui::Color32::from_rgb(20, 20, 20);
pub const ACCENT_PURPLE: egui::Color32 = egui::Color32::from_rgb(160, 80, 255);
pub const TEXT_GRAY: egui::Color32 = egui::Color32::from_rgb(200, 200, 200);

pub fn setup_custom_fonts(_ctx: &egui::Context) {
    // Implementação futura de fontes
}

pub fn apply_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    
    // Estética Opera-like: arredondamento suave e contraste moderado
    style.visuals.window_rounding = 16.0.into();
    style.visuals.widgets.noninteractive.rounding = 12.0.into();
    style.visuals.widgets.inactive.rounding = 12.0.into();
    style.visuals.widgets.hovered.rounding = 12.0.into();
    style.visuals.widgets.active.rounding = 12.0.into();
    
    style.visuals.window_shadow.extrusion = 20.0;
    style.visuals.window_shadow.color = egui::Color32::from_black_alpha(100);
    
    style.visuals.override_text_color = Some(egui::Color32::from_rgb(240, 240, 240));
    style.visuals.extreme_bg_color = egui::Color32::from_rgb(30, 30, 30);
    style.visuals.faint_bg_color = PANEL_DARK;
    
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.window_margin = egui::Margin::same(15.0);
    
    ctx.set_style(style);
}
