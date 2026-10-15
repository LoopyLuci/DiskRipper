mod ui;

use eframe::NativeOptions;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "DiskRipper",
        options,
        Box::new(|cc| Ok(Box::new(ui::DiskRipperApp::new(cc)))),
    )
}
