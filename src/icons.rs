use egui::{Color32, Pos2, Rect, Stroke, Ui, Vec2};
use std::f32::consts::PI;

pub use egui_icons::icons::*;

/// Helper para formatear texto con icono a la izquierda
pub fn with_icon(icon: egui_icons::MaterialIcon, text: &str) -> String {
    format!("{} {}", icon.codepoint, text)
}


/// Dibuja un icono vectorial de satélite con cuerpo central, paneles solares y ondas RF
pub fn draw_satellite_icon(ui: &mut Ui, rect: Rect, color: Color32) {
    let painter = ui.painter();
    let center = rect.center();
    let size = rect.size().min_elem();
    let stroke = Stroke::new(1.5_f32, color);

    // Cuerpo central del satélite (rectángulo o rombo inclinado)
    let body_w = size * 0.28;
    let body_h = size * 0.28;
    let body_rect = Rect::from_center_size(center, Vec2::new(body_w, body_h));
    painter.rect_stroke(body_rect, 2.0, stroke, egui::StrokeKind::Inside);

    // Panel solar izquierdo
    let panel_w = size * 0.25;
    let panel_h = size * 0.20;
    let left_panel_rect = Rect::from_center_size(
        Pos2::new(center.x - body_w * 0.5 - panel_w * 0.5 - 2.0, center.y),
        Vec2::new(panel_w, panel_h),
    );
    painter.rect_stroke(left_panel_rect, 1.0, stroke, egui::StrokeKind::Inside);
    // Cuadrícula panel izquierdo
    painter.line_segment(
        [
            Pos2::new(left_panel_rect.center().x, left_panel_rect.top()),
            Pos2::new(left_panel_rect.center().x, left_panel_rect.bottom()),
        ],
        Stroke::new(1.0_f32, color),
    );

    // Panel solar derecho
    let right_panel_rect = Rect::from_center_size(
        Pos2::new(center.x + body_w * 0.5 + panel_w * 0.5 + 2.0, center.y),
        Vec2::new(panel_w, panel_h),
    );
    painter.rect_stroke(right_panel_rect, 1.0, stroke, egui::StrokeKind::Inside);
    // Cuadrícula panel derecho
    painter.line_segment(
        [
            Pos2::new(right_panel_rect.center().x, right_panel_rect.top()),
            Pos2::new(right_panel_rect.center().x, right_panel_rect.bottom()),
        ],
        Stroke::new(1.0_f32, color),
    );

    // Antena y arco de onda RF inferior
    let antenna_bottom = Pos2::new(center.x, center.y + body_h * 0.5 + 4.0);
    painter.line_segment(
        [Pos2::new(center.x, center.y + body_h * 0.5), antenna_bottom],
        stroke,
    );

    // Arco de onda RF
    let wave_radius = size * 0.18;
    let mut wave_points = Vec::new();
    for angle_deg in 40..=140 {
        let rad = (angle_deg as f32) * PI / 180.0;
        let x = antenna_bottom.x + wave_radius * rad.cos();
        let y = antenna_bottom.y + wave_radius * rad.sin() * 0.5;
        wave_points.push(Pos2::new(x, y));
    }
    for w in wave_points.windows(2) {
        painter.line_segment([w[0], w[1]], Stroke::new(1.2_f32, color));
    }
}

/// Dibuja un icono vectorial de engranaje mecánico (ajustes) con dientes y orificio central
pub fn draw_settings_icon(ui: &mut Ui, rect: Rect, color: Color32) {
    let painter = ui.painter();
    let center = rect.center();
    let radius = rect.size().min_elem() * 0.38;
    let inner_radius = radius * 0.72;
    let hole_radius = radius * 0.35;
    let stroke = Stroke::new(1.5_f32, color);

    // Círculo del orificio interior
    painter.circle_stroke(center, hole_radius, stroke);

    // Dientes de engranaje (8 dientes)
    let num_teeth = 8;
    let mut points = Vec::new();

    for i in 0..num_teeth {
        let angle0 = (i as f32) * 2.0 * PI / (num_teeth as f32);
        let angle1 = angle0 + 0.15;
        let angle2 = angle0 + 0.35;
        let angle3 = angle0 + 0.50;

        points.push(Pos2::new(center.x + inner_radius * angle0.cos(), center.y + inner_radius * angle0.sin()));
        points.push(Pos2::new(center.x + radius * angle1.cos(), center.y + radius * angle1.sin()));
        points.push(Pos2::new(center.x + radius * angle2.cos(), center.y + radius * angle2.sin()));
        points.push(Pos2::new(center.x + inner_radius * angle3.cos(), center.y + inner_radius * angle3.sin()));
    }

    if !points.is_empty() {
        for i in 0..points.len() {
            let next = (i + 1) % points.len();
            painter.line_segment([points[i], points[next]], stroke);
        }
    }
}

