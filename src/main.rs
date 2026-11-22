// QKZ TC MAX EQ — live register edit only, no save to flash.
mod device;
mod presets;

use device::{
    Band, Device, BANDS, EQ_OFF, EQ_ON, REG_DAC_EQ_EN, REG_DAC_GAIN, TYPE_HIGH_SHELF, TYPE_LOW_SHELF, TYPE_PEAK,
};
use eframe::egui::{self, Color32, FontId, RichText, Rounding, Stroke, Vec2};
use presets::Preset;

const BG: Color32 = Color32::from_rgb(18, 19, 24);
const CARD: Color32 = Color32::from_rgb(28, 30, 38);
const CARD_HI: Color32 = Color32::from_rgb(36, 39, 50);
const ACCENT: Color32 = Color32::from_rgb(94, 156, 255);
const PENDING: Color32 = Color32::from_rgb(255, 184, 77);
const OK: Color32 = Color32::from_rgb(92, 214, 140);
const ERR: Color32 = Color32::from_rgb(255, 99, 99);
const GAIN_DEFAULT: u32 = 0x200;
const DIM: Color32 = Color32::from_rgb(130, 136, 152);

fn main() -> eframe::Result<()> {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([720.0, 620.0])
            .with_min_inner_size([640.0, 560.0])
            .with_icon(eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png")).unwrap_or_default())
            .with_decorations(false)
            .with_resizable(true),
        ..Default::default()
    };
    eframe::run_native(
        "QKZ TC MAX EQ",
        opts,
        Box::new(|cc| {
            setup_style(&cc.egui_ctx);
            round_corners(cc);
            Ok(Box::new(App::new()))
        }),
    )
}

fn setup_style(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = BG;
    v.window_fill = BG;
    v.extreme_bg_color = Color32::from_rgb(14, 15, 19);
    v.selection.bg_fill = ACCENT;
    v.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);
    let r = Rounding::same(6.0);
    for w in [
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.rounding = r;
    }
    v.widgets.noninteractive.rounding = r;
    v.widgets.inactive.weak_bg_fill = CARD_HI;
    v.widgets.inactive.bg_fill = Color32::from_rgb(48, 52, 66);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(48, 52, 66);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    v.widgets.active.weak_bg_fill = ACCENT;
    v.widgets.active.bg_fill = ACCENT;
    ctx.set_visuals(v);

    ctx.style_mut(|s| {
        s.spacing.item_spacing = Vec2::new(6.0, 6.0);
        s.spacing.button_padding = Vec2::new(10.0, 4.0);
        s.spacing.slider_width = 200.0;
        s.spacing.interact_size.y = 22.0;
        s.text_styles.insert(egui::TextStyle::Body, FontId::proportional(13.0));
        s.text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(13.0));
    });
}

fn card(ui: &mut egui::Ui, fill: Color32, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::none()
        .fill(fill)
        .rounding(Rounding::same(10.0))
        .inner_margin(egui::Margin::same(10.0))
        .show(ui, add);
}

struct App {
    dev: Option<Device>,
    bands: [Band; BANDS],
    loaded: [Band; BANDS],
    eq_en: u32,
    gain: u32,
    gain_loaded: u32,
    status: String,
    status_ok: bool,
    presets: Vec<Preset>,
    preset_sel: Option<usize>,
    preset_name: String,
    auto_apply: bool,
    dev_info: String,
    last_poll: std::time::Instant,
    logo: Option<egui::TextureHandle>,
    saving: Option<PendingSave>,
    confirm_save: bool,
}

// What a SAVE sent, checked against a read once the dongle is back on USB.
struct PendingSave {
    started: std::time::Instant,
    bands: [Band; BANDS],
    eq_en: u32,
}

impl App {
    fn new() -> Self {
        let flat = Band {
            freq: 1000,
            gain: 0.0,
            q: 0.707,
            kind: 0,
        };
        let mut app = App {
            dev: None,
            bands: [flat; BANDS],
            loaded: [flat; BANDS],
            eq_en: 0,
            gain: 0x200,
            gain_loaded: 0x200,
            status: String::new(),
            status_ok: true,
            presets: presets::builtins().into_iter().chain(presets::load_user()).collect(),
            preset_sel: None,
            preset_name: String::new(),
            auto_apply: false,
            dev_info: String::new(),
            last_poll: std::time::Instant::now(),
            logo: None,
            saving: None,
            confirm_save: false,
        };
        app.connect();
        app
    }

