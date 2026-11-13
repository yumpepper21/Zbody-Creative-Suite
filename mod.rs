//! Zbody Vector —— 基于 egui 的极简矢量图形编辑器模块。
//!
//! 布局：左侧工具栏 · 中间深色画布 · 右侧属性面板。
//! 功能：矩形 / 椭圆拖拽创建（黑色填充、无描边）、选择工具点选并拖拽移动、
//! 钢笔工具（kurbo 三次贝塞尔）点击加点、拖拽拉出对称控制手柄、Enter 闭合路径。

mod export;
mod shape;

pub use shape::{Anchor, Shape, ShapeKind, Tool};

use eframe::egui::{
    self, Align2, Color32, FontId, Image, Pos2, Rect, Response, RichText, Sense, TextureHandle,
    Stroke, StrokeKind, Ui, Vec2,
};

const ACCENT: Color32 = Color32::from_rgb(86, 148, 255);
const CANVAS_BG: Color32 = Color32::from_rgb(42, 42, 48);
const GRID_LINE: Color32 = Color32::from_rgb(52, 52, 59);
const PANEL_BG: Color32 = Color32::from_rgb(30, 30, 35);
const TOOLBAR_BG: Color32 = Color32::from_rgb(24, 24, 28);
const TEXT_DIM: Color32 = Color32::from_rgb(142, 142, 152);

pub struct ZbodyVectorApp {
    editor: VectorEditor,
}

impl ZbodyVectorApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        install_chinese_font(&cc.egui_ctx);
        configure_style(&cc.egui_ctx);

        let mut editor = VectorEditor::default();
        if let Some(icon) = load_icon_texture(&cc.egui_ctx) {
            editor.set_icon(icon);
        }

        Self { editor }
    }
}

impl eframe::App for ZbodyVectorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.editor.ui(ctx);
    }
}

/// 图标资源路径（相对工作目录，其次可执行文件目录）。
const ICON_PATHS: &[&str] = &["assets/zbody-icon.png", "../assets/zbody-icon.png"];

fn load_icon_rgba() -> Option<(u32, u32, Vec<u8>)> {
    let bytes = ICON_PATHS
        .iter()
        .find_map(|p| std::fs::read(p).ok())
        .or_else(|| {
            let exe = std::env::current_exe().ok()?;
            let base = exe.parent()?.join("assets/zbody-icon.png");
            std::fs::read(base).ok()
        })?;

    let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    Some((w, h, img.into_raw()))
}

/// 解码应用图标，用于窗口图标（ViewportBuilder::with_icon）。
pub fn load_icon() -> Option<egui::IconData> {
    let (width, height, rgba) = load_icon_rgba()?;
    Some(egui::IconData { width, height, rgba })
}

/// 解码应用图标为 GPU 纹理，用于工具栏导出按钮。
fn load_icon_texture(ctx: &egui::Context) -> Option<TextureHandle> {
    let (w, h, rgba) = load_icon_rgba()?;
    let image =
        egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
    Some(ctx.load_texture("zbody-icon", image, egui::TextureOptions::LINEAR))
}

/// 拖拽创建图形时的临时状态。
#[derive(Clone, Copy)]
struct Draft {
    start: Pos2,
    current: Pos2,
}

impl Draft {
    fn rect(&self) -> Rect {
        Rect::from_two_pos(self.start, self.current)
    }
}

/// 矢量编辑器本体，可独立嵌入到其他 egui 应用。
pub struct VectorEditor {
    tool: Tool,
    shapes: Vec<Shape>,
    selected: Option<u64>,
    next_id: u64,
    draft: Option<Draft>,
    /// 正在绘制中的钢笔锚点。
    pen_anchors: Vec<Anchor>,
    /// 鼠标位置，用于绘制钢笔橡皮筋预览。
    pen_hover: Option<Pos2>,
    /// 正在拖拽控制手柄的锚点下标。
    pen_drag: Option<usize>,
    /// 导出按钮使用的应用图标纹理。
    icon: Option<TextureHandle>,
    /// 最近一次操作状态（导出结果等），显示在属性面板底部。
    status: Option<String>,
}