/// Dibuja un icono de flecha hacia la izquierda (retorno)
pub fn draw_back_arrow(ui: &mut Ui, rect: Rect, color: Color32) {
    let painter = ui.painter();
    let center = rect.center();
    let size = rect.size().min_elem() * 0.5;
    let stroke = Stroke::new(2.0_f32, color);

    let start = Pos2::new(center.x + size * 0.5, center.y);
    let end = Pos2::new(center.x - size * 0.5, center.y);

    painter.line_segment([start, end], stroke);
    painter.line_segment([end, Pos2::new(end.x + size * 0.4, end.y - size * 0.4)], stroke);
    painter.line_segment([end, Pos2::new(end.x + size * 0.4, end.y + size * 0.4)], stroke);
}

/// Dibuja un icono de reproducción (triángulo)
pub fn draw_play_icon(ui: &mut Ui, rect: Rect, color: Color32) {
    let painter = ui.painter();
    let center = rect.center();
    let r = rect.size().min_elem() * 0.4;

    let p0 = Pos2::new(center.x - r * 0.6, center.y - r * 0.8);
    let p1 = Pos2::new(center.x - r * 0.6, center.y + r * 0.8);
    let p2 = Pos2::new(center.x + r * 0.8, center.y);

    painter.line_segment([p0, p1], Stroke::new(1.8_f32, color));
    painter.line_segment([p1, p2], Stroke::new(1.8_f32, color));
    painter.line_segment([p2, p0], Stroke::new(1.8_f32, color));
}

/// Dibuja un icono de parada (cuadrado)
pub fn draw_stop_icon(ui: &mut Ui, rect: Rect, color: Color32) {
    let painter = ui.painter();
    let center = rect.center();
    let size = rect.size().min_elem() * 0.6;
    let r = Rect::from_center_size(center, Vec2::new(size, size));
    painter.rect_stroke(r, 1.0, Stroke::new(1.8_f32, color), egui::StrokeKind::Inside);
}

/// Dibuja un icono de carpeta
pub fn draw_folder_icon(ui: &mut Ui, rect: Rect, color: Color32) {
    let painter = ui.painter();
    let center = rect.center();
    let w = rect.size().x * 0.7;
    let h = rect.size().y * 0.55;
    let stroke = Stroke::new(1.5_f32, color);

    let base = Rect::from_min_size(
        Pos2::new(center.x - w * 0.5, center.y - h * 0.25),
        Vec2::new(w, h),
    );
    painter.rect_stroke(base, 2.0, stroke, egui::StrokeKind::Inside);

    // Lengüeta de la carpeta
    let tab = Rect::from_min_size(
        Pos2::new(center.x - w * 0.5, center.y - h * 0.55),
        Vec2::new(w * 0.45, h * 0.35),
    );
    painter.rect_stroke(tab, 1.0, stroke, egui::StrokeKind::Inside);
}

/// Dibuja un icono vectorial de rayo (energía / nativo)
pub fn draw_bolt_icon(ui: &mut Ui, rect: Rect, color: Color32) {
    let painter = ui.painter();
    let center = rect.center();
    let s = rect.size().min_elem() * 0.5;

    let pts = [
        Pos2::new(center.x + s * 0.1, center.y - s * 0.9),
        Pos2::new(center.x - s * 0.6, center.y + s * 0.05),
        Pos2::new(center.x - s * 0.05, center.y + s * 0.05),
        Pos2::new(center.x - s * 0.3, center.y + s * 0.95),
        Pos2::new(center.x + s * 0.6, center.y - s * 0.1),
        Pos2::new(center.x + s * 0.05, center.y - s * 0.1),
    ];

    let stroke = Stroke::new(1.4_f32, color);
    for i in 0..pts.len() {
        let next = (i + 1) % pts.len();
        painter.line_segment([pts[i], pts[next]], stroke);
    }
}

