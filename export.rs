//! SVG 导出：把画布上的所有图形序列化为标准 SVG XML。

use eframe::egui::Rect;

use super::shape::{Anchor, Shape, ShapeKind};

/// 导出边距（画布四周留白）。
const MARGIN: f64 = 20.0;

/// 标准 XML 声明（svg crate 不自动生成）。
const XML_DECLARATION: &str = r#"<?xml version="1.0" encoding="UTF-8"?>"#;

/// 计算所有图形的整体包围盒。
pub fn content_bounds(shapes: &[Shape]) -> Option<Rect> {
    let mut bounds: Option<Rect> = None;
    for shape in shapes {
        let rect = shape.bounds();
        bounds = Some(match bounds {
            Some(acc) => acc.union(rect),
            None => rect,
        });
    }
    bounds
}

/// 把图形列表序列化为标准 SVG 文档字符串。
pub fn to_svg_document(shapes: &[Shape]) -> String {
    // 空画布时导出一个 100x100 的空白文档
    let Some(bounds) = content_bounds(shapes) else {
        return format!(
            "{}\n<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"100\" viewBox=\"0 0 100 100\"/>",
            XML_DECLARATION
        );
    };

    // 重新定基到原点，并留出边距
    let origin_x = bounds.min.x as f64 - MARGIN;
    let origin_y = bounds.min.y as f64 - MARGIN;
    let width = (bounds.width() as f64).max(1.0) + MARGIN * 2.0;
    let height = (bounds.height() as f64).max(1.0) + MARGIN * 2.0;

    let mut doc = svg::Document::new()
        .set("xmlns", "http://www.w3.org/2000/svg")
        .set("width", width)
        .set("height", height)
        .set("viewBox", (0, 0, width, height));

    for shape in shapes {
        // 颜色的 #RRGGBB 表示
        let hex = format!(
            "#{:02X}{:02X}{:02X}",
            shape.color.r(),
            shape.color.g(),
            shape.color.b()
        );

        match &shape.kind {
            ShapeKind::Rect { rect } => {
                let rect = translate_rect(*rect, origin_x, origin_y);
                doc = doc.add(
                    svg::node::element::Rectangle::new()
                        .set("x", fmt(rect.min.x))
                        .set("y", fmt(rect.min.y))
                        .set("width", fmt(rect.width()))
                        .set("height", fmt(rect.height()))
                        .set("fill", hex.clone())
                        .set("stroke", "none"),
                );
            }
            ShapeKind::Ellipse { rect } => {
                let rect = translate_rect(*rect, origin_x, origin_y);
                doc = doc.add(
                    svg::node::element::Ellipse::new()
                        .set("cx", fmt(rect.center().x))
                        .set("cy", fmt(rect.center().y))
                        .set("rx", fmt((rect.width() * 0.5).abs()))
                        .set("ry", fmt((rect.height() * 0.5).abs()))
                        .set("fill", hex.clone())
                        .set("stroke", "none"),
                );
            }
            ShapeKind::Pen { anchors, closed } => {
                let offset = eframe::egui::vec2(origin_x as f32, origin_y as f32);
                let moved: Vec<Anchor> = anchors
                    .iter()
                    .map(|a| Anchor {
                        pos: a.pos + offset,
                        in_handle: a.in_handle,
                        out_handle: a.out_handle,
                    })
                    .collect();

                let path = super::shape::build_bez_path(&moved, *closed);
                doc = doc.add(
                    svg::node::element::Path::new()
                        .set("d", path.to_svg())
                        .set("fill", if *closed { hex.clone() } else { "none".to_string() })
                        .set("stroke", if *closed { "none" } else { &hex })
                        .set("stroke-width", if *closed { 0 } else { 2 })
                        .set("stroke-linecap", "round")
                        .set("stroke-linejoin", "round"),
                );
            }
        }
    }

    format!("{}\n{}", XML_DECLARATION, doc.to_string())
}

/// 生成 SVG 并写入文件，返回写入的字节数。
pub fn save_to_file(path: &std::path::Path, shapes: &[Shape]) -> std::io::Result<u64> {
    let xml = to_svg_document(shapes);
    std::fs::write(path, xml.as_bytes())?;
    Ok(xml.len() as u64)
}

fn translate_rect(rect: Rect, dx: f64, dy: f64) -> Rect {
    Rect::from_min_size(
        eframe::egui::pos2(
            rect.min.x - dx as f32,
            rect.min.y - dy as f32,
        ),
        rect.size(),
    )
}

/// 统一坐标精度，避免浮点噪声。
fn fmt(v: f32) -> String {
    format!("{:.2}", v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{pos2, vec2};

    #[test]
    fn rect_and_ellipse_and_pen_are_exported() {
        let shapes = vec![
            Shape::new(
                1,
                ShapeKind::Rect {
                    rect: Rect::from_min_size(pos2(10.0, 20.0), vec2(100.0, 50.0)),
                },
            ),
            Shape::new(
                2,
                ShapeKind::Ellipse {
                    rect: Rect::from_min_size(pos2(200.0, 0.0), vec2(80.0, 80.0)),
                },
            ),
            Shape::new(
                3,
                ShapeKind::Pen {
                    anchors: vec![
                        Anchor {
                            pos: pos2(0.0, 0.0),
                            in_handle: vec2(0.0, 0.0),
                            out_handle: vec2(30.0, 0.0),
                        },
                        Anchor {
                            pos: pos2(100.0, 100.0),
                            in_handle: vec2(-30.0, 0.0),
                            out_handle: vec2(0.0, 0.0),
                        },
                    ],
                    closed: true,
                },
            ),
        ];

        let svg = to_svg_document(&shapes);

        assert!(svg.starts_with("<?xml"), "缺少 XML 声明: {svg}");
        assert!(svg.contains("<svg"), "缺少 svg 根元素");
        assert!(svg.contains("<rect"), "缺少 rect 元素");
        assert!(svg.contains("<ellipse"), "缺少 ellipse 元素");
        assert!(svg.contains("<path"), "缺少 path 元素");
        assert!(svg.contains("C"), "贝塞尔路径缺少 C 指令");
        assert!(svg.contains("Z"), "闭合路径缺少 Z 指令");
        assert!(svg.contains(r##"fill="#000000""##), "缺少黑色填充");
    }

    #[test]
    fn empty_canvas_exports_blank_document() {
        let svg = to_svg_document(&[]);
        assert!(svg.contains(r#"width="100""#));
        assert!(svg.contains(r#"height="100""#));
    }
}