impl Default for VectorEditor {
    fn default() -> Self {
        Self {
            tool: Tool::Select,
            shapes: Vec::new(),
            selected: None,
            next_id: 0,
            draft: None,
            pen_anchors: Vec::new(),
            pen_hover: None,
            pen_drag: None,
            icon: None,
            status: None,
        }
    }
}

impl VectorEditor {
    /// 设置应用图标（用于工具栏导出按钮）。
    pub fn set_icon(&mut self, icon: TextureHandle) {
        self.icon = Some(icon);
    }

    pub fn ui(&mut self, ctx: &egui::Context) {
        self.handle_shortcuts(ctx);
        self.toolbar(ctx);
        self.properties(ctx);
        self.canvas_panel(ctx);
    }

    // ---------- 界面 ----------

    fn toolbar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("zv_toolbar")
            .exact_width(72.0)
            .resizable(false)
            .frame(
                egui::Frame::NONE
                    .fill(TOOLBAR_BG)
                    .inner_margin(egui::Margin::symmetric(16, 16)),
            )
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("ZV").size(15.0).strong().color(ACCENT));
                    ui.add_space(16.0);

                    for tool in [Tool::Select, Tool::Rect, Tool::Ellipse, Tool::Pen] {
                        if tool_button(ui, tool, self.tool == tool) {
                            self.set_tool(tool);
                        }
                        ui.add_space(4.0);
                    }

                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(10.0);

                    // 导出 SVG 按钮（使用应用图标）
                    if export_button(ui, self.icon.as_ref()) {
                        self.export_svg();
                    }

                    ui.add_space(10.0);
                    ui.label(RichText::new("Zbody").size(10.0).color(TEXT_DIM));
                    ui.label(RichText::new("Vector").size(10.0).color(TEXT_DIM));
                });
            });
    }

    fn properties(&mut self, ctx: &egui::Context) {
        // 先算出选中图形的下标，闭包内可通过它直接修改图形属性
        let selected_idx = self
            .selected
            .and_then(|id| self.shapes.iter().position(|s| s.id == id));

        egui::SidePanel::right("zv_properties")
            .exact_width(256.0)
            .resizable(false)
            .frame(
                egui::Frame::NONE
                    .fill(PANEL_BG)
                    .inner_margin(egui::Margin::same(16)),
            )
            .show(ctx, |ui| {
                ui.label(RichText::new("属性").size(15.0).strong());
                ui.add_space(2.0);
                ui.label(RichText::new("Zbody Vector").size(11.0).color(TEXT_DIM));
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(10.0);

                match selected_idx {
                    Some(idx) => {
                        let (kind, bounds, pen_info, color_hex) = {
                            let s = &self.shapes[idx];
                            let pen_info = if let ShapeKind::Pen { anchors, closed } = &s.kind {
                                Some((anchors.len(), *closed))
                            } else {
                                None
                            };
                            (
                                s.kind.label().to_owned(),
                                s.bounds(),
                                pen_info,
                                s.color_hex(),
                            )
                        };
                        let filled = pen_info.is_none_or(|(_, closed)| closed);

                        egui::Grid::new("zv_shape_grid")
                            .num_columns(2)
                            .spacing([14.0, 8.0])
                            .show(ui, |ui| {
                                ui.label(RichText::new("类型").color(TEXT_DIM));
                                ui.label(kind);
                                ui.end_row();

                                if let Some((count, closed)) = pen_info {
                                    ui.label(RichText::new("锚点").color(TEXT_DIM));
                                    ui.label(format!("{count}"));
                                    ui.end_row();

                                    ui.label(RichText::new("闭合").color(TEXT_DIM));
                                    ui.label(if closed { "是" } else { "否" });
                                    ui.end_row();
                                }

                                ui.label(RichText::new("X").color(TEXT_DIM));
                                ui.label(format!("{:.1}", bounds.min.x));
                                ui.end_row();

                                ui.label(RichText::new("Y").color(TEXT_DIM));
                                ui.label(format!("{:.1}", bounds.min.y));
                                ui.end_row();

                                ui.label(RichText::new("宽度").color(TEXT_DIM));
                                ui.label(format!("{:.1}", bounds.width()));
                                ui.end_row();

                                ui.label(RichText::new("高度").color(TEXT_DIM));
                                ui.label(format!("{:.1}", bounds.height()));
                                ui.end_row();

                                // 颜色行：实时编辑，画布即时变色
                                let color_label = if filled { "填充" } else { "描边" };
                                ui.label(RichText::new(color_label).color(TEXT_DIM));
                                let mut rgb = {
                                    let c = self.shapes[idx].color;
                                    [c.r(), c.g(), c.b()]
                                };
                                if ui
                                    .color_edit_button_srgb(&mut rgb)
                                    .on_hover_text("点击修改颜色")
                                    .changed()
                                {
                                    self.shapes[idx].color = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                                }
                                ui.label(RichText::new(color_hex.clone()).size(12.0));
                                ui.end_row();

                                ui.label(RichText::new("描边").color(TEXT_DIM));
                                ui.label(if filled {
                                    "无".to_string()
                                } else {
                                    format!("{} · 2 px", color_hex)
                                });
                                ui.end_row();
                            });

                        ui.add_space(14.0);
                        ui.separator();
                        ui.add_space(10.0);

                        let delete_button = egui::Button::new(
                            RichText::new("删除图形").color(Color32::from_rgb(235, 235, 240)),
                        )
                        .fill(Color32::from_rgb(56, 56, 64));

                        if ui
                            .add_sized([ui.available_width(), 30.0], delete_button)
                            .clicked()
                        {
                            self.delete_selected();
                        }
                    }
                    None => {
                        ui.label(RichText::new("未选中图形").color(TEXT_DIM));
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new(
                                "用矩形 / 椭圆工具在画布中拖拽即可创建图形。\n\
                                 钢笔工具：点击加点，按住拖拽拉出控制手柄，Enter 闭合路径。",
                            )
                            .size(12.0)
                            .color(TEXT_DIM),
                        );
                    }
                }

                ui.add_space(14.0);
                ui.separator();
                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!("图形数量：{}", self.shapes.len()))
                        .size(12.0)
                        .color(TEXT_DIM),
                );
                ui.label(
                    RichText::new(format!("当前工具：{}", self.tool.label()))
                        .size(12.0)
                        .color(TEXT_DIM),
                );

                if let Some(status) = &self.status {
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(6.0);
                    ui.label(RichText::new(status).size(11.0).color(ACCENT));
                }
            });
    }

    fn canvas_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(CANVAS_BG))
            .show(ctx, |ui| {
                let (response, painter) =
                    ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
                self.canvas(ui, &response, &painter);
            });
    }

    fn canvas(&mut self, ui: &mut Ui, response: &Response, painter: &egui::Painter) {
        let canvas_rect = response.rect;
        painter.rect_filled(canvas_rect, 0.0, CANVAS_BG);
        paint_grid(painter, canvas_rect);

        self.handle_canvas_input(ui, response);

        for shape in &self.shapes {
            shape.paint(painter);
        }
        self.paint_draft(painter);
        self.paint_pen_draft(painter);
        self.paint_selection(painter);

        if self.shapes.is_empty() && self.draft.is_none() && self.pen_anchors.is_empty() {
            painter.text(
                canvas_rect.center() - egui::vec2(0.0, 12.0),
                Align2::CENTER_CENTER,
                "Zbody Vector",
                FontId::proportional(22.0),
                Color32::from_rgb(118, 118, 128),
            );
            painter.text(
                canvas_rect.center() + egui::vec2(0.0, 18.0),
                Align2::CENTER_CENTER,
                "拖拽创建矩形 / 椭圆　·　钢笔点击加点、拖拽拉手柄，Enter 闭合、Esc 取消",
                FontId::proportional(13.0),
                TEXT_DIM,
            );
        }

        if response.hovered() {
            let cursor = match self.tool {
                Tool::Select => {
                    if self.selected.is_some() && response.dragged() {
                        egui::CursorIcon::Grabbing
                    } else if response
                        .hover_pos()
                        .is_some_and(|p| self.shape_at(p).is_some())
                    {
                        egui::CursorIcon::Grab
                    } else {
                        egui::CursorIcon::Default
                    }
                }
                Tool::Rect | Tool::Ellipse | Tool::Pen => egui::CursorIcon::Crosshair,
            };
            ui.ctx().set_cursor_icon(cursor);
        }
    }

    // ---------- 交互 ----------

    fn handle_canvas_input(&mut self, ui: &mut Ui, response: &Response) {
        match self.tool {
            Tool::Rect | Tool::Ellipse => {
                if response.drag_started() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        self.draft = Some(Draft {
                            start: pos,
                            current: pos,
                        });
                    }
                }

                if response.dragged() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        if let Some(draft) = self.draft.as_mut() {
                            draft.current = pos;
                        }
                    }
                }

                if response.drag_stopped() {
                    if let Some(draft) = self.draft.take() {
                        let rect = draft.rect();
                        if rect.width() >= 3.0 && rect.height() >= 3.0 {
                            let kind = match self.tool {
                                Tool::Rect => ShapeKind::Rect { rect },
                                _ => ShapeKind::Ellipse { rect },
                            };
                            let id = self.push_shape(kind);
                            self.selected = Some(id);
                        }
                    }
                }
            }

            Tool::Select => {
                if response.drag_started() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        self.selected = self.shape_at(pos).map(|s| s.id);
                    }
                }

                if response.dragged() {
                    let delta = response.drag_delta();
                    if delta != egui::Vec2::ZERO {
                        if let Some(id) = self.selected {
                            if let Some(shape) = self.shapes.iter_mut().find(|s| s.id == id) {
                                shape.translate(delta);
                            }
                        }
                    }
                }

                if response.clicked() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        self.selected = self.shape_at(pos).map(|s| s.id);
                    }
                }
            }

            Tool::Pen => {
                self.handle_pen_input(ui, response);
            }
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let (delete, escape, enter) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace),
                i.key_pressed(egui::Key::Escape),
                i.key_pressed(egui::Key::Enter),
            )
        });

        if delete && self.selected.is_some() && self.pen_anchors.is_empty() {
            self.delete_selected();
        }

        if enter && self.tool == Tool::Pen && !self.pen_anchors.is_empty() {
            self.finish_pen(true);
        }

        if escape {
            if !self.pen_anchors.is_empty() || self.pen_drag.is_some() {
                self.cancel_pen();
            } else {
                self.selected = None;
                self.tool = Tool::Select;
            }
        }
    }

    /// 钢笔交互：按下鼠标时只记录位置，开始拖拽后才生成锚点并拉出对称控制手柄；
    /// 不拖拽的单击则在松手时生成一个角点。
    fn handle_pen_input(&mut self, ui: &mut Ui, response: &Response) {
        if response.drag_started() {
            let press_origin = ui
                .input(|i| i.pointer.press_origin())
                .or_else(|| response.interact_pointer_pos());

            if let Some(pos) = press_origin {
                if self.pen_anchors.is_empty() {
                    self.selected = None;
                }
                self.pen_anchors.push(Anchor::new(pos));
                let index = self.pen_anchors.len() - 1;
                self.pen_drag = Some(index);

                if let Some(current) = response.interact_pointer_pos() {
                    self.update_pen_handle(index, current);
                }
            }
        }

        if let Some(index) = self.pen_drag {
            if response.dragged() {
                if let Some(pos) = response.interact_pointer_pos() {
                    self.update_pen_handle(index, pos);
                }
            }

            if response.drag_stopped() {
                // 手柄太短时视为角点
                if let Some(anchor) = self.pen_anchors.get_mut(index) {
                    if anchor.out_handle.length() < 2.0 {
                        anchor.in_handle = Vec2::ZERO;
                        anchor.out_handle = Vec2::ZERO;
                    }
                }
                self.pen_drag = None;
            }
        }

        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                if self.can_close_pen(pos) {
                    // 点击起点闭合路径
                    self.finish_pen(true);
                } else {
                    if self.pen_anchors.is_empty() {
                        self.selected = None;
                    }
                    self.pen_anchors.push(Anchor::new(pos));
                }
            }
        }

        self.pen_hover = if self.pen_drag.is_none() {
            response.hover_pos()
        } else {
            None
        };
    }

    fn update_pen_handle(&mut self, index: usize, pointer: Pos2) {
        if let Some(anchor) = self.pen_anchors.get_mut(index) {
            // 对称手柄，得到平滑曲线
            let handle = pointer - anchor.pos;
            anchor.out_handle = handle;
            anchor.in_handle = -handle;
        }
    }

    fn can_close_pen(&self, pos: Pos2) -> bool {
        self.pen_anchors.len() >= 3
            && self
                .pen_anchors
                .first()
                .is_some_and(|first| first.pos.distance(pos) <= 8.0)
    }

    fn finish_pen(&mut self, closed: bool) {
        self.pen_drag = None;
        self.pen_hover = None;

        let anchors = std::mem::take(&mut self.pen_anchors);
        if anchors.len() >= 2 {
            let id = self.push_shape(ShapeKind::Pen { anchors, closed });
            self.selected = Some(id);
        }
    }

    fn cancel_pen(&mut self) {
        self.pen_anchors.clear();
        self.pen_hover = None;
        self.pen_drag = None;
    }

    fn set_tool(&mut self, tool: Tool) {
        // 切换工具时保留未闭合路径
        if self.tool == Tool::Pen && tool != Tool::Pen && !self.pen_anchors.is_empty() {
            self.finish_pen(false);
        }
        self.tool = tool;
        if tool != Tool::Pen {
            self.cancel_pen();
        }
    }

    fn push_shape(&mut self, kind: ShapeKind) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        self.shapes.push(Shape::new(id, kind));
        id
    }

    fn shape_at(&self, pos: Pos2) -> Option<&Shape> {
        self.shapes.iter().rev().find(|s| s.hit_test(pos, 4.0))
    }

    fn selected_shape(&self) -> Option<&Shape> {
        let id = self.selected?;
        self.shapes.iter().find(|s| s.id == id)
    }

    fn delete_selected(&mut self) {
        if let Some(id) = self.selected.take() {
            self.shapes.retain(|s| s.id != id);
        }
    }

    /// 弹出保存对话框并把画布内容导出为 SVG。
    fn export_svg(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("导出 SVG")
            .add_filter("SVG 图形", &["svg"])
            .set_file_name("zbody-vector.svg")
            .save_file()
        else {
            return;
        };

        match export::save_to_file(&path, &self.shapes) {
            Ok(bytes) => {
                self.status = Some(format!(
                    "已导出 {}（{:.1} KB）",
                    path.display(),
                    bytes as f64 / 1024.0
                ));
            }
            Err(err) => {
                self.status = Some(format!("导出失败：{err}"));
            }
        }
    }

    // ---------- 绘制 ----------

    fn paint_draft(&self, painter: &egui::Painter) {
        let Some(draft) = self.draft else {
            return;
        };
        let rect = draft.rect();

        match self.tool {
            Tool::Rect => {
                painter.rect_filled(rect, 0.0, shape::SHAPE_FILL);
                painter.rect_stroke(rect, 0.0, Stroke::new(1.0_f32, ACCENT), StrokeKind::Inside);
            }
            Tool::Ellipse => {
                painter.add(egui::Shape::ellipse_filled(
                    rect.center(),
                    rect.size() * 0.5,
                    shape::SHAPE_FILL,
                ));
                painter.add(egui::Shape::ellipse_stroke(
                    rect.center(),
                    rect.size() * 0.5,
                    Stroke::new(1.0_f32, ACCENT),
                ));
            }
            _ => {}
        }
    }

    fn paint_pen_draft(&self, painter: &egui::Painter) {
        if self.pen_anchors.is_empty() {
            return;
        }

        // 已确定的路径段
        let path = shape::build_bez_path(&self.pen_anchors, false);
        let points = shape::flatten_path(&path, 0.25);
        if points.len() >= 2 {
            painter.add(egui::Shape::line(points, Stroke::new(1.0_f32, ACCENT)));
        }

        // 从最后一个锚点到鼠标位置的橡皮筋预览
        if self.pen_drag.is_none() {
            if let (Some(hover), Some(last)) = (self.pen_hover, self.pen_anchors.last()) {
                let mut preview = kurbo::BezPath::new();
                preview.move_to(shape::to_kurbo_point(last.pos));
                preview.curve_to(
                    shape::to_kurbo_point(last.out_point()),
                    shape::to_kurbo_point(hover),
                    shape::to_kurbo_point(hover),
                );
                let points = shape::flatten_path(&preview, 0.25);
                if points.len() >= 2 {
                    painter.add(egui::Shape::line(
                        points,
                        Stroke::new(1.0_f32, Color32::from_rgb(130, 160, 210)),
                    ));
                }
            }
        }

        paint_anchor_overlay(painter, &self.pen_anchors, true, self.pen_drag);
    }

    fn paint_selection(&self, painter: &egui::Painter) {
        let Some(shape) = self.selected_shape() else {
            return;
        };
        let bounds = shape.bounds();

        if bounds.width() > 0.0 && bounds.height() > 0.0 {
            painter.rect_stroke(
                bounds.expand(1.0),
                2.0,
                Stroke::new(1.5_f32, ACCENT),
                StrokeKind::Outside,
            );

            for corner in [
                bounds.left_top(),
                bounds.right_top(),
                bounds.right_bottom(),
                bounds.left_bottom(),
            ] {
                painter.rect_filled(
                    Rect::from_center_size(corner, egui::vec2(6.0, 6.0)),
                    1.0,
                    ACCENT,
                );
            }
        }

        // 选中钢笔路径时，像 Illustrator 一样显示锚点与控制手柄
        if let ShapeKind::Pen { anchors, .. } = &shape.kind {
            paint_anchor_overlay(painter, anchors, false, None);
        }
    }
}

