use avian3d::prelude::{FillMode, VhacdParameters};
use egui::{Checkbox, CollapsingHeader, Label, Slider, Ui};

pub fn vhacd_parameters_sidebar(ui: &mut Ui, vhacd_parameters: &mut VhacdParameters) -> bool {
    let mut vhacd = vhacd_parameters.clone();
    let mut changed = false;

    CollapsingHeader::new("V-HACD Parameters").show(ui, |ui| {
        ui.style_mut().spacing.slider_width = ui.available_width() - 48.0;

        ui.add(Label::new("Concavity"));
        changed |= ui
            .add(Slider::new(&mut vhacd.concavity, 0.0..=1.0))
            .changed();
        ui.add_space(8.0);

        ui.add(Label::new("Alpha"));
        changed |= ui.add(Slider::new(&mut vhacd.alpha, 0.0..=1.0)).changed();
        ui.add_space(8.0);

        ui.add(Label::new("Beta"));
        changed |= ui.add(Slider::new(&mut vhacd.beta, 0.0..=1.0)).changed();
        ui.add_space(8.0);

        ui.add(Label::new("Resolution"));
        changed |= ui
            .add(Slider::new(&mut vhacd.resolution, 1..=128))
            .changed();
        ui.add_space(8.0);

        ui.add(Label::new("Convex hull downsampling"));
        changed |= ui
            .add(Slider::new(&mut vhacd.convex_hull_downsampling, 1..=16))
            .changed();
        ui.add_space(8.0);

        ui.add(Label::new("Max convex hulls"));
        changed |= ui
            .add(Slider::new(&mut vhacd.max_convex_hulls, 64..=2048))
            .changed();
        ui.add_space(8.0);

        'cavities: {
            let checked = match vhacd.fill_mode {
                FillMode::SurfaceOnly => break 'cavities,
                FillMode::FloodFill {
                    ref mut detect_cavities,
                } => detect_cavities,
            };
            changed |= ui.add(Checkbox::new(checked, "Detect cavities")).changed();
        }
    });

    if changed {
        *vhacd_parameters = vhacd;
    }

    changed
}
