use cgmath::{MetricSpace, Vector2};
use std::f64::{consts::PI, INFINITY};

pub type Point = Vector2<f64>;

pub struct Unistroke {
    pub name: String,
    pub path: Vec<Point>,
}

#[allow(dead_code)]
pub struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

pub struct UnistrokeRecognizer {
    pub angle_range_rad: f64,
    pub angle_precision: f64,
    pub width: f64,
    pub height: f64,
    pub resample_num_points: u32,
    pub patterns: Vec<Unistroke>,
}

impl UnistrokeRecognizer {
    pub fn recognize_unistroke(&self, path: &[Point]) -> (&Unistroke, f64) {
        let diagonal: f64 = Point::new(self.width, self.height).distance(Point::new(0.0, 0.0));
        let path = self.normalize_stroke_path(path);

        let mut similar_pattern = &self.patterns[0];
        let mut b = f64::INFINITY;
        for pattern in &self.patterns {
            let d = distance_at_best_angle(
                &path,
                &pattern.path,
                -self.angle_range_rad,
                self.angle_range_rad,
                self.angle_precision,
            );

            if d < b {
                b = d;
                similar_pattern = &pattern;
            }
        }

        (similar_pattern, 1.0 - b / (diagonal / 2.0))
    }

    pub fn add_pattern(&mut self, name: String, path: &[Point]) {
        let path = self.normalize_stroke_path(path);
        let unistroke = Unistroke {
            name,
            path: path.to_vec(),
        };
        self.patterns.push(unistroke);
    }

    fn normalize_stroke_path(&self, path: &[Point]) -> Vec<Point> {
        let path = resample(&path, self.resample_num_points);
        let path = scale_to(&path, Point::new(self.width, self.height));
        let path = translate_to(&path, Point::new(0.0, 0.0));

        path
    }
}

pub fn degrees_to_radians(degrees: f64) -> f64 {
    degrees * PI / 180.0
}

fn resample(path: &[Point], n: u32) -> Vec<Point> {
    if path.is_empty() {
        return Vec::new();
    }

    let path_length = path_length(path) / (n - 1) as f64;
    let mut distance_acc: f64 = 0.0;

    let mut new_path: Vec<Point> = vec![path[0]];

    for i in 1..path.len() {
        let prev_point = path[i - 1];
        let curr_point = path[i];

        let d = curr_point.distance(prev_point);

        distance_acc += d;
        let mid_point_count = (distance_acc / path_length) as i32;
        if mid_point_count > 0 {
            for mid_point in 0..mid_point_count {
                let mid_ratio = mid_point as f64 / mid_point_count as f64;
                let q_point = Point::new(
                    prev_point.x + mid_ratio * (curr_point.x - prev_point.x),
                    prev_point.y + mid_ratio * (curr_point.y - prev_point.y),
                );
                new_path.push(q_point);
            }
            distance_acc -= mid_point_count as f64 * path_length;
        }
    }

    if new_path.len() == (n - 1) as usize {
        // fix rounding-error
        new_path.push(*path.last().unwrap());
    }

    assert_eq!(
        new_path.len(),
        n as usize,
        "path length after resampling not equal expected"
    );

    new_path
}

fn path_length(path: &[Point]) -> f64 {
    let mut total_distance = 0.0;
    for i in 1..path.len() {
        let prev_point = path[i - 1];
        let curr_point = path[i];
        total_distance += prev_point.distance(curr_point);
    }
    return total_distance;
}

fn rotate_by(path: &[Point], theta: f64) -> Vec<Point> {
    let c = centroid(path);
    let mut new_path = Vec::new();
    for p in path {
        let q_x = (p.x - c.x) * theta.cos() - (p.y - c.y) * theta.sin() + c.x;
        let q_y = (p.x - c.x) * theta.sin() + (p.y - c.y) * theta.cos() + c.x;
        new_path.push(Point::new(q_x, q_y));
    }

    new_path
}

fn bounding_box(path: &[Point]) -> Rect {
    let mut min_x = INFINITY;
    let mut min_y = INFINITY;
    let mut max_x = -INFINITY;
    let mut max_y = -INFINITY;

    for p in path {
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
    }

    Rect {
        x: min_x,
        y: min_y,
        w: max_x - min_x,
        h: max_y - min_y,
    }
}

fn centroid(path: &[Point]) -> Point {
    let sum_point: Point = path.iter().sum();
    sum_point / path.len() as f64
}

fn distance_at_angle(path: &[Point], template_path: &[Point], radians: f64) -> f64 {
    let new_path = rotate_by(path, radians);
    path_distance(&new_path, template_path)
}

fn path_distance(lhs_path: &[Point], rhs_path: &[Point]) -> f64 {
    assert_eq!(lhs_path.len(), rhs_path.len());

    let mut d = 0.0;

    for (p1, p2) in lhs_path.iter().zip(rhs_path) {
        d += p1.distance(*p2);
    }

    d / lhs_path.len() as f64
}

fn translate_to(path: &[Point], pt: Point) -> Vec<Point> {
    let c = centroid(path);
    let mut new_points = Vec::new();
    for p in path {
        let q_x = p.x + pt.x - c.x;
        let q_y = p.y + pt.y - c.y;
        new_points.push(Point::new(q_x, q_y));
    }

    new_points
}

fn scale_to(path: &[Point], size: Point) -> Vec<Point> {
    let bound = bounding_box(path);
    let mut new_points = Vec::new();

    let scale = f64::min(size.x / bound.w, size.y / bound.h);

    for p in path {
        let q_x = p.x * scale;
        let q_y = p.y * scale;
        new_points.push(Point::new(q_x, q_y));
    }

    new_points
}

fn distance_at_best_angle(
    path: &[Point],
    template_path: &[Point],
    mut a: f64,
    mut b: f64,
    threshold: f64,
) -> f64 {
    let phi = 0.5 * (-1.0 + (5.0f64).sqrt()); // Golden Ratio
    let mut x1 = phi * a + (1.0 - phi) * b;
    let mut f1 = distance_at_angle(path, template_path, x1);
    let mut x2 = (1.0 - phi) * a + phi * b;
    let mut f2 = distance_at_angle(path, template_path, x2);

    while f64::abs(b - a) > threshold {
        if f1 < f2 {
            b = x2;
            x2 = x1;
            f2 = f1;
            x1 = phi * a + (1.0 - phi) * b;
            f1 = distance_at_angle(path, template_path, x1);
        } else {
            a = x1;
            x1 = x2;
            f1 = f2;
            x2 = (1.0 - phi) * a + phi * b;
            f2 = distance_at_angle(path, template_path, x2);
        }
    }

    f64::min(f1, f2)
}
