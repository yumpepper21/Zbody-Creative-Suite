//! 图形数据模型：类型定义、kurbo 贝塞尔路径、命中检测与绘制。

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

/// 新图形的填充色（纯黑）。
pub const SHAPE_FILL: Color32 = Color32::BLACK;

/// 左侧工具栏的工具类型。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    Select,
    Rect,
    Ellipse,
    Pen,
}

impl Default for Tool {
    fn default() -> Self {
        Self::Select
    }
}

impl Tool {
    pub fn label(self) -> &'static str {
        match self {
            Self::Select => "选择",
            Self::Rect => "矩形",
            Self::Ellipse => "椭圆",
            Self::Pen => "钢笔",
        }
    }
}

/// 钢笔路径上的锚点，控制手柄以相对锚点的偏移量保存。
#[derive(Clone, Copy, Debug)]
pub struct Anchor {
    pub pos: Pos2,
    /// 入控制手柄（影响进入该锚点的那一段曲线）。
    pub in_handle: Vec2,
    /// 出控制手柄（影响离开该锚点的那一段曲线）。
    pub out_handle: Vec2,
}

impl Anchor {
    pub fn new(pos: Pos2) -> Self {
        Self {
            pos,
            in_handle: Vec2::ZERO,
            out_handle: Vec2::ZERO,
        }
    }

    pub fn in_point(&self) -> Pos2 {
        self.pos + self.in_handle
    }

    pub fn out_point(&self) -> Pos2 {
        self.pos + self.out_handle
    }
}

/// 图形的几何类型。
#[derive(Clone, Debug)]
pub enum ShapeKind {
    Rect { rect: Rect },
    Ellipse { rect: Rect },
    Pen { anchors: Vec<Anchor>, closed: bool },
}

impl ShapeKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Rect { .. } => "矩形",
            Self::Ellipse { .. } => "椭圆",
            Self::Pen { .. } => "钢笔路径",
        }
    }

    pub fn bounds(&self) -> Rect {
        match self {
            Self::Rect { rect } | Self::Ellipse { rect } => *rect,
            Self::Pen { anchors, .. } => {
                let mut rect = Rect::NOTHING;
                for anchor in anchors {
                    rect.extend_with(anchor.pos);
                    rect.extend_with(anchor.in_point());
                    rect.extend_with(anchor.out_point());
                }
                rect
            }
        }
    }
}

/// 画布上的一个矢量图形。
#[derive(Clone, Debug)]
pub struct Shape {
    pub id: u64,
    pub kind: ShapeKind,
    /// 图形颜色（闭合图形为填充色，未闭合路径为描边色）。
    pub color: Color32,
}

impl Shape {
    pub fn new(id: u64, kind: ShapeKind) -> Self {
        Self {
            id,
            kind,
            color: SHAPE_FILL,
        }
    }

    /// 颜色的 #RRGGBB 十六进制表示。
    pub fn color_hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.color.r(), self.color.g(), self.color.b())
    }

    pub fn bounds(&self) -> Rect {
        self.kind.bounds()
    }

    pub fn translate(&mut self, delta: Vec2) {
        match &mut self.kind {
            ShapeKind::Rect { rect } | ShapeKind::Ellipse { rect } => {
                *rect = rect.translate(delta);
            }
            ShapeKind::Pen { anchors, .. } => {
                for anchor in anchors {
                    anchor.pos += delta;
                }
            }
        }
    }

    /// 绘制图形本体（闭合图形填充、无描边；未闭合路径描边）。
    pub fn paint(&self, painter: &egui::Painter) {
        match &self.kind {
            ShapeKind::Rect { rect } => {
                painter.rect_filled(*rect, 0.0, self.color);
            }
            ShapeKind::Ellipse { rect } => {
                painter.add(egui::Shape::ellipse_filled(
                    rect.center(),
                    rect.size() * 0.5,
                    self.color,
                ));
            }
            ShapeKind::Pen { anchors, closed } => {
                let path = build_bez_path(anchors, *closed);
                let points = flatten_path(&path, 0.25);

                if *closed && points.len() >= 3 {
                    painter.add(egui::Shape::Path(egui::epaint::PathShape {
                        points,
                        closed: true,
                        fill: self.color,
                        stroke: egui::epaint::PathStroke::NONE,
                    }));
                } else if points.len() >= 2 {
                    // 未闭合的路径画成细线，方便继续编辑
                    painter.add(egui::Shape::line(points, Stroke::new(2.0_f32, self.color)));
                }
            }
        }
    }

    /// 判断某个点是否落在图形内（tolerance 为额外的容差，方便点选）。
    pub fn hit_test(&self, pos: Pos2, tolerance: f32) -> bool {
        match &self.kind {
            ShapeKind::Rect { rect } => rect.expand(tolerance).contains(pos),
            ShapeKind::Ellipse { rect } => ellipse_contains(*rect, pos, tolerance),
            ShapeKind::Pen { anchors, closed } => pen_contains(anchors, *closed, pos, tolerance),
        }
    }
}

