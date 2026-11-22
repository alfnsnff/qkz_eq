// QKZ TC MAX EQ — live register edit only, no save to flash.
mod device;
mod presets;

use device::{Band, Device, BANDS, REG_DAC_EQ_EN, REG_DAC_GAIN};
use eframe::egui::{self, Color32, FontId, RichText, Rounding, Stroke, Vec2};
use presets::Preset;

const BG: Color32 = Color32::from_rgb(18, 19, 24);
const CARD: Color32 = Color32::from_rgb(28, 30, 38);
const CARD_HI: Color32 = Color32::from_rgb(36, 39, 50);
const ACCENT: Color32 = Color32::from_rgb(94, 156, 255);
const PENDING: Color32 = Color32::from_rgb(255, 184, 77);
const OK: Color32 = Color32::from_rgb(92, 214, 140);
const ERR: Color32 = Color32::from_rgb(255, 99, 99);
const DIM: Color32 = Color32::from_rgb(130, 136, 152);

fn main() -> eframe::Result<()> {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([820.0, 840.0])
            .with_min_inner_size([720.0, 780.0])
            .with_icon(eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png")).unwrap_or_default()),
        ..Default::default()
    };
    eframe::run_native(
        "QKZ TC MAX EQ",
        opts,
        Box::new(|cc| {
            setup_style(&cc.egui_ctx);
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
    let r = Rounding::same(8.0);
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
        s.spacing.item_spacing = Vec2::new(8.0, 8.0);
        s.spacing.button_padding = Vec2::new(14.0, 7.0);
        s.spacing.slider_width = 200.0;
        s.spacing.interact_size.y = 26.0;
        s.text_styles.insert(egui::TextStyle::Body, FontId::proportional(14.0));
        s.text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(14.0));
    });
}

fn card(ui: &mut egui::Ui, fill: Color32, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::none()
        .fill(fill)
        .rounding(Rounding::same(12.0))
        .inner_margin(egui::Margin::same(14.0))
        .show(ui, add);
}