    fn set_status(&mut self, ok: bool, msg: impl Into<String>) {
        self.status_ok = ok;
        self.status = msg.into();
    }

    fn connect(&mut self) {
        match Device::open() {
            Ok(d) => {
                let fw = d.read_str(0x04, 3).unwrap_or_default();
                self.dev_info = format!(
                    "QKZ TC MAX · KT Micro · firmware {}",
                    if fw.is_empty() { "?" } else { &fw }
                );
                self.dev = Some(d);
                self.read_all();
                // Right after a SAVE the dongle can enumerate before it answers reads; try again next poll.
                if !self.status_ok {
                    self.dev = None;
                }
            }
            Err(_) => self.dev = None,
        }
    }

    // Reconnect while unplugged, and notice an unplug while connected, without user action.
    fn poll_connection(&mut self, ctx: &egui::Context) {
        let every = if self.dev.is_some() { 2.0 } else { 1.0 };
        ctx.request_repaint_after(std::time::Duration::from_secs_f32(every));
        if self.last_poll.elapsed().as_secs_f32() < every {
            return;
        }
        self.last_poll = std::time::Instant::now();
        match &self.dev {
            None => {
                self.connect();
                if self.dev.is_some() {
                    match self.saving.take() {
                        Some(s) => self.finish_save(s),
                        None => self.set_status(true, "DAC terhubung"),
                    }
                } else if self.saving.as_ref().is_some_and(|s| s.started.elapsed().as_secs() > 12) {
                    self.saving = None;
                    self.set_status(false, "DAC belum kembali setelah menyimpan. Cabut lalu colokkan lagi.");
                }
            }
            Some(d) => {
                if d.read(REG_DAC_EQ_EN).is_err() {
                    self.dev = None;
                    self.set_status(false, "DAC terputus");
                }
            }
        }
    }

    // Apply pending edits, then SAVE. The dongle drops off USB right after; poll_connection picks it up again
    // and finish_save compares a fresh read with what was sent.
    fn save_to_dac(&mut self) {
        if self.pending() > 0 {
            self.apply();
            if !self.status_ok {
                return;
            }
        }
        let Some(d) = &self.dev else { return };
        let expected = PendingSave {
            started: std::time::Instant::now(),
            bands: self.loaded,
            eq_en: self.eq_en & 0xFF,
        };
        match d.save() {
            Ok(()) => {
                self.dev = None;
                self.saving = Some(expected);
                self.last_poll = std::time::Instant::now();
                self.set_status(true, "Menyimpan ke DAC…");
            }
            Err(e) => self.set_status(false, format!("Gagal menyimpan: {e}")),
        }
    }

    fn finish_save(&mut self, s: PendingSave) {
        let same = (0..BANDS).all(|i| self.loaded[i].to_regs() == s.bands[i].to_regs()) && self.eq_en & 0xFF == s.eq_en;
        if same {
            self.set_status(true, "Tersimpan di DAC — setelan ikut ke perangkat lain");
        } else {
            self.set_status(false, "DAC kembali, tapi setelannya tidak sama dengan yang disimpan");
        }
    }