/// Dibuja un icono vectorial de radio transceptor / antena
pub fn draw_radio_icon(ui: &mut Ui, rect: Rect, color: Color32) {
    let painter = ui.painter();
    let center = rect.center();
    let s = rect.size().min_elem() * 0.5;
    let stroke = Stroke::new(1.5_f32, color);

    // Caja base del radio
    let body = Rect::from_center_size(
        Pos2::new(center.x, center.y + s * 0.25),
        Vec2::new(s * 1.5, s * 0.9),
    );
    painter.rect_stroke(body, 2.0, stroke, egui::StrokeKind::Inside);

    // Dial / altavoz circular en el cuerpo
    painter.circle_stroke(Pos2::new(center.x - s * 0.35, center.y + s * 0.25), s * 0.22, Stroke::new(1.0_f32, color));

    // Antena vertical telescópica
    let ant_base = Pos2::new(center.x + s * 0.45, center.y - s * 0.2);
    let ant_tip = Pos2::new(center.x + s * 0.45, center.y - s * 0.85);
    painter.line_segment([ant_base, ant_tip], Stroke::new(1.4_f32, color));

    // Onda RF emitida desde la punta de la antena
    let r1 = s * 0.25;
    let r2 = s * 0.45;
    painter.line_segment(
        [Pos2::new(ant_tip.x - r1, ant_tip.y - r1 * 0.5), Pos2::new(ant_tip.x + r1, ant_tip.y - r1 * 0.5)],
        Stroke::new(1.0_f32, color),
    );
    painter.line_segment(
        [Pos2::new(ant_tip.x - r2, ant_tip.y - r2 * 0.5), Pos2::new(ant_tip.x + r2, ant_tip.y - r2 * 0.5)],
        Stroke::new(1.0_f32, color),
    );
}

/// Dibuja un icono vectorial de refresco / sincronización (flecha circular)
pub fn draw_refresh_icon(ui: &mut Ui, rect: Rect, color: Color32) {
    let painter = ui.painter();
    let center = rect.center();
    let radius = rect.size().min_elem() * 0.36;
    let stroke = Stroke::new(1.6_f32, color);

    // Arco de ~270 grados
    let mut points = Vec::new();
    for deg in 40..=310 {
        let rad = (deg as f32) * PI / 180.0;
        points.push(Pos2::new(center.x + radius * rad.cos(), center.y + radius * rad.sin()));
    }
    for w in points.windows(2) {
        painter.line_segment([w[0], w[1]], stroke);
    }

    // Punta de flecha al final del arco
    if let Some(&tip) = points.last() {
        let arrow_len = radius * 0.4;
        painter.line_segment([tip, Pos2::new(tip.x - arrow_len, tip.y - arrow_len * 0.4)], stroke);
        painter.line_segment([tip, Pos2::new(tip.x - arrow_len * 0.3, tip.y + arrow_len * 0.8)], stroke);
    }
}

/// Dibuja un icono vectorial de copiar (dos páginas superpuestas)
pub fn draw_copy_icon(ui: &mut Ui, rect: Rect, color: Color32) {
    let painter = ui.painter();
    let center = rect.center();
    let s = rect.size().min_elem() * 0.38;
    let stroke = Stroke::new(1.3_f32, color);

    // Rectángulo posterior
    let back = Rect::from_min_size(
        Pos2::new(center.x - s * 0.7, center.y - s * 0.8),
        Vec2::new(s * 1.1, s * 1.2),
    );
    painter.rect_stroke(back, 1.0, stroke, egui::StrokeKind::Inside);

    // Rectángulo frontal
    let front = Rect::from_min_size(
        Pos2::new(center.x - s * 0.3, center.y - s * 0.3),
        Vec2::new(s * 1.1, s * 1.2),
    );
    // Relleno sutil para tapar el de atrás
    painter.rect_filled(front, 1.0, ui.visuals().window_fill);
    painter.rect_stroke(front, 1.0, stroke, egui::StrokeKind::Inside);
}