struct App {
    dev: Option<Device>,
    bands: [Band; BANDS],
    loaded: [Band; BANDS],
    eq_en: u32,
    gain: u32,
    gain_loaded: u32,
    gain_enabled: bool,
    status: String,
    status_ok: bool,
    presets: Vec<Preset>,
    preset_sel: Option<usize>,
    preset_name: String,
    auto_apply: bool,
    dev_info: String,
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
            gain_enabled: false,
            status: String::new(),
            status_ok: true,
            presets: presets::builtins().into_iter().chain(presets::load_user()).collect(),
            preset_sel: None,
            preset_name: String::new(),
            auto_apply: false,
            dev_info: String::new(),
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
            }
            Err(e) => self.set_status(false, e),
        }
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
            if self.gain_enabled && self.gain != self.gain_loaded {
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
                self.set_status(true, format!("DAC EQ {}", if en != 0 { "aktif" } else { "nonaktif" }));
            }
            Err(e) => self.set_status(false, format!("Gagal: {e}")),
        }
    }

    fn pending(&self) -> usize {
        let b = (0..BANDS).filter(|&i| self.bands[i] != self.loaded[i]).count();
        b + (self.gain_enabled && self.gain != self.gain_loaded) as usize
    }

    fn header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("QKZ TC MAX").size(22.0).strong().color(Color32::WHITE));
            ui.label(RichText::new("Equalizer").size(22.0).color(DIM));
        });
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        card(ui, CARD, |ui| {
            ui.horizontal(|ui| {
                if self.dev.is_none() {
                    if ui
                        .add(egui::Button::new(RichText::new("Connect").color(Color32::WHITE)).fill(ACCENT))
                        .clicked()
                    {
                        self.connect();
                    }
                    ui.label(RichText::new("Colokkan DAC lalu tekan Connect").color(DIM));
                    return;
                }
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
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut on = self.eq_en != 0;
                    if toggle(ui, &mut on).changed() {
                        self.set_eq_enable(if on { 3 } else { 0 });
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
        let eq_on = self.eq_en != 0;
        ui.columns(BANDS, |cols| {
            for (i, ui) in cols.iter_mut().enumerate() {
                let dirty = self.bands[i] != self.loaded[i];
                let b = &mut self.bands[i];
                card(ui, if dirty { CARD_HI } else { CARD }, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new(format!("BAND {}", i + 1)).size(11.0).color(DIM));
                        ui.label(
                            RichText::new(fmt_freq(b.freq))
                                .size(17.0)
                                .strong()
                                .color(Color32::WHITE),
                        );
                        let gcol = if b.gain > 0.05 {
                            ACCENT
                        } else if b.gain < -0.05 {
                            PENDING
                        } else {
                            DIM
                        };
                        ui.label(RichText::new(format!("{:+.1} dB", b.gain)).size(15.0).color(gcol));
                        ui.add_space(4.0);
                        // Vertical sliders ignore the parent's center alignment, so place one in an explicit centered rect.
                        let (row, _) =
                            ui.allocate_exact_size(Vec2::new(ui.available_width(), 200.0), egui::Sense::hover());
                        let slot = egui::Rect::from_center_size(row.center(), Vec2::new(32.0, 200.0));
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
                        ui.add_space(4.0);
                        field(
                            ui,
                            "Freq",
                            egui::DragValue::new(&mut b.freq)
                                .range(20..=20000)
                                .speed(5.0)
                                .suffix(" Hz"),
                        );
                        field(
                            ui,
                            "Q",
                            egui::DragValue::new(&mut b.q)
                                .range(0.1..=10.0)
                                .speed(0.01)
                                .fixed_decimals(3),
                        );
                        field(ui, "Type", egui::DragValue::new(&mut b.kind).range(0..=7));
                        ui.label(RichText::new(type_name(b.kind)).size(11.0).color(DIM))
                            .on_hover_text("Arti type belum dipastikan; hanya 0 yang sudah terlihat di perangkat");
                        let (rect, _) =
                            ui.allocate_exact_size(Vec2::new(ui.available_width(), 4.0), egui::Sense::hover());
                        if dirty {
                            ui.painter()
                                .rect_filled(rect.shrink2(Vec2::new(20.0, 0.0)), 2.0, PENDING);
                        }
                    });
                });
            }
        });
    }

    fn preset_ui(&mut self, ui: &mut egui::Ui) {
        card(ui, CARD, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Preset").strong().color(Color32::WHITE));
                let cur = self
                    .preset_sel
                    .map(|i| self.presets[i].name.clone())
                    .unwrap_or_else(|| "— pilih —".into());
                let mut pick = None;
                egui::ComboBox::from_id_salt("preset")
                    .selected_text(cur)
                    .width(150.0)
                    .show_ui(ui, |ui| {
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
                    });
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
                let can_del = self.preset_sel.is_some_and(|i| !self.presets[i].builtin);
                if ui
                    .add_enabled(can_del, egui::Button::new("Hapus").min_size(Vec2::new(0.0, 30.0)))
                    .clicked()
                {
                    let i = self.preset_sel.take().unwrap();
                    let name = self.presets.remove(i).name;
                    match presets::save_user(&self.presets) {
                        Ok(()) => self.set_status(true, format!("Preset \"{name}\" dihapus")),
                        Err(e) => self.set_status(false, format!("Gagal simpan: {e}")),
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let name = self.preset_name.trim().to_string();
                    if ui
                        .add_enabled(
                            !name.is_empty(),
                            egui::Button::new("Simpan").min_size(Vec2::new(0.0, 30.0)),
                        )
                        .clicked()
                    {
                        self.save_preset(name);
                    }
                    ui.add(
                        egui::TextEdit::singleline(&mut self.preset_name)
                            .hint_text("Nama preset baru")
                            .desired_width(160.0)
                            .min_size(Vec2::new(160.0, 30.0))
                            .margin(egui::Margin::symmetric(10.0, 7.0))
                            .vertical_align(egui::Align::Center),
                    );
                });
            });
        });
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
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 150.0), egui::Sense::hover());
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

            let on = self.eq_en != 0;
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

    fn preamp_ui(&mut self, ui: &mut egui::Ui) {
        card(ui, CARD, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Preamp").strong().color(Color32::WHITE));
                ui.label(
                    RichText::new("(eksperimen, nyalakan saklar untuk mengubah)")
                        .size(12.0)
                        .color(DIM),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("di perangkat: 0x{:03X}", self.gain_loaded)).color(DIM))
                        .on_hover_text("Nilai register 0x66 yang terakhir dibaca/ditulis ke DAC");
                    ui.add_enabled(
                        self.gain_enabled,
                        egui::DragValue::new(&mut self.gain)
                            .range(0..=0x400)
                            .hexadecimal(3, false, true),
                    );
                    toggle(ui, &mut self.gain_enabled);
                });
            });
        });
    }
}