// ---------- 无状态的辅助函数 ----------

fn install_chinese_font(ctx: &egui::Context) {
    let candidates = [
        "C:/Windows/Fonts/simhei.ttf",
        "C:/Windows/Fonts/msyh.ttc",
        "C:/Windows/Fonts/simsun.ttc",
        "/System/Library/Fonts/PingFang.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    ];

    let Some(bytes) = candidates.iter().find_map(|path| std::fs::read(path).ok()) else {
        return;
    };

    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert("cjk".to_owned(), egui::FontData::from_owned(bytes).into());

    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, "cjk".to_owned());
    }

    ctx.set_fonts(fonts);
}

fn configure_style(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = PANEL_BG;
    visuals.window_fill = PANEL_BG;
    visuals.extreme_bg_color = Color32::from_rgb(20, 20, 24);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(44, 44, 50);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(58, 58, 66);
    visuals.widgets.active.bg_fill = Color32::from_rgb(70, 70, 80);
    visuals.selection.bg_fill = Color32::from_rgb(46, 78, 132);
    visuals.selection.stroke = Stroke::new(1.0_f32, ACCENT);
    ctx.set_visuals(visuals);

    ctx.style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(10.0, 5.0);
        style.text_styles.insert(
            egui::TextStyle::Body,
            FontId::new(13.0, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Small,
            FontId::new(11.5, egui::FontFamily::Proportional),
        );
    });
}

