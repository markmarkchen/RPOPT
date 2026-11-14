//! 2D partition plot as SVG (stand-in for PPOPT's matplotlib `parametric_plot`).

use crate::point_location::geometry::enumerate_vertices;
use crate::solution::Solution;
use std::fmt::Write;

/// Writes the critical regions of a 2-parameter solution as polygons; returns the SVG text.
pub fn parametric_plot_svg(sol: &Solution, width: f64, height: f64) -> String {
    assert_eq!(sol.theta_dim(), 2, "parametric_plot_svg needs a 2D parameter space");
    let polys: Vec<Vec<Vec<f64>>> = sol
        .critical_regions
        .iter()
        .map(|cr| {
            let mut v = enumerate_vertices(&cr.e, &cr.f);
            let cx = v.iter().map(|p| p[0]).sum::<f64>() / v.len().max(1) as f64;
            let cy = v.iter().map(|p| p[1]).sum::<f64>() / v.len().max(1) as f64;
            v.sort_by(|a, b| (a[1] - cy).atan2(a[0] - cx).partial_cmp(&(b[1] - cy).atan2(b[0] - cx)).unwrap());
            v
        })
        .collect();
    let (mut x0, mut x1, mut y0, mut y1) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
    for p in polys.iter().flatten() {
        x0 = x0.min(p[0]);
        x1 = x1.max(p[0]);
        y0 = y0.min(p[1]);
        y1 = y1.max(p[1]);
    }
    let pad = 30.0;
    let sx = (width - 2.0 * pad) / (x1 - x0).max(1e-12);
    let sy = (height - 2.0 * pad) / (y1 - y0).max(1e-12);
    let mut s = String::new();
    let _ = writeln!(s, r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#);
    let _ = writeln!(s, r#"<rect width="100%" height="100%" fill="white"/>"#);
    for (i, poly) in polys.iter().enumerate() {
        if poly.len() < 3 {
            continue;
        }
        let hue = (i as f64 * 137.508) % 360.0;
        let pts: Vec<String> = poly.iter().map(|p| format!("{:.2},{:.2}", pad + (p[0] - x0) * sx, height - pad - (p[1] - y0) * sy)).collect();
        let _ = writeln!(s, r#"<polygon points="{}" fill="hsl({hue:.0},55%,70%)" stroke="black" stroke-width="0.6"/>"#, pts.join(" "));
    }
    let _ = writeln!(s, r#"<text x="{}" y="{}" font-size="12" font-family="sans-serif">θ1 ∈ [{x0:.3}, {x1:.3}], θ2 ∈ [{y0:.3}, {y1:.3}], {} regions</text>"#, pad, pad - 10.0, sol.len());
    s.push_str("</svg>\n");
    s
}