    fn save_confirm_ui(&mut self, ctx: &egui::Context) {
        if !self.confirm_save {
            return;
        }
        let mut go = false;
        let mut close = false;
        egui::Window::new("Simpan ke DAC")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(CARD)
                    .rounding(Rounding::same(10.0))
                    .inner_margin(egui::Margin::same(16.0)),
            )
            .show(ctx, |ui| {
                ui.set_max_width(320.0);
                ui.label("Setelan EQ saat ini akan disimpan di dongle, sehingga tetap terpakai saat dicolok ke HP atau PC lain.");
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Dongle akan terputus sekitar 1–3 detik dan suara berhenti sebentar.")
                        .size(12.0)
                        .color(DIM),
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(egui::Button::new(RichText::new("Simpan").color(Color32::WHITE)).fill(ACCENT))
                        .clicked()
                    {
                        go = true;
                    }
                    if ui.button("Batal").clicked() {
                        close = true;
                    }
                });
            });
        if go {
            self.confirm_save = false;
            self.save_to_dac();
        } else if close || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.confirm_save = false;
        }
    }

    fn disconnected_ui(&mut self, ui: &mut egui::Ui) {
        let logo = self
            .logo
            .get_or_insert_with(|| {
                let icon = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png")).unwrap_or_default();
                let img =
                    egui::ColorImage::from_rgba_unmultiplied([icon.width as usize, icon.height as usize], &icon.rgba);
                ui.ctx().load_texture("logo", img, egui::TextureOptions::LINEAR)
            })
            .clone();
        let h = 64.0 + 16.0 + 22.0 + 6.0 + 36.0 + 22.0 + 30.0;
        ui.add_space(((ui.available_height() - h) / 2.0).max(16.0));
        ui.vertical_centered(|ui| {
            ui.add(
                egui::Image::new(&logo)
                    .fit_to_exact_size(Vec2::splat(64.0))
                    .tint(Color32::from_gray(150)),
            );
            ui.add_space(16.0);
            let (title, l1, l2) = if self.saving.is_some() {
                (
                    "Menyimpan ke DAC…",
                    "Dongle sedang menyimpan dan tersambung ulang.",
                    "Jangan cabut dongle.",
                )
            } else {
                (
                    "DAC tidak terhubung",
                    "Colokkan QKZ TC MAX ke port USB.",
                    "Aplikasi akan tersambung otomatis.",
                )
            };
            ui.label(RichText::new(title).size(18.0).strong().color(Color32::WHITE));
            ui.add_space(2.0);
            ui.label(RichText::new(l1).color(DIM));
            ui.label(RichText::new(l2).color(DIM));
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                let w = 16.0 + 6.0 + 110.0;
                ui.add_space((ui.available_width() - w) / 2.0);
                ui.add(egui::Spinner::new().size(14.0).color(DIM));
                ui.label(RichText::new("Mencari perangkat…").size(12.0).color(DIM));
            });
            ui.add_space(10.0);
            if ui.button("Coba sekarang").clicked() {
                self.last_poll = std::time::Instant::now() - std::time::Duration::from_secs(5);
            }
        });
    }

    fn read_all(&mut self) {
        let Some(d) = &self.dev else { return };
        let r = (|| -> Result<_, String> { Ok((d.read_bands()?, d.read(REG_DAC_EQ_EN)?, d.read(REG_DAC_GAIN)?)) })();
        match r {
            Ok((b, en, g)) => {
                self.bands = b;
                self.loaded = b;
                self.eq_en = en;
                self.gain = g;
                self.gain_loaded = g;
                self.set_status(true, "Read OK");
            }
            Err(e) => self.set_status(false, format!("Read gagal: {e}")),
        }
    }

    fn apply(&mut self) {
        let Some(d) = &self.dev else { return };
        let mut n = 0;
        let r = (|| -> Result<(), String> {
            for i in 0..BANDS {
                if self.bands[i] != self.loaded[i] {
                    d.write_band(i, &self.bands[i])?;
                    self.loaded[i] = self.bands[i];
                    n += 1;
                }
            }
            if self.gain != self.gain_loaded {
                d.write(REG_DAC_GAIN, self.gain)?;
                self.gain_loaded = self.gain;
                n += 1;
            }
            Ok(())
        })();
        match r {
            Ok(()) => self.set_status(true, format!("Apply OK — {n} perubahan (tidak disimpan ke flash)")),
            Err(e) => self.set_status(false, format!("Apply gagal: {e}")),
        }
    }

    fn set_eq_enable(&mut self, en: u32) {
        let Some(d) = &self.dev else { return };
        match d.write(REG_DAC_EQ_EN, en) {
            Ok(()) => {
                self.eq_en = en;
                self.set_status(
                    true,
                    format!("DAC EQ {}", if en == EQ_ON { "aktif" } else { "nonaktif" }),
                );
            }
            Err(e) => self.set_status(false, format!("Gagal: {e}")),
        }
    }

    fn pending(&self) -> usize {
        let b = (0..BANDS).filter(|&i| self.bands[i] != self.loaded[i]).count();
        b + (self.gain != self.gain_loaded) as usize
    }

    // Custom title bar replacing the native window decorations: drag to move, double-click to maximize.
    fn title_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("titlebar")
            .exact_height(36.0)
            .show_separator_line(false)
            .frame(egui::Frame::none().fill(BG).inner_margin(egui::Margin {
                left: 12.0,
                ..Default::default()
            }))
            .show(ctx, |ui| {
                let bar = ui.max_rect();
                let drag = ui.interact(bar, ui.id().with("drag"), egui::Sense::click_and_drag());
                if drag.double_clicked() {
                    let max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
                } else if drag.drag_started_by(egui::PointerButton::Primary) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }

                ui.horizontal_centered(|ui| {
                    ui.label(RichText::new("QKZ TC MAX").size(14.0).strong().color(Color32::WHITE));
                    ui.label(RichText::new("Equalizer").size(14.0).color(DIM));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        if title_button(ui, TitleIcon::Close).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if title_button(ui, TitleIcon::Maximize).clicked() {
                            let max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
                        }
                        if title_button(ui, TitleIcon::Minimize).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                    });
                });
            });
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        card(ui, CARD, |ui| {
            ui.horizontal(|ui| {
                let n = self.pending();
                if ui.button("⟲  Read").clicked() {
                    self.read_all();
                }
                let apply = egui::Button::new(
                    RichText::new(if n > 0 {
                        format!("✔  Apply ({n})")
                    } else {
                        "✔  Apply".into()
                    })
                    .color(if n > 0 { Color32::BLACK } else { DIM }),
                )
                .fill(if n > 0 { PENDING } else { CARD_HI });
                if ui.add_enabled(n > 0, apply).clicked() {
                    self.apply();
                }
                if ui.button("Flat").clicked() {
                    for b in &mut self.bands {
                        b.gain = 0.0;
                    }
                }
                if ui
                    .button("Simpan ke DAC")
                    .on_hover_text("Simpan setelan di dongle agar terbawa ke perangkat lain")
                    .clicked()
                {
                    self.confirm_save = true;
                }
                ui.separator();
                self.preset_controls(ui);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut on = self.eq_en & 0xFF == EQ_ON;
                    if toggle(ui, &mut on).changed() {
                        self.set_eq_enable(if on { EQ_ON } else { EQ_OFF });
                    }
                    ui.label("DAC EQ");
                    ui.separator();
                    toggle(ui, &mut self.auto_apply).on_hover_text("Kirim perubahan otomatis setiap slider dilepas");
                    ui.label("Auto apply");
                });
            });
        });
    }

    fn bands_ui(&mut self, ui: &mut egui::Ui) {
        let eq_on = self.eq_en & 0xFF == EQ_ON;
        ui.columns(BANDS, |cols| {
            for (i, ui) in cols.iter_mut().enumerate() {
                let dirty = self.bands[i] != self.loaded[i];
                let b = &mut self.bands[i];
                let frame = egui::Frame::none()
                    .fill(if dirty { CARD_HI } else { CARD })
                    .rounding(Rounding::same(10.0))
                    .stroke(Stroke::new(1.0_f32, if dirty { PENDING } else { CARD }))
                    .inner_margin(egui::Margin::symmetric(8.0, 8.0));
                frame.show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("{}", i + 1)).size(11.0).color(DIM));
                            ui.label(
                                RichText::new(fmt_freq(b.freq))
                                    .size(14.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            );
                        });
                        let gcol = if b.gain > 0.05 {
                            ACCENT
                        } else if b.gain < -0.05 {
                            PENDING
                        } else {
                            DIM
                        };
                        ui.label(RichText::new(format!("{:+.1} dB", b.gain)).size(13.0).color(gcol));
                        // Vertical sliders ignore the parent's center alignment, so place one in an explicit centered rect.
                        let (row, _) =
                            ui.allocate_exact_size(Vec2::new(ui.available_width(), 140.0), egui::Sense::hover());
                        let slot = egui::Rect::from_center_size(row.center(), Vec2::new(32.0, 140.0));
                        let s = ui
                            .add_enabled_ui(eq_on, |ui| {
                                ui.put(
                                    slot,
                                    egui::Slider::new(&mut b.gain, -12.0..=12.0)
                                        .vertical()
                                        .step_by(0.1)
                                        .show_value(false),
                                )
                            })
                            .inner
                            .on_hover_text("Double-click: 0 dB");
                        if s.double_clicked() {
                            b.gain = 0.0;
                        }
                        field(ui, "Hz", egui::DragValue::new(&mut b.freq).range(20..=20000).speed(5.0));
                        field(
                            ui,
                            "Q",
                            egui::DragValue::new(&mut b.q)
                                .range(0.1..=10.0)
                                .speed(0.01)
                                .fixed_decimals(3),
                        );
                        egui::ComboBox::from_id_salt(("type", i))
                            .selected_text(type_name(b.kind))
                            .width(ui.available_width())
                            .show_ui(ui, |ui| {
                                for k in [TYPE_PEAK, TYPE_LOW_SHELF, TYPE_HIGH_SHELF] {
                                    ui.selectable_value(&mut b.kind, k, type_name(k));
                                }
                            });
                    });
                });
            }
        });
    }

    // Preset dropdown for the toolbar row; saving and deleting live inside the popup to keep the row short.
    fn preset_controls(&mut self, ui: &mut egui::Ui) {
        let cur = self
            .preset_sel
            .map(|i| self.presets[i].name.clone())
            .unwrap_or_else(|| "Preset".into());
        let mut pick = None;
        let mut save = None;
        let mut delete = false;
        egui::ComboBox::from_id_salt("preset")
            .selected_text(cur)
            .width(130.0)
            .height(320.0)
            .show_ui(ui, |ui| {
                ui.set_min_width(200.0);
                for (i, p) in self.presets.iter().enumerate() {
                    let label = if p.builtin {
                        p.name.clone()
                    } else {
                        format!("★ {}", p.name)
                    };
                    if ui.selectable_label(self.preset_sel == Some(i), label).clicked() {
                        pick = Some(i);
                    }
                }
                ui.separator();
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.preset_name)
                            .hint_text("Nama preset")
                            .desired_width(120.0),
                    );
                    let name = self.preset_name.trim().to_string();
                    if ui.add_enabled(!name.is_empty(), egui::Button::new("Simpan")).clicked() {
                        save = Some(name);
                    }
                });
                let can_del = self.preset_sel.is_some_and(|i| !self.presets[i].builtin);
                if can_del && ui.small_button("Hapus preset ini").clicked() {
                    delete = true;
                }
            });
        if let Some(name) = save {
            self.save_preset(name);
        }
        if delete {
            let i = self.preset_sel.take().unwrap();
            let name = self.presets.remove(i).name;
            match presets::save_user(&self.presets) {
                Ok(()) => self.set_status(true, format!("Preset \"{name}\" dihapus")),
                Err(e) => self.set_status(false, format!("Gagal simpan: {e}")),
            }
        }
        if let Some(i) = pick {
            self.preset_sel = Some(i);
            self.bands = self.presets[i].bands;
            if !self.presets[i].builtin {
                self.preset_name = self.presets[i].name.clone();
            }
            let msg = format!(
                "Preset \"{}\" dimuat{}",
                self.presets[i].name,
                if self.auto_apply { "" } else { " — tekan Apply" }
            );
            self.set_status(true, msg);
        }
    }

    fn save_preset(&mut self, name: String) {
        let p = Preset {
            name: name.clone(),
            bands: self.bands,
            builtin: false,
        };
        match self.presets.iter().position(|x| !x.builtin && x.name == name) {
            Some(i) => {
                self.presets[i] = p;
                self.preset_sel = Some(i);
            }
            None => {
                self.presets.push(p);
                self.preset_sel = Some(self.presets.len() - 1);
            }
        }
        match presets::save_user(&self.presets) {
            Ok(()) => self.set_status(true, format!("Preset \"{name}\" disimpan di PC")),
            Err(e) => self.set_status(false, format!("Gagal simpan: {e}")),
        }
    }

    fn curve_ui(&self, ui: &mut egui::Ui) {
        card(ui, CARD, |ui| {
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 110.0), egui::Sense::hover());
            let p = ui.painter_at(rect);
            let (fmin, fmax, db) = (20.0_f32, 20000.0_f32, 15.0_f32);
            let x_of = |f: f32| rect.left() + rect.width() * (f / fmin).log10() / (fmax / fmin).log10();
            let y_of = |g: f32| rect.center().y - g.clamp(-db, db) / db * rect.height() / 2.0;
            let grid = Stroke::new(1.0_f32, Color32::from_rgb(44, 47, 58));
            for f in [50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0, 10000.0] {
                let x = x_of(f);
                p.line_segment([egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())], grid);
                p.text(
                    egui::pos2(x + 3.0, rect.bottom() - 2.0),
                    egui::Align2::LEFT_BOTTOM,
                    fmt_freq(f as u16),
                    FontId::proportional(10.0),
                    DIM,
                );
            }
            for g in [-12.0, -6.0, 6.0, 12.0] {
                let y = y_of(g);
                p.line_segment([egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)], grid);
                p.text(
                    egui::pos2(rect.left() + 3.0, y - 1.0),
                    egui::Align2::LEFT_BOTTOM,
                    format!("{g:+.0}"),
                    FontId::proportional(10.0),
                    DIM,
                );
            }
            p.line_segment(
                [egui::pos2(rect.left(), y_of(0.0)), egui::pos2(rect.right(), y_of(0.0))],
                Stroke::new(1.0_f32, DIM),
            );

            let on = self.eq_en & 0xFF == EQ_ON;
            let n = rect.width().max(2.0) as usize;
            let pts: Vec<egui::Pos2> = (0..=n)
                .map(|i| {
                    let f = fmin * (fmax / fmin).powf(i as f32 / n as f32);
                    let g = if on {
                        self.bands.iter().map(|b| response_db(b, f)).sum()
                    } else {
                        0.0
                    };
                    egui::pos2(x_of(f), y_of(g))
                })
                .collect();
            let col = if on { ACCENT } else { DIM };
            p.add(egui::Shape::line(pts, Stroke::new(2.0_f32, col)));
            for (i, b) in self.bands.iter().enumerate() {
                let c = egui::pos2(x_of(b.freq as f32), y_of(if on { b.gain } else { 0.0 }));
                p.circle_filled(c, 5.0, if self.bands[i] != self.loaded[i] { PENDING } else { col });
                p.text(
                    c - Vec2::new(0.0, 8.0),
                    egui::Align2::CENTER_BOTTOM,
                    format!("{}", i + 1),
                    FontId::proportional(10.0),
                    Color32::WHITE,
                );
            }
            if let Some(pos) = resp.hover_pos() {
                let f = fmin * (fmax / fmin).powf((pos.x - rect.left()) / rect.width());
                let g: f32 = if on {
                    self.bands.iter().map(|b| response_db(b, f)).sum()
                } else {
                    0.0
                };
                p.text(
                    rect.right_top() + Vec2::new(-6.0, 4.0),
                    egui::Align2::RIGHT_TOP,
                    format!("{} · {g:+.1} dB", fmt_freq(f as u16)),
                    FontId::proportional(11.0),
                    Color32::WHITE,
                );
            }
        });
    }

    // Output gain (reg 0x66) lives in a collapsed "Lanjutan" section: testing showed the EQ doesn't clip,
    // so it only works as a volume trim. Shown as % of the factory value 0x200.
    fn advanced_ui(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new(RichText::new("Lanjutan").size(13.0).color(DIM))
            .id_salt("advanced")
            .default_open(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Output gain").color(DIM))
                        .on_hover_text("Volume digital DAC (register 0x66). 100% = bawaan pabrik.");
                    let mut pct = self.gain as f32 * 100.0 / GAIN_DEFAULT as f32;
                    let s = ui.add(egui::Slider::new(&mut pct, 0.0..=200.0).step_by(1.0).suffix(" %"));
                    if s.changed() {
                        self.gain = (pct * GAIN_DEFAULT as f32 / 100.0).round() as u32;
                    }
                    if self.gain != GAIN_DEFAULT && ui.small_button("Reset").clicked() {
                        self.gain = GAIN_DEFAULT;
                    }
                });
            });
    }
}