fn tool_button(ui: &mut Ui, tool: Tool, active: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(40.0, 40.0), Sense::click());

    if ui.is_rect_visible(rect) {
        let background = if active {
            ACCENT
        } else if response.hovered() {
            Color32::from_rgb(45, 45, 52)
        } else {
            Color32::TRANSPARENT
        };
        if background != Color32::TRANSPARENT {
            ui.painter().rect_filled(rect, 8.0, background);
        }

        let foreground = if active {
            Color32::WHITE
        } else {
            Color32::from_rgb(200, 200, 208)
        };
        paint_tool_icon(ui.painter(), rect.shrink(9.0), tool, foreground);
    }

    response.on_hover_text(tool.label()).clicked()
}

fn paint_tool_icon(painter: &egui::Painter, rect: Rect, tool: Tool, color: Color32) {
    let stroke = Stroke::new(1.5_f32, color);

    match tool {
        Tool::Select => {
            let tip = egui::pos2(rect.left(), rect.top());
            let wing = egui::pos2(
                rect.left() + rect.width() * 0.82,
                rect.top() + rect.height() * 0.55,
            );
            let notch = egui::pos2(
                rect.left() + rect.width() * 0.34,
                rect.top() + rect.height() * 0.62,
            );
            let tail = egui::pos2(
                rect.left() + rect.width() * 0.20,
                rect.top() + rect.height() * 1.0,
            );

            painter.add(egui::Shape::convex_polygon(
                vec![tip, wing, notch],
                color,
                Stroke::NONE,
            ));
            painter.line_segment([notch, tail], stroke);
        }
        Tool::Rect => {
            painter.rect_stroke(rect, 2.0, stroke, StrokeKind::Inside);
        }
        Tool::Ellipse => {
            painter.add(egui::Shape::ellipse_stroke(
                rect.center(),
                rect.size() * 0.5,
                stroke,
            ));
        }
        Tool::Pen => {
            let top = egui::pos2(rect.center().x, rect.top());
            let left = egui::pos2(rect.left(), rect.top() + rect.height() * 0.62);
            let right = egui::pos2(rect.right(), rect.top() + rect.height() * 0.62);
            let tip = egui::pos2(rect.center().x, rect.bottom());

            painter.add(egui::Shape::convex_polygon(
                vec![top, right, tip, left],
                color,
                Stroke::NONE,
            ));
        }
    }
}

