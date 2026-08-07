use plotters::{coord::Shift, prelude::*};

use crate::params::Parameters;

const PLOT_PATH: &str = "tuning/plot.png";
const CELL_WIDTH: u32 = 500;
const CELL_HEIGHT: u32 = 300;

pub fn save_png(history: &[Parameters]) {
    let params = history[0].params();

    let (rows, cols) = grid_size(params.len());

    let width = cols as u32 * CELL_WIDTH;
    let height = rows as u32 * CELL_HEIGHT;

    let root = BitMapBackend::new(PLOT_PATH, (width, height)).into_drawing_area();
    root.fill(&WHITE).unwrap();

    let cells = root.split_evenly((rows, cols));
    for (i, param) in params.iter().enumerate() {
        let values: Vec<f64> = history.iter().map(|snap| snap.params()[i].value).collect();

        draw_param(&cells[i], &param.name, &values);
    }

    root.present().unwrap();
}

fn draw_param(cell: &DrawingArea<BitMapBackend<'_>, Shift>, name: &str, values: &[f64]) {
    let (xmin, xmax, ymin, ymax) = cell_ranges(values);

    let mut chart = ChartBuilder::on(cell)
        .caption(name, ("sans-serif", 18))
        .margin(12)
        .x_label_area_size(10)
        .y_label_area_size(50)
        .build_cartesian_2d(xmin..xmax, ymin..ymax)
        .unwrap();

    chart
        .configure_mesh()
        .light_line_style(TRANSPARENT)
        .draw()
        .unwrap();

    let points = values.iter().copied().enumerate(); // (i, value) = (x, y)
    chart.draw_series(LineSeries::new(points, &BLUE)).unwrap();
}

fn grid_size(n: usize) -> (usize, usize) {
    let cols = (n as f64).sqrt().ceil() as usize;
    (n.div_ceil(cols), cols)
}

fn cell_ranges(values: &[f64]) -> (usize, usize, f64, f64) {
    let xmin = 0;
    let xmax = values.len();

    let ymin = values.iter().copied().reduce(f64::min).unwrap();
    let ymax = values.iter().copied().reduce(f64::max).unwrap();

    // Let's just add 10% padding to the y-axis
    let padding = ((ymax - ymin) * 0.1).max(1.0);

    (xmin, xmax, ymin - padding, ymax + padding)
}
