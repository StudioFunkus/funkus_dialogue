use bevy_egui::egui::{self, Color32, Ui};

use crate::state::{EditorStatusMessages, StatusLevel};

pub struct StatusBarWidget;

impl StatusBarWidget {
    pub fn show(&mut self, ui: &mut Ui, status: &mut EditorStatusMessages) {
        ui.horizontal(|ui| {
            ui.label("Status");
            if ui.button("Clear").clicked() {
                status.clear();
            }
            ui.separator();
            ui.label(format!("Messages: {}", status.messages.len()));
        });

        ui.separator();

        if status.messages.is_empty() {
            ui.label("No status messages.");
            return;
        }

        let mut dismiss = Vec::new();
        egui::ScrollArea::vertical()
            .max_height(160.0)
            .show(ui, |ui| {
                for (index, message) in status.messages.iter().enumerate().rev() {
                    let color = status_color(message.level);
                    ui.horizontal(|row| {
                        let text = egui::RichText::new(format!(
                            "[{}] {}",
                            status_label(message.level),
                            message.text
                        ))
                        .color(color);
                        row.label(text);
                        if row.small_button("Dismiss").clicked() {
                            dismiss.push(index);
                        }
                    });
                }
            });

        if !dismiss.is_empty() {
            dismiss.sort_unstable();
            dismiss.dedup();
            for index in dismiss.into_iter().rev() {
                status.remove(index);
            }
        }
    }
}

fn status_color(level: StatusLevel) -> Color32 {
    match level {
        StatusLevel::Info => Color32::from_rgb(0x64, 0xB5, 0xF6),
        StatusLevel::Success => Color32::from_rgb(0x43, 0xA0, 0x47),
        StatusLevel::Warning => Color32::from_rgb(0xFB, 0x8C, 0x00),
        StatusLevel::Error => Color32::from_rgb(0xE5, 0x39, 0x35),
    }
}

fn status_label(level: StatusLevel) -> &'static str {
    match level {
        StatusLevel::Info => "Info",
        StatusLevel::Success => "Success",
        StatusLevel::Warning => "Warning",
        StatusLevel::Error => "Error",
    }
}