/// Dibuja un icono vectorial de objetivo / diana (selección de hardware)
pub fn draw_target_icon(ui: &mut Ui, rect: Rect, color: Color32) {
    let painter = ui.painter();
    let center = rect.center();
    let r = rect.size().min_elem() * 0.4;
    let stroke = Stroke::new(1.4_f32, color);

    painter.circle_stroke(center, r, stroke);
    painter.circle_filled(center, r * 0.3, color);

    // Marcas de retícula
    let tick_in = r * 0.7;
    let tick_out = r * 1.15;
    painter.line_segment([Pos2::new(center.x - tick_out, center.y), Pos2::new(center.x - tick_in, center.y)], stroke);
    painter.line_segment([Pos2::new(center.x + tick_in, center.y), Pos2::new(center.x + tick_out, center.y)], stroke);
    painter.line_segment([Pos2::new(center.x, center.y - tick_out), Pos2::new(center.x, center.y - tick_in)], stroke);
    painter.line_segment([Pos2::new(center.x, center.y + tick_in), Pos2::new(center.x, center.y + tick_out)], stroke);
}

/// Dibuja un icono vectorial de marca de verificación (✓)
pub fn draw_check_icon(ui: &mut Ui, rect: Rect, color: Color32) {
    let painter = ui.painter();
    let center = rect.center();
    let s = rect.size().min_elem() * 0.4;
    let stroke = Stroke::new(2.0_f32, color);

    let p0 = Pos2::new(center.x - s * 0.8, center.y);
    let p1 = Pos2::new(center.x - s * 0.2, center.y + s * 0.6);
    let p2 = Pos2::new(center.x + s * 0.8, center.y - s * 0.6);

    painter.line_segment([p0, p1], stroke);
    painter.line_segment([p1, p2], stroke);
}

/// Dibuja un icono vectorial de advertencia (triángulo con signo de exclamación)
pub fn draw_warning_icon(ui: &mut Ui, rect: Rect, color: Color32) {
    let painter = ui.painter();
    let center = rect.center();
    let s = rect.size().min_elem() * 0.45;
    let stroke = Stroke::new(1.6_f32, color);

    let top = Pos2::new(center.x, center.y - s * 0.9);
    let bl = Pos2::new(center.x - s, center.y + s * 0.8);
    let br = Pos2::new(center.x + s, center.y + s * 0.8);

    painter.line_segment([top, bl], stroke);
    painter.line_segment([bl, br], stroke);
    painter.line_segment([br, top], stroke);

    // Barra de exclamación
    painter.line_segment(
        [Pos2::new(center.x, center.y - s * 0.3), Pos2::new(center.x, center.y + s * 0.2)],
        Stroke::new(1.8_f32, color),
    );
    // Punto de exclamación
    painter.circle_filled(Pos2::new(center.x, center.y + s * 0.5), 1.2, color);
}

/// Dibuja un icono vectorial de error (círculo con cruz)
pub fn draw_error_icon(ui: &mut Ui, rect: Rect, color: Color32) {
    let painter = ui.painter();
    let center = rect.center();
    let r = rect.size().min_elem() * 0.42;
    let stroke = Stroke::new(1.6_f32, color);

    painter.circle_stroke(center, r, stroke);

    let cross_s = r * 0.5;
    painter.line_segment(
        [Pos2::new(center.x - cross_s, center.y - cross_s), Pos2::new(center.x + cross_s, center.y + cross_s)],
        stroke,
    );
    painter.line_segment(
        [Pos2::new(center.x + cross_s, center.y - cross_s), Pos2::new(center.x - cross_s, center.y + cross_s)],
        stroke,
    );
}

/// Dibuja un icono vectorial de información (círculo con 'i')
pub fn draw_info_icon(ui: &mut Ui, rect: Rect, color: Color32) {
    let painter = ui.painter();
    let center = rect.center();
    let r = rect.size().min_elem() * 0.42;
    let stroke = Stroke::new(1.5_f32, color);

    painter.circle_stroke(center, r, stroke);

    // Punto superior de la 'i'
    painter.circle_filled(Pos2::new(center.x, center.y - r * 0.4), 1.3, color);
    // Barra de la 'i'
    painter.line_segment(
        [Pos2::new(center.x, center.y - r * 0.1), Pos2::new(center.x, center.y + r * 0.5)],
        Stroke::new(1.6_f32, color),
    );
}