fn field(ui: &mut egui::Ui, label: &str, w: egui::DragValue) -> egui::Response {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(11.0).color(DIM));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| ui.add(w))
            .inner
    })
    .inner
}

// Magnitude (dB) of an RBJ biquad (peak or shelf, shelf Q = register Q) at the 48 kHz stream rate.
fn response_db(b: &Band, f: f32) -> f32 {
    if b.gain.abs() < 0.01 || b.freq == 0 {
        return 0.0;
    }
    let fs = 48000.0_f32;
    let a = 10f32.powf(b.gain / 40.0);
    let w0 = 2.0 * std::f32::consts::PI * (b.freq as f32).min(fs * 0.49) / fs;
    let (cw, alpha) = (w0.cos(), w0.sin() / (2.0 * b.q.max(0.05)));
    let sa = 2.0 * a.sqrt() * alpha;
    let ((b0, b1, b2), (a0, a1, a2)) = match b.kind {
        TYPE_LOW_SHELF => (
            (
                a * ((a + 1.0) - (a - 1.0) * cw + sa),
                2.0 * a * ((a - 1.0) - (a + 1.0) * cw),
                a * ((a + 1.0) - (a - 1.0) * cw - sa),
            ),
            (
                (a + 1.0) + (a - 1.0) * cw + sa,
                -2.0 * ((a - 1.0) + (a + 1.0) * cw),
                (a + 1.0) + (a - 1.0) * cw - sa,
            ),
        ),
        TYPE_HIGH_SHELF => (
            (
                a * ((a + 1.0) + (a - 1.0) * cw + sa),
                -2.0 * a * ((a - 1.0) + (a + 1.0) * cw),
                a * ((a + 1.0) + (a - 1.0) * cw - sa),
            ),
            (
                (a + 1.0) - (a - 1.0) * cw + sa,
                2.0 * ((a - 1.0) - (a + 1.0) * cw),
                (a + 1.0) - (a - 1.0) * cw - sa,
            ),
        ),
        _ => (
            (1.0 + alpha * a, -2.0 * cw, 1.0 - alpha * a),
            (1.0 + alpha / a, -2.0 * cw, 1.0 - alpha / a),
        ),
    };
    let w = 2.0 * std::f32::consts::PI * f / fs;
    let (c1, s1, c2, s2) = (w.cos(), w.sin(), (2.0 * w).cos(), (2.0 * w).sin());
    let num = (b0 + b1 * c1 + b2 * c2).powi(2) + (b1 * s1 + b2 * s2).powi(2);
    let den = (a0 + a1 * c1 + a2 * c2).powi(2) + (a1 * s1 + a2 * s2).powi(2);
    10.0 * (num / den).log10()
}