/// 用 kurbo 构建路径：相邻锚点之间是三次贝塞尔曲线。
pub fn build_bez_path(anchors: &[Anchor], closed: bool) -> kurbo::BezPath {
    let mut path = kurbo::BezPath::new();
    let Some(first) = anchors.first() else {
        return path;
    };

    path.move_to(to_kurbo_point(first.pos));

    for pair in anchors.windows(2) {
        let from = pair[0];
        let to = pair[1];
        path.curve_to(
            to_kurbo_point(from.out_point()),
            to_kurbo_point(to.in_point()),
            to_kurbo_point(to.pos),
        );
    }

    if closed && anchors.len() >= 2 {
        let last = *anchors.last().unwrap();
        path.curve_to(
            to_kurbo_point(last.out_point()),
            to_kurbo_point(first.in_point()),
            to_kurbo_point(first.pos),
        );
        path.close_path();
    }

    path
}

/// 把 kurbo 路径离散成折线，交给 egui 绘制 / 命中检测。
pub fn flatten_path(path: &kurbo::BezPath, tolerance: f64) -> Vec<Pos2> {
    let mut points = Vec::new();
    let mut start = None;

    kurbo::flatten(path.elements().iter().copied(), tolerance, |el| match el {
        kurbo::PathEl::MoveTo(p) => {
            start = Some(to_egui_pos(p));
            points.clear();
            points.push(to_egui_pos(p));
        }
        kurbo::PathEl::LineTo(p) => points.push(to_egui_pos(p)),
        kurbo::PathEl::ClosePath => {
            if let Some(first) = start {
                points.push(first);
            }
        }
        _ => {}
    });

    points
}

pub fn to_kurbo_point(pos: Pos2) -> kurbo::Point {
    kurbo::Point::new(pos.x as f64, pos.y as f64)
}

fn to_egui_pos(point: kurbo::Point) -> Pos2 {
    egui::pos2(point.x as f32, point.y as f32)
}

fn ellipse_contains(rect: Rect, pos: Pos2, tolerance: f32) -> bool {
    let size = rect.size() * 0.5;
    let radius = egui::vec2(size.x.max(0.0), size.y.max(0.0));
    if radius.x <= f32::EPSILON || radius.y <= f32::EPSILON {
        return rect.expand(tolerance).contains(pos);
    }
    let d = pos - rect.center();
    let dx = d.x / (radius.x + tolerance);
    let dy = d.y / (radius.y + tolerance);
    dx * dx + dy * dy <= 1.0
}

fn pen_contains(anchors: &[Anchor], closed: bool, pos: Pos2, tolerance: f32) -> bool {
    let path = build_bez_path(anchors, closed);
    let points = flatten_path(&path, 0.25);

    if points.len() < 2 {
        return false;
    }

    if closed && points.len() >= 3 && point_in_polygon(pos, &points) {
        return true;
    }

    points
        .windows(2)
        .any(|w| dist_to_segment(pos, w[0], w[1]) <= tolerance)
}

fn dist_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let len_sq = ab.length_sq();
    if len_sq <= f32::EPSILON {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

fn point_in_polygon(p: Pos2, poly: &[Pos2]) -> bool {
    let n = poly.len();
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let a = poly[i];
        let b = poly[j];
        if (a.y > p.y) != (b.y > p.y) {
            let x = (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x;
            if p.x < x {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}