fn paint_anchor_overlay(
    painter: &egui::Painter,
    anchors: &[Anchor],
    draft: bool,
    active: Option<usize>,
) {
    // 控制手柄：一条穿过锚点的方向线 + 两端圆点（正在拖拽的手柄高亮）
    for (index, anchor) in anchors.iter().enumerate() {
        let has_in = anchor.in_handle.length_sq() > 0.001;
        let has_out = anchor.out_handle.length_sq() > 0.001;
        if !has_in && !has_out {
            continue;
        }

        let is_active = active == Some(index);
        let line_color = if is_active {
            Color32::from_rgb(126, 180, 255)
        } else {
            Color32::from_rgb(150, 172, 205)
        };
        let stroke = Stroke::new(if is_active { 1.3_f32 } else { 1.0_f32 }, line_color);
        painter.line_segment([anchor.in_point(), anchor.out_point()], stroke);

        let dot_color = if is_active {
            Color32::WHITE
        } else {
            Color32::from_rgb(198, 212, 234)
        };
        let dot_radius = if is_active { 3.2 } else { 2.6 };
        if has_in {
            painter.circle_filled(anchor.in_point(), dot_radius, dot_color);
        }
        if has_out {
            painter.circle_filled(anchor.out_point(), dot_radius, dot_color);
        }
    }

    // 锚点：白色圆点 + 深色描边
    for (index, anchor) in anchors.iter().enumerate() {
        let is_active = active == Some(index);
        let radius = if is_active { 4.2 } else { 3.4 };

        painter.circle_filled(anchor.pos, radius, Color32::WHITE);
        painter.circle_stroke(
            anchor.pos,
            radius,
            Stroke::new(1.0_f32, Color32::from_rgb(26, 26, 32)),
        );

        if draft && index == 0 {
            // 起点高亮，提示可以点击闭合
            painter.circle_stroke(anchor.pos, 7.5, Stroke::new(1.0_f32, ACCENT));
        }
        if is_active {
            painter.circle_stroke(anchor.pos, radius + 2.0, Stroke::new(1.3_f32, ACCENT));
        }
    }
}