fn type_name(k: u16) -> &'static str {
    match k {
        TYPE_PEAK => "Peak",
        TYPE_LOW_SHELF => "Low shelf",
        TYPE_HIGH_SHELF => "High shelf",
        _ => "Lainnya",
    }
}

fn fmt_freq(f: u16) -> String {
    if f >= 1000 {
        let k = f as f32 / 1000.0;
        if f % 1000 == 0 {
            format!("{k:.0} kHz")
        } else {
            format!("{k:.1} kHz")
        }
    } else {
        format!("{f} Hz")
    }
}

// Undecorated windows lose the native resize border, so detect the pointer near an edge and hand off to the OS.
fn resize_edges(ctx: &egui::Context) {
    use egui::{CursorIcon as C, ResizeDirection as D};
    if ctx.input(|i| i.viewport().maximized.unwrap_or(false)) {
        return;
    }
    let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) else {
        return;
    };
    let r = ctx.screen_rect();
    let m = 6.0;
    let (l, rt, t, b) = (
        pos.x < r.left() + m,
        pos.x > r.right() - m,
        pos.y < r.top() + m,
        pos.y > r.bottom() - m,
    );
    let dir = match (l, rt, t, b) {
        (true, _, true, _) => Some((D::NorthWest, C::ResizeNorthWest)),
        (_, true, true, _) => Some((D::NorthEast, C::ResizeNorthEast)),
        (true, _, _, true) => Some((D::SouthWest, C::ResizeSouthWest)),
        (_, true, _, true) => Some((D::SouthEast, C::ResizeSouthEast)),
        (true, ..) => Some((D::West, C::ResizeWest)),
        (_, true, ..) => Some((D::East, C::ResizeEast)),
        (_, _, true, _) => Some((D::North, C::ResizeNorth)),
        (.., true) => Some((D::South, C::ResizeSouth)),
        _ => None,
    };
    if let Some((d, cursor)) = dir {
        ctx.set_cursor_icon(cursor);
        if ctx.input(|i| i.pointer.primary_pressed()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(d));
        }
    }
}

