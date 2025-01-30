use egui::{vec2, Color32, Sense, Shape, Ui};

pub fn changed_default(ui: &mut Ui) {
    changed(ui, 8.0, 8.0, Color32::from_gray(140));
}

pub fn changed(ui: &mut Ui, w: f32, h: f32, color: Color32) {
    let size = vec2(w, h);
    let (response, painter) = ui.allocate_painter(size, Sense::hover());
    let rect = response.rect;

    painter.add(Shape::circle_filled(
        rect.center(),
        size.min_elem() * 0.4,
        color,
    ));
}