fn export_button(ui: &mut Ui, icon: Option<&TextureHandle>) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(40.0, 40.0), Sense::click());

    if ui.is_rect_visible(rect) {
        let background = if response.hovered() {
            Color32::from_rgb(45, 45, 52)
        } else {
            Color32::TRANSPARENT
        };
        if background != Color32::TRANSPARENT {
            ui.painter().rect_filled(rect, 8.0, background);
        }

        match icon {
            Some(texture) => {
                let size = 32.0;
                Image::new(texture)
                    .fit_to_exact_size(egui::vec2(size, size))
                    .corner_radius(6.0)
                    .paint_at(
                        ui,
                        Rect::from_center_size(rect.center(), egui::vec2(size, size)),
                    );
            }
            None => {
                // 图标尚未加载时的占位文字
                ui.painter().text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    "SVG",
                    FontId::proportional(13.0),
                    Color32::from_rgb(200, 200, 208),
                );
            }
        }
    }

    response.on_hover_text("导出 SVG").clicked()
}

fn paint_grid(painter: &egui::Painter, rect: Rect) {
    const STEP: f32 = 32.0;
    let stroke = Stroke::new(1.0_f32, GRID_LINE);

    let mut x = rect.left();
    while x <= rect.right() {
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            stroke,
        );
        x += STEP;
    }

    let mut y = rect.top();
    while y <= rect.bottom() {
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            stroke,
        );
        y += STEP;
    }
}