// Ask Windows 11 DWM for rounded corners and a shadow on the undecorated window. No-op on older Windows.
#[cfg(windows)]
fn round_corners(cc: &eframe::CreationContext) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND};
    let Ok(h) = cc.window_handle() else { return };
    if let RawWindowHandle::Win32(w) = h.as_raw() {
        let pref = DWMWCP_ROUND;
        unsafe {
            DwmSetWindowAttribute(
                w.hwnd.get() as _,
                DWMWA_WINDOW_CORNER_PREFERENCE as u32,
                &pref as *const _ as *const _,
                std::mem::size_of_val(&pref) as u32,
            );
        }
    }
}

#[cfg(not(windows))]
fn round_corners(_: &eframe::CreationContext) {}

#[derive(Clone, Copy, PartialEq)]
enum TitleIcon {
    Minimize,
    Maximize,
    Close,
}

// Flat 46x44 caption button with a painted glyph; close turns red on hover.
fn title_button(ui: &mut egui::Ui, icon: TitleIcon) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(40.0, 36.0), egui::Sense::click());
    let p = ui.painter();
    if resp.hovered() {
        let fill = if icon == TitleIcon::Close {
            Color32::from_rgb(196, 43, 28)
        } else {
            CARD_HI
        };
        p.rect_filled(rect, 0.0, fill);
    }
    let col = if resp.hovered() {
        Color32::WHITE
    } else {
        Color32::from_rgb(190, 195, 208)
    };
    let s = Stroke::new(1.2_f32, col);
    let c = rect.center();
    let h = 5.0;
    match icon {
        TitleIcon::Minimize => {
            p.line_segment([c + Vec2::new(-h, 0.0), c + Vec2::new(h, 0.0)], s);
        }
        TitleIcon::Maximize => {
            p.rect_stroke(egui::Rect::from_center_size(c, Vec2::splat(2.0 * h)), 1.0, s);
        }
        TitleIcon::Close => {
            p.line_segment([c + Vec2::new(-h, -h), c + Vec2::new(h, h)], s);
            p.line_segment([c + Vec2::new(-h, h), c + Vec2::new(h, -h)], s);
        }
    }
    resp
}

