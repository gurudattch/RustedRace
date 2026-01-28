use eframe::egui;
use std::time::{Duration, Instant};

pub struct LoadingScreen {
    start_time: Instant,
    logo_texture: Option<egui::TextureHandle>,
}

impl LoadingScreen {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            logo_texture: None,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        let elapsed = self.start_time.elapsed();
        let total_duration = Duration::from_millis(1200);

        // Force smooth animation
        ctx.request_repaint();

        // Load logo once
        if self.logo_texture.is_none() {
            let image_bytes = include_bytes!("rustedrace.png");

            if let Ok(img) = image::load_from_memory(image_bytes) {
                let rgba = img.to_rgba8();

                let size = [
                    rgba.width() as usize,
                    rgba.height() as usize,
                ];

                let color_image =
                    egui::ColorImage::from_rgba_unmultiplied(size, &rgba);

                self.logo_texture = Some(ctx.load_texture(
                    "rustedrace_logo",
                    color_image,
                    Default::default(),
                ));
            }
        }

        // Progress from 0.0 → 1.0 in 1.2s
        let progress = (elapsed.as_secs_f32()
            / total_duration.as_secs_f32())
            .clamp(0.0, 1.0);

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::TRANSPARENT))
            .show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {

                    // Logo
                    if let Some(texture) = &self.logo_texture {
                        let size = egui::Vec2::new(160.0, 160.0);

                        ui.add(
                            egui::Image::new(texture)
                                .fit_to_exact_size(size),
                        );
                    }

                    ui.add_space(20.0);

                    // Progress bar (maroon)
                    let progress_bar = egui::ProgressBar::new(progress)
                        .desired_width(260.0)
                        .fill(egui::Color32::from_rgb(128, 0, 0))
                        .show_percentage();

                    ui.add(progress_bar);
                });
            });
        });

        // Show only for 1.2s
        elapsed < total_duration
    }
}
