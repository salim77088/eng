//! Editor panels - each top-level window of the editor. Designed to be
//! called from inside `egui::Context::run` so they can add themselves
//! via `egui::SidePanel` / `egui::TopBottomPanel` / `egui::Window`.

use crate::state::{EditorMode, EditorState};
use egui::*;

/// The top menu bar - File / Edit / View / Help.
pub fn top_menu(ctx: &Context, state: &mut EditorState) {
    TopBottomPanel::top("top_menu").show(ctx, |ui| {
        menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New Scene").clicked() {
                    state.log("File > New Scene (stub)");
                }
                if ui.button("Open Scene...").clicked() {
                    state.log("File > Open Scene (stub)");
                }
                if ui.button("Save Scene").clicked() {
                    state.log("File > Save Scene (stub)");
                }
                ui.separator();
                if ui.button("Exit").clicked() {
                    state.log("File > Exit (stub)");
                }
            });
            ui.menu_button("Edit", |ui| {
                if ui.button("Undo  (Ctrl+Z)").clicked() {
                    state.log("Edit > Undo (stub)");
                }
                if ui.button("Redo  (Ctrl+Y)").clicked() {
                    state.log("Edit > Redo (stub)");
                }
            });
            ui.menu_button("View", |ui| {
                ui.checkbox(&mut state.show_hierarchy, "Hierarchy");
                ui.checkbox(&mut state.show_inspector, "Inspector");
                ui.checkbox(&mut state.show_asset_browser, "Asset Browser");
                ui.checkbox(&mut state.show_console, "Console");
            });
            ui.menu_button("Help", |ui| {
                if ui.button("About Lumina").clicked() {
                    state.show_about = true;
                }
                if ui.button("Documentation").clicked() {
                    state.log("Help > Documentation: https://github.com/salim77088/eng");
                }
            });

            ui.separator();
            // Play / Pause / Stop controls.
            ui.horizontal(|ui| {
                let play = ui.button("Play").clicked();
                let pause = ui.button("Pause").clicked();
                let stop = ui.button("Stop").clicked();
                if play {
                    state.mode = EditorMode::Play;
                    state.log("Entered Play mode");
                }
                if pause {
                    state.mode = EditorMode::Pause;
                    state.log("Paused");
                }
                if stop {
                    state.mode = EditorMode::Edit;
                    state.log("Returned to Edit mode");
                }
                ui.label(format!("Mode: {:?}", state.mode));
            });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(format!("FPS: {:.0}", state.stats.fps));
                ui.label(format!("Entities: {}", state.stats.entity_count));
                ui.label(format!("Particles: {}", state.stats.particles));
                ui.label(format!("Draws: {}", state.stats.draw_calls));
                ui.separator();
                ui.label(
                    RichText::new("LUMINA")
                        .strong()
                        .color(Color32::from_rgb(0, 220, 230)),
                );
            });
        });
    });
}