fn field(ui: &mut egui::Ui, label: &str, w: egui::DragValue) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(12.0).color(DIM));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| ui.add(w));
    });
}

// Magnitude (dB) of an RBJ peaking biquad at 48 kHz. Assumes every type is peaking (type mapping unverified).
fn response_db(b: &Band, f: f32) -> f32 {
    if b.gain.abs() < 0.01 || b.freq == 0 {
        return 0.0;
    }
    let fs = 48000.0_f32;
    let a = 10f32.powf(b.gain / 40.0);
    let w0 = 2.0 * std::f32::consts::PI * (b.freq as f32).min(fs * 0.49) / fs;
    let alpha = w0.sin() / (2.0 * b.q.max(0.05));
    let (b0, b1, b2) = (1.0 + alpha * a, -2.0 * w0.cos(), 1.0 - alpha * a);
    let (a0, a1, a2) = (1.0 + alpha / a, -2.0 * w0.cos(), 1.0 - alpha / a);
    let w = 2.0 * std::f32::consts::PI * f / fs;
    let (c1, s1, c2, s2) = (w.cos(), w.sin(), (2.0 * w).cos(), (2.0 * w).sin());
    let num = (b0 + b1 * c1 + b2 * c2).powi(2) + (b1 * s1 + b2 * s2).powi(2);
    let den = (a0 + a1 * c1 + a2 * c2).powi(2) + (a1 * s1 + a2 * s2).powi(2);
    10.0 * (num / den).log10()
}

fn type_name(k: u16) -> &'static str {
    match k {
        0 => "Peak",
        _ => "belum diketahui",
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
        // Auto apply waits for mouse release so a drag doesn't flood the device with writes.
        if self.auto_apply && self.dev.is_some() && self.pending() > 0 && !ctx.input(|i| i.pointer.any_down()) {
            self.apply();
            if !self.status_ok {
                self.auto_apply = false;
            }
        }
        // Status bar: device identity on the left, last action result on the right.
        egui::TopBottomPanel::bottom("status")
            .exact_height(32.0)
            .show_separator_line(false)
            .frame(
                egui::Frame::none()
                    .fill(Color32::from_rgb(14, 15, 19))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(36, 39, 50)))
                    .inner_margin(egui::Margin::symmetric(16.0, 0.0)),
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
                        .inner_margin(egui::Margin::same(16.0))
                        .show(ui, |ui| {
                            self.header(ui);
                            ui.add_space(4.0);
                            self.toolbar(ui);
                            if self.dev.is_none() {
                                return;
                            }
                            ui.add_space(4.0);
                            self.preset_ui(ui);
                            ui.add_space(4.0);
                            self.curve_ui(ui);
                            ui.add_space(4.0);
                            self.bands_ui(ui);
                            ui.add_space(4.0);
                            self.preamp_ui(ui);
                        });
                });
            });
    }
}
