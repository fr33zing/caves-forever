use egui::{vec2, Color32, Rect, Rounding, Sense, Shape, Stroke, Ui};

pub fn lock(ui: &mut Ui, color: Color32) {
    let h = ui.available_height() - 8.0;
    let w = h * 0.65;

    let size = egui::vec2(w, h);
    let (response, painter) = ui.allocate_painter(size, Sense::hover());
    let rect = response.rect;

    let lock_body = Rect {
        min: rect.left_top() + vec2(0.0, h / 2.0),
        max: rect.right_bottom(),
    };

    painter.add(Shape::rect_filled(
        lock_body,
        Rounding::same(rect.height() / 6.0),
        color,
    ));

    let stroke_width = h / 8.0;
    painter.add(Shape::ellipse_stroke(
        lock_body.center_top(),
        vec2(w / 2.0 - stroke_width * 1.35, h / 2.0 - stroke_width),
        Stroke::new(stroke_width, color),
    ));
}