// iOS-style switch
fn toggle(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let size = Vec2::new(40.0, 22.0);
    let (rect, mut resp) = ui.allocate_exact_size(size, egui::Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let t = ui.ctx().animate_bool_responsive(resp.id, *on);
    let bg = if *on { ACCENT } else { Color32::from_rgb(60, 64, 78) };
    let p = ui.painter();
    p.rect_filled(rect, rect.height() / 2.0, bg);
    let r = rect.height() / 2.0 - 3.0;
    let x = egui::lerp(rect.left() + r + 3.0..=rect.right() - r - 3.0, t);
    p.circle_filled(egui::pos2(x, rect.center().y), r, Color32::WHITE);
    resp
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.poll_connection(ctx);
        self.save_confirm_ui(ctx);
        // Auto apply waits for mouse release so a drag doesn't flood the device with writes.
        if self.auto_apply && self.dev.is_some() && self.pending() > 0 && !ctx.input(|i| i.pointer.any_down()) {
            self.apply();
            if !self.status_ok {
                self.auto_apply = false;
            }
        }
        resize_edges(ctx);
        self.title_bar(ctx);
        // Status bar: device identity on the left, last action result on the right.
        egui::TopBottomPanel::bottom("status")
            .exact_height(26.0)
            .show_separator_line(false)
            .frame(
                egui::Frame::none()
                    .fill(Color32::from_rgb(14, 15, 19))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(36, 39, 50)))
                    .inner_margin(egui::Margin::symmetric(12.0, 0.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    let col = if self.dev.is_some() { OK } else { ERR };
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(8.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 3.5, col);
                    let id = if self.dev.is_some() {
                        self.dev_info.as_str()
                    } else {
                        "Tidak terhubung"
                    };
                    ui.label(RichText::new(id).size(12.0).color(Color32::from_rgb(190, 195, 208)));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if !self.status.is_empty() {
                            let c = if self.status_ok { DIM } else { ERR };
                            ui.label(RichText::new(&self.status).size(12.0).color(c));
                        }
                    });
                });
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(BG))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                    egui::Frame::none()
                        .inner_margin(egui::Margin::same(10.0))
                        .show(ui, |ui| {
                            if self.dev.is_none() {
                                self.disconnected_ui(ui);
                                return;
                            }
                            self.toolbar(ui);
                            ui.add_space(4.0);
                            self.curve_ui(ui);
                            ui.add_space(4.0);
                            self.bands_ui(ui);
                            ui.add_space(4.0);
                            self.advanced_ui(ui);
                        });
                });
            });
    }
}