/// Left-side scene hierarchy. Lists every entity by name.
/// `names` is a list of (entity_id_as_u64, display_name).
pub fn hierarchy(ctx: &Context, state: &mut EditorState, names: Vec<(u64, String)>) {
    if !state.show_hierarchy {
        return;
    }
    SidePanel::left("hierarchy")
        .resizable(true)
        .default_width(220.0)
        .show(ctx, |ui| {
            ui.heading("Hierarchy");
            ui.separator();
            ScrollArea::vertical().show(ui, |ui| {
                if names.is_empty() {
                    ui.label(
                        RichText::new("No entities yet.\nUse the + button below to spawn one.")
                            .weak(),
                    );
                }
                for (id, name) in &names {
                    let selected = state.selected_entity == Some(*id);
                    if ui
                        .selectable_label(selected, format!("{}  {}", "\u{25C8}", name))
                        .clicked()
                    {
                        state.selected_entity = Some(*id);
                    }
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("+ Add Entity").clicked() {
                    state.log("Add Entity: stub - wire to scene API");
                }
                if ui.button("- Delete").clicked() {
                    state.log("Delete Entity: stub - wire to scene API");
                }
            });
        });
}

/// Right-side inspector. Shows components of the selected entity.
pub fn inspector(ctx: &Context, state: &mut EditorState, selected: Option<InspectorInfo>) {
    if !state.show_inspector {
        return;
    }
    SidePanel::right("inspector")
        .resizable(true)
        .default_width(280.0)
        .show(ctx, |ui| {
            ui.heading("Inspector");
            ui.separator();
            match selected {
                None => {
                    ui.label("No entity selected.");
                }
                Some(info) => {
                    ui.label(format!("Entity: {}", info.name));
                    ui.separator();
                    CollapsingHeader::new("Transform")
                        .default_open(true)
                        .show(ui, |ui| {
                            let mut pos = info.position;
                            ui.horizontal(|ui| {
                                ui.label("Pos");
                                ui.add(DragValue::new(&mut pos[0]).prefix("X: "));
                                ui.add(DragValue::new(&mut pos[1]).prefix("Y: "));
                                ui.add(DragValue::new(&mut pos[2]).prefix("Z: "));
                            });
                            let mut scale = info.scale;
                            ui.horizontal(|ui| {
                                ui.label("Scale");
                                ui.add(DragValue::new(&mut scale[0]));
                                ui.add(DragValue::new(&mut scale[1]));
                                ui.add(DragValue::new(&mut scale[2]));
                            });
                            ui.label("(Editing is read-only in v0.1.)");
                        });
                    CollapsingHeader::new("Mesh / Sprite").show(ui, |ui| {
                        ui.label(format!("Asset: {}", info.asset));
                    });
                    CollapsingHeader::new("LuminaScript").show(ui, |ui| {
                        ui.label(format!("Script: {}", info.script));
                        if ui.button("Reload").clicked() {
                            state.log("Script reload requested.");
                        }
                    });
                }
            }
        });
}

/// Bottom-left asset browser. Lists files under the assets root.
pub fn asset_browser(ctx: &Context, state: &mut EditorState, files: Vec<String>) {
    if !state.show_asset_browser {
        return;
    }
    TopBottomPanel::bottom("asset_browser")
        .resizable(true)
        .default_height(160.0)
        .show(ctx, |ui| {
            ui.heading("Asset Browser");
            ui.separator();
            ScrollArea::horizontal().show(ui, |ui| {
                if files.is_empty() {
                    ui.label("No assets found. Drop files into the project's assets/ folder.");
                }
                for f in files {
                    let _ = ui.selectable_label(false, &f);
                }
            });
        });
}

/// Bottom console - shows log messages, supports clearing.
pub fn console(ctx: &Context, state: &mut EditorState) {
    if !state.show_console {
        return;
    }
    TopBottomPanel::bottom("console")
        .resizable(true)
        .default_height(140.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Console");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Clear").clicked() {
                        state.console_log.clear();
                    }
                });
            });
            ui.separator();
            ScrollArea::vertical().show(ui, |ui| {
                for line in &state.console_log {
                    ui.label(RichText::new(line).weak());
                }
            });
        });
}

/// Central viewport - the rendered game view is drawn into a texture and
/// shown here as an image. For v0.1 we show a placeholder.
pub fn viewport(ctx: &Context, _state: &mut EditorState, viewport_texture: Option<TextureId>) {
    CentralPanel::default().show(ctx, |ui| {
        ui.spacing_mut().item_spacing.y = 6.0;
        match viewport_texture {
            Some(tex) => {
                let size = ui.available_size();
                ui.image((tex, size));
            }
            None => {
                let (rect, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
                ui.painter()
                    .rect_filled(rect, 0.0, Color32::from_rgb(0x10, 0x12, 0x16));
                ui.painter().text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    "Lumina Viewport\n(rendering in headless mode)",
                    FontId::proportional(18.0),
                    Color32::from_rgb(0x55, 0x60, 0x66),
                );
            }
        }
    });
}

/// About dialog.
pub fn about(ctx: &Context, state: &mut EditorState) {
    if !state.show_about {
        return;
    }
    Window::new("About Lumina Engine")
        .open(&mut state.show_about)
        .resizable(false)
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(8.0);
                ui.heading(
                    RichText::new("LUMINA ENGINE")
                        .strong()
                        .color(Color32::from_rgb(0, 220, 230)),
                );
                ui.label(format!("Version {}", lumina_core::VERSION));
                ui.add_space(8.0);
                ui.label("A lightweight 2D/3D game engine with an integrated");
                ui.label("editor and the easy LuminaScript language.");
                ui.add_space(8.0);
                ui.hyperlink("https://github.com/salim77088/eng");
                ui.add_space(8.0);
                ui.label("Licensed under MIT OR Apache-2.0.");
            });
        });
}

/// Aggregated info about the selected entity, computed by the engine
/// before calling `inspector()`.
#[derive(Clone)]
pub struct InspectorInfo {
    pub name: String,
    pub position: [f32; 3],
    pub scale: [f32; 3],
    pub asset: String,
    pub script: String,
}
